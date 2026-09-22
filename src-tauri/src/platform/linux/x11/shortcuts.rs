use tauri::Manager;

pub async fn start_x11_hotkey_engine(app_handle: tauri::AppHandle) -> Result<(), String> {
    let hotkey_str = {
        let state = app_handle.state::<crate::AppState>();
        let config = state.config.lock().unwrap();
        config.hotkey.clone()
    };

    if hotkey_str.is_empty() {
        return Err("No hotkey configured for X11.".to_string());
    }

    crate::hotkey::register_plugin_hotkey(&app_handle, &hotkey_str, "X11")
}
