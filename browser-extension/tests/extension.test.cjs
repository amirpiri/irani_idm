const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const extensionRoot = path.resolve(__dirname, '..');

function loadBackground({ enabled = false, firefoxData = null, deferGets = false, nativeResponse = { ok: true }, nativeError = null, startupDownloads = [], sessionState = {}, deferStartupSearch = false, startupSearchError = null } = {}) {
  const listeners = { created: null, installed: null, removed: null, startup: null };
  const state = {
    downloadHandoffEnabled: enabled,
    downloadHandoffConsentVersion: 0,
    downloadHandoffDecisionMade: null,
    downloadHandoffPrompted: false,
    downloadHandoffPromptReason: null,
  };
  const calls = { cancel: [], tabs: [], nativeMessages: [], openedOptions: 0, startupSearches: 0, startupQueries: [] };
  const pendingGets = [];
  let pendingStartupSearch = null;
  const chrome = {
    downloads: {
      onCreated: { addListener: (fn) => { listeners.created = fn; } },
      search: (query, callback) => {
        calls.startupSearches += 1;
        calls.startupQueries.push(query);
        const respond = () => {
          chrome.runtime.lastError = startupSearchError ? { message: startupSearchError } : undefined;
          callback(startupDownloads);
          chrome.runtime.lastError = undefined;
        };
        if (deferStartupSearch) pendingStartupSearch = respond;
        else respond();
      },
      cancel: (id, callback) => { calls.cancel.push(id); callback(); },
    },
    storage: {
      local: {
        get: (defaults, callback) => {
          const result = { ...defaults, ...state };
          if (deferGets) pendingGets.push(() => callback(result));
          else callback(result);
        },
        set: (values, callback) => { Object.assign(state, values); callback?.(); },
      },
      session: {
        get: (defaults, callback) => callback({ ...defaults, ...sessionState }),
        set: (values, callback) => { Object.assign(sessionState, values); callback?.(); },
      },
    },
    tabs: {
      create: (details, callback) => { calls.tabs.push(details); callback?.({ id: 7 }); },
    },
    runtime: {
      lastError: undefined,
      onStartup: { addListener: (fn) => { listeners.startup = fn; } },
      onInstalled: { addListener: (fn) => { listeners.installed = fn; } },
      openOptionsPage: () => { calls.openedOptions += 1; },
      sendNativeMessage: (host, message, callback) => {
        calls.nativeMessages.push({ host, message });
        chrome.runtime.lastError = nativeError ? { message: nativeError } : undefined;
        callback(nativeResponse);
        chrome.runtime.lastError = undefined;
      },
    },
  };
  // Simulate Chromium exposing the browser compatibility namespace too.
  const sandbox = {
    chrome,
    browser: {
      permissions: {
        getAll: async () => firefoxData === null ? {} : ({ data_collection: firefoxData }),
        remove: async () => { state.downloadHandoffEnabled = false; },
        onRemoved: { addListener: (fn) => { listeners.removed = fn; } },
      },
    },
    navigator: { userAgent: firefoxData === null ? 'Mozilla/5.0 Chrome/157.0.0.0 Safari/537.36' : 'Mozilla/5.0 Firefox/140.0' },
    console: { info() {}, warn() {}, error() {} },
    encodeURIComponent,
  };
  vm.runInNewContext(fs.readFileSync(path.join(extensionRoot, 'background.js'), 'utf8'), sandbox);
  return {
    listeners, state, calls, sessionState,
    flushGets: () => pendingGets.splice(0).forEach((callback) => callback()),
    flushStartupSearch: () => { const callback = pendingStartupSearch; pendingStartupSearch = null; callback?.(); },
  };
}

const sampleDownload = { id: 42, url: 'https://files.example.test/file.zip?token=private', byExtensionId: undefined };

test('download handoff is disabled by default', () => {
  const { listeners, calls, state } = loadBackground();
  listeners.created(sampleDownload);
  assert.deepEqual(calls.cancel, []);
  assert.deepEqual(calls.tabs, []);
  assert.equal(calls.openedOptions, 1);
  assert.equal(state.downloadHandoffPrompted, true);
  assert.equal(state.downloadHandoffPromptReason, 'download-attempt');
  listeners.created(sampleDownload);
  assert.equal(calls.openedOptions, 1, 'the reminder is shown only once until the user makes a choice');
});

