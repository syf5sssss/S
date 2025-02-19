<template>
  <div>
    <div id="map-container" ref="mapContainer">
    </div>
    <div style="font-size: xx-small;">
      <span class="info-text" @click="dialogVisible = true">
        成功加载图片数量 {{ count }} 当前展示级别 {{ ratio }}
      </span>
    </div>
  </div>
  <div class="card">
    <Dialog v-model:visible="dialogVisible" header="详细信息" modal class="p-dialog-maximized">
      <DataTable :value="imgs" scrollable scrollHeight="flex" tableStyle="min-width: 50rem">
        <Column v-for="column in columns" :key="column.field" :field="column.field" :header="column.header"></Column>
      </DataTable>
    </Dialog>
  </div>
  <div class="card">
    <Dialog v-model:visible="dialogimgsVisible" header="预览图片" modal class="p-dialog-maximized">
      <DataView :value="products" :layout="layout">
        <template #grid="slotProps">
          <div class="grid grid-cols-12 ">
            <div v-for="(item, index) in slotProps.items" :key="index"
              class="col-span-12 sm:col-span-6 md:col-span-4 xl:col-span-6 p-1">
              <div
                class="p-1 border border-surface-200 dark:border-surface-700 bg-surface-0 dark:bg-surface-900 rounded flex flex-col">
                <div class="bg-surface-50 flex justify-center rounded p-1">
                  <div class="relative mx-auto">
                    <Image :src="`http://asset.localhost/${item}`" :alt="item" width="250" preview />
                    <!-- <Image :src="`http://asset.localhost/D:/TEST/rt2/2222222222222/IMG_20241013_200808.jpg`" :alt="item"
                      width="250" preview /> -->

                    <!-- <img class="rounded w-full" :src="`http://asset.localhost/${item}`" :alt="item"
                      style="max-width: 300px" /> -->
                  </div>
                </div>
              </div>
            </div>
          </div>
        </template>
      </DataView>
    </Dialog>
  </div>
</template>

