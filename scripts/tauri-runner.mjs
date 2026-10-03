#!/usr/bin/env node

import { execFileSync, spawnSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { logError, run } from "@tauri-apps/cli/main.js";

function commandExists(command) {
  const locator = process.platform === "win32" ? "where.exe" : "which";
  return spawnSync(locator, [command], { stdio: "ignore" }).status === 0;
}

if (process.platform === "win32") {
  process.env.CARGO_TARGET_DIR ||= "C:\\voquill-build";
  // Build whisper.cpp with Ninja instead of the default MSBuild generator.
  // MSBuild's tracked CustomBuild steps can break nested CMake try_compile
  // checks (ggml-vulkan builds vulkan-shaders-gen via ExternalProject), and
  // Ninja builds faster.
  process.env.CMAKE_GENERATOR = "Ninja";
}

process.env.GGML_NATIVE = "OFF";
process.env.GGML_AVX512 = "OFF";
process.env.GGML_AVX512_VBMI = "OFF";
process.env.GGML_AVX512_VNNI = "OFF";
process.env.GGML_AVX512_BF16 = "OFF";
process.env.GGML_AMX_TILE = "OFF";
process.env.GGML_AMX_INT8 = "OFF";
process.env.GGML_AMX_BF16 = "OFF";
process.env.GGML_AVX_VNNI = "OFF";

// whisper-rs-sys compiles whisper.cpp/ggml natively but does not declare
// cargo:rerun-if-env-changed for the GGML_* variables above, so changing them
// never invalidates Cargo's build cache. Without a forced rebuild, stale
// objects (e.g. compiled with /arch:AVX512 from a previous GGML_NATIVE=ON
// build) get silently relinked into the release binary. Clean the package
// before every release build so these flags always reach the compiler.
const args = process.argv.slice(2);

// Reuse build artifacts across builds and worktrees when sccache is present.
// RUSTC_WRAPPER covers rustc; the CMake launcher variables cover native
// C/C++ builds such as whisper.cpp (built through the cmake crate). This also
// keeps the whisper-rs-sys clean-then-rebuild below cheap: the rebuild hits
// the cache instead of recompiling every object.
const useSccache = commandExists("sccache");
if (!process.env.RUSTC_WRAPPER && useSccache) {
  process.env.RUSTC_WRAPPER = "sccache";
}
if (!process.env.CMAKE_C_COMPILER_LAUNCHER && useSccache) {
  process.env.CMAKE_C_COMPILER_LAUNCHER = "sccache";
}
if (!process.env.CMAKE_CXX_COMPILER_LAUNCHER && useSccache) {
  process.env.CMAKE_CXX_COMPILER_LAUNCHER = "sccache";
}

// Production bundles keep whisper.cpp Vulkan acceleration ("Turbo Mode") on
// the platforms that ship it. Development builds and plain cargo checks stay
// CPU-only so they skip the expensive shader generation. Explicit caller
// features win.
const isBuild = args[0] === "build";
const supportsVulkan = process.platform === "linux" || process.platform === "win32";
const hasExplicitFeatures =
  args.includes("--features") || args.includes("-f") || args.includes("--no-default-features");
if (isBuild && supportsVulkan && !hasExplicitFeatures) {
  // Insert before `--` so passthrough arguments keep their position.
  const passthroughIndex = args.indexOf("--");
  args.splice(passthroughIndex === -1 ? args.length : passthroughIndex, 0, "--features", "vulkan");
}

// NOTE: cargo clean -p without --release only cleans dev-profile artifacts,
// so --release is required here to match the `tauri build` profile.
if (isBuild) {
  execFileSync("cargo", ["clean", "-p", "whisper-rs-sys", "--release"], {
    cwd: new URL("../src-tauri", import.meta.url),
    env: process.env,
    stdio: "inherit",
  });
}

// Windows Authenticode signing activates only when a certificate thumbprint is
// provided via the environment (see docs/WINDOWS_CODESIGN.md). The thumbprint
// is merged into the Tauri config through a temporary --config overlay so the
// committed tauri.conf.json never carries signing identity material.
if (process.platform === "win32" && isBuild) {
  const thumbprint = (process.env.WINDOWS_CODESIGN_THUMBPRINT ?? "")
    .replace(/[\s:-]/g, "")
    .toUpperCase();
  if (thumbprint) {
    const overlayPath = join(tmpdir(), `voquill-signing-${process.pid}.json`);
    writeFileSync(
      overlayPath,
      JSON.stringify({ bundle: { windows: { certificateThumbprint: thumbprint } } }),
    );
    // Insert before `--` so passthrough arguments keep their position.
    const configIndex = args.indexOf("--");
    args.splice(configIndex === -1 ? args.length : configIndex, 0, "--config", overlayPath);
    console.log(`Windows code signing enabled (thumbprint ${thumbprint}).`);
  } else {
    console.log(
      "Windows code signing disabled: WINDOWS_CODESIGN_THUMBPRINT is not set; artifacts will be unsigned.",
    );
  }
}

try {
  await run(args, "tauri");
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  if (typeof logError === "function") logError(message);
  console.error(error);
  process.exit(1);
}
