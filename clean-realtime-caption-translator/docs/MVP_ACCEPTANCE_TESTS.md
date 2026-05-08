# MVP Acceptance Tests

- System audio session starts without touching microphone store.
- On Windows, SystemSubtitle captures default output device through cpal/WASAPI loopback and sends mono PCM16 to local ASR.
- System ASR partial immediately updates raw caption.
- System final updates a `RecognitionItem`, not necessarily a finalized visual segment.
- System translation fills `translatedText` without overwriting raw text.
- Microphone session starts without touching system subtitle store.
- Microphone capture sends mono PCM16 to local ASR.
- Microphone final queues local translation.
- Microphone completed translation queues Qwen TTS with text from `translatedText`.
- TTS failure updates only TTS status and does not clear translation text.
- Every event includes `sessionKind`.
- Repo contains no cloud ASR, cloud translation, or LLM corrector implementation.
- Logs redact API keys, bearer tokens, authorization headers, secrets, and passwords.