<script setup>
import { onMounted, ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { message } from '@tauri-apps/plugin-dialog';
import DataTable from 'primevue/datatable';
import Dialog from 'primevue/dialog';
import Column from 'primevue/column';
import DataView from 'primevue/dataview';
import Image from 'primevue/image';



let imgs = ref([]);
let ratio = ref(5);
let count = ref(0);
let map = null;
var convertor = new BMapGL.Convertor();

const products = ref();
const layout = ref('grid');
let maxlevel = ref(4);;

// 定义列的字段和标题
const columns = ref([
  { field: 'name', header: '名称' },
  { field: 'lng', header: '经度' },
  { field: 'lat', header: '纬度' },
  { field: 'time', header: '时间' },
  { field: 'path', header: '地址' }
]);
const dialogVisible = ref(false);
const dialogimgsVisible = ref(false);

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function getdirpath() {
  try {
    const file = await open({
      multiple: false,
      directory: true,
    });
    if (file && file.length > 0) {
      const start = performance.now();
      truncate();
      imgs.value = await invoke('load_dir_imgs', { path: file });
      const end1 = performance.now();
      console.log(`加载 运行时间: ${end1 - start}ms`);
      if (imgs.value && imgs.value.length > 0) {
        // console.log(imgs.value);//不知道为什么，这这里会很自然地将后端的数据修改掉，lat和lng数据交换了？？？
        const batchSize = 10; // 每次处理的批大小
        const totalBatches = Math.ceil(imgs.value.length / batchSize); // 计算总批次数
        for (let batchIndex = 0; batchIndex < totalBatches; batchIndex++) {
          let lrr = []; // 用于存储当前批次的点
          const start = batchIndex * batchSize; // 当前批次的起始索引
          const end = Math.min(start + batchSize, imgs.value.length); // 当前批次的结束索引

          for (let i = start; i < end; i++) {
            if (imgs.value[i].lat && imgs.value[i].lng) {
              //(x,y) 这里按照api的意思，x是相对来说比较大的那个
              if (imgs.value[i].lng >= imgs.value[i].lat) {
                lrr.push(new BMapGL.Point(imgs.value[i].lng, imgs.value[i].lat));
              } else {
                lrr.push(new BMapGL.Point(imgs.value[i].lat, imgs.value[i].lng));
              }
            }
          }

          if (lrr.length > 0) {
            console.log('转换一次 数量' + lrr.length);
            convertor.translate(lrr, COORDINATES_WGS84, COORDINATES_BD09, function (data) {
              if (data.status === 0) {
                // console.log(data.points);
                for (let i = 0; i < data.points.length; i++) {
                  imgs.value[start + i].lat = data.points[i].lat;
                  imgs.value[start + i].lng = data.points[i].lng;
                }
              } else {
                console.error("转换失败", data);
              }
            });
          }
          await sleep(200);
        }
      }
      console.log('准备插入图片');
      // console.log(imgs.value);
      invoke('insert_imgs', { imgs: imgs.value })
        .then(() => {
          // 插入完成后刷新页面
          location.reload();
        })
        .catch((error) => {
          console.error("插入失败", error);
        });
      const end2 = performance.now();
      console.log('图片数量' + imgs.value.length);
      console.log(`转换 运行时间: ${end2 - end1}ms`);
    }
  } catch (error) {
    console.log(error);
    message(JSON.stringify(error, null, 2), { title: 'Tauri', kind: 'getdirpath error' });
  }
}

async function get_all() {
  try {
    let res = await invoke('query_all');
    if (res && res.length > 0) {
      imgs.value = res;
      load();
    }
  } catch (error) {
    console.log(error);
    message(JSON.stringify(error, null, 2), { title: 'Tauri', kind: 'get_all error' });
  }
}
async function truncate() {
  try {
    let res = await invoke('truncate');
  } catch (error) {
    console.log(error);
    message(JSON.stringify(error, null, 2), { title: 'Tauri', kind: 'truncate error' });
  }
}

function load() {
  map.clearOverlays();
  if (imgs.value.length > 0) {
    let clusters = clusterMarkers(map.getZoom());
    reloadMarker(map, clusters);
  }
}


function showImg(i) {
  products.value = clusters[i].arr;
  dialogimgsVisible.value = true;
}
window.showImg = showImg;

//定义一个控件类
function ZoomControl() {
  this.defaultAnchor = BMAP_ANCHOR_TOP_LEFT;
  this.defaultOffset = new BMapGL.Size(20, 20)
}

function reloadMarker(map, clusters) {
  try {
    // console.log(clusters);
    // 创建 RichMarker 并添加到地图上
    for (let i = 0; i < clusters.length; i++) {
      var badgeContent = clusters[i].arr.length + ''; // 将数字转换为字符串
      // 根据数字长度动态调整徽章的宽度和字体大小
      var badgeWidth = Math.max(20, 15 + badgeContent.length * 4); // 最小宽度为20px，每多一个字符增加12px
      var badgeFontSize = 10; //Math.min(10, 9 + 12 / badgeContent.length); // 字体大小，最多12px，根据字符长度动态调整
      var imgurl = 'http://asset.localhost/' + clusters[i].path;
      var htm1 =
        "<div id='overLay' style='width:93px;height:116px; background:url(./back.png) left top no-repeat;position: absolute;'>" +
        "<div class='image-container'>" +
        "<img onclick='showImg(\"" + i + "\")'  style='margin-left:9px;margin-top: 8px;width:75px;height:75px;' src='" + imgurl + "'/>" +
        "</div>" +
        "<div style='position: absolute; top: -8px; right: -9px; background-color: red; color: white; font-size: " + badgeFontSize + "px; width: " + badgeWidth + "px; height: " + badgeWidth + "px; line-height: " + badgeWidth + "px; text-align: center; border-radius: 50%;'>" +
        badgeContent +
        "</div>" +
        "</div>";
      const point = new window.BMapGL.Point(clusters[i].lng, clusters[i].lat);
      const richMarker = new window.BMapGLLib.RichMarker(htm1, point, {
        anchor: new window.BMapGL.Size(-47, -116),
        enableDragging: true,
      });
      map.addOverlay(richMarker);
      console.log('添加一个标记');
    }
    count.value = imgs.value.length;
  } catch (error) {
    console.log(error);
  }
}


onMounted(async () => {
  try {
    // 确保百度地图和扩展库已加载
    if (!window.BMapGL || !window.BMapGLLib) {
      alert("百度地图或扩展库未正确加载");
      console.error("百度地图或扩展库未正确加载");
    }
    const mapContainer = document.getElementById('map-container');
    map = new BMapGL.Map(mapContainer);
    map.centerAndZoom(new BMapGL.Point(105.401952, 40.032704), ratio.value);
    map.enableScrollWheelZoom();
    map.enablePinchToZoom();
    // 添加比例尺控件
    var scaleCtrl = new BMapGL.ScaleControl();
    map.addControl(scaleCtrl);
    var zoomCtrl = new BMapGL.ZoomControl();  // 添加缩放控件
    map.addControl(zoomCtrl);
    //通过JavaScript的prototype属性继承于BMap.Control
    ZoomControl.prototype = new BMapGL.Control();

    // 监听缩放事件
    map.addEventListener("zoomend", function () {
      var currentZoom = map.getZoom(); // 获取当前缩放级别
      if (currentZoom > maxlevel.value) {
        load();
      }
      ratio.value = currentZoom;
      if (currentZoom > maxlevel.value) {
        maxlevel.value = currentZoom;
      }
    });

    //自定义控件必须实现自己的initialize方法，并且将控件的DOM元素返回
    //在本方法中创建个div元素作为控件的容器，并将其添加到地图容器中
    ZoomControl.prototype.initialize = function (map) {
      //创建一个dom元素
      var div = document.createElement('div');
      //添加文字说明
      div.appendChild(document.createTextNode('重新加载'));
      // 设置样式
      div.style.cursor = "pointer";
      div.style.padding = "7px 10px";
      div.style.boxShadow = "0 2px 6px 0 rgba(27, 142, 236, 0.5)";
      div.style.borderRadius = "5px";
      div.style.backgroundColor = "white";
      // 绑定事件,点击一次放大两级
      div.onclick = function (e) {
        getdirpath();
      }
      // 添加DOM元素到地图中
      map.getContainer().appendChild(div);
      // 将DOM元素返回
      return div;
    }
    //创建控件元素
    var myZoomCtrl = new ZoomControl();
    //添加到地图中
    map.addControl(myZoomCtrl);
    get_all();
  } catch (error) {
    console.error('Failed to initialize map or RichMarker:', error);
    message(JSON.stringify(error, null, 2), { title: 'Tauri', kind: 'error' });
  }
});


let clusters = [];
function getGridSize(zooms) {
  let zoom = parseFloat(zooms + '');
  if (zoom <= 5) {
    return 1;
  }
  if (7 >= zoom && zoom > 5) {
    return 0.85;//200km
  }
  if (9 >= zoom && zoom > 7) {
    return 0.75;//50km
  }
  if (11 >= zoom && zoom > 9) {
    return 0.65;//20km
  }
  if (15 >= zoom && zoom > 11) {
    return 0.01;//5km
  }
  if (19 >= zoom && zoom > 15) {
    return 0.005;//50
  }
  if (21 >= zoom && zoom > 19) {
    return 0.0001;//10
  }
  return 1;
}

// 聚合算法（基于网格）
function clusterMarkers(zoom) {
  let gridSize = getGridSize(zoom);
  clusters = [];
  // 将点分配到网格
  imgs.value.forEach(point => {
    let flag = 0;
    if (clusters.length > 0) {
      for (let i = 0; i < clusters.length; i++) {
        if ((clusters[i].lng + gridSize) >= point.lng && (clusters[i].lng - gridSize) <= point.lng) {
          if ((clusters[i].lat + gridSize) >= point.lat && (clusters[i].lat - gridSize) <= point.lat) {
            clusters[i].arr.push(point.path);
            flag = 1;
          }
        }
      }
      if (flag === 0) {
        clusters.push({
          lng: point.lng,
          lat: point.lat,
          id: point.id,
          name: point.name,
          path: point.path,
          time: point.time,
          arr: [point.path]
        });
      }
    } else {
      clusters.push({
        lng: point.lng,
        lat: point.lat,
        id: point.id,
        name: point.name,
        path: point.path,
        time: point.time,
        arr: [point.path]
      });
    }
  });

  return Object.values(clusters);
}

</script>

<style>
#map-container {
  width: 100%;
  height: 95vh;
  /* 使用 vh 单位使地图容器占满整个视口高度 */
  margin: 0;
  padding: 0;
}

.image-container {
  position: relative;
}

.image-container img {
  transition: transform 0.3s ease, box-shadow 0.3s ease;
}

.image-container:hover img {
  transform: scale(1.1);
  box-shadow: 0 0 10px rgba(0, 0, 0, 0.5);
}

.info-text {
  cursor: pointer;
  /* 鼠标悬浮时显示手型 */
  color: black;
  /* 默认文字颜色 */
}

.info-text:hover {
  color: blue;
  /* 鼠标悬浮时文字颜色变为蓝色 */
}

/* 全局调整 DataTable 的字体大小 */
.p-datatable .p-datatable-tbody td {
  font-size: 10px;
  /* 设置为所需字体大小 */
}
</style>