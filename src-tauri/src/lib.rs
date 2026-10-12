mod commands;
mod filetypes;
mod scanner;
mod tree;
mod treemap;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::initial_path,
            commands::list_drives,
            commands::start_scan,
            commands::cancel_scan,
            commands::get_node,
            commands::get_children,
            commands::get_file_types,
            commands::render_treemap,
            commands::hit_test,
            commands::node_info,
            commands::reveal,
            commands::trash,
            commands::largest_files,
            commands::open_recycle_bin,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
