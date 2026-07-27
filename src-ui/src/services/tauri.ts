import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ConfigBackupInfo, ConnectionBackup, ConnectionInput, DisplaySettings, ExportConfigResult, ImportConfigResult, PluginInput, PluginManifest, ProviderConnection, QuotaSummary } from "../types";

export function listConnections(): Promise<ProviderConnection[]> {
  return invoke("connection_list");
}

export function saveConnection(input: ConnectionInput): Promise<ProviderConnection> {
  return invoke("connection_save", { input });
}

export function exportConnectionConfigToFile(filePath: string): Promise<ExportConfigResult> {
  return invoke("connection_export_config_to_file", { filePath });
}

export function importConnectionConfig(backup: ConnectionBackup): Promise<ImportConfigResult> {
  return invoke("connection_import_config", { backup });
}

export function readConnectionConfigBackup(filePath: string): Promise<ConfigBackupInfo> {
  return invoke("connection_read_config_backup", { filePath });
}

export function importConnectionConfigFromFile(filePath: string): Promise<ImportConfigResult> {
  return invoke("connection_import_config_from_file", { filePath });
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

export function minimizeMainWindow(): Promise<void> {
  return invoke("window_minimize_main");
}

export function toggleMainWindowMaximize(): Promise<void> {
  return invoke("window_toggle_main_maximize");
}

export function closeMainWindow(): Promise<void> {
  return invoke("window_close_main");
}

export function openExternalUrl(url: string): Promise<void> {
  return invoke("open_external_url", { url });
}

export function getOrbVisible(): Promise<boolean> {
  return invoke("orb_get_visible");
}

export interface UpdateInfo {
  currentVersion: string;
  available: boolean;
  version?: string;
  date?: string;
  notes?: string;
}

export interface DownloadProgress {
  downloaded: number;
  total?: number;
  percent?: number;
}

export function checkForUpdate(): Promise<UpdateInfo> {
  return invoke("updater_check");
}

export function downloadAndInstallUpdate(): Promise<void> {
  return invoke("updater_download_and_install");
}

export function restartApp(): Promise<void> {
  return invoke("updater_restart");
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

export function onConnectionsUpdated(callback: (connections: ProviderConnection[]) => void) {
  return listen<ProviderConnection[]>("connection://updated", (event) => callback(event.payload));
}

export function onShowOverview(callback: () => void) {
  return listen("main://show-overview", () => callback());
}

export function onShowUpdate(callback: () => void) {
  return listen("main://show-update", () => callback());
}

export function onUpdaterStatus(callback: (info: UpdateInfo) => void) {
  return listen<UpdateInfo>("updater://status", (event) => callback(event.payload));
}

export function onUpdaterDownloadStarted(callback: () => void) {
  return listen("updater://download-started", () => callback());
}

export function onUpdaterDownloadProgress(callback: (progress: DownloadProgress) => void) {
  return listen<DownloadProgress>("updater://download-progress", (event) => callback(event.payload));
}

export function onUpdaterDownloadFinished(callback: () => void) {
  return listen("updater://download-finished", () => callback());
}

export function onUpdaterInstalled(callback: () => void) {
  return listen("updater://installed", () => callback());
}

export function onUpdaterFailed(callback: (message: string) => void) {
  return listen<string>("updater://failed", (event) => callback(event.payload));
}
