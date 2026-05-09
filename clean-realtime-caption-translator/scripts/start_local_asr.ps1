$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$envFile = Join-Path $root ".env"
if (Test-Path -LiteralPath $envFile) {
  Get-Content -LiteralPath $envFile | ForEach-Object {
    $line = $_.Trim()
    if (-not $line -or $line.StartsWith("#") -or -not $line.Contains("=")) { return }
    $parts = $line.Split("=", 2)
    $name = $parts[0].Trim()
    $value = $parts[1].Trim()
    if ($name -and -not [Environment]::GetEnvironmentVariable($name, "Process")) {
      [Environment]::SetEnvironmentVariable($name, $value, "Process")
    }
  }
}

$settingsFile = Join-Path $root "apps\desktop-shell\app-settings.json"
$appSettings = $null
if (Test-Path -LiteralPath $settingsFile) {
  try {
    $appSettings = Get-Content -LiteralPath $settingsFile -Raw -Encoding UTF8 | ConvertFrom-Json
  } catch {
    Write-Host "Failed to read app settings, falling back to defaults: $($_.Exception.Message)"
  }
}

$env:LOCAL_ASR_PROVIDER = if ($env:LOCAL_ASR_PROVIDER) { $env:LOCAL_ASR_PROVIDER } elseif ($appSettings -and $appSettings.localAsr.provider) { $appSettings.localAsr.provider } else { "vosk" }
$defaultEnModelDir = Join-Path $root "local-asr\models\vosk-model-small-en-us-0.15"
$defaultZhModelDir = Join-Path $root "local-asr\models\sherpa-onnx-streaming-paraformer-bilingual-zh-en"
$env:LOCAL_ASR_MODEL_DIR_EN = if ($env:LOCAL_ASR_MODEL_DIR_EN) { $env:LOCAL_ASR_MODEL_DIR_EN } else { $defaultEnModelDir }
$env:LOCAL_ASR_MODEL_DIR_ZH = if ($env:LOCAL_ASR_MODEL_DIR_ZH) { $env:LOCAL_ASR_MODEL_DIR_ZH } else { $defaultZhModelDir }
$env:LOCAL_ASR_MODEL_DIR = if ($env:LOCAL_ASR_MODEL_DIR) { $env:LOCAL_ASR_MODEL_DIR } else { $env:LOCAL_ASR_MODEL_DIR_EN }
$env:LOCAL_ASR_SAMPLE_RATE = if ($env:LOCAL_ASR_SAMPLE_RATE) { $env:LOCAL_ASR_SAMPLE_RATE } elseif ($appSettings -and $appSettings.localAsr.sampleRate) { [string]$appSettings.localAsr.sampleRate } else { "16000" }
$env:LOCAL_ASR_PORT = if ($env:LOCAL_ASR_PORT) { $env:LOCAL_ASR_PORT } else { "8765" }

$existingListeners = Get-NetTCPConnection -LocalPort ([int]$env:LOCAL_ASR_PORT) -State Listen -ErrorAction SilentlyContinue
foreach ($listener in $existingListeners) {
  if ($listener.OwningProcess -and $listener.OwningProcess -ne $PID) {
    Write-Host "Stopping stale ASR listener on port $($env:LOCAL_ASR_PORT), pid=$($listener.OwningProcess)"
    Stop-Process -Id $listener.OwningProcess -Force -ErrorAction SilentlyContinue
  }
}

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
  --en-model-dir $env:LOCAL_ASR_MODEL_DIR_EN `
  --zh-model-dir $env:LOCAL_ASR_MODEL_DIR_ZH `
  --sample-rate $env:LOCAL_ASR_SAMPLE_RATE `
  --port $env:LOCAL_ASR_PORT
