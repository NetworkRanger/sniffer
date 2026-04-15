<script setup lang="ts">
import {computed, onMounted, onUnmounted, ref, shallowRef} from "vue";
import {invoke} from "@tauri-apps/api/core";
import type {Connection} from "../types/connection";
import {filesize} from "filesize";
import * as echarts from "echarts/core";
import {LineChart} from "echarts/charts";
import {GridComponent, TooltipComponent} from "echarts/components";
import {CanvasRenderer} from "echarts/renderers";

echarts.use([LineChart, GridComponent, TooltipComponent, CanvasRenderer]);

const connections = ref<Connection[]>([]);
let timer: number | null = null;
const chartEl = ref<HTMLDivElement>();
const chart = shallowRef<echarts.ECharts>();

// rolling 10-min history (600 points at 1s interval)
const MAX_POINTS = 600;
const uploadHistory = ref<number[]>([]);
const downloadHistory = ref<number[]>([]);
const timeLabels = ref<string[]>([]);

const bytesize = (n: number) =>
    filesize(n, {base: 2, standard: "jedec", round: 2}).replace(" ", "");

const speedFmt = (n: number) => {
  const r = filesize(n, {base: 2, standard: "jedec", round: 2, output: "object"}) as any;
  return `${r.value} ${r.symbol.replace("B", "")}B/s`;
};

const uploadSpeed = computed(() => connections.value.reduce((s, c) => s + c.upload_speed, 0));
const downloadSpeed = computed(() => connections.value.reduce((s, c) => s + c.download_speed, 0));
const totalUpload = computed(() => connections.value.reduce((s, c) => s + c.bytes_sent, 0));
const totalDownload = computed(() => connections.value.reduce((s, c) => s + c.bytes_recv, 0));
const activeCount = computed(() => connections.value.filter(c => !c.isDelted && c.status !== 'CLOSED').length);
const kernelBytes = computed(() => {
  return connections.value
      .filter(c => c.process_connection?.pid === null)
      .reduce((s, c) => s + c.bytes_sent + c.bytes_recv, 0);
});

// IP 信息
const ipInfo = ref({
  country: 'Hong Kong', ip: '58.152.22.15', asn: 'AS4760',
  isp: 'HKT Limited', org: 'HKT Limited', location: 'Unknown',
  timezone: 'Asia/Hong_Kong', footer: 'HK, 114.17, 22.26',
});
const ipRefreshCountdown = ref(300);

async function refreshIp() {
  try {
    const r = await fetch('https://ipapi.co/json/');
    const d = await r.json();
    ipInfo.value = {
      country: d.country_name ?? '-', ip: d.ip ?? '-', asn: d.asn ?? '-',
      isp: d.org ?? '-', org: d.org ?? '-', location: d.city ?? 'Unknown',
      timezone: d.timezone ?? '-', footer: `${d.country_code}, ${d.latitude}, ${d.longitude}`,
    };
  } catch {}
  ipRefreshCountdown.value = 300;
}

// 系统信息
const sysInfo = ref({
  os: navigator.userAgent.includes('Win') ? 'Windows' : navigator.userAgent.includes('Mac') ? 'macOS' : 'Linux',
  autostart: '未启用', mode: '服务模式',
  lastCheck: new Date().toLocaleString(),
  version: 'v2.4.7',
});

function pushHistory() {
  const now = new Date();
  const label = `${String(now.getHours()).padStart(2,'0')}:${String(now.getMinutes()).padStart(2,'0')}`;
  uploadHistory.value.push(uploadSpeed.value);
  downloadHistory.value.push(downloadSpeed.value);
  timeLabels.value.push(label);
  if (uploadHistory.value.length > MAX_POINTS) {
    uploadHistory.value.shift();
    downloadHistory.value.shift();
    timeLabels.value.shift();
  }
  updateChart();
}

