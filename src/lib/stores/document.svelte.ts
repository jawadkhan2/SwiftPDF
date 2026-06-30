// Shared reactive state for the currently open document. Svelte 5 runes work in
// `.svelte.ts` modules, so any route can read/update `doc.current` reactively.

import { closeDoc, type OpenResult } from "$lib/api";
import { clearDocThumbnails } from "$lib/thumbnails";
import { viewer } from "$lib/stores/viewer.svelte";

export const doc = $state<{ current: OpenResult | null }>({ current: null });

async function disposeDoc(docId: string | null | undefined) {
  if (!docId) return;
  clearDocThumbnails(docId);
  try {
    await closeDoc(docId);
  } catch {
    // The UI can continue; the backend may already have dropped the document.
  }
}

export async function setDoc(result: OpenResult) {
  const previousId = doc.current?.doc_id;
  doc.current = result;
  if (previousId !== result.doc_id) {
    viewer.page = 0;
    await disposeDoc(previousId);
  }
}

export async function clearDoc() {
  const previousId = doc.current?.doc_id;
  doc.current = null;
  await disposeDoc(previousId);
}
