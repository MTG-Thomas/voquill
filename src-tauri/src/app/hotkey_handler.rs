use crate::app::commands::recording::{cancel_session, start_recording, stop_recording};
use crate::app::state::{AppState, SessionState};
use crate::config::HotkeyMode;

/// Single owner for hotkey press semantics across every platform backend
/// (Wayland portal, X11, Windows):
/// - Idle: press starts a dictation session.
/// - Recording: press stops and transcribes in Toggle mode; ignored in
///   HoldToTalk mode (the user is still holding the keys).
/// - Transcribing/Typing: press cancels and discards the in-flight session.
pub async fn handle_hotkey_press(state: tauri::State<'_, AppState>, app_handle: tauri::AppHandle) {
    let hotkey_mode = state.config.lock().unwrap().hotkey_mode.clone();
    let session = *state.session_state.lock().unwrap();

    crate::log_info!(
        "Hotkey press: session_state={:?}, hotkey_mode={:?}",
        session,
        hotkey_mode
    );

    match session {
        SessionState::Idle => {
            let _ = start_recording(state, app_handle).await;
        }
        SessionState::Recording => {
            if hotkey_mode == HotkeyMode::Toggle {
                let _ = stop_recording(state).await;
            }
        }
        SessionState::Transcribing | SessionState::Typing => {
            cancel_session(state).await;
        }
    }
}

/// Single owner for hotkey release semantics: only HoldToTalk mode stops
/// recording on release. Toggle mode ignores releases entirely.
pub async fn handle_hotkey_release(state: tauri::State<'_, AppState>) {
    let hotkey_mode = state.config.lock().unwrap().hotkey_mode.clone();
    let session = *state.session_state.lock().unwrap();

    if hotkey_mode == HotkeyMode::HoldToTalk && session == SessionState::Recording {
        let _ = stop_recording(state).await;
    }
}

/// External activation (`voquill --record-start`) into the existing
/// recording lifecycle: starts only from Idle, otherwise a no-op.
/// Toggle-mode stops and in-flight cancels stay on the toggle path so a
/// desktop-bound start shortcut can never stop or discard a session.
pub async fn handle_external_activation_start(
    state: tauri::State<'_, AppState>,
    app_handle: tauri::AppHandle,
) {
    let session = *state.session_state.lock().unwrap();

    crate::log_info!("External activation start: session_state={:?}", session);

    if session == SessionState::Idle {
        let _ = start_recording(state, app_handle).await;
    }
}

/// External activation (`voquill --record-stop`) into the existing
/// recording lifecycle: stops an active recording, otherwise a no-op.
pub async fn handle_external_activation_stop(state: tauri::State<'_, AppState>) {
    let session = *state.session_state.lock().unwrap();

    crate::log_info!("External activation stop: session_state={:?}", session);

    if session == SessionState::Recording {
        let _ = stop_recording(state).await;
    }
}
