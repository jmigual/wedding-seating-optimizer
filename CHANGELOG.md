# Changelog

All notable changes to this project will be documented in this file.

## [1.0.1] - 2026-06-01

### Fixed

- Missing PNG assets in build.

## [1.0.0] - 2026-05-11

Initial stable release of the wedding seating optimizer workspace.

### Added

- Core wedding seating models, CSV parsing, validation, scoring, rendering, and optimization logic in `seating-core`.
- Command-line workflows in `seating-cli` for validating inputs, generating optimized seatings, scoring arrangements, and rendering seating plans.
- Native desktop application in `seating-gui` with structured editors for people, closeness rules, tables, optimization controls, seating-plan rendering, and CSV import/export.
- Example datasets and GUI documentation assets to support local testing and usage.
- GitHub Actions release automation to build CLI and GUI binaries for Windows and Linux.
