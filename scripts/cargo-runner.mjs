#!/usr/bin/env node

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawn, spawnSync } from "node:child_process";

function firstExistingPath(candidates) {
  return candidates.find((candidate) => fs.existsSync(candidate));
}

function getLatestDirectory(parent) {
  if (!fs.existsSync(parent)) return undefined;
  const directories = fs
    .readdirSync(parent, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => path.join(parent, entry.name))
    .sort()
    .reverse();
  return directories[0];
}

function commandExists(command) {
  const locator = process.platform === "win32" ? "where.exe" : "which";
  return spawnSync(locator, [command], { stdio: "ignore" }).status === 0;
}

function quoteCmd(value) {
  return `"${String(value).replace(/"/g, '""')}"`;
}

function privateCargoTargetDir() {
  const home = os.homedir();
  const cache = path.join(home, ".cache");
  const voquill = path.join(cache, "voquill");
  const target = path.join(voquill, "cargo-target");
  const uid = process.getuid?.();

  for (const directory of [home, cache, voquill, target]) {
    if (!fs.existsSync(directory)) fs.mkdirSync(directory, { mode: 0o700 });
    const metadata = fs.lstatSync(directory);
    if (
      !metadata.isDirectory() ||
      (uid !== undefined && metadata.uid !== uid) ||
      (metadata.mode & 0o022) !== 0 ||
      ((directory === voquill || directory === target) && (metadata.mode & 0o077) !== 0)
    ) {
      throw new Error(`Unsafe Cargo target directory: ${directory}`);
    }
  }
  return target;
}

function runCargoDirect(args) {
  const cargo = commandExists("cargo")
    ? "cargo"
    : path.join(process.env.USERPROFILE ?? process.env.HOME ?? "", ".cargo", "bin", "cargo.exe");
  const env = { ...process.env };
  // Reuse build artifacts across builds and worktrees when sccache is present.
  // RUSTC_WRAPPER covers rustc; the CMake launcher variables cover native
  // C/C++ builds such as whisper.cpp, which whisper-rs-sys compiles through
  // the cmake crate (CMake reads these from the environment). Explicit caller
  // settings win in every case.
  const useSccache = commandExists("sccache");
  if (!env.RUSTC_WRAPPER && useSccache) {
    env.RUSTC_WRAPPER = "sccache";
  }
  if (!env.CMAKE_C_COMPILER_LAUNCHER && useSccache) {
    env.CMAKE_C_COMPILER_LAUNCHER = "sccache";
  }
  if (!env.CMAKE_CXX_COMPILER_LAUNCHER && useSccache) {
    env.CMAKE_CXX_COMPILER_LAUNCHER = "sccache";
  }
  // Share artifacts between this user's worktrees without using a predictable
  // path in a shared temporary directory. Explicit caller settings still win.
  env.CARGO_TARGET_DIR ||= privateCargoTargetDir();
  const child = spawn(cargo, args, { stdio: "inherit", shell: false, env });
  child.on("exit", (code, signal) => {
    if (signal) {
      process.kill(process.pid, signal);
    }
    process.exit(code ?? 1);
  });
}

function runCargoOnWindows(args) {
  const programFilesX86 = process.env["ProgramFiles(x86)"] ?? "C:\\Program Files (x86)";
  const programFiles = process.env.ProgramFiles ?? "C:\\Program Files";
  const vsDevCmd = firstExistingPath([
    path.join(
      programFilesX86,
      "Microsoft Visual Studio",
      "2022",
      "BuildTools",
      "Common7",
      "Tools",
      "VsDevCmd.bat",
    ),
    path.join(
      programFiles,
      "Microsoft Visual Studio",
      "2022",
      "Community",
      "Common7",
      "Tools",
      "VsDevCmd.bat",
    ),
  ]);
  const llvmBin = firstExistingPath([
    path.join(programFiles, "LLVM", "bin"),
    path.join(programFilesX86, "LLVM", "bin"),
  ]);
  const cmakeBin = firstExistingPath([
    path.join(programFiles, "CMake", "bin"),
    path.join(programFilesX86, "CMake", "bin"),
  ]);
  const vulkanRoot = process.env.VULKAN_SDK || getLatestDirectory("C:\\VulkanSDK");

  if (!vsDevCmd) {
    console.error(
      "Visual Studio Build Tools were not found. Run `npm run deps:check` for setup guidance.",
    );
    process.exit(1);
  }

  if (!llvmBin || !cmakeBin || !vulkanRoot) {
    console.error(
      "LLVM, CMake, or the Vulkan SDK were not found. Run `npm run deps:check` for setup guidance.",
    );
    process.exit(1);
  }

  const cargoArgs = args.map(quoteCmd).join(" ");
  const pathPrefix = [llvmBin, cmakeBin, path.join(vulkanRoot, "Bin")].join(";");
  const sccacheSetup = commandExists("sccache")
    ? [
        `set ${quoteCmd("RUSTC_WRAPPER=sccache")}`,
        "&&",
        `set ${quoteCmd("CMAKE_C_COMPILER_LAUNCHER=sccache")}`,
        "&&",
        `set ${quoteCmd("CMAKE_CXX_COMPILER_LAUNCHER=sccache")}`,
        "&&",
      ]
    : [];
  const command = [
    "call",
    quoteCmd(vsDevCmd),
    "-arch=x64",
    "-host_arch=x64",
    "&&",
    `set ${quoteCmd(`PATH=${pathPrefix};%PATH%`)}`,
    "&&",
    `set ${quoteCmd(`VULKAN_SDK=${vulkanRoot}`)}`,
    "&&",
    `set ${quoteCmd("CARGO_TARGET_DIR=C:\\voquill-build")}`,
    "&&",
    ...sccacheSetup,
    "cargo",
    cargoArgs,
  ].join(" ");

  const scriptPath = path.join(os.tmpdir(), `voquill-cargo-${process.pid}-${Date.now()}.cmd`);
  fs.writeFileSync(scriptPath, `@echo off\r\n${command}\r\n`, "utf8");

  const child = spawn("cmd.exe", ["/d", "/c", scriptPath], {
    stdio: "inherit",
    shell: false,
  });
  child.on("exit", (code, signal) => {
    try {
      fs.unlinkSync(scriptPath);
    } catch {
      // Best-effort cleanup; the temp file is harmless if Windows still has a handle briefly.
    }

    if (signal) {
      process.kill(process.pid, signal);
    }
    process.exit(code ?? 1);
  });
}

const args = process.argv.slice(2);
if (process.platform === "win32") {
  runCargoOnWindows(args);
} else {
  runCargoDirect(args);
}
