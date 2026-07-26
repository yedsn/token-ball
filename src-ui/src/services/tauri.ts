import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ConnectionInput, DisplaySettings, PluginInput, PluginManifest, ProviderConnection, QuotaSummary } from "../types";

export function listConnections(): Promise<ProviderConnection[]> {
  return invoke("connection_list");
}

export function saveConnection(input: ConnectionInput): Promise<ProviderConnection> {
  return invoke("connection_save", { input });
}

export function deleteConnection(id: string): Promise<void> {
  return invoke("connection_delete", { id });
}

export function setConnectionEnabled(id: string, enabled: boolean): Promise<ProviderConnection> {
  return invoke("connection_set_enabled", { id, enabled });
}

export function testConnection(id: string): Promise<void> {
  return invoke("connection_test", { id });
}

export function getLatestQuota(): Promise<QuotaSummary> {
  return invoke("quota_get_latest");
}

export function refreshAllQuota(): Promise<QuotaSummary> {
  return invoke("quota_refresh_all");
}

export function getDisplaySettings(): Promise<DisplaySettings> {
  return invoke("settings_get_display");
}

export function saveDisplaySettings(settings: DisplaySettings): Promise<DisplaySettings> {
  return invoke("settings_save_display", { settings });
}

export function setAppIconStyle(style: DisplaySettings["appIconStyle"], customDataUrl?: string): Promise<void> {
  return invoke("app_icon_set_style", { style, customDataUrl: customDataUrl || null });
}

export function listPlugins(): Promise<PluginManifest[]> {
  return invoke("plugin_list");
}

export function setPluginEnabled(id: string, enabled: boolean): Promise<PluginManifest[]> {
  return invoke("plugin_set_enabled", { id, enabled });
}

export function addPlugin(input: PluginInput): Promise<PluginManifest[]> {
  return invoke("plugin_add", { input });
}

export function deletePlugin(id: string): Promise<PluginManifest[]> {
  return invoke("plugin_delete", { id });
}

export function showWindow(label: string): Promise<void> {
  return invoke("window_show", { label });
}

export function hideWindow(label: string): Promise<void> {
  return invoke("window_hide", { label });
}

export function openMainOverview(): Promise<void> {
  return invoke("window_open_main_overview");
}

export function getOrbVisible(): Promise<boolean> {
  return invoke("orb_get_visible");
}

export function onQuotaUpdated(callback: (summary: QuotaSummary) => void) {
  return listen<QuotaSummary>("quota://updated", (event) => callback(event.payload));
}

export function onRefreshStarted(callback: () => void) {
  return listen("quota://refresh-started", () => callback());
}

export function onRefreshCompleted(callback: (summary: QuotaSummary) => void) {
  return listen<QuotaSummary>("quota://refresh-completed", (event) => callback(event.payload));
}

export function onProviderError(callback: (message: string) => void) {
  return listen<string>("provider://error", (event) => callback(event.payload));
}

export function onDisplaySettingsUpdated(callback: (settings: DisplaySettings) => void) {
  return listen<DisplaySettings>("settings://display-updated", (event) => callback(event.payload));
}

export function onShowOverview(callback: () => void) {
  return listen("main://show-overview", () => callback());
}
