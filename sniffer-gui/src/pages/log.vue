<script setup lang="ts">
import {computed, nextTick, onMounted, onUnmounted, ref} from "vue";
import {invoke} from "@tauri-apps/api/core";

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
const logs = ref<LogEntry[]>([]);
const logListEl = ref<HTMLDivElement>();
let timer: number | null = null;
let autoScroll = true;

async function loadLogs() {
  if (paused.value) return;
  try {
    const result = await invoke<LogEntry[]>('get_logs', {limit: 1000});
    logs.value = result;
    if (autoScroll) {
      await nextTick();
      if (logListEl.value) {
        logListEl.value.scrollTop = logListEl.value.scrollHeight;
      }
    }
  } catch {}
}

function onScroll() {
  if (!logListEl.value) return;
  const el = logListEl.value;
  autoScroll = el.scrollTop + el.clientHeight >= el.scrollHeight - 20;
}

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

function clearLogs() { logs.value = []; }
function togglePause() { paused.value = !paused.value; }

const levelColor: Record<string, string> = {
  INFO: '#16a34a', WARN: '#d97706', ERROR: '#dc2626', DEBUG: '#6b7280', TRACE: '#a78bfa',
};

onMounted(() => {
  loadLogs();
  timer = window.setInterval(loadLogs, 2000);
});
onUnmounted(() => { if (timer) clearInterval(timer); });
</script>

<template>
  <div class="page">
    <div class="page-header">
      <span class="page-title">日志</span>
      <div class="header-actions">
        <span class="action-btn" :class="{active: paused}" @click="togglePause" :title="paused ? '继续' : '暂停'">{{ paused ? '▶' : '⏸' }}</span>
        <el-button type="primary" size="small" @click="clearLogs">清除</el-button>
      </div>
    </div>

    <div class="toolbar">
      <el-select v-model="levelFilter" size="small" style="width: 100px; flex-shrink: 0;">
        <el-option v-for="l in levelOptions" :key="l" :label="l" :value="l" />
      </el-select>
      <el-input v-model="filter" placeholder="过滤条件" clearable size="small" />
    </div>

    <div class="log-list" ref="logListEl" @scroll="onScroll">
      <div v-if="filteredLogs.length === 0" class="log-empty">暂无日志</div>
      <div v-for="(entry, i) in filteredLogs" :key="i" class="log-entry">
        <span class="log-time">{{ entry.time }}</span>
        <span class="log-level" :style="{color: levelColor[entry.level] || '#374151'}">{{ entry.level }}</span>
        <span class="log-target">{{ entry.target }}</span>
        <span class="log-msg">{{ entry.message }}</span>
      </div>
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
.log-list {
  flex: 1;
  overflow-y: auto;
  font-family: 'Menlo', 'Consolas', monospace;
  font-size: 12px;
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
  padding: 3px 12px;
  border-bottom: 1px solid #f3f4f6;
  line-height: 1.5;
  align-items: baseline;
}
.log-entry:hover { background: #f9fafb; }
.log-time { color: #9ca3af; flex-shrink: 0; }
.log-level { font-weight: 700; flex-shrink: 0; width: 44px; }
.log-target { color: #6b7280; flex-shrink: 0; max-width: 160px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.log-msg { color: #374151; word-break: break-all; flex: 1; }
</style>
