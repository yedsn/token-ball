import { defineStore } from "pinia";
import type { ConnectionInput, DisplaySettings, PluginManifest, ProviderConnection, QuotaAccount, QuotaSummary } from "./types";
import {
  getDisplaySettings,
  getLatestQuota,
  listConnections,
  onProviderError,
  onQuotaUpdated,
  onRefreshCompleted,
  onRefreshStarted,
  onDisplaySettingsUpdated,
  addPlugin,
  deletePlugin,
  refreshAllQuota,
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
    displaySettings: defaultDisplaySettings as DisplaySettings
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
    displayAccounts(): QuotaAccount[] {
      return this.selectedAccounts.length ? this.selectedAccounts : this.summary.accounts;
    },
    totalRemainingPercent: (state) => {
      const values = state.summary.accounts
        .map((account) => account.windows.find((window) => window.id === account.criticalWindowId)?.remainingPercent)
        .filter((value): value is number => typeof value === "number");
      if (!values.length) return null;
      return values.reduce((sum, value) => sum + value, 0) / values.length;
    },
    totalEquivalentAccounts: (state) => {
      return state.summary.accounts.reduce((sum, account) => {
        const value = account.windows.find((window) => window.id === account.criticalWindowId)?.remainingPercent;
        return sum + (typeof value === "number" ? value / 100 : 0);
      }, 0);
    }
  },
  actions: {
    async init() {
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
      if (summaryR.status === "fulfilled") this.summary = summaryR.value;
      else this.error = `加载额度数据失败：${String(summaryR.reason)}`;
      if (displayR.status === "fulfilled") this.displaySettings = normalizeDisplaySettings(displayR.value);
      else this.error = `加载显示设置失败：${String(displayR.reason)}`;
      if (pluginsR.status === "fulfilled") this.plugins = pluginsR.value;
      else this.error = `加载插件列表失败：${String(pluginsR.reason)}`;
      this.ready = true;
      await onQuotaUpdated((summary) => (this.summary = summary));
      await onRefreshStarted(() => (this.refreshing = true));
      await onRefreshCompleted((summary) => {
        this.summary = summary;
        this.refreshing = false;
      });
      await onProviderError((message) => {
        this.error = message;
        this.refreshing = false;
      });
      await onDisplaySettingsUpdated((settings) => {
        this.displaySettings = normalizeDisplaySettings(settings);
      });
    },
    async saveProviderConnection(input: ConnectionInput) {
      const connection = await saveConnection(input);
      await this.loadConnections();
      return connection;
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
    async testCliProxyConnection(id: string) {
      await testConnection(id);
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
      } finally {
        this.refreshing = false;
      }
    }
  }
});
