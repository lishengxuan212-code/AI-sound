use crate::audio_session::SessionKind;
use crate::caption_state::{update_raw_transcript, update_translated_text};
use crate::diagnostics::diagnostic;
use crate::event_emitter::emit_session_event;
use crate::local_translation_bridge::{translate_local, LocalTranslationRequest};
use crate::settings::load_app_settings;
use crate::tts_bridge::{synthesize_qwen_tts, TtsInvokeRequest};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::json;
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread;
use tauri::AppHandle;
use tokio::sync::{mpsc, Mutex};
use tokio::time::{sleep, timeout, Duration, Instant};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

const ASR_SAMPLE_RATE: u32 = 16_000;
const ASR_CHUNK_SAMPLES: usize = 1_600;
const VOICE_RMS_THRESHOLD: f32 = 400.0 / i16::MAX as f32;
const FORCE_FINALIZE_MS: u64 = 3_500;
const TRANSLATION_CONTEXT_SENTENCES: usize = 4;

#[derive(Debug, Clone, Copy)]
pub enum AudioCaptureSource {
    Microphone,
    SystemLoopback,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalAsrConnectionStatus {
    #[serde(rename = "sessionKind")]
    pub session_kind: SessionKind,
    pub provider: String,
    pub endpoint: String,
    pub connected: bool,
    pub error: Option<String>,
}

pub async fn check_local_asr(session_kind: SessionKind) -> LocalAsrConnectionStatus {
    let settings = load_app_settings();
    match connect_async(&settings.local_asr.ws_url).await {
        Ok((mut socket, _response)) => {
            let health = json!({ "type": "health" }).to_string();
            if socket.send(Message::Text(health)).await.is_err() {
                return LocalAsrConnectionStatus {
                    session_kind,
                    provider: settings.local_asr.provider,
                    endpoint: settings.local_asr.ws_url,
                    connected: false,
                    error: Some("ASR 服务已连接，但健康检查发送失败。".to_string()),
                };
            }
            match timeout(Duration::from_secs(2), socket.next()).await {
                Ok(Some(Ok(Message::Text(text)))) => {
                    match serde_json::from_str::<serde_json::Value>(&text) {
                        Ok(value)
                            if value
                                .get("ok")
                                .and_then(|item| item.as_bool())
                                .unwrap_or(false)
                                && value
                                    .get("provider")
                                    .and_then(|item| item.as_str())
                                    .map(|provider| {
                                        provider.contains(settings.local_asr.provider.as_str())
                                    })
                                    .unwrap_or(false) =>
                        {
                            LocalAsrConnectionStatus {
                                session_kind,
                                provider: value
                                    .get("provider")
                                    .and_then(|item| item.as_str())
                                    .unwrap_or(settings.local_asr.provider.as_str())
                                    .to_string(),
                                endpoint: settings.local_asr.ws_url,
                                connected: true,
                                error: None,
                            }
                        }
                        Ok(value)
                            if value
                                .get("ok")
                                .and_then(|item| item.as_bool())
                                .unwrap_or(false) =>
                        {
                            LocalAsrConnectionStatus {
                                session_kind,
                                provider: value
                                    .get("provider")
                                    .and_then(|item| item.as_str())
                                    .unwrap_or(settings.local_asr.provider.as_str())
                                    .to_string(),
                                endpoint: settings.local_asr.ws_url,
                                connected: false,
                                error: Some(format!(
                                    "ASR 服务引擎不匹配：当前={}，期望={}",
                                    value
                                        .get("provider")
                                        .and_then(|item| item.as_str())
                                        .unwrap_or("unknown"),
                                    settings.local_asr.provider
                                )),
                            }
                        }
                        Ok(value) => LocalAsrConnectionStatus {
                            session_kind,
                            provider: value
                                .get("provider")
                                .and_then(|item| item.as_str())
                                .unwrap_or(settings.local_asr.provider.as_str())
                                .to_string(),
                            endpoint: settings.local_asr.ws_url,
                            connected: false,
                            error: Some(
                                value
                                    .get("error")
                                    .and_then(|item| item.as_str())
                                    .unwrap_or("ASR 服务未就绪。")
                                    .to_string(),
                            ),
                        },
                        Err(_) => LocalAsrConnectionStatus {
                            session_kind,
                            provider: settings.local_asr.provider,
                            endpoint: settings.local_asr.ws_url,
                            connected: false,
                            error: Some("ASR 服务健康检查返回格式无效。".to_string()),
                        },
                    }
                }
                Ok(_) | Err(_) => LocalAsrConnectionStatus {
                    session_kind,
                    provider: settings.local_asr.provider,
                    endpoint: settings.local_asr.ws_url,
                    connected: false,
                    error: Some("ASR 服务健康检查无响应。".to_string()),
                },
            }
        }
        Err(error) => LocalAsrConnectionStatus {
            session_kind,
            provider: settings.local_asr.provider,
            endpoint: settings.local_asr.ws_url,
            connected: false,
            error: Some(error.to_string()),
        },
    }
}

pub async fn stream_audio_to_local_asr(
    app: AppHandle,
    session_kind: SessionKind,
    session_id: String,
    running: Arc<AtomicBool>,
    source: AudioCaptureSource,
) -> Result<(), String> {
    let settings = load_app_settings();
    let (socket, _response) = connect_async(&settings.local_asr.ws_url)
        .await
        .map_err(|error| error.to_string())?;
    let (mut writer, mut reader) = socket.split();
    let (tx, mut rx) = mpsc::channel::<Vec<u8>>(32);
    let captured = build_audio_stream(tx, source, running.clone())?;
    let speech_state = Arc::new(Mutex::new(SpeechSegmentState::default()));
    let finalize_task = tokio::spawn(run_silence_finalizer(
        app.clone(),
        speech_state.clone(),
        running.clone(),
        session_kind,
        session_id.clone(),
    ));
    writer
        .send(Message::Text(
            json!({
                "type": "start",
                "sessionKind": session_kind,
                "sessionId": session_id,
                "provider": settings.local_asr.provider,
                "language": match session_kind {
                    SessionKind::SystemSubtitle => settings.language.system_source_lang,
                    SessionKind::MicInterpretation => settings.language.mic_source_lang,
                },
                "sampleRate": captured.sample_rate,
                "format": "pcm_s16le"
            })
            .to_string(),
        ))
        .await
        .map_err(|error| error.to_string())?;
    captured.start()?;
    diagnostic(
        match session_kind {
            SessionKind::SystemSubtitle => "[SYS][AUDIO]",
            SessionKind::MicInterpretation => "[MIC][AUDIO]",
        },
        json!({
            "event.type": "audio_stream_playing",
            "sessionKind": session_kind,
            "sessionId": session_id,
            "sampleRate": captured.sample_rate,
            "source": match source {
                AudioCaptureSource::Microphone => "microphone",
                AudioCaptureSource::SystemLoopback => "system_loopback",
            }
        }),
    );
    let mut audio_chunk_count: u64 = 0;

    while running.load(Ordering::SeqCst) {
        tokio::select! {
            Some(chunk) = rx.recv() => {
                audio_chunk_count += 1;
                {
                    let mut state = speech_state.lock().await;
                    state.record_audio(pcm16_has_voice(&chunk));
                }
                if audio_chunk_count <= 3 || audio_chunk_count % 100 == 0 {
                    diagnostic(
                        match session_kind {
                            SessionKind::SystemSubtitle => "[SYS][AUDIO]",
                            SessionKind::MicInterpretation => "[MIC][AUDIO]",
                        },
                        json!({
                            "event.type": "audio_chunk",
                            "sessionKind": session_kind,
                            "sessionId": session_id,
                            "chunkIndex": audio_chunk_count,
                            "byteLength": chunk.len()
                        }),
                    );
                }
                writer.send(Message::Binary(chunk)).await.map_err(|error| error.to_string())?;
            }
            Some(message) = reader.next() => {
                match message {
                    Ok(Message::Text(text)) => {
                        emit_asr_text_message(&app, speech_state.clone(), session_kind, &session_id, &text).await?;
                    }
                    Ok(_) => {}
                    Err(error) => return Err(error.to_string()),
                }
            }
            else => break,
        }
    }

    drop(captured);
    let _ = writer.close().await;
    let _ = finalize_task.await;
    Ok(())
}

async fn emit_asr_text_message(
    app: &AppHandle,
    speech_state: Arc<Mutex<SpeechSegmentState>>,
    session_kind: SessionKind,
    session_id: &str,
    text: &str,
) -> Result<(), String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
    let parsed = parse_asr_message(&value);
    let utterance_id = parsed.utterance_id.as_str();
    let transcript = parsed.transcript.as_str();
    let is_final = parsed.is_final;
    let event_type = match (session_kind, is_final) {
        (SessionKind::SystemSubtitle, false) => "system_asr_partial",
        (SessionKind::SystemSubtitle, true) => "system_asr_final",
        (SessionKind::MicInterpretation, false) => "mic_asr_partial",
        (SessionKind::MicInterpretation, true) => "mic_asr_final",
    };
    diagnostic(
        match (session_kind, is_final) {
            (SessionKind::SystemSubtitle, false) => "[SYS][ASR_PARTIAL]",
            (SessionKind::SystemSubtitle, true) => "[SYS][ASR_FINAL]",
            (SessionKind::MicInterpretation, false) => "[MIC][ASR_PARTIAL]",
            (SessionKind::MicInterpretation, true) => "[MIC][ASR_FINAL]",
        },
        json!({
            "sessionKind": session_kind,
            "sessionId": session_id,
            "utteranceId": utterance_id,
            "rawTextLength": transcript.chars().count(),
            "finalizeReason": if is_final { "asr_final" } else { "partial" }
        }),
    );
    let source_lang = configured_source_lang(session_kind);
    let visible_transcript = {
        let mut state = speech_state.lock().await;
        state.record_asr(&parsed);
        state.combined_text()
    };
    if !visible_transcript.trim().is_empty() {
        update_raw_transcript(session_kind, session_id, utterance_id, &visible_transcript);
    }
    emit_session_event(
        app,
        event_type,
        session_kind,
        session_id,
        json!({
            "utteranceId": utterance_id,
            "text": transcript,
            "provider": parsed.provider,
            "renderPolicy": if is_final { "await_vad_freeze" } else { "realtime_draft" }
        }),
    )?;
    if is_final && detected_lang(transcript) != Some(source_lang.as_str()) {
        diagnostic(
            match session_kind {
                SessionKind::SystemSubtitle => "[SYS][ASR_FINAL]",
                SessionKind::MicInterpretation => "[MIC][ASR_FINAL]",
            },
            json!({
                "sessionKind": session_kind,
                "sessionId": session_id,
                "utteranceId": utterance_id,
                "rawTextLength": transcript.chars().count(),
                "finalizeReason": "asr_final_source_language_mismatch",
                "expectedSourceLang": source_lang
            }),
        );
    }

