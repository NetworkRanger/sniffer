<script setup lang="ts">
import {computed, onMounted, onUnmounted, ref} from "vue";
import {invoke} from "@tauri-apps/api/core";
import type {Connection} from "../types/connection";
import {filesize} from "filesize";
import ConnectionsTable from "../components/ConnectionsTable.vue";
import StatusBar from "../components/StatusBar.vue";

const connections = ref<Connection[]>([]);
const activeTab = ref<'active' | 'closed'>('active');
const filter = ref('');

type SortField = 'last_active' | 'bytes_sent' | 'bytes_recv' | 'upload_speed' | 'download_speed';
const sortField = ref<SortField>('last_active');
const sortOptions: { label: string; value: SortField }[] = [
  {label: '时间', value: 'last_active'},
  {label: '上传量', value: 'bytes_sent'},
  {label: '下载量', value: 'bytes_recv'},
  {label: '上传速度', value: 'upload_speed'},
  {label: '下载速度', value: 'download_speed'},
];
const sortLabel = computed(() => sortOptions.find(o => o.value === sortField.value)?.label ?? '时间');

let autoRefreshTimer: number | null = null;

const bytesize = (bytes: number) =>
    filesize(bytes, {base: 2, standard: "jedec", round: 1}).replace(" ", "");

const totalDownload = computed(() =>
    bytesize(connections.value.reduce((s, c) => s + c.bytes_recv, 0)));

const totalUpload = computed(() =>
    bytesize(connections.value.reduce((s, c) => s + c.bytes_sent, 0)));

const activeList = computed(() =>
    connections.value.filter(c => !c.isDelted && c.status !== 'CLOSED'));

const closedList = computed(() =>
    connections.value.filter(c => c.isDelted || c.status === 'CLOSED'));

const statusBar = computed(() => {
  const all = connections.value;
  return {
    totalConnections: all.length,
    tcpConnections: all.filter(c => c.protocol === 'TCP').length,
    udpConnections: all.filter(c => c.protocol === 'UDP').length,
    establishedConnections: all.filter(c => c.status === 'ESTABLISHED').length,
    listenConnections: all.filter(c => c.status === 'LISTEN').length,
    timeWaitConnections: all.filter(c => c.status === 'TIME_WAIT').length,
    closeWaitConnections: all.filter(c => c.status === 'CLOSE_WAIT').length,
    otherConnections: all.filter(c => !['ESTABLISHED','LISTEN','TIME_WAIT','CLOSE_WAIT','CLOSED'].includes(c.status)).length,
    kernelConnections: all.filter(c => c.process_connection?.pid === null).length,
    lastUpdate: new Date().toLocaleTimeString(),
  };
});

const filteredConnections = computed(() => {
  const list = activeTab.value === 'active' ? activeList.value : closedList.value;
  const filtered = !filter.value ? list : list.filter(c => {
    const host = `${c.domain || c.remote_addr}:${c.remote_port}`;
    return host.toLowerCase().includes(filter.value.toLowerCase());
  });
  return [...filtered].sort((a, b) => b[sortField.value] - a[sortField.value]);
});

const getHost = (c: Connection) => `${c.domain || c.remote_addr}:${c.remote_port}`;

const mockConnections: Connection[] = Array.from({length: 10}, (_, i) => ({
  id: `mock-${i}`,
  local_addr: '192.168.1.100',
  local_port: 50000 + i,
  remote_addr: `203.0.113.${i + 1}`,
  remote_port: [80, 443, 8080, 5228, 8838][i % 5],
  protocol: i % 2 === 0 ? 'TCP' : 'UDP',
  domain: [`www.google.com`, `mtalk.google.com`, `api.github.com`, `www.baidu.com`, `cdn.jsdelivr.net`,
    `fonts.googleapis.com`, `www.msftconnecttest.com`, `update.microsoft.com`, `ocsp.apple.com`, `push.apple.com`][i],
  path: i % 3 === 0 ? '/api/v1/data' : null,
  bytes_sent: Math.floor(Math.random() * 100000),
  bytes_recv: Math.floor(Math.random() * 500000),
  packets_sent: Math.floor(Math.random() * 200),
  packets_recv: Math.floor(Math.random() * 800),
  upload_speed: Math.floor(Math.random() * 5000),
  download_speed: Math.floor(Math.random() * 20000),
  start_time: Math.floor(Date.now() / 1000) - 3600 + i * 300,
  last_active: Math.floor(Date.now() / 1000) - i * 10,
  status: i < 8 ? 'ESTABLISHED' : 'CLOSED',
  isDelted: i >= 8,
  process_connection: {
    protocol: i % 2 === 0 ? 'TCP' : 'UDP',
    local_addr: '192.168.1.100',
    local_port: 50000 + i,
    remote_addr: `203.0.113.${i + 1}`,
    remote_port: [80, 443, 8080, 5228, 8838][i % 5],
    state: i < 8 ? 'ESTABLISHED' : 'CLOSED',
    pid: 1000 + i * 100,
    process_name: ['Chrome', 'Firefox', 'Safari', 'curl', 'node', 'python3', 'git', 'npm', 'code', 'ssh'][i],
    icon: null,
    start_time: Math.floor(Date.now() / 1000) - 7200,
    fill_column: '',
  },
  packet_connection: null,
}));

async function loadConnections() {
  try {
    const result = await invoke<Connection[]>("get_connections");
    connections.value = result;
  } catch (e) {
    // fallback to mock data in browser preview
    connections.value = mockConnections;
  }
}

onMounted(() => {
  loadConnections();
  autoRefreshTimer = window.setInterval(loadConnections, 1000);
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
          活跃 {{ activeList.length }}
        </button>
        <button
            :class="['tab-btn', activeTab === 'closed' ? 'tab-btn--active' : '']"
            @click="activeTab = 'closed'">
          已关闭 {{ closedList.length }}
        </button>
      </div>
      <div class="filter-wrap">
        <el-input v-model="filter" placeholder="过滤条件" clearable />
      </div>
      <el-dropdown trigger="click" @command="(v: SortField) => sortField = v">
        <button class="sort-btn">
          {{ sortLabel }} <span class="sort-arrow">&#8963;</span>
        </button>
        <template #dropdown>
          <el-dropdown-menu>
            <el-dropdown-item
                v-for="opt in sortOptions"
                :key="opt.value"
                :command="opt.value">
              <span style="display:flex;align-items:center;gap:6px;">
                <span style="width:14px;">{{ sortField === opt.value ? '✓' : '' }}</span>
                {{ opt.label }}
              </span>
            </el-dropdown-item>
          </el-dropdown-menu>
        </template>
      </el-dropdown>
    </div>

    <!-- Table -->
    <ConnectionsTable :connections="filteredConnections" />
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


</style>
