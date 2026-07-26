<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { RefreshCw } from "lucide-vue-next";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { hideWindow } from "../services/tauri";
import { useTokenBallStore } from "../store";
import type { QuotaAccount, QuotaWindow } from "../types";

type BalanceRankingRow = { account: QuotaAccount; window: QuotaWindow | null };
type BalancePeriod = "fiveHour" | "weekly" | "monthly";

const store = useTokenBallStore();
const visible = ref(false);
const pointerInsidePanel = ref(false);
const denseTableRef = ref<HTMLElement | null>(null);
let leaveTimer: number | undefined;
let unlistenOrbEnter: UnlistenFn | undefined;
let unlistenOrbLeave: UnlistenFn | undefined;
let unlistenFocusChanged: UnlistenFn | undefined;
const panelHideDelayMs = 900;

const balanceExpiryRanking = computed<BalanceRankingRow[]>(() => {
  const rows: BalanceRankingRow[] = store.enabledAccounts.map((account) => ({ account, window: primaryExpiryWindow(account) }));
  return rows.sort((left, right) => {
    const leftTime = balanceResetTime(left);
    const rightTime = balanceResetTime(right);
    if (leftTime === null && rightTime === null) return left.account.displayName.localeCompare(right.account.displayName, "zh-CN");
    if (leftTime === null) return 1;
    if (rightTime === null) return -1;
    if (leftTime !== rightTime) return leftTime - rightTime;
    return left.account.displayName.localeCompare(right.account.displayName, "zh-CN");
  });
});

const totalBalanceClass = computed(() => {
  const percent = store.totalRemainingPercent;
  if (typeof percent !== "number") return "unknown";
  if (percent < 30) return "critical";
  if (percent < 60) return "warning";
  return "healthy";
});

const enabledAccountCountLabel = computed(() => `${store.enabledAccounts.length} / ${store.summary.totalAccounts}`);

function accountPercentValue(account: QuotaAccount) {
  const window = account.windows.find((item) => item.id === account.criticalWindowId);
  return typeof window?.remainingPercent === "number" ? window.remainingPercent : null;
}

function quotaLabel(account: QuotaAccount) {
  const window = account.windows.find((item) => item.id === account.criticalWindowId);
  if (typeof window?.remainingPercent === "number") return `${Math.round(window.remainingPercent)}%`;
  const usageWindow = account.windows.find((item) => typeof item.used === "number");
  if (usageWindow) return formatWindowUsed(usageWindow);
  return "未知";
}

function accountBalanceClass(account: QuotaAccount) {
  const percent = accountPercentValue(account);
  if (percent === null) return "unknown";
  if (percent < 30) return "critical";
  if (percent < 60) return "warning";
  return "healthy";
}

function accountConnectionLabel(account: QuotaAccount) {
  return store.connections.find((connection) => connection.id === account.connectionId)?.displayName ?? "未知实例";
}

function primaryExpiryWindow(account: QuotaAccount) {
  return findExpiryWindow(account, "monthly")
    ?? highestQuotaWindow(account)
    ?? account.windows.find((window) => window.id === account.criticalWindowId && window.resetAt)
    ?? account.windows.find((window) => window.resetAt)
    ?? null;
}

function findExpiryWindow(account: QuotaAccount, period: "weekly" | "monthly") {
  return account.windows.find((window) => window.resetAt && windowPeriodKey(window) === period) ?? null;
}

function windowPeriodKey(window: QuotaWindow) {
  const value = `${window.periodType} ${window.id} ${window.name}`.toLowerCase();
  if (value.includes("5h") || value.includes("5 h") || window.periodSeconds === 5 * 60 * 60) return "fiveHour";
  if (value.includes("weekly") || value.includes("week") || value.includes("7d") || value.includes("每周") || value.includes("近 1 周")) return "weekly";
  if (value.includes("monthly") || value.includes("month") || value.includes("30d") || value.includes("每月") || value.includes("近 1 月")) return "monthly";
  return "other";
}

function highestQuotaWindow(account: QuotaAccount) {
  return account.windows
    .filter((window) => window.resetAt)
    .sort((left, right) => windowPriority(right) - windowPriority(left))[0] ?? null;
}

function windowPriority(window: QuotaWindow) {
  const period = windowPeriodKey(window);
  if (period === "monthly") return 30 * 24 * 60 * 60;
  if (period === "weekly") return 7 * 24 * 60 * 60;
  if (period === "fiveHour") return 5 * 60 * 60;
  if (typeof window.periodSeconds === "number") return window.periodSeconds;
  return typeof window.total === "number" ? window.total : 0;
}

function accountPeriodWindow(account: QuotaAccount, period: BalancePeriod) {
  return account.windows.find((window) => windowPeriodKey(window) === period) ?? null;
}

function accountPeriodRemainingLabel(account: QuotaAccount, period: BalancePeriod) {
  const window = accountPeriodWindow(account, period);
  return window ? windowPercentLabel(window) : "--";
}

