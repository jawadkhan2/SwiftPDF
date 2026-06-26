//! Test-only helpers for constructing PDFs in memory.

use lopdf::{dictionary, Document, Object, Stream};

/// Build a minimal valid single-page PDF of the given size (in points) and
/// return its bytes. The page draws a thin rectangle so it isn't entirely
/// blank, which keeps rasterization meaningful.
pub fn sample_pdf_bytes(width_pt: f32, height_pt: f32) -> Vec<u8> {
    multi_page_pdf_bytes(&[(width_pt, height_pt)])
}

/// Build a multi-page PDF; each tuple is one page's (width, height) in points.
pub fn multi_page_pdf_bytes(sizes: &[(f32, f32)]) -> Vec<u8> {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();

    let kids: Vec<Object> = sizes
        .iter()
        .enumerate()
        .map(|(i, (w, h))| {
            // A simple content stream: draw a rectangle outline.
            let ops = format!(
                "1 1 1 rg 0 0 {w} {h} re f 0 0 0 RG 10 10 {0} {1} re S",
                w - 20.0,
                h - 20.0
            );
            let content_id = doc.add_object(Stream::new(dictionary! {}, ops.into_bytes()));
            let page_id = doc.add_object(dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "Contents" => content_id,
                "MediaBox" => vec![0.into(), 0.into(), (*w as i64).into(), (*h as i64).into()],
            });
            let _ = i;
            page_id.into()
        })
        .collect();

    let count = kids.len() as i64;
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => kids,
            "Count" => count,
        }),
    );

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    let mut buf = Vec::new();
    doc.save_to(&mut buf).expect("save sample pdf");
    buf
}
