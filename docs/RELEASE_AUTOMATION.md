# Release Automation

Voquill uses GitHub Actions for CI and fork-local release packaging.

## Tag Namespace

Use `mtg-v*` tags for this fork, for example:

```powershell
git tag mtg-v1.3.11
git push origin mtg-v1.3.11
```

Do not use upstream-style `v*` tags for fork releases. The release workflow refuses tags outside the `mtg-v*` namespace so our artifacts are not confused with upstream Voquill releases.

## CI

`.github/workflows/ci.yml` runs on pull requests and pushes to `main`, `dev`, `codex/**`, and `mtg/**` branches.

It verifies:

- `npm ci`
- `npm run typecheck`
- `npm run build`
- `cargo check --manifest-path src-tauri/Cargo.toml`
- `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`

The workflow runs on Windows and Ubuntu. It installs Tauri, WebKit, GTK, PulseAudio, Vulkan, CMake, LLVM, and related native dependencies needed by this repo.

## Release

`.github/workflows/release.yml` runs when a `mtg-v*` tag is pushed. It can also be run manually with a `release_tag` input that still must start with `mtg-v`.

The release workflow:

1. Builds Windows and Linux Tauri bundles.
2. Uploads platform artifacts to the workflow run.
3. Creates a draft GitHub Release named `Voquill MTG <tag>`.
4. Attaches generated installers/packages.

Expected artifacts:

- Windows: NSIS `.exe` and/or MSI `.msi`
- Linux: `.deb`, `.rpm`, and AppImage when produced by Tauri

## Signing

Windows NSIS and MSI artifacts are Authenticode-signed automatically when
signing secrets are configured, and ship unsigned otherwise. Full details,
including the remaining certificate-acquisition blocker and the finishing
checklist, live in [WINDOWS_CODESIGN.md](WINDOWS_CODESIGN.md).

Secrets (register only one path):

- PFX import: `WINDOWS_CODESIGN_PFX_BASE64` + `WINDOWS_CODESIGN_PFX_PASSWORD`
- Store certificate: `WINDOWS_CODESIGN_THUMBPRINT`

The release workflow imports or selects the certificate before the Windows
build, the Tauri bundler signs during packaging, and
`scripts/verify-windows-signatures.ps1` fails the job when signing was
expected but any artifact is missing, unsigned, or invalid. `SHA256SUMS.txt`
is generated from the final (signed, when configured) artifacts, so WinGet
manifests must take the MSI hash from there.

## Operator Checklist

Before creating a release tag:

1. Update versions in `package.json` and `src-tauri/tauri.conf.json`.
2. Run local verification:

```powershell
npm run typecheck
npm run build
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

3. Push the release branch or merge it.
4. Create and push a fork-local tag:

```powershell
git tag mtg-v1.3.11
git push origin mtg-v1.3.11
```

5. Review the draft GitHub Release.
6. Publish manually once artifacts look right.
