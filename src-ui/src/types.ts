export type ConnectionStatus = "unknown" | "healthy" | "degraded" | "failed";
export type AccountStatus =
  | "available"
  | "warning"
  | "cooldown"
  | "exhausted"
  | "disabled"
  | "authExpired"
  | "offline"
  | "error"
  | "unknown";

export interface ProviderConnection {
  id: string;
  providerType: ProviderType;
  displayName: string;
  baseUrl: string;
  enabled: boolean;
  status: ConnectionStatus;
  lastSyncedAt?: string | null;
  createdAt: string;
  updatedAt: string;
  maskedManagementKey?: string | null;
  providerConfigHint?: {
    region?: string | null;
    service?: string | null;
    hasAccessKeyId?: boolean;
    hasSecretAccessKey?: boolean;
    channel?: string | null;
    syncAgentPlan?: boolean;
    syncCodingPlan?: boolean;
    codingProjectName?: string | null;
    codingSeatId?: string | null;
    codingWebBaseUrl?: string | null;
    hasCodingWebCookie?: boolean;
    qianwenProductCode?: string | null;
    qianwenGatewayBaseUrl?: string | null;
    hasQianwenCookie?: boolean;
  } | null;
}

export interface ConnectionInput {
  id?: string;
  providerType: ProviderType;
  displayName: string;
  baseUrl: string;
  managementKey: string;
  enabled?: boolean;
}

export type ProviderType = "cliProxyApi" | "volcengine" | "qianwen";

export interface PluginManifest {
  id: string;
  name: string;
  version: string;
  category: string;
  capability: string;
  permissions: string[];
  installed: boolean;
  enabled: boolean;
  configurable: boolean;
  builtIn: boolean;
  settingsKey?: string | null;
}

export interface PluginInput {
  id: string;
  name: string;
  version: string;
  category: string;
  capability: string;
  permissions: string[];
  configurable?: boolean;
}

export interface QuotaWindow {
  id: string;
  name: string;
  periodType: string;
  periodSeconds?: number | null;
  total?: number | null;
  used?: number | null;
  remaining?: number | null;
  remainingPercent?: number | null;
  unit: string;
  resetAt?: string | null;
  isActive: boolean;
  isCurrentConstraint: boolean;
  dataSource: string;
}

export interface QuotaAccount {
  id: string;
  connectionId: string;
  externalId: string;
  displayName: string;
  maskedIdentifier?: string | null;
  planName: string;
  status: AccountStatus;
  windows: QuotaWindow[];
  criticalWindowId?: string | null;
  nextResetAt?: string | null;
  successCount?: number | null;
  failedCount?: number | null;
  recentRequests: RequestActivity[];
  subscriptionUntil?: string | null;
  chatgptAccountId?: string | null;
  syncedAt: string;
}

export interface RequestActivity {
  time: string;
  success: number;
  failed: number;
}

export interface DisplaySettings {
  showTotalRemaining: boolean;
  showAvailableAccounts: boolean;
  showConnectionStatus: boolean;
  showAccountsInTooltip: boolean;
  showOrbRefreshButton: boolean;
  orbAnimationEnabled: boolean;
  trayIconStyle: "orb" | "minimal";
  appIconStyle: "meter" | "orb" | "custom";
  customAppIconDataUrl: string;
  selectedAccountIds: string[];
  customItems: DisplayCustomItem[];
}

export interface DisplayCustomItem {
  id: string;
  label: string;
  value: string;
  enabled: boolean;
}

export interface QuotaSummary {
  provider: ProviderType;
  totalAccounts: number;
  availableAccounts: number;
  lowestRemainingPercent?: number | null;
  nearestResetAt?: string | null;
  lastSyncedAt?: string | null;
  stale: boolean;
  status: ConnectionStatus;
  accounts: QuotaAccount[];
}
