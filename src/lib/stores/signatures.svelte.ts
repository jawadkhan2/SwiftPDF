// Saved signatures, persisted locally in the browser's localStorage so they
// survive restarts without ever leaving the device (privacy-first). Each entry
// stores a PNG data URL plus a friendly label.

export interface SavedSignature {
  id: string;
  label: string;
  /** Full data URL, e.g. "data:image/png;base64,...". */
  dataUrl: string;
}

const KEY = "swiftpdf.signatures.v1";

function load(): SavedSignature[] {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

export const signatures = $state<{ list: SavedSignature[] }>({ list: load() });

function persist() {
  try {
    localStorage.setItem(KEY, JSON.stringify(signatures.list));
  } catch {
    // Storage full or unavailable — saved signatures just won't persist.
  }
}

export function addSignature(label: string, dataUrl: string): SavedSignature {
  const entry: SavedSignature = {
    id: `sig-${Date.now()}-${Math.floor(Math.random() * 1e6)}`,
    label: label.trim() || "Signature",
    dataUrl,
  };
  signatures.list = [entry, ...signatures.list];
  persist();
  return entry;
}

export function removeSignature(id: string) {
  signatures.list = signatures.list.filter((s) => s.id !== id);
  persist();
}
