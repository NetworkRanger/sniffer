<script setup lang="ts">
import {computed, ref} from "vue";

type LogLevel = 'ALL' | 'INFO' | 'WARNING' | 'ERROR' | 'DEBUG';

interface LogEntry {
  id: number;
  time: string;
  level: 'INFO' | 'WARNING' | 'ERROR' | 'DEBUG';
  message: string;
}

const filter = ref('');
const levelFilter = ref<LogLevel>('ALL');
const paused = ref(false);
const levelOptions: LogLevel[] = ['ALL', 'INFO', 'WARNING', 'ERROR', 'DEBUG'];

// mock logs
let idSeq = 0;
const mkLog = (level: LogEntry['level'], msg: string): LogEntry => ({
  id: ++idSeq,
  time: new Date().toLocaleString('zh-CN', {month:'2-digit',day:'2-digit',hour:'2-digit',minute:'2-digit',second:'2-digit'}).replace(/\//g,'-'),
  level,
  message: msg,
});

const logs = ref<LogEntry[]>([
  mkLog('WARNING', '[TCP] dial DIRECT 198.18.0.1:50468 --> 172.30.96.110:13023 error: connect failed: dial tcp 172.30.96.110:13023: i/o timeout'),
  mkLog('WARNING', '[TCP] dial DIRECT 198.18.0.1:46864 --> ipv6.msftconnecttest.com:80 error: connect failed: dial tcp [2600:1417:4400:24::17d2:7ad]:80: connectex: A socket operation was attempted to an unreachable network.\\ndial tcp [2600:1417:4400:24::17d2:7a6]:80: connectex: A socket operation was attempted to an unreachable network.'),
  mkLog('INFO',    '[TCP] dial DIRECT 192.168.1.1:443 --> api.github.com:443 connected'),
  mkLog('ERROR',   '[UDP] dial DIRECT 10.0.0.1:53 --> 8.8.8.8:53 error: timeout'),
  mkLog('INFO',    '[TCP] dial DIRECT 192.168.1.100:8080 --> www.google.com:443 connected'),
  mkLog('DEBUG',   'DNS resolve api.github.com -> 140.82.114.5 (cached)'),
]);

const filteredLogs = computed(() => {
  return logs.value.filter(l => {
    if (levelFilter.value !== 'ALL' && l.level !== levelFilter.value) return false;
    if (filter.value && !l.message.toLowerCase().includes(filter.value.toLowerCase())) return false;
    return true;
  });
});

function clearLogs() { logs.value = []; }
function togglePause() { paused.value = !paused.value; }

const levelColor: Record<LogEntry['level'], string> = {
  INFO: '#16a34a', WARNING: '#d97706', ERROR: '#dc2626', DEBUG: '#6b7280',
};
</script>

<template>
  <div class="page">
    <div class="page-header">
      <span class="page-title">日志</span>
      <div class="header-actions">
        <span class="action-btn" :class="{active: paused}" @click="togglePause" title="暂停">⏸</span>
        <span class="action-btn" title="排序">⇅</span>
        <el-button type="primary" size="small" @click="clearLogs">清除</el-button>
      </div>
    </div>

    <div class="toolbar">
      <el-select v-model="levelFilter" size="small" style="width: 100px; flex-shrink: 0;">
        <el-option v-for="l in levelOptions" :key="l" :label="l" :value="l" />
      </el-select>
      <el-input v-model="filter" placeholder="过滤条件" clearable size="small" />
    </div>

    <div class="log-list">
      <div v-if="filteredLogs.length === 0" class="log-empty">暂无日志</div>
      <div v-for="entry in filteredLogs" :key="entry.id" class="log-entry">
        <div class="log-meta">
          <span class="log-time">{{ entry.time }}</span>
          <span class="log-level" :style="{color: levelColor[entry.level]}">{{ entry.level }}</span>
        </div>
        <div class="log-msg">{{ entry.message }}</div>
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
  padding: 0;
}

.log-empty {
  text-align: center;
  color: #9ca3af;
  padding: 40px;
  font-size: 13px;
}

.log-entry {
  padding: 8px 12px;
  border-bottom: 1px solid #f3f4f6;
  font-size: 12px;
  font-family: 'Menlo', 'Consolas', monospace;
}

.log-entry:hover { background: #f9fafb; }

.log-meta {
  display: flex;
  gap: 10px;
  margin-bottom: 2px;
}

.log-time { color: #9ca3af; }
.log-level { font-weight: 700; }
.log-msg { color: #374151; line-height: 1.5; word-break: break-all; }
</style>
