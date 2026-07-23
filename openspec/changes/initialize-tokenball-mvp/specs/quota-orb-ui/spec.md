## ADDED Requirements

### Requirement: Quota orb summary display
The system SHALL display the current normalized quota summary in a small desktop orb window.

#### Scenario: Summary data is available
- **WHEN** synchronized quota summary data exists
- **THEN** the orb displays the current available percentage or account availability summary
- **THEN** the orb uses visual state to distinguish normal, warning, exhausted, stale, and error states

### Requirement: Orb carousel
The system SHALL support a simple carousel of important quota views.

#### Scenario: Multiple views are available
- **WHEN** the summary includes overall quota, CLIProxyAPI account availability, and next reset time
- **THEN** the orb rotates through the available views on a timed interval
- **THEN** the user can pause rotation by hovering the orb

### Requirement: Hover details panel
The system SHALL show a hover details panel with account-level quota status.

#### Scenario: User hovers over orb
- **WHEN** the user hovers over the quota orb
- **THEN** the system displays a details panel near the orb
- **THEN** the panel lists CLIProxyAPI accounts, their status, critical remaining percentage when known, and reset time when known
- **THEN** the panel displays the last successful synchronization time

### Requirement: Manual refresh action
The system SHALL provide a user action to refresh quota data manually.

#### Scenario: User triggers refresh
- **WHEN** the user clicks the refresh action in the UI or tray
- **THEN** the system requests a quota refresh for configured connections
- **THEN** the UI shows refresh progress or an updated synchronization state

### Requirement: Empty and setup states
The system SHALL provide useful UI states when no CLIProxyAPI connection or no quota data exists.

#### Scenario: No connection configured
- **WHEN** the user opens TokenBall before configuring CLIProxyAPI
- **THEN** the orb and management UI indicate that setup is required
- **THEN** the management UI provides access to connection configuration

### Requirement: Stale data indication
The system SHALL clearly indicate when displayed data comes from cache after a failed synchronization.

#### Scenario: Cached data is displayed after failure
- **WHEN** the latest synchronization fails and cached quota data is available
- **THEN** the orb or hover panel marks the data as stale or possibly outdated
- **THEN** the hover panel shows the last successful synchronization time
