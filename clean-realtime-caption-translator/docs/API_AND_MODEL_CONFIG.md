# API And Model Config

## ASR

Only local ASR is allowed.

- Provider: `sherpa_onnx` or `vosk`
- Server: `scripts/local_streaming_asr_server.py`
- Endpoint: `ws://127.0.0.1:8765/ws`
- Default model dir: `local-asr/models/sherpa-onnx-streaming-paraformer-bilingual-zh-en`
- Model identity: `local-sherpa-onnx-realtime`

The local server accepts `language` and `prompt` for request compatibility, but the default local engine may ignore them. Language preference must be handled by local model configuration or post-processing.

## Translation

Only local translation is allowed. The provider reads:

- `LOCAL_TRANSLATION_PROVIDER`
- `LOCAL_TRANSLATION_ENDPOINT`
- `LOCAL_TRANSLATION_MODEL_PATH`
- `LOCAL_TRANSLATION_MODEL_NAME`
- `LOCAL_TRANSLATION_SOURCE_LANG`
- `LOCAL_TRANSLATION_TARGET_LANG`

If translation is not configured, the UI shows `本地翻译模型未配置`. There is no cloud LLM fallback.

## TTS

Only microphone interpretation calls TTS.

- Endpoint default: `https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation`
- Model: `qwen-qwen-tts-latest`
- API key priority: `GAME_TTS_API_KEY`, then `DASHSCOPE_API_KEY`

If the API returns unknown model or model not found, report the error. Do not change model names automatically.
