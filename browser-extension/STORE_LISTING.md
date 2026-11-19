# Browser extension store submission details

Store-specific copy and reviewer instructions are maintained in:

- [`STORE_LISTING_CHROME.md`](STORE_LISTING_CHROME.md)
- [`STORE_LISTING_EDGE.md`](STORE_LISTING_EDGE.md)
- [`STORE_LISTING_FIREFOX.md`](STORE_LISTING_FIREFOX.md)
- [`FIREFOX_POLICY_INQUIRY.md`](FIREFOX_POLICY_INQUIRY.md) — draft question for Mozilla; not sent.

The publisher accounts have not yet been enrolled and no store submission has
been attempted. The Chrome Web Store registration fee is currently a blocker
for the publisher; do not attempt alternate payment routes or submit through
another person's account.

## Listing

**Name:** Irani IDM — Browser Integration

**Short description:** Send browser downloads directly to the Irani IDM desktop app.

**Category:** Productivity (Chrome / Edge); Download Management (Firefox).

**Full description:**

Send your browser downloads to Irani IDM, the multi-connection download manager.

When you start an HTTP or HTTPS download, this extension hands its URL to the
Irani IDM desktop application. The desktop app downloads the file using its
multi-connection engine and manages pause, resume, speed, and progress.

**How it works**

1. Install and open the Irani IDM desktop application.
2. Install this browser extension and review its first-run data-transfer notice.
3. Explicitly enable download handoff in the Options page.
4. Start a normal HTTP or HTTPS download in your browser. The extension hands
   the download URL to Irani IDM.
5. Confirm the browser's prompt to open Irani IDM, if shown.

**Important:** Irani IDM must be installed and its `iranidm://` protocol handler
must be registered. Downloads that require browser-only cookies, authentication
state, or custom request headers might not work in the desktop app.

**Privacy:** The extension transfers the full download URL to the Irani IDM
desktop app on this device. URLs may contain private query parameters or
temporary access tokens. The desktop app stores download URLs in its local
download history and requests the file from its host (or the proxy configured
by the user). The app may send the download server's IP address to `ipwho.is`
to display its country. The extension does not send URLs to the publisher or
an analytics service. See the full privacy policy before installing.

## Privacy and permission disclosures

**Single purpose:** With the user's explicit opt-in, transfer browser-initiated
HTTP/HTTPS download URLs to the locally installed Irani IDM desktop application.

**`downloads` permission justification:** Required to detect new browser
downloads, cancel the browser's transfer, and pass the original URL to Irani
IDM.

**`storage` permission justification:** Required only to store the user's
handoff preference and consent-version marker locally in the extension.

**Firefox data-category justification:** `browsingActivity` is required to
identify the complete URL of the download the user has opted to hand off;
`websiteActivity` describes the user's download action. Both are optional and
are requested only after the user enables handoff. If Firefox denies or the
user later revokes either category, the extension disables handoff.

**Data handling:** The extension processes the full HTTP/HTTPS download URL
(including query parameters) and transfers it to the Irani IDM desktop
application on the same device through the registered OS protocol handler. The
extension does not transmit the URL to the publisher or an analytics service
and does not persist it. The desktop app stores the URL in its local
`downloads.json` history and requests the file from the URL's host, or through
the user's configured proxy. The desktop app may send the file server's IP
address to `ipwho.is` for country lookup; it does not send the full URL to that
service. If the user manually tests a proxy, the desktop app contacts
`api.ipify.org`; if the user manually checks for updates, it contacts the
GitHub Releases API. Neither action includes the download URL. The extension
does not inspect page contents, cookies, passwords, or
unrelated browsing history. This transfer is disabled by default and requires
the user to accept the first-run disclosure and enable it in the Options page.
Firefox additionally requests the optional `browsingActivity` and
`websiteActivity` data permissions. Users can turn handoff off at any time.

**Privacy policy URL (after this file is pushed to the public repository):**
https://github.com/amirpiri/irani_idm/blob/main/browser-extension/PRIVACY.md

**Support website:** https://github.com/amirpiri/irani_idm/issues

**Support contact:** GitHub Issues is the public support channel. No separate
support email address has been provided, so none is invented here.

