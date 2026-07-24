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
  providerType: "cliProxyApi";
  displayName: string;
  baseUrl: string;
  enabled: boolean;
  status: ConnectionStatus;
  lastSyncedAt?: string | null;
  createdAt: string;
  updatedAt: string;
  maskedManagementKey?: string | null;
}

export interface ConnectionInput {
  id?: string;
  displayName: string;
  baseUrl: string;
  managementKey: string;
  enabled?: boolean;
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
  syncedAt: string;
}

export interface QuotaSummary {
  provider: "cliProxyApi";
  totalAccounts: number;
  availableAccounts: number;
  lowestRemainingPercent?: number | null;
  nearestResetAt?: string | null;
  lastSyncedAt?: string | null;
  stale: boolean;
  status: ConnectionStatus;
  accounts: QuotaAccount[];
}