function updateChart() {
  if (!chart.value) return;
  chart.value.setOption({
    xAxis: {data: timeLabels.value},
    series: [
      {name: '上传', data: uploadHistory.value},
      {name: '下载', data: downloadHistory.value},
    ],
  });
}

async function load() {
  try {
    connections.value = await invoke<Connection[]>("get_connections");
  } catch {
    // mock in browser
    connections.value = [];
  }
  pushHistory();
}

onMounted(() => {
  chart.value = echarts.init(chartEl.value!);
  chart.value.setOption({
    grid: {top: 24, bottom: 28, left: 48, right: 60},
    xAxis: {
      type: 'category', data: [],
      axisLine: {lineStyle: {color: '#e4e7ed'}},
      axisLabel: {color: '#9ca3af', fontSize: 11},
      splitLine: {show: false},
    },
    yAxis: {
      type: 'value',
      axisLabel: {
        color: '#9ca3af', fontSize: 11,
        formatter: (v: number) => v === 0 ? '0' : filesize(v, {base:2, standard:'jedec', round:0}).replace(' ',''),
      },
      splitLine: {lineStyle: {color: '#f3f4f6'}},
    },
    tooltip: {trigger: 'axis', formatter: (p: any[]) =>
        p.map(i => `${i.seriesName}: ${speedFmt(i.value)}`).join('<br/>')},
    series: [
      {name:'上传', type:'line', data:[], smooth:true, symbol:'none',
        lineStyle:{color:'#f97316', width:1.5}, areaStyle:{color:'rgba(249,115,22,0.06)'}},
      {name:'下载', type:'line', data:[], smooth:true, symbol:'none',
        lineStyle:{color:'#3b82f6', width:1.5}, areaStyle:{color:'rgba(59,130,246,0.06)'}},
    ],
  });

  const ro = new ResizeObserver(() => chart.value?.resize());
  ro.observe(chartEl.value!);

  refreshIp();
  load();
  timer = window.setInterval(() => {
    load();
    ipRefreshCountdown.value--;
    if (ipRefreshCountdown.value <= 0) refreshIp();
  }, 1000);
});

onUnmounted(() => {
  if (timer) clearInterval(timer);
  chart.value?.dispose();
});
</script>

