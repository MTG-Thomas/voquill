# Voquill Build Guide

Voquill uses npm scripts with the Tauri CLI to build across platforms.

## Quick Start

The main build command will check for dependencies, build the frontend, and then build the Tauri application.

```bash
# Start desktop development
npm run tauri:dev

# Standard Tauri bundle build
npm run tauri:build
```

### When to use each command:
- **npm run tauri:build**: Use this for the final app you intend to share or use daily. It produces optimized, small executables and bundled installers.
- **npm run tauri:dev**: Use this for active development. It provides hot-reloading for both the frontend and backend.

## Requirements

- **Rust** with Cargo
- **Node.js** and **npm** (for frontend dependencies)

### Platform-Specific Requirements

#### Linux (Ubuntu/Debian)
The build script checks for these packages:
- `libpulse-dev`
- `libgtk-layer-shell-dev`
- `cmake`
- `pkg-config`
- `libclang-dev`
- `build-essential`

Additional Tauri requirements:
- `libwebkit2gtk-4.1-dev`
- `libgtk-3-dev`
- `libayatana-appindicator3-dev`
- `librsvg2-dev`

## Known Runtime Warnings

### libayatana-appindicator Deprecation (Linux)

When running Voquill on Linux, you may see a warning in the terminal:
`libayatana-appindicator-WARNING: libayatana-appindicator is deprecated. Please use libayatana-appindicator-glib in newly written code.`

**Status:** This is a cosmetic warning that affects all Tauri v2 applications using tray icons on Linux. It does not affect functionality.

**Cause:** Tauri's tray implementation currently depends on the older `libayatana-appindicator3` library. The upstream project has released a newer `-glib` variant, but the Rust ecosystem bindings haven't migrated yet. No action is required from users or developers.

## What the Build Process Does

1. **Checks Dependencies** - Runs `npm run deps:check` and verifies required system libraries and toolchains.
2. **Builds Frontend** - Runs `npm run build` (type-check + Vite build).
3. **Builds Application** - Runs `tauri build` to create the final executable and installers.

## GPU (Turbo Mode) and Build Performance

Whisper.cpp's Vulkan backend ("Turbo Mode") is enabled by the opt-in `vulkan` Cargo
feature. It is **not** compiled by default because building it generates thousands
of compute shaders on every clean build, which dominates compile time for local
checks, tests, and fresh worktrees.

- **Release bundles** enable it explicitly (`--features vulkan`) on Linux and
  Windows. macOS uses Metal and is unaffected.
- **Local checks** (`npm run cargo:check`, direct `cargo check`/`clippy`/`test`)
  are CPU-only and skip shader generation. The app still runs; it simply reports
  no GPU support, so use a full `npm run tauri:build` when you need Turbo Mode.
- **Local acceleration**: the npm Cargo/Tauri runners reuse `sccache` when it is
  installed and share a single `CARGO_TARGET_DIR` (`$TMPDIR/voquill-target`) for
  local builds, so repeat builds and parallel worktrees reuse warm artifacts
  instead of cold-building a private `src-tauri/target` each time.

To force Turbo Mode for a local build:

```bash
npm run tauri -- build --features vulkan
```

## Output

After a successful build, you can find the artifacts in:
- **Linux**: `src-tauri/target/release/bundle/` (contains `.deb`, `.rpm`, `.AppImage`)
- **Windows** (via `npm run tauri:build` or `npm run tauri:*`): `C:\voquill-build\release\bundle/` (contains `.msi`, `.exe`) — the npm cargo/tauri runners set `CARGO_TARGET_DIR` to avoid long-path failures.

## Troubleshooting

- **Missing dependencies on Linux**: If the build fails with missing library errors, follow the instructions provided by the script to install the necessary `apt` packages.
- **Frontend build issues**: If the UI fails to build, try clearing `node_modules` and running the build again.
- **Rust compilation errors**: Ensure your Rust toolchain is up to date with `rustup update`.
- **Windows whisper/Vulkan build failures**: Run `npm run deps:check`, then use `npm run cargo:check` or `npm run tauri:dev` instead of `cargo` directly from `src-tauri/`. The cargo runner loads Visual Studio, CMake, LLVM, the Vulkan SDK, and sets `CARGO_TARGET_DIR=C:\voquill-build` to avoid long-path failures.
- **No GPU / Turbo Mode engine missing locally**: Vulkan is opt-in. Build with `npm run tauri -- build --features vulkan` (Linux/Windows) to enable Whisper.cpp GPU acceleration.
- **Fedora AppImage bundling**: Some Fedora toolchains ship RELR-enabled libraries that fail when stripped by the linuxdeploy binary bundled with Tauri. On Fedora, use `npm run tauri -- build --bundles deb,rpm` for distro packages, and build AppImage on Ubuntu/Mint/Kubuntu.