test('a legacy disabled preference gets one recovery prompt because old versions could not distinguish an opt-out from a runtime reset', () => {
  const { listeners, calls, state } = loadBackground();
  state.downloadHandoffConsentVersion = 2;
  listeners.created(sampleDownload);
  assert.equal(state.downloadHandoffPrompted, true);
  assert.equal(calls.openedOptions, 1);
});

test('a new explicitly saved opt-out is respected without repeatedly prompting', () => {
  const { listeners, calls, state } = loadBackground();
  state.downloadHandoffConsentVersion = 2;
  state.downloadHandoffDecisionMade = true;
  listeners.created(sampleDownload);
  assert.equal(state.downloadHandoffPrompted, false);
  assert.equal(calls.openedOptions, 0);
});

test('enabled Chromium handoff sends the URL through Native Messaging and cancels only after acceptance', () => {
  const { listeners, calls } = loadBackground({ enabled: true });
  listeners.created(sampleDownload);
  assert.deepEqual(calls.cancel, [42]);
  assert.equal(calls.nativeMessages.length, 1);
  assert.equal(calls.nativeMessages[0].host, 'com.iranidm.browser');
  assert.equal(calls.nativeMessages[0].message.url, sampleDownload.url);
  assert.deepEqual(calls.tabs, [], 'Chromium handoff must not create a restorable custom-protocol tab');
});

test('Chromium ignores existing downloads of every state when the extension starts', () => {
  const restoredDownload = { ...sampleDownload, id: 220, state: 'in_progress' };
  const completedDownload = { ...sampleDownload, id: 219, state: 'complete' };
  const { listeners, calls } = loadBackground({ enabled: true, startupDownloads: [restoredDownload, completedDownload] });
  listeners.created(restoredDownload);
  listeners.created(completedDownload);
  assert.equal(calls.startupSearches, 1);
  assert.equal(Object.keys(calls.startupQueries[0]).length, 0, 'the startup snapshot must include completed and interrupted records too');
  assert.deepEqual(calls.nativeMessages, [], 'existing downloads must not be imported again');
  assert.deepEqual(calls.cancel, []);
});

test('Chromium registers a browser-startup listener so the snapshot runs before restored download events', () => {
  const { listeners, calls } = loadBackground({ enabled: true });
  assert.equal(typeof listeners.startup, 'function');
  assert.equal(calls.startupSearches, 1);
});

test('Chromium waits for the startup snapshot before deciding whether a created event is new', () => {
  const restoredDownload = { ...sampleDownload, id: 221, state: 'in_progress' };
  const { listeners, calls, flushStartupSearch } = loadBackground({
    enabled: true,
    startupDownloads: [restoredDownload],
    deferStartupSearch: true,
  });
  listeners.created(restoredDownload);
  assert.deepEqual(calls.nativeMessages, [], 'do not hand off before the startup snapshot is ready');
  flushStartupSearch();
  assert.deepEqual(calls.nativeMessages, [], 'the queued restored event must be ignored after the snapshot');
});

test('Chromium drops queued startup events if the existing-download snapshot fails', () => {
  const { listeners, calls, flushStartupSearch } = loadBackground({ enabled: true, deferStartupSearch: true, startupSearchError: 'API unavailable' });
  listeners.created(sampleDownload);
  flushStartupSearch();
  assert.deepEqual(calls.nativeMessages, [], 'uncertain startup events must not be imported');
  listeners.created({ ...sampleDownload, id: 44 });
  assert.equal(calls.nativeMessages.length, 1, 'new downloads after startup should still work');
});

test('Chromium remembers handled IDs across service-worker restarts but accepts genuinely new IDs', () => {
  const first = loadBackground({ enabled: true });
  first.listeners.created(sampleDownload);
  assert.equal(first.calls.nativeMessages.length, 1);

  const restarted = loadBackground({ enabled: true, sessionState: first.sessionState });
  restarted.listeners.created(sampleDownload);
  assert.deepEqual(restarted.calls.nativeMessages, [], 'the same download ID must not be handed off twice');
  restarted.listeners.created({ ...sampleDownload, id: 43 });
  assert.equal(restarted.calls.nativeMessages.length, 1, 'a new download ID should still be handed off');
});

test('Chromium keeps the browser download if the Native Messaging host is unavailable', () => {
  const { listeners, calls } = loadBackground({ enabled: true, nativeError: 'Specified native messaging host not found' });
  listeners.created(sampleDownload);
  assert.equal(calls.nativeMessages.length, 1);
  assert.equal(calls.nativeMessages[0].host, 'com.iranidm.browser');
  assert.equal(calls.nativeMessages[0].message.url, sampleDownload.url);
  assert.deepEqual(calls.cancel, [], 'the original browser download must continue when handoff fails');
  assert.deepEqual(calls.tabs, []);
});

