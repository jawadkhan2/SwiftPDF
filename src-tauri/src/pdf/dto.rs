//! Serializable data-transfer objects shared across the Tauri IPC boundary.

use serde::{Deserialize, Serialize};

/// A single page's intrinsic size, in PDF points (1pt = 1/72 inch).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    pub width_pt: f32,
    pub height_pt: f32,
    /// Page rotation already baked into the document, in degrees (0/90/180/270).
    pub rotation: i32,
}

/// Result of opening a document: a handle plus lightweight metadata. The actual
/// bytes stay in the worker; the frontend only ever holds this summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenResult {
    pub doc_id: String,
    /// Original file name (e.g. `Contract.pdf`), used to suggest the save name.
    pub source_name: String,
    pub page_count: usize,
    pub pages: Vec<PageInfo>,
    pub has_form: bool,
}

/// One entry in a "build" plan: take source page `page` (0-based index into the
/// originally opened document) and give it final absolute `rotation` degrees
/// (0/90/180/270). The order of the plan is the output page order, so omitting a
/// page deletes it and reordering the list reorders the output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagePlan {
    pub page: usize,
    pub rotation: i32,
}

/// Something the user has placed on a page in the Fill & Sign editor. Positions
/// and sizes are **fractions of the page box** (0..1, top-left origin), so they
/// are independent of the zoom/render resolution used in the UI. The backend
/// converts them to PDF points at stamp time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Stamp {
    /// Free text typed onto the page.
    Text {
        page: usize,
        fx: f32,
        fy: f32,
        /// Text height as a fraction of page height (becomes the font size).
        fh: f32,
        text: String,
        /// RGB fill colour.
        color: [u8; 3],
    },
    /// A placed image (a drawn or typed signature rasterized to PNG).
    Image {
        page: usize,
        fx: f32,
        fy: f32,
        fw: f32,
        fh: f32,
        /// Bare base64 PNG (no data-URL prefix).
        png_base64: String,
    },
}

/// A rendered raster of one page. `png_base64` is a bare base64 string (no data
/// URL prefix); the frontend wraps it as `data:image/png;base64,...`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderResult {
    pub png_base64: String,
    pub width_px: u32,
    pub height_px: u32,
    /// Source page size in points, so the frontend can map pixel <-> point
    /// coordinates for stamping later.
    pub width_pt: f32,
    pub height_pt: f32,
}
