use crate::audio_session::SessionKind;
use crate::event_emitter::emit_session_event;
use crate::diagnostics::diagnostic;
use crate::local_translation_bridge::{translate_local, LocalTranslationRequest};
use crate::settings::load_app_settings;
use crate::tts_bridge::{synthesize_qwen_tts, TtsInvokeRequest};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde_json::json;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use tauri::AppHandle;

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
        Ok((_socket, _response)) => LocalAsrConnectionStatus {
            session_kind,
            provider: "local-sherpa-onnx-realtime".to_string(),
            endpoint: settings.local_asr.ws_url,
            connected: true,
            error: None,
        },
        Err(error) => LocalAsrConnectionStatus {
            session_kind,
            provider: "local-sherpa-onnx-realtime".to_string(),
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
    let captured = build_audio_stream(tx, source)?;
    writer
        .send(Message::Text(
            json!({
                "type": "start",
                "sessionKind": session_kind,
                "sessionId": session_id,
                "provider": settings.local_asr.provider,
                "language": settings.local_translation.source_lang,
                "sampleRate": captured.sample_rate,
                "format": "pcm_s16le"
            })
            .to_string(),
        ))
        .await
        .map_err(|error| error.to_string())?;
    captured.stream.play().map_err(|error| error.to_string())?;

    while running.load(Ordering::SeqCst) {
        tokio::select! {
            Some(chunk) = rx.recv() => {
                writer.send(Message::Binary(chunk)).await.map_err(|error| error.to_string())?;
            }
            Some(message) = reader.next() => {
                match message {
                    Ok(Message::Text(text)) => {
                        emit_asr_text_message(&app, session_kind, &session_id, &text)?;
                    }
                    Ok(_) => {}
                    Err(error) => return Err(error.to_string()),
                }
            }
            else => break,
        }
    }

    drop(captured.stream);
    let _ = writer.close().await;
    Ok(())
}

fn emit_asr_text_message(app: &AppHandle, session_kind: SessionKind, session_id: &str, text: &str) -> Result<(), String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
    let utterance_id = value
        .get("utteranceId")
        .or_else(|| value.get("utterance_id"))
        .or_else(|| value.get("segment_id"))
        .and_then(|item| item.as_str())
        .unwrap_or("local-asr-utterance");
    let transcript = value
        .get("text")
        .or_else(|| value.get("rawTranscript"))
        .and_then(|item| item.as_str())
        .unwrap_or("");
    let is_final = value
        .get("isFinal")
        .or_else(|| value.get("is_final"))
        .and_then(|item| item.as_bool())
        .unwrap_or(false);
    let event_type = match (session_kind, is_final) {
        (SessionKind::SystemSubtitle, false) => "system_subtitle_partial",
        (SessionKind::SystemSubtitle, true) => "system_subtitle_final",
        (SessionKind::MicInterpretation, false) => "mic_asr_partial",
        (SessionKind::MicInterpretation, true) => "mic_asr_final",
    };

    emit_session_event(
        app,
        event_type,
        session_kind,
        session_id,
        json!({
            "utteranceId": utterance_id,
            "text": transcript,
            "provider": value.get("provider").and_then(|item| item.as_str()).unwrap_or("local-sherpa-onnx-realtime")
        }),
    )?;

    if is_final && !transcript.trim().is_empty() {
            let app_for_task = app.clone();
            let session_id_for_task = session_id.to_string();
            let source_text = transcript.to_string();
            let recognition_id = utterance_id.to_string();
            tokio::spawn(async move {
                let settings = load_app_settings();
                let source_lang = settings.local_translation.source_lang.clone();
                let target_lang = settings.local_translation.target_lang.clone();
                let request_id = uuid::Uuid::new_v4().to_string();
                diagnostic(
                    match session_kind {
                        SessionKind::SystemSubtitle => "[SYS][TRANSLATION_PENDING]",
                        SessionKind::MicInterpretation => "[MIC][TRANSLATION_PENDING]",
                    },
                    json!({
                        "sessionKind": session_kind,
                        "sessionId": session_id_for_task,
                        "utteranceId": recognition_id,
                        "translationId": request_id,
                        "textLength": source_text.chars().count(),
                        "translationStatus": "pending"
                    }),
                );
                let response = translate_local(LocalTranslationRequest {
                    request_id: request_id.clone(),
                    session_kind,
                    source_text: source_text.clone(),
                    source_lang: source_lang.clone(),
                    target_lang: target_lang.clone(),
                    context_before: Vec::new(),
                })
            .await;

            let translation_event = match session_kind {
                SessionKind::SystemSubtitle => "system_translation_final",
                SessionKind::MicInterpretation => "mic_translation_final",
            };
            let _ = emit_session_event(
                &app_for_task,
                translation_event,
                session_kind,
                &session_id_for_task,
                json!({
                    "id": request_id,
                    "sessionKind": session_kind,
                    "recognitionItemIds": [recognition_id],
                    "sourceText": source_text,
                    "translatedText": response.translated_text,
                    "sourceLang": source_lang,
                    "targetLang": target_lang,
                    "status": response.status,
                    "error": response.error,
                    "startedAt": chrono_like_now_ms(),
                    "completedAt": chrono_like_now_ms()
                }),
            );
            diagnostic(
                match session_kind {
                    SessionKind::SystemSubtitle => "[SYS][TRANSLATION_FINAL]",
                    SessionKind::MicInterpretation => "[MIC][TRANSLATION_FINAL]",
                },
                json!({
                    "sessionKind": session_kind,
                    "sessionId": session_id_for_task,
                    "utteranceId": recognition_id,
                    "translationId": request_id,
                    "textLength": source_text.chars().count(),
                    "translationStatus": response.status,
                    "error": response.error
                }),
            );

            if session_kind == SessionKind::MicInterpretation && response.status == "completed" && !response.translated_text.trim().is_empty() {
                let _ = synthesize_qwen_tts(
                    app_for_task,
                    TtsInvokeRequest {
                        session_kind: SessionKind::MicInterpretation,
                        translation_item_id: request_id,
                        text: response.translated_text,
                        target_lang,
                    },
                )
                .await;
            }
        });
    }

    Ok(())
}

