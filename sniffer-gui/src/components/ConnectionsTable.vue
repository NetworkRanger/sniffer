<script setup lang="ts">
import type {Connection} from "../types/connection";
import {filesize} from "filesize";
import {ref} from "vue";


defineProps<{ connections: Connection[] }>();

const bytesize = (bytes: number) =>
    filesize(bytes, {base: 2, standard: "jedec", round: 1}).replace(" ", "");

const speedsize = (bytes: number) => bytesize(bytes) + "/s";

const formatTime = (ts: number) => ts ? new Date(ts * 1000).toLocaleString() : '-';

function fmtProcName(row: { process_name?: string | null; kernel_name?: string | null } | null): string {
  if (!row) return '-';
  const pname = row.process_name || null;
  const kname = row.kernel_name || null;
  if (!pname && !kname) return '-';
  if (!pname) return kname!;
  if (!kname) return pname;
  if (kname.toLowerCase() === pname.toLowerCase()) return pname;
  return `${kname}(${pname})`;
}

function fmtStartTime(us: number): string {
  if (!us) return '-';
  const ms = Math.floor(us / 1000);
  const d = new Date(ms);
  const yyyy = d.getFullYear();
  const M = d.getMonth() + 1;
  const day = d.getDate();
  const hh = String(d.getHours()).padStart(2, '0');
  const mm = String(d.getMinutes()).padStart(2, '0');
  const ss = String(d.getSeconds()).padStart(2, '0');
  const msec = String(d.getMilliseconds()).padStart(3, '0');
  return `${yyyy}/${M}/${day} ${hh}:${mm}:${ss}.${msec}`;
}

function fmtDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const s = ms / 1000;
  if (s < 60) {
    const rounded = Math.round(s * 100) / 100;
    return `${rounded}s`;
  }
  const mins = Math.floor(s / 60);
  const secs = Math.round(s % 60);
  return secs > 0 ? `${mins}min${secs}s` : `${mins}min`;
}

function getDurationMs(row: Connection): number {
  const nowUs = Date.now() * 1000;
  const endUs = row.status === 'closed' ? row.last_active * 1_000_000 : nowUs;
  const startUs = row.start_time_us || row.start_time * 1_000_000;
  return Math.max(0, Math.floor((endUs - startUs) / 1000));
}

// 列定义
interface ColDef {
  key: string;
  label: string;
  width?: number;
  minWidth?: number;
  align?: 'left' | 'right' | 'center';
  visible: boolean;
}

const columns = ref<ColDef[]>([
  {key: 'process_group',  label: '进程',      width: 150, visible: true},
  {key: 'protocol_group', label: '协议',      width: 80,  visible: true},
  {key: 'traffic',        label: '流量',      width: 200, align: 'center', visible: true},
  {key: 'addr_group',     label: '地址',      width: 160, visible: true},
  {key: 'status',         label: '状态',      width: 95,  visible: true},
  {key: 'domain_group',   label: '域名/路径', width: 160, visible: true},
  {key: 'time_group',     label: '时间',      width: 175, visible: true},
  {key: 'duration',       label: '持续时间',  width: 120,  align: 'center', visible: true},
]);

const showColMenu = ref(false);

defineExpose({columns, showColMenu});
</script>