test('Chromium keeps the browser download if the Native Messaging host rejects the URL', () => {
  const { listeners, calls } = loadBackground({ enabled: true, nativeResponse: { ok: false, error: 'Unsupported URL' } });
  listeners.created(sampleDownload);
  assert.equal(calls.nativeMessages.length, 1);
  assert.deepEqual(calls.cancel, []);
});

test('simultaneous downloads open the first-run Options page only once', () => {
  const { listeners, calls, flushGets } = loadBackground({ deferGets: true });
  listeners.created(sampleDownload);
  listeners.created({ ...sampleDownload, id: 43 });
  flushGets();
  assert.equal(calls.openedOptions, 1);
});

test('Chromium browser compatibility namespace does not trigger Firefox data permission checks', () => {
  const { listeners, state, calls } = loadBackground({ enabled: true });
  listeners.created(sampleDownload);
  assert.equal(state.downloadHandoffEnabled, true);
  assert.deepEqual(calls.cancel, [42]);
  assert.equal(calls.nativeMessages.length, 1);
});

test('Firefox does not transfer URLs when optional data permissions are missing', async () => {
  const { listeners, state, calls } = loadBackground({ enabled: true, firefoxData: ['browsingActivity'] });
  listeners.created(sampleDownload);
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(calls.cancel, []);
  assert.equal(state.downloadHandoffEnabled, false);
});

test('Firefox transfers URLs only when both optional data permissions are granted', async () => {
  const { listeners, calls } = loadBackground({ enabled: true, firefoxData: ['browsingActivity', 'websiteActivity'] });
  listeners.created(sampleDownload);
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(calls.cancel, [42]);
  assert.equal(calls.tabs.length, 1);
  assert.deepEqual(calls.nativeMessages, []);
});

test('revoking either Firefox data permission disables handoff', () => {
  const { listeners, state } = loadBackground({ enabled: true, firefoxData: ['browsingActivity', 'websiteActivity'] });
  listeners.removed({ data_collection: ['websiteActivity'] });
  assert.equal(state.downloadHandoffEnabled, false);
  assert.equal(state.downloadHandoffDecisionMade, false);
  assert.equal(state.downloadHandoffPromptReason, 'permission-missing');
});

test('disabling handoff from Options is not mistaken for an external permission revocation', () => {
  const { listeners, state } = loadBackground({ enabled: true, firefoxData: ['browsingActivity', 'websiteActivity'] });
  state.downloadHandoffEnabled = false;
  state.downloadHandoffDecisionMade = true;
  listeners.removed({ data_collection: ['websiteActivity'] });
  assert.equal(state.downloadHandoffEnabled, false);
  assert.equal(state.downloadHandoffDecisionMade, true);
  assert.equal(state.downloadHandoffPromptReason, null);
});

test('install opens the consent page and an update with old consent resets handoff', () => {
  const { listeners, state, calls } = loadBackground({ enabled: true });
  listeners.installed({ reason: 'install' });
  assert.equal(calls.openedOptions, 1);
  listeners.installed({ reason: 'update' });
  assert.equal(state.downloadHandoffEnabled, false);
  assert.equal(state.downloadHandoffConsentVersion, 2);
  assert.equal(state.downloadHandoffDecisionMade, false);
  assert.equal(state.downloadHandoffPrompted, false);
  assert.equal(calls.openedOptions, 2);
});

test('Firefox update revokes old data permissions before asking for consent again', async () => {
  const { listeners, state, calls } = loadBackground({
    enabled: true,
    firefoxData: ['browsingActivity', 'websiteActivity'],
  });
  state.downloadHandoffConsentVersion = 1;
  listeners.installed({ reason: 'update' });
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(state.downloadHandoffEnabled, false);
  assert.equal(state.downloadHandoffConsentVersion, 2);
  assert.equal(calls.openedOptions, 1);
});

test('non-HTTP downloads and extension-originated downloads are ignored', () => {
  const { listeners, calls } = loadBackground({ enabled: true });
  listeners.created({ ...sampleDownload, url: 'blob:https://example.test/id' });
  listeners.created({ ...sampleDownload, byExtensionId: 'some-extension' });
  assert.deepEqual(calls.cancel, []);
  assert.deepEqual(calls.tabs, []);
});

