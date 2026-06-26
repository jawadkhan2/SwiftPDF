//! Stamping free text and signature images onto pages (Fill & Sign).
//!
//! Input positions are fractions of the page box with a top-left origin (matching
//! the UI overlay). PDF user space has a bottom-left origin, so each fraction is
//! converted to points and flipped vertically here.

use super::{dto::Stamp, pdfium_page_index};
use base64::Engine as _;
use pdfium_render::prelude::*;

/// Apply every stamp to `doc` (a fresh, mutable copy supplied by the caller) and
/// return the new document bytes. The caller's cached pristine document must not
/// be passed here, so repeated saves never compound.
pub fn apply_stamps(doc: &mut PdfDocument, stamps: &[Stamp]) -> Result<Vec<u8>, String> {
    // A single built-in font shared by all text stamps. The token is a plain
    // handle (Copy, no borrow of `doc`), so it's reused freely across the loop.
    let font = doc.fonts_mut().helvetica();

    for stamp in stamps {
        match stamp {
            Stamp::Text {
                page,
                fx,
                fy,
                fh,
                text,
                color,
            } => {
                let page_index = pdfium_page_index(*page, "Page")?;
                let mut p = doc
                    .pages()
                    .get(page_index)
                    .map_err(|e| format!("No page {}: {e}", page + 1))?;
                let geom = PageGeom::of(&p);
                // Font height is a fraction of the *displayed* page height.
                let size = (fh * geom.disp_h).max(4.0);
                // Baseline sits one font-size below the top of the box, in display
                // space, then mapped into the page's unrotated user space.
                let (bx, by) = geom.view_to_user(fx * geom.disp_w, fy * geom.disp_h + size);
                let (cos, sin) = geom.upright_cos_sin();

                let mut obj = p
                    .objects_mut()
                    .create_text_object(
                        PdfPoints::new(0.0),
                        PdfPoints::new(0.0),
                        text,
                        font,
                        PdfPoints::new(size),
                    )
                    .map_err(|e| format!("Could not add text: {e}"))?;
                // Counter-rotate so the text stays upright once the viewer
                // re-applies the page's /Rotate, and land the baseline at (bx, by).
                obj.apply_matrix(PdfMatrix::new(cos, sin, -sin, cos, bx, by))
                    .map_err(|e| format!("Could not orient text: {e}"))?;
                obj.set_fill_color(PdfColor::new(color[0], color[1], color[2], 255))
                    .map_err(|e| format!("Could not colour text: {e}"))?;
                // Adding the object already regenerated the page's content stream
                // with the default (black) fill. set_fill_color mutates the object
                // but does not re-trigger regeneration, so without this the saved
                // stream keeps the stale black. Force a regen after the colour set.
                p.regenerate_content()
                    .map_err(|e| format!("Could not apply text colour: {e}"))?;
            }
            Stamp::Image {
                page,
                fx,
                fy,
                fw,
                fh,
                png_base64,
            } => {
                let png = base64::engine::general_purpose::STANDARD
                    .decode(png_base64)
                    .map_err(|e| format!("Bad image data: {e}"))?;
                let image = image::load_from_memory(&png)
                    .map_err(|e| format!("Could not decode signature image: {e}"))?;

                let page_index = pdfium_page_index(*page, "Page")?;
                let mut p = doc
                    .pages()
                    .get(page_index)
                    .map_err(|e| format!("No page {}: {e}", page + 1))?;
                let geom = PageGeom::of(&p);
                // Width/height are fractions of the *displayed* page box.
                let w_img = fw * geom.disp_w;
                let h_img = fh * geom.disp_h;
                // Bottom-left corner of the upright image, in display space,
                // mapped into the page's unrotated user space.
                let (ix, iy) = geom.view_to_user(fx * geom.disp_w, fy * geom.disp_h + h_img);
                let (cos, sin) = geom.upright_cos_sin();

                let mut obj = p
                    .objects_mut()
                    .create_image_object(
                        PdfPoints::new(0.0),
                        PdfPoints::new(0.0),
                        &image,
                        None,
                        None,
                    )
                    .map_err(|e| format!("Could not place signature: {e}"))?;
                // A freshly created image object maps the unit square to a 1x1 pt
                // box at the origin (identity matrix), so apply_matrix sets the
                // object matrix outright: scale + counter-rotate + translate so the
                // image stays upright after the viewer re-applies /Rotate.
                obj.apply_matrix(PdfMatrix::new(
                    cos * w_img,
                    sin * w_img,
                    -sin * h_img,
                    cos * h_img,
                    ix,
                    iy,
                ))
                .map_err(|e| format!("Could not orient signature: {e}"))?;
                p.regenerate_content()
                    .map_err(|e| format!("Could not place signature: {e}"))?;
            }
        }
    }

    doc.save_to_bytes()
        .map_err(|e| format!("Could not serialize PDF: {e}"))
}

/// Geometry helper mapping the editor's display-space fractions (top-left
/// origin, matching the rendered, rotation-applied bitmap) onto the page's
/// unrotated PDF user space (bottom-left origin).
///
/// `pdfium`'s `width()`/`height()` already report the *displayed* (rotation-
/// applied) box — exactly what the editor fractions are relative to. The page's
/// content, though, lives in unrotated user space (with width/height swapped at
/// 90/270 degrees), and viewers re-apply `/Rotate` at display time. So a stamp's
/// display position must be mapped back into user space here.
struct PageGeom {
    /// Displayed page dimensions in points (what the UI fractions reference).
    disp_w: f32,
    disp_h: f32,
    /// Unrotated user-space dimensions: disp_w/disp_h swapped at 90/270 degrees.
    user_w: f32,
    user_h: f32,
    /// Page rotation, normalised to 0/90/180/270 degrees clockwise.
    deg: i32,
}

