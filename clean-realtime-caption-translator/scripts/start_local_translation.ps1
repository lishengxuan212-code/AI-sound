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

$env:LOCAL_TRANSLATION_PROVIDER = if ($env:LOCAL_TRANSLATION_PROVIDER) { $env:LOCAL_TRANSLATION_PROVIDER } else { "transformers_seq2seq" }
$env:LOCAL_TRANSLATION_ENDPOINT = if ($env:LOCAL_TRANSLATION_ENDPOINT) { $env:LOCAL_TRANSLATION_ENDPOINT } else { "http://127.0.0.1:8777" }
$env:LOCAL_TRANSLATION_PORT = if ($env:LOCAL_TRANSLATION_PORT) { $env:LOCAL_TRANSLATION_PORT } else { "8777" }
$env:LOCAL_TRANSLATION_SOURCE_LANG = if ($env:LOCAL_TRANSLATION_SOURCE_LANG) { $env:LOCAL_TRANSLATION_SOURCE_LANG } else { "en" }
$env:LOCAL_TRANSLATION_TARGET_LANG = if ($env:LOCAL_TRANSLATION_TARGET_LANG) { $env:LOCAL_TRANSLATION_TARGET_LANG } else { "zh" }

$projectPython = Join-Path $root ".venv-local-translation\Scripts\python.exe"
$pythonExe = if (Test-Path -LiteralPath $projectPython) { $projectPython } else { "python" }

Write-Host "Using Python: $pythonExe"
Write-Host "Local translation endpoint: $($env:LOCAL_TRANSLATION_ENDPOINT)/translate"
& $pythonExe .\scripts\local_translation_server.py `
  --provider $env:LOCAL_TRANSLATION_PROVIDER `
  --model-path $env:LOCAL_TRANSLATION_MODEL_PATH `
  --model-name $env:LOCAL_TRANSLATION_MODEL_NAME `
  --source-lang $env:LOCAL_TRANSLATION_SOURCE_LANG `
  --target-lang $env:LOCAL_TRANSLATION_TARGET_LANG `
  --port $env:LOCAL_TRANSLATION_PORT
