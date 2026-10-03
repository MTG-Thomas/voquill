use std::path::PathBuf;

use serde::Serialize;
use tauri::Manager;

use crate::platform::linux::wayland::portal::capabilities::{
    probe_global_shortcuts, GlobalShortcutsProbe,
};

/// Wayland push-to-talk activation provider.
///
/// `GlobalShortcutsPortal` is the canonical path: Voquill owns its shortcut
/// through `org.freedesktop.portal.GlobalShortcuts`. `ExternalDesktopBinding`
/// is the fallback for desktops whose portal backend does not implement
/// GlobalShortcuts (e.g. COSMIC): the desktop owns the key binding and
/// invokes Voquill through the thin local control surface below
/// (`voquill --record-start/--record-stop/--record-toggle`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivationProvider {
    GlobalShortcutsPortal,
    ExternalDesktopBinding,
}

impl ActivationProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GlobalShortcutsPortal => "global-shortcuts",
            Self::ExternalDesktopBinding => "external-desktop-binding",
        }
    }
}

/// Select the activation provider from probed portal capabilities.
///
/// GlobalShortcuts stays the preferred/default path everywhere it is
/// available and usable (version >= 1). The external provider is selected
/// only when the portal is absent or unusable — never by desktop name.
pub fn select_activation_provider(probe: &GlobalShortcutsProbe) -> ActivationProvider {
    if probe.available && probe.version >= 1 {
        ActivationProvider::GlobalShortcutsPortal
    } else {
        ActivationProvider::ExternalDesktopBinding
    }
}

pub async fn select_current_activation_provider() -> ActivationProvider {
    select_activation_provider(&probe_global_shortcuts().await)
}

/// Action requested through the external activation control surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternalActivationCommand {
    Start,
    Stop,
    Toggle,
}

impl ExternalActivationCommand {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Toggle => "toggle",
        }
    }

    pub fn cli_flag(&self) -> &'static str {
        match self {
            Self::Start => "--record-start",
            Self::Stop => "--record-stop",
            Self::Toggle => "--record-toggle",
        }
    }
}

/// Parse one protocol line (`start`, `stop`, `toggle`, case-insensitive).
/// Returns `None` for anything else so malformed input is rejected loudly.
pub fn parse_external_activation_command(line: &str) -> Option<ExternalActivationCommand> {
    match line.trim().to_ascii_lowercase().as_str() {
        "start" => Some(ExternalActivationCommand::Start),
        "stop" => Some(ExternalActivationCommand::Stop),
        "toggle" => Some(ExternalActivationCommand::Toggle),
        _ => None,
    }
}

fn activation_binary_name() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_string())
        })
        .unwrap_or_else(|| "voquill".to_string())
}

#[derive(Clone, Debug, Serialize)]
pub struct ExternalActivationGuidance {
    pub start_command: String,
    pub stop_command: String,
    pub toggle_command: String,
    pub setup_hint: String,
}

/// Copy-pasteable commands for binding Voquill in desktop keyboard settings
/// (COSMIC Settings -> Keyboard -> Custom Shortcuts, or equivalent).
pub fn external_activation_guidance() -> ExternalActivationGuidance {
    let binary = activation_binary_name();
    ExternalActivationGuidance {
        start_command: format!("{binary} --record-start"),
        stop_command: format!("{binary} --record-stop"),
        toggle_command: format!("{binary} --record-toggle"),
        setup_hint: "Bind one command per shortcut. Use start/stop on two shortcuts for push-to-talk, or toggle on a single shortcut.".to_string(),
    }
}

/// Runtime socket for external activation requests. Per-user and
/// session-scoped via the runtime dir so concurrent sessions do not collide.
pub fn external_activation_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        if !runtime_dir.trim().is_empty() {
            return PathBuf::from(runtime_dir)
                .join("voquill")
                .join("activation.sock");
        }
    }

    let user_id = nix::unistd::Uid::current().as_raw();
    std::env::temp_dir()
        .join(format!("voquill-{user_id}"))
        .join("activation.sock")
}

/// Best-effort probe: is a Voquill instance currently serving external
/// activation requests? Used by diagnostics; never binds or mutates state.
pub fn is_external_activation_available() -> bool {
    let socket_path = external_activation_socket_path();
    std::os::unix::net::UnixStream::connect(socket_path).is_ok()
}

