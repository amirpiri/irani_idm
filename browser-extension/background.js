const APP_SCHEME = "iranidm://download?url=";
const CONSENT_VERSION = 2;

chrome.downloads.onCreated.addListener((download) => {
  // A URL may contain personal information or temporary access tokens. Do
  // not inspect or transfer it until the user has explicitly enabled handoff.
  chrome.storage.local.get({ downloadHandoffEnabled: false }, ({ downloadHandoffEnabled }) => {
    if (!downloadHandoffEnabled || !download.url || !/^https?:\/\//i.test(download.url) || download.byExtensionId) return;

    const transferDownload = () => {
      // Do not write full download URLs to the browser console.
      console.info("Irani IDM detected a browser download and is handing it off.");

      // Stop the browser transfer before handing the original URL to the desktop app.
      chrome.downloads.cancel(download.id, () => {
        if (chrome.runtime.lastError) {
          console.warn("Irani IDM could not cancel the browser download:", chrome.runtime.lastError.message);
          return;
        }
        // Keep the tab alive until the OS launches Irani IDM via iranidm://.
        chrome.tabs.create({ url: APP_SCHEME + encodeURIComponent(download.url), active: true }, (tab) => {
          if (chrome.runtime.lastError) {
            console.error("Could not open Irani IDM:", chrome.runtime.lastError.message);
          } else {
            console.info("Sent browser download to Irani IDM.", tab?.id);
          }
        });
      });
    };

    // Firefox's built-in optional data permission is a second, revocable
    // consent gate. Never transfer URLs if it is absent or has been revoked.
    if (typeof browser !== "undefined" && browser.permissions?.getAll) {
      browser.permissions.getAll().then((permissions) => {
        const data = permissions.data_collection || [];
        if (data.includes("browsingActivity") && data.includes("websiteActivity")) {
          transferDownload();
        } else {
          chrome.storage.local.set({ downloadHandoffEnabled: false });
        }
      }).catch((error) => console.warn("Could not verify Firefox data permissions:", error));
      return;
    }

    transferDownload();
  });
});

if (typeof browser !== "undefined" && browser.permissions?.onRemoved) {
  browser.permissions.onRemoved.addListener((removed) => {
    const data = removed.data_collection || [];
    if (data.includes("browsingActivity") || data.includes("websiteActivity")) {
      chrome.storage.local.set({ downloadHandoffEnabled: false });
    }
  });
}

// Show the consent screen on installation and after updates that may change
// what information is handed to the desktop application.
chrome.runtime.onInstalled.addListener((details) => {
  if (details.reason === "install") {
    chrome.runtime.openOptionsPage();
    return;
  }
  if (details.reason !== "update") return;

  chrome.storage.local.get({ downloadHandoffEnabled: false, downloadHandoffConsentVersion: 0 }, (settings) => {
    if (settings.downloadHandoffConsentVersion >= CONSENT_VERSION) return;
    const resetConsent = () => chrome.storage.local.set({
      downloadHandoffEnabled: false,
      downloadHandoffConsentVersion: CONSENT_VERSION
    }, () => chrome.runtime.openOptionsPage());

    if (typeof browser !== "undefined" && browser.permissions?.remove) {
      browser.permissions.remove({ data_collection: ["browsingActivity", "websiteActivity"] })
        .catch(() => {})
        .finally(resetConsent);
    } else {
      resetConsent();
    }
  });
});