<template>
  <div class="page">
    <div class="page-header">
      <span class="page-title">首页</span>
    </div>

    <div class="section-card">
      <div class="section-header">
        <span class="section-icon">📶</span>
        <span class="section-title">流量统计</span>
      </div>

      <div class="chart-wrap">
      <div class="chart-toolbar">
        <span class="chart-tag">10 分钟</span>
        <span class="chart-legend up">上传</span>
        <span class="chart-legend down">下载</span>
      </div>
      <div ref="chartEl" class="chart"></div>
    </div>

    <div class="stats-grid">
      <div class="stat-card orange-light">
        <div class="stat-icon-wrap orange">↑</div>
        <div class="stat-body">
          <div class="stat-label">上传速度</div>
          <div class="stat-value">{{ speedFmt(uploadSpeed) }}</div>
        </div>
      </div>
      <div class="stat-card blue-light">
        <div class="stat-icon-wrap blue">↓</div>
        <div class="stat-body">
          <div class="stat-label">下载速度</div>
          <div class="stat-value">{{ speedFmt(downloadSpeed) }}</div>
        </div>
      </div>
      <div class="stat-card green-light">
        <div class="stat-icon-wrap green">⇄</div>
        <div class="stat-body">
          <div class="stat-label">活跃连接</div>
          <div class="stat-value">{{ activeCount }}</div>
        </div>
      </div>
      <div class="stat-card orange-light">
        <div class="stat-icon-wrap orange-soft">↑</div>
        <div class="stat-body">
          <div class="stat-label">上传量</div>
          <div class="stat-value">{{ bytesize(totalUpload) }}</div>
        </div>
      </div>
      <div class="stat-card blue-light">
        <div class="stat-icon-wrap blue-soft">↓</div>
        <div class="stat-body">
          <div class="stat-label">下载量</div>
          <div class="stat-value">{{ bytesize(totalDownload) }}</div>
        </div>
      </div>
      <div class="stat-card red-light">
        <div class="stat-icon-wrap red">⊕</div>
        <div class="stat-body">
          <div class="stat-label">内核占用</div>
          <div class="stat-value">{{ bytesize(kernelBytes) }}</div>
        </div>
      </div>
    </div>
  </div>

  <!-- IP 信息 + 系统信息 横向排列 -->
  <div class="info-row">
    <!-- IP 信息 -->
    <div class="section-card info-card">
      <div class="section-header">
        <span class="section-icon blue-icon">📍</span>
        <span class="section-title">IP 信息</span>
        <span class="section-action" @click="refreshIp" title="刷新">↻</span>
      </div>
      <div class="ip-body">
        <div class="ip-left">
          <div class="ip-region">🇭🇰 {{ ipInfo.country }}</div>
          <div class="ip-addr">IP: <span>{{ ipInfo.ip }}</span></div>
          <div class="ip-asn">自治域: {{ ipInfo.asn }}</div>
          <div class="ip-refresh">自动刷新: {{ ipRefreshCountdown }}s</div>
        </div>
        <div class="ip-right">
          <div class="ip-row"><span class="ip-key">服务商:</span><span class="ip-val">{{ ipInfo.isp }}</span></div>
          <div class="ip-row"><span class="ip-key">组织:</span><span class="ip-val">{{ ipInfo.org }}</span></div>
          <div class="ip-row"><span class="ip-key">位置:</span><span class="ip-val">{{ ipInfo.location }}</span></div>
          <div class="ip-row"><span class="ip-key">时区:</span><span class="ip-val">{{ ipInfo.timezone }}</span></div>
        </div>
      </div>
      <div class="ip-footer">{{ ipInfo.footer }}</div>
    </div>

    <!-- 系统信息 -->
    <div class="section-card info-card">
      <div class="section-header">
        <span class="section-icon blue-icon">ℹ</span>
        <span class="section-title">系统信息</span>
        <span class="section-action" title="设置">⚙</span>
      </div>
      <div class="sys-rows">
        <div class="sys-row">
          <span class="sys-key">操作系统信息</span>
          <span class="sys-val">{{ sysInfo.os }}</span>
        </div>
        <div class="sys-row">
          <span class="sys-key">最后检查更新</span>
          <span class="sys-val link">{{ sysInfo.lastCheck }}</span>
        </div>
        <div class="sys-row">
          <span class="sys-key">版本</span>
          <span class="sys-val">{{ sysInfo.version }}</span>
        </div>
      </div>
    </div>
  </div>
</div>
</template>

<style scoped>
.page {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow-y: auto;
  padding: 0 12px 12px;
  background: #fff;
  box-sizing: border-box;
  gap: 10px;
}

.page-header {
  display: flex;
  align-items: center;
  padding: 10px 0 4px;
  border-bottom: 1px solid #f3f4f6;
  flex-shrink: 0;
}

.header-icon { font-size: 20px; }

.chart-wrap {
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  padding: 8px;
}

.chart-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}

.chart-tag {
  font-size: 12px;
  background: #f3f4f6;
  border: 1px solid #e4e7ed;
  border-radius: 4px;
  padding: 1px 8px;
  color: #374151;
}

.chart-legend { font-size: 12px; font-weight: 600; }
.chart-legend.up   { color: #f97316; margin-left: auto; }
.chart-legend.down { color: #3b82f6; }

.chart { height: 140px; width: 100%; }

.stats-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 8px;
}

.stat-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  border-radius: 8px;
  border: 1px solid transparent;
}