function accountPeriodRemainingClass(account: QuotaAccount, period: BalancePeriod) {
  const window = accountPeriodWindow(account, period);
  return window ? windowClass(window) : "unknown";
}

function balanceResetTime(row: BalanceRankingRow) {
  const value = row.window?.resetAt ?? row.account.nextResetAt;
  if (!value) return null;
  const time = new Date(value).getTime();
  return Number.isFinite(time) && time > 0 ? time : null;
}

function balanceRowClass(row: BalanceRankingRow) {
  return row.window ? windowClass(row.window) : accountBalanceClass(row.account);
}

function balanceWindowName(row: BalanceRankingRow) {
  return row.window?.name ?? "账号状态";
}

function balanceRemainingLabel(row: BalanceRankingRow) {
  return row.window ? windowPercentLabel(row.window) : quotaLabel(row.account);
}

function balanceUsageLabel(row: BalanceRankingRow) {
  return row.window ? windowUsageLabel(row.window) : activityLabel(row.account);
}

function balanceResetLabel(row: BalanceRankingRow) {
  return reset(row.window?.resetAt ?? row.account.nextResetAt);
}

function balanceResetClass(row: BalanceRankingRow) {
  const time = balanceResetTime(row);
  if (time === null) return "unknown";
  const days = (time - Date.now()) / 86400000;
  if (days <= 3) return "critical";
  if (days <= 7) return "warning";
  return "healthy";
}

function windowPercent(window: QuotaWindow) {
  return typeof window.remainingPercent === "number" ? Math.round(window.remainingPercent) : null;
}

function windowPercentLabel(window: QuotaWindow) {
  const percent = windowPercent(window);
  return percent === null ? "--" : `${percent}%`;
}

function windowUsageLabel(window: QuotaWindow) {
  if (window.unit === "percent") {
    const used = typeof window.used === "number" ? `${formatQuotaNumber(window.used)}%` : "--";
    const total = typeof window.total === "number" ? `${formatQuotaNumber(window.total)}%` : "--";
    return `${used} / ${total}`;
  }
  if (typeof window.used === "number" && typeof window.total !== "number") return formatWindowUsed(window);
  const used = typeof window.used === "number" ? formatQuotaNumber(window.used) : "--";
  const total = typeof window.total === "number" ? formatQuotaNumber(window.total) : "--";
  return `${used} / ${total}`;
}

function formatWindowUsed(window: QuotaWindow) {
  if (typeof window.used !== "number") return "--";
  const unit = window.unit === "token" ? "Tokens" : window.unit === "request" ? "次" : window.unit === "credit" ? "Credit" : window.unit === "afp" ? "AFP" : "";
  return `${formatCompactNumber(window.used)}${unit ? ` ${unit}` : ""}`;
}

function formatCompactNumber(value: number) {
  const abs = Math.abs(value);
  if (abs >= 1_000_000_000) return `${formatQuotaNumber(value / 1_000_000_000)}B`;
  if (abs >= 1_000_000) return `${formatQuotaNumber(value / 1_000_000)}M`;
  if (abs >= 1_000) return `${formatQuotaNumber(value / 1_000)}K`;
  return formatQuotaNumber(value);
}

function formatQuotaNumber(value: number) {
  return Number.isInteger(value) ? String(value) : value.toFixed(2).replace(/\.00$/, "");
}

function windowClass(window: QuotaWindow) {
  const percent = windowPercent(window);
  if (percent === null) return "unknown";
  if (percent < 30) return "critical";
  if (percent < 60) return "warning";
  return "healthy";
}

function activityLabel(account: QuotaAccount) {
  const latest = account.recentRequests?.[account.recentRequests.length - 1];
  if (latest) return `${latest.success}/${latest.failed} · ${latest.time}`;
  const success = account.successCount ?? 0;
  const failed = account.failedCount ?? 0;
  return success || failed ? `${success}/${failed} 累计` : account.planName;
}

function reset(value?: string | null) {
  if (!value) return "--";
  return new Date(value).toLocaleString("zh-CN", { hour: "2-digit", minute: "2-digit", month: "2-digit", day: "2-digit" });
}

function showDelayed() {
  pointerInsidePanel.value = true;
  visible.value = true;
  if (leaveTimer) window.clearTimeout(leaveTimer);
}

async function showFromOrb() {
  visible.value = true;
  if (leaveTimer) window.clearTimeout(leaveTimer);
  await store.loadConnections();
  resetListScroll();
}

async function refreshPanel() {
  try {
    await store.refresh();
  } finally {
    await store.loadConnections();
  }
}

function resetListScroll() {
  window.requestAnimationFrame(() => {
    if (denseTableRef.value) denseTableRef.value.scrollTop = 0;
  });
}

