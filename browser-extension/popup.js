const enabledInput = document.getElementById("enabled");
const saveButton = document.getElementById("save");
const status = document.getElementById("status");
const optionsLink = document.getElementById("options");

const isFirefox = typeof navigator !== "undefined" && /Firefox\//.test(navigator.userAgent);
const storage = isFirefox ? browser.storage.local : chrome.storage.local;
const firefoxDataPermissions = ["browsingActivity", "websiteActivity"];

const readSettings = () => isFirefox
  ? storage.get({ downloadHandoffEnabled: false })
  : new Promise((resolve, reject) => storage.get({ downloadHandoffEnabled: false }, (settings) => {
    if (chrome.runtime.lastError) reject(new Error(chrome.runtime.lastError.message));
    else resolve(settings);
  }));

const saveSettings = (enabled) => {
  const values = {
    downloadHandoffEnabled: enabled,
    downloadHandoffConsentVersion: 2,
    downloadHandoffDecisionMade: true,
    downloadHandoffPrompted: true,
    downloadHandoffPromptReason: null,
  };
  return isFirefox
    ? storage.set(values)
    : new Promise((resolve, reject) => storage.set(values, () => {
      if (chrome.runtime.lastError) reject(new Error(chrome.runtime.lastError.message));
      else resolve();
    }));
};

readSettings().then(async (settings) => {
  let enabled = settings.downloadHandoffEnabled === true;
  if (enabled && isFirefox) {
    const granted = await browser.permissions.getAll();
    const data = granted.data_collection || [];
    enabled = firefoxDataPermissions.every((permission) => data.includes(permission));
    if (!enabled) await saveSettings(false);
  }
  enabledInput.checked = enabled;
}).catch((error) => {
  status.textContent = `Could not read setting: ${error.message}`;
  saveButton.disabled = true;
});

saveButton.addEventListener("click", async () => {
  const enabled = enabledInput.checked;
  saveButton.disabled = true;
  try {
    if (enabled && isFirefox) {
      const granted = await browser.permissions.request({ data_collection: firefoxDataPermissions });
      if (!granted) {
        enabledInput.checked = false;
        await saveSettings(false);
        status.textContent = "Permission declined. Browser downloads will stay in your browser.";
        return;
      }
    } else if (!enabled && isFirefox) {
      // Persist the opt-out before revoking permissions so the background
      // onRemoved listener recognizes this as the user's choice.
      await saveSettings(false);
      await browser.permissions.remove({ data_collection: firefoxDataPermissions });
      status.textContent = "Disabled. New downloads will stay in your browser.";
      return;
    }

    await saveSettings(enabled);
    status.textContent = enabled
      ? "Enabled. New downloads will be sent to Irani IDM."
      : "Disabled. New downloads will stay in your browser.";
  } catch (error) {
    enabledInput.checked = false;
    if (enabled && isFirefox) {
      try { await browser.permissions.remove({ data_collection: firefoxDataPermissions }); } catch {}
    }
    status.textContent = `Could not save setting: ${error.message}`;
  } finally {
    saveButton.disabled = false;
  }
});

optionsLink.addEventListener("click", (event) => {
  event.preventDefault();
  if (isFirefox) browser.runtime.openOptionsPage();
  else chrome.runtime.openOptionsPage();
});
