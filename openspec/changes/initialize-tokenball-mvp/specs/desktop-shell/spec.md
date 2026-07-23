## ADDED Requirements

### Requirement: Desktop application shell
The system SHALL provide a Tauri 2 desktop application shell with a Vue 3 and TypeScript frontend and a Rust backend.

#### Scenario: Application starts successfully
- **WHEN** the user launches TokenBall
- **THEN** the system opens the desktop application without requiring a browser
- **THEN** the Rust backend is available for Tauri Commands
- **THEN** the Vue frontend is rendered in an application window

### Requirement: Window structure
The system SHALL provide separate desktop surfaces for the quota orb, hover details, and simplified management UI.

#### Scenario: Orb and management surfaces are available
- **WHEN** the application starts
- **THEN** the system can display a small quota orb window
- **THEN** the system can display a hover details surface near the orb
- **THEN** the system can open a simplified management window for configuration and status review

### Requirement: Local settings and cache initialization
The system SHALL initialize local storage for settings, provider connections, accounts, and latest quota state.

#### Scenario: First launch creates storage
- **WHEN** the user launches TokenBall for the first time
- **THEN** the system creates the required local database or storage files
- **THEN** the system creates default settings for orb display and synchronization
- **THEN** the application remains usable when no provider connection has been configured

### Requirement: System tray entry
The system SHALL provide a system tray entry for basic application access.

#### Scenario: Tray menu opens management UI
- **WHEN** the user selects the management action from the tray menu
- **THEN** the system opens or focuses the management window

### Requirement: Single local instance
The system MUST prevent multiple background synchronization processes from running for the same user session.

#### Scenario: Application launched twice
- **WHEN** the user starts TokenBall while another instance is already running
- **THEN** the system focuses or activates the existing instance
- **THEN** the system does not start a second synchronization scheduler
