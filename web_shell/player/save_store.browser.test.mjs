import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { existsSync } from 'node:fs';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { chromeBinary, connectCdp, serveStaticSite, until } from './browser_test_support.mjs';

const shellRoot = resolve(fileURLToPath(new URL('../', import.meta.url)));

test('IndexedDB saves commit atomically and survive a player reload by game ID', async t => {
  assert.ok(existsSync(chromeBinary), `Chrome not found at ${chromeBinary}`);
  const profile = await mkdtemp(resolve(tmpdir(), 'bishop-idb-test-'));
  const server = serveStaticSite(shellRoot);
  let chrome;
  let cdp;
  t.after(async () => {
    cdp?.socket.close();
    if (chrome && chrome.exitCode === null) {
      await new Promise(resolve => {
        const timer = setTimeout(resolve, 5000);
        chrome.once('exit', () => { clearTimeout(timer); resolve(); });
        chrome.kill();
      });
    }
    await new Promise(resolve => server.close(resolve));
    await rm(profile, { recursive: true, force: true, maxRetries: 8, retryDelay: 100 });
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const site = `http://127.0.0.1:${server.address().port}/player.html`;
  const page = await fetch(site);
  assert.equal(page.status, 200, 'the test server must serve the player');
  chrome = spawn(chromeBinary, [
    '--headless=new', '--no-first-run', '--disable-extensions', '--no-default-browser-check',
    '--remote-allow-origins=*', '--remote-debugging-port=0', `--user-data-dir=${profile}`, site,
  ], { stdio: 'ignore' });
  cdp = await connectCdp(profile);
  await cdp.command('Page.enable');
  await cdp.command('Runtime.enable');
  await until(() => cdp.evaluate('Boolean(window.bishopSaveStore)'), 'browser save module')
    .catch(async error => {
      const page = await cdp.evaluate(`({url: location.href, ready: document.readyState,
        status: document.getElementById('status')?.textContent,
        resources: performance.getEntriesByType('resource').map(entry => entry.name)})`);
      throw new Error(`${error.message}: ${JSON.stringify(page)}`);
    });

  const first = randomUUID(), second = randomUUID();
  const before = await cdp.evaluate(`(async () => {
    const store = window.bishopSaveStore;
    const first = ${JSON.stringify(first)}, second = ${JSON.stringify(second)};
    const opened = await store.open(first);
    const writes = [
      store.write(first, 'default', 'manual', 'first', 1),
      store.write(first, 'default', 'autosave', 'second', 2),
    ];
    await Promise.all(writes);
    const completed = [store.takeResult(), store.takeResult(), store.takeResult()];
    const otherBefore = await store.open(second);
    await store.write(second, 'default', 'manual', 'other game', 3);
    const otherWrite = store.takeResult();
    const latest = await store.open(first);
    const other = await store.open(second);
    return { opened, completed, otherBefore, otherWrite, latest, other };
  })()`);
  assert.equal(before.opened.status, 'ready');
  assert.equal(before.opened.latest, null);
  assert.deepEqual(before.completed.map(result => result?.requestId), [1, 2, undefined]);
  assert.equal(before.completed[0].ok, true);
  assert.equal(before.completed[1].ok, true);
  assert.equal(before.latest.latest, 'second');
  assert.equal(before.latest.slot, 'default');
  assert.equal(before.latest.lane, 'autosave');
  assert.equal(before.otherBefore.latest, null);
  assert.deepEqual(before.otherWrite, { requestId: 3, ok: true });
  assert.equal(before.other.latest, 'other game');

  const loaded = cdp.nextEvent('Page.loadEventFired');
  await cdp.command('Page.reload', { ignoreCache: true });
  await loaded;
  await until(() => cdp.evaluate('Boolean(window.bishopSaveStore)'), 'reloaded save module');
  const after = await cdp.evaluate(`(async () => {
    const { BrowserSaveStore, STORAGE_NAME, DOCUMENTS_STORE, LATEST_STORE } =
      await import('./player/save_store.mjs');
    const store = window.bishopSaveStore;
    const first = ${JSON.stringify(first)}, second = ${JSON.stringify(second)};
    const reloaded = await store.open(first);
    const isolated = await store.open(second);
    const bridge = new BrowserSaveStore();
    bridge.beginOpen(first);
    const openedBridge = await new Promise((resolve, reject) => {
      const timeout = setTimeout(() => reject(new Error('bridge open timed out')), 4000);
      function poll() {
        const result = bridge.takeOpenResult();
        if (result) { clearTimeout(timeout); resolve(result); }
        else setTimeout(poll, 0);
      }
      poll();
    });
    const drainedBridge = bridge.takeOpenResult();
    const oldTransaction = IDBDatabase.prototype.transaction;
    IDBDatabase.prototype.transaction = function(names, mode, ...args) {
      const tx = oldTransaction.call(this, names, mode, ...args);
      if (mode === 'readwrite' && names.includes(DOCUMENTS_STORE)) queueMicrotask(() => tx.abort());
      return tx;
    };
    let aborted;
    try {
      aborted = await store.write(first, 'default', 'manual', 'aborted', 4)
        .then(() => null, error => error.message);
    } finally {
      IDBDatabase.prototype.transaction = oldTransaction;
    }
    const completion = store.takeResult();
    const stillLatest = await store.open(first);
    const db = await new Promise((resolve, reject) => {
      const request = indexedDB.open(STORAGE_NAME);
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    const stores = Array.from(db.objectStoreNames);
    const expectedStores = [DOCUMENTS_STORE, LATEST_STORE];
    const pointer = await new Promise((resolve, reject) => {
      const request = db.transaction(LATEST_STORE).objectStore(LATEST_STORE).get(first);
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    const originalManual = await new Promise((resolve, reject) => {
      const request = db.transaction(DOCUMENTS_STORE).objectStore(DOCUMENTS_STORE)
        .get([first, 'default', 'manual']);
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    db.close();
    const idb = window.indexedDB;
    Object.defineProperty(window, 'indexedDB', { configurable: true, value: undefined });
    let unavailable;
    try { unavailable = await new BrowserSaveStore().open(first); }
    finally { Object.defineProperty(window, 'indexedDB', { configurable: true, value: idb }); }
    return { reloaded, isolated, openedBridge, drainedBridge, aborted, completion, stillLatest,
      stores, expectedStores, pointer, originalManual, unavailable };
  })()`);
  assert.equal(after.reloaded.latest, 'second');
  assert.equal(after.isolated.latest, 'other game');
  assert.equal(after.openedBridge.status, 'ready');
  assert.equal(after.openedBridge.latest, 'second');
  assert.equal(after.openedBridge.slot, 'default');
  assert.equal(after.openedBridge.lane, 'autosave');
  assert.equal(after.drainedBridge, null);
  assert.match(after.aborted, /abort|fail/i);
  assert.deepEqual(after.completion?.requestId, 4);
  assert.equal(after.completion.ok, false);
  assert.equal(after.stillLatest.latest, 'second');
  assert.deepEqual(after.stores, after.expectedStores);
  assert.deepEqual(after.pointer, { slot: 'default', lane: 'autosave' });
  assert.equal(after.originalManual, 'first');
  assert.equal(after.unavailable.status, 'unavailable');
  assert.equal(after.unavailable.latest, null);
});
