<template>
  <div>
    <div id="map-container" ref="mapContainer">
    </div>
  </div>
</template>

<script setup>
console.log('start');
import { onMounted, ref, createVNode, render } from 'vue';
import { appDataDir, appConfigDir, appLocalDataDir, audioDir, dataDir, desktopDir, documentDir, homeDir, localDataDir, join } from '@tauri-apps/api/path';
import { convertFileSrc } from '@tauri-apps/api/core';
const desktopDirPath = await desktopDir();
console.log('desktopDir', desktopDirPath);

const homeDirPath = await homeDir();
console.log('homeDir', homeDirPath);

const filePath = await join(desktopDirPath, 'back.png');
const assetUrl = convertFileSrc(filePath);

console.log('assetUrl', assetUrl);
// const video = document.getElementById('my-video');
// const source = document.createElement('source');
// source.type = 'video/mp4';
// source.src = assetUrl;
// video.appendChild(source);
// video.load();

// 假设这是你的图片数组和坐标数组
const imgarr = [
  './a1.png',
  './a2.png',
  './a3.png',
  './a4.png',
  './a4.png',
  assetUrl,
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

// 确保百度地图和扩展库已加载
if (!window.BMapGL || !window.BMapGLLib) {
  console.error("百度地图或扩展库未正确加载");
}

function showImg(uuid) {
  alert("展示图片" + uuid);
}
window.showImg = showImg;
const uuid = '999';


onMounted(async () => {
  try {

    const mapContainer = document.getElementById('map-container');
    const map = new BMapGL.Map(mapContainer);
    map.centerAndZoom(new BMapGL.Point(105.401952, 40.032704), 5);
    map.enableScrollWheelZoom();
    map.enablePinchToZoom();

    // 创建 RichMarker 并添加到地图上
    for (let i = 0; i < imgarr.length && i < points.length; i++) {
      var badgeContent = sizes[i].toString(); // 将数字转换为字符串

      // 根据数字长度动态调整徽章的宽度和字体大小
      var badgeWidth = Math.max(20, 15 + badgeContent.length * 4); // 最小宽度为20px，每多一个字符增加12px
      var badgeFontSize = 10; Math.min(10, 9 + 12 / badgeContent.length); // 字体大小，最多12px，根据字符长度动态调整
      console.log('badgeWidth', badgeWidth);
      console.log('badgeFontSize', badgeFontSize);
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
  } catch (error) {
    console.error('Failed to initialize map or RichMarker:', error);
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