fn chrono_like_now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

struct CapturedAudioStream {
    stream: cpal::Stream,
    sample_rate: u32,
}

fn build_audio_stream(tx: mpsc::Sender<Vec<u8>>, source: AudioCaptureSource) -> Result<CapturedAudioStream, String> {
    let host = cpal::default_host();
    let device = match source {
        AudioCaptureSource::Microphone => host
            .default_input_device()
            .ok_or_else(|| "No default microphone input device".to_string())?,
        AudioCaptureSource::SystemLoopback => host
            .default_output_device()
            .ok_or_else(|| "No default system output device for loopback".to_string())?,
    };
    let config = match source {
        AudioCaptureSource::Microphone => device.default_input_config().map_err(|error| error.to_string())?,
        AudioCaptureSource::SystemLoopback => device.default_output_config().map_err(|error| error.to_string())?,
    };
    let stream_config: cpal::StreamConfig = config.clone().into();
    let sample_rate = stream_config.sample_rate.0;
    let channels = usize::from(stream_config.channels);
    let err_fn = |error| eprintln!("[MIC][AUDIO] input stream error: {}", error);

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device
            .build_input_stream(
                &stream_config,
                move |data: &[f32], _| {
                    let bytes = f32_to_i16_le_bytes(data, channels);
                    let _ = tx.try_send(bytes);
                },
                err_fn,
                None,
            )
            .map_err(|error| error.to_string()),
        cpal::SampleFormat::I16 => device
            .build_input_stream(
                &stream_config,
                move |data: &[i16], _| {
                    let mono = downmix_i16(data, channels);
                    let bytes = i16_to_le_bytes(&mono);
                    let _ = tx.try_send(bytes);
                },
                err_fn,
                None,
            )
            .map_err(|error| error.to_string()),
        cpal::SampleFormat::U16 => device
            .build_input_stream(
                &stream_config,
                move |data: &[u16], _| {
                    let downmixed = downmix_u16(data, channels);
                    let converted: Vec<i16> = downmixed.iter().map(|sample| (*sample as i32 - 32768) as i16).collect();
                    let bytes = i16_to_le_bytes(&converted);
                    let _ = tx.try_send(bytes);
                },
                err_fn,
                None,
            )
            .map_err(|error| error.to_string()),
        other => Err(format!("Unsupported microphone sample format: {:?}", other)),
    }?;

    Ok(CapturedAudioStream { stream, sample_rate })
}

fn f32_to_i16_le_bytes(samples: &[f32], channels: usize) -> Vec<u8> {
    let mono = downmix_f32(samples, channels);
    let converted: Vec<i16> = mono
        .iter()
        .map(|sample| (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)
        .collect();
    i16_to_le_bytes(&converted)
}

fn i16_to_le_bytes(samples: &[i16]) -> Vec<u8> {
    samples.iter().flat_map(|sample| sample.to_le_bytes()).collect()
}

fn downmix_f32(samples: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return samples.to_vec();
    }
    samples
        .chunks(channels)
        .map(|frame| frame.iter().copied().sum::<f32>() / frame.len() as f32)
        .collect()
}

fn downmix_i16(samples: &[i16], channels: usize) -> Vec<i16> {
    if channels <= 1 {
        return samples.to_vec();
    }
    samples
        .chunks(channels)
        .map(|frame| (frame.iter().map(|sample| i32::from(*sample)).sum::<i32>() / frame.len() as i32) as i16)
        .collect()
}

fn downmix_u16(samples: &[u16], channels: usize) -> Vec<u16> {
    if channels <= 1 {
        return samples.to_vec();
    }
    samples
        .chunks(channels)
        .map(|frame| (frame.iter().map(|sample| u32::from(*sample)).sum::<u32>() / frame.len() as u32) as u16)
        .collect()
}
