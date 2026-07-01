# Voquill Agent Manifesto & Guidelines

This document serves as a constitution for all agentic coding entities (and humans) operating within the Voquill repository. Integrity, cleanliness, and architectural soundness are our primary metrics of success.

---

## Quick Navigation

| Topic                         | Document                                                            |
| ----------------------------- | ------------------------------------------------------------------- |
| Build and dev commands        | This file, [BUILD.md](docs/BUILD.md)                                |
| CI and pre-PR verification    | [REPO_HYGIENE.md](docs/REPO_HYGIENE.md)                             |
| Wayland portal quirks         | [PORTAL_COMPATIBILITY.md](docs/PORTAL_COMPATIBILITY.md)             |
| Windows audio device naming   | [WINDOWS_AUDIO_HANDOVER.md](docs/WINDOWS_AUDIO_HANDOVER.md)         |
| Product architecture overview | [ARCHITECTURE.md](docs/ARCHITECTURE.md)                             |
| Code vs philosophy gaps       | [GitHub issue #16](https://github.com/MTG-Thomas/voquill/issues/16) |

---

## Repo Map

```
voquill/                          # npm root — scripts, Vite, Preact UI
├── src/                          # Frontend (Preact + TypeScript)
│   ├── pages/                    # StatusPage, ConfigPage, HistoryPage, InitialSetupPage, UiLabPage
│   ├── components/               # Reusable UI (Button, Modal, ModelSelectionPanel, …)
│   ├── theme/                    # ui-primitives.ts, component-styles.ts
│   ├── design-tokens.ts          # Shared spacing, color, typography tokens
│   ├── App.tsx                   # Main window shell and tab routing
│   └── Overlay.tsx               # Always-on-top status overlay
├── src-tauri/                    # Rust backend (Tauri 2)
│   ├── src/
│   │   ├── main.rs               # App entry; registers invoke_handler commands
│   │   ├── app/                  # Bootstrap, AppState, recording_flow, status, session_log
│   │   │   └── commands/         # Tauri commands (config, recording, hotkey, transcription, …)
│   │   ├── platform/             # OS/display backends (traits + per-platform impls)
│   │   │   ├── linux/wayland/    # XDG Portals (ashpd), portal capabilities
│   │   │   ├── linux/x11/        # Native X11 shortcuts, input, overlay
│   │   │   ├── windows/          # WASAPI, SendInput, global shortcuts
│   │   │   └── macos/            # Experimental — not a release target
│   │   ├── audio.rs              # Capture, device enumeration
│   │   ├── typing.rs             # Keystroke injection orchestration
│   │   ├── transcription.rs      # TranscriptionService trait + API backend
│   │   ├── local_whisper.rs      # whisper.cpp local engine
│   │   ├── openvino_whisper.rs   # OpenVINO engine (Windows)
│   │   ├── mlx_whisper.rs        # MLX engine (macOS)
│   │   ├── model_manager.rs      # Model catalog, download, path resolution
│   │   └── config.rs             # User configuration load/save
│   ├── capabilities/             # Tauri capability manifests
│   └── tauri.conf.json
├── scripts/                      # deps:check, cargo-runner, tauri-runner, hardening checks
└── docs/                         # Architecture, build, portal, and handover docs
```

**Build output locations**

- **Linux:** `src-tauri/target/release/bundle/`
- **Windows (via `npm run cargo:*` / `npm run tauri:*`):** `C:\voquill-build\release\bundle/` — the cargo runner sets `CARGO_TARGET_DIR` to avoid long-path failures.

---

## 🏛️ The Voquill Philosophy

### 1. Integrity Over Expediency

We do not value "quick hacks" that work today but create technical debt for tomorrow. If a feature or fix cannot be implemented cleanly, pause and find a proper architectural solution — or deliver incrementally through a clean seam rather than a workaround.

- **No Shortcuts:** "Temporary" workarounds are forbidden. If a platform (like Wayland) restricts an action, we find the compliant API (like XDG Portals) instead of forcing a legacy hack.
- **No Half-Efforts:** Features must be substantially complete and polished. This includes proper error handling, logging, and UI feedback.
- **Clean Seams Over Hacks:** Ship features incrementally when each step has a clear boundary. Prefer a missing feature over a messy implementation, but do not block reasonable progress waiting for a perfect refactor.

### 2. Neatness, Tidiness, and OCD-Standard Code

Code is for humans to read, and only secondarily for machines to execute.

- **Semantic Clarity:** In new or touched code, use descriptive names. Avoid abbreviations like `amt` for `amount` or `idx` for `index`. Do not rename unrelated symbols just to satisfy naming rules.
- **Single Responsibility:** Functions and modules must do one thing and do it well. Large functions should be decomposed into logical units.
- **Formatting:** Strict adherence to `cargo fmt` and `npm run typecheck`.
- **Scoped Cleanup:** If you see messy code near your change, mention it in your response. Implement cleanup only when the user asks, or when it is inseparable from the task at hand. Do not expand scope to unrelated files.

### 3. Linux Display Server Support

Linux support targets both Wayland and X11, with clear platform boundaries.

- **Wayland Path:** Use **XDG Portals** (via `ashpd`) for hardware access (Microphone, Shortcuts, Input Emulation).
- **X11 Path:** Use native X11-compatible backends for shortcuts/input while keeping behavior aligned with Wayland as closely as possible.
- **Compositor Awareness:** Recognize that Wayland compositors (GNOME, KDE, Hyprland) have strict security models; keep those integrations explicit and future-proof.
- **Primary Delivery:** Prefer distro-native Linux packages (`.deb` / `.rpm`) where possible, and treat AppImage as the cross-distro fallback.

### 4. Root Cause First

We solve problems at their origin. If data is messy, redundant, or incorrect, do not "clean it up" at the consumer level (e.g., in the UI or intermediate wrappers). Trace the data back to its absolute source of truth and fix the generation/fetching logic there. A workaround is technical debt; a root-cause fix is engineering.

**Example:** If microphone labels are generic in the UI dropdown, fix enumeration in `audio.rs` — do not filter or relabel in the Preact component.

### 5. Lean, Durable Architecture (No Bloat)

We design for long-term maintainability as a solo-developed project. Architecture must remain clean and scalable without over-engineering.

- **Capability-Driven, Not Distro-Driven:** Organize by platform and protocol capabilities, not by distro names. Prefer runtime capability detection over hardcoded Fedora/GNOME/KDE branching.
- **One Owner Per Concern:** Session lifecycle, portal API integration, state transitions, and UI mapping should each have a clear single owner.
- **No Abstraction Without Payoff:** New modules or traits must reduce duplication, simplify reasoning, or improve reliability. Avoid "future-proof" layers that are unused.
- **Small, Localized Change Surface:** Future platform changes (portal updates, new compositor behavior) should require minor edits in capability/adapter modules, not architectural rewrites.
- **State Machines Over Ad-Hoc Flags:** For non-trivial flows (permissions, hotkeys, portal sessions), prefer explicit state transitions over scattered booleans. When touching these flows, move toward explicit states — do not add new standalone boolean flags. See [Architectural Debt](#architectural-debt) for current gaps.

### 6. Platform Adaptation Pattern

When implementing platform-sensitive features, follow this structure:

1. **Platform Boundary First:** Keep OS/display boundaries (`linux/wayland`, `linux/x11`, `windows`) as top-level separations.
2. **Provider Layer Second:** Within a platform, isolate backend/provider behavior (e.g., portal capabilities and session handling).
3. **Quirks Last:** Only add DE/provider-specific quirk modules when a real incompatibility is confirmed and cannot be solved generically.

This pattern keeps the codebase clean as new distros, compositor versions, or portal changes appear.

---

## 🛠️ Essential Commands

### Project-wide (Root)

Managed via **npm** scripts and the Tauri CLI.

- **Dependency Check:** `npm run deps:check`
  - Verifies required system dependencies and prints install commands when missing.
- **Dev:** `npm run tauri:dev`
  - Runs dependency checks and starts the Tauri development server.
- **Build:** `npm run tauri:build`
  - Runs dependency checks, builds the frontend, and packages the app.
- **Tauri CLI:** `npm run tauri -- <command>`
  - Use for tauri-specific tasks like `tauri icon` or `tauri info`.

### Backend (`src-tauri/`)

- **Lint:** `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` and `cargo fmt --manifest-path src-tauri/Cargo.toml`
- **Check:** `cargo check --manifest-path src-tauri/Cargo.toml`. On Windows from a normal shell, prefer `npm run cargo:check` so the short Cargo target directory and Visual Studio/Vulkan toolchain environment are loaded consistently.
- **Test:** `cargo test --manifest-path src-tauri/Cargo.toml`
- **Single Test:** `cargo test --manifest-path src-tauri/Cargo.toml -- <name>`
- **Doc:** `cargo doc --manifest-path src-tauri/Cargo.toml --open`

### Frontend (`src/`)

- **Type Check:** `npm run typecheck`
- **Lint:** `npm run lint`
- **Format Check:** `npm run format:check`
- **Dev Server:** `npm run dev` — Vite only, for UI-only iteration
- **Preview:** `npm run preview`

### Pre-completion verification

Run the full checklist in [REPO_HYGIENE.md](docs/REPO_HYGIENE.md) before declaring a task complete or opening a PR. At minimum:

```powershell
npm run format:check
npm run lint
npm run typecheck
npm run harden:check
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

On Windows, prefer `npm run cargo:check` for a fast compile check when the full clippy run is not needed mid-task.

---

## 🏗️ Architecture & Patterns

### 1. Backend (Rust)

- **Async Flow:** Use `tokio` or `tauri::async_runtime` for all I/O, network, and audio operations. Never block the main thread.
- **Error Handling:** Use `anyhow` for internal propagation to maintain context.
- **Command Safety:** Return `Result<T, String>` for all `#[tauri::command]` functions. The error string is what the frontend `Promise.reject` receives.
- **Command Organization:** Implement commands in `src-tauri/src/app/commands/` and register them in `main.rs` via `generate_handler!`.
- **State Management:** Use `AppState` (managed by Tauri) for shared resources like `Config`, audio engine, and hotkey state.
- **Modularity:** Keep hardware-specific logic in `platform/` and top-level modules (`audio.rs`, `typing.rs`, `hotkey.rs`).

### 2. Frontend (Preact)

- **Strict TypeScript:** No `any`. Explicit interfaces for all data structures (API responses, state slices).
- **Hooks over Classes:** Use functional components. Extract reusable logic into custom hooks colocated with the feature (e.g. alongside the page or component that owns it). There is no central `hooks/` directory today.
- **Styles (Current Convention):** Prefer component-local inline style objects with design tokens (`design-tokens.ts`, `theme/ui-primitives.ts`) for layout, spacing, and color. Use global CSS (`index.css`) for resets, root-level variables, and truly global concerns only.
- **Style Consistency:** When touching existing UI, follow the style approach already used in that component/file. Do not introduce a separate styling pattern unless there is a clear architectural reason.
- **Tauri Core:** Use `@tauri-apps/api` for communication with the backend.

### 3. Transcription Engines

| Engine                | Module                | Platform | Model storage       |
| --------------------- | --------------------- | -------- | ------------------- |
| OpenAI-compatible API | `transcription.rs`    | All      | N/A (remote)        |
| whisper.cpp (GGML)    | `local_whisper.rs`    | All      | `models/<size>.bin` |
| OpenVINO              | `openvino_whisper.rs` | Windows  | `models/openvino/…` |
| MLX                   | `mlx_whisper.rs`      | macOS    | `models/mlx/…`      |

Model catalog, download, and path resolution live in `model_manager.rs`. Keep model I/O asynchronous.

### 4. Status and Events

- **Recording lifecycle states** (Recording, Transcribing, Ready, etc.): use `emit_status_update` in `src-tauri/src/app/status.rs` as the single source of truth.
- **Domain-specific updates** (history changes, config updates, download progress, mic-test volume): dedicated Tauri events are appropriate. Do not route these through `emit_status_update`.

---

## 📋 Platform Compatibility & Requirements

| Platform    | Status       | Display Server | Audio Backend     | Hardware Access                                                 |
| ----------- | ------------ | -------------- | ----------------- | --------------------------------------------------------------- |
| **Linux**   | Release      | Wayland, X11   | ALSA / PulseAudio | Wayland: XDG Portals (`ashpd`); X11: native backends            |
| **Windows** | Release      | Desktop        | WASAPI            | Native APIs (SendInput, MMDevice)                               |
| **macOS**   | Experimental | Desktop        | CoreAudio         | Global shortcuts + clipboard-paste typing; not a release target |

### Linux Permission Setup

On Wayland, Voquill triggers standard XDG Portal prompts for microphone, global shortcuts, and remote desktop (input simulation). On X11, equivalent capabilities use native X11 backends and should still surface clear setup/readiness state in the UI.

Use `get_portal_diagnostics` and `get_linux_setup_status` when debugging Wayland permission or shortcut issues.

---

## 🔄 Development Workflow for New Features

When adding a new feature, follow this sequence:

1. **Analyze Environment:** Check for platform-specific constraints (Wayland and X11 where relevant).
2. **Scaffold Backend:** Implement the logic in a new or existing Rust module under `src-tauri/src/`.
3. **Expose Command:** Add a `#[tauri::command]` in `app/commands/` and register it in `main.rs`.
4. **Implement UI:** Create or extend Preact components under `src/` and connect via `invoke`.
5. **Verify Integrity:** Run the [pre-completion verification](#pre-completion-verification) checklist.
6. **Test Platform Parity:** Verify the feature on Windows and Linux (Wayland and X11). Consider macOS only if explicitly in scope.

---

## Architectural Debt

Tracked gaps where the codebase diverges from this document's philosophy are maintained in **[GitHub issue #16](https://github.com/MTG-Thomas/voquill/issues/16)**. Check that issue before large refactors in the affected areas.

When working on transcription, see the engine modules listed above — there is no separate integration plan document.

Ongoing convention (not debt): keep frontend/Tauri script orchestration in npm and Rust build logic in Cargo/Tauri.

---

## 🤖 Interaction Guidelines for Agents

- **Look for Improvement:** Analyze surrounding code for problems, but keep changes scoped to the task unless the user approves broader cleanup.
- **Correct Inaccuracies Proactively:** If a user statement is technically incorrect or based on a false assumption, explicitly correct it and proceed with the correct approach. Do not silently follow an incorrect premise.
- **Ask, Don't Assume:** If a change involves structural moves (folders, module renames), explain why and ask for approval.
- **Trace the Data:** Before proposing a fix for any data-related issue, trace the information back to its origin. Propose a fix for the source logic rather than a filter for the consumer.
- **Status Updates:** Use `emit_status_update` for standard recording/transcription lifecycle states. Use dedicated events for domain-specific UI updates.
- **Platform Parity:** When adding a feature, consider Windows and Linux (Wayland and X11). Isolate platform-specific logic under `src-tauri/src/platform/`.
- **UI Consistency First:** Keep UI behavior, structure, and interaction flow identical across systems whenever possible. Only diverge where an OS/backend capability requires it.
- **Linux Integrity:** Do not break or degrade Linux functionality when fixing other platforms. See [WINDOWS_AUDIO_HANDOVER.md](docs/WINDOWS_AUDIO_HANDOVER.md) for the active Windows parity task.
- **Documentation:** Update `AGENTS.md` or other docs when introducing a new architectural pattern or major dependency.
- **Self-Verification:** Run the [pre-completion verification](#pre-completion-verification) checklist before declaring a task complete.
- **Git Commits:** Do not perform git commits without explicit user approval.

### Solo-Scale Guardrails

- **Prefer Simplicity by Default:** Use the simplest clean solution that meets current requirements and known near-term needs.
- **Delay Splits Until Needed:** Do not create DE-specific files/folders until at least one concrete, recurring incompatibility exists.
- **Keep Files Focused:** A file should answer one question clearly. Split only when readability materially improves.
- **No Silent Failure Paths:** Always surface actionable errors in logs and, when relevant, to UI status.
- **Diagnostics Before Guesswork:** Add clear capability/version/runtime diagnostics before introducing conditional behavior.

---

## ⚠️ Common Pitfalls to Avoid

- **Blocking the UI:** Never run expensive calculations or blocking I/O on the main thread.
- **Hardcoding Paths:** Always use the Tauri `PathResolver` or standard `dirs` crate to locate configuration and data directories.
- **Silent Failures:** Always log errors and, if relevant, notify the user via a Toast or Status update.
- **Inconsistent Naming:** Do not mix `camelCase` and `snake_case` in the same context. Follow established patterns (Rust: `snake_case`, TS: `camelCase`).
- **Over-Engineering:** Prefer simple, readable code over complex "clever" solutions. If a function is hard to explain, it needs to be simplified.
- **Ignoring Warnings:** Treat compiler warnings as errors. Clean code means zero warnings.

---

_Voquill: Clean code is a requirement, not a feature._