<template>
    <el-table :data="connections" size="default" stripe border>
      <template v-for="col in columns" :key="col.key">
        <!-- 流量列：嵌套分组，双行表头 TX/RX/TOTAL -->
        <el-table-column v-if="col.key === 'traffic' && col.visible" label="流量" resizable>
          <el-table-column label="TX" width="100" align="right" resizable>
            <template #header>
              <div class="traffic-header">
                <span class="traffic-bps">下载速度</span>
                <span class="traffic-total">下载量</span>
              </div>
            </template>
            <template #default="{ row }">
              <div class="traffic-cell">
                <span class="traffic-bps">{{ speedsize(row.upload_speed) }}</span>
                <span class="traffic-total">{{ bytesize(row.bytes_sent) }}</span>
              </div>
            </template>
          </el-table-column>
          <el-table-column label="RX" width="100" align="right" resizable>
            <template #header>
              <div class="traffic-header">
                <span class="traffic-bps">上传速度</span>
                <span class="traffic-total">上传量</span>
              </div>
            </template>
            <template #default="{ row }">
              <div class="traffic-cell">
                <span class="traffic-bps">{{ speedsize(row.download_speed) }}</span>
                <span class="traffic-total">{{ bytesize(row.bytes_recv) }}</span>
              </div>
            </template>
          </el-table-column>
          <el-table-column label="TOTAL" width="100" align="right" resizable>
            <template #header>
              <div class="traffic-header">
                <span class="traffic-bps">总网速</span>
                <span class="traffic-total">总量</span>
              </div>
            </template>
            <template #default="{ row }">
              <div class="traffic-cell">
                <span class="traffic-bps">{{ speedsize(row.upload_speed + row.download_speed) }}</span>
                <span class="traffic-total">{{ bytesize(row.bytes_sent + row.bytes_recv) }}</span>
              </div>
            </template>
          </el-table-column>
        </el-table-column>

        <!-- 协议列：传输协议 / 应用协议 上下显示 -->
        <el-table-column v-else-if="col.key === 'protocol_group' && col.visible" label="协议" width="100" align="center" resizable>
          <template #header>
            <div class="traffic-header">
              <span class="traffic-bps">传输协议</span>
              <span class="traffic-total">应用协议</span>
            </div>
          </template>
          <template #default="{ row }">
            <div class="traffic-cell" style="align-items:center;">
              <span class="traffic-bps">{{ row.protocol }}</span>
              <span class="traffic-total">{{ row.packet_connection?.protocol || '-' }}</span>
            </div>
          </template>
        </el-table-column>

        <!-- 地址列：本地地址 / 远程地址 上下显示 -->
        <el-table-column v-else-if="col.key === 'addr_group' && col.visible" label="地址" width="160" resizable>
          <template #header>
            <div class="traffic-header">
              <span class="traffic-bps">本地地址</span>
              <span class="traffic-total">远程地址</span>
            </div>
          </template>
          <template #default="{ row }">
            <div class="traffic-cell" style="align-items:flex-start;">
              <span class="traffic-bps">{{ row.local_addr }}:{{ row.local_port }}</span>
              <span class="traffic-total">{{ row.remote_addr }}:{{ row.remote_port }}</span>
            </div>
          </template>
        </el-table-column>

        <!-- 进程列：进程名称 / 进程ID 上下显示 -->
        <el-table-column v-else-if="col.key === 'process_group' && col.visible" label="进程" width="200" resizable>
          <template #header>
            <div class="traffic-header">
              <span class="traffic-bps">进程名称</span>
              <span class="traffic-total">进程ID</span>
            </div>
          </template>
          <template #default="{ row }">
            <div class="traffic-cell" style="align-items:flex-start;">
              <div class="proc-cell">
                <img v-if="row.process_connection?.icon"
                     :src="'data:image/png;base64,' + row.process_connection.icon"
                     class="proc-icon"
                     @error="(e: Event) => (e.target as HTMLImageElement).style.display = 'none'" />
                <span class="traffic-bps">{{ fmtProcName(row.process_connection) }}</span>
              </div>
              <span class="traffic-total">{{ row.process_connection?.pid || '-' }}</span>
            </div>
          </template>
        </el-table-column>

        <!-- 域名列：域名 / 路径 上下显示 -->
        <el-table-column v-else-if="col.key === 'domain_group' && col.visible" label="域名/路径" width="160" resizable>
          <template #header>
            <div class="traffic-header">
              <span class="traffic-bps">域名</span>
              <span class="traffic-total">路径</span>
            </div>
          </template>
          <template #default="{ row }">
            <div class="traffic-cell" style="align-items:flex-start;">
              <span class="traffic-bps">{{ row.domain || '-' }}</span>
              <span class="traffic-total">{{ row.path || '-' }}</span>
            </div>
          </template>
        </el-table-column>

        <!-- 时间列：活跃时间 / 启动时间 上下显示 -->
        <el-table-column v-else-if="col.key === 'time_group' && col.visible" label="时间" width="175" resizable>
          <template #header>
            <div class="traffic-header">
              <span class="traffic-bps">活跃时间</span>
              <span class="traffic-total">启动时间</span>
            </div>
          </template>
          <template #default="{ row }">
            <div class="traffic-cell" style="align-items:flex-start;">
              <span class="traffic-bps">{{ formatTime(row.last_active) }}</span>
              <span class="traffic-total">{{ fmtStartTime(row.start_time_us) }}</span>
            </div>
          </template>
        </el-table-column>

        <!-- 其他列 -->
        <el-table-column
            v-else-if="!['traffic','protocol_group','addr_group','process_group','domain_group','time_group'].includes(col.key) && col.visible"
            :label="col.label"
            :width="col.width"
            :min-width="col.minWidth"
            :align="col.align || 'left'"
            resizable>
          <template #default="{ row }">
            <template v-if="col.key === 'status'">{{ row.status }}</template>
            <template v-else-if="col.key === 'duration'">{{ fmtDuration(getDurationMs(row)) }}</template>
          </template>
        </el-table-column>
      </template>
    </el-table>
</template>

<style scoped>
.proc-cell {
  display: flex;
  align-items: center;
  gap: 4px;
  overflow: hidden;
}

.proc-icon {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
  object-fit: contain;
}

.traffic-header {
  display: flex;
  flex-direction: column;
  align-items: center;
  line-height: 1.3;
}

.traffic-cell {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  line-height: 1.4;
}

.traffic-bps {
  color: #374151;
  font-size: 12px;
}

.traffic-total {
  color: #333;
  font-size: 12px;
}

:deep(.el-table__header-wrapper th .cell) {
  justify-content: center;
  text-align: center;
}
</style>
