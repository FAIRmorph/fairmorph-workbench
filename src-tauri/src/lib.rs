mod engine;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            engine::engine_handshake,
            engine::choose_project_parent,
            engine::choose_source_files,
            engine::create_project,
            engine::import_project_source,
            engine::choose_pipeline_template,
            engine::save_project_pipeline_draft,
            engine::register_project_pipeline,
            engine::select_registered_pipeline,
            engine::run_project,
            engine::verify_project,
            engine::dataset_metadata_status,
            engine::save_dataset_metadata_draft,
            engine::stage_dataset_metadata,
            engine::confirm_dataset_metadata
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
