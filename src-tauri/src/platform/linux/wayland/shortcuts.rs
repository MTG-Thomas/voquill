pub use crate::platform::linux::wayland::portal::global_shortcuts::{
    normalize_wayland_trigger, start_linux_portal_hotkey_engine,
    try_open_linux_portal_shortcut_configuration,
};

use crate::platform::linux::wayland::activation::{
    external_activation_guidance, is_external_activation_available,
    select_current_activation_provider, ActivationProvider,
};

/// Start the Wayland activation engine for the selected provider.
///
/// GlobalShortcuts stays the preferred path everywhere available; on
/// desktops without it (e.g. COSMIC) the external desktop binding is
/// selected and startup succeeds without a portal shortcut, since the
/// desktop owns the key binding and invokes Voquill externally.
pub async fn start_wayland_activation_engine(
    app_handle: tauri::AppHandle,
    force: bool,
) -> Result<(), String> {
    let provider = select_current_activation_provider().await;
    crate::log_info!(
        "Wayland activation provider selected: {}",
        provider.as_str()
    );

    if provider == ActivationProvider::GlobalShortcutsPortal {
        return start_linux_portal_hotkey_engine(app_handle, force).await;
    }

    let guidance = external_activation_guidance();
    if is_external_activation_available() {
        crate::set_hotkey_binding_state(
            &app_handle,
            true,
            false,
            Some(format!(
                "Desktop-managed shortcut: bind '{}' (or '{}' / '{}') as a custom shortcut in system settings.",
                guidance.toggle_command, guidance.start_command, guidance.stop_command
            )),
            None,
        );
        Ok(())
    } else {
        let detail = "External activation unavailable: the Voquill activation listener is not running. Restart Voquill and retry.".to_string();
        crate::set_hotkey_binding_state(&app_handle, false, false, Some(detail.clone()), None);
        Err(detail)
    }
}