    Ok(())
}

#[derive(Default)]
struct SpeechSegmentState {
    committed_text: String,
    partial_text: String,
    completed_prefix: String,
    context_history: VecDeque<String>,
    utterance_id: String,
    provider: String,
    segment_started_at: Option<Instant>,
    last_voice_at: Option<Instant>,
    last_text_at: Option<Instant>,
    translating: bool,
}

impl SpeechSegmentState {
    fn record_audio(&mut self, has_voice: bool) {
        if has_voice {
            self.last_voice_at = Some(Instant::now());
        }
    }

    fn record_asr(&mut self, parsed: &ParsedAsrMessage) {
        let text = trim_completed_prefix(parsed.transcript.trim(), &self.completed_prefix);
        if text.is_empty() {
            return;
        }
        self.last_text_at = Some(Instant::now());
        if self.segment_started_at.is_none() {
            self.segment_started_at = self.last_voice_at.or(self.last_text_at);
        }
        self.utterance_id = parsed.utterance_id.clone();
        self.provider = parsed.provider.clone();
        if parsed.is_final {
            merge_transcript(&mut self.committed_text, &text);
            self.partial_text.clear();
        } else {
            self.partial_text = text;
        }
    }

    fn should_finalize(
        &self,
        now: Instant,
        config: &SegmentFinalizeConfig,
    ) -> Option<&'static str> {
        if self.translating || self.combined_text().trim().is_empty() {
            return None;
        }
        let combined = self.combined_text();
        let readable_len = readable_len(&combined, &config.source_lang);
        if readable_len >= config.hard_chars.max(config.preferred_chars) {
            return Some("hard_length_limit");
        }
        if self
            .segment_started_at
            .map(|started| now.duration_since(started) >= Duration::from_millis(FORCE_FINALIZE_MS))
            .unwrap_or(false)
        {
            return Some("force_timeout");
        }
        if readable_len >= config.preferred_chars && has_semantic_boundary(&combined) {
            return Some("semantic_length_boundary");
        }
        let Some(started_at) = self.segment_started_at else {
            return None;
        };
        if now.duration_since(started_at)
            >= Duration::from_millis(u64::from(config.silence_ms + config.tail_delay_ms))
        {
            Some("interval")
        } else {
            None
        }
    }

    fn take_segment(&mut self, finalize_reason: &'static str) -> Option<PendingSpeechSegment> {
        let text = self.combined_text();
        if text.trim().is_empty() {
            return None;
        }
        self.translating = true;
        merge_transcript(&mut self.completed_prefix, &text);
        let pending = PendingSpeechSegment {
            utterance_id: if self.utterance_id.is_empty() {
                uuid::Uuid::new_v4().to_string()
            } else {
                self.utterance_id.clone()
            },
            source_text: text,
            finalize_reason,
            context_before: self.context_before(TRANSLATION_CONTEXT_SENTENCES),
        };
        self.committed_text.clear();
        self.partial_text.clear();
        self.last_voice_at = None;
        self.last_text_at = None;
        self.segment_started_at = None;
        self.utterance_id.clear();
        self.translating = false;
        Some(pending)
    }

    fn finish_translation(&mut self, completed_text: &str) {
        let completed = completed_text.trim();
        if completed.is_empty() {
            return;
        }
        self.context_history.push_back(completed.to_string());
        while self.context_history.len() > TRANSLATION_CONTEXT_SENTENCES {
            self.context_history.pop_front();
        }
    }

    fn context_before(&self, max_sentences: usize) -> Vec<String> {
        self.context_history
            .iter()
            .rev()
            .take(max_sentences)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    fn combined_text(&self) -> String {
        if self.partial_text.is_empty() {
            return self.committed_text.clone();
        }
        if self.committed_text.is_empty() || self.partial_text.starts_with(&self.committed_text) {
            return self.partial_text.clone();
        }
        format!("{} {}", self.committed_text, self.partial_text)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}

struct PendingSpeechSegment {
    utterance_id: String,
    source_text: String,
    finalize_reason: &'static str,
    context_before: Vec<String>,
}

#[derive(Clone)]
struct SegmentFinalizeConfig {
    silence_ms: u32,
    tail_delay_ms: u32,
    preferred_chars: u32,
    hard_chars: u32,
    source_lang: String,
}

async fn run_silence_finalizer(
    app: AppHandle,
    speech_state: Arc<Mutex<SpeechSegmentState>>,
    running: Arc<AtomicBool>,
    session_kind: SessionKind,
    session_id: String,
) {
    loop {
        sleep(Duration::from_millis(200)).await;
        let should_stop = !running.load(Ordering::SeqCst);
        let pending = {
            let mut state = speech_state.lock().await;
            let finalize_config = segment_finalize_config(session_kind);
            if let Some(finalize_reason) = state.should_finalize(Instant::now(), &finalize_config) {
                state.take_segment(finalize_reason)
            } else if should_stop && !state.combined_text().trim().is_empty() {
                state.take_segment("session_stop")
            } else {
                None
            }
        };
        if let Some(segment) = pending {
            let completed_source_text = segment.source_text.clone();
            translate_final_segment(
                &app,
                session_kind,
                &session_id,
                segment.utterance_id,
                segment.source_text,
                segment.finalize_reason,
                segment.context_before,
            )
            .await;
            let mut state = speech_state.lock().await;
            state.finish_translation(&completed_source_text);
        }
        if should_stop {
            break;
        }
    }
}

fn segment_finalize_config(session_kind: SessionKind) -> SegmentFinalizeConfig {
    let settings = load_app_settings();
    match session_kind {
        SessionKind::SystemSubtitle => SegmentFinalizeConfig {
            silence_ms: settings
                .system_segmenter
                .silence_for_visual_finalize_ms
                .clamp(200, 5_000),
            tail_delay_ms: 0,
            preferred_chars: readable_limit_for_lang(&settings.language.system_source_lang),
            hard_chars: settings.system_segmenter.hard_max_chars,
            source_lang: settings.language.system_source_lang.clone(),
        },
        SessionKind::MicInterpretation => SegmentFinalizeConfig {
            silence_ms: settings
                .mic_segmenter
                .silence_for_translate_ms
                .clamp(200, 5_000),
            tail_delay_ms: 0,
            preferred_chars: readable_limit_for_lang(&settings.language.mic_source_lang),
            hard_chars: settings.mic_segmenter.hard_max_chars,
            source_lang: settings.language.mic_source_lang.clone(),
        },
    }
}

async fn translate_final_segment(
    app: &AppHandle,
    session_kind: SessionKind,
    session_id: &str,
    utterance_id: String,
    asr_text: String,
    finalize_reason: &str,
    context_before: Vec<String>,
) {
    let settings = load_app_settings();
    let (source_lang, target_lang) = match session_kind {
        SessionKind::SystemSubtitle => (
            settings.language.system_source_lang.clone(),
            settings.language.system_target_lang.clone(),
        ),
        SessionKind::MicInterpretation => (
            settings.language.mic_source_lang.clone(),
            settings.language.mic_target_lang.clone(),
        ),
    };
    let Some(source_text) = filter_source_text_by_expected_lang(
        session_kind,
        session_id,
        &utterance_id,
        asr_text,
        &source_lang,
    ) else {
        return;
    };
    update_raw_transcript(session_kind, session_id, &utterance_id, &source_text);

    let request_id = uuid::Uuid::new_v4().to_string();
    diagnostic(
        match session_kind {
            SessionKind::SystemSubtitle => "[SYS][TRANSLATION_PENDING]",
            SessionKind::MicInterpretation => "[MIC][TRANSLATION_PENDING]",
        },
        json!({
            "sessionKind": session_kind,
            "sessionId": session_id,
            "utteranceId": utterance_id,
            "translationId": request_id,
            "textLength": source_text.chars().count(),
            "translationStatus": "pending",
            "finalizeReason": finalize_reason
        }),
    );
    let response = translate_local(LocalTranslationRequest {
        request_id: request_id.clone(),
        session_kind,
        source_text: source_text.clone(),
        source_lang: source_lang.clone(),
        target_lang: target_lang.clone(),
        context_before,
    })
    .await;
    let translated_text = normalize_translated_text(
        session_kind,
        session_id,
        &utterance_id,
        response.translated_text.clone(),
        &target_lang,
    )
    .await;
    if response.status == "completed" && !translated_text.trim().is_empty() {
        update_translated_text(session_kind, session_id, &utterance_id, &translated_text);
    }
    let translation_event = match session_kind {
        SessionKind::SystemSubtitle => "system_translation_final",
        SessionKind::MicInterpretation => "mic_translation_final",
    };
    let _ = emit_session_event(
        app,
        translation_event,
        session_kind,
        session_id,
        json!({
            "id": request_id,
            "sessionKind": session_kind,
            "recognitionItemIds": [utterance_id],
            "sourceText": source_text,
            "translatedText": translated_text.clone(),
            "sourceLang": source_lang,
            "targetLang": target_lang,
            "status": response.status,
            "error": response.error.clone(),
            "startedAt": chrono_like_now_ms(),
            "completedAt": chrono_like_now_ms()
        }),
    );
    diagnostic(
        match session_kind {
            SessionKind::SystemSubtitle => "[SYS][SEGMENT]",
            SessionKind::MicInterpretation => "[MIC][SEGMENT]",
        },
        json!({
            "sessionKind": session_kind,
            "sessionId": session_id,
            "utteranceId": utterance_id,
            "rawTextLength": source_text.chars().count(),
            "translationId": request_id,
            "translationStatus": response.status,
            "finalizeReason": finalize_reason
        }),
    );
    diagnostic(
        match session_kind {
            SessionKind::SystemSubtitle => "[SYS][TRANSLATION_FINAL]",
            SessionKind::MicInterpretation => "[MIC][TRANSLATION_FINAL]",
        },
        json!({
            "sessionKind": session_kind,
            "sessionId": session_id,
            "utteranceId": utterance_id,
            "translationId": request_id,
            "textLength": source_text.chars().count(),
            "translationStatus": response.status,
            "error": response.error
        }),
    );
    if session_kind == SessionKind::MicInterpretation
        && response.status == "completed"
        && !translated_text.trim().is_empty()
    {
        let _ = synthesize_qwen_tts(
            app.clone(),
            TtsInvokeRequest {
                session_kind: SessionKind::MicInterpretation,
                tts_id: uuid::Uuid::new_v4().to_string(),
                translation_item_id: request_id,
                text: translated_text,
                target_lang,
            },
        )
        .await;
    }
}

struct ParsedAsrMessage {
    utterance_id: String,
    transcript: String,
    is_final: bool,
    provider: String,
}

fn parse_asr_message(value: &serde_json::Value) -> ParsedAsrMessage {
    let utterance_id = value
        .get("utteranceId")
        .or_else(|| value.get("utterance_id"))
        .or_else(|| value.get("segment_id"))
        .or_else(|| value.pointer("/header/task_id"))
        .and_then(|item| item.as_str())
        .unwrap_or("local-asr-utterance")
        .to_string();
    let transcript = value
        .get("text")
        .or_else(|| value.get("rawTranscript"))
        .or_else(|| value.pointer("/payload/output/sentence/text"))
        .and_then(|item| item.as_str())
        .unwrap_or("")
        .to_string();
    let is_final = value
        .get("isFinal")
        .or_else(|| value.get("is_final"))
        .or_else(|| value.pointer("/payload/output/sentence/sentence_end"))
        .and_then(|item| item.as_bool())
        .unwrap_or(false);
    let provider = value
        .get("provider")
        .or_else(|| value.pointer("/header/provider"))
        .and_then(|item| item.as_str())
        .unwrap_or("local-sherpa-onnx-realtime")
        .to_string();
    ParsedAsrMessage {
        utterance_id,
        transcript,
        is_final,
        provider,
    }
}

fn filter_source_text_by_expected_lang(
    session_kind: SessionKind,
    session_id: &str,
    utterance_id: &str,
    text: String,
    expected_source_lang: &str,
) -> Option<String> {
    let normalized_expected = normalize_lang(expected_source_lang);
    let Some(detected) = detected_lang(&text) else {
        return Some(text);
    };
    if detected == normalized_expected {
        return Some(text);
    }

    diagnostic(
        match session_kind {
            SessionKind::SystemSubtitle => "[SYS][ASR_FINAL]",
            SessionKind::MicInterpretation => "[MIC][ASR_FINAL]",
        },
        json!({
            "sessionKind": session_kind,
            "sessionId": session_id,
            "utteranceId": utterance_id,
            "textLength": text.chars().count(),
            "finalizeReason": "source_language_mismatch_kept",
            "sourceLang": detected,
            "expectedSourceLang": normalized_expected
        }),
    );
    Some(text)
}

async fn normalize_translated_text(
    session_kind: SessionKind,
    session_id: &str,
    utterance_id: &str,
    text: String,
    expected_target_lang: &str,
) -> String {
    let normalized_expected = normalize_lang(expected_target_lang);
    let Some(detected) = detected_lang(&text) else {
        return text;
    };
    if detected == normalized_expected {
        return text;
    }
    if normalized_expected != "en" && normalized_expected != "zh" {
        return text;
    }

    let request_id = uuid::Uuid::new_v4().to_string();
    diagnostic(
        match session_kind {
            SessionKind::SystemSubtitle => "[SYS][TRANSLATION_PENDING]",
            SessionKind::MicInterpretation => "[MIC][TRANSLATION_PENDING]",
        },
        json!({
            "sessionKind": session_kind,
            "sessionId": session_id,
            "utteranceId": utterance_id,
            "translationId": request_id,
            "textLength": text.chars().count(),
            "translationStatus": "pending",
            "finalizeReason": "target_language_normalize",
            "sourceLang": detected,
            "targetLang": normalized_expected
        }),
    );
    let response = translate_local(LocalTranslationRequest {
        request_id,
        session_kind,
        source_text: text.clone(),
        source_lang: detected.to_string(),
        target_lang: normalized_expected.to_string(),
        context_before: Vec::new(),
    })
    .await;
    if response.status == "completed" && !response.translated_text.trim().is_empty() {
        response.translated_text
    } else {
        diagnostic(
            match session_kind {
                SessionKind::SystemSubtitle => "[SYS][TRANSLATION_ERROR]",
                SessionKind::MicInterpretation => "[MIC][TRANSLATION_ERROR]",
            },
            json!({
                "sessionKind": session_kind,
                "sessionId": session_id,
                "utteranceId": utterance_id,
                "translationStatus": response.status,
                "finalizeReason": "target_language_normalize_failed",
                "error": response.error
            }),
        );
        text
    }
}

fn configured_source_lang(session_kind: SessionKind) -> String {
    let settings = load_app_settings();
    match session_kind {
        SessionKind::SystemSubtitle => settings.language.system_source_lang,
        SessionKind::MicInterpretation => settings.language.mic_source_lang,
    }
}

fn normalize_lang(lang: &str) -> &str {
    let normalized = lang.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "zh" | "zh-cn" | "zh_hans" | "chinese" => "zh",
        "en" | "en-us" | "english" => "en",
        _ => lang.trim(),
    }
}

