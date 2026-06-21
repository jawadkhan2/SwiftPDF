// Thumbnail fetching with a shared LRU cache and a concurrency-limited queue.
//
// The backend renders pages on a single worker thread, so firing one IPC render
// per page on a thousands-of-page document would flood that thread and pile up
// thousands of base64 PNGs. Instead, callers (the lazy `Thumbnail` component) go
// through `getThumbnail`, which:
//   - serves cached results instantly (so re-scrolling is free),
//   - caps how many renders are in flight at once,
//   - and lets a caller abort a queued render it no longer needs (scrolled away).

import { renderThumbnail, toDataUrl } from "$lib/api";

const MAX_CONCURRENT = 6;
const CACHE_CAP = 400; // ~ most-recent thumbnails kept in memory

// key -> data URL. Insertion order is the LRU order; a cache hit re-inserts to
// mark it most-recent.
const cache = new Map<string, string>();

interface QueueItem {
  key: string;
  docId: string;
  page: number;
  size: number;
  resolve: (url: string) => void;
  reject: (err: unknown) => void;
  signal?: AbortSignal;
}

const queue: QueueItem[] = [];
let active = 0;

const keyOf = (docId: string, page: number, size: number) =>
  `${docId}:${page}:${size}`;

const abortError = () =>
  typeof DOMException === "function"
    ? new DOMException("Aborted", "AbortError")
    : Object.assign(new Error("Aborted"), { name: "AbortError" });

function cacheGet(key: string): string | undefined {
  const v = cache.get(key);
  if (v !== undefined) {
    cache.delete(key);
    cache.set(key, v); // mark most-recently used
  }
  return v;
}

function cacheSet(key: string, url: string): void {
  cache.set(key, url);
  while (cache.size > CACHE_CAP) {
    const oldest = cache.keys().next().value;
    if (oldest === undefined) break;
    cache.delete(oldest);
  }
}

function pump(): void {
  while (active < MAX_CONCURRENT && queue.length > 0) {
    const item = queue.shift()!;

    if (item.signal?.aborted) {
      item.reject(abortError());
      continue;
    }
    // Another entry may have filled the cache while this one waited.
    const cached = cacheGet(item.key);
    if (cached !== undefined) {
      item.resolve(cached);
      continue;
    }

    active++;
    renderThumbnail(item.docId, item.page, item.size)
      .then((r) => {
        const url = toDataUrl(r);
        cacheSet(item.key, url);
        item.resolve(url);
      })
      .catch((e) => item.reject(e))
      .finally(() => {
        active--;
        pump();
      });
  }
}

/**
 * Resolve to a data-URL thumbnail for one page, using the cache and the shared
 * render queue. Pass an `AbortSignal` to drop the request if it's still queued
 * when the caller no longer needs it (e.g. the page scrolled out of view).
 */
export function getThumbnail(
  docId: string,
  page: number,
  size: number,
  signal?: AbortSignal,
): Promise<string> {
  const key = keyOf(docId, page, size);

  const cached = cacheGet(key);
  if (cached !== undefined) return Promise.resolve(cached);

  return new Promise<string>((resolve, reject) => {
    if (signal?.aborted) {
      reject(abortError());
      return;
    }
    queue.push({ key, docId, page, size, resolve, reject, signal });
    pump();
  });
}

/** Drop every cached thumbnail for a document (e.g. when it's closed). */
export function clearDocThumbnails(docId: string): void {
  const prefix = `${docId}:`;
  for (const k of [...cache.keys()]) {
    if (k.startsWith(prefix)) cache.delete(k);
  }
}
