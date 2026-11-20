const APP_SCHEME = "iranidm://download?url=";
const NATIVE_HOST = "com.iranidm.browser";
const CONSENT_VERSION = 2;
let consentPromptRequested = false;
// Chromium may expose the WebExtensions `browser` compatibility namespace.
// Only Firefox supports the data_collection permission fields used below.
const isFirefox = typeof navigator !== "undefined" && /Firefox\//.test(navigator.userAgent);

function handleCreatedDownload(download) {
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
      if (decisionMade || settings.downloadHandoffPrompted || consentPromptRequested) return;
      consentPromptRequested = true;
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

      if (!isFirefox) {
        // Native Messaging sends the URL directly to the desktop app. It does
        // not navigate a tab, so Chrome cannot restore or replay a handoff.
        chrome.runtime.sendNativeMessage(NATIVE_HOST, { url: download.url }, (response) => {
          if (chrome.runtime.lastError || response?.ok !== true) {
            console.warn("Irani IDM could not accept the browser download:", chrome.runtime.lastError?.message || response?.error || "Native host unavailable");
            return;
          }
          chrome.downloads.cancel(download.id, () => {
            if (chrome.runtime.lastError) {
              console.warn("Irani IDM accepted the download, but Chrome could not cancel its copy:", chrome.runtime.lastError.message);
            }
          });
        });
        return;
      }

      // Firefox continues to use the registered protocol handler.
      chrome.downloads.cancel(download.id, () => {
        if (chrome.runtime.lastError) {
          console.warn("Irani IDM could not cancel the browser download:", chrome.runtime.lastError.message);
          return;
        }
        chrome.tabs.create({ url: APP_SCHEME + encodeURIComponent(download.url), active: true });
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
}

if (isFirefox) {
  chrome.downloads.onCreated.addListener(handleCreatedDownload);
} else {
  // Chrome can emit onCreated again for download records it restores when the
  // browser starts. Snapshot every existing download once per browser session
  // and keep the IDs in storage.session (which survives MV3 worker restarts,
  // but is cleared when Chrome exits). Searching only in-progress downloads
  // misses restored records whose state has already changed during startup.
  const SESSION_INITIALIZED_KEY = "downloadHandoffSessionInitialized";
  const SEEN_DOWNLOAD_IDS_KEY = "downloadHandoffSeenDownloadIds";
  const seenDownloadIds = new Set();
  const pendingCreatedDownloads = [];
  let startupReady = false;

  // Ensure the worker is started at profile launch, before restored download
  // events are dispatched. The startup snapshot below then sees those items.
  chrome.runtime.onStartup.addListener(() => {});

  const processNewDownload = (download) => {
    if (!download || !Number.isInteger(download.id) || seenDownloadIds.has(download.id)) return;
    seenDownloadIds.add(download.id);
    chrome.storage.session.set({ [SEEN_DOWNLOAD_IDS_KEY]: [...seenDownloadIds] });
    handleCreatedDownload(download);
  };

  const finishStartup = (processPending = true) => {
    startupReady = true;
    const queued = pendingCreatedDownloads.splice(0);
    if (processPending) {
      for (const download of queued) processNewDownload(download);
    }
  };

  chrome.storage.session.get({
    [SESSION_INITIALIZED_KEY]: false,
    [SEEN_DOWNLOAD_IDS_KEY]: [],
  }, (session) => {
    for (const id of session[SEEN_DOWNLOAD_IDS_KEY] || []) seenDownloadIds.add(id);

    if (session[SESSION_INITIALIZED_KEY]) {
      finishStartup();
      return;
    }

    chrome.downloads.search({}, (downloads) => {
      if (chrome.runtime.lastError) {
        console.warn("Irani IDM could not inspect Chrome's existing downloads:", chrome.runtime.lastError.message);
        // Without a reliable snapshot, queued startup events may be restored
        // history. Drop them to avoid importing dozens of old downloads.
        finishStartup(false);
        return;
      }

      for (const download of downloads || []) {
        if (Number.isInteger(download.id)) seenDownloadIds.add(download.id);
      }
      chrome.storage.session.set({
        [SESSION_INITIALIZED_KEY]: true,
        [SEEN_DOWNLOAD_IDS_KEY]: [...seenDownloadIds],
      }, finishStartup);
    });
  });

  chrome.downloads.onCreated.addListener((download) => {
    if (!startupReady) {
      pendingCreatedDownloads.push(download);
      return;
    }
    processNewDownload(download);
  });
}

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