function hideDelayed() {
  pointerInsidePanel.value = false;
  if (leaveTimer) window.clearTimeout(leaveTimer);
  leaveTimer = window.setTimeout(() => {
    hideNow();
  }, panelHideDelayMs);
}

function hideAfterOrbLeave() {
  if (leaveTimer) window.clearTimeout(leaveTimer);
  leaveTimer = window.setTimeout(() => {
    if (!pointerInsidePanel.value) hideNow();
  }, panelHideDelayMs);
}

function hideNow() {
  if (leaveTimer) window.clearTimeout(leaveTimer);
  leaveTimer = undefined;
  visible.value = false;
  pointerInsidePanel.value = false;
  hideWindow("hover");
}

function closeIfOutside(focused: boolean) {
  if (!focused && !pointerInsidePanel.value) hideNow();
}

onMounted(async () => {
  showFromOrb();
  unlistenOrbEnter = await listen("hover://orb-enter", showFromOrb);
  unlistenOrbLeave = await listen("hover://orb-leave", hideAfterOrbLeave);
  unlistenFocusChanged = await getCurrentWindow().onFocusChanged((event) => closeIfOutside(event.payload));
});

onUnmounted(() => {
  if (leaveTimer) window.clearTimeout(leaveTimer);
  unlistenOrbEnter?.();
  unlistenOrbLeave?.();
  unlistenFocusChanged?.();
});
</script>

<template>
  <main class="hover-panel" :class="{ visible }" @mouseenter="showDelayed" @mouseleave="hideDelayed">
    <header class="hover-panel-hero" :class="totalBalanceClass">
      <div class="hero-copy">
        <span>额度</span>
        <strong>{{ typeof store.totalRemainingPercent === 'number' ? Math.round(store.totalRemainingPercent) + '%' : store.percentLabel }}</strong>
        <small>{{ enabledAccountCountLabel }} 启用账号</small>
      </div>
      <button class="hover-refresh" type="button" title="刷新" :disabled="store.refreshing" @click="refreshPanel">
        <RefreshCw :size="16" :class="{ spin: store.refreshing }" />
      </button>
    </header>

    <section v-if="!store.hasConnection" class="empty-panel">尚未配置 CLIProxyAPI</section>
    <section v-else-if="store.enabledAccounts.length === 0" class="empty-panel">已连接，暂无启用实例账号数据</section>
    <section v-else class="hover-list-wrap">
      <div v-if="store.displaySettings.showTotalRemaining" class="hover-summary-line">
        <span>等效剩余</span>
        <strong>{{ store.totalEquivalentAccounts.toFixed(2) }} 账号</strong>
      </div>
      <div ref="denseTableRef" class="hover-dense-table">
        <article v-for="row in balanceExpiryRanking" :key="`${row.account.id}-${row.window?.id ?? 'account'}`" class="hover-dense-row" :class="balanceRowClass(row)">
          <div class="hover-dense-main">
            <div class="dense-account-title">
              <strong>{{ row.account.displayName }}</strong>
              <span>{{ row.account.planName }} · {{ row.account.status }}</span>
            </div>
            <b class="dense-total-value">{{ balanceRemainingLabel(row) }}</b>
          </div>
          <div class="hover-period-row">
            <span><em>5 小时</em><b class="balance-quota" :class="accountPeriodRemainingClass(row.account, 'fiveHour')">{{ accountPeriodRemainingLabel(row.account, 'fiveHour') }}</b></span>
            <span><em>周</em><b class="balance-quota" :class="accountPeriodRemainingClass(row.account, 'weekly')">{{ accountPeriodRemainingLabel(row.account, 'weekly') }}</b></span>
            <span><em>月</em><b class="balance-quota" :class="accountPeriodRemainingClass(row.account, 'monthly')">{{ accountPeriodRemainingLabel(row.account, 'monthly') }}</b></span>
          </div>
          <div class="hover-secondary-row">
            <span>到期 <b class="balance-reset" :class="balanceResetClass(row)">{{ balanceResetLabel(row) }}</b></span>
            <span>{{ balanceWindowName(row) }}</span>
            <span>{{ balanceUsageLabel(row) }}</span>
            <span>{{ accountConnectionLabel(row.account) }}</span>
          </div>
        </article>
      </div>
      <article v-for="item in store.displaySettings.customItems.filter((item) => item.enabled)" :key="item.id" class="account-row total-row">
        <div>
          <strong>{{ item.label || '自定义' }}</strong>
          <span>手动显示内容</span>
        </div>
        <div class="quota-cell">
          <b>{{ item.value || '--' }}</b>
          <span>显示</span>
        </div>
      </article>
    </section>

    <footer class="hover-panel-foot">
      <span :class="{ stale: store.summary.stale }">{{ store.summary.stale ? '缓存数据' : '最新数据' }}</span>
      <span>同步 {{ reset(store.summary.lastSyncedAt) }}</span>
    </footer>
  </main>
</template>
