const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const extensionRoot = path.resolve(__dirname, '..');

function loadBackground({ enabled = false, firefoxData = null } = {}) {
  const listeners = { created: null, installed: null, removed: null };
  const state = {
    downloadHandoffEnabled: enabled,
    downloadHandoffConsentVersion: 0,
    downloadHandoffDecisionMade: null,
    downloadHandoffPrompted: false,
    downloadHandoffPromptReason: null,
  };
  const calls = { cancel: [], tabs: [], openedOptions: 0 };
  const chrome = {
    downloads: {
      onCreated: { addListener: (fn) => { listeners.created = fn; } },
      cancel: (id, callback) => { calls.cancel.push(id); callback(); },
    },
    storage: {
      local: {
        get: (defaults, callback) => callback({ ...defaults, ...state }),
        set: (values, callback) => { Object.assign(state, values); callback?.(); },
      },
    },
    tabs: {
      create: (details, callback) => { calls.tabs.push(details); callback?.({ id: 7 }); },
    },
    runtime: {
      lastError: undefined,
      onInstalled: { addListener: (fn) => { listeners.installed = fn; } },
      openOptionsPage: () => { calls.openedOptions += 1; },
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
  return { listeners, state, calls };
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

test('enabled Chromium handoff cancels the browser transfer and opens the app protocol', () => {
  const { listeners, calls } = loadBackground({ enabled: true });
  listeners.created(sampleDownload);
  assert.deepEqual(calls.cancel, [42]);
  assert.equal(calls.tabs.length, 1);
  assert.equal(calls.tabs[0].url, `iranidm://download?url=${encodeURIComponent(sampleDownload.url)}`);
});

test('Chromium browser compatibility namespace does not trigger Firefox data permission checks', () => {
  const { listeners, state, calls } = loadBackground({ enabled: true });
  listeners.created(sampleDownload);
  assert.equal(state.downloadHandoffEnabled, true);
  assert.deepEqual(calls.cancel, [42]);
  assert.equal(calls.tabs.length, 1);
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
