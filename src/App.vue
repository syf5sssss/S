<template>
  <div>
    <div id="map-container" ref="mapContainer">
    </div>
    <div class="card flex justify-center">
      <Button label="Submit" @click="getdirpath">获取文件目录</Button>
      <Button label="Submit" @click="get_all">获取所有信息</Button>
      <Button label="Submit" @click="truncate">清空所有信息</Button>
      <Button label="Submit" @click="load">从新加载标记</Button>
    </div>
    {{ msg }}
    <h1>-------------------</h1>
    {{ msg2 }}
    <h1>--------------------</h1>
    {{ imgs }}
  </div>
</template>

<script setup>
import { onMounted, ref, createVNode, render } from 'vue';
import { appDataDir, appConfigDir, appLocalDataDir, audioDir, dataDir, desktopDir, documentDir, homeDir, localDataDir, join } from '@tauri-apps/api/path';
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { message } from '@tauri-apps/plugin-dialog';
import Button from 'primevue/button';
import { sendNotification } from '@tauri-apps/plugin-notification';

let msg = ref("");
let msg2 = ref("");
let imgs = ref([]);
let map = null;
var convertor = new BMapGL.Convertor();

async function getdirpath() {
  try {
    const file = await open({
      multiple: false,
      directory: true,
    });
    imgs.value = await invoke('load_dir_imgs', { path: file });
    if (imgs.value && imgs.value.length > 0) {
      // message(JSON.stringify(imgs.value, null, 2), { title: 'Tauri', kind: 'before error' });
      let lrr = [];
      for (let i = 0; i < imgs.value.length; i++) {
        // alert('lat:' + imgs.value[i].lat + '  ' + 'lng:' + imgs.value[i].lon);
        lrr.push(new BMapGL.Point(imgs.value[i].lat, imgs.value[i].lon));
      }
      msg.value = lrr;
      convertor.translate(lrr, COORDINATES_WGS84, COORDINATES_BD09, translateCallback);
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
      msg.value = JSON.stringify(res, null, 2);
    }
  } catch (error) {
    console.log(error);
    message(JSON.stringify(error, null, 2), { title: 'Tauri', kind: 'get_all error' });
  }
}
async function truncate() {
  try {
    let res = await invoke('truncate');
    if (res && res.length > 0) {
      msg.value = JSON.stringify(res, null, 2);
    }
  } catch (error) {
    console.log(error);
    message(JSON.stringify(error, null, 2), { title: 'Tauri', kind: 'truncate error' });
  }
}

//坐标转换完之后的回调函数
function translateCallback(data) {
  if (data.status === 0) {
    message(JSON.stringify((data.points, null, 2)), { title: 'Tauri after', kind: 'translateCallback error' });
    if (data.points && data.points.length > 0) {
      if (data.points.length === imgs.value.length) {
        for (let i = 0; i < imgs.value.length; i++) {
          imgs.value[i].lat = data.points[i].lat;
          imgs.value[i].lon = data.points[i].lng;
        }
        msg2.value = data.points;
        let res = invoke('insert_imgs', { imgs: imgs.value });
      } else {
        message("获取网络位置信息存在丢失", { title: 'Tauri translateCallback', kind: 'translateCallback error' });
      }
    }
  }
}

function load() {
  if (imgs.value && imgs.value.length > 0) {
    imgarr = [];
    points = [];
    sizes = [];

    map.clearOverlays();
    for (let i = 0; i < imgs.value.length; i++) {
      imgarr.push('http://asset.localhost/' + imgs.value[i].path);
      points.push({ lng: imgs.value[i].lon, lat: imgs.value[i].lat });
      sizes.push(1);
    }
    reloadMarker(map);
  }
}

// 假设这是你的图片数组和坐标数组
const imgarr = [
  './a1.png',
  './a2.png',
  './a3.png',
  './a4.png',
  './a4.png',
  'http://asset.localhost/D:/TEST/rt2/IMG_20250209_164108/IMG_20250209_124541.jpg'
];
const points = [
  { lng: 120, lat: 45 },
  { lng: 116, lat: 40 },
  { lng: 100, lat: 35 },
  { lng: 90, lat: 29 },
  { lng: 88, lat: 29 },
  { lng: 120, lat: 29 },
];
const sizes = [
  12, 89, 15, 2, 998, 77
];



function showImg(uuid) {
  alert("展示图片" + uuid);
}
window.showImg = showImg;
const uuid = '999';


function reloadMarker(map) {
  // 创建 RichMarker 并添加到地图上
  for (let i = 0; i < imgarr.length && i < points.length; i++) {
    var badgeContent = sizes[i].toString(); // 将数字转换为字符串
    // 根据数字长度动态调整徽章的宽度和字体大小
    var badgeWidth = Math.max(20, 15 + badgeContent.length * 4); // 最小宽度为20px，每多一个字符增加12px
    var badgeFontSize = 10; Math.min(10, 9 + 12 / badgeContent.length); // 字体大小，最多12px，根据字符长度动态调整
    var htm1 =
      "<div id='overLay' style='width:93px;height:116px; background:url(./back.png) left top no-repeat;position: absolute;'>" +
      "<div class='image-container'>" +
      "<img onclick='showImg(\"" + uuid + "\")'  style='margin-left:9px;margin-top: 8px;width:75px;height:75px;' src='" + imgarr[i] + "'/>" +
      "</div>" +
      "<div style='position: absolute; top: -8px; right: -9px; background-color: red; color: white; font-size: " + badgeFontSize + "px; width: " + badgeWidth + "px; height: " + badgeWidth + "px; line-height: " + badgeWidth + "px; text-align: center; border-radius: 50%;'>" +
      badgeContent +
      "</div>" +
      "</div>";
    const point = new window.BMapGL.Point(points[i].lng, points[i].lat);
    const richMarker = new window.BMapGLLib.RichMarker(htm1, point, {
      anchor: new window.BMapGL.Size(-47, -116),
      enableDragging: true,
    });
    map.addOverlay(richMarker);
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
    map.centerAndZoom(new BMapGL.Point(105.401952, 40.032704), 5);
    map.enableScrollWheelZoom();
    map.enablePinchToZoom();
    reloadMarker(map);
  } catch (error) {
    console.error('Failed to initialize map or RichMarker:', error);
    sendNotification({ title: 'Tauri error', body: error });
    message(JSON.stringify(error, null, 2), { title: 'Tauri', kind: 'error' });
  }
});
</script>

<style>
#map-container {
  width: 100%;
  height: 97vh;
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
</style>