function loadOptions({ enabled = false, permissionGranted = true, firefox = true, promptReason = null } = {}) {
  const state = {
    downloadHandoffEnabled: enabled,
    downloadHandoffConsentVersion: 0,
    downloadHandoffDecisionMade: null,
    downloadHandoffPrompted: false,
    downloadHandoffPromptReason: promptReason,
  };
  const elements = {
    consent: { checked: false, disabled: false },
    save: { disabled: false, addEventListener(event, fn) { this.handler = fn; } },
    decline: { hidden: true, addEventListener(event, fn) { this.handler = fn; } },
    prompt: { hidden: true, textContent: '' },
    status: { textContent: '' },
  };
  const calls = { requested: [], removed: [], uninstall: [] };
  const chrome = {
    storage: {
      local: {
        get: (defaults, callback) => callback({ ...defaults, ...state }),
        set: (values, callback) => { Object.assign(state, values); callback?.(); },
      },
    },
    runtime: { lastError: undefined },
  };
  const browser = {
    // Deliberately expose getBrowserInfo in both cases: browser API presence
    // alone must not make Chromium take Firefox's data_collection path.
    runtime: { getBrowserInfo: async () => ({ name: firefox ? 'Firefox' : 'Chrome' }) },
    storage: {
      local: {
        get: async (defaults) => ({ ...defaults, ...state }),
        set: async (values) => { Object.assign(state, values); },
      },
    },
    permissions: {
      getAll: async () => ({ data_collection: enabled && permissionGranted ? ['browsingActivity', 'websiteActivity'] : [] }),
      request: async (value) => { calls.requested.push(value); return permissionGranted; },
      remove: async (value) => { calls.removed.push(value); return true; },
    },
    management: {
      uninstallSelf: async (options) => { calls.uninstall.push(options); },
    },
    tabs: { create: () => {} },
  };
  vm.runInNewContext(fs.readFileSync(path.join(extensionRoot, 'options.js'), 'utf8'), {
    chrome,
    browser,
    navigator: { userAgent: firefox ? 'Mozilla/5.0 Firefox/140.0' : 'Mozilla/5.0 Chrome/157.0.0.0 Safari/537.36' },
    document: { getElementById: (id) => elements[id] },
    console: { error() {} },
    Promise,
  });
  return { state, elements, calls };
}

test('Chromium saves consent without sending Firefox-only data_collection permissions', async () => {
  const { state, elements, calls } = loadOptions({ firefox: false });
  await new Promise((resolve) => setImmediate(resolve));
  elements.consent.checked = true;
  elements.save.handler();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(state.downloadHandoffEnabled, true);
  assert.equal(state.downloadHandoffConsentVersion, 2);
  assert.equal(state.downloadHandoffDecisionMade, true);
  assert.equal(calls.requested.length, 0);
  assert.equal(elements.status.textContent, 'Saved. Browser downloads will now be handed to Irani IDM.');
});

test('Options displays and acknowledges the automatic reminder without enabling handoff', async () => {
  const { state, elements } = loadOptions({ firefox: false, promptReason: 'download-attempt' });
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(elements.prompt.hidden, false);
  assert.match(elements.prompt.textContent, /started a download while handoff was turned off/i);
  assert.equal(elements.consent.checked, false);
  assert.equal(state.downloadHandoffEnabled, false);
  assert.equal(state.downloadHandoffPromptReason, null);
  assert.equal(state.downloadHandoffPrompted, true);
});

test('Firefox consent requests both optional data categories only after Save is clicked', async () => {
  const { state, elements, calls } = loadOptions({ permissionGranted: true });
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(calls.requested.length, 0);
  elements.consent.checked = true;
  elements.save.handler();
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(Array.from(calls.requested[0].data_collection), ['browsingActivity', 'websiteActivity']);
  assert.equal(state.downloadHandoffEnabled, true);
  assert.equal(state.downloadHandoffConsentVersion, 2);
  assert.equal(state.downloadHandoffDecisionMade, true);
});

test('declining Firefox data permission keeps handoff disabled and exposes uninstall route', async () => {
  const { state, elements, calls } = loadOptions({ permissionGranted: false });
  elements.consent.checked = true;
  elements.save.handler();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(state.downloadHandoffEnabled, false);
  assert.equal(elements.consent.checked, false);
  assert.equal(elements.decline.hidden, false);
  assert.match(elements.status.textContent, /permission was declined/i);
  assert.equal(calls.requested.length, 1);
});