/// Bind the external activation socket. Synchronous so startup can bind
/// before the activation engine probes, eliminating any bind/probe race.
pub fn prepare_external_activation_listener() -> Result<tokio::net::UnixListener, String> {
    let socket_path = external_activation_socket_path();
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create activation socket dir: {error}"))?;
    }

    match tokio::net::UnixListener::bind(&socket_path) {
        Ok(listener) => Ok(listener),
        Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => {
            // A socket file already exists. If a live instance owns it, back
            // off silently; otherwise it is stale, so reclaim it.
            if is_external_activation_available() {
                return Err(
                    "External activation socket is already served by another Voquill instance."
                        .to_string(),
                );
            }
            let _ = std::fs::remove_file(&socket_path);
            tokio::net::UnixListener::bind(&socket_path)
                .map_err(|error| format!("Failed to bind activation socket: {error}"))
        }
        Err(error) => Err(format!("Failed to bind activation socket: {error}")),
    }
}

async fn handle_activation_connection(
    stream: tokio::net::UnixStream,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

    let (reader, mut writer) = stream.into_split();
    let mut reader = tokio::io::BufReader::new(reader);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .await
        .map_err(|error| format!("Failed to read activation request: {error}"))?;

    let Some(command) = parse_external_activation_command(&line) else {
        let response = format!("err unknown activation command: '{}'\n", line.trim());
        let _ = writer.write_all(response.as_bytes()).await;
        return Err(format!(
            "Rejected malformed external activation request: '{}'",
            line.trim()
        ));
    };

    crate::log_info!(
        "External activation request: command='{}'",
        command.as_str()
    );

    let state = app_handle.state::<crate::AppState>();
    match command {
        ExternalActivationCommand::Start => {
            crate::app::hotkey_handler::handle_external_activation_start(state, app_handle.clone())
                .await;
        }
        ExternalActivationCommand::Stop => {
            crate::app::hotkey_handler::handle_external_activation_stop(state).await;
        }
        ExternalActivationCommand::Toggle => {
            crate::app::hotkey_handler::handle_hotkey_press(state, app_handle.clone()).await;
        }
    }

    writer
        .write_all(b"ok\n")
        .await
        .map_err(|error| format!("Failed to write activation response: {error}"))?;
    Ok(())
}

/// Serve external activation requests for the lifetime of the app.
/// Spawned once at startup on Linux; bind failures are logged, never fatal.
pub async fn serve_external_activation_requests(app_handle: tauri::AppHandle) {
    let listener = match prepare_external_activation_listener() {
        Ok(listener) => listener,
        Err(error) => {
            crate::log_warn!("External activation unavailable: {}", error);
            return;
        }
    };

    serve_external_activation_listener(listener, app_handle).await;
}

/// Accept loop for an already-bound activation socket. Prefer binding
/// early via [`prepare_external_activation_listener`] during startup so
/// the activation engine probe never races the bind.
pub async fn serve_external_activation_listener(
    listener: tokio::net::UnixListener,
    app_handle: tauri::AppHandle,
) {
    crate::log_info!(
        "External activation listening on {}",
        external_activation_socket_path().display()
    );

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(accepted) => accepted,
            Err(error) => {
                crate::log_warn!("External activation accept failed: {}", error);
                continue;
            }
        };

        let app_handle = app_handle.clone();
        tauri::async_runtime::spawn(async move {
            if let Err(error) = handle_activation_connection(stream, app_handle).await {
                crate::log_warn!("{}", error);
            }
        });
    }
}

/// CLI entry point for `--record-start/--record-stop/--record-toggle`.
/// Forwards one command to the running instance and returns the process
/// exit code. Never boots Tauri.
pub fn run_external_activation_cli(
    record_start: bool,
    record_stop: bool,
    record_toggle: bool,
) -> i32 {
    let requested = [record_start, record_stop, record_toggle]
        .iter()
        .filter(|selected| **selected)
        .count();
    if requested > 1 {
        eprintln!("Only one of --record-start, --record-stop, --record-toggle may be given.");
        return 2;
    }

    let command = if record_start {
        ExternalActivationCommand::Start
    } else if record_stop {
        ExternalActivationCommand::Stop
    } else {
        ExternalActivationCommand::Toggle
    };

    match send_external_activation_command(command) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}

