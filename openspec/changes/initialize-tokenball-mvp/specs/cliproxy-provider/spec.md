## ADDED Requirements

### Requirement: CLIProxyAPI connection configuration
The system SHALL allow the user to configure a CLIProxyAPI connection with a display name, base URL, and management key.

#### Scenario: User saves a connection
- **WHEN** the user enters a display name, base URL, and management key
- **THEN** the system saves the connection for local use
- **THEN** the system does not display the complete management key after saving

### Requirement: CLIProxyAPI connection test
The system SHALL test a configured CLIProxyAPI connection before or after saving it.

#### Scenario: Connection test succeeds
- **WHEN** the configured CLIProxyAPI endpoint is reachable and accepts the management key
- **THEN** the system reports the connection as healthy

#### Scenario: Connection test fails
- **WHEN** the CLIProxyAPI endpoint is unreachable, unauthorized, or returns an invalid response
- **THEN** the system reports a user-readable failure reason
- **THEN** the system does not expose the complete management key in the failure details

### Requirement: CLIProxyAPI account discovery
The system SHALL discover Codex accounts managed by CLIProxyAPI.

#### Scenario: Accounts are returned
- **WHEN** the system synchronizes a healthy CLIProxyAPI connection
- **THEN** the system reads the available managed auth files or accounts
- **THEN** the system stores each account with a stable external identifier, display name, masked identifier when available, plan name, and status

### Requirement: CLIProxyAPI quota synchronization
The system SHALL synchronize quota windows and account status from CLIProxyAPI into the unified quota model.

#### Scenario: Quota data is available
- **WHEN** CLIProxyAPI returns quota information for an account
- **THEN** the system stores the quota snapshot and quota windows locally
- **THEN** the system calculates the account critical window
- **THEN** the system emits or exposes updated normalized quota state to the frontend

### Requirement: CLIProxyAPI error mapping
The system SHALL map CLIProxyAPI network, authentication, timeout, unsupported version, and invalid response errors into user-readable application errors.

#### Scenario: Provider returns an error
- **WHEN** CLIProxyAPI synchronization fails
- **THEN** the system records the connection as degraded or failed
- **THEN** the system presents an actionable error message to the user
- **THEN** the system preserves existing cached quota data when available

### Requirement: Per-connection synchronization lock
The system MUST prevent overlapping synchronization runs for the same CLIProxyAPI connection.

#### Scenario: Manual refresh during scheduled sync
- **WHEN** a scheduled sync is already running for a CLIProxyAPI connection and the user triggers manual refresh
- **THEN** the system does not run two concurrent sync operations for that connection
- **THEN** the system either queues, skips, or reuses the in-flight synchronization result
