//! The single-threaded PDF worker.
//!
//! All PDFium work happens on one owned thread. Tauri command handlers (which
//! run on arbitrary pool threads) submit closures over a channel; each closure
//! receives `&mut WorkerState` and runs to completion on the worker thread. A
//! per-call reply channel carries the result back to the caller.

use super::engine::init_pdfium;
use pdfium_render::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Sender};
use std::thread;

/// One open document (see module docs in `mod.rs`).
///
/// `doc` is a live, lazily-loaded `PdfDocument` we render and read from directly
/// — for disk files PDFium streams pages on demand, so a thousands-of-page file
/// never has to live in memory all at once. `doc` is treated as pristine and is
/// never mutated; mutation operations snapshot the opened document bytes and
/// work on that copy so saves always match the preview.
pub struct DocEntry {
    pub doc: PdfDocument<'static>,
    /// `Some` for files opened from disk; used only to prevent saving a copy
    /// over the original source file. `None` for in-memory documents such as
    /// merge results.
    pub source_path: Option<PathBuf>,
}

/// State owned exclusively by the worker thread.
///
/// `pdfium` is leaked to `'static` at startup (it lives for the whole program),
/// which lets each [`DocEntry`] hold a `PdfDocument<'static>` borrowing it.
pub struct WorkerState {
    pub pdfium: &'static Pdfium,
    pub docs: HashMap<String, DocEntry>,
}

impl WorkerState {
    /// Borrow an entry or return a friendly error.
    pub fn entry(&self, doc_id: &str) -> Result<&DocEntry, String> {
        self.docs
            .get(doc_id)
            .ok_or_else(|| format!("Unknown document '{doc_id}' (was it closed?)"))
    }

    /// Load a fresh, independent copy of a document for mutation, leaving the
    /// cached pristine `doc` untouched.
    #[allow(dead_code)]
    pub fn fresh_doc(&self, doc_id: &str) -> Result<PdfDocument<'static>, String> {
        let entry = self.entry(doc_id)?;
        match &entry.source_path {
            Some(path) => self
                .pdfium
                .load_pdf_from_file(path, None)
                .map_err(|e| format!("Could not reopen PDF: {e}")),
            None => {
                let bytes = entry
                    .doc
                    .save_to_bytes()
                    .map_err(|e| format!("Could not snapshot document: {e}"))?;
                self.pdfium
                    .load_pdf_from_byte_vec(bytes, None)
                    .map_err(|e| format!("Could not reopen document: {e}"))
            }
        }
    }
}

type Job = Box<dyn FnOnce(&mut WorkerState) + Send>;

/// Handle stored in Tauri state. Cloneable, `Send + Sync`.
#[derive(Clone)]
pub struct PdfWorker {
    sender: Option<Sender<Job>>,
    startup_error: Option<String>,
}

impl PdfWorker {
    /// Spawn the worker thread. `resource_dir` is the Tauri resource directory
    /// used to locate the bundled PDFium library.
    pub fn start(resource_dir: Option<PathBuf>) -> Self {
        // Initialize PDFium on a probe to surface load errors before spawning.
        let (ready_tx, ready_rx) = channel::<Result<(), String>>();
        let (job_tx, job_rx) = channel::<Job>();

        if let Err(e) = thread::Builder::new()
            .name("pdf-worker".into())
            .spawn(move || {
                let pdfium = match init_pdfium(resource_dir) {
                    Ok(p) => p,
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                        return;
                    }
                };
                // Leak to `'static`: the worker (and thus PDFium) lives for the
                // whole program, so every cached `PdfDocument` can borrow it.
                let pdfium: &'static Pdfium = Box::leak(Box::new(pdfium));
                let _ = ready_tx.send(Ok(()));

                let mut state = WorkerState {
                    pdfium,
                    docs: HashMap::new(),
                };

                // Run jobs until all senders drop (app shutdown).
                while let Ok(job) = job_rx.recv() {
                    job(&mut state);
                }
            })
        {
            return PdfWorker::unavailable(format!("Failed to spawn PDF worker: {e}"));
        }

        match ready_rx.recv() {
            Ok(Ok(())) => PdfWorker {
                sender: Some(job_tx),
                startup_error: None,
            },
            Ok(Err(e)) => PdfWorker::unavailable(e),
            Err(e) => PdfWorker::unavailable(format!("PDF worker died on startup: {e}")),
        }
    }

    fn unavailable(error: String) -> Self {
        let message = format!("PDF engine could not start: {error}");
        eprintln!("{message}");
        PdfWorker {
            sender: None,
            startup_error: Some(message),
        }
    }

    /// Run `f` on the worker thread and block for its result.
    pub fn call<T, F>(&self, f: F) -> Result<T, String>
    where
        T: Send + 'static,
        F: FnOnce(&mut WorkerState) -> Result<T, String> + Send + 'static,
    {
        let sender = self.sender.as_ref().ok_or_else(|| {
            self.startup_error
                .clone()
                .unwrap_or_else(|| "PDF worker is not running".to_string())
        })?;
        let (tx, rx) = channel::<Result<T, String>>();
        sender
            .send(Box::new(move |state| {
                let _ = tx.send(f(state));
            }))
            .map_err(|_| "PDF worker is not running".to_string())?;
        rx.recv()
            .map_err(|_| "PDF worker dropped the request".to_string())?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_worker_returns_startup_error() {
        let worker = PdfWorker {
            sender: None,
            startup_error: Some("PDF engine could not start: missing library".to_string()),
        };

        let err = worker.call(|_| Ok::<(), String>(())).unwrap_err();

        assert!(err.contains("missing library"));
    }
}
