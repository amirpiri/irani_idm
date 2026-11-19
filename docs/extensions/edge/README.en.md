# Install the Irani IDM extension in Microsoft Edge

These instructions install the extension from the official Irani IDM GitHub release. Install and launch Irani IDM for Windows at least once before using browser handoff.

## 1. Download the extension

1. Open the [Irani IDM releases page](https://github.com/amirpiri/irani_idm/releases/latest).
2. Expand **Assets** for the latest release.
3. Download `Irani-IDM-Edge.zip`.

## 2. Extract the ZIP file

1. Open your Downloads folder.
2. Right-click `Irani-IDM-Edge.zip` and choose **Extract All…**.
3. Keep the extracted `Irani-IDM-Edge` folder in a permanent location. Edge loads the extension from this folder; moving or deleting it can break the installation.

## 3. Load the extension in Edge

1. Open Edge and enter `edge://extensions` in the address bar.
2. Turn on **Developer mode** in the left-side panel or toolbar (the location depends on your Edge version).
3. Click **Load unpacked**.
4. Select the extracted folder that directly contains `manifest.json`.
5. Confirm that **Irani IDM Browser Integration** appears in the extensions list.

## 4. Pin the extension (optional)

1. Click the **Extensions** button in the Edge toolbar.
2. Find **Irani IDM Browser Integration**.
3. Use its visibility/pin control to show it on the toolbar.

## 5. Enable download handoff

1. After installation, Edge opens the extension's Options page. Read the first-run notice about sending download URLs to the Irani IDM app on this computer.
2. Enable handoff only if you agree, then save. Handoff starts disabled.
3. Keep Irani IDM running. Allow Edge to open Irani IDM if it asks about the `iranidm://` link.
4. Test with a regular HTTP or HTTPS download and confirm it appears in Irani IDM.

To change your choice later, open `edge://extensions` → **Irani IDM Browser Integration** → **Extension options**. Disabling handoff keeps downloads in Edge.

## Troubleshooting

- **Load unpacked is unavailable:** Enable Developer mode.
- **Manifest file not found:** Choose the extracted extension folder containing `manifest.json` directly.
- **The extension stops working after moving its folder:** Load it again from its new permanent location.
- **Updating:** Extract the new release, replace the old folder contents, then use the reload control on `edge://extensions`. You can also remove the old extension and load the new folder.
- **Downloads remain in Edge:** Verify that handoff is enabled and Irani IDM is running. Downloads that depend on browser-only cookies, authentication, or custom headers may not work in the desktop app.
- **The consent checkbox is unchecked or handoff was turned off:** Open `edge://extensions` → **Irani IDM Browser Integration** → **Extension options**. Read the data-transfer notice, tick the consent checkbox, and click **Save preference**. If you start a download before making a choice, the extension opens this page once to remind you; it never enables handoff automatically. If you previously saved the checkbox as unchecked, open Options yourself to change that choice.
- **The checkbox or Options page seems missing:** Open `edge://extensions`, make sure Irani IDM Browser Integration is enabled, click its reload control, then open **Extension options**. If it is not listed, load the extracted extension folder again using **Load unpacked**.

Only HTTP and HTTPS download URLs are supported. Read the [privacy policy](https://github.com/amirpiri/irani_idm/blob/main/browser-extension/PRIVACY.md).
