//! Structural page operations: building an output document from a plan.
//!
//! Editing (reorder / delete / rotate) happens entirely in the frontend; the
//! backend receives the final [`PagePlan`] list once at save time and assembles
//! a fresh document in a single pass. This keeps the UI instant and the on-disk
//! write atomic.

use super::dto::PagePlan;
use pdfium_render::prelude::*;

/// Assemble a new PDF from `source` according to `plan` and return its bytes.
/// Pages are copied in plan order (so reorder/delete fall out naturally) and
/// each is given its requested absolute rotation.
pub fn build_document(
    pdfium: &Pdfium,
    source: &PdfDocument,
    plan: &[PagePlan],
) -> Result<Vec<u8>, String> {
    if plan.is_empty() {
        return Err("A document must keep at least one page.".to_string());
    }

    let src_len = source.pages().len() as usize;

    let mut out = pdfium
        .create_new_pdf()
        .map_err(|e| format!("Could not create output PDF: {e}"))?;

    for (dest, item) in plan.iter().enumerate() {
        if item.page >= src_len {
            return Err(format!(
                "Page {} is out of range (document has {src_len} pages).",
                item.page + 1
            ));
        }
        out.pages_mut()
            .copy_page_from_document(source, item.page as u16, dest as u16)
            .map_err(|e| format!("Failed to copy page {}: {e}", item.page + 1))?;
    }

    // Apply absolute rotations after all pages are in place.
    for (dest, item) in plan.iter().enumerate() {
        let mut page = out
            .pages()
            .get(dest as u16)
            .map_err(|e| format!("Failed to access output page {dest}: {e}"))?;
        page.set_rotation(rotation_from_degrees(item.rotation));
    }

    out.save_to_bytes()
        .map_err(|e| format!("Could not serialize PDF: {e}"))
}

/// Concatenate several source documents (given as byte blobs, in order) into a
/// single new PDF. Page rotations are preserved as-is.
pub fn merge_documents(pdfium: &Pdfium, sources: &[Vec<u8>]) -> Result<Vec<u8>, String> {
    if sources.is_empty() {
        return Err("Pick at least one PDF to merge.".to_string());
    }

    let mut out = pdfium
        .create_new_pdf()
        .map_err(|e| format!("Could not create output PDF: {e}"))?;
    let mut dest: u16 = 0;

    for (file_i, bytes) in sources.iter().enumerate() {
        let src = pdfium
            .load_pdf_from_byte_slice(bytes, None)
            .map_err(|e| format!("File {} could not be opened: {e}", file_i + 1))?;
        let n = src.pages().len();
        if n == 0 {
            continue;
        }
        out.pages_mut()
            .copy_page_range_from_document(&src, 0..=(n - 1), dest)
            .map_err(|e| format!("Failed to merge file {}: {e}", file_i + 1))?;
        dest += n;
    }

    if dest == 0 {
        return Err("The selected files had no pages to merge.".to_string());
    }

    out.save_to_bytes()
        .map_err(|e| format!("Could not serialize merged PDF: {e}"))
}

/// Build a new PDF containing only `indices` from `source`, in the order given.
/// Page rotations are preserved (no override). Used for split/extract.
pub fn extract_pages(
    pdfium: &Pdfium,
    source: &PdfDocument,
    indices: &[usize],
) -> Result<Vec<u8>, String> {
    if indices.is_empty() {
        return Err("No pages selected to extract.".to_string());
    }

    let src_len = source.pages().len() as usize;

    let mut out = pdfium
        .create_new_pdf()
        .map_err(|e| format!("Could not create output PDF: {e}"))?;

    for (dest, &idx) in indices.iter().enumerate() {
        if idx >= src_len {
            return Err(format!(
                "Page {} is out of range (document has {src_len} pages).",
                idx + 1
            ));
        }
        out.pages_mut()
            .copy_page_from_document(source, idx as u16, dest as u16)
            .map_err(|e| format!("Failed to copy page {}: {e}", idx + 1))?;
    }

    out.save_to_bytes()
        .map_err(|e| format!("Could not serialize PDF: {e}"))
}

