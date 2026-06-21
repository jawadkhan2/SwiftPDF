// Typed wrappers over the Rust backend. One function per Tauri command, plus
// small helpers for native dialogs. Keeping all `invoke` calls here means the
// rest of the UI never touches stringly-typed command names.

import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

export interface PageInfo {
  width_pt: number;
  height_pt: number;
  rotation: number;
}

export interface OpenResult {
  doc_id: string;
  source_name: string;
  page_count: number;
  pages: PageInfo[];
  has_form: boolean;
}

export interface RenderResult {
  png_base64: string;
  width_px: number;
  height_px: number;
  width_pt: number;
  height_pt: number;
}

/** Convert a RenderResult into a data URL usable as an <img> src. */
export function toDataUrl(r: RenderResult): string {
  return `data:image/png;base64,${r.png_base64}`;
}

/** Show the native "open file" dialog, restricted to PDFs. Returns a path or null. */
export async function pickPdf(): Promise<string | null> {
  const selected = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  return typeof selected === "string" ? selected : null;
}

/** Show the native dialog allowing multiple PDFs (for merge). */
export async function pickPdfs(): Promise<string[]> {
  const selected = await open({
    multiple: true,
    directory: false,
    filters: [{ name: "PDF", extensions: ["pdf"] }],
  });
  if (Array.isArray(selected)) return selected;
  if (typeof selected === "string") return [selected];
  return [];
}

export function openPdf(path: string): Promise<OpenResult> {
  return invoke("open_pdf", { path });
}

export function renderThumbnail(
  docId: string,
  page: number,
  size = 240,
): Promise<RenderResult> {
  return invoke("render_thumbnail", { docId, page, size });
}

export function renderPage(
  docId: string,
  page: number,
  size = 1400,
): Promise<RenderResult> {
  return invoke("render_page", { docId, page, size });
}

export interface PagePlan {
  page: number;
  rotation: number;
}

/**
 * Build a new PDF from `plan` and prompt for a save location. Returns the saved
 * path, or null if the user cancelled. Never modifies the original file.
 */
export function saveBuiltPdf(
  docId: string,
  plan: PagePlan[],
  suggestedName: string,
): Promise<string | null> {
  return invoke("save_built_pdf", { docId, plan, suggestedName });
}

/** Turn `Contract.pdf` into `Contract (edited).pdf`. */
export function editedName(sourceName: string): string {
  const dot = sourceName.toLowerCase().endsWith(".pdf")
    ? sourceName.length - 4
    : sourceName.length;
  return `${sourceName.slice(0, dot)} (edited).pdf`;
}

/** Merge several PDFs (in order) into one new in-memory doc; returns its metadata. */
export function mergePdfs(paths: string[]): Promise<OpenResult> {
  return invoke("merge_pdfs", { paths });
}

/**
 * Split/extract: each group of page indices becomes one output file. Prompts for
 * a folder; returns the saved paths (empty if cancelled).
 */
export function splitPdf(
  docId: string,
  groups: number[][],
  baseName: string,
): Promise<string[]> {
  return invoke("split_pdf", { docId, groups, baseName });
}

/**
 * A Fill & Sign item. Positions/sizes are fractions of the page box (0..1,
 * top-left origin) so they survive any zoom level. Mirrors the Rust `Stamp`
 * enum: the `kind` tag selects which fields apply.
 */
export type Stamp =
  | {
      kind: "Text";
      page: number;
      fx: number;
      fy: number;
      fh: number;
      text: string;
      color: [number, number, number];
    }
  | {
      kind: "Image";
      page: number;
      fx: number;
      fy: number;
      fw: number;
      fh: number;
      png_base64: string;
    };

/**
 * Stamp the given items onto the document and prompt for a save location.
 * Returns the saved path, or null if cancelled. Never modifies the original.
 */
export function saveSignedPdf(
  docId: string,
  stamps: Stamp[],
  suggestedName: string,
): Promise<string | null> {
  return invoke("save_signed_pdf", { docId, stamps, suggestedName });
}

export function closeDoc(docId: string): Promise<void> {
  return invoke("close_doc", { docId });
}