impl PageGeom {
    fn of(p: &PdfPage) -> Self {
        let disp_w = p.width().value;
        let disp_h = p.height().value;
        let deg = match p.rotation().unwrap_or(PdfPageRenderRotation::None) {
            PdfPageRenderRotation::Degrees90 => 90,
            PdfPageRenderRotation::Degrees180 => 180,
            PdfPageRenderRotation::Degrees270 => 270,
            _ => 0,
        };
        let (user_w, user_h) = if deg == 90 || deg == 270 {
            (disp_h, disp_w)
        } else {
            (disp_w, disp_h)
        };
        PageGeom {
            disp_w,
            disp_h,
            user_w,
            user_h,
            deg,
        }
    }

    /// Map a point in display space (top-left origin, y down) to unrotated PDF
    /// user space (bottom-left origin, y up).
    fn view_to_user(&self, dx: f32, dy: f32) -> (f32, f32) {
        let xv = dx;
        let yv = self.disp_h - dy; // flip to bottom-left origin within the view
        match self.deg {
            90 => (self.user_w - yv, xv),
            180 => (self.user_w - xv, self.user_h - yv),
            270 => (yv, self.user_h - xv),
            _ => (xv, yv),
        }
    }

    /// (cos, sin) of the counter-rotation that keeps an object upright once the
    /// viewer re-applies the page's clockwise /Rotate. Exact integers avoid
    /// floating-point drift at the four right angles.
    fn upright_cos_sin(&self) -> (f32, f32) {
        match self.deg {
            90 => (0.0, 1.0),
            180 => (-1.0, 0.0),
            270 => (0.0, -1.0),
            _ => (1.0, 0.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdf::engine::init_pdfium;
    use crate::pdf::testutil::multi_page_pdf_bytes;

    #[test]
    fn stamps_text_and_image_onto_page() {
        let pdfium = init_pdfium(None).expect("pdfium");
        let src = multi_page_pdf_bytes(&[(300.0, 400.0)]);
        let mut doc = pdfium.load_pdf_from_byte_vec(src, None).expect("load");

        // A tiny 4x4 red PNG for the image stamp.
        let mut img = image::RgbaImage::new(4, 4);
        for px in img.pixels_mut() {
            *px = image::Rgba([255, 0, 0, 255]);
        }
        let mut png = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let png_b64 = base64::engine::general_purpose::STANDARD.encode(&png);

        let stamps = vec![
            Stamp::Text {
                page: 0,
                fx: 0.1,
                fy: 0.1,
                fh: 0.04,
                text: "Hello".into(),
                color: [0, 0, 0],
            },
            Stamp::Image {
                page: 0,
                fx: 0.5,
                fy: 0.5,
                fw: 0.2,
                fh: 0.1,
                png_base64: png_b64,
            },
        ];

        let out = apply_stamps(&mut doc, &stamps).expect("apply");

        // Reload and confirm the page gained two new objects.
        let doc = pdfium.load_pdf_from_byte_slice(&out, None).unwrap();
        let count = doc.pages().get(0).unwrap().objects().len();
        assert!(count >= 2, "expected stamped objects, found {count}");
    }

    #[test]
    fn text_is_counter_rotated_on_a_rotated_page() {
        // On a 90-degree page the viewer re-applies /Rotate at display time, so
        // the stamped text object must carry the opposite rotation to stay upright
        // and land inside the raw (unrotated) page box.
        let pdfium = init_pdfium(None).expect("pdfium");
        let src = multi_page_pdf_bytes(&[(300.0, 400.0)]);
        let mut doc = pdfium.load_pdf_from_byte_vec(src, None).expect("load");
        doc.pages()
            .get(0)
            .unwrap()
            .set_rotation(PdfPageRenderRotation::Degrees90);

        let stamps = vec![Stamp::Text {
            page: 0,
            fx: 0.1,
            fy: 0.1,
            fh: 0.04,
            text: "Upright".into(),
            color: [0, 0, 0],
        }];
        let out = apply_stamps(&mut doc, &stamps).expect("apply");

        let doc = pdfium.load_pdf_from_byte_slice(&out, None).unwrap();
        let page = doc.pages().get(0).unwrap();
        let text = page
            .objects()
            .iter()
            .find(|o| o.object_type() == PdfPageObjectType::Text)
            .expect("a text object");
        let m = text.matrix().expect("matrix");

        // 90-degree counter-rotation: a~0, b~1, c~-1, d~0 (not the identity the
        // old code produced).
        assert!(m.a().abs() < 0.01, "a should be ~0, got {}", m.a());
        assert!((m.b() - 1.0).abs() < 0.01, "b should be ~1, got {}", m.b());
        assert!((m.c() + 1.0).abs() < 0.01, "c should be ~-1, got {}", m.c());
        assert!(m.d().abs() < 0.01, "d should be ~0, got {}", m.d());
        // Translation lands at the expected user-space baseline. MediaBox is
        // 300x400; rotated 90deg the displayed box is 400x300. fx=0.1, fy=0.1,
        // size=0.04*300=12 -> display baseline (40, 42) maps to user (42, 40).
        assert!((m.e() - 42.0).abs() < 0.5, "e should be ~42, got {}", m.e());
        assert!((m.f() - 40.0).abs() < 0.5, "f should be ~40, got {}", m.f());
    }
}