fn detected_lang(text: &str) -> Option<&'static str> {
    let mut chinese = 0usize;
    let mut latin = 0usize;
    for ch in text.chars() {
        if ('\u{4e00}'..='\u{9fff}').contains(&ch) {
            chinese += 1;
        } else if ch.is_ascii_alphabetic() {
            latin += 1;
        }
    }
    if chinese == 0 && latin == 0 {
        None
    } else if chinese >= latin {
        Some("zh")
    } else {
        Some("en")
    }
}

fn merge_transcript(target: &mut String, next: &str) {
    let next = next.trim();
    if next.is_empty() || target == next || target.ends_with(next) {
        return;
    }
    if next.starts_with(target.as_str()) {
        *target = next.to_string();
        return;
    }
    if !target.is_empty() {
        target.push(' ');
    }
    target.push_str(next);
}

fn trim_completed_prefix(text: &str, completed_prefix: &str) -> String {
    let text = text.trim();
    let completed = completed_prefix.trim();
    if text.is_empty() || completed.is_empty() {
        return text.to_string();
    }
    if text == completed || completed.ends_with(text) {
        return String::new();
    }
    if let Some(rest) = text.strip_prefix(completed) {
        return rest.trim().to_string();
    }
    text.to_string()
}

fn pcm16_has_voice(chunk: &[u8]) -> bool {
    if chunk.len() < 2 {
        return false;
    }
    let mut sum = 0f64;
    let mut count = 0usize;
    for bytes in chunk.chunks_exact(2) {
        let sample = i16::from_le_bytes([bytes[0], bytes[1]]) as f64 / i16::MAX as f64;
        sum += sample * sample;
        count += 1;
    }
    if count == 0 {
        return false;
    }
    (sum / count as f64).sqrt() as f32 >= VOICE_RMS_THRESHOLD
}