fn send_external_activation_command(command: ExternalActivationCommand) -> Result<(), String> {
    use std::io::{BufRead, Write};

    let socket_path = external_activation_socket_path();
    let mut stream = std::os::unix::net::UnixStream::connect(&socket_path).map_err(|_| {
        "Voquill does not appear to be running (external activation socket unavailable). Start Voquill first, then retry.".to_string()
    })?;

    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .map_err(|error| format!("Failed to configure activation socket: {error}"))?;
    stream
        .set_write_timeout(Some(std::time::Duration::from_secs(5)))
        .map_err(|error| format!("Failed to configure activation socket: {error}"))?;

    writeln!(stream, "{}", command.as_str())
        .map_err(|error| format!("Failed to send activation request: {error}"))?;

    let mut reader = std::io::BufReader::new(stream);
    let mut response = String::new();
    reader
        .read_line(&mut response)
        .map_err(|error| format!("Failed to read activation response: {error}"))?;

    let response = response.trim();
    if response == "ok" {
        Ok(())
    } else if let Some(detail) = response.strip_prefix("err ") {
        Err(format!("Activation request rejected: {detail}"))
    } else if response.is_empty() {
        Err("Activation request failed: empty response from Voquill.".to_string())
    } else {
        Err(format!(
            "Activation request failed: unexpected response '{response}'."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        external_activation_guidance, parse_external_activation_command,
        select_activation_provider, ActivationProvider, ExternalActivationCommand,
    };
    use crate::platform::linux::wayland::portal::capabilities::GlobalShortcutsProbe;

    fn probe(available: bool, version: u32) -> GlobalShortcutsProbe {
        GlobalShortcutsProbe { available, version }
    }

    #[test]
    fn portal_available_v1_selects_global_shortcuts() {
        assert_eq!(
            select_activation_provider(&probe(true, 1)),
            ActivationProvider::GlobalShortcutsPortal
        );
    }

    #[test]
    fn portal_available_v2_selects_global_shortcuts() {
        assert_eq!(
            select_activation_provider(&probe(true, 2)),
            ActivationProvider::GlobalShortcutsPortal
        );
    }

    #[test]
    fn portal_absent_selects_external_binding() {
        assert_eq!(
            select_activation_provider(&probe(false, 0)),
            ActivationProvider::ExternalDesktopBinding
        );
    }

    #[test]
    fn portal_available_but_unusable_version_selects_external_binding() {
        assert_eq!(
            select_activation_provider(&probe(true, 0)),
            ActivationProvider::ExternalDesktopBinding
        );
    }

    #[test]
    fn provider_names_are_stable() {
        assert_eq!(
            ActivationProvider::GlobalShortcutsPortal.as_str(),
            "global-shortcuts"
        );
        assert_eq!(
            ActivationProvider::ExternalDesktopBinding.as_str(),
            "external-desktop-binding"
        );
    }

    #[test]
    fn parse_accepts_known_commands_case_insensitively() {
        assert_eq!(
            parse_external_activation_command("start\n"),
            Some(ExternalActivationCommand::Start)
        );
        assert_eq!(
            parse_external_activation_command("  STOP  "),
            Some(ExternalActivationCommand::Stop)
        );
        assert_eq!(
            parse_external_activation_command("Toggle"),
            Some(ExternalActivationCommand::Toggle)
        );
    }

    #[test]
    fn parse_rejects_unknown_commands() {
        assert_eq!(parse_external_activation_command(""), None);
        assert_eq!(parse_external_activation_command("record"), None);
        assert_eq!(parse_external_activation_command("start stop"), None);
        assert_eq!(parse_external_activation_command("rm -rf /"), None);
    }

    #[test]
    fn command_flags_match_documented_cli_surface() {
        assert_eq!(
            ExternalActivationCommand::Start.cli_flag(),
            "--record-start"
        );
        assert_eq!(ExternalActivationCommand::Stop.cli_flag(), "--record-stop");
        assert_eq!(
            ExternalActivationCommand::Toggle.cli_flag(),
            "--record-toggle"
        );
    }

    #[test]
    fn guidance_commands_carry_cli_flags() {
        let guidance = external_activation_guidance();
        assert!(guidance.start_command.ends_with(" --record-start"));
        assert!(guidance.stop_command.ends_with(" --record-stop"));
        assert!(guidance.toggle_command.ends_with(" --record-toggle"));
        assert!(!guidance.setup_hint.is_empty());
    }
}
