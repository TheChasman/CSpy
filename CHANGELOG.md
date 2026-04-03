# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-04-03

### Added
- Initial release: real-time Claude quota monitoring in macOS menu bar
- 5-hour quota progress bar with countdown timer
- Burn rate indicator (%/hour)
- Dynamic tray icon reflecting usage level
- Popover display with usage details
- Support for OAuth token from Claude Code or token file (~/.config/cspy/token)
- 3-minute polling with quiet hours (23:00-08:00)
- Auto-reload on token staleness (429 errors)
