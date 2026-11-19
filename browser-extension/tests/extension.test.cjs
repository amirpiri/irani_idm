const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');

const extensionRoot = path.resolve(__dirname, '..');

function loadBackground({ enabled = false, firefoxData = null } = {}) {
  const listeners = { created: null, installed: null, removed: null };
  const state = { downloadHandoffEnabled: enabled, downloadHandoffConsentVersion: 0 };
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
  const sandbox = { chrome, console: { info() {}, warn() {}, error() {} }, encodeURIComponent };
  if (firefoxData !== null) {
    sandbox.browser = {
      permissions: {
        getAll: async () => ({ data_collection: firefoxData }),
        remove: async () => { state.downloadHandoffEnabled = false; },
        onRemoved: { addListener: (fn) => { listeners.removed = fn; } },
      },
    };
  }
  vm.runInNewContext(fs.readFileSync(path.join(extensionRoot, 'background.js'), 'utf8'), sandbox);
  return { listeners, state, calls };
}

const sampleDownload = { id: 42, url: 'https://files.example.test/file.zip?token=private', byExtensionId: undefined };

test('download handoff is disabled by default', () => {
  const { listeners, calls } = loadBackground();
  listeners.created(sampleDownload);
  assert.deepEqual(calls.cancel, []);
  assert.deepEqual(calls.tabs, []);
});

test('enabled Chromium handoff cancels the browser transfer and opens the app protocol', () => {
  const { listeners, calls } = loadBackground({ enabled: true });
  listeners.created(sampleDownload);
  assert.deepEqual(calls.cancel, [42]);
  assert.equal(calls.tabs.length, 1);
  assert.equal(calls.tabs[0].url, `iranidm://download?url=${encodeURIComponent(sampleDownload.url)}`);
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
});

test('install opens the consent page and an update with old consent resets handoff', () => {
  const { listeners, state, calls } = loadBackground({ enabled: true });
  listeners.installed({ reason: 'install' });
  assert.equal(calls.openedOptions, 1);
  listeners.installed({ reason: 'update' });
  assert.equal(state.downloadHandoffEnabled, false);
  assert.equal(state.downloadHandoffConsentVersion, 2);
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

function loadOptions({ enabled = false, permissionGranted = true } = {}) {
  const state = { downloadHandoffEnabled: enabled, downloadHandoffConsentVersion: 0 };
  const elements = {
    consent: { checked: false, disabled: false },
    save: { disabled: false, addEventListener(event, fn) { this.handler = fn; } },
    decline: { hidden: true, addEventListener(event, fn) { this.handler = fn; } },
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
    document: { getElementById: (id) => elements[id] },
    console: { error() {} },
    Promise,
  });
  return { state, elements, calls };
}

test('Firefox consent requests both optional data categories only after Save is clicked', async () => {
  const { state, elements, calls } = loadOptions({ permissionGranted: true });
  assert.equal(calls.requested.length, 0);
  elements.consent.checked = true;
  elements.save.handler();
  await new Promise((resolve) => setImmediate(resolve));
  assert.deepEqual(Array.from(calls.requested[0].data_collection), ['browsingActivity', 'websiteActivity']);
  assert.equal(state.downloadHandoffEnabled, true);
  assert.equal(state.downloadHandoffConsentVersion, 2);
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
