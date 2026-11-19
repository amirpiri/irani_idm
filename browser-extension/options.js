const consent = document.getElementById("consent");
const save = document.getElementById("save");
const decline = document.getElementById("decline");
const status = document.getElementById("status");
const prompt = document.getElementById("prompt");

const firefoxDataPermissions = ["browsingActivity", "websiteActivity"];
// Chromium may expose a `browser` namespace (or compatibility APIs), so use
// the actual host browser's user agent before sending Firefox-only fields.
const isFirefox = typeof navigator !== "undefined" && /Firefox\//.test(navigator.userAgent);

const extensionStorage = isFirefox ? browser.storage.local : chrome.storage.local;

const readPreference = () => isFirefox
  ? extensionStorage.get({ downloadHandoffEnabled: false, downloadHandoffPromptReason: null })
  : new Promise((resolve, reject) => extensionStorage.get({ downloadHandoffEnabled: false, downloadHandoffPromptReason: null }, (value) => {
    if (chrome.runtime.lastError) reject(new Error(chrome.runtime.lastError.message));
    else resolve(value);
  }));

const persistPreference = (value) => isFirefox
  ? extensionStorage.set({
      downloadHandoffEnabled: value,
      downloadHandoffConsentVersion: 2,
      downloadHandoffDecisionMade: true,
      downloadHandoffPrompted: true,
      downloadHandoffPromptReason: null,
    })
  : new Promise((resolve, reject) => extensionStorage.set({
      downloadHandoffEnabled: value,
      downloadHandoffConsentVersion: 2,
      downloadHandoffDecisionMade: true,
      downloadHandoffPrompted: true,
      downloadHandoffPromptReason: null,
    }, () => {
      if (chrome.runtime.lastError) reject(new Error(chrome.runtime.lastError.message));
      else resolve();
    }));

const persistPromptAcknowledgement = () => isFirefox
  ? extensionStorage.set({ downloadHandoffPromptReason: null, downloadHandoffPrompted: true })
  : new Promise((resolve, reject) => extensionStorage.set({
    downloadHandoffPromptReason: null,
    downloadHandoffPrompted: true,
  }, () => {
    if (chrome.runtime.lastError) reject(new Error(chrome.runtime.lastError.message));
    else resolve();
  }));

readPreference().then(async (settings) => {
  const { downloadHandoffEnabled } = settings;
  let enabled = downloadHandoffEnabled === true;
  if (isFirefox) {
    try {
      const granted = await browser.permissions.getAll();
      const data = granted.data_collection || [];
      if (!firefoxDataPermissions.every((permission) => data.includes(permission))) {
        enabled = false;
        await persistPreference(false);
      }
    } catch {
      enabled = false;
      await persistPreference(false);
    }
  }
  consent.checked = enabled;
  if (settings.downloadHandoffPromptReason) {
    prompt.textContent = settings.downloadHandoffPromptReason === "permission-missing"
      ? "Download handoff was turned off because a required Firefox permission is missing. Review the notice below, then enable it and save if you still agree."
      : "You started a download while handoff was turned off. Review the notice below and enable handoff only if you agree; otherwise, your downloads will stay in the browser.";
    prompt.hidden = false;
    await persistPromptAcknowledgement();
  }
});

decline.hidden = !isFirefox;

save.addEventListener("click", () => {
  const enabled = consent.checked;
  save.disabled = true;

  (async () => {
    try {
      if (enabled && isFirefox) {
        const granted = await browser.permissions.request({
          data_collection: firefoxDataPermissions
        });
        if (!granted) {
          consent.checked = false;
          await persistPreference(false);
          decline.hidden = false;
          status.textContent = "Firefox permission was declined. This extension's download handoff cannot work without it. Choose ‘Decline and continue to uninstall’ if you do not want to enable handoff.";
          return;
        }
      }

      if (!enabled && isFirefox) {
        await browser.permissions.remove({
          data_collection: firefoxDataPermissions
        });
      }

      await persistPreference(enabled);
      decline.hidden = true;
      status.textContent = enabled
        ? "Saved. Browser downloads will now be handed to Irani IDM."
        : "Saved. Browser downloads will remain in your browser.";
    } catch (error) {
      if (enabled) {
        consent.checked = false;
        if (isFirefox) {
          try {
            await browser.permissions.remove({ data_collection: firefoxDataPermissions });
          } catch {
            // The stored disabled state and background permission check still fail closed.
          }
        }
      }
      const reason = error instanceof Error && error.message ? ` (${error.message})` : "";
      status.textContent = `Could not update your preference${reason}. Downloads remain in your browser.`;
      console.error("Irani IDM preference update failed:", error);
    } finally {
      save.disabled = false;
    }
  })();
});

decline.addEventListener("click", () => {
  persistPreference(false).then(() => {
    if (isFirefox && browser.management?.uninstallSelf) {
      browser.management.uninstallSelf({
        showConfirmDialog: true,
        dialogMessage: "Download handoff requires the optional download-activity data permission. Uninstall Irani IDM Browser Integration if you do not want to grant it."
      }).catch(() => {
        status.textContent = "Uninstall was canceled. Download handoff remains disabled; you can remove the extension from Firefox's Add-ons Manager."
      });
    } else if (isFirefox && browser.tabs?.create) {
      browser.tabs.create({ url: "about:addons" });
      status.textContent = "Remove Irani IDM Browser Integration from Firefox's Add-ons Manager to finish uninstalling.";
    }
  }).catch((error) => {
    status.textContent = "Could not save the disabled preference. You can remove the extension from Firefox's Add-ons Manager.";
    console.error("Irani IDM preference update failed:", error);
  });
});
