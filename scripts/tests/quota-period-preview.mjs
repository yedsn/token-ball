import { createServer } from "vite";

// Isolated UI verification: synthetic data only, no desktop API or credentials.
const syncedAt = new Date(2026, 9, 29, 8, 0).toISOString();
const date = (month, day, hour = 0, minute = 0) => new Date(2026, month - 1, day, hour, minute).toISOString();
const afterHours = (hours) => new Date(Date.now() + hours * 3600000).toISOString();
const quotaWindow = (id, name, percent, resetAt, props = {}) => ({
  id, name, periodType: "custom", remainingPercent: percent, resetAt,
  total: 100, used: percent === null ? null : 100 - percent, remaining: percent,
  unit: "percent", isActive: true, isCurrentConstraint: true, dataSource: "synthetic-test", ...props
});
const account = (id, displayName, windows, props = {}) => ({
  id, displayName, connectionId: "test-enabled", externalId: id,
  planName: "回归样例", status: "available", windows,
  criticalWindowId: windows[0]?.id ?? null, nextResetAt: date(10, 30, 12),
  recentRequests: [], subscriptionUntil: date(12, 10), syncedAt, ...props
});
const accounts = [
  account("all-periods", "三周期 · 不同时间", [
    quotaWindow("all-5h", "5h", 80, afterHours(0.5), { periodSeconds: 18000 }),
    quotaWindow("all-week", "每周", 50, afterHours(48), { periodType: "weekly" }),
    quotaWindow("all-month", "每月", 20, afterHours(240), { periodType: "monthly" })
  ], { subscriptionUntil: date(12, 1) }),
  account("volc-afp", "火山 AFP · 五小时别名", [
    quotaWindow("afp-five-hour", "5 小时 AFP", 90, date(10, 29, 19, 15), { periodSeconds: null }),
    quotaWindow("afp-weekly", "每周 AFP", 70, date(11, 3), { periodType: "weekly" }),
    quotaWindow("afp-monthly", "每月 AFP", 65, date(11, 21), { periodType: "monthly" }),
    quotaWindow("afp-daily", "每日 AFP", 10, date(10, 30), { periodType: "daily" })
  ], { subscriptionUntil: date(12, 2) }),
  account("cli-two", "CLIProxyAPI · 两周期", [
    quotaWindow("cli-5h", "5h", 75, date(10, 29, 20), { periodType: "rolling", periodSeconds: 18000 }),
    quotaWindow("cli-7d", "7d", 45, date(11, 4), { periodType: "rolling", periodSeconds: 604800 })
  ], { subscriptionUntil: date(12, 3) }),
  account("qianwen-missing", "千问 · 额度未知 / 时间缺失", [
    quotaWindow("qianwen-5h", "5 小时", null, date(10, 29, 21)),
    quotaWindow("qianwen-weekly", "每周", 80, null, { periodType: "weekly" })
  ], { subscriptionUntil: date(12, 4) }),
  account("invalid-past", "无效日期 / 缓存过去时间", [
    quotaWindow("invalid-5h", "5h", 0, "invalid-date"),
    quotaWindow("invalid-weekly", "每周", 30, "1970-01-01T00:00:00Z", { periodType: "weekly" }),
    quotaWindow("past-monthly", "每月", 60, new Date(2020, 0, 2, 3, 4).toISOString(), { periodType: "monthly" })
  ], { subscriptionUntil: date(12, 5) }),
  account("empty", "无周期 · 不借用汇总时间", [], { subscriptionUntil: date(12, 6) }),
  account("duplicate-session", "首匹配 / 不推断会话周期", [
    quotaWindow("first-5h", "5h", 85, null),
    quotaWindow("second-5h", "5h", 15, date(10, 29, 22)),
    quotaWindow("coding-web-session", "当前会话", 50, date(10, 30))
  ], { subscriptionUntil: date(12, 7) }),
  account("disabled", "禁用实例 · 不应出现", [], { connectionId: "test-disabled" })
];
const connections = [true, false].map((enabled) => ({
  id: enabled ? "test-enabled" : "test-disabled", providerType: "cliProxyApi",
  displayName: enabled ? "合成数据实例" : "禁用实例", baseUrl: "http://test.invalid",
  enabled, status: "healthy", createdAt: syncedAt, updatedAt: syncedAt
}));
const summary = {
  provider: "cliProxyApi", accounts, totalAccounts: accounts.length, availableAccounts: 7,
  lowestRemainingPercent: 20, nearestResetAt: date(10, 29, 18, 30),
  lastSyncedAt: syncedAt, stale: true, status: "degraded"
};

const bridge = `(() => {
  const data = ${JSON.stringify({ connections, summary })};
  const callbacks = new Map();
  let callbackId = 0;
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: new URLSearchParams(location.search).get('view') === 'hover' ? 'hover' : 'main' } },
    transformCallback: (callback) => { callbacks.set(++callbackId, callback); return callbackId; },
    unregisterCallback: (id) => callbacks.delete(id),
    invoke: async (command) => {
      if (command === 'connection_list') return data.connections;
      if (command === 'quota_get_latest' || command === 'quota_refresh_all') return data.summary;
      if (command === 'settings_get_display') return {};
      if (command === 'plugin_list') return [];
      if (command === 'plugin:event|listen') return ++callbackId;
      if (command === 'orb_get_visible') return true;
      if (command === 'updater_check') return { available: false };
      return null;
    }
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };
})();`;

const server = await createServer({
  server: { host: "127.0.0.1", port: 1421, strictPort: true },
  plugins: [{ name: "quota-period-synthetic-preview", transformIndexHtml: {
    order: "pre", handler: () => [{ tag: "script", children: bridge, injectTo: "head-prepend" }]
  } }]
});
await server.listen();
server.printUrls();
