<#
.SYNOPSIS
  Verifies Authenticode signatures on Windows release artifacts.

.DESCRIPTION
  Checks every NSIS setup executable and MSI produced by the Tauri Windows
  build. When -ExpectSigned is set (CI sets it whenever signing secrets are
  configured), any missing or invalid signature fails the run. Otherwise the
  script reports signature status and warns about unsigned artifacts without
  failing, so unsigned local builds keep working.

.PARAMETER BundleDir
  Directory containing the Tauri bundle output (the folder that holds the
  `nsis` and `msi` subdirectories).

.PARAMETER ExpectSigned
  Fail when an expected artifact is missing, unsigned, or has an invalid
  signature.

.EXAMPLE
  powershell -File scripts/verify-windows-signatures.ps1 -BundleDir C:\t\release\bundle
#>
[CmdletBinding()]
param(
  [Parameter(Mandatory = $true)]
  [string]$BundleDir,

  [switch]$ExpectSigned
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$targets = @(
  (Join-Path $BundleDir "nsis" "*.exe"),
  (Join-Path $BundleDir "msi" "*.msi")
)

$artifacts = @()
foreach ($pattern in $targets) {
  $artifacts += Get-ChildItem -Path $pattern -File -ErrorAction SilentlyContinue
}

if ($artifacts.Count -eq 0) {
  $message = "No Windows release artifacts found under '$BundleDir' (expected nsis/*.exe and msi/*.msi)."
  if ($ExpectSigned) {
    throw $message
  }
  Write-Warning $message
  exit 0
}

$failures = @()
foreach ($artifact in $artifacts) {
  $signature = Get-AuthenticodeSignature -FilePath $artifact.FullName
  $status = $signature.Status
  $subject = "(no signature)"
  if ($null -ne $signature.SignerCertificate) {
    $subject = $signature.SignerCertificate.Subject
  }
  Write-Host "$($artifact.Name): $status $subject"
  if ($status -ne "Valid") {
    $failures += "$($artifact.Name): $status"
  }
}

if ($failures.Count -gt 0) {
  $detail = $failures -join "; "
  if ($ExpectSigned) {
    throw "Windows signature verification failed: $detail"
  }
  Write-Warning "Unsigned or invalid Windows artifacts (signing not configured): $detail"
  exit 0
}

Write-Host "All $($artifacts.Count) Windows artifact(s) carry valid Authenticode signatures."
