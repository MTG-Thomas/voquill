#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { logError, run } from "@tauri-apps/cli/main.js";

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
// NOTE: cargo clean -p without --release only cleans dev-profile artifacts,
// so --release is required here to match the `tauri build` profile.
if (process.argv[2] === "build") {
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
const cliArgs = process.argv.slice(2);
if (process.platform === "win32" && cliArgs[0] === "build") {
  const thumbprint = (process.env.WINDOWS_CODESIGN_THUMBPRINT ?? "")
    .replace(/[\s:-]/g, "")
    .toUpperCase();
  if (thumbprint) {
    const overlayPath = join(tmpdir(), `voquill-signing-${process.pid}.json`);
    writeFileSync(
      overlayPath,
      JSON.stringify({ bundle: { windows: { certificateThumbprint: thumbprint } } }),
    );
    cliArgs.push("--config", overlayPath);
    console.log(`Windows code signing enabled (thumbprint ${thumbprint}).`);
  } else {
    console.log(
      "Windows code signing disabled: WINDOWS_CODESIGN_THUMBPRINT is not set; artifacts will be unsigned.",
    );
  }
}

try {
  await run(cliArgs, "tauri");
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  if (typeof logError === "function") logError(message);
  console.error(error);
  process.exit(1);
}
