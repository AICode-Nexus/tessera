use std::sync::{Mutex, MutexGuard};

use tauri::State;
use tessera_client::{ClientIntent, ClientSnapshot};
use tessera_gui_bridge::{GuiBridge, GuiBridgeError, GuiCommandOutcome, GuiProfile};
use tessera_protocol::{TaskId, TraceRecord};

pub struct GuiState {
    bridge: Mutex<GuiBridge>,
}

impl GuiState {
    fn bridge(&self) -> Result<MutexGuard<'_, GuiBridge>, String> {
        self.bridge
            .lock()
            .map_err(|_| "GUI bridge state lock was poisoned".to_string())
    }
}

impl Default for GuiState {
    fn default() -> Self {
        Self {
            bridge: Mutex::new(GuiBridge::default()),
        }
    }
}

#[tauri::command]
pub fn list_profiles(state: State<'_, GuiState>) -> Result<Vec<GuiProfile>, String> {
    Ok(state.bridge()?.list_profiles())
}

#[tauri::command]
pub fn load_client_snapshot(state: State<'_, GuiState>) -> Result<ClientSnapshot, String> {
    Ok(state.bridge()?.load_client_snapshot())
}

#[tauri::command]
pub fn submit_client_intent(
    state: State<'_, GuiState>,
    intent: ClientIntent,
) -> Result<GuiCommandOutcome, String> {
    state
        .bridge()?
        .submit_client_intent(intent)
        .map_err(gui_error)
}

#[tauri::command]
pub fn cancel_task(
    state: State<'_, GuiState>,
    task_id: Option<TaskId>,
) -> Result<GuiCommandOutcome, String> {
    state.bridge()?.cancel_task(task_id).map_err(gui_error)
}

#[tauri::command]
pub fn load_trace_projection(
    state: State<'_, GuiState>,
    records: Vec<TraceRecord>,
) -> Result<GuiCommandOutcome, String> {
    state
        .bridge()?
        .load_trace_projection(records)
        .map_err(gui_error)
}

#[tauri::command]
pub fn export_thread(state: State<'_, GuiState>) -> Result<String, String> {
    Ok(state.bridge()?.export_thread())
}

fn gui_error(error: GuiBridgeError) -> String {
    error.to_string()
}
