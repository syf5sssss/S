<template>
  <div>
    <div class="container" style="font-size: 16px;padding: 3px;">
      <div class="left">
        <span class="info-text" @click="dialogVisible = true">
          图片数量 {{ count }} 当前展示级别 {{ ratio.toFixed(2) }}&nbsp;&nbsp;&nbsp;&nbsp; {{ info }}
        </span>
      </div>
      <div class="right">
        <Button class="info-text" @click="load_file_img()" size="small">
          读取图片
        </Button>
        &nbsp;&nbsp;
        <Button class="info-text" @click="dialogLocationVisible = true" size="small">
          设置定位
        </Button>
        &nbsp;&nbsp;
        <Button class="info-text" @click="convertImages()" size="small">
          加载目录
        </Button>
        &nbsp;&nbsp;
        <Button class="info-text" @click="showRedPoint()" size="small">
          红点
        </Button>
        &nbsp;&nbsp;
        <Button class="info-text" @click="showImgPoint()" size="small">
          图片
        </Button>
      </div>
    </div>
    <div id="map-container" ref="mapContainer">
    </div>
    <!-- <img :src="`http://asset.localhost/C:/Users/Administrator/Pictures/中国地图.png`" :alt="item" width="250"> -->
  </div>
  <div class="card">
    <Dialog v-model:visible="dialogVisible" header="详细信息" modal class="p-dialog-maximized">
      <DataTable v-model:filters="filters" :value="imgs" scrollable scrollHeight="flex" paginator :rows="20"
        :rowsPerPageOptions="[20, 100, 500]" tableStyle="min-width: 50rem"
        :globalFilterFields="['name', 'path', 'lat', 'lng', 'time']" filterDisplay="row" :loading="loading">
        <template #header>
          <InputText v-model="filters['global'].value" placeholder="搜索" fluid />
        </template>
        <template #empty> No customers found. </template>
        <template #loading> Loading customers data. Please wait. </template>
        <Column v-for="column in columns" :key="column.field" :field="column.field" :header="column.header"></Column>
      </DataTable>
    </Dialog>
  </div>
  <div class="card">
    <Dialog v-model:visible="dialogimgsVisible" header="预览图片" modal class="p-dialog-maximized">
      <template #header>
        <div>
          预览图片&nbsp;&nbsp;&nbsp;&nbsp;
          <Button class="info-text" @click="openLocation" size="small">
            设置定位
          </Button>
        </div>
      </template>
      <DataView :value="products" :layout="layout">
        <template #grid="slotProps">
          <div class="grid grid-cols-12 ">
            <div v-for="(item, index) in slotProps.items" :key="index"
              class="col-span-12 sm:col-span-6 md:col-span-4 xl:col-span-6 p-1">
              <div
                class="p-1 border border-surface-200 dark:border-surface-700 bg-surface-0 dark:bg-surface-900 rounded flex flex-col">
                <div class="bg-surface-50 flex justify-center rounded p-1">
                  <div class="relative mx-auto">
                    <!-- <img :src="`http://asset.localhost/${item}`" :alt="item" width="250"> -->
                    <Image :src="`http://asset.localhost/${item}`" :alt="item" width="250" preview />
                  </div>
                </div>
              </div>
            </div>
          </div>
        </template>
      </DataView>
    </Dialog>
  </div>
  <div class="card">
    <Dialog v-model:visible="dialogLocationVisible" :header="location_info" modal class="p-dialog-maximized"
      @show="initMap" @hide="closeMap">
      <!-- 用于布局的容器 -->
      <div class="dialog-content-container">
        <!-- 左边部分 -->
        <div class="left-section">
          <div class="table-container">
            <DataTable v-model:selection="selectedProduct" :value="locationImgs" selectionMode="single" dataKey="id"
              :metaKeySelection="false" paginator :rows="1" @page="onPageChange"
              paginatorTemplate="RowsPerPageDropdown FirstPageLink PrevPageLink CurrentPageReport NextPageLink LastPageLink"
              currentPageReportTemplate="{first} / {totalRecords}" @rowSelect="onRowSelect" scrollable>
              <template #header>
                <InputGroup>
                  <InputText v-model="pathinfo" placeholder="搜索" />
                  <Button label="查找地址" severity="contrast" @click="selLocation()"></Button>
                </InputGroup>
                <InputGroup class="w-full mt-2">
                  <Button label="选择目录(创建定位)" @click="load_dir_nogpsimgs()" class="w-full"></Button>
                  <Button label="选择目录(调整定位)" severity="info" @click="load_location_img()" class="w-full"></Button>
                </InputGroup>
              </template>
              <template #empty> No customers found. </template>
              <template #loading> Loading customers data. Please wait. </template>
              <Column style="width: 100%;">
                <template #body="slotProps">
                  <div class="column-content">
                    <Image :src="`http://asset.localhost/${slotProps.data.path}`" width="280" preview />
                    <div>{{ slotProps.data.path }}</div>
                  </div>
                </template>
              </Column>
            </DataTable>
          </div>
        </div>
        <!-- 右边部分 -->
        <div class="right-section">
          <div id="map-location" ref="mapLocation">
          </div>
        </div>
      </div>
    </Dialog>
  </div>
  <div>
    <BlockUI :blocked="isLoading" fullScreen>
    </BlockUI>
    <ConfirmDialog></ConfirmDialog>
  </div>
