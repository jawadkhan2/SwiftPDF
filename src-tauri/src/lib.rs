mod commands;
mod pdf;

use pdf::PdfWorker;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Locate the bundled PDFium library and spin up the worker thread.
            let resource_dir = app.path().resource_dir().ok();
            let worker = PdfWorker::start(resource_dir);
            app.manage(worker);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_pdf,
            commands::merge_pdfs,
            commands::render_thumbnail,
            commands::render_page,
            commands::save_built_pdf,
            commands::save_signed_pdf,
            commands::split_pdf,
            commands::close_doc,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
