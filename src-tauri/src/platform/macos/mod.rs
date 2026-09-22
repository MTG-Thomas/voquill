use async_trait::async_trait;
use std::process::Command;
use std::sync::Arc;
use tauri::{Manager, WebviewWindow};

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

/// Runs a System Events script through `osascript`, mapping failures to an
/// actionable Accessibility/Input Monitoring permission hint.
fn run_osascript(action: &str, script: &str) -> Result<(), String> {
    let status = Command::new("osascript")
        .args(["-e", script])
        .status()
        .map_err(|error| format!("Failed to invoke macOS {action}: {error}"))?;

    if !status.success() {
        return Err(format!(
            "macOS {action} failed with status {status}. Grant Voquill Accessibility/Input Monitoring permission and try again."
        ));
    }

    Ok(())
}

/// Maps a cross-platform modifier token to its AppleScript counterpart.
/// `ctrl` maps to `command` because Command is the primary modifier on macOS
/// (so cross-platform shortcuts like Ctrl+V become Cmd+V).
fn modifier_token_to_applescript(token: &str) -> Option<&'static str> {
    match token.trim().to_lowercase().as_str() {
        "ctrl" | "control" | "lctrl" | "rctrl" | "super" | "win" | "cmd" | "meta" | "command" => {
            Some("command")
        }
        "alt" | "option" | "lalt" | "ralt" => Some("option"),
        "shift" | "lshift" | "rshift" => Some("shift"),
        _ => None,
    }
}

/// Maps a non-modifier key token to a System Events statement.
/// Single characters use `keystroke`; named keys use `key code` with macOS
/// virtual key codes. Returns an error for keys with no macOS equivalent.
fn key_token_to_applescript_statement(token: &str) -> Result<String, String> {
    let lower = token.trim().to_lowercase();
    let stripped_key = lower.strip_prefix("key").unwrap_or(&lower);
    let stripped_digit = stripped_key.strip_prefix("digit").unwrap_or(stripped_key);
    let stripped = stripped_digit.strip_prefix("num").unwrap_or(stripped_digit);

    if stripped.len() == 1 {
        let ch = stripped.chars().next().unwrap();
        if ch.is_ascii_alphanumeric() || ch == ' ' {
            let escaped = if ch == '"' {
                "\\\"".to_string()
            } else {
                ch.to_string()
            };
            return Ok(format!("keystroke \"{escaped}\""));
        }
        return Err(format!("Unsupported macOS key token '{token}'"));
    }

    let code: u8 = match stripped {
        "space" => 49,
        "enter" | "return" => 36,
        "tab" => 48,
        "esc" | "escape" => 53,
        "backspace" => 51,
        "delete" | "del" => 117,
        "home" => 115,
        "end" => 119,
        "pageup" | "pgup" => 116,
        "pagedown" | "pgdn" => 121,
        "up" | "arrowup" => 126,
        "down" | "arrowdown" => 125,
        "left" | "arrowleft" => 123,
        "right" | "arrowright" => 124,
        "f1" => 122,
        "f2" => 120,
        "f3" => 99,
        "f4" => 118,
        "f5" => 96,
        "f6" => 97,
        "f7" => 98,
        "f8" => 100,
        "f9" => 101,
        "f10" => 109,
        "f11" => 103,
        "f12" => 111,
        _ => {
            return Err(format!(
                "Unrecognized key token '{token}'. Expected a valid key (e.g. F1-F12, Ctrl, Alt, Shift, Command, A-Z, 0-9, Space, Enter, Tab, Escape, etc.)"
            ))
        }
    };
    Ok(format!("key code {code}"))
}

