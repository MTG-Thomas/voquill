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

function runCargoDirect(args) {
  const cargo = commandExists("cargo")
    ? "cargo"
    : path.join(process.env.USERPROFILE ?? process.env.HOME ?? "", ".cargo", "bin", "cargo.exe");
  const child = spawn(cargo, args, { stdio: "inherit", shell: false });
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
