# Wayland Portal Compatibility Matrix

This document tracks expected behavior for Wayland portal integrations, with emphasis on `org.freedesktop.portal.GlobalShortcuts`.

## Scope

- Platform: Linux Wayland sessions (portal path). X11 uses native backends and is tracked outside this document.
- Features: Global shortcuts, input emulation, microphone access
- Backends: GNOME, KDE (others may work if they implement the required portal interfaces)

## Current Baseline

| Environment | Portal Backend | GlobalShortcuts Version | Expected Flow | Notes |
| --- | --- | --- | --- | --- |
| Fedora GNOME 49 | `xdg-desktop-portal-gnome` | 1 | Bind/List | No `ConfigureShortcuts`; use explicit bind flow |
| KDE Plasma 6 (Wayland) | `xdg-desktop-portal-kde` | 1+ (varies) | Bind/List or Configure when available | Version-dependent behavior |
| COSMIC (Wayland) | `xdg-desktop-portal-cosmic` | absent | External desktop binding | No GlobalShortcuts; RemoteDesktop present (see COSMIC section) |

## Runtime Rules

1. Detect portal capabilities at runtime.
2. Choose flow by capability, not distro name.
3. Prefer GlobalShortcuts everywhere available; select the external desktop binding only when the portal is absent or unusable (version 0).
4. Keep one active shortcut session owner and close old sessions on replacement.
5. Surface actionable errors to logs and UI.

## Activation providers

Wayland push-to-talk activation goes through one provider, selected at
runtime from the GlobalShortcuts presence/version probe
(`select_activation_provider` in
`src-tauri/src/platform/linux/wayland/activation.rs`):

- `global-shortcuts`: canonical path. Voquill owns the `record` shortcut
  through `org.freedesktop.portal.GlobalShortcuts` (GNOME, KDE, and any
  desktop whose portal backend implements it).
- `external-desktop-binding`: fallback for desktops without GlobalShortcuts
  (e.g. COSMIC). The desktop owns the key binding; Voquill exposes a thin
  local control surface for binding from system settings:
  - `voquill --record-start` — start recording (no-op unless idle)
  - `voquill --record-stop` — stop recording (no-op unless recording)
  - `voquill --record-toggle` — press semantics (start/stop/cancel per mode)
  - Transport: Unix socket at `$XDG_RUNTIME_DIR/voquill/activation.sock`
    (fallback `/tmp/voquill-<uid>/activation.sock`), protocol `start|stop|toggle`
    line in, `ok` / `err <detail>` line out.

All three commands route into the existing recording/session state machine
(`app::hotkey_handler`); there is no parallel recording lifecycle. Push-talk
remains possible by binding start/stop on two desktop shortcuts.

Diagnostics (`get_portal_diagnostics`) report `activation_provider`,
`external_activation_available`, and `external_activation_detail` alongside
the GlobalShortcuts fields, so COSMIC reports "app-managed global shortcuts:
unsupported; external desktop shortcut activation: available" instead of a
generic unsupported Wayland state. Setup readiness reports
`shortcuts_status="external"` in that case, and `configure_hotkey` returns
the `external_activation` outcome with binding guidance.

## COSMIC RemoteDesktop verification (unverified)

The existing RemoteDesktop input path (keyboard device selection,
`PersistMode::ExplicitlyRevoked`, restore tokens, reconnect after session
expiration, synthesized paste shortcuts, typewriter key events) is reused
unchanged on COSMIC. The following have NOT been runtime-verified against
`xdg-desktop-portal-cosmic` (no COSMIC compositor available to this project
yet); verify on a current COSMIC Wayland session before treating COSMIC as
equivalent to mature GNOME/KDE portal behavior:

- keyboard device selection succeeds
- `PersistMode::ExplicitlyRevoked` accepted
- restore tokens issued and accepted on reconnect
- reconnect after session expiration recovers typing
- synthesized paste shortcuts delivered
- normal typewriter key events delivered

If restore-token persistence proves unreliable on COSMIC, surface the
limitation through diagnostics/setup status — do not add hidden retries or
compositor-specific branches in the generic path.

## Verification Checklist

- `get_portal_diagnostics` returns `available=true` on supported Wayland systems.
- `get_linux_setup_status` reports `shortcuts=true` only when `record` is actually bound.
- Initial startup reuses existing shortcut binding when present.
- Manual bind path triggers portal request and persists trigger description.
- Press/release emits recording start/stop transitions.
- On a desktop without GlobalShortcuts: diagnostics report
  `activation_provider="external-desktop-binding"`, setup reports
  `shortcuts_status="external"`, and `voquill --record-toggle` toggles
  recording in the running instance.

## Fedora GNOME release-order edge case (GlobalShortcuts)

### Observed behavior

- Environment: Fedora GNOME Wayland with `xdg-desktop-portal-gnome`
- Shortcut: `Ctrl+Shift+Space` push-to-talk hold flow
- Failure pattern: portal emits repeated `Activated` events (roughly every 30ms) and does not emit a matching `Deactivated` for that shortcut cycle when keys are released in a problematic order
- Recovery pattern: a later press/release often emits `Activated` then `Deactivated`, which finally unlatches recording

### Rationale for Voquill fallback

- Push-to-talk must never remain latched after keys are physically released
- We keep the native portal as the primary source of truth (`Activated` starts, `Deactivated` stops)
- We add a defensive repeat-heartbeat fallback only for this edge case class:
  - detect rapid repeated `Activated` cadence while already recording
  - treat the cadence as a temporary heartbeat mode
  - if heartbeat stops for a short silence window and recording is still active, force `stop_recording`

### Current thresholds

- `REPEAT_ACTIVATION_WINDOW_MS = 120`
- `REPEAT_SILENCE_TIMEOUT_MS = 220`
- `REPEAT_WATCHDOG_TICK_MS = 50`

These values are intentionally conservative and should be tuned only with portal event logs from affected systems.

## Troubleshooting Signals

- `status=unavailable`: Portal service/backend missing or inaccessible.
- `status=unsupported`: Wayland not available or portal version not usable.
- `status=error`: Portal call failed (inspect `detail` for root cause).
- `shortcuts=false` with `status=ready`: Portal available but no `record` binding yet.
- `activation_provider=external-desktop-binding`: GlobalShortcuts unavailable;
  bind `voquill --record-toggle` (or start/stop) as a desktop custom shortcut.
- `external_activation_available=false` with the external provider: the
  activation listener failed to bind (see logs); restart Voquill.
