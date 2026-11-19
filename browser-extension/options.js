const consent = document.getElementById("consent");
const save = document.getElementById("save");
const decline = document.getElementById("decline");
const status = document.getElementById("status");

const firefoxDataPermissions = ["browsingActivity", "websiteActivity"];
const isFirefox = typeof browser !== "undefined" && !!browser.permissions?.request;

chrome.storage.local.get({ downloadHandoffEnabled: false }, async ({ downloadHandoffEnabled }) => {
  let enabled = downloadHandoffEnabled === true;
  if (isFirefox) {
    try {
      const granted = await browser.permissions.getAll();
      const data = granted.data_collection || [];
      if (!firefoxDataPermissions.every((permission) => data.includes(permission))) {
        enabled = false;
        await new Promise((resolve) => chrome.storage.local.set({ downloadHandoffEnabled: false }, resolve));
      }
    } catch {
      enabled = false;
      await new Promise((resolve) => chrome.storage.local.set({ downloadHandoffEnabled: false }, resolve));
    }
  }
  consent.checked = enabled;
});

decline.hidden = !isFirefox;

save.addEventListener("click", () => {
  const enabled = consent.checked;
  save.disabled = true;

  const persistPreference = (value) => new Promise((resolve, reject) => {
    chrome.storage.local.set({
      downloadHandoffEnabled: value,
      downloadHandoffConsentVersion: 2
    }, () => {
      if (chrome.runtime.lastError) reject(new Error(chrome.runtime.lastError.message));
      else resolve();
    });
  });

  (async () => {
    try {
      if (enabled && typeof browser !== "undefined" && browser.permissions?.request) {
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

      if (!enabled && typeof browser !== "undefined" && browser.permissions?.remove) {
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
        if (typeof browser !== "undefined" && browser.permissions?.remove) {
          try {
            await browser.permissions.remove({ data_collection: firefoxDataPermissions });
          } catch {
            // The stored disabled state and background permission check still fail closed.
          }
        }
      }
      status.textContent = "Could not update your preference. Downloads remain in your browser.";
      console.error("Irani IDM preference update failed:", error);
    } finally {
      save.disabled = false;
    }
  })();
});

decline.addEventListener("click", () => {
  chrome.storage.local.set({ downloadHandoffEnabled: false }, () => {
    if (typeof browser !== "undefined" && browser.management?.uninstallSelf) {
      browser.management.uninstallSelf({
        showConfirmDialog: true,
        dialogMessage: "Download handoff requires the optional download-activity data permission. Uninstall Irani IDM Browser Integration if you do not want to grant it."
      }).catch(() => {
        status.textContent = "Uninstall was canceled. Download handoff remains disabled; you can remove the extension from Firefox's Add-ons Manager."
      });
    } else if (typeof browser !== "undefined" && browser.tabs?.create) {
      browser.tabs.create({ url: "about:addons" });
      status.textContent = "Remove Irani IDM Browser Integration from Firefox's Add-ons Manager to finish uninstalling.";
    }
  });
});