/// Wraps System Events statements in the required `tell` block.
fn system_events_script(statements: &[String]) -> String {
    let mut script = String::from("tell application \"System Events\"\n");
    for statement in statements {
        script.push_str(statement);
        script.push('\n');
    }
    script.push_str("end tell");
    script
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

        run_osascript(
            "paste automation",
            r#"tell application "System Events" to keystroke "v" using command down"#,
        )
    }

    async fn send_paste_shortcut(
        &self,
        _app_handle: &tauri::AppHandle,
        shortcut: crate::config::PasteShortcut,
    ) -> Result<(), String> {
        crate::log_info!("[macOS] Sending paste shortcut: {shortcut:?}");
        match shortcut {
            crate::config::PasteShortcut::CtrlV => run_osascript(
                "paste shortcut",
                r#"tell application "System Events" to keystroke "v" using command down"#,
            ),
            crate::config::PasteShortcut::CtrlShiftV => run_osascript(
                "paste shortcut",
                r#"tell application "System Events" to keystroke "v" using {command down, shift down}"#,
            ),
            // macOS keyboards have no Insert key; fall back to Cmd+V paste.
            crate::config::PasteShortcut::ShiftInsert => {
                crate::log_info!("[macOS] Shift+Insert has no macOS equivalent; using Cmd+V");
                run_osascript(
                    "paste shortcut",
                    r#"tell application "System Events" to keystroke "v" using command down"#,
                )
            }
        }
    }

    async fn send_key_combination(
        &self,
        _app_handle: &tauri::AppHandle,
        combination: &str,
        hold_duration_ms: u64,
    ) -> Result<(), String> {
        let parts: Vec<&str> = combination
            .split('+')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .collect();
        if parts.is_empty() {
            return Err("Empty key combination".to_string());
        }

        let mut modifiers: Vec<&str> = Vec::new();
        let mut main_statements: Vec<String> = Vec::new();
        for part in parts {
            if let Some(modifier) = modifier_token_to_applescript(part) {
                if !modifiers.contains(&modifier) {
                    modifiers.push(modifier);
                }
            } else {
                main_statements.push(key_token_to_applescript_statement(part)?);
            }
        }

        crate::log_info!(
            "[macOS] Sending key combination: '{combination}' (modifiers: {}, keys: {}, hold: {hold_duration_ms}ms)",
            modifiers.len(),
            main_statements.len()
        );

        if main_statements.is_empty() {
            // Modifiers only: hold them for the requested duration.
            let modifier_list = modifiers.join(", ");
            run_osascript(
                "key combination",
                &system_events_script(&[format!("key down {{{modifier_list}}}")]),
            )?;
            std::thread::sleep(std::time::Duration::from_millis(hold_duration_ms.max(20)));
            return run_osascript(
                "key combination",
                &system_events_script(&[format!("key up {{{modifier_list}}}")]),
            );
        }

        let mut statements = Vec::new();
        if !modifiers.is_empty() {
            statements.push(format!("key down {{{}}}", modifiers.join(", ")));
        }
        statements.extend(main_statements);
        if !modifiers.is_empty() {
            let hold_seconds = f64::from(hold_duration_ms.min(5000) as u32) / 1000.0;
            statements.push(format!("delay {hold_seconds}"));
            statements.push(format!("key up {{{}}}", modifiers.join(", ")));
        }
        run_osascript("key combination", &system_events_script(&statements))
    }

    async fn send_key_down(&self, _app_handle: &tauri::AppHandle, key: &str) -> Result<(), String> {
        crate::log_info!("[macOS] Sending KeyDown: '{key}'");
        if let Some(modifier) = modifier_token_to_applescript(key) {
            return run_osascript(
                "key press",
                &system_events_script(&[format!("key down {{{modifier}}}")]),
            );
        }
        // System Events cannot hold non-modifier keys; press atomically so the
        // matching KeyUp becomes a harmless no-op instead of a second press.
        run_osascript(
            "key press",
            &system_events_script(&[key_token_to_applescript_statement(key)?]),
        )
    }

    async fn send_key_up(&self, _app_handle: &tauri::AppHandle, key: &str) -> Result<(), String> {
        crate::log_info!("[macOS] Sending KeyUp: '{key}'");
        if let Some(modifier) = modifier_token_to_applescript(key) {
            return run_osascript(
                "key release",
                &system_events_script(&[format!("key up {{{modifier}}}")]),
            );
        }
        // Non-modifiers are pressed atomically by send_key_down, so there is
        // nothing still held here.
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

        crate::hotkey::register_plugin_hotkey(&app_handle, &hotkey_string, "macOS")
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