</template>

<script setup>
import { onMounted, ref, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { message } from '@tauri-apps/plugin-dialog';
import { listen } from '@tauri-apps/api/event';
import DataTable from 'primevue/datatable';
import Dialog from 'primevue/dialog';
import Column from 'primevue/column';
import DataView from 'primevue/dataview';
import Image from 'primevue/image';
import Button from 'primevue/button';
import BlockUI from 'primevue/blockui';
import { FilterMatchMode } from '@primevue/core/api';
import InputText from 'primevue/inputtext';
import InputGroup from 'primevue/inputgroup';
import ConfirmDialog from 'primevue/confirmdialog';
import { useConfirm } from "primevue/useconfirm";

const confirm = useConfirm();
const info = ref('');
const location_info = ref('设置定位');
const pathinfo = ref('');
const loading = ref(true);
const isLoading = ref(false);
let imgs = ref([]);
let locationImgs = ref([]);
let updatels = ref([]);
let insertls = ref([]);
let ratio = ref(5);
let count = ref(0);
let map = null;
let map_location = null;
var convertor = new BMapGL.Convertor();
var myGeo = new BMapGL.Geocoder();
const products = ref();
const layout = ref('grid');
const currentPage = ref(0); // 当前页码，从 0 开始
//magick D:/TEST/IMG_9866.CR3 D:/TEST/IMG_9866.jpg
//magick --version
let maxlevel = ref(4);
let gridsizelast = ref(2);

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
const dialogLocationVisible = ref(false);
const selectedProduct = ref();
const onRowSelect = (event) => {
  console.log(event.data.path);
};
const filters = ref({
  global: { value: null, matchMode: FilterMatchMode.CONTAINS }
});

// 监听后端事件
listen('current_file', (event) => {
  if (dialogLocationVisible.value) {
    location_info.value = event.payload;
  } else {
    info.value = event.payload;
  }
});
// 监听后端事件
listen('location_file', (event) => {
  location_info.value = event.payload;
});

function openLocation() {
  dialogimgsVisible.value = false;
  dialogLocationVisible.value = true;
  const ptemp = [];
  for (const product of products.value) {
    for (const img of imgs.value) {
      if (img.path === product) {
        ptemp.push(img);
        break;
      }
    }
  }
  locationImgs.value = ptemp;
  console.log(locationImgs.value);
}
async function runcmd() {
  console.log(str.value);
  let res = await invoke('runcmd', { cmdstr: str.value });
  console.log(res);
}
//加载目录，先将不认识的格式转成jpg，在使用exiftool工具进行读取。
const convertImages = async () => {
  const directory = await open({
    multiple: false,
    directory: true,
  });
  if (directory && directory.length > 0) {
    isLoading.value = true;
    try {
      console.log('转换开始');
      await invoke('convert_image_with_metadata', { dir: directory });
      info.value = '所有图片转换完成！';
    } catch (error) {
      info.value = `转换出错: ${error}`;
    } finally {
      setTimeout(() => {
        console.log('转换结束');
        info.value = "转换结束";
        info.value = "";
      }, 100);

      try {
        if (directory && directory.length > 0) {
          //先转换一次
          console.log("开始加载地址信息");
          info.value = "开始加载地址信息";
          const start = performance.now();
          let block = false;
          let tls = await invoke('load_dir_imgs', { path: directory, scope: 0 });
          console.log(tls);
          const end1 = performance.now();
          console.log(`加载地址信息 运行时间: ${end1 - start}ms`);
          if (tls && tls.length > 0) {
            //批量修改
            updatels.value = [];
            //批量添加
            insertls.value = [];
            if (imgs.value && imgs.value.length > 0) {
              for (let k = 0; k < tls.length; k++) {
                let b = false;
                for (let p = 0; p < imgs.value.length; p++) {
                  if (tls[k].name === imgs.value[p].name) {
                    let pk = 0;
                    if (tls[k].path !== imgs.value[p].path) {
                      imgs.value[p].path = tls[k].path;
                      pk++;
                    }
                    if (tls[k].lat !== imgs.value[p].lat) {
                      imgs.value[p].lat = tls[k].lat;
                      pk++;
                    }
                    if (tls[k].lng !== imgs.value[p].lng) {
                      imgs.value[p].lng = tls[k].lng;
                      pk++;
                    }
                    if (pk > 0) {
                      updatels.value.push(imgs.value[p]);
                    }
                    b = true;
                    break;
                  }
                }
                console.log(b);
                if (!b) {
                  insertls.value.push(tls[k]);
                }
              }
            } else {
              insertls.value = tls;
              console.log('批量插入全部' + tls.length);
            }
            if (updatels && updatels.value.length > 0) {
              block = true;
              invoke('update_paths', { imgs: updatels.value })
                .then(() => {
                  block = false;
                  console.log('批量修改完成:' + updatels.value.length);
                })
                .catch((error) => {
                  block = false;
                  console.error("批量修改失败", error);
                });
            }

            const batchSize = 10; // 每次处理的批大小
            const totalBatches = Math.ceil(insertls.value.length / batchSize); // 计算总批次数
            for (let batchIndex = 0; batchIndex < totalBatches; batchIndex++) {
              let lrr = []; // 用于存储当前批次的点
              const start = batchIndex * batchSize; // 当前批次的起始索引
              const end = Math.min(start + batchSize, insertls.value.length); // 当前批次的结束索引

              for (let i = start; i < end; i++) {
                console.log(insertls.value[i]);
                if (insertls.value[i].lat && insertls.value[i].lng) {
                  //(x,y) 这里按照api的意思，x是相对来说比较大的那个
                  if (insertls.value[i].lng >= insertls.value[i].lat) {
                    lrr.push(new BMapGL.Point(insertls.value[i].lng, insertls.value[i].lat));
                  } else {
                    lrr.push(new BMapGL.Point(insertls.value[i].lat, insertls.value[i].lng));
                  }
                }
                console.log(insertls.value[i]);
              }

              if (lrr.length > 0) {
                console.log('转换一次 数量' + lrr.length);
                convertor.translate(lrr, COORDINATES_WGS84, COORDINATES_BD09, function (data) {
                  if (data.status === 0) {
                    // console.log(data.points);
                    for (let i = 0; i < data.points.length; i++) {
                      insertls.value[start + i].lat = data.points[i].lat;
                      insertls.value[start + i].lng = data.points[i].lng;
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
          info.value = "开始插入数据";
          console.log(insertls.value);
          invoke('insert_imgs', { imgs: insertls.value })
            .then(() => {
              while (block) {
                sleep(1000);
                console.log('睡眠1s等待修改完成');
              }
              sleep(100);
              // 插入完成后刷新页面
              console.log('刷新页面');
              location.reload();
            })
            .catch((error) => {
              console.error("插入失败", error);
            });
          const end2 = performance.now();
          console.log('图片数量' + insertls.value.length);
          console.log(`插入数据 运行时间: ${end2 - end1}ms`);
        }
      } catch (error) {
        isLoading.value = false;
        console.log(error);
        message(JSON.stringify(error, null, 2), { title: 'Tauri', kind: 'getdirpath error' });
      }
      isLoading.value = false;
    }
  }

};

//读取一个目录的还没有定位的文件
async function load_dir_nogpsimgs() {
  const directory = await open({
    multiple: false,
    directory: true,
  });
  if (directory && directory.length > 0) {
    isLoading.value = true;//location_file
    try {
      console.log(directory);
      locationImgs.value = [];
      locationImgs.value = await invoke('load_dir_nogpsimgs', { path: directory });
      console.log(locationImgs.value);
    } catch (error) {
      console.log(error);
    }
    isLoading.value = false;
  }
  location_info.value = "设置定位";
}
//设置一个文件的定位
async function load_location_img() {
  const directory = await open({
    multiple: false,
    directory: true
  });
  if (directory && directory.length > 0) {
    isLoading.value = true;//location_file
    try {
      locationImgs.value = []; // 先清空数组
      locationImgs.value = await invoke('load_dir_imgs', { path: directory, scope: 1 });
      console.log(locationImgs.value);
      if (locationImgs.value.length > 0 && locationImgs.value[0].lat && locationImgs.value[0].lng) {
        let p = new BMapGL.Point(locationImgs.value[0].lng, locationImgs.value[0].lat);
        map_location.centerAndZoom(p, 12);
        const richMarker = new window.BMapGL.Marker(p);
        map_location.addOverlay(richMarker);
      }
    } catch (error) {
      console.log(error);
    }
    isLoading.value = false;
  }
  location_info.value = "设置定位";
}

function sleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

// 处理分页变化事件
const onPageChange = (event) => {
  currentPage.value = event.page;
  if (currentPage.value > 0 && locationImgs.value[currentPage.value].lat && locationImgs.value[currentPage.value].lng) {
    let p = new BMapGL.Point(locationImgs.value[currentPage.value].lng, locationImgs.value[currentPage.value].lat);
    map_location.centerAndZoom(p, 12);
    const richMarker = new window.BMapGL.Marker(p);
    map_location.addOverlay(richMarker);
  }
};

async function load_file_img() {
  try {
    const file = await open({
      multiple: false,
      directory: false,
    });
    if (file && file.length > 0) {
      // let tls = await invoke('load_file_img', { path: file });
      console.log(file);
      let tls = await invoke('run_command', { command: file });
      // console.log(tls);
      message(tls, { title: '读取信息', kind: '读取信息' });
    }
  } catch (error) {
    message(error, { title: '读取信息出错', kind: '读取信息出错' });
    console.log(error);
  }
}

function showImgPoint() {
  sessionStorage.setItem('redimg', 1);
  location.reload();
}

function showRedPoint() {
  sessionStorage.setItem('redimg', 0);
  location.reload();
}

async function get_all() {
  try {
    let res = await invoke('query_all');
    if (res && res.length > 0) {
      console.log(res);
      imgs.value = res;
      loading.value = false;
      load();
    }
  } catch (error) {
    console.log(error);
    //message(JSON.stringify(error, null, 2), { title: 'Tauri', kind: 'get_all error' });
  }
}
function load() {
  try {
    var currentZoom = map.getZoom(); // 获取当前缩放级别
    ratio.value = currentZoom;
    let gridSize = getGridSize(currentZoom);
    const value = sessionStorage.getItem('redimg');
    if (value && value === '0') {
      console.log('红点' + value);
      if (gridsizelast.value > gridSize) {
        maxlevel.value = currentZoom;
        gridsizelast.value = gridSize;
        map.clearOverlays();
        if (imgs.value.length > 0) {
          let clusters = allot(gridSize, imgs.value);
          reloadPoint(map, clusters);
        }
      }
      if (currentZoom > maxlevel.value) {
      }
    } else {
      console.log('图片' + value);
      // if (gridsizelast.value !== gridSize) {
      gridsizelast.value = gridSize;
      map.clearOverlays();
      if (imgs.value.length > 0) {
        //按照页面显示的地图范围实时的展示包含的图片,为了节省加载的资源
        // 获取地图当前视野范围的边界
        var bounds = map.getBounds();
        let ps = [];
        // 遍历坐标点，判断是否在地图视野范围内
        for (var i = 0; i < imgs.value.length; i++) {
          var point = new BMapGL.Point(imgs.value[i].lng, imgs.value[i].lat);
          if (bounds.containsPoint(point)) {
            ps.push(imgs.value[i]);
          }
        }
        console.log("当前页面存在定位数量:" + ps.length);
        let clusters = allot(gridSize, ps);
        reloadMarker(map, clusters);
      }
      // }
    }
  } catch (error) {
    console.log(error);
  }
}


function showImg(i) {
  products.value = clusters[i].arr;
  dialogimgsVisible.value = true;
}
window.showImg = showImg;

//红点
function reloadPoint(map, clusters) {
  for (let i = 0; i < clusters.length; i++) {
    const point = new window.BMapGL.Point(clusters[i].lng, clusters[i].lat);
    const richMarker = new window.BMapGL.Marker(point);
    map.addOverlay(richMarker);
    // 点标记添加点击事件
    richMarker.addEventListener('click', function () {
      showImg(i); // 开启信息窗口
    });
    let offx = -8;
    if (clusters[i].arr.length < 10) {
      offx = -2;
    } else if (clusters[i].arr.length < 100) {
      offx = -6;
    } else {
      offx = -10;
    }
    var opts = {
      position: point, // 指定文本标注所在的地理位置
      offset: new BMapGL.Size(offx, -30) // 设置文本偏移量
    };
    // 创建文本标注对象
    var label = new BMapGL.Label(clusters[i].arr.length, opts);
    // 自定义文本标注样式
    label.setStyle({
      color: '#ffffff',
      backgroundColor: "rgba(0, 0, 0, 0)",
      borderColor: "rgba(0, 0, 0, 0)",
      border: "none",
      borderRadius: '5px',
      borderColor: '#ccc',
      padding: '0px',
      fontSize: '8px',
      fontWeight: "bold",
      height: '30px',
      lineHeight: '30px',
      fontFamily: '微软雅黑'
    });
    map.addOverlay(label);
    // console.log('添加一个标记');
  }
  count.value = imgs.value.length;
}

//图片
function reloadMarker(map, clusters) {
  try {
    // 创建 RichMarker 并添加到地图上
    for (let i = 0; i < clusters.length; i++) {
      var badgeContent = clusters[i].arr.length + ''; // 将数字转换为字符串
      // 根据数字长度动态调整徽章的宽度和字体大小
      var badgeWidth = Math.max(20, 15 + badgeContent.length * 4); // 最小宽度为20px，每多一个字符增加12px
      var badgeFontSize = 10; //Math.min(10, 9 + 12 / badgeContent.length); // 字体大小，最多12px，根据字符长度动态调整
      var imgurl = 'http://asset.localhost/' + getNarrowImg(clusters[i].path);
      var htm1 =
        "<div id='overLay' style='width:93px;height:116px; background:url(./back.png) left top no-repeat;position: absolute;'>" +
        "<div class='image-container'>" +
        "<img onclick='showImg(\"" + i + "\")'  style='margin-left:9px;margin-top: 8px;width:75px;height:75px;object-fit: cover;' src='" + imgurl + "'/>" +
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
      // console.log('添加一个标记');
    }
    count.value = imgs.value.length;
  } catch (error) {
    console.log(error);
  }
}

function getNarrowImg(filePath) {
  // 将路径按斜杠分割成数组
  var pathParts = filePath.split('\\');
  // 获取文件名（最后一个元素）
  var fileName = pathParts.pop();
  //去掉ToJpg目录
  pathParts.pop();
  // 在倒数第二个位置插入 'narrow' 目录
  pathParts.push('narrow');
  // 再将文件名添加回去
  pathParts.push(fileName);
  // 将数组重新拼接成路径字符串
  return pathParts.join('\\');
}
// 定义键盘事件处理函数
const handleKeyDown = (event) => {
  // 阻止按键的默认行为
  // 判断是否按下 F5、F12 或者 Ctrl + R
  if (event.key === 'F5' || event.key === 'F12' || (event.ctrlKey && event.key === 'r')) {
    // 阻止按键的默认行为
    event.preventDefault();
  }
};

// 组件卸载时移除事件监听
onUnmounted(() => {
  window.removeEventListener('keydown', handleKeyDown);
});

function selLocation() {
  // 将地址解析结果显示在地图上，并调整地图视野
  if (pathinfo.value && pathinfo.value.length > 0) {
    myGeo.getPoint(pathinfo.value, function (point) {
      if (point) {
        map_location.centerAndZoom(point, 16);
        map_location.addOverlay(new BMapGL.Marker(point, {
          title: pathinfo.value
        }))
      } else {
        alert('您选择的地址没有解析到结果！');
      }
    }, '')
  }
}

function closeMap() {
  location.reload();
}
const initMap = () => {
  try {
    console.log("开始加载定位地图");
    const mapLocation = document.getElementById('map-location');
    map_location = new BMapGL.Map(mapLocation);
    map_location.centerAndZoom(new BMapGL.Point(116.40394873334277, 39.9133216708228), 6);
    map_location.enableScrollWheelZoom();
    map_location.enablePinchToZoom();
    // 添加比例尺控件
    var scaleCtrl2 = new BMapGL.ScaleControl();
    map_location.addControl(scaleCtrl2);
    //点击事件
    map_location.addEventListener('click', function (e) {
      map_location.clearOverlays();
      const point = new window.BMapGL.Point(e.latlng.lng, e.latlng.lat);
      const richMarker = new window.BMapGL.Marker(point);
      map_location.addOverlay(richMarker);
      //获取当前的表格数据
      if (locationImgs.value.length > currentPage.value) {
        confirm.require({
          message: '确定设置图片 ' + locationImgs.value[currentPage.value].name + ' 的定位?',
          header: '确认',
          icon: 'pi pi-exclamation-triangle',
          rejectProps: {
            label: '取消',
            severity: 'secondary',
            outlined: true
          },
          acceptProps: {
            label: '确认'
          },
          accept: async () => {
            console.log("确认了");
            isLoading.value = true;
            try {
              let tls = await invoke('set_location', { filepath: locationImgs.value[currentPage.value].path, latlng: e.latlng.lat + "," + e.latlng.lng });
              let f = imgs.value.find(img => img.name === locationImgs.value[currentPage.value].name) || null;
              if (f === null) {
                console.log("添加一个记录");
                locationImgs.value[currentPage.value].lat = e.latlng.lat;
                locationImgs.value[currentPage.value].lng = e.latlng.lng;
                console.log(locationImgs.value[currentPage.value]);
                let is = [];
                is.push(locationImgs.value[currentPage.value]);
                invoke('insert_imgs', { imgs: is })
                  .then(() => {
                    console.log("插入成功");
                  })
                  .catch((error) => {
                    console.error("插入失败", error);
                  });
              } else {
                console.log("修改一条记录");
                let us = [];
                f.path = locationImgs.value[currentPage.value].path;
                f.lat = e.latlng.lat;
                f.lng = e.latlng.lng;
                console.log(f);
                us.push(f);
                invoke('update_paths', { imgs: us })
                  .then(() => {
                    console.log('修改完成');
                  })
                  .catch((error) => {
                    console.error("修改失败", error);
                  });
              }
              message(tls, { title: '设置定位', kind: '设置定位' });
            } catch (error2) {
              message(error2, { title: '设置定位出错', kind: '设置定位出错' });
              console.log(error2);
            } finally {
              // 不管成功还是失败，都移除当前图片
              if (locationImgs.value.length > currentPage.value) {
                locationImgs.value.splice(currentPage.value, 1);
              }
            }
            isLoading.value = false;
          }
        });
      } else {
        message("没有图片可设置", { title: '设置定位', kind: '设置定位' });
      }
    });
    console.log("加载定位地图结束");
  } catch (error) {
    console.log(error);
  }
}
onMounted(async () => {
  try {
    window.addEventListener('keydown', handleKeyDown);
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
    // 监听缩放事件
    map.addEventListener("zoomend", function () {
      load();
    });
    // 监听地图拖动结束事件
    map.addEventListener("dragend", function () {
      load();
    });
    get_all();
  } catch (error) {
    console.error('Failed to initialize map or RichMarker:', error);
    // message(JSON.stringify(error, null, 2), { title: 'Tauri', kind: 'error' });
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
    return 0.000001;//10
  }
  return 1;
}

//根据级别分配坐标集合
function allot(gridSize, ps) {
  clusters = [];
  // 将点分配到网格
  ps.forEach(point => {
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
  console.log("当前分配了集合数量:" + clusters.length);
  return Object.values(clusters);
}

</script>

<style>
#map-container {
  width: 100%;
  height: 92vh;
  /* 使用 vh 单位使地图容器占满整个视口高度 */
  margin: 0;
  padding: 0;
}

#map-location {
  width: 100%;
  height: 85vh;
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

.container {
  display: flex;
  /* 使用 Flexbox 布局 */
  justify-content: space-between;
  /* 两端对齐 */
  align-items: center;
  /* 垂直居中 */
  width: 100%;
  /* 占满父容器宽度 */
}

.left {
  display: flex;
  /* 左侧内容也使用 Flexbox */
  align-items: center;
  /* 垂直居中 */
}

.right {
  display: flex;
  /* 右侧内容也使用 Flexbox */
  align-items: center;
  /* 垂直居中 */
}

/* 对话框内容容器，使用 flexbox 布局 */
.dialog-content-container {
  display: flex;
  height: 100%;
}

/* 左边部分，固定宽度 500px */
.left-section {
  width: 400px;
  padding: 1rem;
  border: 1px solid #ccc;
  /* 添加边框 */
  box-sizing: border-box;
  /* 使边框包含在宽度内 */
}

/* 右边部分，占据剩余宽度 */
.right-section {
  flex-grow: 1;
  padding: 1rem;
  border: 1px solid #ccc;
  /* 添加边框 */
  box-sizing: border-box;
  /* 使边框包含在宽度内 */
}

/* 表格容器，设置固定高度和滚动条 */
.table-container {
  height: 98%;
  /* 设置表格容器的高度，可根据需要调整 */
  overflow-y: auto;
  /* 当内容超出高度时显示垂直滚动条 */
}

/* 设置列内容居中显示 */
.column-content {
  text-align: center;
}

.column-content img {
  margin: 0 auto;
  display: block;
}
</style>