fn readable_limit_for_lang(lang: &str) -> u32 {
    match normalize_lang(lang) {
        "zh" => 16,
        "en" => 60,
        _ => 40,
    }
}

fn readable_len(text: &str, lang: &str) -> u32 {
    match normalize_lang(lang) {
        "en" => text.chars().filter(|ch| !ch.is_whitespace()).count() as u32,
        _ => text.chars().filter(|ch| !ch.is_whitespace()).count() as u32,
    }
}

fn has_semantic_boundary(text: &str) -> bool {
    text.trim_end()
        .chars()
        .last()
        .map(|ch| {
            matches!(
                ch,
                '.' | '?' | '!' | ',' | ';' | ':' | '。' | '？' | '！' | '，' | '；' | '：'
            )
        })
        .unwrap_or(false)
}

fn chrono_like_now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

struct CapturedAudioStream {
    cpal_stream: Option<cpal::Stream>,
    loopback_stop: Option<Arc<AtomicBool>>,
    loopback_handle: Option<thread::JoinHandle<()>>,
    sample_rate: u32,
}

impl CapturedAudioStream {
    fn start(&self) -> Result<(), String> {
        if let Some(stream) = &self.cpal_stream {
            stream.play().map_err(|error| error.to_string())?;
        }
        Ok(())
    }
}

