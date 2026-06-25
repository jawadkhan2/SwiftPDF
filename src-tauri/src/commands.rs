//! Tauri command handlers. These are thin: they validate input, then dispatch
//! a closure to the PDF worker and return its result.

use crate::pdf::dto::{OpenResult, PagePlan, RenderResult, Stamp};
use crate::pdf::worker::DocEntry;
use crate::pdf::{document, pages, render, stamp, PdfWorker};
use std::path::PathBuf;
use tauri::State;
use uuid::Uuid;

/// Open a PDF from disk. PDFium streams pages from the file lazily, so even a
/// thousands-of-page document opens without reading the whole file into memory.
/// The original file is never modified.
#[tauri::command]
pub fn open_pdf(worker: State<PdfWorker>, path: String) -> Result<OpenResult, String> {
    let path_buf = PathBuf::from(&path);
    let source_name = path_buf
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document.pdf".to_string());

    worker.call(move |state| {
        let doc = state
            .pdfium
            .load_pdf_from_file(&path_buf, None)
            .map_err(|e| format!("Could not open PDF: {e}"))?;
        register_doc(state, doc, source_name, Some(path_buf))
    })
}

/// Merge several PDFs (in the given order) into one new in-memory document, then
/// return it like a freshly opened file so the user can review/reorder/save it.
#[tauri::command]
pub fn merge_pdfs(worker: State<PdfWorker>, paths: Vec<String>) -> Result<OpenResult, String> {
    if paths.is_empty() {
        return Err("Pick at least one PDF to merge.".to_string());
    }

    let mut sources = Vec::with_capacity(paths.len());
    for p in &paths {
        sources
            .push(std::fs::read(p).map_err(|e| format!("Could not read {p}: {e}"))?);
    }

    worker.call(move |state| {
        let merged = pages::merge_documents(state.pdfium, &sources)?;
        // The merge result has no backing file; keep it in memory (the document
        // takes ownership of the bytes) with no source path.
        let doc = state
            .pdfium
            .load_pdf_from_byte_vec(merged, None)
            .map_err(|e| format!("Could not open merged PDF: {e}"))?;
        register_doc(state, doc, "Merged.pdf".to_string(), None)
    })
}

/// Register an already-open document under a fresh id and return its metadata.
/// Shared by `open_pdf` and `merge_pdfs`.
fn register_doc(
    state: &mut crate::pdf::worker::WorkerState,
    doc: pdfium_render::prelude::PdfDocument<'static>,
    source_name: String,
    source_path: Option<PathBuf>,
) -> Result<OpenResult, String> {
    let doc_id = Uuid::new_v4().to_string();
    let meta = document::metadata(&doc, doc_id.clone(), source_name)?;
    state.docs.insert(doc_id, DocEntry { doc, source_path });
    Ok(meta)
}

/// Render a thumbnail (longest edge `size` px) for one page.
#[tauri::command]
pub fn render_thumbnail(
    worker: State<PdfWorker>,
    doc_id: String,
    page: usize,
    size: u32,
) -> Result<RenderResult, String> {
    worker.call(move |state| {
        let entry = state.entry(&doc_id)?;
        render::render_page(&entry.doc, page, size)
    })
}

/// Render a page at higher resolution for the editor canvas.
#[tauri::command]
pub fn render_page(
    worker: State<PdfWorker>,
    doc_id: String,
    page: usize,
    size: u32,
) -> Result<RenderResult, String> {
    worker.call(move |state| {
        let entry = state.entry(&doc_id)?;
        render::render_page(&entry.doc, page, size)
    })
}

/// Build an output PDF from a page plan (reorder/delete/rotate already decided
/// in the UI) and write a NEW file to the path chosen by the frontend dialog.
/// The original is never touched.
#[tauri::command]
pub fn save_built_pdf(
    worker: State<PdfWorker>,
    doc_id: String,
    plan: Vec<PagePlan>,
    path: String,
) -> Result<Option<String>, String> {
    // 1. Assemble the bytes on the worker thread.
    let bytes = worker.call(move |state| {
        let entry = state.entry(&doc_id)?;
        pages::build_document(state.pdfium, &entry.doc, &plan)
    })?;

    // 2. Write the new file to the path chosen by the frontend dialog.
    write_pdf(path, &bytes).map(Some)
}

/// Fill & Sign: stamp the given text/signature items onto the document, then save
/// a new copy to the path chosen by the frontend dialog. The original file is
/// never modified.
#[tauri::command]
pub fn save_signed_pdf(
    worker: State<PdfWorker>,
    doc_id: String,
    stamps: Vec<Stamp>,
    path: String,
) -> Result<Option<String>, String> {
    if stamps.is_empty() {
        return Err("Add some text or a signature first.".to_string());
    }

    let bytes = worker.call(move |state| {
        // Stamp a fresh copy so the cached pristine document is never mutated.
        let mut doc = state.fresh_doc(&doc_id)?;
        stamp::apply_stamps(&mut doc, &stamps)
    })?;

    write_pdf(path, &bytes).map(Some)
}

/// Split/extract: each inner list of page indices becomes one output PDF. Writes
/// to the folder chosen by the frontend dialog as `base_name (1).pdf`, `(2)` ...
#[tauri::command]
pub fn split_pdf(
    worker: State<PdfWorker>,
    doc_id: String,
    groups: Vec<Vec<usize>>,
    base_name: String,
    folder: String,
) -> Result<Vec<String>, String> {
    if groups.is_empty() {
        return Err("Nothing to split — choose at least one group of pages.".to_string());
    }

    // Build all output blobs on the worker first.
    let blobs = worker.call(move |state| {
        let entry = state.entry(&doc_id)?;
        let mut out = Vec::with_capacity(groups.len());
        for group in &groups {
            out.push(pages::extract_pages(state.pdfium, &entry.doc, group)?);
        }
        Ok(out)
    })?;

    let dir = PathBuf::from(folder);

    let stem = base_name.trim_end_matches(".pdf");
    let mut saved = Vec::with_capacity(blobs.len());
    for (i, bytes) in blobs.iter().enumerate() {
        let name = if blobs.len() == 1 {
            format!("{stem}.pdf")
        } else {
            format!("{stem} ({}).pdf", i + 1)
        };
        let path = dir.join(name);
        std::fs::write(&path, bytes).map_err(|e| format!("Could not save file: {e}"))?;
        saved.push(path.to_string_lossy().into_owned());
    }
    Ok(saved)
}

fn write_pdf(path: String, bytes: &[u8]) -> Result<String, String> {
    let path = PathBuf::from(path);
    std::fs::write(&path, bytes).map_err(|e| format!("Could not save file: {e}"))?;
    Ok(path.to_string_lossy().into_owned())
}

/// Drop a document from the registry to free memory.
#[tauri::command]
pub fn close_doc(worker: State<PdfWorker>, doc_id: String) -> Result<(), String> {
    worker.call(move |state| {
        state.docs.remove(&doc_id);
        Ok(())
    })
}