.orange-light { background: #fff7ed; border-color: #fed7aa; }
.blue-light   { background: #eff6ff; border-color: #bfdbfe; }
.green-light  { background: #f0fdf4; border-color: #bbf7d0; }
.red-light    { background: #fff1f2; border-color: #fecdd3; }

.stat-icon-wrap {
  width: 36px; height: 36px;
  border-radius: 50%;
  display: flex; align-items: center; justify-content: center;
  font-size: 16px; font-weight: 700; flex-shrink: 0;
}
.stat-icon-wrap.orange      { background: #fed7aa; color: #ea580c; }
.stat-icon-wrap.orange-soft { background: #ffedd5; color: #f97316; }
.stat-icon-wrap.blue        { background: #bfdbfe; color: #2563eb; }
.stat-icon-wrap.blue-soft   { background: #dbeafe; color: #3b82f6; }
.stat-icon-wrap.green       { background: #bbf7d0; color: #16a34a; }
.stat-icon-wrap.red         { background: #fecdd3; color: #e11d48; }

.stat-label { font-size: 12px; color: #6b7280; margin-bottom: 2px; }
.stat-value { font-size: 15px; font-weight: 700; color: #111827; }

/* section card shared */
.section-card {
  border: 1px solid #e4e7ed;
  border-radius: 8px;
  padding: 12px 14px;
  background: #fff;
}

.section-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 10px;
  padding-bottom: 8px;
  border-bottom: 1px solid #f3f4f6;
}

.section-icon { font-size: 18px; }
.blue-icon { color: #3b82f6; }

.section-title {
  font-size: 14px;
  font-weight: 600;
  color: #1f2937;
  flex: 1;
}

.section-action {
  font-size: 16px;
  color: #9ca3af;
  cursor: pointer;
}
.section-action:hover { color: #3b82f6; }

/* info row */
.info-row {
  display: flex;
  gap: 10px;
}

.info-card { flex: 1; }

/* IP card */
.ip-body {
  display: flex;
  gap: 16px;
  margin-bottom: 8px;
}

.ip-left { flex-shrink: 0; }

.ip-region { font-size: 14px; font-weight: 600; color: #1f2937; margin-bottom: 4px; }
.ip-addr   { font-size: 12px; color: #6b7280; margin-bottom: 2px; }
.ip-addr span { color: #374151; font-weight: 500; }
.ip-asn    { font-size: 12px; color: #6b7280; margin-bottom: 2px; }
.ip-refresh { font-size: 11px; color: #9ca3af; margin-top: 6px; }

.ip-right { flex: 1; }

.ip-row {
  display: flex;
  gap: 6px;
  font-size: 12px;
  margin-bottom: 3px;
}
.ip-key { color: #6b7280; flex-shrink: 0; }
.ip-val { color: #3b82f6; font-weight: 500; }

.ip-footer {
  font-size: 11px;
  color: #9ca3af;
  text-align: right;
  border-top: 1px solid #f3f4f6;
  padding-top: 6px;
}

/* sys card */
.sys-rows { display: flex; flex-direction: column; gap: 0; }

.sys-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 0;
  border-bottom: 1px solid #f9fafb;
  font-size: 13px;
}
.sys-row:last-child { border-bottom: none; }

.sys-key { color: #6b7280; }
.sys-val { color: #1f2937; font-weight: 500; }
.sys-val.mode { color: #3b82f6; }
.sys-val.link { color: #3b82f6; text-decoration: underline; cursor: pointer; }

.sys-badge {
  font-size: 12px;
  padding: 2px 10px;
  border: 1px solid #e4e7ed;
  border-radius: 12px;
  color: #6b7280;
  background: #f9fafb;
}

/* page title */
.page-title {
  font-size: 16px;
  font-weight: 600;
  color: #1f2937;
}

/* section card inside flow stats */
.section-card .chart-wrap {
  border: 1px solid #e4e7ed;
  border-radius: 6px;
  padding: 8px;
  margin-bottom: 10px;
}
</style>
