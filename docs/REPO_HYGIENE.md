# Repository Hygiene

Voquill's CI treats formatting, linting, dependency policy, security scanning, Tauri hardening, and release artifact integrity as first-class checks.

## Local Checks

Run these before opening or updating a pull request:

```powershell
npm run format:check
npm run lint
npm run typecheck
npm run harden:check
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

On Windows, the Rust checks may need the same native build dependencies as the Tauri build. On this workstation that includes `C:\Program Files\CMake\bin`, `C:\Program Files\LLVM\bin`, and the Vulkan SDK on `PATH`.

## Security Gates

The security workflow runs:

- `cargo audit` against `src-tauri/Cargo.lock`.
- `cargo deny` for Rust license, duplicate dependency, and source policy.
- GitHub CodeQL for Rust and TypeScript.
- Semgrep with default, secrets, JavaScript, TypeScript, and Rust rules.

`cargo audit` currently reports allowed warnings from the inherited Tauri/Linux GTK dependency graph, including GTK3 bindings, `glib`, and `urlpattern` transitive dependencies. Those are tracked as an upgrade pressure signal rather than an immediate blocker because they are not direct Voquill dependencies and do not have clean safe upgrades in the current Tauri stack.

## Tauri Hardening

`npm run harden:check` verifies the high-risk Tauri settings that should stay explicit:

- A CSP must be configured.
- The main window capability must not use `shell:default`.
- Shell execute, spawn, and kill permissions are forbidden.
- The overlay capability must not receive shell or app-open permissions.

The check warns, but does not fail, when Windows release artifacts are unsigned. Treat that as the next release-hardening step once a signing certificate is available.

## Release Provenance

Release builds upload platform artifacts plus a generated `SHA256SUMS.txt` file. Keep tagged MTG releases under the `mtg-v*` namespace so they remain distinct from upstream releases.
