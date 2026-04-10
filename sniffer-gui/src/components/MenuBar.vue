<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";

let {t} = useI18n();

// 定义组件属性
interface Props {
  // filterProtocol: "all" | "TCP" | "UDP";
  // filterState: string;
  // searchProcessName: string;
  // searchLocalPort: string;
  isAutoRefreshEnabled: boolean;
  selectedRefreshInterval: number;
  refreshIntervals: number[];
  // isDarkMode: boolean;
}

defineProps<Props>();

// 定义事件发射器
interface Emits {
  (e: "update:filterProtocol", value: "all" | "TCP" | "UDP"): void;
  (e: "update:filterState", value: string): void;
  (e: "update:searchProcessName", value: string): void;
  (e: "update:searchLocalPort", value: string): void;
  (e: "applyFiltersAndSearch"): void;
  (e: "toggleAutoRefresh"): void;
  (e: "changeRefreshInterval", interval: number): void;
  (e: "update:selectedRefreshInterval", value: number): void;
  (e: "changeLanguage", lang: "zh" | "en"): void;
  (e: "toggleTheme"): void;
  (e: "setProtocolFilter", protocol: "all" | "TCP" | "UDP"): void;
  (e: "showAboutDialog"): void;
}

const emit = defineEmits<Emits>();

const protocols = ref([
   "ALL", "TCP", "UDP", "HTTP", "HTTPS", "DNS",
]);
const protocol = ref("ALL");

const toggleAutoRefresh = () => {
  emit('toggleAutoRefresh');
};
const handleRefreshIntervalChange = (event: Event) => {
  const target = event.target as HTMLSelectElement;
  const interval = Number(target.value);
  emit('update:selectedRefreshInterval', interval);
  emit('changeRefreshInterval', interval);
};
</script>

<template>
  <!-- 主菜单容器，分为左右两组，两端对齐 -->
  <div class="main-menu-container ">
    <label class="menu-label">协议</label>
    <span style="width: 10px;"></span>
    <div class="protocol-buttons">
      <button v-for="item in protocols" @click="protocol = item">
        {{ item }}
      </button>
      <span></span>
      <div class="menu-group">
        <label class="menu-label">{{ t("menu.autoRefresh") }}</label>
        <div class="refresh-controls">
          <button
              :class="['refresh-toggle-btn', { active: isAutoRefreshEnabled }]"
              @click="toggleAutoRefresh"
          >
            {{
              isAutoRefreshEnabled
                  ? t("menu.refreshStop")
                  : t("menu.refreshStart")
            }}
          </button>
          <select
              :value="selectedRefreshInterval"
              @change="handleRefreshIntervalChange"
              class="refresh-interval-select"
              :disabled="!isAutoRefreshEnabled"
          >
            <option
                v-for="interval in refreshIntervals"
                :key="interval"
                :value="interval"
            >
              {{ interval }}{{ t("menu.refreshInterval") }}
            </option>
          </select>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* 主菜单容器样式 - 用于将左右菜单组两端对齐 */
.main-menu-container {
  display: flex;
  justify-content: flex-start; /* 改为flex-start，使用margin-left: auto来分离元素 */
  align-items: center;
  width: 100%;
  padding: 6px 10px; /* 恢复左右padding以提供内边距 */
  background-color: #e2e8f0;
  border-bottom: 1px solid #cbd5e1;
  box-shadow:
      0 1px 3px rgba(0, 0, 0, 0.12),
      0 1px 2px rgba(0, 0, 0, 0.24);
  min-height: 32px;
  position: sticky; /* 使菜单栏固定在顶部 */
  top: 0; /* 固定在顶部 */
  z-index: 100; /* 确保菜单栏在其他内容之上 */
  flex-wrap: nowrap; /* 防止换行 */
  min-width: fit-content; /* 确保菜单栏宽度适应内容 */
}

.menu-group {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0; /* 防止菜单组被压缩 */
  flex-wrap: nowrap; /* 防止组内元素换行 */
  white-space: nowrap; /* 防止文字换行 */
  min-width: fit-content; /* 确保内容适应其内容 */
  padding-right: 10px;
}

/* 特别针对包含主题和语言选择的菜单组 */
.menu-group:last-child {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
  flex-wrap: nowrap;
  white-space: nowrap;
}

.menu-label {
  font-size: 0.8rem;
  color: #334155;
  font-weight: 500;
  white-space: nowrap;
}

.protocol-buttons {
  display: flex;
  gap: 2px;
}

