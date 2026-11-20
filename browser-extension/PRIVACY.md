# Irani IDM Browser Integration — Privacy Policy

**Last updated: 2026-11-20**

Irani IDM Browser Integration hands browser download URLs to the Irani IDM
desktop application on the same device. The extension does not operate an
account system, use analytics, serve advertisements, or send download URLs to
the extension publisher.

## Information the extension handles

When the user explicitly enables download handoff, the extension handles the
full URL of each new HTTP or HTTPS browser download. This includes the hostname,
path, and query string. A URL can contain personal or confidential information,
including account identifiers, query parameters, or temporary access tokens,
depending on the website. The extension does not read page contents, cookies,
passwords, or unrelated browsing history.

## How the information is used

After the user enables handoff in the extension's first-run Options page,
Chrome and Edge pass the original URL to the Irani IDM desktop application
through a Native Messaging host and a local named pipe. The browser download is
cancelled only after the desktop application confirms that it accepted the URL.
No custom-protocol browser tab is opened. Firefox continues to use the operating
system's local `iranidm://` protocol handler. If the user declines or disables
handoff, browser downloads continue normally and their URLs are not handed to
Irani IDM.

The extension itself does not send URLs to the Irani IDM publisher, an
analytics provider, or another extension-operated remote service. It does not
persist download URLs. The desktop application does store download records,
including the full URL, in its local `downloads.json` history file in the
application's data directory so downloads can be restored after a restart.
That file remains on the user's device unless the user moves, backs up, or
otherwise shares the application data directory.

The desktop application requests the file from the host in the supplied URL.
If the user has configured an HTTP or SOCKS5 proxy in Irani IDM, that proxy also
handles the download request and can process the URL. The app may additionally
send the download server's IP address (not the full download URL) to the
third-party `ipwho.is` geolocation service to display the server's country.
These network requests are made by the desktop application, not by the browser
extension. The file host and any configured proxy receive the information
needed to make the download request; `ipwho.is` receives the server IP for
geolocation and the source IP of the app's request.

Two other desktop-app actions are separate from extension handoff: when the
user selects **Test Proxy**, the app contacts `api.ipify.org` to obtain the
public IP seen through that proxy; when the user selects **Check for updates**,
the app contacts the GitHub Releases API. Those services receive normal network
request metadata (including the request's source IP); neither request includes
the browser download URL.

## Permissions

- **downloads:** detect new browser downloads and cancel the browser transfer
  only after Irani IDM confirms that it accepted the URL.
- **storage:** remember whether the user has enabled download handoff and the
  version of the consent they accepted.
- **nativeMessaging (Chrome and Edge):** send the full URL of an explicitly
  enabled download to the locally installed Irani IDM Native Messaging host.
  The host forwards it to the desktop application over a local named pipe; it
  does not send the URL to a remote service.
- **Firefox data collection:** `browsingActivity` (the full download URL) and
  `websiteActivity` (the user's action of starting a download) are optional
  data categories. Firefox asks for these permissions when the user enables
  handoff. They are removed when the user disables handoff.

## User choice and control

The extension opens its Options page after installation and starts with handoff
disabled. Users must explicitly enable it before any download URL is transferred.
In Firefox, the browser also asks for the optional data-collection permissions;
handoff remains disabled if those permissions are denied or later revoked.
Users can disable handoff at any time from the toolbar popup or Options page, or uninstall the
extension. If the user declines the required Firefox data transfer, the Options
page offers a one-click uninstall action with Firefox's confirmation dialog.
Because download URLs may contain access tokens or other sensitive
query parameters, users should only enable handoff if they intend to send their
browser downloads to Irani IDM.

## Changes and contact

This policy will be updated if the extension's data handling changes. For
questions or privacy concerns, use the project's issue tracker:
https://github.com/amirpiri/irani_idm/issues
