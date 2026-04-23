<script setup lang="ts">
import {computed, nextTick, onMounted, onUnmounted, ref, watch} from "vue";
import {invoke} from "@tauri-apps/api/core";
import type {Connection, ProcessGroup, TreeRow} from "../types/connection";
import {filesize} from "filesize";
import ConnectionsTable from "../components/ConnectionsTable.vue";
import StatusBar from "../components/StatusBar.vue";

const tableRef = ref<InstanceType<typeof ConnectionsTable>>();

const groups = ref<ProcessGroup[]>([]);
const activeTab = ref<'active' | 'closed'>('active');
const filter = ref('');
const appProtoFilter = ref('');
const protoOptions = ['TCP', 'UDP', 'HTTP', 'HTTPS', 'H2C', 'QUIC', 'DNS'];
const capturing = ref(true); // 启动时默认抓包中

let autoRefreshTimer: number | null = null;

const bytesize = (bytes: number) =>
    filesize(bytes, {base: 2, standard: "jedec", round: 1}).replace(" ", "");

// 所有连接的扁平列表（用于统计）
const allConnections = computed(() =>
    groups.value.flatMap(g => g.connections));

const totalDownload = computed(() =>
    bytesize(allConnections.value.reduce((s, c) => s + c.bytes_recv, 0)));

const totalUpload = computed(() =>
    bytesize(allConnections.value.reduce((s, c) => s + c.bytes_sent, 0)));

const activeCount = computed(() =>
    allConnections.value.filter(c => !c.isDelted && c.status !== 'closed').length);

const closedCount = computed(() =>
    allConnections.value.filter(c => c.isDelted || c.status === 'closed').length);

const statusBar = computed(() => {
  const all = allConnections.value;
  return {
    totalConnections: all.length,
    tcpConnections: all.filter(c => c.protocol === 'TCP').length,
    udpConnections: all.filter(c => c.protocol === 'UDP').length,
    closedConnections: all.filter(c => c.status !== 'active').length,
    lastUpdate: new Date().toLocaleTimeString(),
  };
});

// 按 active/closed 过滤 group 内的 connections，再过滤掉空 group
const filteredGroups = computed(() => {
  const isActive = activeTab.value === 'active';
  return groups.value
      .map(g => {
        const conns = g.connections.filter(c => {
          const closed = c.isDelted || c.status === 'closed';
          return isActive ? !closed : closed;
        }).filter(c => {
          if (appProtoFilter.value) {
            const f = appProtoFilter.value.toUpperCase();
            const match = c.protocol.toUpperCase() === f || (c.app_protocol || '').toUpperCase() === f;
            if (!match) return false;
          }
          if (!filter.value) return true;
          const host = `${c.domain || c.remote_addr}:${c.remote_port}`;
          return host.toLowerCase().includes(filter.value.toLowerCase());
        });
        return {...g, connections: conns};
      })
      .filter(g => g.connections.length > 0);
});

// 构建 el-table tree 数据
function buildGroupRow(g: ProcessGroup): TreeRow {
  const conns = g.connections;
  const sum = (fn: (c: Connection) => number) => conns.reduce((s, c) => s + fn(c), 0);
  // 找当前网速最大的连接，用于显示协议、地址、域名、路径、持续时间
  const top = conns.reduce((best, c) => {
    const speed = c.upload_speed + c.download_speed;
    const bestSpeed = best.upload_speed + best.download_speed;
    return speed > bestSpeed ? c : best;
  }, conns[0]);
  return {
    _isGroup: true,
    id: `group-${g.pid ?? 'none'}`,
    local_addr: top?.local_addr ?? '', local_port: top?.local_port ?? 0,
    remote_addr: top?.remote_addr ?? '', remote_port: top?.remote_port ?? 0,
    protocol: top?.protocol ?? '', app_protocol: top?.app_protocol ?? '',
    domain: top?.domain ?? null, path: top?.path ?? null,
    bytes_sent: sum(c => c.bytes_sent),
    bytes_recv: sum(c => c.bytes_recv),
    packets_sent: sum(c => c.packets_sent),
    packets_recv: sum(c => c.packets_recv),
    upload_speed: sum(c => c.upload_speed),
    download_speed: sum(c => c.download_speed),
    start_time: top?.start_time ?? 0,
    start_time_us: top?.start_time_us ?? 0,
    last_active: top?.last_active ?? 0,
    status: conns.some(c => c.status === 'active') ? 'active' : 'closed',
    process_connection: conns[0]?.process_connection ?? null,
    packet_connection: null,
    children: conns,
    hasChildren: true,
  } as TreeRow;
}

