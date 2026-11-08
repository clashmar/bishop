export const STORAGE_NAME = 'bishop-runtime-saves';
export const DOCUMENTS_STORE = 'documents';
export const LATEST_STORE = 'latest';

export class BrowserSaveStore {
  #databasePromise;
  #latest = new Map();
  #openResult = null;
  #writeTail = new Map();
  #completed = [];

  async #database() {
    this.#databasePromise ??= new Promise((resolve, reject) => {
      if (!globalThis.indexedDB) {
        reject(new Error('IndexedDB is unavailable'));
        return;
      }
      const request = indexedDB.open(STORAGE_NAME, 1);
      request.onupgradeneeded = () => {
        const db = request.result;
        db.createObjectStore(DOCUMENTS_STORE);
        db.createObjectStore(LATEST_STORE);
      };
      request.onsuccess = () => {
        request.result.onversionchange = () => request.result.close();
        resolve(request.result);
      };
      request.onerror = () => reject(request.error ?? new Error('IndexedDB open failed'));
      request.onblocked = () => reject(new Error('IndexedDB open blocked'));
    });
    return this.#databasePromise;
  }

  async open(gameId) {
    try {
      const db = await this.#database();
      const { document, selected } = await new Promise((resolve, reject) => {
        const tx = db.transaction([LATEST_STORE, DOCUMENTS_STORE], 'readonly');
        let document = null;
        let selected = null;
        const pointer = tx.objectStore(LATEST_STORE).get(gameId);
        pointer.onsuccess = () => {
          selected = pointer.result ?? null;
          if (!selected) return;
          const { slot, lane } = selected;
          const saved = tx.objectStore(DOCUMENTS_STORE).get([gameId, slot, lane]);
          saved.onsuccess = () => { document = saved.result ?? null; };
        };
        tx.oncomplete = () => {
          if (selected && document === null) {
            reject(new Error('latest save document is missing'));
          } else {
            resolve({ document, selected });
          }
        };
        tx.onabort = () => reject(tx.error ?? new Error('IndexedDB read aborted'));
        tx.onerror = () => reject(tx.error ?? new Error('IndexedDB read failed'));
      });
      this.#latest.set(gameId, document);
      return { status: 'ready', latest: document,
        slot: selected?.slot ?? null, lane: selected?.lane ?? null };
    } catch (error) {
      this.#latest.delete(gameId);
      return { status: 'unavailable', latest: null, error: String(error?.message ?? error) };
    }
  }

  beginOpen(gameId) {
    this.#openResult = null;
    void this.open(gameId).then(result => { this.#openResult = result; });
  }

  takeOpenResult() {
    const result = this.#openResult;
    this.#openResult = null;
    return result;
  }

  latest(gameId) {
    return this.#latest.get(gameId) ?? null;
  }

  write(gameId, slot, lane, ron, requestId) {
    const previous = this.#writeTail.get(gameId) ?? Promise.resolve();
    const write = previous.catch(() => {}).then(() => this.#commit(gameId, slot, lane, ron));
    this.#writeTail.set(gameId, write);
    void write.then(
      () => {
        this.#completed.push({ requestId, ok: true });
        if (this.#writeTail.get(gameId) === write) this.#writeTail.delete(gameId);
      },
      error => {
        this.#completed.push({ requestId, ok: false, error: String(error?.message ?? error) });
        if (this.#writeTail.get(gameId) === write) this.#writeTail.delete(gameId);
      },
    );
    return write;
  }

  async #commit(gameId, slot, lane, ron) {
    const db = await this.#database();
    const tx = db.transaction([DOCUMENTS_STORE, LATEST_STORE], 'readwrite');
    const completion = new Promise((resolve, reject) => {
      tx.oncomplete = resolve;
      tx.onabort = () => reject(tx.error ?? new Error('save transaction aborted'));
      tx.onerror = () => reject(tx.error ?? new Error('save transaction failed'));
    });
    try {
      tx.objectStore(DOCUMENTS_STORE).put(ron, [gameId, slot, lane]);
      tx.objectStore(LATEST_STORE).put({ slot, lane }, gameId);
    } catch (error) {
      tx.abort();
      await completion.catch(() => {});
      throw error;
    }
    await completion;
    this.#latest.set(gameId, ron);
  }

  takeResult() {
    return this.#completed.shift() ?? null;
  }
}

export const browserSaveStore = new BrowserSaveStore();
