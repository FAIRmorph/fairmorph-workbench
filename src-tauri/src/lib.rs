mod engine;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![engine::run_poc, engine::select_csv_file, engine::inspect_csv])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}