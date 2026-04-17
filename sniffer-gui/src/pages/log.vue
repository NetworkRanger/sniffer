<script setup lang="ts">
import {computed, onMounted, onUnmounted, ref} from 'vue';
import VirtualList from '../components/VirtualList.vue';

type LogLevel = 'ALL' | 'INFO' | 'WARN' | 'ERROR' | 'DEBUG' | 'TRACE';

interface LogEntry {
  time: string;
  level: string;
  target: string;
  message: string;
}

const filter = ref('');
const levelFilter = ref<LogLevel>('INFO');
const paused = ref(false);
const levelOptions: LogLevel[] = ['ALL', 'TRACE', 'DEBUG', 'INFO', 'WARN', 'ERROR'];
const MAX_LOGS = 50000;

const mockLogs: LogEntry[] = [
  {time: '10:00:00.001', level: 'INFO',  target: 'sniffer_lib', message: 'Capture started on en0'},
  {time: '10:00:00.123', level: 'DEBUG', target: 'sniffer_lib::aggregator', message: 'Aggregator started'},
  {time: '10:00:00.456', level: 'INFO',  target: 'sniffer_lib', message: 'Using device: en0'},
  {time: '10:00:01.001', level: 'TRACE', target: 'sniffer_lib::capture', message: 'IPv4: 192.168.1.1 -> 8.8.8.8'},
  {time: '10:00:01.200', level: 'WARN',  target: 'sniffer_lib', message: 'channel full, dropping packet: channel full'},
  {time: '10:00:02.300', level: 'ERROR', target: 'sniffer_lib::process', message: 'failed to get process icon: permission denied'},
  {time: '10:00:03.100', level: 'INFO',  target: 'sniffer_lib', message: 'result len: 14'},
];

const vlistRef = ref<InstanceType<typeof VirtualList>>();
let autoScroll = true;
let ws: WebSocket | null = null;

function connect() {
  ws = new WebSocket('ws://127.0.0.1:9999');
  ws.onopen = () => {
    // clear mock data once real connection established
    if (logs.value === mockLogs) logs.value = [];
  };
  ws.onmessage = (e) => {
    if (paused.value) return;
    try {
      const entry: LogEntry = JSON.parse(e.data);
      logs.value.push(entry);
      // 超出上限时批量裁剪，保留最新的 MAX_LOGS 条
      if (logs.value.length > MAX_LOGS) {
        logs.value.splice(0, logs.value.length - MAX_LOGS);
      }
      if (autoScroll) {
        setTimeout(() => vlistRef.value?.scrollToBottom(), 0);
      }
    } catch {}
  };
  ws.onclose = () => {
    setTimeout(connect, 2000);
  };
}

// 浏览器预览时 WebSocket 连接失败，保留 mock 数据
const logs = ref<LogEntry[]>(mockLogs);

onMounted(connect);
onUnmounted(() => { ws?.close(); ws = null; });

const filteredLogs = computed(() => {
  return logs.value.filter(l => {
    if (levelFilter.value !== 'ALL' && l.level !== levelFilter.value) return false;
    if (filter.value) {
      const q = filter.value.toLowerCase();
      return l.message.toLowerCase().includes(q) || l.target.toLowerCase().includes(q);
    }
    return true;
  });
});

function onVlistScroll(e: Event) {
  const el = e.target as HTMLDivElement;
  autoScroll = el.scrollTop + el.clientHeight >= el.scrollHeight - 40;
}

function clearLogs() { logs.value = []; }
function togglePause() { paused.value = !paused.value; }

const levelColor: Record<string, string> = {
  INFO: '#16a34a', WARN: '#d97706', ERROR: '#dc2626', DEBUG: '#6b7280', TRACE: '#a78bfa',
};
</script>

<template>
  <div class="page">
    <div class="page-header">
      <span class="page-title">日志</span>
      <div class="header-actions">
        <span class="action-btn" :class="{active: paused}" @click="togglePause" :title="paused ? '继续' : '暂停'">
          {{ paused ? '▶' : '⏸' }}
        </span>
        <el-button type="primary" size="small" @click="clearLogs">清除</el-button>
      </div>
    </div>

    <div class="toolbar">
      <el-select v-model="levelFilter" size="small" style="width: 100px; flex-shrink: 0;">
        <el-option v-for="l in levelOptions" :key="l" :label="l" :value="l" />
      </el-select>
      <el-input v-model="filter" placeholder="过滤条件" clearable size="small" />
      <span class="log-count">{{ filteredLogs.length }} 条</span>
    </div>

    <div class="log-area">
      <div v-if="filteredLogs.length === 0" class="log-empty">暂无日志</div>
      <VirtualList
          v-else
          ref="vlistRef"
          :items="filteredLogs"
          :item-height="28"
          :overscan="10"
          @scroll="onVlistScroll"
      >
        <template #default="{ item }">
          <div class="log-entry">
            <span class="log-time">{{ (item as LogEntry).time }}</span>
            <span class="log-level" :style="{color: levelColor[(item as LogEntry).level] || '#374151'}">
              {{ (item as LogEntry).level }}
            </span>
            <span class="log-target">{{ (item as LogEntry).target }}</span>
            <span class="log-msg">{{ (item as LogEntry).message }}</span>
          </div>
        </template>
      </VirtualList>
    </div>
  </div>
</template>

<style scoped>
.page {
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
  flex-shrink: 0;
}
.page-title {
  font-size: 16px;
  font-weight: 600;
  color: #1f2937;
  flex: 1;
}
.header-actions {
  display: flex;
  align-items: center;
  gap: 10px;
}
.action-btn {
  font-size: 18px;
  cursor: pointer;
  color: #6b7280;
  user-select: none;
}
.action-btn:hover, .action-btn.active { color: #3b82f6; }
.toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px;
  border-bottom: 1px solid #e4e7ed;
  flex-shrink: 0;
}
.log-count {
  font-size: 12px;
  color: #9ca3af;
  white-space: nowrap;
}
.log-area {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
.log-empty {
  text-align: center;
  color: #9ca3af;
  padding: 40px;
  font-size: 13px;
}
.log-entry {
  display: flex;
  gap: 8px;
  padding: 4px 12px;
  height: 28px;
  align-items: center;
  font-family: 'Menlo', 'Consolas', monospace;
  font-size: 12px;
  border-bottom: 1px solid #f3f4f6;
  box-sizing: border-box;
}
.log-entry:hover { background: #f9fafb; }
.log-time { color: #9ca3af; flex-shrink: 0; }
.log-level { font-weight: 700; flex-shrink: 0; width: 44px; }
.log-target { color: #6b7280; flex-shrink: 0; max-width: 160px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.log-msg { color: #374151; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; }
</style>
