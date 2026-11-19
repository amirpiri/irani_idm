# Install the Irani IDM extension in Mozilla Firefox

The GitHub release provides an unsigned ZIP for manual testing. Firefox's **Load Temporary Add-on** method is temporary: Firefox removes the add-on when it closes. A permanent public installation requires a signed release through Mozilla Add-ons (AMO), once that listing is published.

Irani IDM for Windows must be installed and launched at least once before browser handoff can work.

## 1. Download and extract the extension

1. Open the [Irani IDM releases page](https://github.com/amirpiri/irani_idm/releases/latest).
2. Expand **Assets** and download `Irani-IDM-Firefox.zip`.
3. In your Downloads folder, right-click the ZIP and choose **Extract All…**.
4. Keep the extracted folder in a permanent location. You will select its `manifest.json` file in the next step.

## 2. Load the add-on temporarily

1. Open Firefox 140 or later.
2. Enter `about:debugging#/runtime/this-firefox` in the address bar.
3. Click **Load Temporary Add-on…**.
4. Open the extracted extension folder and select `manifest.json`.
5. Confirm that **Irani IDM Browser Integration** appears in the temporary extensions list.

The temporary add-on is removed when Firefox closes. Repeat this step after restarting Firefox.

## 3. Enable download handoff

1. Firefox opens the extension's Options page. Read the first-run notice about sending download URLs to Irani IDM on this computer.
2. Enable handoff only if you agree. Firefox will ask for its optional data permissions for download URLs and download actions. Grant both only if you want to use handoff; if either is denied or revoked, handoff remains disabled.
3. Save your choice and keep Irani IDM running.
4. If Firefox asks permission to open Irani IDM for an `iranidm://` link, allow it.
5. Test with a regular HTTP or HTTPS download and confirm it appears in Irani IDM.

You can change the handoff choice from the add-on's **Options** page or Firefox's Add-ons Manager (`about:addons`). Disabling handoff also revokes the optional Firefox data permissions.

## 4. Install permanently

The ZIP attached to a GitHub release is not signed and cannot be installed as a normal permanent Firefox add-on. For permanent installation, use the Irani IDM listing on [Mozilla Add-ons](https://addons.mozilla.org/) after it has been published and approved. The AMO listing link will be added to the project when available.

## Troubleshooting

- **Load Temporary Add-on is missing:** Open `about:debugging#/runtime/this-firefox` in Firefox 140 or later.
- **Firefox cannot find the manifest:** Extract the ZIP first, then select `manifest.json` inside the extracted folder.
- **The extension disappears after restarting Firefox:** This is expected for a temporary add-on. Load it again, or install the signed AMO version when published.
- **Handoff cannot be enabled:** Open the extension's Options page and grant both optional Firefox data permissions. If either permission is revoked, enable it again and accept the prompt.
- **Downloads remain in Firefox:** Check that handoff is enabled and Irani IDM is running. Browser-only cookies, authentication, or custom headers may prevent some downloads from working in the desktop app.

Only HTTP and HTTPS download URLs are supported. See the [privacy policy](https://github.com/amirpiri/irani_idm/blob/main/browser-extension/PRIVACY.md).
