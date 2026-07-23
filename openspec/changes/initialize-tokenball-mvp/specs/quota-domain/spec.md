## ADDED Requirements

### Requirement: Unified quota account model
The system SHALL represent provider quota data using a unified account and quota window model before exposing it to the frontend.

#### Scenario: Provider data is normalized
- **WHEN** a provider returns account and quota data
- **THEN** the system maps the data into provider connection, quota account, quota window, and quota snapshot records
- **THEN** the frontend receives normalized quota data rather than provider-specific raw responses

### Requirement: Critical window selection
The system SHALL calculate the current critical quota window for each account using active windows with computable remaining percentages.

#### Scenario: Lowest remaining percentage wins
- **WHEN** an account has multiple active quota windows with remaining percentages
- **THEN** the system marks the active window with the lowest remaining percentage as the current critical window

#### Scenario: Equal percentages use later reset
- **WHEN** multiple active quota windows have the same lowest remaining percentage
- **THEN** the system selects the window with the later reset time as the current critical window

#### Scenario: Unknown windows are ignored
- **WHEN** a quota window has no computable remaining percentage
- **THEN** the system does not use that window as the current critical window

### Requirement: Multi-account summary
The system SHALL summarize CLIProxyAPI accounts using available account count, lowest critical remaining percentage, and nearest reset time.

#### Scenario: Accounts are summarized
- **WHEN** multiple accounts have synchronized quota data
- **THEN** the system reports the number of accounts currently available
- **THEN** the system reports the total number of known accounts
- **THEN** the system reports the lowest critical remaining percentage among computable accounts
- **THEN** the system reports the nearest known reset time among constrained accounts

### Requirement: Unknown quota is distinct from exhausted quota
The system MUST distinguish unknown quota state from zero remaining quota.

#### Scenario: Synchronization cannot determine quota
- **WHEN** the system cannot calculate remaining quota for an account
- **THEN** the system marks the quota value as unknown
- **THEN** the system does not display the account as 0% solely because the value is unknown

### Requirement: Cached state degradation
The system SHALL retain and expose the last successful quota snapshot when synchronization fails.

#### Scenario: Sync fails after a successful snapshot
- **WHEN** a provider synchronization fails after previous successful data exists
- **THEN** the system keeps the previous quota snapshot available for display
- **THEN** the system marks the data with its last successful synchronization time
- **THEN** the system marks the connection or summary as possibly stale

### Requirement: Sensitive value redaction
The system MUST redact sensitive values in logs and user-visible errors.

#### Scenario: Error includes a management key
- **WHEN** an internal error contains a CLIProxyAPI management key or authorization value
- **THEN** the system writes only a redacted value to logs
- **THEN** the system does not show the complete sensitive value in the UI
