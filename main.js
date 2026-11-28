import init, {
  seed_web_state,
  seed_web_book,
  start_bookworm_web,
  import_web_epub,
  update_web_safe_area,
} from './pkg/bookworm_mobile.js';

const DB_NAME = 'bookworm-pwa';
const DB_VERSION = 1;
const STORE_KV = 'kv';
const STORE_BOOKS = 'books';

function openDatabase() {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION);
    req.onupgradeneeded = () => {
      const db = req.result;
      if (!db.objectStoreNames.contains(STORE_KV)) {
        db.createObjectStore(STORE_KV);
      }
      if (!db.objectStoreNames.contains(STORE_BOOKS)) {
        db.createObjectStore(STORE_BOOKS);
      }
    };
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error || new Error('Failed to open IndexedDB'));
  });
}

function idbGet(db, storeName, key) {
  return new Promise((resolve, reject) => {
    const tx = db.transaction(storeName, 'readonly');
    const store = tx.objectStore(storeName);
    const req = store.get(key);
    req.onsuccess = () => resolve(req.result ?? null);
    req.onerror = () => reject(req.error);
  });
}

function idbGetAllBooks(db) {
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_BOOKS, 'readonly');
    const store = tx.objectStore(STORE_BOOKS);
    const entries = [];
    const req = store.openCursor();
    req.onsuccess = () => {
      const cursor = req.result;
      if (cursor) {
        entries.push([String(cursor.key), cursor.value]);
        cursor.continue();
      } else {
        resolve(entries);
      }
    };
    req.onerror = () => reject(req.error);
  });
}

function idbPut(dbPromise, storeName, key, value) {
  return dbPromise
    .then(
      (db) =>
        new Promise((resolve, reject) => {
          const tx = db.transaction(storeName, 'readwrite');
          tx.objectStore(storeName).put(value, key);
          tx.oncomplete = () => resolve();
          tx.onerror = () => reject(tx.error);
        })
    )
    .catch((err) => {
      console.error('[bookworm-pwa] IndexedDB put failed:', storeName, key, err);
    });
}

function idbDelete(dbPromise, storeName, key) {
  return dbPromise
    .then(
      (db) =>
        new Promise((resolve, reject) => {
          const tx = db.transaction(storeName, 'readwrite');
          tx.objectStore(storeName).delete(key);
          tx.oncomplete = () => resolve();
          tx.onerror = () => reject(tx.error);
        })
    )
    .catch((err) => {
      console.error('[bookworm-pwa] IndexedDB delete failed:', storeName, key, err);
    });
}

function readSafeAreaInsets() {
  const probe = document.getElementById('safe-area-probe');
  if (!probe) {
    return { top: 0, bottom: 0, left: 0, right: 0 };
  }
  const style = getComputedStyle(probe);
  return {
    top: parseFloat(style.paddingTop) || 0,
    bottom: parseFloat(style.paddingBottom) || 0,
    left: parseFloat(style.paddingLeft) || 0,
    right: parseFloat(style.paddingRight) || 0,
  };
}

function installTouchInputGuard() {
  const touchOnly =
    window.matchMedia &&
    window.matchMedia('(hover: none) and (pointer: coarse)').matches;
  if (!touchOnly) return;

  const observer = new MutationObserver((mutations) => {
    for (const mutation of mutations) {
      for (const node of mutation.addedNodes) {
        if (
          node instanceof HTMLInputElement &&
          node.id !== 'epub-picker' &&
          node.parentNode === document.body
        ) {
          node.readOnly = true;
          node.setAttribute('inputmode', 'none');
        }
      }
    }
  });
  observer.observe(document.body, { childList: true });
}

function registerServiceWorker() {
  if ('serviceWorker' in navigator) {
    navigator.serviceWorker.register('./sw.js').catch((err) => {
      console.warn('[bookworm-pwa] Service worker registration failed:', err);
    });
  }
}

function setBootError(message) {
  const spinner = document.getElementById('boot-spinner');
  const status = document.getElementById('boot-status');
  if (spinner) spinner.style.display = 'none';
  if (status) {
    status.textContent = message;
    status.classList.add('error');
  }
}

async function boot() {
  registerServiceWorker();
  installTouchInputGuard();

  const dbPromise = openDatabase();
  const picker = document.getElementById('epub-picker');

  // Copy WASM slice views synchronously before entering async IDB transactions.
  window.__bookwormBridge = {
    pickEpubFiles() {
      if (picker) {
        picker.click();
      }
    },
    saveBook(key, bytes) {
      const copy = new Uint8Array(bytes);
      idbPut(dbPromise, STORE_BOOKS, String(key), copy);
    },
    deleteBook(key) {
      idbDelete(dbPromise, STORE_BOOKS, String(key));
    },
    saveLibraryJson(json) {
      idbPut(dbPromise, STORE_KV, 'library_json', String(json));
    },
    saveConfigJson(json) {
      idbPut(dbPromise, STORE_KV, 'config_json', String(json));
    },
    downloadText(filename, content) {
      const blob = new Blob([String(content)], { type: 'text/markdown;charset=utf-8' });
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = String(filename);
      document.body.appendChild(link);
      link.click();
      link.remove();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    },
    onReady() {
      const splash = document.getElementById('boot-splash');
      if (splash) splash.classList.add('hidden');
    },
    onError(message) {
      setBootError(String(message));
    },
  };

  if (navigator.storage && navigator.storage.persist) {
    navigator.storage.persist().catch(() => {});
  }

  try {
    const db = await dbPromise;
    const [libraryJson, configJson, books] = await Promise.all([
      idbGet(db, STORE_KV, 'library_json'),
      idbGet(db, STORE_KV, 'config_json'),
      idbGetAllBooks(db),
    ]);

    const status = document.getElementById('boot-status');
    if (status) status.textContent = 'Starting reader engine…';

    await init();

    seed_web_state(
      typeof libraryJson === 'string' ? libraryJson : null,
      typeof configJson === 'string' ? configJson : null
    );

    for (const [key, value] of books) {
      if (value instanceof Uint8Array) {
        seed_web_book(key, value);
      } else if (value instanceof ArrayBuffer) {
        seed_web_book(key, new Uint8Array(value));
      } else if (ArrayBuffer.isView(value)) {
        seed_web_book(key, new Uint8Array(value.buffer, value.byteOffset, value.byteLength));
      }
    }

    if (picker) {
      picker.addEventListener('change', async () => {
        const files = picker.files ? Array.from(picker.files) : [];
        for (const file of files) {
          try {
            const buf = await file.arrayBuffer();
            import_web_epub(file.name, new Uint8Array(buf));
          } catch (err) {
            console.error('[bookworm-pwa] Failed to import EPUB:', file.name, err);
          }
        }
        picker.value = '';
      });
    }

    const syncSafeArea = () => {
      const { top, bottom, left, right } = readSafeAreaInsets();
      update_web_safe_area(top, bottom, left, right);
    };
    window.addEventListener('resize', syncSafeArea);
    window.addEventListener('orientationchange', syncSafeArea);
    if (window.visualViewport) {
      window.visualViewport.addEventListener('resize', syncSafeArea);
    }

    const initialInsets = readSafeAreaInsets();
    await start_bookworm_web(
      initialInsets.top,
      initialInsets.bottom,
      initialInsets.left,
      initialInsets.right
    );
  } catch (err) {
    console.error('[bookworm-pwa] Boot failed:', err);
    setBootError(
      'Could not start Bookworm in this browser (WebGPU/WebGL2 required). ' +
        (err && err.message ? err.message : String(err))
    );
  }
}

boot();
