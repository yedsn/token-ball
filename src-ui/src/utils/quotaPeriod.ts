import type { QuotaAccount, QuotaWindow } from "../types";

export type BalancePeriod = "fiveHour" | "weekly" | "monthly";
export type ResetTimeState = "unknown" | "critical" | "warning" | "healthy";

const resetThresholdHours: Record<BalancePeriod, { critical: number; warning: number }> = {
  fiveHour: { critical: 1, warning: 3 },
  weekly: { critical: 24, warning: 3 * 24 },
  monthly: { critical: 3 * 24, warning: 7 * 24 }
};

export const balancePeriods = [
  { key: "fiveHour", label: "5 小时" },
  { key: "weekly", label: "周" },
  { key: "monthly", label: "月" }
] as const;

export function windowPeriodKey(window: QuotaWindow): BalancePeriod | "other" {
  const type = window.periodType.toLowerCase();
  if (type === "daily") return "other";
  if (type === "weekly") return "weekly";
  if (type === "monthly") return "monthly";
  if (window.periodSeconds === 5 * 60 * 60) return "fiveHour";
  if (window.periodSeconds === 7 * 24 * 60 * 60) return "weekly";
  if (window.periodSeconds === 30 * 24 * 60 * 60) return "monthly";

  const value = `${window.id} ${window.name}`.toLowerCase();
  if (/(?:^|[^0-9])5\s*h\b|five-hour|(?:^|[^0-9])5\s*小时/.test(value)) return "fiveHour";
  if (/week|(?:^|[^0-9])7d\b|每周|近\s*1\s*周/.test(value)) return "weekly";
  if (/month|(?:^|[^0-9])30d\b|每月|近\s*1\s*月/.test(value)) return "monthly";
  return "other";
}

export function accountPeriodWindow(account: QuotaAccount, period: BalancePeriod): QuotaWindow | null {
  return account.windows.find((window) => windowPeriodKey(window) === period) ?? null;
}

export function periodResetLabel(value?: string | null): string {
  if (!value) return "--";
  const date = new Date(value);
  if (!Number.isFinite(date.getTime()) || date.getTime() <= 0) return "--";
  const pad = (part: number) => String(part).padStart(2, "0");
  return `${pad(date.getMonth() + 1)}/${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`;
}

export function accountPeriodResetLabel(account: QuotaAccount, period: BalancePeriod): string {
  return periodResetLabel(accountPeriodWindow(account, period)?.resetAt);
}

export function resetTimeClass(value?: string | null, period?: BalancePeriod | "other"): ResetTimeState {
  if (!value) return "unknown";
  const time = new Date(value).getTime();
  if (!Number.isFinite(time) || time <= 0) return "unknown";
  const hours = (time - Date.now()) / 3600000;
  const thresholds = period && period !== "other"
    ? resetThresholdHours[period]
    : { critical: 3 * 24, warning: 7 * 24 };
  if (hours <= thresholds.critical) return "critical";
  if (hours <= thresholds.warning) return "warning";
  return "healthy";
}

export function accountPeriodResetClass(account: QuotaAccount, period: BalancePeriod) {
  return resetTimeClass(accountPeriodWindow(account, period)?.resetAt, period);
}
