// Shared reactive state for the currently open document. Svelte 5 runes work in
// `.svelte.ts` modules, so any route can read/update `doc.current` reactively.

import type { OpenResult } from "$lib/api";

export const doc = $state<{ current: OpenResult | null }>({ current: null });

export function setDoc(result: OpenResult) {
  doc.current = result;
}

export function clearDoc() {
  doc.current = null;
}
