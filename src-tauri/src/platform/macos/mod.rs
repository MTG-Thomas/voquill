use async_trait::async_trait;
use std::process::Command;
use std::sync::Arc;
use tauri::{Manager, WebviewWindow};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

use crate::platform::traits::{
    DisplayBackend, GlobalShortcutEngine, InputSimulation, PermissionManager, WindowManagement,
};

pub struct MacOsBackend;

impl MacOsBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MacOsBackend {
    fn default() -> Self {
        Self::new()
    }
}

pub fn initialize() -> Arc<dyn DisplayBackend> {
    Arc::new(MacOsBackend::new())
}

#[async_trait]
impl InputSimulation for MacOsBackend {
    async fn type_text_hardware(
        &self,
        _app_handle: &tauri::AppHandle,
        text: &str,
        _typing_speed_interval: f64,
        _key_press_duration_ms: u64,
    ) -> Result<(), String> {
        crate::typing::copy_to_clipboard(text).map_err(|error| error.to_string())?;

        let status = Command::new("osascript")
            .args([
                "-e",
                r#"tell application "System Events" to keystroke "v" using command down"#,
            ])
            .status()
            .map_err(|error| format!("Failed to invoke macOS paste automation: {error}"))?;

        if !status.success() {
            return Err(format!(
                "macOS paste automation failed with status {status}. Grant Voquill Accessibility/Input Monitoring permission and try again."
            ));
        }

        Ok(())
    }
}

#[async_trait]
impl GlobalShortcutEngine for MacOsBackend {
    async fn start_engine(&self, app_handle: tauri::AppHandle, _force: bool) -> Result<(), String> {
        let hotkey_string = {
            let state = app_handle.state::<crate::AppState>();
            let config = state.config.lock().unwrap();
            config.hotkey.clone()
        };

        crate::log_info!("Re-registering macOS hotkey: {}", hotkey_string);
        let _ = app_handle.global_shortcut().unregister_all();

        let shortcut = crate::hotkey::parse_hotkey_string(&hotkey_string)
            .map_err(|error| format!("Failed to parse hotkey string: {error}"))?;
        app_handle
            .global_shortcut()
            .register(shortcut)
            .map_err(|error| format!("Failed to register macOS global hotkey: {error}"))?;

        Ok(())
    }
}

#[async_trait]
impl PermissionManager for MacOsBackend {
    async fn request_permissions(&self, _app_handle: tauri::AppHandle) -> Result<(), String> {
        Ok(())
    }

    async fn check_permissions(
        &self,
        _config: &crate::config::Config,
    ) -> crate::platform::permissions::LinuxPermissions {
        crate::platform::permissions::LinuxPermissions {
            audio: true,
            shortcuts: true,
            input_emulation: true,
            shortcuts_status: "ready".to_string(),
            shortcuts_detail: Some(
                "macOS may still prompt for Microphone, Accessibility, and Input Monitoring at runtime."
                    .to_string(),
            ),
            manual_overlay_offset_supported: true,
            overlay_positioning_detail: None,
        }
    }
}

#[async_trait]
impl WindowManagement for MacOsBackend {
    fn apply_overlay_hints(&self, window: &WebviewWindow) {
        let _ = window.set_focusable(false);
        let _ = window.set_ignore_cursor_events(true);
    }

    fn position_overlay_window(
        &self,
        window: &WebviewWindow,
        pixels_from_bottom: i32,
    ) -> Result<(), String> {
        let monitor = window
            .primary_monitor()
            .map_err(|error| error.to_string())?
            .ok_or("No primary monitor found")?;
        let monitor_size = monitor.size();
        let monitor_position = monitor.position();
        let scale_factor = monitor.scale_factor();
        let window_width_logical = 260.0;
        let window_height_logical = 140.0;
        let x = monitor_position.x
            + (monitor_size.width as i32 - (window_width_logical * scale_factor) as i32) / 2;
        let y = monitor_position.y + monitor_size.height as i32
            - (window_height_logical * scale_factor) as i32
            - (pixels_from_bottom as f64 * scale_factor) as i32;

        window
            .set_position(tauri::Position::Physical(tauri::PhysicalPosition::new(
                x, y,
            )))
            .map_err(|error| error.to_string())?;
        window
            .set_size(tauri::LogicalSize::new(
                window_width_logical,
                window_height_logical,
            ))
            .map_err(|error| error.to_string())?;
        Ok(())
    }
}
