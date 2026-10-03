#!/usr/bin/env node

import { execFileSync, spawnSync } from "node:child_process";
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

// Reuse rustc artifacts across builds and worktrees when sccache is present.
if (!process.env.RUSTC_WRAPPER && commandExists("sccache")) {
  process.env.RUSTC_WRAPPER = "sccache";
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

try {
  await run(args, "tauri");
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  if (typeof logError === "function") logError(message);
  console.error(error);
  process.exit(1);
}
