# Changelog

All notable changes to Irani IDM are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [2.0.2]

### Changed
- Refresh the README screenshots with the current main window, download details, and Persian settings screens.

## [2.0.1]

### Added
- Add a direct link to the published Firefox Add-ons listing from Browser integration settings.

### Changed
- Refresh the README main-window screenshot to show the 2.0 glass interface.

## [2.0.0]

### Added
- Add daily download scheduling with configurable start/end times and optional shutdown or sleep actions at the end of the schedule.
- Add a configurable limit for simultaneous downloads; excess downloads wait in the queue.
- Add a pastel color palette and animated sparkle effects to the main and download-detail windows.

### Changed
- Improve the glass UI, download-list rendering and selection behavior; reverse horizontal hover motion in RTL languages.

### Fixed
- Trigger the configured end action when a schedule ends.
- Apply language changes to open download-detail windows and stop the empty-search indicator from blinking indefinitely.

## [1.16.0]

### Added
- Add Chinese, Russian, Arabic, Urdu, Dutch, and Spanish translations for the desktop interface.
- Add locale-aware number and date formatting for supported languages.
- Add RTL layout support for Arabic and Urdu.

### Changed
- Allow browser-install guide routing to accept all supported app locales, with English fallback where localized guides are not yet available.

## [1.15.1]

### Fixed
- Avoid a dead-code warning for the Windows-only native-host URL validator on non-Windows builds.

## [1.15.0]

### Changed
- Rebalance active downloads dynamically: when a connection becomes free, split the largest remaining byte range in half and assign the new half to that connection.

### Fixed
- Apply the user's configured connections-per-download setting consistently to manual downloads, browser Native Messaging handoffs, and `iranidm://` links, and persist it for future sessions.

## [1.14.1]

### Fixed
- Honor the download folder selected in Settings for manual downloads, browser handoffs, and `iranidm://` links, and persist the selection across app restarts.

## [1.14.0]

### Added
- Add a manual update check and show release notes in the update prompt.

### Changed
- Increase download percentage contrast on progress bars at 55% and above, adapting the text treatment to light and dark themes.

## [1.13.0]

### Added
- Show when each download was added and allow sorting downloads by name, added time, size, connections, status, time left, and speed.
- Hide temporary `.downloading` filenames in the download list and display the final file name instead.

### Fixed
- Preserve stable download group controls while status updates arrive, preventing intermittent missed clicks.

## [1.12.0]

### Added
- Package the Windows Native Messaging host with MSI, NSIS, and portable builds, then install and register it automatically for detected Chrome and Edge extensions when Irani IDM starts.

## [1.11.1]

### Added
- Add a browser toolbar toggle for enabling or disabling download handoff.
- Add Windows Native Messaging support for Chrome and Edge to transfer downloads without opening custom-protocol tabs.

### Fixed
- Prevent simultaneous first-run download events from opening multiple extension Options tabs.
- Avoid replaying restored Chrome downloads by ignoring existing downloads during browser startup.

## [1.11.0]

### Fixed
- Make a macOS tray icon left click restore the main window, matching Windows behavior; the tray menu remains available from the context click.

## [1.10.0]

### Added
- Add native macOS support for shutting down or sleeping after downloads finish.

### Fixed
- Route macOS completion actions through `System Events` and `pmset` while preserving existing Windows and Linux behavior.

## [1.9.0]

### Added
- Add a system tray menu with download controls and active-download status.
- Add animated and high-contrast progress feedback for active downloads.
- Add `.downloading` temporary filenames with automatic rename after successful completion.

### Changed
- Keep application windows open in the tray when closed instead of exiting.
- Increase the Firefox minimum version requirement to Firefox 142.
- Improve download finalization so completed files cannot overwrite existing files.

### Fixed
- Restore legacy incomplete downloads safely under `.downloading` temporary filenames.

## [1.8.0]

### Added
- Open the browser extension consent page once when a download is started without an explicit handoff decision.
- Remember and respect explicit browser download-handoff opt-out choices.
- Distinguish Firefox permission revocation from handoff disabled in Options.

## [1.7.1]

### Fixed
- Run configured shutdown and sleep actions after downloads finish on Linux.

## [1.7.0]

### Added
- Add step-by-step English and Persian installation guides for Chrome, Edge, and Firefox.
- Add Firefox support for handing browser downloads to Irani IDM.
- Add browser setup buttons for Firefox, Edge, and Chrome in application settings.
- Prepare store submission packages and document extension privacy and store listing details.
- Add an explicit first-run consent screen and opt-in control before transferring download URLs to Irani IDM.
- Add Firefox's built-in optional data-collection consent, revocation handling, and decline-to-uninstall route.
- Prepare separate Chrome, Edge, and Firefox listing copy, reviewer instructions, and store artwork.
- Keep the desktop app's bundled browser-install flow in sync with the new Options page assets.

### Changed
- Open the selected browser's installation guide in the app's current language instead of copying extension files and showing a short installation alert.
- Avoid writing full download URLs, which may contain private tokens, to the browser console.
- Remove broad web-page content scripts; browser integration now uses the downloads and storage permissions only.
- Disclose local download-history persistence and the desktop app's server-IP geolocation request in the browser privacy policy.

## [1.6.0]

### Added
- Capture browser downloads through the companion extension and start them in Irani IDM using the registered `iranidm://` link handler.
- Preserve the browser's external-protocol handoff tab and launch explicit download links directly from the user's click.

## [1.5.0]

### Added
- Configure the app to shut down or sleep after all active downloads finish.
- Adjust window transparency from the appearance settings.

### Fixed
- Prevent duplicate IP geolocation lookups while a request is pending and allow retries after failures.

## [1.4.0]

### Added
- Remember whether each download's connection-parts section is expanded.

## [1.3.3]

### Added
- Preserve the main window's state between launches.

### Changed
- Improve the main window layout and download table sizing.

## [1.3.2]

### Added
- Show the download server's country and flag in download details.
- Show whether a download supports resuming.

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

