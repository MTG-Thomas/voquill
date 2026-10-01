# Repository guidance

Voquill combines a Preact/TypeScript frontend (`src/`), Tauri 2 Rust backend (`src-tauri/`), and pure-execution Python sidecars (`python-runner/`). Load only the applicable routes below before changing their surface. `AGENT-GUIDE.md` preserves detailed conventions; `CONTRIBUTING.md` owns upstream contribution conventions.

## Critical boundaries

Preserve Windows and Linux parity, including both Wayland and X11; macOS is experimental and needs explicit scope. Use capability-driven platform/provider boundaries: Wayland XDG Portals, native X11 and Windows APIs. Fix data at its source rather than masking errors in the UI. Keep recording lifecycle status in `emit_status_update`; use dedicated events for domain updates.

Do not block the UI or hide failures. Use explicit state transitions for nontrivial lifecycle flows, async model/audio/network I/O, and standard data-path resolution. Follow the existing component's styling and design tokens. Keep changes scoped; explain structural moves or module renames and obtain user approval before doing them. Do not make Git commits without explicit user approval. Compiler warnings are errors. Use descriptive names in touched code without unrelated renames; expose actionable errors and keep simple, focused modules.

## Verification

Run the pre-PR checklist in `docs/REPO_HYGIENE.md`:

```sh
npm run format:check
npm run lint
npm run typecheck
npm run harden:check
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Use npm's Cargo/Tauri wrappers for Windows toolchain and short target-directory setup as documented in `docs/BUILD.md`. Verify relevant platform flows and state any untested platform explicitly. Keep CSP and capability restrictions; do not add shell execute/spawn/kill privileges or shell/app-open access to the overlay. Release procedures and integrity/signing evidence live in `docs/RELEASE.md`, `docs/RELEASE_AUTOMATION.md`, and `docs/REPO_HYGIENE.md`; MTG release tags use `mtg-v*`.

## Read by task

- Backend, lifecycle, transcription, or model changes: read [architecture and engines](AGENT-GUIDE.md#architecture-patterns), including command `Result<T, String>`, AppState ownership, async I/O, and status/event rules.
- UI changes: read the Frontend subsection of [architecture and patterns](AGENT-GUIDE.md#architecture-patterns); strict TypeScript has no `any`, use hooks, existing styles, and design tokens.
- Platform or permissions changes: read [adaptation principles](AGENT-GUIDE.md#platform-principles) and [compatibility](AGENT-GUIDE.md#platform-compatibility), plus the affected portal/audio handover document.
- Build/toolchain changes: read [commands](AGENT-GUIDE.md#essential-commands), `docs/BUILD.md`, and `docs/REPO_HYGIENE.md`.
- Large refactors: inspect the linked architectural-debt issue and relevant architecture sections before choosing a seam. Releases: read `docs/RELEASE.md` and `docs/RELEASE_AUTOMATION.md`.
