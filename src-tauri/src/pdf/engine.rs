//! Locating and binding the native PDFium library.

use pdfium_render::prelude::*;
use std::path::PathBuf;

/// Directories to search for the platform PDFium shared library, in priority
/// order. The first that yields a successful bind wins.
///
/// - In a packaged build the library is bundled under the Tauri resource dir
///   (see `tauri.conf.json` -> `bundle.resources`).
/// - In `tauri dev` resources aren't copied, so we fall back to the in-repo
///   `resources/pdfium` directory via `CARGO_MANIFEST_DIR`.
fn candidate_dirs(resource_dir: Option<PathBuf>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(res) = resource_dir {
        dirs.push(res.join("pdfium"));
        dirs.push(res.clone());
    }

    // Dev fallback: the library committed next to the crate.
    dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/pdfium"));

    // Last resort: next to the executable.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.to_path_buf());
        }
    }

    dirs
}

/// Bind to the first PDFium library we can find, returning a ready `Pdfium`.
pub fn init_pdfium(resource_dir: Option<PathBuf>) -> Result<Pdfium, String> {
    let mut last_err = String::from("no candidate directories searched");

    for dir in candidate_dirs(resource_dir) {
        let lib_path = Pdfium::pdfium_platform_library_name_at_path(&dir);
        match Pdfium::bind_to_library(&lib_path) {
            Ok(bindings) => return Ok(Pdfium::new(bindings)),
            Err(e) => last_err = format!("{}: {}", lib_path.display(), e),
        }
    }

    // Final attempt: a system-installed PDFium, if any.
    match Pdfium::bind_to_system_library() {
        Ok(bindings) => Ok(Pdfium::new(bindings)),
        Err(e) => Err(format!(
            "Could not load PDFium. Last error: {last_err}. System library: {e}"
        )),
    }
}