// 列头排序状态
const colSortProp = ref<string | null>(null);
const colSortOrder = ref<string | null>(null);

function onTableSortChange({ prop, order }: { prop: string | null; order: string | null }) {
  colSortProp.value = prop;
  colSortOrder.value = order;
}

function getDurationMs(row: Connection): number {
  const endUs = row.last_active * 1_000_000;
  const startUs = row.start_time_us || row.start_time * 1_000_000;
  return Math.max(0, Math.floor((endUs - startUs) / 1000));
}

function getSortValue(row: TreeRow, prop: string): number | string {
  switch (prop) {
    case 'total_bytes': return row.bytes_sent + row.bytes_recv;
    case 'total_speed': return row.upload_speed + row.download_speed;
    case 'duration': return getDurationMs(row);
    case 'status': return row.status;
    default: return (row as any)[prop] ?? 0;
  }
}

const treeData = computed(() => {
  let rows = filteredGroups.value.map(buildGroupRow);
  if (colSortProp.value && colSortOrder.value) {
    const prop = colSortProp.value;
    const asc = colSortOrder.value === 'ascending' ? 1 : -1;
    rows.sort((a, b) => {
      const va = getSortValue(a, prop);
      const vb = getSortValue(b, prop);
      if (va < vb) return -1 * asc;
      if (va > vb) return 1 * asc;
      return 0;
    });
  }
  return rows;
});

const mockGroups: ProcessGroup[] = [
  {
    pid: 1234, process_name: 'Cursor Helper', kernel_name: 'Cursor Helper', icon: null,
    connections: [
      { id: 'mock-1', local_addr: '192.168.1.100', local_port: 52341, remote_addr: '140.82.114.26', remote_port: 443, protocol: 'TCP', app_protocol: 'TLS', domain: 'github.com', path: null, bytes_sent: 204800, bytes_recv: 1048576, packets_sent: 150, packets_recv: 800, upload_speed: 2048, download_speed: 10240, start_time: Math.floor(Date.now()/1000) - 300, start_time_us: (Date.now() - 300000) * 1000, last_active: Math.floor(Date.now()/1000), status: 'active', process_connection: { protocol: 'TCP', local_addr: '192.168.1.100', local_port: 52341, remote_addr: '140.82.114.26', remote_port: 443, state: 'ESTABLISHED', pid: 1234, process_name: 'Cursor Helper', kernel_name: 'Cursor Helper', icon: null, start_time: null, fill_column: '' }, packet_connection: null },
      { id: 'mock-2', local_addr: '192.168.1.100', local_port: 52342, remote_addr: '20.205.243.166', remote_port: 443, protocol: 'TCP', app_protocol: 'TLS', domain: 'api.github.com', path: '/graphql', bytes_sent: 51200, bytes_recv: 524288, packets_sent: 60, packets_recv: 400, upload_speed: 512, download_speed: 5120, start_time: Math.floor(Date.now()/1000) - 120, start_time_us: (Date.now() - 120000) * 1000, last_active: Math.floor(Date.now()/1000), status: 'active', process_connection: { protocol: 'TCP', local_addr: '192.168.1.100', local_port: 52342, remote_addr: '20.205.243.166', remote_port: 443, state: 'ESTABLISHED', pid: 1234, process_name: 'Cursor Helper', kernel_name: 'Cursor Helper', icon: null, start_time: null, fill_column: '' }, packet_connection: null },
    ],
  },
  {
    pid: 5678, process_name: 'Google Chrome', kernel_name: 'Google Chrome', icon: null,
    connections: [
      { id: 'mock-3', local_addr: '192.168.1.100', local_port: 61001, remote_addr: '142.250.80.46', remote_port: 443, protocol: 'TCP', app_protocol: 'HTTP/2', domain: 'www.google.com', path: '/search', bytes_sent: 102400, bytes_recv: 2097152, packets_sent: 200, packets_recv: 1500, upload_speed: 1024, download_speed: 20480, start_time: Math.floor(Date.now()/1000) - 600, start_time_us: (Date.now() - 600000) * 1000, last_active: Math.floor(Date.now()/1000), status: 'active', process_connection: { protocol: 'TCP', local_addr: '192.168.1.100', local_port: 61001, remote_addr: '142.250.80.46', remote_port: 443, state: 'ESTABLISHED', pid: 5678, process_name: 'Google Chrome', kernel_name: 'Google Chrome', icon: null, start_time: null, fill_column: '' }, packet_connection: null },
    ],
  },
  {
    pid: null, process_name: null, kernel_name: null, icon: null,
    connections: [
      { id: 'mock-4', local_addr: '192.168.1.100', local_port: 55555, remote_addr: '8.8.8.8', remote_port: 53, protocol: 'UDP', app_protocol: 'DNS', domain: null, path: null, bytes_sent: 512, bytes_recv: 1024, packets_sent: 4, packets_recv: 4, upload_speed: 0, download_speed: 0, start_time: Math.floor(Date.now()/1000) - 60, start_time_us: (Date.now() - 60000) * 1000, last_active: Math.floor(Date.now()/1000) - 50, status: 'closed', process_connection: null, packet_connection: null },
    ],
  },
];

