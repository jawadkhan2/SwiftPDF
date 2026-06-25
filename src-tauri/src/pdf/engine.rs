//! Locating and binding the native PDFium library.

use pdfium_render::prelude::*;
use std::path::{Path, PathBuf};

/// Directories to search for the platform PDFium shared library, in priority
/// order. The first that yields a successful bind wins.
///
/// - In a packaged build the library is bundled under the Tauri resource dir
///   (see `tauri.conf.json` -> `bundle.resources`). Tauri's resource map puts
///   the file at `$RESOURCE/pdfium/<library>`, while the extra
///   `$RESOURCE/resources/pdfium` candidate covers older/preserved layouts.
/// - In `tauri dev` resources aren't copied, so we fall back to the in-repo
///   `resources/pdfium` directory via `CARGO_MANIFEST_DIR`.
fn candidate_dirs(resource_dir: Option<PathBuf>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Some(res) = resource_dir {
        push_resource_candidates(&mut dirs, &res);
    }

    // Dev fallback: the library committed next to the crate.
    push_unique(
        &mut dirs,
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources/pdfium"),
    );

    // Last resort: app bundle resources derived from the executable path, then
    // next to the executable. This helps if Tauri cannot resolve resource_dir()
    // in a packaged macOS app.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            push_resource_candidates(&mut dirs, parent);
            push_unique(&mut dirs, parent.join("pdfium"));
            push_unique(&mut dirs, parent.to_path_buf());
        }
    }

    dirs
}

fn push_resource_candidates(dirs: &mut Vec<PathBuf>, resource_dir: &Path) {
    if let Some(bundle_resources) = macos_bundle_resources_from_macos_dir(resource_dir) {
        push_unique(dirs, bundle_resources.join("pdfium"));
        push_unique(dirs, bundle_resources);
    }

    push_unique(dirs, resource_dir.join("pdfium"));
    push_unique(dirs, resource_dir.join("resources/pdfium"));
    push_unique(dirs, resource_dir.to_path_buf());
}

fn macos_bundle_resources_from_macos_dir(dir: &Path) -> Option<PathBuf> {
    if dir.file_name().and_then(|name| name.to_str()) != Some("MacOS") {
        return None;
    }

    let contents_dir = dir.parent()?;
    if contents_dir.file_name().and_then(|name| name.to_str()) != Some("Contents") {
        return None;
    }

    Some(contents_dir.join("Resources"))
}

fn push_unique(dirs: &mut Vec<PathBuf>, dir: PathBuf) {
    if !dirs.iter().any(|existing| existing == &dir) {
        dirs.push(dir);
    }
}

/// Bind to the first PDFium library we can find, returning a ready `Pdfium`.
pub fn init_pdfium(resource_dir: Option<PathBuf>) -> Result<Pdfium, String> {
    let mut attempts = Vec::new();

    for dir in candidate_dirs(resource_dir) {
        let lib_path = Pdfium::pdfium_platform_library_name_at_path(&dir);

        if !lib_path.exists() {
            attempts.push(format!("{}: not found", lib_path.display()));
            continue;
        }

        match Pdfium::bind_to_library(&lib_path) {
            Ok(bindings) => return Ok(Pdfium::new(bindings)),
            Err(e) => attempts.push(format!("{}: {}", lib_path.display(), e)),
        }
    }

    // Final attempt: a system-installed PDFium, if any.
    match Pdfium::bind_to_system_library() {
        Ok(bindings) => Ok(Pdfium::new(bindings)),
        Err(e) => Err(format!(
            "Could not load PDFium. Tried bundled libraries: {}. System library: {e}",
            if attempts.is_empty() {
                "none".to_string()
            } else {
                attempts.join("; ")
            }
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_dir_candidates_cover_tauri_resource_layouts() {
        let resource_dir = PathBuf::from("/Applications/SwiftPDF.app/Contents/Resources");
        let dirs = candidate_dirs(Some(resource_dir.clone()));

        assert!(dirs.contains(&resource_dir.join("pdfium")));
        assert!(dirs.contains(&resource_dir.join("resources/pdfium")));
        assert!(dirs.contains(&resource_dir));
    }

    #[test]
    fn macos_executable_dir_candidates_cover_bundle_resources() {
        let macos_dir = PathBuf::from("/Applications/SwiftPDF.app/Contents/MacOS");
        let resources_dir = PathBuf::from("/Applications/SwiftPDF.app/Contents/Resources");
        let dirs = candidate_dirs(Some(macos_dir));

        assert!(dirs.contains(&resources_dir.join("pdfium")));
        assert!(dirs.contains(&resources_dir));
    }
}
