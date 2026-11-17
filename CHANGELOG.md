# Changelog

All notable changes to Irani IDM are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.3.0]

### Added
- Display the application version in the main window.

### Changed
- Use the running application's version when checking for updates.

## [1.2.2]

### Fixed
- Updated release version handling to keep `Cargo.toml`, `Cargo.lock`, and `tauri.conf.json` synchronized during builds.

## [1.2.0]

### Added
- Added application-wide HTTP and SOCKS5 proxy settings, including optional authentication, for download requests.
- Added proxy usage indicators to the main download list and download detail windows.
- Added eleven selectable color themes, with light and dark variants.
- Synchronized the selected color theme across the main window and download detail windows.
- Added English and Persian application screenshots to the README.

