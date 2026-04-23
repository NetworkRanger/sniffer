<script setup lang="ts">
import {ref, computed, onMounted, onUnmounted} from 'vue';

interface Props {
  items: unknown[];
  itemHeight: number;  // estimated row height px
  overscan?: number;
}

const props = withDefaults(defineProps<Props>(), {overscan: 5});

const containerRef = ref<HTMLDivElement>();
const scrollTop = ref(0);
const containerHeight = ref(0);

let ro: ResizeObserver | null = null;

onMounted(() => {
  ro = new ResizeObserver(entries => {
    containerHeight.value = entries[0].contentRect.height;
  });
  if (containerRef.value) ro.observe(containerRef.value);
});
onUnmounted(() => ro?.disconnect());

const emit = defineEmits<{scroll: [e: Event]}>();

function onScroll(e: Event) {
  scrollTop.value = (e.target as HTMLDivElement).scrollTop;
  emit('scroll', e);
}

const totalHeight = computed(() => props.items.length * props.itemHeight);

const virtualItems = computed(() => {
  const start = Math.max(0, Math.floor(scrollTop.value / props.itemHeight) - props.overscan);
  const visibleCount = Math.ceil(containerHeight.value / props.itemHeight);
  const end = Math.min(props.items.length - 1, start + visibleCount + props.overscan * 2);
  const result = [];
  for (let i = start; i <= end; i++) {
    result.push({index: i, start: i * props.itemHeight});
  }
  return result;
});

// expose scroll control
const scrollToBottom = () => {
  if (containerRef.value) {
    containerRef.value.scrollTop = containerRef.value.scrollHeight;
  }
};
const scrollToIndex = (index: number) => {
  if (containerRef.value) {
    containerRef.value.scrollTop = index * props.itemHeight;
  }
};

defineExpose({scrollToBottom, scrollToIndex});
</script>

<template>
  <div ref="containerRef" class="vlist-container" @scroll="onScroll">
    <div :style="{height: totalHeight + 'px', position: 'relative'}">
      <div
          v-for="vi in virtualItems"
          :key="vi.index"
          :style="{position:'absolute', top:0, left:0, width:'100%', transform:`translateY(${vi.start}px)`}"
      >
        <slot :item="items[vi.index]" :index="vi.index" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.vlist-container {
  width: 100%;
  height: 100%;
  overflow-y: auto;
  overflow-x: hidden;
}
</style>
