import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ConnectionInput, ProviderConnection, QuotaSummary } from "../types";

export function listConnections(): Promise<ProviderConnection[]> {
  return invoke("connection_list");
}

export function saveConnection(input: ConnectionInput): Promise<ProviderConnection> {
  return invoke("connection_save", { input });
}

export function deleteConnection(id: string): Promise<void> {
  return invoke("connection_delete", { id });
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

export function showWindow(label: string): Promise<void> {
  return invoke("window_show", { label });
}

export function hideWindow(label: string): Promise<void> {
  return invoke("window_hide", { label });
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
