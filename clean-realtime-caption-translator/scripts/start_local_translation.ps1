$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
$env:LOCAL_TRANSLATION_PROVIDER = if ($env:LOCAL_TRANSLATION_PROVIDER) { $env:LOCAL_TRANSLATION_PROVIDER } else { "transformers_seq2seq" }
$env:LOCAL_TRANSLATION_ENDPOINT = if ($env:LOCAL_TRANSLATION_ENDPOINT) { $env:LOCAL_TRANSLATION_ENDPOINT } else { "http://127.0.0.1:8770" }
$env:LOCAL_TRANSLATION_PORT = if ($env:LOCAL_TRANSLATION_PORT) { $env:LOCAL_TRANSLATION_PORT } else { "8770" }

if (-not $env:LOCAL_TRANSLATION_MODEL_PATH -and -not $env:LOCAL_TRANSLATION_MODEL_NAME) {
  Write-Host "本地翻译模型未配置或未启动"
  Write-Host "Set LOCAL_TRANSLATION_MODEL_PATH to a local Hugging Face seq2seq model directory before starting translation."
  exit 1
}

Set-Location $root
$projectPython = Join-Path $root ".venv-local-translation\Scripts\python.exe"
$pythonExe = if (Test-Path -LiteralPath $projectPython) { $projectPython } else { "python" }

Write-Host "Using Python: $pythonExe"
Write-Host "Local translation endpoint: $env:LOCAL_TRANSLATION_ENDPOINT/translate"
& $pythonExe .\scripts\local_translation_server.py `
  --model-path $env:LOCAL_TRANSLATION_MODEL_PATH `
  --model-name $env:LOCAL_TRANSLATION_MODEL_NAME `
  --port $env:LOCAL_TRANSLATION_PORT
