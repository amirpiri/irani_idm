# Install the Irani IDM extension in Mozilla Firefox

Install the signed add-on from [Mozilla Add-ons](https://addons.mozilla.org/en-US/firefox/addon/irani-idm-browser-integration/). Firefox 142 or later is required. The GitHub release ZIP remains available for temporary testing.

Irani IDM for Windows must be installed and launched at least once before browser handoff can work.

## Install from Mozilla Add-ons

1. Open the [Irani IDM listing](https://addons.mozilla.org/en-US/firefox/addon/irani-idm-browser-integration/).
2. Click **Add to Firefox** and confirm the installation.
3. Continue to **Enable download handoff** below and review the first-run consent.

## Temporarily load the GitHub release for testing

1. Open the [Irani IDM releases page](https://github.com/amirpiri/irani_idm/releases/latest).
2. Expand **Assets** and download `Irani-IDM-Firefox.zip`.
3. In your Downloads folder, right-click the ZIP and choose **Extract All…**.
4. Keep the extracted folder in a permanent location. You will select its `manifest.json` file in the next step.

### Load the add-on temporarily

1. Open Firefox 142 or later.
2. Enter `about:debugging#/runtime/this-firefox` in the address bar.
3. Click **Load Temporary Add-on…**.
4. Open the extracted extension folder and select `manifest.json`.
5. Confirm that **Irani IDM Browser Integration** appears in the temporary extensions list.

The temporary add-on is removed when Firefox closes. Repeat this step after restarting Firefox.

## Enable download handoff

1. Firefox opens the extension's Options page. Read the first-run notice about sending download URLs to Irani IDM on this computer.
2. Enable handoff only if you agree. Firefox will ask for its optional data permissions for download URLs and download actions. Grant both only if you want to use handoff; if either is denied or revoked, handoff remains disabled.
3. Save your choice and keep Irani IDM running.
4. If Firefox asks permission to open Irani IDM for an `iranidm://` link, allow it.
5. Test with a regular HTTP or HTTPS download and confirm it appears in Irani IDM.

You can change the handoff choice from the add-on's **Options** page or Firefox's Add-ons Manager (`about:addons`). Disabling handoff also revokes the optional Firefox data permissions.

## Permanent installation

The ZIP attached to a GitHub release is unsigned and cannot be installed as a normal permanent Firefox add-on. Use the signed version from [Mozilla Add-ons](https://addons.mozilla.org/en-US/firefox/addon/irani-idm-browser-integration/) for permanent installation.

## Troubleshooting

- **Load Temporary Add-on is missing:** Open `about:debugging#/runtime/this-firefox` in Firefox 142 or later.
- **Firefox cannot find the manifest:** Extract the ZIP first, then select `manifest.json` inside the extracted folder.
- **The extension disappears after restarting Firefox:** This is expected for a temporary add-on. Load it again, or install the signed AMO version.
- **Handoff cannot be enabled:** Open the extension's Options page and grant both optional Firefox data permissions. If either permission is revoked, enable it again and accept the prompt.
- **Downloads remain in Firefox:** Check that handoff is enabled and Irani IDM is running. Browser-only cookies, authentication, or custom headers may prevent some downloads from working in the desktop app.
- **The consent checkbox is unchecked or handoff was turned off:** Open `about:addons` → **Irani IDM Browser Integration** → **Options**. Read the data-transfer notice, tick the consent checkbox, click **Save preference**, and grant both optional Firefox data permissions when asked. If you start a download before making a choice, the extension opens Options once to remind you; it never enables handoff automatically. If you previously saved the checkbox as unchecked, open Options yourself to change that choice.
- **The checkbox or Options page seems missing:** Check that the add-on is present and enabled in `about:addons`. For a temporary installation, load it again from `about:debugging#/runtime/this-firefox` after restarting Firefox. Then reopen the add-on's Options page.

Only HTTP and HTTPS download URLs are supported. See the [privacy policy](https://github.com/amirpiri/irani_idm/blob/main/browser-extension/PRIVACY.md).
