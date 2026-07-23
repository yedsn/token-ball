## 1. Project Setup

- [ ] 1.1 Scaffold a Tauri 2 application with Vue 3, TypeScript, Vite, and Pinia.
- [ ] 1.2 Add Rust dependencies for Tokio, Reqwest, Serde, SQLx SQLite, Chrono, Tracing, Thiserror, and UUID handling.
- [ ] 1.3 Configure frontend dependencies, scripts, TypeScript settings, and base CSS entry points.
- [ ] 1.4 Create the initial Rust module structure for commands, providers, quota, storage, scheduler, events, and windows.
- [ ] 1.5 Configure Tauri capabilities so the frontend can call only the required application commands.

## 2. Desktop Shell

- [ ] 2.1 Configure the quota orb window with transparent, undecorated, always-on-top desktop behavior.
- [ ] 2.2 Configure the simplified management window for setup and status review.
- [ ] 2.3 Implement hover details window or panel creation and positioning near the orb.
- [ ] 2.4 Add a system tray menu with actions to show the orb, open management UI, refresh quota, and exit.
- [ ] 2.5 Enable single-instance behavior so repeated launches activate the existing app instead of starting another scheduler.
- [ ] 2.6 Initialize default local settings for orb size, carousel interval, and synchronization frequency.

## 3. Storage

- [ ] 3.1 Add SQLx SQLite initialization with WAL mode, foreign keys, and busy timeout.
- [ ] 3.2 Create migrations for provider_connections, provider_accounts, quota_snapshots, quota_windows, and settings.
- [ ] 3.3 Implement repositories for saving and loading provider connections.
- [ ] 3.4 Implement repositories for upserting accounts and replacing latest quota snapshots and windows.
- [ ] 3.5 Implement settings persistence for local application preferences.
- [ ] 3.6 Verify first launch creates storage and remains usable with no configured connection.

## 4. Quota Domain

- [ ] 4.1 Define shared Rust models for provider type, connection status, account status, quota unit, period type, quota account, quota window, quota snapshot, and quota summary.
- [ ] 4.2 Define matching TypeScript types for frontend display and command payloads.
- [ ] 4.3 Implement current critical window selection using active windows with computable remaining percentages.
- [ ] 4.4 Implement multi-account summary with available account count, total account count, lowest critical remaining percentage, and nearest reset time.
- [ ] 4.5 Implement unknown quota handling so unknown values are never displayed as 0%.
- [ ] 4.6 Implement redaction helpers for management keys, authorization values, and other sensitive strings.
- [ ] 4.7 Add unit tests for critical window selection, equal-percent reset tie-breaks, unknown quota, summary aggregation, and redaction.

## 5. CLIProxyAPI Provider

- [ ] 5.1 Implement CLIProxyAPI connection configuration commands for create, update, list, and delete.
- [ ] 5.2 Implement a CLIProxyAPI HTTP client with base URL normalization, management key headers, request timeout, and structured errors.
- [ ] 5.3 Implement connection testing with user-readable error mapping and sensitive value redaction.
- [ ] 5.4 Implement account discovery from CLIProxyAPI managed auth files or equivalent account endpoint.
- [ ] 5.5 Implement quota fetching strategy for the selected CLIProxyAPI quota source based on real response shape.
- [ ] 5.6 Implement mapper logic from CLIProxyAPI responses into the unified quota domain model.
- [ ] 5.7 Persist discovered accounts, latest quota snapshots, and quota windows after successful synchronization.
- [ ] 5.8 Add provider tests with mocked success, unauthorized, connection refused, timeout, invalid JSON, and missing-field responses.

## 6. Synchronization and Commands

- [ ] 6.1 Implement quota_get_latest command to return cached normalized summary and account details.
- [ ] 6.2 Implement quota_refresh_all command for manual refresh across configured connections.
- [ ] 6.3 Implement a background quota scheduler using the configured synchronization interval.
- [ ] 6.4 Add per-connection synchronization locking to prevent overlapping sync runs.
- [ ] 6.5 Emit quota update, refresh started, refresh completed, and provider error events to the frontend.
- [ ] 6.6 Preserve previous cached data on synchronization failure and mark stale status with last successful sync time.

## 7. Frontend UI

- [ ] 7.1 Create Pinia stores for quota state, connection state, settings, and UI window state.
- [ ] 7.2 Implement Tauri service wrappers for connection and quota commands plus event listeners.
- [ ] 7.3 Build the quota orb component with percentage/account/reset views and normal, warning, exhausted, stale, and error states.
- [ ] 7.4 Implement orb carousel timing and hover pause behavior.
- [ ] 7.5 Build the hover details panel listing CLIProxyAPI accounts, status, critical remaining percentage, reset time, and last sync time.
- [ ] 7.6 Build the simplified management UI for CLIProxyAPI setup, connection test, current status, and manual refresh.
- [ ] 7.7 Implement empty/setup state when no connection is configured.
- [ ] 7.8 Implement stale data indicators when cached data is shown after failed synchronization.

## 8. Verification

- [ ] 8.1 Run Rust formatting, linting where configured, and unit tests.
- [ ] 8.2 Run frontend type checking and build.
- [ ] 8.3 Run the Tauri app locally and verify first launch, connection setup, manual refresh, cached display, and tray menu behavior.
- [ ] 8.4 Verify Chinese UI text and Markdown documents remain readable with no mojibake, truncation, or accidental replacement.
- [ ] 8.5 Document remaining open questions for CLIProxyAPI quota endpoint differences and default synchronization frequency.
