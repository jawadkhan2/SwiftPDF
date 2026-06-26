//! Rasterizing pages to PNG for thumbnails and the editor canvas.

use super::{dto::RenderResult, pdfium_page_index};
use base64::Engine as _;
use image::ImageFormat;
use pdfium_render::prelude::*;
use std::io::Cursor;

/// Render a single page so its longest edge is `target_long_px` pixels, keeping
/// aspect ratio. Returns base64 PNG plus pixel and point dimensions. Reads from
/// an already-open document, so no per-call reparse of the source file.
pub fn render_page(
    doc: &PdfDocument,
    page_index: usize,
    target_long_px: u32,
) -> Result<RenderResult, String> {
    let page_index = pdfium_page_index(page_index, "Page")?;
    let page = doc
        .pages()
        .get(page_index)
        .map_err(|e| format!("No page {page_index}: {e}"))?;

    let width_pt = page.width().value;
    let height_pt = page.height().value;
    let long_pt = width_pt.max(height_pt).max(1.0);
    let scale = target_long_px as f32 / long_pt;

    let config = PdfRenderConfig::new().scale_page_by_factor(scale);

    let bitmap = page
        .render_with_config(&config)
        .map_err(|e| format!("Render failed: {e}"))?;

    let image = bitmap.as_image();
    let width_px = image.width();
    let height_px = image.height();

    let mut png = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|e| format!("PNG encode failed: {e}"))?;

    let png_base64 = base64::engine::general_purpose::STANDARD.encode(&png);

    Ok(RenderResult {
        png_base64,
        width_px,
        height_px,
        width_pt,
        height_pt,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::engine::init_pdfium;
    use crate::pdf::testutil::sample_pdf_bytes;

    #[test]
    fn renders_a_page_to_png() {
        // Proves the full core pipeline headlessly: locate + bind PDFium,
        // load a document from bytes, and rasterize a page.
        let pdfium = init_pdfium(None).expect("PDFium should load from resources/pdfium");
        let bytes = sample_pdf_bytes(200.0, 300.0);
        let doc = pdfium
            .load_pdf_from_byte_vec(bytes, None)
            .expect("load sample");

        let result = render_page(&doc, 0, 240).expect("render should succeed");

        assert!(result.width_px > 0 && result.height_px > 0);
        // 200x300 pt page -> long edge 300pt scaled to 240px.
        assert_eq!(result.height_px, 240);
        assert!(!result.png_base64.is_empty());
        assert_eq!(result.width_pt, 200.0);
        assert_eq!(result.height_pt, 300.0);
    }
}
