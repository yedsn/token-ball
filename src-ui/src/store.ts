import { defineStore } from "pinia";
import type { ConnectionInput, DisplaySettings, PluginManifest, ProviderConnection, QuotaAccount, QuotaSummary } from "./types";
import {
  type DownloadProgress,
  type UpdateInfo,
  exportConnectionConfigToFile,
  getDisplaySettings,
  getLatestQuota,
  importConnectionConfig,
  importConnectionConfigFromFile,
  listConnections,
  onUpdaterDownloadFinished,
  onUpdaterDownloadProgress,
  onUpdaterDownloadStarted,
  onUpdaterFailed,
  onUpdaterInstalled,
  onUpdaterStatus,
  onProviderError,
  onQuotaUpdated,
  onRefreshCompleted,
  onRefreshStarted,
  onDisplaySettingsUpdated,
  onConnectionsUpdated,
  addPlugin,
  deletePlugin,
  getAppVersion,
  checkForUpdate,
  downloadAndInstallUpdate,
  restartApp,
  setAppIconStyle,
  refreshAllQuota,
  readConnectionConfigBackup,
  setConnectionEnabled,
  saveDisplaySettings,
  saveConnection,
  listPlugins,
  setPluginEnabled,
  testConnection,
 deleteConnection
} from "./services/tauri";

const emptySummary: QuotaSummary = {
  provider: "cliProxyApi",
  totalAccounts: 0,
  availableAccounts: 0,
  lowestRemainingPercent: null,
  nearestResetAt: null,
  lastSyncedAt: null,
  stale: false,
  status: "unknown",
  accounts: []
};

const defaultDisplaySettings: DisplaySettings = {
  showTotalRemaining: true,
  showAvailableAccounts: true,
  showConnectionStatus: true,
  showAccountsInTooltip: true,
  showOrbRefreshButton: true,
  orbAnimationEnabled: true,
  trayIconStyle: "orb",
  appIconStyle: "meter",
  customAppIconDataUrl: "",
  selectedAccountIds: [],
  customItems: []
};

function normalizeDisplaySettings(settings: DisplaySettings): DisplaySettings {
  return { ...defaultDisplaySettings, ...settings, customItems: settings.customItems ?? [], selectedAccountIds: settings.selectedAccountIds ?? [] };
}