## Reviewer notes

This is a companion extension; it requires the Irani IDM desktop application
to be installed and the `iranidm://` protocol handler to be registered. After
installation, an Options page opens and handoff is disabled by default. The
user must explicitly opt in. To verify it, install and launch Irani IDM, install
the extension, accept the data-transfer disclosure, enable handoff, then
initiate an HTTP/HTTPS file download from a test page. Confirm the external-
protocol prompt and verify that the desktop app receives the URL. Disable
handoff and verify that browser downloads remain in the browser. A URL requiring
browser session cookies or browser-only headers is not a valid test case.

## Publisher-owned information still required

- Publisher/developer display name and the account contact details required by
  each store. GitHub Issues is prepared as the public support URL.
- Push the privacy policy to the public repository before entering its URL in a
  store dashboard. The repository remote is reachable, but this local policy
  file has not yet been published to GitHub.
- The license to select in the Firefox Add-ons listing (the repository does
  not currently contain a `LICENSE` file; the owner must choose the applicable
  rights/license rather than having one assumed. If no license is intended,
  select **All Rights Reserved** only after confirming that matches the owner's
  intent.
- Publisher account enrollment and any identity/verification steps. Chrome's
  one-time $5 developer registration fee and Iran's absence from Google's
  supported registration regions are blockers; no alternate payment or
  third-party account route will be attempted. Edge enrollment has no fee but
  requires a personal Microsoft Account (MSA), a country/region accepted by
  Partner Center, and any requested verification. AMO enrollment and review
  remain pending.

## Firefox policy assessment — reviewer confirmation required

The implementation now declares Firefox's optional `browsingActivity` and
`websiteActivity` data categories, requests them only from the explicit Options
page save action, fails closed if they are absent/revoked, and provides a
decline-to-uninstall path using Firefox's native `management.uninstallSelf()`
API (without requesting the broad `management` permission). The add-on is
restricted to Firefox 142+ so the built-in data-consent API is supported on
both desktop and Android.

There is a separate upcoming eligibility question that code changes cannot
conclusively resolve: Mozilla's published policy preview is scheduled to take
effect on December 1, 2026 and explicitly prohibits add-ons whose sole purpose
is promoting, installing, loading, or launching an outside application. This
extension is a browser-download integration whose handoff target is the Irani
IDM desktop application. AMO reviewers must determine whether this qualifies as
a download-manager integration or is treated as a prohibited external-app
launcher. The same preview also prohibits leaking local or user-specific
information to other applications; a download URL can contain user-specific
tokens. Because handing the URL to the desktop app is the core function, there
is no code-only change that can guarantee compliance while preserving that
behavior. The preview applies to both listed and self-distributed add-ons, so
self-hosting an XPI is not a policy workaround. Do not claim guaranteed AMO
acceptance. Ask Mozilla for a policy determination before submitting and
provide the draft in `FIREFOX_POLICY_INQUIRY.md`.

Official policy references:

- https://extensionworkshop.com/documentation/publish/add-on-policies/
- https://extensionworkshop.com/documentation/publish/add-on-policies-preview-2026-12/
- https://extensionworkshop.com/documentation/develop/firefox-builtin-data-consent/

## Generated submission files

Run these from PowerShell in the repository root:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File browser-extension/build-store-packages.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File browser-extension/build-store-assets.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File browser-extension/capture-store-screenshot.ps1
```

The first command creates `browser-extension/store-packages/` containing
`Irani-IDM-Chrome.zip`, `Irani-IDM-Edge.zip`, and `Irani-IDM-Firefox.zip`.
The second creates the 440x280 small promo tile, the 300x300 Edge logo, and the
optional 1400x560 marquee graphic in `browser-extension/store-assets/`. The
third captures the actual Options-page UI at 1280x800 in
`browser-extension/store-assets/screenshots/`. The generated archives contain
only the runtime manifest, scripts, and icons.

The promotional tiles are store graphics, not product screenshots. The
1280x800 screenshot shows the real Options-page HTML with handoff disabled by
default. Do not submit a mockup or a desktop-app screenshot as if it were an
extension screenshot.
