//! Tauri command handlers. These are thin: they validate input, then dispatch
//! a closure to the PDF worker and return its result.

use crate::pdf::dto::{OpenResult, PagePlan, RenderResult, Stamp};
use crate::pdf::worker::DocEntry;
use crate::pdf::{document, pages, render, stamp, PdfWorker};
use std::io::Write;
use std::path::{Path, PathBuf};
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
        sources.push(std::fs::read(p).map_err(|e| format!("Could not read {p}: {e}"))?);
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
    let (bytes, source_path) = worker.call(move |state| {
        let entry = state.entry(&doc_id)?;
        let source_path = entry.source_path.clone();
        let bytes = pages::build_document(state.pdfium, &entry.doc, &plan)?;
        Ok((bytes, source_path))
    })?;

    // 2. Write the new file to the path chosen by the frontend dialog.
    write_pdf(path, &bytes, source_path.as_deref()).map(Some)
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

    let (bytes, source_path) = worker.call(move |state| {
        let entry = state.entry(&doc_id)?;
        let source_path = entry.source_path.clone();
        let snapshot = entry
            .doc
            .save_to_bytes()
            .map_err(|e| format!("Could not snapshot document: {e}"))?;
        let mut doc = state
            .pdfium
            .load_pdf_from_byte_vec(snapshot, None)
            .map_err(|e| format!("Could not prepare document for signing: {e}"))?;
        let bytes = stamp::apply_stamps(&mut doc, &stamps)?;
        Ok((bytes, source_path))
    })?;

    write_pdf(path, &bytes, source_path.as_deref()).map(Some)
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
    let (blobs, source_path) = worker.call(move |state| {
        let entry = state.entry(&doc_id)?;
        let source_path = entry.source_path.clone();
        let mut out = Vec::with_capacity(groups.len());
        for group in &groups {
            out.push(pages::extract_pages(state.pdfium, &entry.doc, group)?);
        }
        Ok((out, source_path))
    })?;

    let dir = PathBuf::from(folder);
    if !dir.is_dir() {
        return Err("Choose an existing folder for the split files.".to_string());
    }

    let stem = safe_pdf_stem(&base_name);
    let mut saved = Vec::with_capacity(blobs.len());
    for (i, bytes) in blobs.iter().enumerate() {
        let base = if blobs.len() == 1 {
            stem.clone()
        } else {
            format!("{stem} ({})", i + 1)
        };
        let path = unique_pdf_path(&dir, &base);
        write_pdf_file(&path, bytes, source_path.as_deref())?;
        saved.push(path.to_string_lossy().into_owned());
    }
    Ok(saved)
}

fn write_pdf(path: String, bytes: &[u8], source_path: Option<&Path>) -> Result<String, String> {
    let path = PathBuf::from(path);
    write_pdf_file(&path, bytes, source_path)?;
    Ok(path.to_string_lossy().into_owned())
}

fn write_pdf_file(path: &Path, bytes: &[u8], source_path: Option<&Path>) -> Result<(), String> {
    ensure_not_source(path, source_path)?;
    if path.exists() {
        return Err(format!(
            "{} already exists. Choose a new file name so your files are not overwritten.",
            path.display()
        ));
    }

    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| "Choose a folder to save the PDF.".to_string())?;
    if !parent.is_dir() {
        return Err(format!("Save folder does not exist: {}", parent.display()));
    }

    let file_name = path
        .file_name()
        .ok_or_else(|| "Choose a file name to save the PDF.".to_string())?
        .to_string_lossy();
    let tmp_path = parent.join(format!(".{file_name}.{}.tmp", Uuid::new_v4()));

    let write_result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)
            .map_err(|e| format!("Could not create temporary file: {e}"))?;
        file.write_all(bytes)
            .map_err(|e| format!("Could not write PDF: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("Could not finish saving PDF: {e}"))?;
        std::fs::rename(&tmp_path, path).map_err(|e| format!("Could not save file: {e}"))
    })();

    if write_result.is_err() {
        let _ = std::fs::remove_file(&tmp_path);
    }

    write_result
}

fn ensure_not_source(path: &Path, source_path: Option<&Path>) -> Result<(), String> {
    let Some(source) = source_path else {
        return Ok(());
    };

    let same_existing_path = path.exists()
        && source.exists()
        && std::fs::canonicalize(path).ok() == std::fs::canonicalize(source).ok();
    if same_existing_path {
        return Err(
            "Choose a different save location so the original PDF is not changed.".to_string(),
        );
    }

    Ok(())
}

fn unique_pdf_path(dir: &Path, stem: &str) -> PathBuf {
    let mut candidate = dir.join(format!("{stem}.pdf"));
    let mut copy = 2;
    while candidate.exists() {
        candidate = dir.join(format!("{stem} ({copy}).pdf"));
        copy += 1;
    }
    candidate
}

fn safe_pdf_stem(base_name: &str) -> String {
    let path = Path::new(base_name.trim());
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document");
    let stem = file_name
        .strip_suffix(".pdf")
        .or_else(|| file_name.strip_suffix(".PDF"))
        .unwrap_or(file_name)
        .trim();

    let cleaned: String = stem
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let cleaned = cleaned.trim_matches(&[' ', '.'][..]);
    if cleaned.is_empty() {
        "document".to_string()
    } else {
        cleaned.to_string()
    }
}

/// Drop a document from the registry to free memory.
#[tauri::command]
pub fn close_doc(worker: State<PdfWorker>, doc_id: String) -> Result<(), String> {
    worker.call(move |state| {
        state.docs.remove(&doc_id);
        Ok(())
    })
}
