param(
    [string]$RuntimeRoot = "$env:LOCALAPPDATA\Voquill\openvino-runtime",
    [switch]$UseNightly
)

$ErrorActionPreference = 'Stop'

$venvPath = Join-Path $RuntimeRoot '.venv'
$pythonPath = Join-Path $venvPath 'Scripts\python.exe'

New-Item -ItemType Directory -Force -Path $RuntimeRoot | Out-Null

if (-not (Test-Path $pythonPath)) {
    py -3 -m venv $venvPath
}

& $pythonPath -m pip install --upgrade pip

if ($UseNightly) {
    & $pythonPath -m pip install --pre --upgrade `
        --extra-index-url https://storage.openvinotoolkit.org/simple/wheels/nightly `
        openvino openvino-genai openvino-tokenizers numpy
} else {
    & $pythonPath -m pip install --upgrade openvino openvino-genai openvino-tokenizers numpy
}

@'
import openvino as ov
import openvino_genai

core = ov.Core()
print("OpenVINO runtime is ready")
print("Devices:", ", ".join(core.available_devices))
'@ | & $pythonPath -

Write-Host "VOQUILL_OPENVINO_PYTHON=$pythonPath"
