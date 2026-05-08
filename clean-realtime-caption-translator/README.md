# Clean Realtime Caption Translator

Clean Windows desktop MVP for two isolated realtime workflows:

- SystemSubtitle: system audio -> local ASR -> raw captions -> local translation -> translated captions.
- MicInterpretation: microphone audio -> local ASR -> local translation -> Qwen TTS -> playback/status.

The project intentionally does not include cloud ASR, cloud translation, LLM correction, polish, rewrite, or fallback logic.

## Stack

- Tauri 2 desktop shell
- Rust backend
- Vue 3 + Vite + TypeScript frontend
- pnpm workspace
- Rust audio capture boundary using `cpal`
- Local ASR WebSocket: `ws://127.0.0.1:8765/ws`
- Local translation provider configured from local env only
- Qwen TTS only for microphone interpretation, model `qwen-qwen-tts-latest`

## Development

```powershell
pnpm install
pnpm test
pnpm build
pnpm tauri:dev
```

Start local services separately:

```powershell
.\scripts\start_local_asr.ps1
.\scripts\start_local_translation.ps1
```

## Safety

API keys are read from environment variables only. Logs and diagnostics must redact `Authorization`, bearer tokens, API keys, secrets, passwords, `DASHSCOPE_API_KEY`, and `GAME_TTS_API_KEY`.

The local ASR server accepts `language` and `prompt` fields for request compatibility, but the default sherpa-onnx/vosk implementation may ignore them. Use local model configuration and post-processing for language preference; do not assume cloud-style `language=en` behavior.
