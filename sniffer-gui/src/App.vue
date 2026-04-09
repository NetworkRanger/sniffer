<script setup lang="ts">
import MenuBar from "./components/MenuBar.vue";
import ConnectionsTable from "./components/ConnectionsTable.vue";
import {onMounted, ref} from "vue";
import { useI18n } from "vue-i18n";
import StatusBar from "./components/StatusBar.vue";
import type { Connection } from "./types/connection";
import {invoke} from "@tauri-apps/api/core";

// 初始化国际化
const {t} = useI18n();

// 状态栏信息
const statusBarInfo = ref({
  totalConnections: 0,
  tcpConnections: 0,
  udpConnections: 0,
  kernelConnections: 0,
  establishedConnections: 0,
  listenConnections: 0,
  timeWaitConnections: 0,
  closeWaitConnections: 0,
  otherConnections: 0,
  lastUpdate: new Date().toLocaleTimeString(),
  refreshInterval: null as number | null,
});

const isLoading = ref(false);
const connections = ref<Connection[]>([]);
// 自动刷新相关状态
const autoRefreshInterval = ref<number | null>(null);
const isAutoRefreshEnabled = ref(false);
// const refreshIntervals = [1, 2, 3, 5, 10]; // 可选的刷新间隔（秒）
// const isFirstLoad = ref(true); // 标记是否是首次加载

const selectedRefreshInterval = ref(3); // 默认选择5秒


// 存储用户自定义的列宽
const customColumnWidths = ref<Record<string, number>>({});

async function loadConnections() {
  isLoading.value = true;
  try {
    const result: Connection[] = await invoke("get_connections");
    console.log('result', result);
    // 应用筛选条件
    let filteredResult = result;

    // 将所有连接（包括标记为删除的）赋值给connections，以便在UI中显示删除状态
    connections.value = filteredResult;
  } catch (error) {
    console.error(t("alerts.getConnectionsFailed", { error }), error);

  } finally {
    isLoading.value = false;
  }
}

// 启用自动刷新
function enableAutoRefresh() {
  if (autoRefreshInterval.value !== null) {
    clearInterval(autoRefreshInterval.value);
  }

  isAutoRefreshEnabled.value = true;
  autoRefreshInterval.value = window.setInterval(() => {
    loadConnections();
  }, selectedRefreshInterval.value * 1000); // 转换为毫秒

  // 更新状态栏信息
  statusBarInfo.value.refreshInterval = selectedRefreshInterval.value;
}

// 页面加载完成后自动获取连接列表
onMounted(async () => {
  // 设置语言为本地存储的语言或浏览器语言
  const {locale} = useI18n();
  locale.value = "zh";

  loadConnections();
  // setTimeout(loadConnections, 5000);
  enableAutoRefresh();
});


</script>

<template>
  <main class="container">
    <MenuBar/>
    <ConnectionsTable
      :connections="connections"
      :custom-column-widths="customColumnWidths"
    />
    <StatusBar :status-bar-info="statusBarInfo" />
  </main>
</template>

<style scoped>
.logo.vite:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

.logo.vue:hover {
  filter: drop-shadow(0 0 2em #249b73);
}

</style>
<style>
:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

.container {
  margin: 0;
  padding-top: 10vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  text-align: center;
}

.logo {
  height: 6em;
  padding: 1.5em;
  will-change: filter;
  transition: 0.75s;
}

.logo.tauri:hover {
  filter: drop-shadow(0 0 2em #24c8db);
}

.row {
  display: flex;
  justify-content: center;
}

a {
  font-weight: 500;
  color: #646cff;
  text-decoration: inherit;
}

a:hover {
  color: #535bf2;
}

h1 {
  text-align: center;
}

input,
button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

button {
  cursor: pointer;
}

button:hover {
  border-color: #396cd8;
}
button:active {
  border-color: #396cd8;
  background-color: #e8e8e8;
}

input,
button {
  outline: none;
}

#greet-input {
  margin-right: 5px;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }

  a:hover {
    color: #24c8db;
  }

  input,
  button {
    color: #ffffff;
    background-color: #0f0f0f98;
  }
  button:active {
    background-color: #0f0f0f69;
  }
}

</style>