# Spec Delta

## Purpose

确保 TokenBall 在托盘悬浮、鼠标点击、面板关闭与后台额度刷新交错执行时仍能持续响应用户操作，并保证被取消的悬浮请求不会重新打开面板或干扰后续请求。

## ADDED Requirements

### Requirement: Tray interactions remain responsive
The system SHALL continue processing tray and management-window interactions when hover preparation overlaps mouse events or background quota updates, without circular waits caused by shared hover state.

#### Scenario: Pointer events overlap hover preparation
- **WHEN** the user moves into and out of the tray icon while a delayed hover request is preparing its position
- **THEN** subsequent tray events continue to be processed
- **THEN** right-click opens the tray menu and left double-click opens or focuses the management window

#### Scenario: Background refresh overlaps tray use
- **WHEN** quota refresh updates the tray while the user repeatedly hovers, right-clicks, and double-clicks the icon
- **THEN** the tray menu, hover panel, and management window remain usable without restarting TokenBall

### Requirement: Cancelled hover requests cannot override newer user actions
The system SHALL discard obsolete hover requests and preserve the effect of later pointer, menu, and close actions.

#### Scenario: Pointer leaves during hover preparation
- **WHEN** the pointer leaves the tray before the pending hover request is committed for display
- **THEN** completion of that request does not open the hover panel

#### Scenario: Menu or management action cancels pending hover
- **WHEN** the user right-clicks or activates the management window while hover preparation is pending
- **THEN** the earlier request does not subsequently display a panel over the requested surface

#### Scenario: Closing the panel invalidates pending work
- **WHEN** the user closes the hover panel while earlier hover preparation is pending
- **THEN** that earlier request cannot reopen the panel
- **THEN** a new hover after the existing suppression period can open it normally

#### Scenario: Old work finishes after a new hover request
- **WHEN** a cancelled request finishes after a newer valid request
- **THEN** the old request neither displays its panel nor hides or cancels the newer panel

### Requirement: Hover preparation failure permits later interaction
The system SHALL leave tray interactions usable and allow a later valid hover request after a hover query, dispatch, or display failure.

#### Scenario: Failed preparation is followed by another hover
- **WHEN** hover preparation or display fails and the user later hovers again under valid display conditions
- **THEN** the later request can display the panel after the existing hover delay
- **THEN** right-click and management-window activation remain available
