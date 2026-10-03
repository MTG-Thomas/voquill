use ashpd::desktop::global_shortcuts::GlobalShortcuts;
use serde::Serialize;

#[derive(Clone, Copy, Debug)]
pub struct GlobalShortcutsCapabilities {
    pub version: u32,
    pub supports_configure_shortcuts: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct PortalDiagnostics {
    pub available: bool,
    pub version: u32,
    pub supports_configure_shortcuts: bool,
    pub has_record_shortcut: bool,
    pub active_trigger: Option<String>,
    pub status: String,
    pub detail: Option<String>,
    pub activation_provider: String,
    pub external_activation_available: bool,
    pub external_activation_detail: Option<String>,
}

/// Lightweight GlobalShortcuts presence/version probe used for activation
/// provider selection. Unlike full diagnostics, it creates no session and
/// performs no shortcut calls.
#[derive(Clone, Copy, Debug)]
pub struct GlobalShortcutsProbe {
    pub available: bool,
    pub version: u32,
}

pub async fn probe_global_shortcuts() -> GlobalShortcutsProbe {
    let unavailable = GlobalShortcutsProbe {
        available: false,
        version: 0,
    };

    let proxy = match GlobalShortcuts::new().await {
        Ok(proxy) => proxy,
        Err(_) => return unavailable,
    };

    use std::ops::Deref;
    match proxy.deref().get_property::<u32>("version").await {
        Ok(version) => GlobalShortcutsProbe {
            available: true,
            version,
        },
        Err(_) => unavailable,
    }
}

pub async fn detect_global_shortcuts_capabilities() -> Result<GlobalShortcutsCapabilities, String> {
    let proxy = GlobalShortcuts::new()
        .await
        .map_err(|error| format!("Failed to connect to GlobalShortcuts portal: {error}"))?;

    use std::ops::Deref;
    let version = proxy
        .deref()
        .get_property::<u32>("version")
        .await
        .map_err(|error| format!("Failed to read GlobalShortcuts portal version: {error}"))?;

    Ok(GlobalShortcutsCapabilities {
        version,
        supports_configure_shortcuts: version >= 2,
    })
}

pub async fn collect_global_shortcuts_diagnostics() -> PortalDiagnostics {
    use crate::platform::linux::wayland::activation::{
        external_activation_guidance, is_external_activation_available, select_activation_provider,
        ActivationProvider,
    };

    let external_activation_available = is_external_activation_available();
    let external_detail = |provider: ActivationProvider| {
        if provider == ActivationProvider::ExternalDesktopBinding {
            let guidance = external_activation_guidance();
            Some(format!(
                "App-managed global shortcuts are unsupported on this desktop. Bind '{}' (or '{}' / '{}' for push-to-talk) as a custom shortcut in system settings.",
                guidance.toggle_command, guidance.start_command, guidance.stop_command
            ))
        } else {
            None
        }
    };

    let proxy = match GlobalShortcuts::new().await {
        Ok(proxy) => proxy,
        Err(error) => {
            let provider = select_activation_provider(&GlobalShortcutsProbe {
                available: false,
                version: 0,
            });
            return PortalDiagnostics {
                available: false,
                version: 0,
                supports_configure_shortcuts: false,
                has_record_shortcut: false,
                active_trigger: None,
                status: "unavailable".to_string(),
                detail: Some(format!("GlobalShortcuts portal unavailable: {error}")),
                activation_provider: provider.as_str().to_string(),
                external_activation_available,
                external_activation_detail: external_detail(provider),
            };
        }
    };

    let capabilities = match detect_global_shortcuts_capabilities().await {
        Ok(capabilities) => capabilities,
        Err(error) => {
            return PortalDiagnostics {
                available: true,
                version: 0,
                supports_configure_shortcuts: false,
                has_record_shortcut: false,
                active_trigger: None,
                status: "error".to_string(),
                detail: Some(error),
                activation_provider: ActivationProvider::GlobalShortcutsPortal
                    .as_str()
                    .to_string(),
                external_activation_available,
                external_activation_detail: None,
            };
        }
    };
    let provider = select_activation_provider(&GlobalShortcutsProbe {
        available: true,
        version: capabilities.version,
    });

    let session = match proxy.create_session().await {
        Ok(session) => session,
        Err(error) => {
            return PortalDiagnostics {
                available: true,
                version: capabilities.version,
                supports_configure_shortcuts: capabilities.supports_configure_shortcuts,
                has_record_shortcut: false,
                active_trigger: None,
                status: "error".to_string(),
                detail: Some(format!("Failed to create GlobalShortcuts session: {error}")),
                activation_provider: provider.as_str().to_string(),
                external_activation_available,
                external_activation_detail: external_detail(provider),
            };
        }
    };

    let list_response = proxy.list_shortcuts(&session).await;
    let diagnostics = match list_response {
        Ok(request) => match request.response() {
            Ok(listed) => {
                let active_trigger = listed
                    .shortcuts()
                    .iter()
                    .find(|shortcut| shortcut.id() == "record")
                    .map(|shortcut| shortcut.trigger_description().to_string());

                PortalDiagnostics {
                    available: true,
                    version: capabilities.version,
                    supports_configure_shortcuts: capabilities.supports_configure_shortcuts,
                    has_record_shortcut: active_trigger.is_some(),
                    active_trigger,
                    status: if capabilities.version >= 1 {
                        "ready".to_string()
                    } else {
                        "unsupported".to_string()
                    },
                    detail: None,
                    activation_provider: provider.as_str().to_string(),
                    external_activation_available,
                    external_activation_detail: external_detail(provider),
                }
            }
            Err(error) => PortalDiagnostics {
                available: true,
                version: capabilities.version,
                supports_configure_shortcuts: capabilities.supports_configure_shortcuts,
                has_record_shortcut: false,
                active_trigger: None,
                status: "error".to_string(),
                detail: Some(format!("Failed to parse ListShortcuts response: {error}")),
                activation_provider: provider.as_str().to_string(),
                external_activation_available,
                external_activation_detail: external_detail(provider),
            },
        },
        Err(error) => PortalDiagnostics {
            available: true,
            version: capabilities.version,
            supports_configure_shortcuts: capabilities.supports_configure_shortcuts,
            has_record_shortcut: false,
            active_trigger: None,
            status: "error".to_string(),
            detail: Some(format!("Failed to call ListShortcuts: {error}")),
            activation_provider: provider.as_str().to_string(),
            external_activation_available,
            external_activation_detail: external_detail(provider),
        },
    };

    let _ = session.close().await;
    diagnostics
}
