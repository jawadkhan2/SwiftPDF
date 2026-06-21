//! PDF engine: a single dedicated worker thread owns the one `Pdfium` instance.
//!
//! `pdfium-render`'s objects are neither `Send` nor thread-safe, and a
//! `PdfDocument` borrows the `Pdfium` that created it. Because the worker thread
//! lives for the whole program, we leak the `Pdfium` to `'static` once at
//! startup; each open file is then kept as a live `PdfDocument<'static>` in
//! `DocEntry`. Disk files are opened with `load_pdf_from_file`, so PDFium streams
//! pages lazily and a thousands-of-page document never has to be read into memory
//! all at once — and rendering a page no longer re-parses the whole file.
//!
//! The cached document is treated as read-only/pristine: rendering and structural
//! copies (merge/split/build) read from it, while the one mutating operation
//! (stamping) works on a fresh copy (`WorkerState::fresh_doc`) so repeated saves
//! never compound.

pub mod dto;
pub mod engine;
pub mod document;
pub mod pages;
pub mod render;
pub mod stamp;
pub mod worker;

#[cfg(test)]
pub mod testutil;

pub use worker::PdfWorker;
