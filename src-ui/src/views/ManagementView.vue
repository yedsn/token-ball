<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { Blocks, CheckCircle2, Gauge, Paintbrush, Plus, RefreshCw, Save, Settings2, Trash2, Wifi } from "lucide-vue-next";
import { Power } from "lucide-vue-next";
import { useTokenBallStore } from "../store";
import { getOrbVisible, hideWindow, showWindow } from "../services/tauri";
import { onShowOverview } from "../services/tauri";
import type { ProviderConnection, ProviderType, QuotaAccount, QuotaWindow } from "../types";

type MainPage = "overview" | "orbSettings" | "instance";
type SettingsSection = "appearance" | "plugins";
type BalanceRankingRow = { account: QuotaAccount; window: QuotaWindow | null };
type BalancePeriod = "fiveHour" | "weekly" | "monthly";

const store = useTokenBallStore();
const page = ref<MainPage>("overview");
const settingsSection = ref<SettingsSection>("appearance");
const orbVisible = ref(true);
const saving = ref(false);
const testing = ref(false);
const notice = ref("");
const iconError = ref("");
const appIconInputRef = ref<HTMLInputElement | null>(null);
const customForm = reactive({ label: "", value: "" });
const pluginForm = reactive({ id: "", name: "", version: "1.0.0", category: "provider", capability: "", permissions: "" });
const form = reactive({
  id: "",
  providerType: "cliProxyApi" as ProviderType,
  displayName: "本机 CLIProxyAPI",
  baseUrl: "http://127.0.0.1:3000",
  managementKey: "",
  volcengineAccessKeyId: "",
  volcengineSecretAccessKey: "",
  volcengineRegion: "cn-beijing",
  volcengineService: "ark",
  volcengineChannel: "official",
  volcengineSyncAgentPlan: true,
  volcengineSyncCodingPlan: true,
  volcengineCodingProjectName: "default",
  volcengineCodingSeatId: "",
  volcengineCodingWebBaseUrl: "https://console.volcengine.com/api/top",
  volcengineCodingWebCookie: ""
});

const providerGroups = computed(() => [
  {
    id: "pro-capi",
    title: "CLIProxyAPI",
    providerType: "cliProxyApi" as ProviderType,
    connections: store.connections.filter((connection) => connection.providerType === "cliProxyApi")
  },
  {
    id: "volcengine",
    title: "火山引擎",
    providerType: "volcengine" as ProviderType,
    connections: store.connections.filter((connection) => connection.providerType === "volcengine")
  }
]);

