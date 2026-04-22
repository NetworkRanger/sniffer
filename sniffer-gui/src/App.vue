<script setup lang="ts">
import {onMounted, onUnmounted, ref} from "vue";
import {useI18n} from "vue-i18n";
import {invoke} from "@tauri-apps/api/core";
import {filesize} from "filesize";

interface NetworkStats {
  upload_speed: number;
  download_speed: number;
  total_bytes_sent: number;
  total_bytes_recv: number;
}

onMounted(async () => {
  const {locale} = useI18n();
  locale.value = "zh";
  loadStats();
  timer = window.setInterval(loadStats, 1000);
});

onUnmounted(() => { if (timer) clearInterval(timer); });

let timer: number | null = null;
const stats = ref<NetworkStats>({ upload_speed: 0, download_speed: 0, total_bytes_sent: 0, total_bytes_recv: 0 });

async function loadStats() {
  try {
    stats.value = await invoke<NetworkStats>("get_network_stats");
  } catch {}
}

const fmtSpeed = (n: number) => {
  const s = filesize(n, {base: 2, standard: "jedec", round: 2, output: "object"}) as any;
  return {val: s.value, unit: s.symbol.replace('B', '') + 'B/s'};
};
const fmtTotal = (n: number) => {
  const s = filesize(n, {base: 2, standard: "jedec", round: 1, output: "object"}) as any;
  return {val: s.value, unit: s.symbol};
};
</script>

<template>
  <el-container style="height: 100vh;">
    <el-aside width="100px" style="background: white; height: 100vh; min-width: 80px; display: flex; flex-direction: column;">
      <el-menu style="flex: 1;" router>
        <el-menu-item index="/index">首页</el-menu-item>
        <el-menu-item index="/connection">连接</el-menu-item>
        <el-menu-item index="/log">日志</el-menu-item>
      </el-menu>
      <div class="net-stats">
        <div class="net-stats__divider"></div>
        <div class="net-stats__row">
          <span class="net-stats__icon up">↑</span>
          <span class="net-stats__val up">{{ fmtSpeed(stats.upload_speed).val }}</span>
          <span class="net-stats__unit">{{ fmtSpeed(stats.upload_speed).unit }}</span>
        </div>
        <div class="net-stats__row">
          <span class="net-stats__icon down">↓</span>
          <span class="net-stats__val down">{{ fmtSpeed(stats.download_speed).val }}</span>
          <span class="net-stats__unit">{{ fmtSpeed(stats.download_speed).unit }}</span>
        </div>
        <div class="net-stats__row">
          <span class="net-stats__icon total">↑</span>
          <span class="net-stats__val total">{{ fmtTotal(stats.total_bytes_sent).val }}</span>
          <span class="net-stats__unit">{{ fmtTotal(stats.total_bytes_sent).unit }}</span>
        </div>
        <div class="net-stats__row">
          <span class="net-stats__icon total">↓</span>
          <span class="net-stats__val total">{{ fmtTotal(stats.total_bytes_recv).val }}</span>
          <span class="net-stats__unit">{{ fmtTotal(stats.total_bytes_recv).unit }}</span>
        </div>
      </div>
    </el-aside>
    <el-main style="padding: 0; flex: 1; overflow: hidden;">
      <router-view></router-view>
    </el-main>
  </el-container>
</template>
<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

.el-header, .el-footer {
  background-color: #B3C0D1;
  color: #333;
  text-align: center;
  line-height: 60px;
}

.el-aside {
  background-color: #fff;
  color: #333;
  text-align: center;
  line-height: normal;
}

.el-aside .el-menu {
  border-right: none;
}

.el-aside .el-menu-item {
  padding: 0 !important;
  justify-content: center;
  font-size: 13px;
  height: 48px;
  line-height: 48px;
}

.net-stats {
  padding: 0 8px 12px;
  font-size: 12px;
}

.net-stats__divider {
  height: 1px;
  background: #e4e7ed;
  margin-bottom: 8px;
}

.net-stats__row {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px 0;
}

.net-stats__icon {
  width: 14px;
  text-align: center;
  font-size: 13px;
  flex-shrink: 0;
}

.net-stats__icon.up   { color: #f97316; }
.net-stats__icon.down { color: #3b82f6; }
.net-stats__icon.total { color: #374151; font-size: 11px; }

.net-stats__val {
  flex: 1;
  text-align: right;
  font-weight: 600;
  font-size: 12px;
}

.net-stats__val.up    { color: #f97316; }
.net-stats__val.down  { color: #3b82f6; }
.net-stats__val.total { color: #1f2937; }

.net-stats__unit {
  color: #9ca3af;
  font-size: 11px;
  white-space: nowrap;
}

.el-main {
  background-color: #fff;
  color: #333;
}

body > .el-container {
  margin: 0;
  padding: 0;
}

body, html {
  margin: 0;
  padding: 0;
  overflow: hidden;
}

a {
  font-weight: 500;
  color: #646cff;
  text-decoration: inherit;
}

a:hover {
  color: #535bf2;
}

h1 {
  text-align: center;
}

input:not(.el-input__inner),
button:not(.el-button) {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

button {
  cursor: pointer;
}

button:hover {
  border-color: #396cd8;
}
button:active {
  border-color: #396cd8;
  background-color: #e8e8e8;
}

input:not(.el-input__inner),
button:not(.el-button) {
  outline: none;
}
</style>