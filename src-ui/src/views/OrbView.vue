<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { RefreshCw } from "lucide-vue-next";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useTokenBallStore } from "../store";
import { refreshAllQuota, showWindow, hideWindow } from "../services/tauri";

const store = useTokenBallStore();
const index = ref(0);
const paused = ref(false);
let timer: number | undefined;

const views = computed(() => [
  { title: "当前", value: store.percentLabel },
  { title: "可用", value: `${store.summary.availableAccounts}/${store.summary.totalAccounts}` },
  { title: "恢复", value: resetLabel(store.summary.nearestResetAt) }
]);
const current = computed(() => views.value[index.value % views.value.length]);
const percent = computed(() => store.summary.lowestRemainingPercent ?? 0);
const stateClass = computed(() => {
  if (!store.hasConnection) return "setup";
  if (store.summary.stale || store.summary.status === "degraded") return "stale";
  if (typeof store.summary.lowestRemainingPercent !== "number") return "unknown";
  if (store.summary.lowestRemainingPercent <= 10) return "critical";
  if (store.summary.lowestRemainingPercent <= 20) return "warning";
  return "normal";
});

onMounted(() => {
  timer = window.setInterval(() => {
    if (!paused.value) index.value += 1;
  }, 4000);
});

onUnmounted(() => {
  if (timer) window.clearInterval(timer);
});

async function openHover() {
  paused.value = true;
  await showWindow("hover");
}

async function closeHover() {
  paused.value = false;
  window.setTimeout(() => hideWindow("hover"), 250);
}

function resetLabel(value?: string | null) {
  if (!value) return "--";
  const minutes = Math.max(0, Math.round((new Date(value).getTime() - Date.now()) / 60000));
  if (minutes < 60) return `${minutes}m`;
  return `${Math.round(minutes / 60)}h`;
}
</script>

<template>
  <main
    class="orb-shell"
    :class="stateClass"
    @mouseenter="openHover"
    @mouseleave="closeHover"
    @dblclick="showWindow('main')"
  >
    <div class="liquid" :style="{ height: `${Math.max(8, Math.min(92, percent))}%` }"></div>
    <button class="drag" title="拖动" @mousedown="getCurrentWindow().startDragging()"></button>
    <div class="orb-content">
      <strong>{{ store.hasConnection ? current.value : '设置' }}</strong>
      <span>{{ store.hasConnection ? current.title : 'CLI' }}</span>
    </div>
    <button class="refresh" title="刷新" @click.stop="refreshAllQuota()">
      <RefreshCw :size="13" :class="{ spin: store.refreshing }" />
    </button>
  </main>
</template>