async function loadConnections() {
  try {
    const result = await invoke<ProcessGroup[]>("get_grouped_connections");
    groups.value = result;
  } catch (e) {
    // mock in browser
    if (!groups.value.length) groups.value = mockGroups;
  }
}

async function loadCaptureStatus() {
  try {
    capturing.value = await invoke<boolean>("get_capture_status");
  } catch (_) {}
}

async function toggleCapture() {
  try {
    if (capturing.value) {
      await invoke("stop_capture");
    } else {
      await invoke("start_capture");
    }
    await loadCaptureStatus();
  } catch (_) {}
}

const refreshInterval = ref(3000);
const refreshRunning = ref(true);
const intervalOptions = [
  {label: '1秒',  value: 1000},
  {label: '2秒',  value: 2000},
  {label: '3秒',  value: 3000},
  {label: '5秒',  value: 5000},
  {label: '10秒', value: 10000},
];

function restartTimer() {
  if (autoRefreshTimer !== null) clearInterval(autoRefreshTimer);
  if (refreshRunning.value) {
    autoRefreshTimer = window.setInterval(loadConnections, refreshInterval.value);
  }
}

watch(refreshInterval, restartTimer);
watch(refreshRunning, restartTimer);

onMounted(async () => {
  loadConnections();
  loadCaptureStatus();
  restartTimer();
  await nextTick();
});

onUnmounted(() => {
  if (autoRefreshTimer !== null) clearInterval(autoRefreshTimer);
});
</script>

<template>
  <div class="page-wrap">
    <!-- Header -->
    <div class="page-header">
      <span class="page-title">连接</span>
      <div class="header-stats">
        <span class="stat-item">下载量: <b>{{ totalDownload }}</b></span>
        <span class="stat-item">上传量: <b>{{ totalUpload }}</b></span>
      </div>
      <div class="header-actions">
        <span class="menu-icon">&#9776;</span>
      </div>
    </div>

    <!-- Toolbar -->
    <div class="toolbar">
      <div class="tab-buttons">
        <button
            :class="['tab-btn', activeTab === 'active' ? 'tab-btn--active' : '']"
            @click="activeTab = 'active'">
          活跃 {{ activeCount }}
        </button>
        <button
            :class="['tab-btn', activeTab === 'closed' ? 'tab-btn--active' : '']"
            @click="activeTab = 'closed'">
          已关闭 {{ closedCount }}
        </button>
      </div>
      <el-select v-model="appProtoFilter" placeholder="协议筛选" clearable class="proto-select">
        <el-option v-for="p in protoOptions" :key="p" :label="p" :value="p" />
      </el-select>
      <div class="filter-wrap">
        <el-input v-model="filter" placeholder="过滤条件" clearable />
      </div>

      <!-- 抓包控制 -->
      <div class="capture-ctrl">
        <button
            :class="['capture-btn', capturing ? 'capture-btn--running' : 'capture-btn--stopped']"
            @click="toggleCapture">
          {{ capturing ? '⏹ 停止抓包' : '▶ 开始抓包' }}
        </button>
      </div>

      <!-- 自动刷新 -->
      <div class="refresh-ctrl">
        <span class="refresh-label">自动刷新:</span>
        <button
            :class="['refresh-toggle', refreshRunning ? 'refresh-toggle--running' : '']"
            @click="refreshRunning = !refreshRunning">
          {{ refreshRunning ? '停止' : '启动' }}
        </button>
        <el-dropdown trigger="click" @command="(v: number) => refreshInterval = v">
          <button class="sort-btn">
            {{ intervalOptions.find(o => o.value === refreshInterval)?.label }}
            <span class="sort-arrow">&#8963;</span>
          </button>
          <template #dropdown>
            <el-dropdown-menu>
              <el-dropdown-item
                  v-for="opt in intervalOptions"
                  :key="opt.value"
                  :command="opt.value">
                <span style="display:flex;align-items:center;gap:6px;">
                  <span style="width:14px;">{{ refreshInterval === opt.value ? '✓' : '' }}</span>
                  {{ opt.label }}
                </span>
              </el-dropdown-item>
            </el-dropdown-menu>
          </template>
        </el-dropdown>
      </div>

      <!-- 列设置 -->
      <el-popover
          v-if="tableRef"
          :visible="tableRef.showColMenu"
          placement="bottom-end"
          :width="160"
          trigger="click">
        <template #reference>
          <button class="sort-btn" @click="tableRef!.showColMenu = !tableRef!.showColMenu" title="列设置">⊞</button>
        </template>
        <div class="col-menu">
          <div v-for="col in tableRef.columns" :key="col.key" class="col-menu-item">
            <el-checkbox v-model="col.visible" :label="col.label" />
          </div>
        </div>
      </el-popover>
    </div>

    <!-- Table -->
    <div class="table-area">
      <ConnectionsTable ref="tableRef" :connections="treeData" @sort-change="onTableSortChange"/>
    </div>
    <StatusBar v-bind="statusBar" />
  </div>
