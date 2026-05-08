# Migration Plan

Migrate only:

- Tauri 2 + Rust + Vue 3 + TypeScript stack
- `cpal` audio capture boundary
- local ASR server concept and default sherpa/vosk model path
- local ASR WebSocket endpoint
- Qwen/DashScope TTS endpoint and key priority
- TTS voice, format, sample rate, speed, and volume settings
- user settings related to local ASR, local translation, TTS, and language direction

Do not migrate:

- cloud streaming ASR
- cloud HTTP ASR
- DashScope ASR
- legacy cloud ASR model names
- legacy cloud translation model names
- LLM corrector, rewrite, polish, terminology repair, fallback
- shared system/microphone draft state
- final-as-visual-segment logic
- startMs/endMs draft keys
- translation-overwrites-raw-caption behavior
