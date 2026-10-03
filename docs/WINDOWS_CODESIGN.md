# Windows Code Signing

Windows NSIS (`.exe`) and MSI (`.msi`) release artifacts support Authenticode
signing. Signing activates automatically whenever a certificate is available;
without one, builds keep working and ship unsigned.

## Current Status

**Blocker: no code-signing certificate has been acquired yet.** All repo-side
automation is in place (Tauri config, release workflow, signature
verification); the only remaining step is acquiring a certificate for Midtown
Technology Group LLC and registering it with GitHub Actions. See
[Finishing Checklist](#finishing-checklist) below.

## How Activation Works

1. `src-tauri/tauri.conf.json` keeps `bundle.windows.certificateThumbprint`
   as `null` so no signing identity is committed, and sets a default
   `timestampUrl` plus `sha256` digest algorithm so signatures are
   timestamped as soon as signing turns on.
2. `scripts/tauri-runner.mjs` reads `WINDOWS_CODESIGN_THUMBPRINT` on Windows
   builds. When set, it merges the thumbprint into the Tauri config via a
   temporary `--config` overlay and the Tauri bundler signs with the matching
   certificate from the Windows certificate store. When unset, the build
   proceeds unsigned with a log line saying so.
3. `.github/workflows/release.yml` provisions the certificate before the
   Windows build (see [Secrets](#secrets)), then runs
   `scripts/verify-windows-signatures.ps1`, which fails the job when signing
   was expected but any artifact is missing, unsigned, or invalid.

## Secrets

| Secret                          | Required when          | Purpose                                                        |
| ------------------------------- | ---------------------- | -------------------------------------------------------------- |
| `WINDOWS_CODESIGN_PFX_BASE64`   | PFX import path        | Base64-encoded `.pfx`/`.p12` code-signing certificate.         |
| `WINDOWS_CODESIGN_PFX_PASSWORD` | PFX import path        | Password protecting the PFX above.                             |
| `WINDOWS_CODESIGN_THUMBPRINT`   | Store-cert path        | SHA-1 thumbprint of a certificate already in the build machine store (KeyLocker agent, EV token, pre-installed cert). |

Only one path is needed. When the PFX secrets are present, CI imports the
certificate into `CurrentUser\My` and derives the thumbprint. Otherwise, when
only the thumbprint secret is present, CI signs with the store certificate
matching that thumbprint. When none are present, artifacts ship unsigned and
CI logs a warning instead of failing.

Never commit certificate files, PFX passwords, or thumbprints tied to a real
identity to the repo. Secrets live in the GitHub repository or environment
settings only.

## Certificate Options

Prefer a managed option that avoids exportable private key material:

1. **Azure Trusted Signing** (recommended): no certificate handling in CI;
   sign with a Microsoft-hosted identity. Wiring it in later means adding a
   post-build signing step for the NSIS `.exe` and MSI before the existing
   verify step, plus the `AZURE_*`/Trusted Signing secrets it needs.
2. **OV/EV certificate via KeyLocker/eSigner agent or EV token**: install or
   attach the provider agent so the certificate appears in the Windows store,
   then set only `WINDOWS_CODESIGN_THUMBPRINT`.
3. **Exportable PFX in GitHub secrets** (fallback): set
   `WINDOWS_CODESIGN_PFX_BASE64` and `WINDOWS_CODESIGN_PFX_PASSWORD`. Least
   preferred because the private key is copyable, but fully supported.

Any option must issue for a publisher identity matching the release (Midtown
Technology Group LLC) and support Authenticode code signing with SHA-256.

## Finishing Checklist

1. Acquire the certificate (see options above).
2. Register secrets: PFX pair or thumbprint, as `Actions` secrets on the
   `MTG-Thomas/voquill` repo.
3. Trigger a throwaway `mtg-v*` workflow-dispatch build or push a release
   tag; confirm the `Prepare Windows code signing` step reports signing and
   `Verify Windows signatures` passes.
4. Confirm `Get-AuthenticodeSignature` shows `Valid` on the published `.exe`
   and `.msi`, with `SignerCertificate.Subject` naming Midtown Technology
   Group LLC.
5. Update the release notes publisher line and close issue #12.

## Local Signing (Windows)

```powershell
$env:WINDOWS_CODESIGN_THUMBPRINT = "<thumbprint of your CurrentUser\My code-signing cert>"
npm run tauri -- build --bundles nsis,msi
powershell -File scripts/verify-windows-signatures.ps1 -BundleDir C:\voquill-build\release\bundle -ExpectSigned
```

The default bundle directory above matches the npm runners; CI uses
`C:\t\release\bundle` via `CARGO_TARGET_DIR`.

## Verification

```powershell
Get-AuthenticodeSignature .\Voquill_*_x64-setup.exe
Get-AuthenticodeSignature .\Voquill_*_x64_en-US.msi
```

Both must report `Status: Valid` once signing is active. The same check runs
in CI via `scripts/verify-windows-signatures.ps1`.

## WinGet Note

Signing changes installer bytes, so the private WinGet manifest must use the
SHA-256 of the **signed** MSI. The release workflow generates
`SHA256SUMS.txt` from the final (signed, when configured) artifacts in the
publish job, after the Windows build and verification steps — always take the
WinGet hash from there, never from a local unsigned build.