impl Drop for CapturedAudioStream {
    fn drop(&mut self) {
        if let Some(stop) = &self.loopback_stop {
            stop.store(false, Ordering::SeqCst);
        }
        if let Some(handle) = self.loopback_handle.take() {
            let _ = handle.join();
        }
    }
}

fn build_audio_stream(
    tx: mpsc::Sender<Vec<u8>>,
    source: AudioCaptureSource,
    running: Arc<AtomicBool>,
) -> Result<CapturedAudioStream, String> {
    if matches!(source, AudioCaptureSource::SystemLoopback) {
        return build_system_loopback_stream(tx, running);
    }

    let settings = load_app_settings();
    let host = cpal::default_host();
    let device = match source {
        AudioCaptureSource::Microphone => {
            selected_input_device(&host, &settings.audio_devices.mic_input_device_name).ok_or_else(
                || "麦克风音频采集失败：未找到配置的麦克风输入设备或默认麦克风。".to_string(),
            )?
        }
        AudioCaptureSource::SystemLoopback => {
            selected_output_device(&host, &settings.audio_devices.system_output_device_name)
                .ok_or_else(|| {
                    "系统音频采集失败：未找到配置的扬声器输出设备或默认输出设备。".to_string()
                })?
        }
    };
    let config = match source {
        AudioCaptureSource::Microphone => device
            .default_input_config()
            .map_err(|error| error.to_string())?,
        AudioCaptureSource::SystemLoopback => device
            .default_output_config()
            .map_err(|error| error.to_string())?,
    };
    let stream_config: cpal::StreamConfig = config.clone().into();
    let source_sample_rate = stream_config.sample_rate.0;
    let channels = usize::from(stream_config.channels);
    let err_fn = |error| eprintln!("[MIC][AUDIO] input stream error: {}", error);

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            let mut normalizer =
                AsrPcmChunker::new(source_sample_rate, ASR_SAMPLE_RATE, ASR_CHUNK_SAMPLES);
            device
                .build_input_stream(
                    &stream_config,
                    move |data: &[f32], _| {
                        normalizer.push_f32_interleaved(data, channels, &tx);
                    },
                    err_fn,
                    None,
                )
                .map_err(|error| error.to_string())
        }
        cpal::SampleFormat::I16 => {
            let mut normalizer =
                AsrPcmChunker::new(source_sample_rate, ASR_SAMPLE_RATE, ASR_CHUNK_SAMPLES);
            device
                .build_input_stream(
                    &stream_config,
                    move |data: &[i16], _| {
                        normalizer.push_i16_interleaved(data, channels, &tx);
                    },
                    err_fn,
                    None,
                )
                .map_err(|error| error.to_string())
        }
        cpal::SampleFormat::U16 => {
            let mut normalizer =
                AsrPcmChunker::new(source_sample_rate, ASR_SAMPLE_RATE, ASR_CHUNK_SAMPLES);
            device
                .build_input_stream(
                    &stream_config,
                    move |data: &[u16], _| {
                        normalizer.push_u16_interleaved(data, channels, &tx);
                    },
                    err_fn,
                    None,
                )
                .map_err(|error| error.to_string())
        }
        other => Err(format!("音频采样格式不支持，请检查 ASR 设置：{:?}", other)),
    }?;

    Ok(CapturedAudioStream {
        cpal_stream: Some(stream),
        loopback_stop: None,
        loopback_handle: None,
        sample_rate: ASR_SAMPLE_RATE,
    })
}

