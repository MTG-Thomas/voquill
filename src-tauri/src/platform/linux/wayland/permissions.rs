use crate::config::Config;
use crate::platform::linux::wayland::input;
use crate::platform::permissions::LinuxPermissions;
use ashpd::desktop::camera::Camera;
use tauri::AppHandle;

pub async fn check_linux_permissions(config: &Config) -> LinuxPermissions {
    let audio = Camera::new().await.is_ok();

    let input_emulation = config.input_token.is_some();

    let provider =
        crate::platform::linux::wayland::activation::select_current_activation_provider().await;
    let (shortcuts, shortcuts_status, shortcuts_detail) = if provider
        == crate::platform::linux::wayland::activation::ActivationProvider::ExternalDesktopBinding
    {
        let available =
            crate::platform::linux::wayland::activation::is_external_activation_available();
        let guidance = crate::platform::linux::wayland::activation::external_activation_guidance();
        (
            available,
            "external".to_string(),
            Some(format!(
                "Desktop-managed shortcut activation via '{}'.",
                guidance.toggle_command
            )),
        )
    } else {
        let bound = config.shortcuts_token.is_some();
        (
            bound,
            if bound {
                "bound".to_string()
            } else {
                "unbound".to_string()
            },
            None,
        )
    };

    LinuxPermissions {
        audio,
        shortcuts,
        input_emulation,
        shortcuts_status,
        shortcuts_detail,
        manual_overlay_offset_supported: false,
        overlay_positioning_detail: Some(
            "Manual overlay position adjustment is not available on your system.".to_string(),
        ),
    }
}

pub async fn request_linux_permissions(app_handle: AppHandle) -> Result<(), String> {
    // 1. Request Audio (Mic) via Camera Portal
    let camera = Camera::new().await.map_err(|e| {
        format!(
            "Audio Portal not available: {}. Is xdg-desktop-portal-gtk/kde installed?",
            e
        )
    })?;
    camera
        .request_access()
        .await
        .map_err(|e| format!("Audio access denied: {}", e))?;

    // 2. Request Input Emulation via Wayland Remote Desktop Portal
    input::establish_input_session(&app_handle, true).await?;

    Ok(())
}
