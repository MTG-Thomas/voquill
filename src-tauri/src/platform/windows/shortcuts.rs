use tauri::Manager;

pub async fn start_windows_hotkey_engine(app_handle: tauri::AppHandle) -> Result<(), String> {
    let hotkey_string = {
        let state = app_handle.state::<crate::AppState>();
        let config = state.config.lock().unwrap();
        config.hotkey.clone()
    };

    crate::hotkey::register_plugin_hotkey(&app_handle, &hotkey_string, "Windows")
}
