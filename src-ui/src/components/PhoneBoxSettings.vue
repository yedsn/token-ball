<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from "vue";
import { getPhoneBoxSettings, getPhoneBoxStatus, savePhoneBoxSettings, pushPhoneBoxNow } from "../services/tauri";
import type { PhoneBoxSettings, PhoneBoxStatus } from "../types";

const form = reactive<PhoneBoxSettings>({ enabled: false, baseUrl: "http://127.0.0.1:8765" });
const saved = ref<PhoneBoxSettings | null>(null);
const status = ref<PhoneBoxStatus>({ sending: false, lastAttemptAt: null, lastSuccessAt: null, error: null });
const busy = ref(false);
const message = ref("");
const loaded = ref(false);
const dirty = computed(() => !saved.value || form.enabled !== saved.value.enabled || form.baseUrl !== saved.value.baseUrl);
let timer: ReturnType<typeof setInterval> | undefined;
let disposed = false;
const time = (value: string | null) => value ? new Date(value).toLocaleString() : "尚无";
async function refreshStatus() {
  try { const result = await getPhoneBoxStatus(); if (!disposed) status.value = result; }
  catch { /* A temporarily unavailable bridge must not overwrite the last delivery result. */ }
}
async function load() {
  try {
    const settings = await getPhoneBoxSettings();
    if (disposed) return;
    Object.assign(form, settings); saved.value = { ...settings }; loaded.value = true; message.value = "";
    await refreshStatus();
  } catch (error) { message.value = String(error); }
}
function repair() {
  Object.assign(form, { enabled: false, baseUrl: "http://127.0.0.1:8765" });
  saved.value = null; loaded.value = true;
  message.value = "已填入关闭状态的默认值，请检查地址并保存以修复配置。";
}
async function save() {
  busy.value = true; message.value = "";
  try {
    const settings = await savePhoneBoxSettings({ ...form });
    Object.assign(form, settings); saved.value = { ...settings };
    message.value = settings.enabled ? "已保存，正在后台推送当前缓存。" : "已停用，PhoneBox 中旧数据最多 180 秒后消失。";
    await refreshStatus();
  } catch (error) { message.value = String(error); }
  finally { busy.value = false; }
}
async function push() {
  if (dirty.value || !saved.value?.enabled) return;
  busy.value = true;
  try { await pushPhoneBoxNow(); message.value = "已请求推送当前缓存，不会额外刷新平台额度。"; await refreshStatus(); }
  catch (error) { message.value = String(error); }
  finally { busy.value = false; }
}
onMounted(async () => { await load(); if (!disposed) timer = setInterval(refreshStatus, 2000); });
onUnmounted(() => { disposed = true; if (timer) clearInterval(timer); });
</script>

<template>
  <div class="setting-block phonebox-settings">
    <div class="setting-block-title">PhoneBox 手机额度面板</div>
    <p class="setting-hint">把启用实例的额度推送到同机 PhoneBox，无需修改 PhoneBox。显示与托盘一致的账号名称（名称可能含邮箱）；实例使用脱敏别名，不发送平台凭据。</p>
    <label class="check-row">
      <input v-model="form.enabled" type="checkbox" :disabled="busy || !loaded" />
      <span><b>启用额度推送</b><small>额度变化后推送，每 60 秒重发缓存；不增加平台采集请求</small></span>
    </label>
    <label class="phonebox-address">
      <span>PhoneBox Agent 地址</span>
      <input v-model="form.baseUrl" type="url" :disabled="busy || !loaded" placeholder="http://127.0.0.1:8765" spellcheck="false" />
    </label>
    <p class="setting-hint">仅接受 127.0.0.1 或 [::1] 的 HTTP 根地址；手机通过原有 USB / Wi-Fi 连接查看。修改后保存生效。</p>
    <div class="phonebox-actions">
      <button type="button" :disabled="busy || !loaded" @click="save">{{ busy ? '处理中…' : '保存 PhoneBox 设置' }}</button>
      <button type="button" :disabled="busy || !loaded || dirty || !saved?.enabled || status.sending" @click="push">立即推送</button>
      <button v-if="!loaded" type="button" @click="load">重新加载</button>
      <button v-if="!loaded" type="button" @click="repair">使用默认值修复</button>
    </div>
    <p v-if="dirty && loaded" class="setting-hint">有未保存修改，请先保存再立即推送。</p>
    <dl class="phonebox-status">
      <div><dt>投递状态</dt><dd>{{ status.sending ? '推送中' : !saved?.enabled ? '已停用' : status.error ? '投递失败' : status.lastSuccessAt ? '已投递' : '尚未投递' }}</dd></div>
      <div><dt>最近尝试</dt><dd>{{ time(status.lastAttemptAt) }}</dd></div>
      <div><dt>最近成功</dt><dd>{{ time(status.lastSuccessAt) }}</dd></div>
    </dl>
    <p v-if="status.error" class="phonebox-error" role="status">{{ status.error }}</p>
    <p v-if="message" class="setting-hint" role="status">{{ message }}</p>
    <p class="setting-hint">在 PhoneBox 的 Pages 导入 tokenball-page.json 并保存。关闭推送后旧数据最多保留 180 秒，隐藏悬浮窗不会停用推送。</p>
  </div>
</template>

<style scoped>
.phonebox-address { display: grid; gap: 8px; margin: 16px 0 8px; font-size: 13px; }
.phonebox-address input { width: 100%; box-sizing: border-box; }
.phonebox-actions { display: flex; flex-wrap: wrap; gap: 8px; margin: 14px 0; }
.phonebox-status { display: grid; gap: 8px; margin: 14px 0; font-size: 12px; }
.phonebox-status div { display: flex; justify-content: space-between; gap: 16px; }
.phonebox-status dt { opacity: .6; }
.phonebox-status dd { margin: 0; font-variant-numeric: tabular-nums; }
.phonebox-error { color: #f4b67b; font-size: 12px; }
</style>
