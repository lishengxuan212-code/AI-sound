# Project Architecture

The app is split into two isolated business modules:

- `SystemSubtitle`: system audio, local ASR, raw caption display, local translation, translated caption display.
- `MicInterpretation`: microphone audio, local ASR, local translation, Qwen TTS, playback/status.

Shared code is limited to common types, utilities, diagnostics, secret redaction, settings, and low-level provider clients. Business state, visual segmentation, translation queues, and TTS state are not shared.

All session events carry `sessionKind` with either `SystemSubtitle` or `MicInterpretation`.

Audio capture uses Rust `cpal`. Microphone capture reads the default input device. SystemSubtitle uses the default output device as a WASAPI loopback input on Windows, then downmixes captured frames to mono PCM16 before sending them to the local ASR WebSocket server.