export const useTokenBallStore = defineStore("token-ball", {
  state: () => ({
    ready: false,
    refreshing: false,
    error: "",
    connectionError: "",
    connections: [] as ProviderConnection[],
    plugins: [] as PluginManifest[],
    summary: emptySummary as QuotaSummary,
    displaySettings: defaultDisplaySettings as DisplaySettings,
    updater: {
      currentVersion: "",
      available: false,
      version: "",
      notes: "",
      date: "",
      checking: false,
      downloading: false,
      downloaded: 0,
      total: null as number | null,
      percent: null as number | null,
      installed: false,
      failed: false,
      message: ""
    }
  }),
  getters: {
    hasConnection: (state) => state.connections.length > 0,
    percentLabel: (state) => {
      const percent = state.summary.lowestRemainingPercent;
      return typeof percent === "number" ? `${Math.round(percent)}%` : "未知";
    },
    selectedAccounts: (state): QuotaAccount[] => {
      const ids = new Set(state.displaySettings.selectedAccountIds);
      return state.summary.accounts.filter((account) => ids.has(account.id));
    },
    enabledAccounts: (state): QuotaAccount[] => {
      const enabledConnectionIds = new Set(state.connections.filter((connection) => connection.enabled).map((connection) => connection.id));
      return state.summary.accounts.filter((account) => enabledConnectionIds.has(account.connectionId));
    },
    displayAccounts(): QuotaAccount[] {
      return this.selectedAccounts.length ? this.selectedAccounts : this.enabledAccounts;
    },
    totalRemainingPercent: (state) => {
      const enabledConnectionIds = new Set(state.connections.filter((connection) => connection.enabled).map((connection) => connection.id));
      const values = state.summary.accounts
        .filter((account) => enabledConnectionIds.has(account.connectionId))
        .map((account) => account.windows.find((window) => window.id === account.criticalWindowId)?.remainingPercent)
        .filter((value): value is number => typeof value === "number");
      if (!values.length) return null;
      return values.reduce((sum, value) => sum + value, 0) / values.length;
    },
    totalEquivalentAccounts: (state) => {
      const enabledConnectionIds = new Set(state.connections.filter((connection) => connection.enabled).map((connection) => connection.id));
      return state.summary.accounts.filter((account) => enabledConnectionIds.has(account.connectionId)).reduce((sum, account) => {
        const value = account.windows.find((window) => window.id === account.criticalWindowId)?.remainingPercent;
        return sum + (typeof value === "number" ? value / 100 : 0);
      }, 0);
    }
  },
  actions: {
    async init() {
      void this.loadAppVersion();
      const [connectionsR, summaryR, displayR, pluginsR] = await Promise.allSettled([
        listConnections(),
        getLatestQuota(),
        getDisplaySettings(),
        listPlugins(),
      ]);
      if (connectionsR.status === "fulfilled") {
        this.connections = connectionsR.value;
        this.connectionError = "";
      } else {
        this.connectionError = `加载连接列表失败：${String(connectionsR.reason)}`;
      }
      if (summaryR.status === "fulfilled") {
        this.summary = summaryR.value;
        this.error = "";
      }
      else this.error = `加载额度数据失败：${String(summaryR.reason)}`;
      if (displayR.status === "fulfilled") this.displaySettings = normalizeDisplaySettings(displayR.value);
      else this.error = `加载显示设置失败：${String(displayR.reason)}`;
      if (pluginsR.status === "fulfilled") this.plugins = pluginsR.value;
      else this.error = `加载插件列表失败：${String(pluginsR.reason)}`;
      this.ready = true;
      await onQuotaUpdated((summary) => {
        this.summary = summary;
        if (!summary.stale) this.error = "";
      });
      await onRefreshStarted(() => (this.refreshing = true));
      await onRefreshCompleted((summary) => {
        this.summary = summary;
        if (!summary.stale) this.error = "";
        this.refreshing = false;
      });
      await onProviderError((message) => {
        this.error = message;
        this.refreshing = false;
      });
      await onDisplaySettingsUpdated((settings) => {
        this.displaySettings = normalizeDisplaySettings(settings);
      });
      await onConnectionsUpdated((connections) => {
        this.connections = connections;
        this.connectionError = "";
      });
      await onUpdaterStatus((info) => this.applyUpdaterStatus(info));
      await onUpdaterDownloadStarted(() => {
        this.updater.downloading = true;
        this.updater.downloaded = 0;
        this.updater.total = null;
        this.updater.percent = null;
        this.updater.installed = false;
        this.updater.failed = false;
        this.updater.message = "正在下载更新…";
      });
      await onUpdaterDownloadProgress((progress) => {
        this.updater.downloaded = progress.downloaded;
        this.updater.total = progress.total ?? null;
        this.updater.percent = progress.percent ?? null;
      });
      await onUpdaterDownloadFinished(() => {
        this.updater.percent = 100;
        this.updater.message = "下载完成，正在安装…";
      });
      await onUpdaterInstalled(() => {
        this.updater.downloading = false;
        this.updater.installed = true;
        this.updater.message = "安装完成，正在重启…";
      });
      await onUpdaterFailed((message) => {
        this.updater.downloading = false;
        this.updater.checking = false;
        this.updater.failed = true;
        this.updater.message = message;
      });
    },
    async saveProviderConnection(input: ConnectionInput) {
      const connection = await saveConnection(input);
      await this.loadConnections();
      return connection;
    },
    async exportConnectionConfigToFile(filePath: string) {
      return exportConnectionConfigToFile(filePath);
    },
    async importConnectionConfig(backup: Parameters<typeof importConnectionConfig>[0]) {
      const result = await importConnectionConfig(backup);
      await this.loadConnections();
      return result;
    },
    async readConnectionConfigBackup(filePath: string) {
      return readConnectionConfigBackup(filePath);
    },
    async importConnectionConfigFromFile(filePath: string) {
      const result = await importConnectionConfigFromFile(filePath);
      await this.loadConnections();
      return result;
    },
    async loadConnections() {
      try {
        this.connections = await listConnections();
        this.connectionError = "";
      } catch (error) {
        this.connectionError = `加载连接列表失败：${String(error)}`;
      }
    },
    async saveCliProxyConnection(input: Omit<ConnectionInput, "providerType">) {
      return this.saveProviderConnection({ ...input, providerType: "cliProxyApi" });
    },
    async saveVolcengineConnection(input: Omit<ConnectionInput, "providerType">) {
      return this.saveProviderConnection({ ...input, providerType: "volcengine" });
    },
    async saveQianwenConnection(input: Omit<ConnectionInput, "providerType">) {
      return this.saveProviderConnection({ ...input, providerType: "qianwen" });
    },
    async testCliProxyConnection(id: string) {
      await testConnection(id);
      this.error = "";
    },
    async deleteCliProxyConnection(id: string) {
      await deleteConnection(id);
      await this.loadConnections();
    },
    async toggleConnectionEnabled(id: string, enabled: boolean) {
      try {
        const connection = await setConnectionEnabled(id, enabled);
        this.connections = this.connections.map((item) =>
          item.id === id ? { ...item, enabled: connection.enabled } : item
        );
        this.connectionError = "";
      } catch (error) {
        this.connectionError = `切换实例启用状态失败：${String(error)}`;
      }
    },
    async togglePlugin(id: string, enabled: boolean) {
      this.plugins = await setPluginEnabled(id, enabled);
    },
    async addLocalPlugin(input: Parameters<typeof addPlugin>[0]) {
      this.plugins = await addPlugin(input);
    },
    async deleteLocalPlugin(id: string) {
      this.plugins = await deletePlugin(id);
    },
    async saveDisplay(settings: DisplaySettings) {
      this.displaySettings = normalizeDisplaySettings(await saveDisplaySettings(normalizeDisplaySettings(settings)));
    },
    async toggleDisplayAccount(accountId: string, enabled: boolean) {
      const ids = new Set(this.displaySettings.selectedAccountIds);
      if (enabled) ids.add(accountId);
      else ids.delete(accountId);
      await this.saveDisplay({ ...this.displaySettings, selectedAccountIds: [...ids] });
    },
    async toggleTotalRemaining(enabled: boolean) {
      await this.saveDisplay({ ...this.displaySettings, showTotalRemaining: enabled });
    },
    async updateDisplayFlag(key: keyof Pick<DisplaySettings, "showTotalRemaining" | "showAvailableAccounts" | "showConnectionStatus" | "showAccountsInTooltip" | "showOrbRefreshButton" | "orbAnimationEnabled">, enabled: boolean) {
      await this.saveDisplay({ ...this.displaySettings, [key]: enabled });
    },
    async updateTrayIconStyle(style: DisplaySettings["trayIconStyle"]) {
      await this.saveDisplay({ ...this.displaySettings, trayIconStyle: style });
    },
    async updateAppIconStyle(style: DisplaySettings["appIconStyle"], customAppIconDataUrl?: string) {
      const nextCustomIcon = customAppIconDataUrl ?? this.displaySettings.customAppIconDataUrl;
      await setAppIconStyle(style, nextCustomIcon);
      await this.saveDisplay({ ...this.displaySettings, appIconStyle: style, customAppIconDataUrl: nextCustomIcon });
    },
    async addCustomDisplayItem(label: string, value: string) {
      const item = { id: crypto.randomUUID(), label: label.trim(), value: value.trim(), enabled: true };
      await this.saveDisplay({ ...this.displaySettings, customItems: [...this.displaySettings.customItems, item] });
    },
    async updateCustomDisplayItem(id: string, patch: Partial<DisplaySettings["customItems"][number]>) {
      const customItems = this.displaySettings.customItems.map((item) => (item.id === id ? { ...item, ...patch } : item));
      await this.saveDisplay({ ...this.displaySettings, customItems });
    },
    async removeCustomDisplayItem(id: string) {
      await this.saveDisplay({ ...this.displaySettings, customItems: this.displaySettings.customItems.filter((item) => item.id !== id) });
    },
    async refresh() {
      this.refreshing = true;
      try {
        this.summary = await refreshAllQuota();
        if (!this.summary.stale) this.error = "";
      } finally {
        this.refreshing = false;
      }
    },
    applyUpdaterStatus(info: UpdateInfo) {
      this.updater.currentVersion = info.currentVersion;
      this.updater.available = info.available;
      this.updater.version = info.version ?? "";
      this.updater.notes = info.notes ?? "";
      this.updater.date = info.date ?? "";
      this.updater.checking = false;
      this.updater.failed = false;
      this.updater.message = info.available
        ? `发现新版本 ${info.version}`
        : "当前已是最新版本";
    },
    async checkForUpdates() {
      this.updater.checking = true;
      this.updater.failed = false;
      this.updater.message = "正在检查更新…";
      try {
        const info = await checkForUpdate();
        this.applyUpdaterStatus(info);
      } catch (error) {
        this.updater.failed = true;
        this.updater.message = `检查更新失败：${String(error)}`;
      } finally {
        this.updater.checking = false;
      }
    },
    async downloadAndInstall() {
      this.updater.failed = false;
      this.updater.installed = false;
      this.updater.message = "正在下载更新…";
      try {
        await downloadAndInstallUpdate();
      } catch (error) {
        this.updater.downloading = false;
        this.updater.failed = true;
        this.updater.message = `下载安装失败：${String(error)}`;
      }
    },
    async loadAppVersion() {
      try {
        this.updater.currentVersion = await getAppVersion();
      } catch {
        // 版本号读取失败不影响主流程
      }
    },
    restart() {
      void restartApp();
    }
  }
});
