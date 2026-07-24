<script setup lang="ts">
import { computed, reactive, ref, watch } from "vue";
import { CheckCircle2, RefreshCw, Save, Settings2, Trash2, Wifi } from "lucide-vue-next";
import { useTokenBallStore } from "../store";
import type { QuotaAccount } from "../types";

const store = useTokenBallStore();
const saving = ref(false);
const testing = ref(false);
const notice = ref("");
const form = reactive({
  id: "",
  displayName: "本机 CLIProxyAPI",
  baseUrl: "http://127.0.0.1:3000",
  managementKey: ""
});

const currentConnection = computed(() => {
  return store.connections.find((connection) => connection.id === form.id) ?? store.connections[0];
});

watch(
  () => store.connections,
  (connections) => {
    const connection = currentConnection.value ?? connections[0];
    if (!connection || form.id) return;
    form.id = connection.id;
    form.displayName = connection.displayName;
    form.baseUrl = connection.baseUrl;
  },
  { immediate: true }
);

async function save() {
  saving.value = true;
  notice.value = "";
  try {
    const connection = await store.saveCliProxyConnection({ ...form, id: form.id || currentConnection.value?.id });
    form.id = connection.id;
    form.displayName = connection.displayName;
    form.baseUrl = connection.baseUrl;
    form.managementKey = "";
    notice.value = "连接已保存";
    return connection;
  } catch (error) {
    notice.value = String(error);
    throw error;
  } finally {
    saving.value = false;
  }
}

async function test() {
  testing.value = true;
  notice.value = "";
  try {
    const connection = form.managementKey.trim() ? await save() : currentConnection.value;
    const id = connection?.id;
    if (!id) return;
    await store.testCliProxyConnection(id);
    notice.value = "连接测试成功";
  } catch (error) {
    notice.value = String(error);
  } finally {
    testing.value = false;
  }
}

async function removeCurrent() {
  const id = currentConnection.value?.id;
  if (!id) return;
  await store.deleteCliProxyConnection(id);
  form.id = "";
  form.managementKey = "";
  notice.value = "连接已删除";
}

function quotaLabel(account: QuotaAccount) {
  const window = account.windows.find((item) => item.id === account.criticalWindowId);
  return typeof window?.remainingPercent === "number" ? `${Math.round(window.remainingPercent)}%` : "未知";
}

function resetLabel(value?: string | null) {
  if (!value) return "--";
  return new Date(value).toLocaleString("zh-CN", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" });
}

function activityLabel(account: QuotaAccount) {
  const latest = account.recentRequests?.[account.recentRequests.length - 1];
  if (latest) return `${latest.time} 成功 ${latest.success} / 失败 ${latest.failed}`;
  const success = account.successCount ?? 0;
  const failed = account.failedCount ?? 0;
  if (success || failed) return `累计成功 ${success} / 失败 ${failed}`;
  return "CLIProxyAPI 未返回余量，仅可显示账号状态";
}

function dateLabel(value?: string | null) {
  if (!value) return "";
  return new Date(value).toLocaleDateString("zh-CN");
}
</script>

<template>
  <main class="management-shell">
    <aside>
      <div class="brand-mark">TB</div>
      <button class="nav active"><Settings2 :size="16" />总览</button>
      <button class="nav"><Wifi :size="16" />CLIProxyAPI</button>
    </aside>

    <section class="workspace">
      <header class="topbar">
        <div>
          <h1>TokenBall</h1>
          <p>CLIProxyAPI 额度余量球</p>
        </div>
        <button class="primary" @click="store.refresh" :disabled="store.refreshing || !store.hasConnection">
          <RefreshCw :size="16" :class="{ spin: store.refreshing }" />立即刷新
        </button>
      </header>

      <section class="metrics">
        <article>
          <span>额度余量</span>
          <strong>{{ store.percentLabel }}</strong>
        </article>
        <article>
          <span>可用账号</span>
          <strong>{{ store.summary.availableAccounts }} / {{ store.summary.totalAccounts }}</strong>
        </article>
        <article>
          <span>连接状态</span>
          <strong>{{ store.summary.status }}</strong>
        </article>
      </section>

      <section class="content-grid">
        <form class="panel" @submit.prevent="save">
          <h2>CLIProxyAPI 连接</h2>
          <label>显示名称<input v-model="form.displayName" /></label>
          <label>服务地址<input v-model="form.baseUrl" /></label>
          <label>管理 Key<input v-model="form.managementKey" type="password" autocomplete="off" /></label>
          <p class="muted">这里需要 remote-management.secret-key 或 MANAGEMENT_PASSWORD，不是普通 API Key。</p>
          <div class="actions">
            <button class="primary" type="submit" :disabled="saving"><Save :size="16" />保存</button>
            <button type="button" @click="test" :disabled="testing || (!store.hasConnection && !form.managementKey.trim())"><CheckCircle2 :size="16" />测试</button>
            <button type="button" @click="removeCurrent" :disabled="!currentConnection"><Trash2 :size="16" />删除</button>
          </div>
          <p v-if="currentConnection" class="muted">已保存：{{ currentConnection.displayName }} · {{ currentConnection.maskedManagementKey }}</p>
          <p v-if="notice" class="notice">{{ notice }}</p>
        </form>

        <section class="panel account-panel">
          <h2>账号状态</h2>
          <p v-if="store.hasConnection && store.summary.lowestRemainingPercent === null" class="muted quota-note">当前 CLIProxyAPI 管理接口未返回真实余量百分比，下面展示账号可用性和请求活动。</p>
          <div v-if="!store.hasConnection" class="empty-state">保存 CLIProxyAPI 连接后开始同步账号。</div>
          <div v-else-if="store.summary.accounts.length === 0" class="empty-state">已连接，点击立即刷新同步账号；如果仍为空，请查看测试返回内容。</div>
          <article v-for="account in store.summary.accounts" :key="account.id" class="account-line">
            <div>
              <strong>{{ account.displayName }}</strong>
              <span>{{ account.planName }} · {{ account.status }}<template v-if="account.subscriptionUntil"> · 到期 {{ dateLabel(account.subscriptionUntil) }}</template></span>
              <span>{{ activityLabel(account) }}</span>
              <div v-if="account.windows.length" class="quota-windows">
                <span v-for="window in account.windows" :key="window.id">
                  {{ window.name }} {{ typeof window.remainingPercent === 'number' ? `${Math.round(window.remainingPercent)}%` : '未知' }} · {{ resetLabel(window.resetAt) }}
                </span>
              </div>
            </div>
            <b>{{ quotaLabel(account) }}</b>
          </article>
        </section>
      </section>
    </section>
  </main>
</template>
