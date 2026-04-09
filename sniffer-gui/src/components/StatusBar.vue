<template>
  <!-- 状态栏 -->
  <div class="status-bar">
    <div class="status-item">
      <span class="status-label">{{ t("statusBar.totalConnections") }}:</span>
      <span class="status-value">{{ statusBarInfo.totalConnections }}</span>
    </div>
    <div class="status-item">
      <span class="status-label">{{ t("statusBar.tcpConnections") }}:</span>
      <span class="status-value tcp-count">{{
          statusBarInfo.tcpConnections
        }}</span>
    </div>
    <div class="status-item">
      <span class="status-label">{{ t("statusBar.udpConnections") }}:</span>
      <span class="status-value udp-count">{{
          statusBarInfo.udpConnections
        }}</span>
    </div>
    <div class="status-item">
      <span class="status-label">{{ t("statusBar.established") }}:</span>
      <span class="status-value established-count">{{
          statusBarInfo.establishedConnections
        }}</span>
    </div>
    <div class="status-item">
      <span class="status-label">{{ t("statusBar.listen") }}:</span>
      <span class="status-value listen-count">{{
          statusBarInfo.listenConnections
        }}</span>
    </div>
    <div class="status-item">
      <span class="status-label">{{ t("statusBar.wait") }}:</span>
      <span class="status-value wait-count">{{
          statusBarInfo.timeWaitConnections
        }}</span>
    </div>
    <div class="status-item">
      <span class="status-label">{{ t("statusBar.closeWait") }}:</span>
      <span class="status-value close-wait-count">{{
          statusBarInfo.closeWaitConnections
        }}</span>
    </div>
    <div class="status-item">
      <span class="status-label">{{ t("statusBar.other") }}:</span>
      <span class="status-value other-count">{{
          statusBarInfo.otherConnections
        }}</span>
    </div>
    <div class="status-item">
      <span class="status-label">{{ t("statusBar.kernel") }}:</span>
      <span class="status-value kernel-count">{{
          statusBarInfo.kernelConnections
        }}</span>
    </div>
    <div class="status-item">
      <span class="status-label">{{ t("statusBar.lastUpdate") }}:</span>
      <span class="status-value">{{ statusBarInfo.lastUpdate }}</span>
    </div>
    <div class="status-item" v-if="statusBarInfo.refreshInterval">
      <span class="status-label">{{ t("statusBar.refreshInterval") }}:</span>
      <span class="status-value"
      >{{ statusBarInfo.refreshInterval
        }}{{ t("menu.refreshInterval") }}</span
      >
    </div>
  </div>
</template>

<script setup lang="ts">
import { useI18n } from "vue-i18n";

// 定义状态栏信息接口
interface StatusBarInfo {
  totalConnections: number;
  tcpConnections: number;
  udpConnections: number;
  kernelConnections: number;
  establishedConnections: number;
  listenConnections: number;
  timeWaitConnections: number;
  closeWaitConnections: number;
  otherConnections: number;
  lastUpdate: string;
  refreshInterval: number | null;
}

// 定义组件属性
interface Props {
  statusBarInfo: StatusBarInfo;
}

defineProps<Props>();

// 使用国际化
const { t } = useI18n();
</script>

<style scoped>
/* 状态栏样式 */
.status-bar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 2px 10px;
  background-color: #f3f4f6;
  border-top: 1px solid #d1d5db;
  font-size: 0.7rem;
  color: #374151;
  min-height: 14px;
  flex-shrink: 0; /* 防止状态栏被压缩 */
  flex-wrap: nowrap; /* 防止换行 */
  overflow: hidden; /* 隐藏溢出内容，不显示滚动条 */
}

.status-item {
  display: flex;
  align-items: center;
  margin-right: 20px;
  white-space: nowrap; /* 防止内容换行 */
  position: relative; /* 为添加分隔符做准备 */
}

/* 为每个状态项添加右侧分隔符（最后一个除外） */
.status-item:not(:last-child)::after {
  content: "|";
  margin-left: 25px; /* 在分隔符左侧添加一些间距 */
  color: #9ca3af; /* 分隔符颜色 */
  opacity: 0.7; /* 稍微降低分隔符的透明度 */
}

.status-label {
  margin-right: 6px;
  font-weight: 500;
  color: #4b5563;
}

.status-value {
  font-weight: 600;
  color: #1f2937;
}

.status-value.tcp-count {
  color: #1d4ed8; /* 蓝色 */
}

.status-value.udp-count {
  color: #c2410c; /* 橙色 */
}

.status-value.established-count {
  color: #16a34a; /* 绿色 - 表示已建立的连接 */
}

.status-value.listen-count {
  color: #3b82f6; /* 蓝色 - 表示监听状态 */
}

.status-value.wait-count {
  color: #eab308; /* 黄色 - 表示等待状态 */
}

.status-value.close-wait-count {
  color: #f97316; /* 橙色 - 表示关闭等待 */
}

.status-value.other-count {
  color: #8b5cf6; /* 紫色 - 表示其他状态 */
}

.status-value.kernel-count {
  color: #7c2d12; /* 深红棕色 */
}

/* 深色模式下的状态栏样式 */
.dark .status-bar {
  background-color: #1a202c; /* 深灰蓝 */
  border-top: 1px solid #2d3748; /* 深中灰蓝 */
  color: #a0aec0; /* 中等亮度的灰蓝 */
}

.dark .status-label {
  color: #718096; /* 中灰 */
}

.dark .status-value {
  color: #a0aec0; /* 中等亮度的灰蓝 */
}

.dark .status-value.tcp-count {
  color: #63b3ed; /* 深一些的蓝色 */
}

.dark .status-value.udp-count {
  color: #f6ad55; /* 深一些的橙色 */
}

.dark .status-value.established-count {
  color: #68d391; /* 深一些的绿色 */
}

.dark .status-value.listen-count {
  color: #90cdf4; /* 深一些的蓝色 */
}

.dark .status-value.wait-count {
  color: #f6e05e; /* 深一些的黄色 */
}

.dark .status-value.close-wait-count {
  color: #fc8181; /* 深一些的红色 */
}

.dark .status-value.other-count {
  color: #b0a1e6; /* 深一些的紫色 */
}

.dark .status-value.kernel-count {
  color: #f59f9f; /* 深一些的红粉色 */
}
</style>