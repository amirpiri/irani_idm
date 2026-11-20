# Chrome Web Store listing

## Listing fields

**Name:** Irani IDM Browser Integration

**Category:** Productivity

**Language:** English (United States)

**Short description (manifest `description`, maximum 132 characters):** Send browser downloads to Irani IDM.

**Description:**

Send browser downloads to Irani IDM, the multi-connection download manager.

When you start an HTTP or HTTPS download, this extension can hand its complete
URL to the Irani IDM desktop application on this computer. Irani IDM downloads
the file and manages its connections, progress, pause, and resume.

**You stay in control**

- Download handoff is off by default.
- A first-run page explains what information is transferred and where it goes.
- Enable or disable handoff at any time from the extension's Options page.
- If handoff is off, the browser handles downloads normally.

**Requirements and limitations**

- Irani IDM for Windows must be installed and launched. It installs the Native Messaging host automatically; restart Irani IDM after installing the extension so it can register the extension ID.
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
that service. If the user manually tests a proxy, the desktop app contacts
`api.ipify.org`; if the user manually checks for updates, it contacts the GitHub
Releases API. Neither request includes the download URL. Read the privacy policy
for details.

## Privacy dashboard

**Single purpose:** With the user's explicit opt-in, transfer browser-initiated
HTTP/HTTPS download URLs to the Irani IDM desktop application on the same device.

**Permission — `downloads`:** Detect new downloads, cancel the browser transfer,
and hand the original URL to Irani IDM.

**Permission — `storage`:** Store the user's handoff preference and consent
version locally in the extension.

**Permission — `nativeMessaging`:** Send an explicitly enabled download URL to
the locally installed Irani IDM Native Messaging host. The host forwards the
URL to the desktop application over a local named pipe.

**User data disclosure:** The extension handles website URLs associated with
downloads (browsing activity). A URL may contain personal or confidential query
parameters. The extension does not collect page contents, cookies, passwords,
search terms, or unrelated browsing history. It does not use analytics or send
download URLs to the publisher. The desktop application stores full URLs in
its local `downloads.json` history, then requests the file from the URL host or
the user's configured proxy. The desktop application may send the download
server IP (not the URL) to `ipwho.is` for geolocation. If the user manually
tests a proxy, the desktop app contacts `api.ipify.org`; if the user manually
checks for updates, it contacts the GitHub Releases API. Neither request
includes the download URL.

**Privacy policy URL:**
https://github.com/amirpiri/irani_idm/blob/main/browser-extension/PRIVACY.md

**Website:** https://github.com/amirpiri/irani_idm

**Support URL:** https://github.com/amirpiri/irani_idm/issues

## Graphic assets

- Store icon: `icons/128.png` (inside ZIP)
- Required small promo tile: `store-assets/store-promo-440x280.png`
- Optional marquee tile: `store-assets/store-marquee-1400x560.png`
- Required screenshot: `store-assets/screenshots/consent-first-run-1280x800.png`

Screenshot caption: “Review the download URL handoff disclosure and keep
automatic handoff disabled until you explicitly enable it.”

## Reviewer notes

Install and launch Irani IDM first. After installing this extension, restart Irani IDM to register it with the automatically installed Native Messaging host.
Install the extension, open its Options page, enable handoff, and initiate a
test HTTP/HTTPS download. Verify that Irani IDM receives the URL without an
external-protocol tab or prompt. Disable handoff and verify that the next
browser download remains in the browser. The extension contains no remotely
hosted executable code. Do not use a test URL requiring browser-only cookies or
custom headers.