fn build_system_loopback_stream(
    tx: mpsc::Sender<Vec<u8>>,
    running: Arc<AtomicBool>,
) -> Result<CapturedAudioStream, String> {
    let stop = running.clone();
    let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
    let handle = thread::Builder::new()
        .name("SystemLoopbackCapture".to_string())
        .spawn(move || {
            if let Err(error) = run_wasapi_loopback_capture(tx, stop, ready_tx) {
                eprintln!("[SYS][AUDIO] loopback capture error: {}", error);
            }
        })
        .map_err(|error| error.to_string())?;

    match ready_rx.recv().map_err(|error| error.to_string())? {
        Ok(()) => {}
        Err(error) => {
            running.store(false, Ordering::SeqCst);
            let _ = handle.join();
            return Err(error);
        }
    }

    Ok(CapturedAudioStream {
        cpal_stream: None,
        loopback_stop: Some(running),
        loopback_handle: Some(handle),
        sample_rate: ASR_SAMPLE_RATE,
    })
}

#[cfg(target_os = "windows")]
fn run_wasapi_loopback_capture(
    tx: mpsc::Sender<Vec<u8>>,
    running: Arc<AtomicBool>,
    ready_tx: std::sync::mpsc::SyncSender<Result<(), String>>,
) -> Result<(), String> {
    use wasapi::{initialize_mta, DeviceEnumerator, Direction, SampleType, StreamMode, WaveFormat};

    let result = (|| -> Result<_, String> {
        initialize_mta().ok().map_err(|error| error.to_string())?;
        let enumerator = DeviceEnumerator::new().map_err(|error| error.to_string())?;
        let device = selected_wasapi_render_device(&enumerator)?;
        let mut audio_client = device
            .get_iaudioclient()
            .map_err(|error| error.to_string())?;
        let desired_format = WaveFormat::new(
            32,
            32,
            &SampleType::Float,
            ASR_SAMPLE_RATE as usize,
            1,
            None,
        );
        let (_, min_time) = audio_client
            .get_device_period()
            .map_err(|error| error.to_string())?;
        let mode = StreamMode::EventsShared {
            autoconvert: true,
            buffer_duration_hns: min_time,
        };

        audio_client
            .initialize_client(&desired_format, &Direction::Capture, &mode)
            .map_err(|error| error.to_string())?;
        let event = audio_client
            .set_get_eventhandle()
            .map_err(|error| error.to_string())?;
        let capture_client = audio_client
            .get_audiocaptureclient()
            .map_err(|error| error.to_string())?;
        let sample_queue = VecDeque::with_capacity(ASR_CHUNK_SAMPLES * 8);
        let chunker = AsrPcmChunker::new(ASR_SAMPLE_RATE, ASR_SAMPLE_RATE, ASR_CHUNK_SAMPLES);

        audio_client
            .start_stream()
            .map_err(|error| error.to_string())?;
        Ok((audio_client, event, capture_client, sample_queue, chunker))
    })();

    let (audio_client, event, capture_client, mut sample_queue, mut chunker) = match result {
        Ok(capture) => {
            let _ = ready_tx.send(Ok(()));
            capture
        }
        Err(error) => {
            let _ = ready_tx.send(Err(error.clone()));
            return Err(error);
        }
    };

    while running.load(Ordering::SeqCst) {
        capture_client
            .read_from_device_to_deque(&mut sample_queue)
            .map_err(|error| error.to_string())?;
        while sample_queue.len() >= 4 * ASR_CHUNK_SAMPLES {
            let mut samples = Vec::with_capacity(ASR_CHUNK_SAMPLES);
            for _ in 0..ASR_CHUNK_SAMPLES {
                let bytes = [
                    sample_queue.pop_front().unwrap_or_default(),
                    sample_queue.pop_front().unwrap_or_default(),
                    sample_queue.pop_front().unwrap_or_default(),
                    sample_queue.pop_front().unwrap_or_default(),
                ];
                samples.push(f32::from_le_bytes(bytes));
            }
            chunker.push_f32_mono(&samples, &tx);
        }
        let _ = event.wait_for_event(100);
    }
    let _ = audio_client.stop_stream();
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn run_wasapi_loopback_capture(
    _tx: mpsc::Sender<Vec<u8>>,
    _running: Arc<AtomicBool>,
    ready_tx: std::sync::mpsc::SyncSender<Result<(), String>>,
) -> Result<(), String> {
    let _ = ready_tx.send(Err("系统音频 loopback 仅支持 Windows。".to_string()));
    Err("系统音频 loopback 仅支持 Windows。".to_string())
}

#[cfg(target_os = "windows")]
fn selected_wasapi_render_device(
    enumerator: &wasapi::DeviceEnumerator,
) -> Result<wasapi::Device, String> {
    let settings = load_app_settings();
    let configured = settings
        .audio_devices
        .system_output_device_name
        .trim()
        .to_string();
    if !configured.is_empty() {
        let collection = enumerator
            .get_device_collection(&wasapi::Direction::Render)
            .map_err(|error| error.to_string())?;
        for device in &collection {
            let device = device.map_err(|error| error.to_string())?;
            let name = device
                .get_friendlyname()
                .map_err(|error| error.to_string())?;
            if name == configured {
                return Ok(device);
            }
        }
    }
    enumerator
        .get_default_device(&wasapi::Direction::Render)
        .map_err(|error| error.to_string())
}

fn selected_input_device(host: &cpal::Host, configured_name: &str) -> Option<cpal::Device> {
    let configured = configured_name.trim();
    if !configured.is_empty() {
        if let Ok(mut devices) = host.input_devices() {
            if let Some(device) =
                devices.find(|device| device.name().ok().as_deref() == Some(configured))
            {
                return Some(device);
            }
        }
    }
    host.default_input_device()
}

fn selected_output_device(host: &cpal::Host, configured_name: &str) -> Option<cpal::Device> {
    let configured = configured_name.trim();
    if !configured.is_empty() {
        if let Ok(mut devices) = host.output_devices() {
            if let Some(device) =
                devices.find(|device| device.name().ok().as_deref() == Some(configured))
            {
                return Some(device);
            }
        }
    }
    host.default_output_device()
}

struct AsrPcmChunker {
    source_rate: u32,
    target_rate: u32,
    resample_accumulator: u32,
    pending: Vec<i16>,
    chunk_samples: usize,
}

impl AsrPcmChunker {
    fn new(source_rate: u32, target_rate: u32, chunk_samples: usize) -> Self {
        Self {
            source_rate: source_rate.max(1),
            target_rate: target_rate.max(1),
            resample_accumulator: 0,
            pending: Vec::with_capacity(chunk_samples * 2),
            chunk_samples,
        }
    }

    fn push_f32_interleaved(
        &mut self,
        samples: &[f32],
        channels: usize,
        tx: &mpsc::Sender<Vec<u8>>,
    ) {
        for sample in samples
            .chunks(channels.max(1))
            .map(|frame| frame.iter().copied().sum::<f32>() / frame.len() as f32)
        {
            self.push_i16_sample((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16, tx);
        }
    }

    fn push_i16_interleaved(
        &mut self,
        samples: &[i16],
        channels: usize,
        tx: &mpsc::Sender<Vec<u8>>,
    ) {
        for sample in samples.chunks(channels.max(1)).map(|frame| {
            (frame.iter().map(|sample| i32::from(*sample)).sum::<i32>() / frame.len() as i32) as i16
        }) {
            self.push_i16_sample(sample, tx);
        }
    }

    fn push_u16_interleaved(
        &mut self,
        samples: &[u16],
        channels: usize,
        tx: &mpsc::Sender<Vec<u8>>,
    ) {
        for sample in samples.chunks(channels.max(1)).map(|frame| {
            let mono =
                frame.iter().map(|sample| u32::from(*sample)).sum::<u32>() / frame.len() as u32;
            (mono as i32 - 32768) as i16
        }) {
            self.push_i16_sample(sample, tx);
        }
    }

    fn push_f32_mono(&mut self, samples: &[f32], tx: &mpsc::Sender<Vec<u8>>) {
        for sample in samples {
            self.push_i16_sample((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16, tx);
        }
    }

    fn push_i16_sample(&mut self, sample: i16, tx: &mpsc::Sender<Vec<u8>>) {
        self.resample_accumulator = self.resample_accumulator.saturating_add(self.target_rate);
        while self.resample_accumulator >= self.source_rate {
            self.pending.push(sample);
            self.resample_accumulator -= self.source_rate;
            if self.pending.len() >= self.chunk_samples {
                let bytes = i16_to_le_bytes(&self.pending[..self.chunk_samples]);
                self.pending.drain(..self.chunk_samples);
                let _ = tx.try_send(bytes);
            }
        }
    }
}

fn i16_to_le_bytes(samples: &[i16]) -> Vec<u8> {
    samples
        .iter()
        .flat_map(|sample| sample.to_le_bytes())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        filter_source_text_by_expected_lang, parse_asr_message, AsrPcmChunker,
        SegmentFinalizeConfig, SpeechSegmentState, ASR_CHUNK_SAMPLES, ASR_SAMPLE_RATE,
    };
    use crate::audio_session::SessionKind;
    use tokio::sync::mpsc;
    use tokio::time::{Duration, Instant};

    #[test]
    fn parses_dashscope_like_local_asr_message() {
        let value = serde_json::json!({
            "header": {
                "event": "result-generated",
                "task_id": "local-asr-1"
            },
            "payload": {
                "output": {
                    "sentence": {
                        "text": "hello world",
                        "sentence_end": true
                    }
                }
            }
        });
        let parsed = parse_asr_message(&value);
        assert_eq!(parsed.utterance_id, "local-asr-1");
        assert_eq!(parsed.transcript, "hello world");
        assert!(parsed.is_final);
    }

    #[tokio::test]
    async fn chunks_microphone_audio_as_16khz_100ms_pcm16() {
        let (tx, mut rx) = mpsc::channel(4);
        let mut chunker = AsrPcmChunker::new(48_000, ASR_SAMPLE_RATE, ASR_CHUNK_SAMPLES);
        let samples = vec![0i16; 4_800];

        chunker.push_i16_interleaved(&samples, 1, &tx);

        let chunk = rx.recv().await.expect("expected one ASR chunk");
        assert_eq!(chunk.len(), ASR_CHUNK_SAMPLES * 2);
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn interval_finalize_does_not_wait_for_voice_silence() {
        let now = Instant::now();
        let mut state = SpeechSegmentState::default();
        state.partial_text = "what is happening here".to_string();
        state.segment_started_at = Some(now - Duration::from_millis(500));
        state.last_voice_at = Some(now);
        state.last_text_at = Some(now);

        let config = SegmentFinalizeConfig {
            silence_ms: 500,
            tail_delay_ms: 0,
            preferred_chars: 60,
            hard_chars: 220,
            source_lang: "en".to_string(),
        };

        assert_eq!(state.should_finalize(now, &config), Some("interval"));
    }

    #[test]
    fn source_language_mismatch_is_kept_for_translation_instead_of_dropped() {
        let filtered = filter_source_text_by_expected_lang(
            SessionKind::SystemSubtitle,
            "session-1",
            "utterance-1",
            "这是中文内容".to_string(),
            "en",
        );

        assert_eq!(filtered.as_deref(), Some("这是中文内容"));
    }

    #[test]
    fn interval_finalize_waits_until_configured_duration() {
        let now = Instant::now();
        let mut state = SpeechSegmentState::default();
        state.partial_text = "what is happening here".to_string();
        state.segment_started_at = Some(now - Duration::from_millis(499));
        state.last_voice_at = Some(now - Duration::from_millis(1_000));
        state.last_text_at = Some(now);

        let config = SegmentFinalizeConfig {
            silence_ms: 500,
            tail_delay_ms: 0,
            preferred_chars: 60,
            hard_chars: 220,
            source_lang: "en".to_string(),
        };

        assert_eq!(state.should_finalize(now, &config), None);
    }
}
