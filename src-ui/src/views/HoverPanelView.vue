<script setup lang="ts">
import { RefreshCw } from "lucide-vue-next";
import { useTokenBallStore } from "../store";
import type { QuotaAccount } from "../types";

const store = useTokenBallStore();

function accountPercentValue(account: QuotaAccount) {
  const window = account.windows.find((item) => item.id === account.criticalWindowId);
  return typeof window?.remainingPercent === "number" ? window.remainingPercent : 0;
}

function percent(account: QuotaAccount) {
  const window = account.windows.find((item) => item.id === account.criticalWindowId);
  return typeof window?.remainingPercent === "number" ? `${Math.round(window.remainingPercent)}%` : "未知";
}

function quotaDetail(account: QuotaAccount) {
  if (!account.windows.length) return activity(account);
  return account.windows
    .slice(0, 2)
    .map((window) => `${window.name} ${typeof window.remainingPercent === "number" ? Math.round(window.remainingPercent) + "%" : "未知"}`)
    .join(" · ");
}

function windowPercent(window: any) {
  return typeof window.remainingPercent === "number" ? Math.round(window.remainingPercent) : null;
}

function windowClass(window: any) {
  const percent = windowPercent(window);
  if (percent === null) return "unknown";
  if (percent < 30) return "critical";
  if (percent < 60) return "warning";
  return "healthy";
}

function expiryClass(value?: string | null) {
  if (!value) return "";
  const days = (new Date(value).getTime() - Date.now()) / 86400000;
  if (days <= 3) return "critical";
  if (days <= 7) return "warning";
  return "";
}

function expiryLabel(value?: string | null) {
  if (!value) return "";
  return new Date(value).toLocaleDateString("zh-CN", { month: "2-digit", day: "2-digit" });
}

function activity(account: QuotaAccount) {
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
</script>

<template>
  <main class="hover-panel">
    <header>
      <div>
        <p>AI 额度状态</p>
        <strong>{{ typeof store.totalRemainingPercent === 'number' ? Math.round(store.totalRemainingPercent) + '%' : store.percentLabel }} · {{ store.summary.availableAccounts }} / {{ store.summary.totalAccounts }}</strong>
      </div>
      <RefreshCw :size="16" :class="{ spin: store.refreshing }" />
    </header>

    <section v-if="!store.hasConnection" class="empty-panel">尚未配置 CLIProxyAPI</section>
    <section v-else-if="store.summary.accounts.length === 0" class="empty-panel">已连接，暂无账号数据</section>
    <section v-else class="account-list quota-mini-list">
      <article v-if="store.displaySettings.showTotalRemaining" class="account-row total-row">
        <div>
          <strong>总额度剩余</strong>
          <span>{{ store.totalEquivalentAccounts.toFixed(2) }} 个账号等效剩余</span>
        </div>
        <div class="quota-cell">
          <b>{{ typeof store.totalRemainingPercent === 'number' ? Math.round(store.totalRemainingPercent) + '%' : '未知' }}</b>
          <span>汇总</span>
        </div>
      </article>
      <article v-for="account in store.summary.accounts.filter((account) => accountPercentValue(account) > 0)" :key="account.id" class="account-row quota-mini-card">
        <div>
          <strong>{{ account.displayName }}</strong>
          <span>{{ quotaDetail(account) }}<template v-if="account.subscriptionUntil"> · <em class="expiry" :class="expiryClass(account.subscriptionUntil)">到期 {{ expiryLabel(account.subscriptionUntil) }}</em></template></span>
          <div v-if="account.windows.length" class="mini-bars">
            <div v-for="window in account.windows.slice(0, 2)" :key="window.id" class="mini-bar" :class="windowClass(window)">
              <span>{{ window.name }}</span><i><b :style="{ width: `${windowPercent(window) ?? 0}%` }"></b></i><em>{{ windowPercent(window) ?? '--' }}%</em>
            </div>
          </div>
        </div>
        <div class="quota-cell">
          <b>{{ percent(account) }}</b>
          <span>{{ account.status }}</span>
        </div>
      </article>
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

    <footer>
      <span :class="{ stale: store.summary.stale }">{{ store.summary.stale ? '缓存数据' : '最新数据' }}</span>
      <span>同步 {{ reset(store.summary.lastSyncedAt) }}</span>
    </footer>
  </main>
</template>