</template>

<style scoped>
.page-wrap {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: #fff;
}

.table-area {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  height: 0;
}

.page-header {
  display: flex;
  align-items: center;
  padding: 6px 10px;
  border-bottom: 1px solid #e4e7ed;
  gap: 12px;
}

.page-title {
  font-size: 16px;
  font-weight: 600;
  color: #1f2937;
  margin-right: auto;
}

.header-stats {
  display: flex;
  gap: 20px;
  font-size: 13px;
  color: #374151;
}

.stat-item b {
  color: #1d4ed8;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}

.menu-icon {
  font-size: 18px;
  cursor: pointer;
  color: #374151;
}

.toolbar {
  display: flex;
  align-items: center;
  padding: 6px 10px;
  gap: 10px;
  border-bottom: 1px solid #e4e7ed;
}

.tab-buttons {
  display: flex;
  flex-shrink: 0;
}

.tab-btn {
  padding: 4px 12px;
  font-size: 13px;
  border: 1px solid #d1d5db !important;
  border-radius: 0 !important;
  background: #fff;
  cursor: pointer;
  color: #374151;
  line-height: 1.5;
  box-shadow: none;
  margin: 0;
}

.tab-btn:first-child {
  border-radius: 4px 0 0 4px !important;
}

.tab-btn:last-child {
  border-radius: 0 4px 4px 0 !important;
  border-left: none !important;
}

.tab-btn--active {
  background: #3b82f6;
  color: #fff;
  border-color: #3b82f6;
}

.filter-wrap {
  flex: 1;
}

.proto-select {
  width: 120px;
  flex-shrink: 0;
}

.refresh-ctrl {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.refresh-label {
  font-size: 13px;
  color: #374151;
  white-space: nowrap;
}

.refresh-toggle {
  padding: 4px 10px !important;
  font-size: 13px;
  border: 1px solid #d1d5db !important;
  border-radius: 4px !important;
  cursor: pointer;
  white-space: nowrap;
  box-shadow: none;
  background: #fff;
  color: #374151;
}

.refresh-toggle--running {
  background: #16a34a !important;
  border-color: #16a34a !important;
  color: #fff;
}

.refresh-toggle--running:hover {
  background: #15803d !important;
}

.sort-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 10px !important;
  font-size: 13px;
  border: 1px solid #d1d5db !important;
  border-radius: 4px !important;
  background: #fff;
  cursor: pointer;
  color: #374151;
  white-space: nowrap;
  box-shadow: none;
}

.sort-btn:hover {
  border-color: #3b82f6 !important;
  color: #3b82f6;
}

.sort-arrow {
  font-size: 11px;
  color: #9ca3af;
}

.capture-ctrl {
  display: flex;
  align-items: center;
  flex-shrink: 0;
}

.capture-btn {
  padding: 4px 12px !important;
  font-size: 13px;
  border: 1px solid #d1d5db !important;
  border-radius: 4px !important;
  cursor: pointer;
  white-space: nowrap;
  box-shadow: none;
}

.capture-btn--running {
  background: #ef4444 !important;
  border-color: #ef4444 !important;
  color: #fff;
}

.capture-btn--running:hover {
  background: #dc2626 !important;
}

.capture-btn--stopped {
  background: #3b82f6 !important;
  border-color: #3b82f6 !important;
  color: #fff;
}

.capture-btn--stopped:hover {
  background: #2563eb !important;
}


</style>
