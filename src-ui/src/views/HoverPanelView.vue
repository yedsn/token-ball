<script setup lang="ts">
import { RefreshCw } from "lucide-vue-next";
import { useTokenBallStore } from "../store";

const store = useTokenBallStore();

function percent(account: any) {
  const window = account.windows.find((item: any) => item.id === account.criticalWindowId);
  return typeof window?.remainingPercent === "number" ? `${Math.round(window.remainingPercent)}%` : "未知";
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
        <strong>{{ store.summary.availableAccounts }} / {{ store.summary.totalAccounts }} 可用</strong>
      </div>
      <RefreshCw :size="16" :class="{ spin: store.refreshing }" />
    </header>

    <section v-if="!store.hasConnection" class="empty-panel">尚未配置 CLIProxyAPI</section>
    <section v-else class="account-list">
      <article v-for="account in store.summary.accounts" :key="account.id" class="account-row">
        <div>
          <strong>{{ account.displayName }}</strong>
          <span>{{ account.maskedIdentifier || account.planName }}</span>
        </div>
        <div class="quota-cell">
          <b>{{ percent(account) }}</b>
          <span>{{ account.status }}</span>
        </div>
      </article>
    </section>

    <footer>
      <span :class="{ stale: store.summary.stale }">{{ store.summary.stale ? '缓存数据' : '最新数据' }}</span>
      <span>同步 {{ reset(store.summary.lastSyncedAt) }}</span>
    </footer>
  </main>
</template>
