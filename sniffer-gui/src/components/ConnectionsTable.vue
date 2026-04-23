<script setup lang="ts">
import type {Connection, TreeRow} from "../types/connection";
import {filesize} from "filesize";
import {ref} from "vue";


defineProps<{ connections: TreeRow[] }>();
const emit = defineEmits<{ 'sort-change': [payload: { prop: string | null; order: string | null }] }>();

const sortOrders = ['descending', 'ascending', null] as const;

function onSortChange({ prop, order }: { prop: string | null; order: string | null }) {
  emit('sort-change', { prop, order });
}

const isGroup = (row: any): boolean => !!row._isGroup;

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
  const endUs = row.last_active * 1_000_000;
  const startUs = row.start_time_us || row.start_time * 1_000_000;
  return Math.max(0, Math.floor((endUs - startUs) / 1000));
}

// 列定义
interface ColDef {
  key: string;
  label: string;
  minWidth?: number;
  align?: 'left' | 'right' | 'center';
  visible: boolean;
}

const columns = ref<ColDef[]>([
  {key: 'process_group',  label: '进程',      minWidth: 15, visible: true},
  {key: 'protocol_group', label: '协议',      minWidth: 7,  visible: true},
  {key: 'traffic',        label: '流量',      minWidth: 23, align: 'center', visible: true},
  {key: 'addr_group',     label: '地址',      minWidth: 13, visible: true},
  {key: 'status',         label: '状态',      minWidth: 6,  visible: true},
  {key: 'domain_group',   label: '域名/路径', minWidth: 13, visible: true},
  {key: 'time_group',     label: '时间',      minWidth: 14, visible: true},
  {key: 'duration',       label: '持续时间',  minWidth: 9,  align: 'center', visible: true},
]);

const showColMenu = ref(false);

defineExpose({columns, showColMenu});
</script>

<template>
    <el-table :data="connections" size="default" stripe border height="100%"
              row-key="id" :tree-props="{ children: 'children', hasChildren: 'hasChildren' }"
              @sort-change="onSortChange">
      <!-- 展开按钮独立列 -->
      <el-table-column width="36" align="center" class-name="expand-col" />
      <template v-for="col in columns" :key="col.key">
        <!-- 流量列：嵌套分组，双行表头 TX/RX/TOTAL -->
        <el-table-column v-if="col.key === 'traffic' && col.visible" label="流量" resizable>
          <el-table-column label="TX" min-width="7.5" align="right" resizable sortable="custom" prop="bytes_sent" :sort-orders="sortOrders">
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
          <el-table-column label="RX" min-width="7.5" align="right" resizable sortable="custom" prop="bytes_recv" :sort-orders="sortOrders">
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
          <el-table-column label="TOTAL" min-width="7.5" align="right" resizable sortable="custom" prop="total_bytes" :sort-orders="sortOrders">
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
        <el-table-column v-else-if="col.key === 'protocol_group' && col.visible" label="协议" min-width="7" align="center" resizable>
          <template #header>
            <div class="traffic-header">
              <span class="traffic-bps">传输协议</span>
              <span class="traffic-total">应用协议</span>
            </div>
          </template>
          <template #default="{ row }">
            <div class="traffic-cell" style="align-items:center;">
              <span class="traffic-bps">{{ row.protocol }}</span>
              <span class="traffic-total">{{ row.app_protocol || '-' }}</span>
            </div>
          </template>
        </el-table-column>

        <!-- 地址列：本地地址 / 远程地址 上下显示 -->
        <el-table-column v-else-if="col.key === 'addr_group' && col.visible" label="地址" min-width="13" resizable>
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
        <el-table-column v-else-if="col.key === 'process_group' && col.visible" label="进程" min-width="15" resizable>
          <template #header>
            <div class="traffic-header">
              <span class="traffic-bps">进程名称</span>
              <span class="traffic-total">进程ID</span>
            </div>
          </template>
          <template #default="{ row }">
            <div v-if="isGroup(row)" class="traffic-cell" style="align-items:flex-start;">
              <div class="proc-cell">
                <img v-if="row.process_connection?.icon"
                     :src="'data:image/png;base64,' + row.process_connection.icon"
                     class="proc-icon"
                     @error="(e: Event) => (e.target as HTMLImageElement).style.display = 'none'" />
                <span class="traffic-bps" style="font-weight:600;">{{ fmtProcName(row.process_connection) }}</span>
              </div>
              <span class="traffic-total">{{ row.process_connection?.pid || '-' }} ({{ row.children?.length ?? 0 }})</span>
            </div>
            <div v-else class="traffic-cell" style="align-items:flex-start;">
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
        <el-table-column v-else-if="col.key === 'domain_group' && col.visible" label="域名/路径" min-width="13" resizable>
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
        <el-table-column v-else-if="col.key === 'time_group' && col.visible" label="时间" min-width="14" resizable sortable="custom" prop="last_active" :sort-orders="sortOrders">
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

        <!-- 状态列 -->
        <el-table-column
            v-else-if="col.key === 'status' && col.visible"
            label="状态"
            :min-width="col.minWidth"
            align="left"
            resizable
            sortable="custom"
            prop="status"
            :sort-orders="sortOrders">
          <template #default="{ row }">{{ row.status }}</template>
        </el-table-column>

        <!-- 持续时间列 -->
        <el-table-column
            v-else-if="col.key === 'duration' && col.visible"
            label="持续时间"
            :min-width="col.minWidth"
            align="center"
            resizable
            sortable="custom"
            prop="duration"
            :sort-orders="sortOrders">
          <template #default="{ row }">{{ fmtDuration(getDurationMs(row)) }}</template>
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

/* 展开按钮独立列 */
:deep(.expand-col .cell) {
  padding: 0;
  display: flex;
  justify-content: center;
  align-items: center;
}

/* 去掉展开按钮的背景和边框 */
:deep(.el-table__expand-icon) {
  background: none;
  border: none;
  box-shadow: none;
}

/* 去掉排序箭头区域的背景，默认隐藏 */
:deep(.caret-wrapper) {
  background: none !important;
  border: none !important;
  box-shadow: none !important;
  opacity: 0 !important;
  transition: opacity 0.2s;
}

/* hover 表头时显示排序图标 */
:deep(th:hover .caret-wrapper) {
  opacity: 1 !important;
}

/* 激活排序时始终显示 */
:deep(th.ascending .caret-wrapper),
:deep(th.descending .caret-wrapper) {
  opacity: 1 !important;
}

:deep(.sort-caret) {
  background: none !important;
}
</style>
