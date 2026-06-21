//! Reading document-level metadata from raw PDF bytes.

use super::dto::{OpenResult, PageInfo};
use pdfium_render::prelude::*;

/// Extract page sizes / form presence from an already-open document. Pure read;
/// does not mutate. `doc_id` and `source_name` are supplied by the caller.
pub fn metadata(
    doc: &PdfDocument,
    doc_id: String,
    source_name: String,
) -> Result<OpenResult, String> {
    let pages: Vec<PageInfo> = doc
        .pages()
        .iter()
        .map(|page| PageInfo {
            width_pt: page.width().value,
            height_pt: page.height().value,
            rotation: rotation_degrees(page.rotation().unwrap_or(PdfPageRenderRotation::None)),
        })
        .collect();

    let has_form = doc.form().is_some();

    Ok(OpenResult {
        doc_id,
        source_name,
        page_count: pages.len(),
        pages,
        has_form,
    })
}

fn rotation_degrees(r: PdfPageRenderRotation) -> i32 {
    match r {
        PdfPageRenderRotation::None => 0,
        PdfPageRenderRotation::Degrees90 => 90,
        PdfPageRenderRotation::Degrees180 => 180,
        PdfPageRenderRotation::Degrees270 => 270,
    }
}