.protocol-btn {
  padding: 3px 8px;
  border: 1px solid #cbd5e1;
  background-color: #e2e8f0;
  color: #475569;
  font-size: 0.75rem;
  border-radius: 3px;
  cursor: pointer;
  transition: all 0.2s ease;
  min-width: 40px;
  text-align: center;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 500;
}

.protocol-btn:hover {
  background-color: #f1f5f9;
  border-color: #94a3b8;
  color: #334155;
}

.protocol-btn.active {
  background-color: #f8fafc;
  color: #1e293b;
  border: 1px solid #94a3b8;
  box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.1);
  font-weight: 600;
}

.menu-search {
  padding: 3px 6px;
  border: 1px solid #94a3b8;
  border-radius: 3px;
  font-size: 0.75rem;
  background-color: #ffffff;
  color: #1e293b;
  min-width: 100px; /* 减少最小宽度以节省空间 */
  max-width: 150px; /* 限制最大宽度 */
  box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.05);
  flex-shrink: 0; /* 防止搜索框被压缩 */
}

.menu-search:focus {
  outline: none;
  border-color: #3b82f6;
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.3);
}

.refresh-controls {
  display: flex;
  align-items: center;
  gap: 5px;
  flex-shrink: 0; /* 防止刷新控件被压缩 */
  flex: none; /* 禁止伸缩 */
}

.refresh-toggle-btn {
  padding: 3px 8px;
  border: 1px solid #cbd5e1;
  background-color: #e2e8f0;
  color: #475569;
  font-size: 0.75rem;
  border-radius: 3px;
  cursor: pointer;
  transition: all 0.2s ease;
  min-width: 40px;
  text-align: center;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 500;
}

.refresh-toggle-btn:hover {
  background-color: #f1f5f9;
  border-color: #94a3b8;
  color: #334155;
}

.refresh-toggle-btn.active {
  background-color: #10b981; /* 绿色表示激活状态 */
  color: white;
  border: 1px solid #059669;
  box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.1);
  font-weight: 600;
}

.refresh-interval-select {
  padding: 2px 4px;
  border: 1px solid #94a3b8;
  border-radius: 3px;
  font-size: 0.75rem;
  background-color: #ffffff;
  color: #1e293b;
  min-width: 60px;
  box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.05);
}

.refresh-interval-select:disabled {
  background-color: #e2e8f0;
  color: #94a3b8;
  cursor: not-allowed;
}

/* 语言切换下拉框样式 */
.lang-select {
  padding: 3px 8px;
  border: 1px solid #cbd5e1;
  color: #475569;
  font-size: 0.75rem;
  border-radius: 3px;
  cursor: pointer;
  transition: all 0.2s ease;
  min-width: 80px;
  font-weight: 500;
  background-color: #e2e8f0;
}

.lang-select:hover {
  background-color: #f1f5f9;
  border-color: #94a3b8;
  color: #334155;
}

.lang-select:focus {
  outline: none;
  border-color: #3b82f6;
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.3);
}

.state-select {
  padding: 3px 8px;
  border: 1px solid #cbd5e1;
  color: #475569;
  font-size: 0.75rem;
  border-radius: 3px;
  cursor: pointer;
  transition: all 0.2s ease;
  min-width: 80px;
  font-weight: 500;
  background-color: #e2e8f0;
}

.state-select:hover {
  background-color: #f1f5f9;
  border-color: #94a3b8;
  color: #334155;
}

.state-select:focus {
  outline: none;
  border-color: #3b82f6;
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.3);
}

/* 主题切换按钮样式 */
.theme-toggle-btn {
  padding: 3px 8px;
  border: 1px solid #cbd5e1;
  background-color: #e2e8f0;
  color: #475569;
  font-size: 0.75rem;
  border-radius: 3px;
  cursor: pointer;
  transition: all 0.2s ease;
  min-width: fit-content;
  text-align: center;
  font-weight: 500;
}

.theme-toggle-btn:hover {
  background-color: #f1f5f9;
  border-color: #94a3b8;
  color: #334155;
}

/* 暗色主题样式 */
.dark .theme-toggle-btn {
  background-color: #2d3748; /* 深中灰蓝 */
  color: #a0aec0; /* 中等亮度的灰蓝 */
  border: 1px solid #4a5568; /* 中灰蓝 */
}

