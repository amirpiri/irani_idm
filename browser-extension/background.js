const APP_SCHEME = "iranidm://download?url=";

chrome.downloads.onCreated.addListener((download) => {
  if (!download.url || !/^https?:\/\//i.test(download.url) || download.byExtensionId) return;

  console.info("Irani IDM detected a browser download:", download.url);

  // Stop the browser transfer before handing the original URL to the desktop app.
  chrome.downloads.cancel(download.id, () => {
    if (chrome.runtime.lastError) {
      console.warn("Irani IDM could not cancel the browser download:", chrome.runtime.lastError.message);
      return;
    }
    // Keep the tab alive: closing it immediately can cancel Chrome's external
    // protocol handoff before Windows has a chance to launch Irani IDM.
    chrome.tabs.create({ url: APP_SCHEME + encodeURIComponent(download.url), active: true }, (tab) => {
      if (chrome.runtime.lastError) {
        console.error("Could not open Irani IDM:", chrome.runtime.lastError.message);
      } else {
        console.info("Sent browser download to Irani IDM.", tab?.id);
      }
    });
  });
});
