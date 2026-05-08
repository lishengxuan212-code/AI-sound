$ErrorActionPreference = "Stop"

if (-not $env:LOCAL_TRANSLATION_ENDPOINT -and -not $env:LOCAL_TRANSLATION_MODEL_PATH) {
  Write-Host "本地翻译模型未配置"
  Write-Host "Set LOCAL_TRANSLATION_PROVIDER, LOCAL_TRANSLATION_ENDPOINT or LOCAL_TRANSLATION_MODEL_PATH before starting translation."
  exit 1
}

Write-Host "Start your local translation service using:"
Write-Host "  Provider: $env:LOCAL_TRANSLATION_PROVIDER"
Write-Host "  Endpoint: $env:LOCAL_TRANSLATION_ENDPOINT"
Write-Host "  Model:    $env:LOCAL_TRANSLATION_MODEL_NAME"
