<script setup lang="ts">
defineProps<{
  totalConnections: number;
  tcpConnections: number;
  udpConnections: number;
  establishedConnections: number;
  listenConnections: number;
  timeWaitConnections: number;
  closeWaitConnections: number;
  otherConnections: number;
  kernelConnections: number;
  lastUpdate: string;
}>();
</script>

<template>
  <div class="status-bar">
    <span class="item">总计连接: <b>{{ totalConnections }}</b></span>
    <span class="sep">|</span>
    <span class="item">TCP连接: <b class="tcp">{{ tcpConnections }}</b></span>
    <span class="sep">|</span>
    <span class="item">UDP连接: <b class="udp">{{ udpConnections }}</b></span>
    <span class="sep">|</span>
    <span class="item">已建立: <b class="established">{{ establishedConnections }}</b></span>
    <span class="sep">|</span>
    <span class="item">监听: <b class="listen">{{ listenConnections }}</b></span>
    <span class="sep">|</span>
    <span class="item">等待: <b class="wait">{{ timeWaitConnections }}</b></span>
    <span class="sep">|</span>
    <span class="item">关闭等待: <b class="close-wait">{{ closeWaitConnections }}</b></span>
    <span class="sep">|</span>
    <span class="item">其他状态: <b class="other">{{ otherConnections }}</b></span>
    <span class="sep">|</span>
    <span class="item">内核连接: <b class="kernel">{{ kernelConnections }}</b></span>
    <span class="sep">|</span>
    <span class="item">上次更新: <b>{{ lastUpdate }}</b></span>
  </div>
</template>

<style scoped>
.status-bar {
  display: flex;
  align-items: center;
  flex-wrap: nowrap;
  overflow: hidden;
  padding: 0 10px;
  height: 28px;
  background: #f3f4f6;
  border-top: 1px solid #e4e7ed;
  font-size: 12px;
  color: #374151;
  gap: 8px;
  flex-shrink: 0;
}

.item {
  white-space: nowrap;
}

.sep {
  color: #d1d5db;
  flex-shrink: 0;
}

b { font-weight: 600; color: #1f2937; }
b.tcp { color: #1d4ed8; }
b.udp { color: #c2410c; }
b.established { color: #16a34a; }
b.listen { color: #3b82f6; }
b.wait { color: #eab308; }
b.close-wait { color: #f97316; }
b.other { color: #8b5cf6; }
b.kernel { color: #7c2d12; }
</style>
