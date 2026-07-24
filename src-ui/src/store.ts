import { defineStore } from "pinia";
import type { ConnectionInput, ProviderConnection, QuotaSummary } from "./types";
import {
  getLatestQuota,
  listConnections,
  onProviderError,
  onQuotaUpdated,
  onRefreshCompleted,
  onRefreshStarted,
  refreshAllQuota,
  saveConnection,
  testConnection
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

export const useTokenBallStore = defineStore("token-ball", {
  state: () => ({
    ready: false,
    refreshing: false,
    error: "",
    connections: [] as ProviderConnection[],
    summary: emptySummary as QuotaSummary
  }),
  getters: {
    hasConnection: (state) => state.connections.length > 0,
    percentLabel: (state) => {
      const percent = state.summary.lowestRemainingPercent;
      return typeof percent === "number" ? `${Math.round(percent)}%` : "--";
    }
  },
  actions: {
    async init() {
      const [connections, summary] = await Promise.all([listConnections(), getLatestQuota()]);
      this.connections = connections;
      this.summary = summary;
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
    },
    async saveCliProxyConnection(input: ConnectionInput) {
      const connection = await saveConnection(input);
      this.connections = await listConnections();
      return connection;
    },
    async testCliProxyConnection(id: string) {
      await testConnection(id);
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
