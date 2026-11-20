# Install the Irani IDM extension in Google Chrome

These instructions install the extension from the official Irani IDM GitHub release. Irani IDM for Windows must be installed and launched at least once before browser downloads can be handed to it.

## 1. Download the extension

1. Open the [Irani IDM releases page](https://github.com/amirpiri/irani_idm/releases/latest).
2. Expand **Assets** for the latest release.
3. Download `Irani-IDM-Chrome.zip`.

## 2. Extract the ZIP file

1. Open your Downloads folder.
2. Right-click `Irani-IDM-Chrome.zip` and choose **Extract All…**.
3. Keep the extracted `Irani-IDM-Chrome` folder somewhere permanent. Chrome loads the extension directly from this folder, so do not delete or move it after installation.

## 3. Load the extension in Chrome

1. Open Chrome and enter `chrome://extensions` in the address bar.
2. Turn on **Developer mode** using the switch near the top-right corner.
3. Click **Load unpacked**.
4. Select the extracted extension folder — the folder containing `manifest.json` — and confirm.
5. Check that **Irani IDM Browser Integration** appears in your extensions list.

## 4. Register the Native Messaging host

1. Open the Irani IDM project folder in File Explorer or PowerShell.
2. From the project folder, run `powershell -ExecutionPolicy Bypass -File .\browser-extension\register-native-host.ps1`. This builds and registers the local Native Messaging host for the Chrome/Edge profiles where this unpacked extension is loaded. It writes the registration under your Windows user account; administrator access is not required.
3. Restart Irani IDM. Keep the extension loaded from the same folder so its extension ID remains unchanged.

## 5. Pin the extension (optional)

1. Click the puzzle-piece **Extensions** button in the Chrome toolbar.
2. Find **Irani IDM Browser Integration**.
3. Click its pin icon to keep it visible in the toolbar.

## 6. Enable download handoff

1. Chrome opens the extension's Options page after installation. Read the first-run notice explaining that download URLs are sent to the Irani IDM app on this computer.
2. Enable handoff only if you agree, then save your choice. Handoff is disabled until you explicitly enable it.
3. Make sure Irani IDM is running. Chrome and Edge send downloads directly to the local Native Messaging host; they do not open a custom-protocol tab.
4. Try a normal HTTP or HTTPS download. The download should appear in Irani IDM.

You can change the handoff setting at any time from `chrome://extensions` → **Irani IDM Browser Integration** → **Extension options**. Turning it off leaves downloads in Chrome.

## Troubleshooting

- **Load unpacked is missing:** Turn on Developer mode.
- **Chrome says the manifest is missing:** Select the extracted folder that directly contains `manifest.json`, not the ZIP file or its parent folder.
- **The extension disappears or stops working after moving files:** Load it again from its new permanent folder.
- **An update is available:** Download and extract the new ZIP, open `chrome://extensions`, and click the extension's reload button after replacing the old folder contents. If needed, remove the old extension and load the new folder.
- **Downloads stay in Chrome:** Check that handoff is enabled in Extension options and Irani IDM is running. Browser-only cookies, authentication, or custom headers may prevent some downloads from working in the desktop app.
- **Native Messaging host not found:** Run the registration command above again after loading the unpacked extension. If you moved or reloaded it from a different folder, run the script again so the registered extension ID matches.
- **The consent checkbox is unchecked or handoff was turned off:** Open `chrome://extensions` → **Irani IDM Browser Integration** → **Extension options**. Read the data-transfer notice, tick the consent checkbox, and click **Save preference**. If you start a download before making a choice, the extension opens this page once to remind you; it never enables handoff automatically. If you previously saved the checkbox as unchecked, open Options yourself to change that choice.
- **The checkbox or Options page seems missing:** Open `chrome://extensions`, make sure Irani IDM Browser Integration is enabled, click its reload icon, then open **Extension options**. If it is not listed, load the extracted extension folder again using **Load unpacked**.

The extension handles HTTP and HTTPS download URLs only. See the [privacy policy](https://github.com/amirpiri/irani_idm/blob/main/browser-extension/PRIVACY.md) for details.
