<script setup lang="ts">
import { computed, reactive, ref } from "vue";
import { CheckCircle2, RefreshCw, Save, Settings2, Wifi } from "lucide-vue-next";
import { useTokenBallStore } from "../store";

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

const firstConnection = computed(() => store.connections[0]);

async function save() {
  saving.value = true;
  notice.value = "";
  try {
    const connection = await store.saveCliProxyConnection({ ...form, id: form.id || firstConnection.value?.id });
    form.id = connection.id;
    notice.value = "连接已保存";
  } catch (error) {
    notice.value = String(error);
  } finally {
    saving.value = false;
  }
}

async function test() {
  const id = form.id || firstConnection.value?.id;
  if (!id) return;
  testing.value = true;
  notice.value = "";
  try {
    await store.testCliProxyConnection(id);
    notice.value = "连接测试成功";
  } catch (error) {
    notice.value = String(error);
  } finally {
    testing.value = false;
  }
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
          <span>当前余量</span>
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
          <label>Management Key<input v-model="form.managementKey" type="password" autocomplete="off" /></label>
          <div class="actions">
            <button class="primary" type="submit" :disabled="saving"><Save :size="16" />保存</button>
            <button type="button" @click="test" :disabled="testing || !store.hasConnection"><CheckCircle2 :size="16" />测试</button>
          </div>
          <p v-if="firstConnection" class="muted">已保存：{{ firstConnection.displayName }} · {{ firstConnection.maskedManagementKey }}</p>
          <p v-if="notice" class="notice">{{ notice }}</p>
        </form>

        <section class="panel account-panel">
          <h2>账号状态</h2>
          <div v-if="!store.hasConnection" class="empty-state">保存 CLIProxyAPI 连接后开始同步额度。</div>
          <article v-for="account in store.summary.accounts" :key="account.id" class="account-line">
            <div>
              <strong>{{ account.displayName }}</strong>
              <span>{{ account.planName }} · {{ account.status }}</span>
            </div>
            <b>{{ account.windows.find(w => w.id === account.criticalWindowId)?.remainingPercent?.toFixed(0) ?? '--' }}%</b>
          </article>
        </section>
      </section>
    </section>
  </main>
</template>
