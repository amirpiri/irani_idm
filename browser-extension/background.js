const APP_SCHEME = "iranidm://download?url=";
const CONSENT_VERSION = 2;
// Chromium may expose the WebExtensions `browser` compatibility namespace.
// Only Firefox supports the data_collection permission fields used below.
const isFirefox = typeof navigator !== "undefined" && /Firefox\//.test(navigator.userAgent);

chrome.downloads.onCreated.addListener((download) => {
  // A URL may contain personal information or temporary access tokens. Do
  // not transfer it until the user has explicitly enabled handoff.
  chrome.storage.local.get({
    downloadHandoffEnabled: false,
    downloadHandoffConsentVersion: 0,
    downloadHandoffDecisionMade: null,
    downloadHandoffPrompted: false,
  }, (settings) => {
    if (!download.url || !/^https?:\/\//i.test(download.url) || download.byExtensionId) return;

    const decisionMade = settings.downloadHandoffDecisionMade === true
      // Older releases did not distinguish an explicit opt-out from a
      // preference disabled by a runtime/permission error. Treat a legacy
      // disabled state as undecided once so affected users can recover.
      || (settings.downloadHandoffDecisionMade === null
        && settings.downloadHandoffConsentVersion >= CONSENT_VERSION
        && settings.downloadHandoffEnabled === true);

    const promptForConsent = () => {
      if (decisionMade || settings.downloadHandoffPrompted) return;
      chrome.storage.local.set({
        downloadHandoffPrompted: true,
        downloadHandoffPromptReason: "download-attempt",
      }, () => chrome.runtime.openOptionsPage());
    };

    if (!settings.downloadHandoffEnabled) {
      promptForConsent();
      return;
    }

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
    if (isFirefox) {
      browser.permissions.getAll().then((permissions) => {
        const data = permissions.data_collection || [];
        if (data.includes("browsingActivity") && data.includes("websiteActivity")) {
          transferDownload();
        } else {
          chrome.storage.local.set({
            downloadHandoffEnabled: false,
            downloadHandoffDecisionMade: false,
            downloadHandoffPrompted: false,
            downloadHandoffPromptReason: "permission-missing",
          }, () => chrome.runtime.openOptionsPage());
        }
      }).catch((error) => console.warn("Could not verify Firefox data permissions:", error));
      return;
    }

    transferDownload();
  });
});

if (isFirefox && browser.permissions?.onRemoved) {
  browser.permissions.onRemoved.addListener((removed) => {
    const data = removed.data_collection || [];
    if (data.includes("browsingActivity") || data.includes("websiteActivity")) {
      // Only treat this as an external revocation if handoff was still on.
      // Disabling handoff from Options also removes these permissions, and
      // that explicit opt-out must not be overwritten by this event.
      chrome.storage.local.get({ downloadHandoffEnabled: false }, ({ downloadHandoffEnabled }) => {
        if (!downloadHandoffEnabled) return;
        chrome.storage.local.set({
          downloadHandoffEnabled: false,
          downloadHandoffDecisionMade: false,
          downloadHandoffPrompted: false,
          downloadHandoffPromptReason: "permission-missing",
        });
      });
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
      downloadHandoffConsentVersion: CONSENT_VERSION,
      downloadHandoffDecisionMade: false,
      downloadHandoffPrompted: false,
      downloadHandoffPromptReason: null,
    }, () => chrome.runtime.openOptionsPage());

    if (isFirefox) {
      browser.permissions.remove({ data_collection: ["browsingActivity", "websiteActivity"] })
        .catch(() => {})
        .finally(resetConsent);
    } else {
      resetConsent();
    }
  });
});
