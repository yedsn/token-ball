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
let hoverTimer: number | undefined;

const views = computed(() => [
  ...(store.displaySettings.showTotalRemaining
    ? [{ title: "总量", value: percentText(store.totalRemainingPercent), percent: store.totalRemainingPercent }]
    : []),
  ...(store.displaySettings.showAvailableAccounts
    ? [{ title: "可用", value: `${store.summary.availableAccounts}/${store.summary.totalAccounts}`, percent: store.totalRemainingPercent }]
    : []),
  ...(store.displaySettings.showConnectionStatus
    ? [{ title: "连接", value: store.summary.status, percent: store.totalRemainingPercent }]
    : []),
  ...(store.displaySettings.showAccountsInTooltip
    ? store.summary.accounts
        .filter((account) => accountPercentValue(account) > 0)
        .map((account) => ({ title: shortName(account.displayName), value: accountPercent(account), percent: accountPercentValue(account) }))
    : []),
  ...store.displaySettings.customItems
    .filter((item) => item.enabled)
    .map((item) => ({ title: shortName(item.label || "自定义"), value: shortValue(item.value || "--"), percent: store.totalRemainingPercent })),
  { title: "Token", value: store.percentLabel, percent: store.totalRemainingPercent }
]);
const current = computed(() => views.value[index.value % views.value.length]);
const percent = computed(() => current.value?.percent ?? store.totalRemainingPercent ?? store.summary.lowestRemainingPercent ?? 0);
const stateClass = computed(() => {
  if (!store.ready) return "loading";
  if (!store.hasConnection) return "setup";
  if (store.summary.stale || store.summary.status === "degraded") return "stale";
  const value = percent.value;
  if (typeof value !== "number") return "unknown";
  if (value < 30) return "critical";
  if (value < 60) return "warning";
  return "normal";
});

onMounted(() => {
  timer = window.setInterval(() => {
    if (!paused.value) index.value += 1;
  }, 4000);
});

onUnmounted(() => {
  if (timer) window.clearInterval(timer);
  if (hoverTimer) window.clearTimeout(hoverTimer);
});

async function openHover() {
  paused.value = true;
  if (hoverTimer) window.clearTimeout(hoverTimer);
  hoverTimer = window.setTimeout(() => showWindow("hover"), 420);
}

async function closeHover() {
  paused.value = false;
  if (hoverTimer) window.clearTimeout(hoverTimer);
  window.setTimeout(() => hideWindow("hover"), 180);
}

function resetLabel(value?: string | null) {
  if (!value) return "--";
  const minutes = Math.max(0, Math.round((new Date(value).getTime() - Date.now()) / 60000));
  if (minutes < 60) return `${minutes}m`;
  return `${Math.round(minutes / 60)}h`;
}

function accountPercent(account: any) {
  return percentText(accountPercentValue(account));
}

function accountPercentValue(account: any) {
  const window = account.windows.find((item: any) => item.id === account.criticalWindowId);
  return typeof window?.remainingPercent === "number" ? window.remainingPercent : null;
}

function percentText(value?: number | null) {
  return typeof value === "number" ? `${Math.round(value)}%` : "未知";
}

function shortName(value: string) {
  return value.length > 4 ? value.slice(0, 4) : value;
}

function shortValue(value: string) {
  return value.length > 5 ? value.slice(0, 5) : value;
}

async function hideOrb() {
  if (hoverTimer) window.clearTimeout(hoverTimer);
  await hideWindow("hover");
  await hideWindow("orb");
}
</script>

<template>
  <main
    class="orb-shell"
    :class="[stateClass, { still: !store.displaySettings.orbAnimationEnabled }]"
    @mouseenter="openHover"
    @mouseleave="closeHover"
    @contextmenu.prevent="hideOrb"
    @dblclick="showWindow('main')"
  >
    <div class="energy-ring"></div>
    <div class="liquid" :style="{ height: `${Math.max(8, Math.min(92, percent))}%` }"></div>
    <div class="orb-gloss"></div>
    <button class="drag" title="拖动" @mousedown="getCurrentWindow().startDragging()"></button>
    <div class="orb-content">
      <strong>{{ !store.ready ? '同步' : store.hasConnection ? current.value : '设置' }}</strong>
      <span>{{ !store.ready ? '加载中' : store.hasConnection ? current.title : 'CLI' }}</span>
    </div>
    <button v-if="store.displaySettings.showOrbRefreshButton" class="refresh" title="刷新" @click.stop="refreshAllQuota()">
      <RefreshCw :size="13" :class="{ spin: store.refreshing }" />
    </button>
  </main>
</template>