.dark .theme-toggle-btn:hover {
  background-color: #4a5568; /* 中灰蓝 */
  border-color: #718096; /* 浅中灰 */
  color: #e2e8f0; /* 浅灰蓝 */
}

/* 关于按钮样式 */
.about-btn {
  padding: 3px 8px;
  border: 1px solid #cbd5e1;
  background-color: #e2e8f0;
  color: #475569;
  font-size: 0.75rem;
  border-radius: 3px;
  cursor: pointer;
  transition: all 0.2s ease;
  min-width: fit-content;
  text-align: center;
  font-weight: 500;
}

.about-btn:hover {
  background-color: #f1f5f9;
  border-color: #94a3b8;
  color: #334155;
}

/* 暗色主题样式 */
.dark .about-btn {
  background-color: #2d3748; /* 深中灰蓝 */
  color: #a0aec0; /* 中等亮度的灰蓝 */
  border: 1px solid #4a5568; /* 中灰蓝 */
}

.dark .about-btn:hover {
  background-color: #4a5568; /* 中灰蓝 */
  border-color: #718096; /* 浅中灰 */
  color: #e2e8f0; /* 浅灰蓝 */
}

/* 暗色主题样式 */
.dark .main-menu-container {
  background-color: #1a202c; /* 深灰蓝 */
  color: #a0aec0; /* 中等亮度的灰蓝，确保可读性 */
  border-bottom: 2px solid #2d3748; /* 深中灰蓝 */
}

.dark .menu-label {
  color: #a0aec0; /* 中等亮度的灰蓝，与菜单栏文字颜色一致 */
}

.dark .protocol-btn {
  background-color: #2d3748; /* 深中灰蓝 */
  color: #a0aec0; /* 中等亮度的灰蓝 */
  border: 1px solid #4a5568; /* 中灰蓝 */
}

.dark .protocol-btn:hover {
  background-color: #4a5568; /* 中灰蓝 */
  border-color: #718096; /* 浅中灰 */
  color: #cbd5e0; /* 浅灰蓝 */
}

.dark .protocol-btn.active {
  background-color: #4c6ef5; /* 深蓝色 */
  color: #e2e8f0; /* 浅灰蓝 */
  border: 1px solid #6c8aee; /* 浅蓝色 */
}

.dark .menu-search {
  background-color: #0f1419; /* 极深的灰蓝 */
  color: #a0aec0; /* 中等亮度的灰蓝 */
  border: 1px solid #4a5568; /* 中灰蓝 */
}

.dark .menu-search:focus {
  border-color: #4c6ef5; /* 深蓝色，与整体风格一致 */
  box-shadow: 0 0 0 2px rgba(76, 110, 245, 0.3); /* 深蓝色透明阴影 */
}

.dark .refresh-toggle-btn {
  background-color: #2d3748; /* 深中灰蓝 */
  color: #a0aec0; /* 中等亮度的灰蓝 */
  border: 1px solid #4a5568; /* 中灰蓝 */
}

.dark .refresh-toggle-btn:hover {
  background-color: #4a5568; /* 中灰蓝 */
  border-color: #718096; /* 浅中灰 */
  color: #cbd5e0; /* 浅灰蓝 */
}

.dark .refresh-toggle-btn.active {
  background-color: #38a169; /* 深橄榄绿 */
  color: #e2e8f0; /* 浅灰蓝 */
  border: 1px solid #4ea27a; /* 浅橄榄绿 */
}

/* 菜单栏中select元素的暗色主题样式 */
.dark .lang-select,
.dark .state-select,
.dark .refresh-interval-select {
  background-color: #2d3748; /* 深中灰蓝 */
  color: #a0aec0; /* 中等亮度的灰蓝 */
  border: 1px solid #4a5568; /* 中灰蓝 */
}

.dark .lang-select:hover,
.dark .state-select:hover,
.dark .refresh-interval-select:hover {
  background-color: #4a5568; /* 中灰蓝 */
  border-color: #718096; /* 浅中灰 */
  color: #e2e8f0; /* 浅灰蓝 */
}

.dark .lang-select:focus,
.dark .state-select:focus,
.dark .refresh-interval-select:focus {
  outline: none;
  border-color: #4c6ef5; /* 深蓝色 */
  box-shadow: 0 0 0 2px rgba(76, 110, 245, 0.3); /* 深蓝色透明阴影 */
}

.dark .refresh-interval-select:disabled {
  background-color: #4a5568; /* 中灰蓝 */
  color: #718096; /* 中灰 */
  cursor: not-allowed;
}
</style>