const currentConnection = computed(() => store.connections.find((connection) => connection.id === form.id) ?? null);
const previewQuotaAccounts = computed(() => (store.displaySettings.showAccountsInTooltip ? store.summary.accounts : []));
const enabledCustomItems = computed(() => store.displaySettings.customItems.filter((item) => item.enabled));
const balanceExpiryRanking = computed<BalanceRankingRow[]>(() => {
  const rows: BalanceRankingRow[] = store.summary.accounts.map((account) => ({ account, window: primaryExpiryWindow(account) }));
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
const groupedAccounts = computed(() => {
  return providerGroups.value.map((provider) => ({
    id: provider.id,
    title: provider.title,
    groups: provider.connections.map((connection) => ({
      connection,
      accounts: store.summary.accounts.filter((account) => account.connectionId === connection.id)
    }))
  })).filter((provider) => provider.groups.length > 0);
});
const currentConnectionAccounts = computed(() => store.summary.accounts.filter((account) => account.connectionId === form.id));

const pageTitle = computed(() => {
  if (page.value === "orbSettings") return "流量球设置";
  if (page.value === "instance") return form.id ? "实例配置" : `新增 ${currentProviderName.value} 实例`;
  return "总览";
});

const pageDescription = computed(() => {
  if (page.value === "orbSettings") return settingsSection.value === "plugins" ? "管理内置扩展和后续可安装插件的启停状态" : "配置流量球、悬浮面板和托盘悬停信息显示哪些内容";
  if (page.value === "instance") return "维护当前 provider 实例的连接信息";
  return "各 Provider 的模型额度与账号状态";
});

const currentProviderName = computed(() => providerLabel(form.providerType));
const savedAccessKeyLabel = computed(() => currentConnection.value?.providerConfigHint?.hasAccessKeyId ? "已保存，留空沿用" : "未保存");
const savedSecretKeyLabel = computed(() => currentConnection.value?.providerConfigHint?.hasSecretAccessKey ? "已保存，留空沿用" : "未保存");
const savedWebCookieLabel = computed(() => currentConnection.value?.providerConfigHint?.hasCodingWebCookie ? "已保存，留空沿用" : "未保存");

const totalRemainingLabel = computed(() => {
  const percent = store.totalRemainingPercent;
  if (typeof percent !== "number") return "未知";
  return `${Math.round(percent)}%`;
});

const equivalentLabel = computed(() => `${store.totalEquivalentAccounts.toFixed(2)} 账号`);
const appIconOptions = computed(() => [
  { id: "meter" as const, label: "V3 余量仪表", description: "用于窗口、任务栏和安装包的默认程序图标" },
  { id: "orb" as const, label: "额度", description: "使用与桌面悬浮窗一致的绿色额度图标" },
  { id: "custom" as const, label: "自定义", description: "上传 PNG、JPG 或 WebP 图标", src: store.displaySettings.customAppIconDataUrl }
]);

watch(
  () => store.connections,
  (connections) => {
    const connection = currentConnection.value ?? connections[0];
    if (!connection || form.id) return;
    selectConnection(connection);
  },
  { immediate: true }
);

watch(
  () => page.value === "orbSettings",
  async (active) => {
    if (active) orbVisible.value = await getOrbVisible();
  },
  { immediate: true }
);

onMounted(async () => {
  await onShowOverview(() => openPage("overview"));
});

function openPage(target: MainPage) {
  page.value = target;
  if (target === "orbSettings" && !settingsSection.value) settingsSection.value = "appearance";
  notice.value = "";
}

function openSettingsSection(section: SettingsSection) {
  settingsSection.value = section;
  page.value = "orbSettings";
  notice.value = "";
}

function selectConnection(connection: ProviderConnection) {
  form.id = connection.id;
  form.providerType = connection.providerType;
  form.displayName = connection.displayName;
  form.baseUrl = connection.baseUrl;
  form.managementKey = "";
  form.volcengineAccessKeyId = "";
  form.volcengineSecretAccessKey = "";
  form.volcengineRegion = connection.providerConfigHint?.region || "cn-beijing";
  form.volcengineService = connection.providerConfigHint?.service || "ark";
  form.volcengineChannel = connection.providerConfigHint?.channel || "official";
  form.volcengineSyncAgentPlan = connection.providerConfigHint?.syncAgentPlan ?? true;
  form.volcengineSyncCodingPlan = connection.providerConfigHint?.syncCodingPlan ?? true;
  form.volcengineCodingProjectName = connection.providerConfigHint?.codingProjectName || "default";
  form.volcengineCodingSeatId = connection.providerConfigHint?.codingSeatId || "";
  form.volcengineCodingWebBaseUrl = connection.providerConfigHint?.codingWebBaseUrl || "https://console.volcengine.com/api/top";
  form.volcengineCodingWebCookie = "";
  notice.value = "";
  page.value = "instance";
}

function newConnection(providerType: ProviderType = "cliProxyApi") {
  form.id = "";
  form.providerType = providerType;
  form.displayName = providerType === "volcengine" ? `火山引擎 ${store.connections.length + 1}` : `CLIProxyAPI ${store.connections.length + 1}`;
  form.baseUrl = providerType === "volcengine" ? "https://open.volcengineapi.com" : "http://127.0.0.1:8317";
  form.managementKey = "";
  form.volcengineAccessKeyId = "";
  form.volcengineSecretAccessKey = "";
  form.volcengineRegion = "cn-beijing";
  form.volcengineService = "ark";
  form.volcengineChannel = "official";
  form.volcengineSyncAgentPlan = providerType === "volcengine";
  form.volcengineSyncCodingPlan = providerType === "volcengine";
  form.volcengineCodingProjectName = "default";
  form.volcengineCodingSeatId = "";
  form.volcengineCodingWebBaseUrl = "https://console.volcengine.com/api/top";
  form.volcengineCodingWebCookie = "";
  notice.value = "";
  page.value = "instance";
}

function connectionPayload() {
  if (form.providerType !== "volcengine") {
    return { id: form.id || undefined, providerType: form.providerType, displayName: form.displayName, baseUrl: form.baseUrl, managementKey: form.managementKey };
  }
  const current = currentConnection.value;
  const accessKeyId = form.volcengineAccessKeyId.trim();
  const secretAccessKey = form.volcengineSecretAccessKey.trim();
  return {
    id: form.id || undefined,
    providerType: form.providerType,
    displayName: form.displayName,
    baseUrl: form.baseUrl,
    managementKey: JSON.stringify({
      accessKeyId,
      secretAccessKey,
      region: form.volcengineRegion.trim() || current?.providerConfigHint?.region || "cn-beijing",
      service: form.volcengineService.trim() || current?.providerConfigHint?.service || "ark",
      channel: form.volcengineChannel,
      syncAgentPlan: form.volcengineSyncAgentPlan,
      syncCodingPlan: form.volcengineSyncCodingPlan,
      codingProjectName: form.volcengineCodingProjectName.trim(),
      codingSeatId: form.volcengineCodingSeatId.trim(),
      codingWebBaseUrl: form.volcengineCodingWebBaseUrl.trim() || "https://console.volcengine.com/api/top",
      codingWebCookie: form.volcengineCodingWebCookie.trim()
    })
  };
}

async function save() {
  saving.value = true;
  notice.value = "";
  try {
    const connection = await store.saveProviderConnection(connectionPayload());
    form.id = connection.id;
    form.displayName = connection.displayName;
    form.baseUrl = connection.baseUrl;
    form.managementKey = "";
    form.volcengineAccessKeyId = "";
    form.volcengineSecretAccessKey = "";
    form.volcengineCodingWebCookie = "";
    form.volcengineChannel = connection.providerConfigHint?.channel || form.volcengineChannel;
    form.volcengineSyncAgentPlan = connection.providerConfigHint?.syncAgentPlan ?? form.volcengineSyncAgentPlan;
    form.volcengineSyncCodingPlan = connection.providerConfigHint?.syncCodingPlan ?? form.volcengineSyncCodingPlan;
    form.volcengineCodingProjectName = connection.providerConfigHint?.codingProjectName || form.volcengineCodingProjectName;
    form.volcengineCodingSeatId = connection.providerConfigHint?.codingSeatId || form.volcengineCodingSeatId;
    form.volcengineCodingWebBaseUrl = connection.providerConfigHint?.codingWebBaseUrl || form.volcengineCodingWebBaseUrl;
    notice.value = "连接已保存";
    await store.refresh();
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
    const hasVolcengineSecret = form.providerType === "volcengine" && (form.volcengineAccessKeyId.trim() || form.volcengineSecretAccessKey.trim() || form.volcengineCodingWebCookie.trim());
    const connection = form.managementKey.trim() || hasVolcengineSecret ? await save() : currentConnection.value;
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
  form.volcengineAccessKeyId = "";
  form.volcengineSecretAccessKey = "";
  form.volcengineCodingWebCookie = "";
  form.volcengineCodingProjectName = "";
  form.volcengineCodingSeatId = "";
  notice.value = "连接已删除";
  page.value = "overview";
}

function quotaLabel(account: QuotaAccount) {
  const window = account.windows.find((item) => item.id === account.criticalWindowId);
  if (typeof window?.remainingPercent === "number") return `${Math.round(window.remainingPercent)}%`;
  const usageWindow = account.windows.find((item) => typeof item.used === "number");
  if (usageWindow) return formatWindowUsed(usageWindow);
  return "未知";
}

function accountRemainingPercent(account: QuotaAccount) {
  const window = account.windows.find((item) => item.id === account.criticalWindowId);
  return typeof window?.remainingPercent === "number" ? window.remainingPercent : null;
}

function accountBalanceClass(account: QuotaAccount) {
  const percent = accountRemainingPercent(account);
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
  return resetLabel(row.window?.resetAt ?? row.account.nextResetAt);
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

function providerLabel(providerType: ProviderType) {
  if (providerType === "volcengine") return "火山引擎";
  return "CLIProxyAPI";
}

function providerHelp(providerType: ProviderType) {
  if (providerType === "volcengine") return "官方渠道使用 Access Key 查询 OpenAPI；页面渠道使用控制台 Cookie 查询 Coding Plan 用量。可以按计划类型拆成多个实例。";
  return "这里需要 remote-management.secret-key 或 MANAGEMENT_PASSWORD，不是普通 API Key。";
}

function canTestConnection() {
  if (currentConnection.value) return true;
  if (form.providerType === "volcengine" && form.volcengineChannel === "web") return Boolean(form.volcengineCodingWebCookie.trim());
  if (form.providerType === "volcengine") return Boolean(form.volcengineAccessKeyId.trim() && form.volcengineSecretAccessKey.trim());
  return Boolean(form.managementKey.trim());
}

function expiryClass(value?: string | null) {
  if (!value) return "";
  const days = (new Date(value).getTime() - Date.now()) / 86400000;
  if (days <= 3) return "critical";
  if (days <= 7) return "warning";
  return "";
}

function toggleTotal(event: Event) {
  store.toggleTotalRemaining((event.target as HTMLInputElement).checked);
}

function toggleDisplayFlag(key: "showAvailableAccounts" | "showConnectionStatus" | "showAccountsInTooltip" | "showOrbRefreshButton" | "orbAnimationEnabled", event: Event) {
  store.updateDisplayFlag(key, (event.target as HTMLInputElement).checked);
}

async function setProgramIcon(style: "meter" | "orb" | "custom") {
  iconError.value = "";
  if (style === "custom" && !store.displaySettings.customAppIconDataUrl) {
    chooseCustomIcon();
    return;
  }
  try {
    await store.updateAppIconStyle(style);
  } catch (error) {
    iconError.value = String(error);
  }
}

function chooseCustomIcon() {
  appIconInputRef.value?.click();
}

function handleProgramIconChange(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  const allowedTypes = ["image/png", "image/jpeg", "image/webp"];
  if (!allowedTypes.includes(file.type)) {
    iconError.value = "请选择 PNG、JPG 或 WebP 图标文件。";
    input.value = "";
    return;
  }
  if (file.size > 1024 * 1024) {
    iconError.value = "图标文件不能超过 1MB。";
    input.value = "";
    return;
  }
  const reader = new FileReader();
  reader.onload = async () => {
    if (typeof reader.result !== "string") return;
    try {
      iconError.value = "";
      await store.updateAppIconStyle("custom", reader.result);
    } catch (error) {
      iconError.value = String(error);
    }
  };
  reader.onerror = () => {
    iconError.value = "图标文件读取失败。";
  };
  reader.readAsDataURL(file);
  input.value = "";
}

async function clearCustomProgramIcon() {
  iconError.value = "";
  try {
    await store.updateAppIconStyle("meter", "");
  } catch (error) {
    iconError.value = String(error);
  }
}

async function addCustomItem() {
  if (!customForm.label.trim() && !customForm.value.trim()) return;
  await store.addCustomDisplayItem(customForm.label, customForm.value);
  customForm.label = "";
  customForm.value = "";
}

async function addPlugin() {
  if (!pluginForm.id.trim() || !pluginForm.name.trim()) {
    notice.value = "插件 ID 和名称不能为空";
    return;
  }
  await store.addLocalPlugin({
    id: pluginForm.id.trim(),
    name: pluginForm.name.trim(),
    version: pluginForm.version.trim() || "1.0.0",
    category: pluginForm.category.trim() || "provider",
    capability: pluginForm.capability.trim() || "本地安装插件",
    permissions: pluginForm.permissions.split(/[，,\n]/).map((item) => item.trim()).filter(Boolean),
    configurable: true
  });
  pluginForm.id = "";
  pluginForm.name = "";
  pluginForm.version = "1.0.0";
  pluginForm.category = "provider";
  pluginForm.capability = "";
  pluginForm.permissions = "";
  notice.value = "插件已添加";
}

async function deletePlugin(id: string, builtIn: boolean) {
  if (builtIn) {
    notice.value = "内置插件不能删除，只能停用";
    return;
  }
  await store.deleteLocalPlugin(id);
  notice.value = "插件已删除";
}

async function toggleOrbVisible(event: Event) {
  const visible = (event.target as HTMLInputElement).checked;
  orbVisible.value = visible;
  if (visible) await showWindow("orb");
  else await hideWindow("orb");
}

function resetLabel(value?: string | null) {
  if (!value) return "--";
  const date = new Date(value);
  if (date.getTime() <= 0) return "--";
  return date.toLocaleString("zh-CN", { month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" });
}

function activityLabel(account: QuotaAccount) {
  if (account.externalId.startsWith("volcengine-coding-plan") && account.windows.length === 0) return "未返回 Coding Plan 用量明细。";
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
      <div class="brand-mark" aria-label="TokenBall">
        <img v-if="store.displaySettings.appIconStyle === 'custom' && store.displaySettings.customAppIconDataUrl" :src="store.displaySettings.customAppIconDataUrl" alt="" />
        <svg v-else-if="store.displaySettings.appIconStyle === 'orb'" viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
          <rect width="512" height="512" rx="96" fill="#0F172A"/>
          <circle cx="256" cy="256" r="150" fill="none" stroke="#F8FAFC" stroke-width="34" stroke-linecap="round"/>
          <path d="M256 104 A152 152 0 0 1 408 256" fill="none" stroke="#38BDF8" stroke-width="42" stroke-linecap="round"/>
          <circle cx="256" cy="256" r="42" fill="none" stroke="#F8FAFC" stroke-width="28"/>
          <line x1="256" y1="180" x2="256" y2="144" stroke="#38BDF8" stroke-width="24" stroke-linecap="round"/>
        </svg>
        <svg v-else viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
          <rect width="512" height="512" rx="96" fill="#1E1B4B"/>
          <path d="M132 286 A124 124 0 0 1 380 286" fill="none" stroke="#E0E7FF" stroke-width="32" stroke-linecap="round" stroke-linejoin="round"/>
          <line x1="256" y1="286" x2="326" y2="214" stroke="#A78BFA" stroke-width="30" stroke-linecap="round" stroke-linejoin="round"/>
          <circle cx="256" cy="286" r="26" fill="#A78BFA"/>
          <line x1="156" y1="354" x2="356" y2="354" stroke="#E0E7FF" stroke-width="28" stroke-linecap="round" stroke-linejoin="round"/>
          <circle cx="182" cy="354" r="12" fill="#A78BFA"/>
          <circle cx="330" cy="354" r="12" fill="#A78BFA"/>
        </svg>
      </div>
      <button class="nav" :class="{ active: page === 'overview' }" @click="openPage('overview')"><Gauge :size="16" />总览</button>
      <button class="nav" :class="{ active: page === 'orbSettings' }" @click="openPage('orbSettings')"><Settings2 :size="16" />设置</button>

      <div v-if="store.connectionError" class="side-error">{{ store.connectionError }}</div>

      <div v-for="group in providerGroups" :key="group.id" class="provider-group">
        <div class="provider-title"><Wifi :size="14" />{{ group.title }}</div>
        <div class="instance-list">
          <button type="button" class="instance-add" @click="newConnection(group.providerType)"><Plus :size="14" />新增实例</button>
          <div
            v-for="connection in group.connections"
            :key="connection.id"
            class="instance-item"
            :class="{ active: page === 'instance' && connection.id === form.id, enabled: connection.enabled }"
            @click="selectConnection(connection)"
          >
            <strong>{{ connection.displayName }}</strong>
            <span>{{ connection.status }} · {{ connection.baseUrl }}</span>
            <button
              type="button"
              class="instance-toggle"
              :class="{ enabled: connection.enabled }"
              :title="connection.enabled ? '已启用，点击停用' : '已停用，点击启用'"
              @click.stop="store.toggleConnectionEnabled(connection.id, !connection.enabled)"
            ><Power :size="12" /></button>
          </div>
        </div>
      </div>
    </aside>

    <section class="workspace">
      <header class="topbar">
        <div>
          <h1>{{ pageTitle }}</h1>
          <p>{{ pageDescription }}</p>
        </div>
        <button class="primary" @click="store.refresh" :disabled="store.refreshing || !store.hasConnection">
          <RefreshCw :size="16" :class="{ spin: store.refreshing }" />立即刷新
        </button>
      </header>

      <section v-if="page === 'overview'">
        <section class="metrics">
          <article>
            <span>额度余量</span>
            <strong>{{ totalRemainingLabel }}</strong>
          </article>
          <article>
            <span>等效剩余</span>
            <strong>{{ equivalentLabel }}</strong>
          </article>
          <article>
            <span>连接状态</span>
            <strong>{{ store.summary.status }}</strong>
          </article>
        </section>

        <div v-if="store.connectionError || store.error" class="error-banner">
          <strong>数据加载异常</strong>
          <span v-if="store.connectionError">{{ store.connectionError }}</span>
          <span v-if="store.error">{{ store.error }}</span>
        </div>

        <section class="panel balance-ranking-panel">
          <header class="balance-ranking-head">
            <div>
              <h2>全账号额度到期表</h2>
              <p>每个账号一行，有月限额看月限额，否则按最高额度周期排序</p>
            </div>
            <span>{{ balanceExpiryRanking.length }} 个账号</span>
          </header>
          <div v-if="!store.hasConnection" class="empty-state">保存实例后展示全账号额度到期表。</div>
          <div v-else-if="balanceExpiryRanking.length === 0" class="empty-state">暂无账号额度数据，点击立即刷新同步。</div>
          <table v-else class="balance-table">
            <thead>
              <tr>
                <th>到期时间</th>
                <th>账号</th>
                <th>优先额度</th>
                <th>剩余</th>
                <th>5h</th>
                <th>周</th>
                <th>月</th>
                <th>用量</th>
                <th>实例</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="row in balanceExpiryRanking" :key="`${row.account.id}-${row.window?.id ?? 'account'}`" :class="balanceRowClass(row)">
                <td><span class="balance-reset" :class="balanceResetClass(row)">{{ balanceResetLabel(row) }}</span></td>
                <td><strong>{{ row.account.displayName }}</strong><span>{{ row.account.planName }} · {{ row.account.status }}</span></td>
                <td>{{ balanceWindowName(row) }}</td>
                <td><b>{{ balanceRemainingLabel(row) }}</b></td>
                <td><b class="balance-quota" :class="accountPeriodRemainingClass(row.account, 'fiveHour')">{{ accountPeriodRemainingLabel(row.account, 'fiveHour') }}</b></td>
                <td><b class="balance-quota" :class="accountPeriodRemainingClass(row.account, 'weekly')">{{ accountPeriodRemainingLabel(row.account, 'weekly') }}</b></td>
                <td><b class="balance-quota" :class="accountPeriodRemainingClass(row.account, 'monthly')">{{ accountPeriodRemainingLabel(row.account, 'monthly') }}</b></td>
                <td>{{ balanceUsageLabel(row) }}</td>
                <td>{{ accountConnectionLabel(row.account) }}</td>
              </tr>
            </tbody>
          </table>
        </section>

        <section class="content-grid overview-grid">
          <section class="panel account-panel quota-board">
            <h2>模型额度</h2>
            <p v-if="store.hasConnection && store.summary.lowestRemainingPercent === null" class="muted quota-note">当前 CLIProxyAPI 管理接口未返回真实余量百分比，下面展示账号可用性和请求活动。</p>
            <div v-if="!store.hasConnection" class="empty-state">保存 CLIProxyAPI 连接后开始同步账号。</div>
            <div v-else-if="store.summary.accounts.length === 0" class="empty-state">已连接，点击立即刷新同步账号；如果仍为空，请查看测试返回内容。</div>
            <div v-else class="connection-groups">
              <section v-for="provider in groupedAccounts" :key="provider.id" class="provider-account-group">
                <h3>{{ provider.title }}</h3>
                <section v-for="group in provider.groups" :key="group.connection.id" class="connection-group">
                  <header>
                    <strong>{{ group.connection.displayName }}</strong>
                    <span>{{ group.connection.baseUrl }}</span>
                  </header>
                  <article v-for="account in group.accounts" :key="account.id" class="quota-card">
                    <div class="quota-card-head">
                      <div>
                        <strong>{{ account.displayName }}</strong>
                        <span>{{ account.planName }} · {{ account.status }}<template v-if="account.subscriptionUntil"> · <em class="expiry" :class="expiryClass(account.subscriptionUntil)">到期 {{ dateLabel(account.subscriptionUntil) }}</em></template></span>
                      </div>
                      <b>{{ quotaLabel(account) }}</b>
                    </div>
                    <div v-if="account.windows.length" class="quota-bars">
                      <div v-for="window in account.windows" :key="window.id" class="quota-bar-row" :class="windowClass(window)">
                        <span>{{ window.name }}</span>
                        <div class="quota-track"><i :style="{ width: `${windowPercent(window) ?? 0}%` }"></i></div>
                        <strong>{{ windowUsageLabel(window) }}</strong>
                        <b>{{ windowPercentLabel(window) }}</b>
                        <em>{{ resetLabel(window.resetAt) }}</em>
                      </div>
                    </div>
                    <span v-else class="muted">{{ activityLabel(account) }}</span>
                  </article>
                  <div v-if="group.accounts.length === 0" class="empty-state">该实例暂无账号数据。</div>
                </section>
              </section>
            </div>
          </section>

          <section class="panel selected-preview">
            <h2>流量球显示预览</h2>
            <div v-if="store.displaySettings.showTotalRemaining" class="preview-line">
              <span>总额度</span><strong>{{ totalRemainingLabel }}</strong><small>{{ equivalentLabel }}</small>
            </div>
            <div v-if="store.displaySettings.showAvailableAccounts" class="preview-line">
              <span>可用账号</span><strong>{{ store.summary.availableAccounts }} / {{ store.summary.totalAccounts }}</strong><small>账号状态</small>
            </div>
            <div v-if="store.displaySettings.showConnectionStatus" class="preview-line">
              <span>连接状态</span><strong>{{ store.summary.status }}</strong><small>CLIProxyAPI</small>
            </div>
            <div v-if="store.displaySettings.showAccountsInTooltip" class="display-pick-title">已选择账号信息</div>
            <div v-for="account in previewQuotaAccounts" :key="account.id" class="preview-line">
              <span>{{ account.displayName }}</span><strong>{{ quotaLabel(account) }}</strong><small>{{ account.planName }}</small>
            </div>
            <div v-for="item in enabledCustomItems" :key="item.id" class="preview-line">
              <span>{{ item.label || '自定义' }}</span><strong>{{ item.value || '--' }}</strong><small>手动内容</small>
            </div>
          </section>
        </section>
      </section>

      <section v-else-if="page === 'orbSettings'" class="settings-page-shell">
        <aside class="settings-subnav">
          <button type="button" class="settings-subnav-item" :class="{ active: settingsSection === 'appearance' }" @click="openSettingsSection('appearance')">
            <Paintbrush :size="16" />
            <span><b>外观</b><small>额度、托盘图标和显示内容</small></span>
          </button>
          <button type="button" class="settings-subnav-item" :class="{ active: settingsSection === 'plugins' }" @click="openSettingsSection('plugins')">
            <Blocks :size="16" />
            <span><b>插件</b><small>安装、启停和删除扩展</small></span>
          </button>
        </aside>

        <section v-if="settingsSection === 'appearance'" class="panel display-panel">
          <h2>外观</h2>

          <div class="setting-block">
            <div class="setting-block-title">额度</div>
            <label class="check-row">
              <input type="checkbox" :checked="orbVisible" @change="toggleOrbVisible" />
              <span><b>显示额度悬浮窗</b><small>控制桌面额度悬浮窗是否显示，设置会在重启后保持</small></span>
            </label>
            <label class="check-row">
              <input type="checkbox" :checked="store.displaySettings.orbAnimationEnabled" @change="toggleDisplayFlag('orbAnimationEnabled', $event)" />
              <span><b>额度动画</b><small>控制能量环和液面动画效果</small></span>
            </label>
            <label class="check-row">
              <input type="checkbox" :checked="store.displaySettings.showOrbRefreshButton" @change="toggleDisplayFlag('showOrbRefreshButton', $event)" />
              <span><b>右下角刷新图标</b><small>控制额度悬浮窗右下角刷新按钮是否显示</small></span>
            </label>
          </div>

          <div class="setting-block">
            <div class="setting-block-title">程序图标</div>
            <div class="app-icon-options">
              <button
                v-for="option in appIconOptions"
                :key="option.id"
                class="app-icon-option"
                :class="{ active: store.displaySettings.appIconStyle === option.id }"
                type="button"
                @click="setProgramIcon(option.id)"
              >
                <span class="app-icon-preview">
                  <svg v-if="option.id === 'meter'" viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
                    <rect width="512" height="512" rx="96" fill="#1E1B4B"/>
                    <path d="M132 286 A124 124 0 0 1 380 286" fill="none" stroke="#E0E7FF" stroke-width="32" stroke-linecap="round" stroke-linejoin="round"/>
                    <line x1="256" y1="286" x2="326" y2="214" stroke="#A78BFA" stroke-width="30" stroke-linecap="round" stroke-linejoin="round"/>
                    <circle cx="256" cy="286" r="26" fill="#A78BFA"/>
                    <line x1="156" y1="354" x2="356" y2="354" stroke="#E0E7FF" stroke-width="28" stroke-linecap="round" stroke-linejoin="round"/>
                    <circle cx="182" cy="354" r="12" fill="#A78BFA"/>
                    <circle cx="330" cy="354" r="12" fill="#A78BFA"/>
                  </svg>
                  <svg v-else-if="option.id === 'orb'" viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
                    <rect width="512" height="512" rx="96" fill="#0F172A"/>
                    <circle cx="256" cy="256" r="150" fill="none" stroke="#F8FAFC" stroke-width="34" stroke-linecap="round"/>
                    <path d="M256 104 A152 152 0 0 1 408 256" fill="none" stroke="#38BDF8" stroke-width="42" stroke-linecap="round"/>
                    <circle cx="256" cy="256" r="42" fill="none" stroke="#F8FAFC" stroke-width="28"/>
                    <line x1="256" y1="180" x2="256" y2="144" stroke="#38BDF8" stroke-width="24" stroke-linecap="round"/>
                  </svg>
                  <img v-else-if="store.displaySettings.customAppIconDataUrl" :src="store.displaySettings.customAppIconDataUrl" alt="" />
                  <svg v-else viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg" aria-hidden="true">
                    <rect width="512" height="512" rx="96" fill="#1E1B4B"/>
                    <path d="M132 286 A124 124 0 0 1 380 286" fill="none" stroke="#E0E7FF" stroke-width="32" stroke-linecap="round" stroke-linejoin="round"/>
                    <line x1="256" y1="286" x2="326" y2="214" stroke="#A78BFA" stroke-width="30" stroke-linecap="round" stroke-linejoin="round"/>
                    <circle cx="256" cy="286" r="26" fill="#A78BFA"/>
                    <line x1="156" y1="354" x2="356" y2="354" stroke="#E0E7FF" stroke-width="28" stroke-linecap="round" stroke-linejoin="round"/>
                    <circle cx="182" cy="354" r="12" fill="#A78BFA"/>
                    <circle cx="330" cy="354" r="12" fill="#A78BFA"/>
                  </svg>
                </span>
                <span class="app-icon-copy">
                  <strong>{{ option.label }}</strong>
                  <small>{{ option.description }}</small>
                </span>
              </button>
            </div>
            <div class="custom-icon-actions">
              <button class="primary" type="button" @click="chooseCustomIcon">上传自定义图标</button>
              <button v-if="store.displaySettings.customAppIconDataUrl" type="button" @click="clearCustomProgramIcon">清除自定义图标</button>
              <input ref="appIconInputRef" class="hidden-file-input" type="file" accept="image/png,image/jpeg,image/webp" @change="handleProgramIconChange" />
            </div>
            <p class="setting-hint">程序图标会立即应用到主窗口和任务栏；如果你要替换安装包图标，还需要重新构建应用。</p>
            <p v-if="iconError" class="setting-error">{{ iconError }}</p>
          </div>

          <div class="setting-block">
            <div class="setting-block-title">托盘图标</div>
            <p class="setting-hint">托盘图标保留额度样式，会根据当前额度状态显示绿色、黄色、红色或灰色。</p>
            <div class="segmented-control">
              <button :class="{ active: store.displaySettings.trayIconStyle === 'orb' }" @click="store.updateTrayIconStyle('orb')">能量球</button>
              <button :class="{ active: store.displaySettings.trayIconStyle === 'minimal' }" @click="store.updateTrayIconStyle('minimal')">简洁色块</button>
            </div>
          </div>

          <div class="setting-block">
            <div class="setting-block-title">显示内容</div>
            <label class="check-row">
              <input type="checkbox" :checked="store.displaySettings.showTotalRemaining" @change="toggleTotal" />
              <span><b>总额度剩余</b><small>聚合所有账号当前限制窗口的剩余比例</small></span>
            </label>
            <label class="check-row">
              <input type="checkbox" :checked="store.displaySettings.showAvailableAccounts" @change="toggleDisplayFlag('showAvailableAccounts', $event)" />
              <span><b>可用账号数量</b><small>显示可用账号 / 总账号</small></span>
            </label>
            <label class="check-row">
              <input type="checkbox" :checked="store.displaySettings.showConnectionStatus" @change="toggleDisplayFlag('showConnectionStatus', $event)" />
              <span><b>连接状态</b><small>显示 healthy / degraded 等连接状态</small></span>
            </label>
            <label class="check-row">
              <input type="checkbox" :checked="store.displaySettings.showAccountsInTooltip" @change="toggleDisplayFlag('showAccountsInTooltip', $event)" />
              <span><b>全部账号剩余额度</b><small>在流量球和托盘悬停信息中显示全部账号的剩余额度</small></span>
            </label>
          </div>

          <div class="setting-block">
            <div class="setting-block-title">手动显示内容</div>
            <form class="custom-item-form" @submit.prevent="addCustomItem">
              <input v-model="customForm.label" placeholder="名称，例如 本机实例" />
              <input v-model="customForm.value" placeholder="内容，例如 http://10.0.7.31:8317" />
              <button class="primary" type="submit"><Plus :size="14" />添加</button>
            </form>
            <div v-for="item in store.displaySettings.customItems" :key="item.id" class="custom-item-row">
              <input :value="item.label" @change="store.updateCustomDisplayItem(item.id, { label: ($event.target as HTMLInputElement).value })" />
              <input :value="item.value" @change="store.updateCustomDisplayItem(item.id, { value: ($event.target as HTMLInputElement).value })" />
              <label class="inline-check"><input type="checkbox" :checked="item.enabled" @change="store.updateCustomDisplayItem(item.id, { enabled: ($event.target as HTMLInputElement).checked })" />显示</label>
              <button type="button" @click="store.removeCustomDisplayItem(item.id)"><Trash2 :size="14" /></button>
            </div>
            <div v-if="store.displaySettings.customItems.length === 0" class="empty-state">可以添加某个实例的固定值、备注、地址或你希望在流量球信息里看到的内容。</div>
          </div>
        </section>

        <section v-if="settingsSection === 'appearance'" class="panel selected-preview settings-preview">
          <h2>显示预览</h2>
          <div v-if="store.displaySettings.showTotalRemaining" class="preview-line">
            <span>总额度</span><strong>{{ totalRemainingLabel }}</strong><small>{{ equivalentLabel }}</small>
          </div>
          <div v-if="store.displaySettings.showAvailableAccounts" class="preview-line">
            <span>可用账号</span><strong>{{ store.summary.availableAccounts }} / {{ store.summary.totalAccounts }}</strong><small>账号状态</small>
          </div>
          <div v-if="store.displaySettings.showConnectionStatus" class="preview-line">
            <span>连接状态</span><strong>{{ store.summary.status }}</strong><small>CLIProxyAPI</small>
          </div>
          <div v-if="store.displaySettings.showAccountsInTooltip" class="display-pick-title">全部账号剩余额度</div>
          <div v-for="account in previewQuotaAccounts" :key="account.id" class="preview-line">
            <span>{{ account.displayName }}</span><strong>{{ quotaLabel(account) }}</strong><small>{{ account.planName }}</small>
          </div>
          <div v-for="item in enabledCustomItems" :key="item.id" class="preview-line">
            <span>{{ item.label || '自定义' }}</span><strong>{{ item.value || '--' }}</strong><small>手动内容</small>
          </div>
          <div v-if="!store.displaySettings.showTotalRemaining && !store.displaySettings.showAvailableAccounts && !store.displaySettings.showConnectionStatus && !store.displaySettings.showAccountsInTooltip && enabledCustomItems.length === 0" class="empty-state">当前没有启用任何显示内容。</div>
        </section>

        <section v-if="settingsSection === 'plugins'" class="panel plugin-panel">
          <h2>已安装插件</h2>
          <form class="plugin-add-form" @submit.prevent="addPlugin">
            <input v-model="pluginForm.id" placeholder="插件 ID，例如 volcengine-usage" />
            <input v-model="pluginForm.name" placeholder="插件名称" />
            <input v-model="pluginForm.version" placeholder="版本" />
            <input v-model="pluginForm.category" placeholder="分类，例如 provider" />
            <input v-model="pluginForm.capability" class="full" placeholder="能力说明" />
            <input v-model="pluginForm.permissions" class="full" placeholder="权限，用逗号分隔，例如 network:example.com, secret:api-key" />
            <button class="primary" type="submit"><Plus :size="14" />添加插件</button>
          </form>
          <article v-for="plugin in store.plugins" :key="plugin.id" class="plugin-card">
            <div>
              <strong>{{ plugin.name }}</strong>
              <span>{{ plugin.category }} · v{{ plugin.version }}<template v-if="plugin.builtIn"> · 内置</template></span>
              <small>{{ plugin.capability }}</small>
            </div>
            <div class="plugin-actions">
              <label class="inline-check plugin-switch"><input type="checkbox" :checked="plugin.enabled" @change="store.togglePlugin(plugin.id, ($event.target as HTMLInputElement).checked)" />启用</label>
              <button type="button" :disabled="plugin.builtIn" :title="plugin.builtIn ? '内置插件不能删除，只能停用' : '删除插件'" @click="deletePlugin(plugin.id, plugin.builtIn)"><Trash2 :size="14" /></button>
            </div>
          </article>
          <div v-if="store.plugins.length === 0" class="empty-state">暂无插件。</div>
          <p v-if="notice" class="notice">{{ notice }}</p>
        </section>

        <section v-if="settingsSection === 'plugins'" class="panel selected-preview">
          <h2>插件安装</h2>
          <div class="preview-line">
            <span>添加方式</span><strong>本地清单</strong><small>当前支持登记本地插件清单；后续可扩展为选择插件包并安装文件。</small>
          </div>
          <div v-for="plugin in store.plugins" :key="plugin.id" class="preview-line">
            <span>{{ plugin.name }}</span><strong>{{ plugin.enabled ? '已启用' : '已停用' }}</strong><small>{{ plugin.permissions.join('、') || '无额外权限' }}</small>
          </div>
        </section>
      </section>

      <section v-else class="content-grid instance-grid">
        <form class="panel" @submit.prevent="save">
          <h2>{{ form.id ? '编辑实例' : `新增${currentProviderName}实例` }}</h2>
          <label>Provider
            <select v-model="form.providerType" :disabled="Boolean(form.id)">
              <option value="cliProxyApi">CLIProxyAPI</option>
              <option value="volcengine">火山引擎</option>
            </select>
          </label>
          <label>显示名称<input v-model="form.displayName" /></label>
          <label v-if="form.providerType !== 'volcengine' || form.volcengineChannel === 'official'">{{ form.providerType === 'volcengine' ? 'OpenAPI Host' : '服务地址' }}<input v-model="form.baseUrl" :placeholder="form.providerType === 'volcengine' ? 'https://open.volcengineapi.com' : ''" /></label>
          <template v-if="form.providerType === 'volcengine'">
            <label>渠道类型
              <select v-model="form.volcengineChannel">
                <option value="official">官方渠道</option>
                <option value="web">页面渠道</option>
              </select>
            </label>
            <div class="inline-options">
              <label class="inline-check"><input type="checkbox" v-model="form.volcengineSyncAgentPlan" />Agent Plan</label>
              <label class="inline-check"><input type="checkbox" v-model="form.volcengineSyncCodingPlan" />Coding Plan</label>
            </div>
            <template v-if="form.volcengineChannel === 'official'">
              <label>Access Key ID<input v-model="form.volcengineAccessKeyId" autocomplete="off" placeholder="例如 AKLT..." /><small>{{ savedAccessKeyLabel }}</small></label>
              <label>Secret Access Key<input v-model="form.volcengineSecretAccessKey" type="password" autocomplete="off" placeholder="保存后不会回显" /><small>{{ savedSecretKeyLabel }}</small></label>
              <label>Region<input v-model="form.volcengineRegion" placeholder="cn-beijing" /></label>
              <label>Service<input v-model="form.volcengineService" placeholder="ark" /></label>
              <label>Coding ProjectName<input v-model="form.volcengineCodingProjectName" placeholder="席位查询可选，默认 default" /></label>
              <label>Coding SeatIDs<input v-model="form.volcengineCodingSeatId" placeholder="高级席位查询可选，多个用逗号分隔" /></label>
            </template>
            <template v-else>
              <label>控制台 API Host<input v-model="form.volcengineCodingWebBaseUrl" placeholder="https://console.volcengine.com/api/top" /></label>
              <label>Coding ProjectName<input v-model="form.volcengineCodingProjectName" placeholder="默认 default" /></label>
              <label>控制台 Cookie<textarea v-model="form.volcengineCodingWebCookie" autocomplete="off" placeholder="从 console.volcengine.com 登录态请求里复制 Cookie；保存后不会回显"></textarea><small>{{ savedWebCookieLabel }}</small></label>
            </template>
          </template>
          <label v-else>管理 Key<input v-model="form.managementKey" type="password" autocomplete="off" /></label>
          <p class="muted">{{ providerHelp(form.providerType) }}</p>
          <div class="actions">
            <button class="primary" type="submit" :disabled="saving"><Save :size="16" />保存</button>
            <button type="button" @click="test" :disabled="testing || !canTestConnection()"><CheckCircle2 :size="16" />测试</button>
            <button type="button" v-if="currentConnection" @click="store.toggleConnectionEnabled(currentConnection.id, !currentConnection.enabled)"><Power :size="16" />{{ currentConnection.enabled ? '停用' : '启用' }}</button>
            <button type="button" @click="removeCurrent" :disabled="!currentConnection"><Trash2 :size="16" />删除</button>
          </div>
          <p v-if="currentConnection" class="muted">已保存：{{ currentConnection.displayName }} · {{ providerLabel(currentConnection.providerType) }} · {{ currentConnection.maskedManagementKey }}</p>
          <p v-if="notice" class="notice">{{ notice }}</p>
        </form>

        <section class="panel account-panel quota-board">
          <h2>实例账号额度</h2>
          <div v-if="!currentConnection" class="empty-state">保存实例后会在这里展示该实例下的账号。</div>
          <div v-else-if="currentConnectionAccounts.length === 0" class="empty-state">该实例暂无账号数据，保存后点击立即刷新同步额度。</div>
          <div v-else class="connection-groups instance-account-groups">
            <article v-for="account in currentConnectionAccounts" :key="account.id" class="quota-card">
              <div class="quota-card-head">
                <div>
                  <strong>{{ account.displayName }}</strong>
                  <span>{{ account.planName }} · {{ account.status }}<template v-if="account.maskedIdentifier"> · {{ account.maskedIdentifier }}</template></span>
                </div>
                <b>{{ quotaLabel(account) }}</b>
              </div>
              <div v-if="account.windows.length" class="quota-bars">
                <div v-for="window in account.windows" :key="window.id" class="quota-bar-row" :class="windowClass(window)">
                  <span>{{ window.name }}</span>
                  <div class="quota-track"><i :style="{ width: `${windowPercent(window) ?? 0}%` }"></i></div>
                  <strong>{{ windowUsageLabel(window) }}</strong>
                  <b>{{ windowPercentLabel(window) }}</b>
                  <em>{{ resetLabel(window.resetAt) }}</em>
                </div>
              </div>
              <span v-else class="muted">{{ activityLabel(account) }}</span>
            </article>
          </div>
        </section>
      </section>
    </section>
  </main>
</template>