/// Map an arbitrary degree value to the nearest valid PDF rotation.
fn rotation_from_degrees(deg: i32) -> PdfPageRenderRotation {
    match ((deg % 360) + 360) % 360 {
        90 => PdfPageRenderRotation::Degrees90,
        180 => PdfPageRenderRotation::Degrees180,
        270 => PdfPageRenderRotation::Degrees270,
        _ => PdfPageRenderRotation::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::engine::init_pdfium;
    use crate::pdf::testutil::multi_page_pdf_bytes;

    fn page_count(pdfium: &Pdfium, bytes: &[u8]) -> usize {
        pdfium
            .load_pdf_from_byte_slice(bytes, None)
            .unwrap()
            .pages()
            .len() as usize
    }

    #[test]
    fn reorder_and_delete_via_plan() {
        let pdfium = init_pdfium(None).expect("pdfium");
        // 3 pages of distinct sizes so we can verify order by dimensions.
        let src = multi_page_pdf_bytes(&[(200.0, 300.0), (400.0, 300.0), (600.0, 300.0)]);
        let source = pdfium.load_pdf_from_byte_vec(src, None).unwrap();

        // Keep pages in reverse, drop the middle one: [2, 0].
        let plan = vec![
            PagePlan { page: 2, rotation: 0 },
            PagePlan { page: 0, rotation: 0 },
        ];
        let out = build_document(&pdfium, &source, &plan).expect("build");
        assert_eq!(page_count(&pdfium, &out), 2);

        let doc = pdfium.load_pdf_from_byte_slice(&out, None).unwrap();
        assert_eq!(doc.pages().get(0).unwrap().width().value, 600.0);
        assert_eq!(doc.pages().get(1).unwrap().width().value, 200.0);
    }

    #[test]
    fn applies_rotation() {
        let pdfium = init_pdfium(None).expect("pdfium");
        let src = multi_page_pdf_bytes(&[(200.0, 300.0)]);
        let source = pdfium.load_pdf_from_byte_vec(src, None).unwrap();
        let plan = vec![PagePlan { page: 0, rotation: 90 }];
        let out = build_document(&pdfium, &source, &plan).expect("build");

        let doc = pdfium.load_pdf_from_byte_slice(&out, None).unwrap();
        assert_eq!(
            doc.pages().get(0).unwrap().rotation().unwrap(),
            PdfPageRenderRotation::Degrees90
        );
    }

    #[test]
    fn empty_plan_is_rejected() {
        let pdfium = init_pdfium(None).expect("pdfium");
        let src = multi_page_pdf_bytes(&[(200.0, 300.0)]);
        let source = pdfium.load_pdf_from_byte_vec(src, None).unwrap();
        assert!(build_document(&pdfium, &source, &[]).is_err());
    }

    #[test]
    fn merge_sums_pages_in_order() {
        let pdfium = init_pdfium(None).expect("pdfium");
        let a = multi_page_pdf_bytes(&[(200.0, 300.0), (210.0, 300.0)]); // 2 pages
        let b = multi_page_pdf_bytes(&[(400.0, 300.0)]); // 1 page

        let out = merge_documents(&pdfium, &[a, b]).expect("merge");
        assert_eq!(page_count(&pdfium, &out), 3);

        let doc = pdfium.load_pdf_from_byte_slice(&out, None).unwrap();
        // First file's pages come first, then the second file's page.
        assert_eq!(doc.pages().get(0).unwrap().width().value, 200.0);
        assert_eq!(doc.pages().get(2).unwrap().width().value, 400.0);
    }

    #[test]
    fn extract_keeps_only_requested_pages() {
        let pdfium = init_pdfium(None).expect("pdfium");
        let src = multi_page_pdf_bytes(&[(200.0, 300.0), (400.0, 300.0), (600.0, 300.0)]);
        let source = pdfium.load_pdf_from_byte_vec(src, None).unwrap();

        let out = extract_pages(&pdfium, &source, &[2, 0]).expect("extract");
        assert_eq!(page_count(&pdfium, &out), 2);

        let doc = pdfium.load_pdf_from_byte_slice(&out, None).unwrap();
        assert_eq!(doc.pages().get(0).unwrap().width().value, 600.0);
        assert_eq!(doc.pages().get(1).unwrap().width().value, 200.0);
    }
}
