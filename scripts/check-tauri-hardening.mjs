import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join } from "node:path";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));
const readJson = (path) => JSON.parse(readFileSync(join(repoRoot, path), "utf8"));

const failures = [];
const warnings = [];

const tauriConfig = readJson("src-tauri/tauri.conf.json");
const mainCapability = readJson("src-tauri/capabilities/main.json");
const overlayCapability = readJson("src-tauri/capabilities/overlay.json");

if (!tauriConfig.app?.security?.csp) {
  failures.push("src-tauri/tauri.conf.json must set app.security.csp.");
}

const mainPermissions = new Set(mainCapability.permissions ?? []);
const overlayPermissions = new Set(overlayCapability.permissions ?? []);

if (mainPermissions.has("shell:default")) {
  failures.push("main capability must not use shell:default; grant shell:allow-open explicitly.");
}

for (const deniedPermission of ["shell:allow-execute", "shell:allow-spawn", "shell:allow-kill"]) {
  if (mainPermissions.has(deniedPermission) || overlayPermissions.has(deniedPermission)) {
    failures.push(`Tauri capability grants dangerous shell permission: ${deniedPermission}`);
  }
}

if (overlayPermissions.has("shell:allow-open") || overlayPermissions.has("allow-app-commands")) {
  failures.push("overlay capability should not receive shell or custom app command permissions.");
}

if (tauriConfig.bundle?.windows?.certificateThumbprint === null) {
  warnings.push(
    "Windows release artifacts are unsigned; set certificateThumbprint when signing is ready.",
  );
}

for (const warning of warnings) {
  console.warn(`warning: ${warning}`);
}

if (failures.length > 0) {
  for (const failure of failures) {
    console.error(`error: ${failure}`);
  }
  process.exit(1);
}

console.log("Tauri hardening checks passed.");
