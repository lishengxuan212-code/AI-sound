$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$env:LOCAL_ASR_PROVIDER = if ($env:LOCAL_ASR_PROVIDER) { $env:LOCAL_ASR_PROVIDER } else { "sherpa_onnx" }
$env:LOCAL_ASR_MODEL_DIR = if ($env:LOCAL_ASR_MODEL_DIR) { $env:LOCAL_ASR_MODEL_DIR } else { "local-asr/models/sherpa-onnx-streaming-paraformer-bilingual-zh-en" }
$env:LOCAL_ASR_SAMPLE_RATE = if ($env:LOCAL_ASR_SAMPLE_RATE) { $env:LOCAL_ASR_SAMPLE_RATE } else { "16000" }
$env:LOCAL_ASR_PORT = if ($env:LOCAL_ASR_PORT) { $env:LOCAL_ASR_PORT } else { "8765" }

Set-Location $root
$projectPython = Join-Path $root ".venv-local-asr\Scripts\python.exe"
$legacyPython = Get-ChildItem -LiteralPath "E:\" -Directory -Filter "AI*" -ErrorAction SilentlyContinue |
  ForEach-Object { Join-Path $_.FullName ".venv-local-asr\Scripts\python.exe" } |
  Where-Object { Test-Path -LiteralPath $_ } |
  Select-Object -First 1
$pythonExe = if (Test-Path -LiteralPath $projectPython) {
  $projectPython
} elseif ($legacyPython) {
  $legacyPython
} else {
  "python"
}

Write-Host "Using Python: $pythonExe"
& $pythonExe .\scripts\local_streaming_asr_server.py `
  --provider $env:LOCAL_ASR_PROVIDER `
  --model-dir $env:LOCAL_ASR_MODEL_DIR `
  --sample-rate $env:LOCAL_ASR_SAMPLE_RATE `
  --port $env:LOCAL_ASR_PORT
