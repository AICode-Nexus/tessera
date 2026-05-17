mod commands;

pub fn run() {
    tauri::Builder::default()
        .manage(commands::GuiState::default())
        .invoke_handler(tauri::generate_handler![
            commands::list_profiles,
            commands::load_client_snapshot,
            commands::submit_client_intent,
            commands::cancel_task,
            commands::load_trace_projection,
            commands::export_thread,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Tessera GUI shell");
}
