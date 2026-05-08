# Module Boundaries

## SystemSubtitle owns

- frontend components under `components/system-subtitle`
- hooks under `hooks/system-subtitle`
- store under `stores/system-subtitle`
- Rust session in `system_subtitle_session.rs`
- independent visual caption segmenter
- independent translation queue
- no TTS

## MicInterpretation owns

- frontend components under `components/mic-interpretation`
- hooks under `hooks/mic-interpretation`
- store under `stores/mic-interpretation`
- Rust session in `mic_interpretation_session.rs`
- independent interpretation segmenter
- independent translation queue
- independent TTS queue/status

Neither module imports the other's business hook, store, or component.