test('disabling Firefox handoff revokes optional data permissions', async () => {
  const { state, elements, calls } = loadOptions({ enabled: true, permissionGranted: true });
  await new Promise((resolve) => setImmediate(resolve));
  elements.consent.checked = false;
  elements.save.handler();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(state.downloadHandoffEnabled, false);
  assert.deepEqual(Array.from(calls.removed[0].data_collection), ['browsingActivity', 'websiteActivity']);
});

test('Firefox decline offers native self-uninstall without requesting management permission', async () => {
  const { state, elements, calls } = loadOptions();
  elements.decline.handler();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(state.downloadHandoffEnabled, false);
  assert.equal(calls.uninstall.length, 1);
  assert.equal(calls.uninstall[0].showConfirmDialog, true);
});

function loadPopup({ enabled = false, firefox = false, permissionGranted = true } = {}) {
  const state = { downloadHandoffEnabled: enabled };
  const elements = {
    enabled: { checked: false },
    save: { disabled: false, addEventListener(event, fn) { this.handler = fn; } },
    status: { textContent: '' },
    options: { addEventListener(event, fn) { this.handler = fn; } },
  };
  const calls = { requested: [], removed: [], openedOptions: 0 };
  const chrome = {
    runtime: { lastError: undefined, openOptionsPage: () => { calls.openedOptions += 1; } },
    storage: { local: {
      get: (defaults, callback) => callback({ ...defaults, ...state }),
      set: (values, callback) => { Object.assign(state, values); callback?.(); },
    } },
  };
  const browser = {
    runtime: { openOptionsPage: () => { calls.openedOptions += 1; } },
    storage: { local: {
      get: async (defaults) => ({ ...defaults, ...state }),
      set: async (values) => { Object.assign(state, values); },
    } },
    permissions: {
      getAll: async () => ({ data_collection: enabled && permissionGranted ? ['browsingActivity', 'websiteActivity'] : [] }),
      request: async (value) => { calls.requested.push(value); return permissionGranted; },
      remove: async (value) => { calls.removed.push(value); },
    },
  };
  vm.runInNewContext(fs.readFileSync(path.join(extensionRoot, 'popup.js'), 'utf8'), {
    chrome, browser,
    navigator: { userAgent: firefox ? 'Mozilla/5.0 Firefox/140.0' : 'Mozilla/5.0 Chrome/157.0.0.0 Safari/537.36' },
    document: { getElementById: (id) => elements[id] },
    Promise,
  });
  return { state, elements, calls };
}

test('toolbar popup loads and saves the Chromium download-catching preference', async () => {
  const { state, elements, calls } = loadPopup();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(elements.enabled.checked, false);
  elements.enabled.checked = true;
  elements.save.handler();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(state.downloadHandoffEnabled, true);
  assert.equal(state.downloadHandoffDecisionMade, true);
  assert.match(elements.status.textContent, /Enabled/);
  assert.equal(calls.requested.length, 0);
});

test('toolbar popup saves a Chromium opt-out', async () => {
  const { state, elements } = loadPopup({ enabled: true });
  await new Promise((resolve) => setImmediate(resolve));
  elements.enabled.checked = false;
  elements.save.handler();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(state.downloadHandoffEnabled, false);
  assert.equal(state.downloadHandoffDecisionMade, true);
  assert.match(elements.status.textContent, /Disabled/);
});

test('toolbar popup requests Firefox data permissions before enabling download catching', async () => {
  const { state, elements, calls } = loadPopup({ firefox: true });
  await new Promise((resolve) => setImmediate(resolve));
  elements.enabled.checked = true;
  elements.save.handler();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(state.downloadHandoffEnabled, true);
  assert.deepEqual(Array.from(calls.requested[0].data_collection), ['browsingActivity', 'websiteActivity']);
});

test('toolbar popup keeps Firefox download catching disabled when permission is declined', async () => {
  const { state, elements } = loadPopup({ firefox: true, permissionGranted: false });
  await new Promise((resolve) => setImmediate(resolve));
  elements.enabled.checked = true;
  elements.save.handler();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(state.downloadHandoffEnabled, false);
  assert.equal(elements.enabled.checked, false);
  assert.match(elements.status.textContent, /Permission declined/);
});

test('toolbar popup disables Firefox download catching and revokes its optional permissions', async () => {
  const { state, elements, calls } = loadPopup({ enabled: true, firefox: true });
  await new Promise((resolve) => setImmediate(resolve));
  elements.enabled.checked = false;
  elements.save.handler();
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(state.downloadHandoffEnabled, false);
  assert.deepEqual(Array.from(calls.removed[0].data_collection), ['browsingActivity', 'websiteActivity']);
});
