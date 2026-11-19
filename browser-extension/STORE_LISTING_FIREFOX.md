# Firefox Add-ons (AMO) listing

## Listing fields

**Name:** Irani IDM Download Integration

**Summary (250 characters maximum):** Send HTTP/HTTPS browser downloads to the
Irani IDM desktop download manager. Handoff is disabled by default and only
starts after you explicitly enable it.

**Category:** Download Management

**Description:**

Send browser downloads to Irani IDM, the multi-connection download manager.

When you start an HTTP or HTTPS download, this extension can hand its complete
URL to the Irani IDM desktop application on this computer. Irani IDM downloads
the file and manages its connections, progress, pause, and resume.

**You stay in control**

- Download handoff is off by default.
- Firefox's built-in data-collection permission prompt is shown when you enable
  handoff. The extension requests only the download URL and download-action
  categories needed for this feature.
- Enable or disable handoff at any time from the extension's Options page or
  Firefox's Add-ons permissions controls.
- If you decline the required data transfer, the Options page takes you to
  Firefox's native uninstall confirmation so you can remove the extension.

**Requirements and limitations**

- Firefox 142 or later is required for built-in data-collection consent on
  desktop and Android.
- Irani IDM must be installed and its `iranidm://` protocol handler registered.
- The extension handles HTTP and HTTPS downloads.
- URLs can include private query parameters or temporary access tokens. The
  desktop app stores download URLs in its local download history.
- Downloads that require browser-only cookies, authentication state, or custom
  request headers might not work in the desktop app.

**Privacy:** The extension transfers the full download URL to the local Irani
IDM desktop app. It does not send URLs to the publisher or use analytics. The
desktop app stores URLs locally and requests files from their hosts (or the
proxy configured by the user). For server-country display, the app may send the
download server's IP address to `ipwho.is`; it does not send the full URL to
that service. If the user manually tests a proxy, the app contacts
`api.ipify.org`; if the user manually checks for updates, it contacts the GitHub
Releases API. Neither request includes the download URL. Read the privacy policy
for details.

## Firefox data collection declaration

The manifest declares:

```json
"data_collection_permissions": {
  "required": ["none"],
  "optional": ["browsingActivity", "websiteActivity"]
}
```

- `browsingActivity`: the complete HTTP/HTTPS download URL, which may include
  sensitive query parameters.
- `websiteActivity`: the user's action of starting a download.
- Both categories are optional and are requested only after the user actively
  enables handoff from the Options page.
- The extension fails closed if either category is missing or revoked.
- If the user declines, the Options page offers Firefox's native
  `management.uninstallSelf()` action; this API does not require the broad
  `management` permission.
- The extension does not transmit technical/interaction telemetry.

## Permissions and data handling

**`downloads`:** Detect new browser downloads, cancel the browser transfer,
and pass the original URL to Irani IDM.

**`storage`:** Store the user's handoff preference and consent version locally.

The extension does not inspect page contents, cookies, passwords, search terms,
or unrelated browsing history. It does not use analytics or send download URLs
to the publisher. The desktop application stores full URLs in its local
`downloads.json` history, then requests the file from the URL host or the user's
configured proxy. The desktop application may send the download server IP (not
the URL) to `ipwho.is` for geolocation. If the user manually tests a proxy, the
app contacts `api.ipify.org`; if the user manually checks for updates, it
contacts the GitHub Releases API. Neither request includes the download URL.

**Privacy policy URL:**
https://github.com/amirpiri/irani_idm/blob/main/browser-extension/PRIVACY.md

**Support website:** https://github.com/amirpiri/irani_idm/issues

## Screenshots

`store-assets/screenshots/consent-first-run-1280x800.png`

Caption: “Review the download URL handoff disclosure before enabling the
integration. Handoff starts disabled.”

## Reviewer notes and test steps

This is a browser download-manager integration, not a generic app launcher. It
intercepts eligible HTTP/HTTPS downloads only after the user enables handoff
and passes the download URL to the Irani IDM desktop application installed on
the same device. Install and launch Irani IDM first so Windows registers
`iranidm://`. Install the add-on, review the first-run page, enable handoff and
accept Firefox's optional data-collection prompt. Start a test HTTP/HTTPS
download and verify that Irani IDM receives the URL. Disable handoff and verify
that subsequent downloads stay in Firefox. A URL requiring browser-only cookies
or custom headers is not a valid test case.

## Policy eligibility note — obtain Mozilla confirmation before submission

The current AMO submission requirements include Firefox's built-in data
collection declaration and consent system for new add-ons. This package
declares the URL and download-action categories as optional, requests them only
after the user enables handoff, and disables handoff if either is revoked.

There is also an upcoming policy risk: Mozilla's published policy preview is
scheduled to take effect on **December 1, 2026** and explicitly prohibits
add-ons whose sole purpose is promoting, installing, loading, or launching an
outside application. This extension is specifically a browser-download
integration with a desktop download manager, but its handoff target is an
external application. The same preview prohibits leaking local or user-specific
information to other applications. A download URL can contain account tokens,
and passing it to Irani IDM is the integration's core function. No code-only
change can guarantee compliance while preserving this behavior. The preview
applies to both listed and self-distributed add-ons, so self-hosting the XPI is
not a policy workaround. Ask AMO reviewers to confirm eligibility before
submission; do not represent acceptance as guaranteed.

Official guidance:

- https://extensionworkshop.com/documentation/publish/add-on-policies/
- https://extensionworkshop.com/documentation/publish/add-on-policies-preview-2026-12/
- https://extensionworkshop.com/documentation/develop/firefox-builtin-data-consent/
- https://extensionworkshop.com/documentation/publish/submitting-an-add-on/
