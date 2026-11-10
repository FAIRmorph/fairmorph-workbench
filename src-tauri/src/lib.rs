mod engine;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            engine::engine_handshake,
            engine::run_project,
            engine::dataset_metadata_status,
            engine::save_dataset_metadata_draft,
            engine::stage_dataset_metadata,
            engine::confirm_dataset_metadata
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
