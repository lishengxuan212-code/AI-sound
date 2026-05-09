use crate::audio_session::SessionKind;
use crate::diagnostics::diagnostic;
use crate::event_emitter::emit_session_event;
use crate::settings::{load_app_settings, tts_api_key};
use base64::Engine;
use cpal::traits::{DeviceTrait, HostTrait};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufReader, Cursor};
use std::sync::{Arc, Mutex as StdMutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::AppHandle;
use tokio::sync::Mutex as AsyncMutex;

static TTS_PLAYBACK_LOCK: OnceLock<AsyncMutex<()>> = OnceLock::new();
static TTS_AUDIO_REGISTRY: OnceLock<StdMutex<HashMap<String, SavedTtsAudio>>> = OnceLock::new();
static CURRENT_TTS_SINK: OnceLock<StdMutex<Option<Arc<rodio::Sink>>>> = OnceLock::new();

#[derive(Debug, Deserialize)]
pub struct TtsInvokeRequest {
    #[serde(rename = "sessionKind")]
    pub session_kind: SessionKind,
    #[serde(rename = "ttsId")]
    pub tts_id: String,
    #[serde(rename = "translationItemId")]
    pub translation_item_id: String,
    pub text: String,
    #[serde(rename = "targetLang")]
    pub target_lang: String,
}

#[derive(Clone, Debug, Serialize)]
struct TtsStatusPayload {
    #[serde(rename = "ttsId")]
    tts_id: String,
    #[serde(rename = "translationItemId")]
    translation_item_id: String,
    status: String,
    model: String,
    voice: String,
    format: String,
    #[serde(rename = "sampleRate")]
    sample_rate: u32,
    #[serde(rename = "audioPath")]
    audio_path: Option<String>,
    #[serde(rename = "audioUrl")]
    audio_url: Option<String>,
    #[serde(rename = "fileSize")]
    file_size: Option<u64>,
    #[serde(rename = "createdAt")]
    created_at: u128,
    #[serde(rename = "completedAt")]
    completed_at: Option<u128>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RetryTtsPlaybackRequest {
    #[serde(rename = "ttsId")]
    pub tts_id: String,
}

#[derive(Debug, Deserialize)]
pub struct StopTtsPlaybackRequest {
    #[serde(rename = "ttsId")]
    pub tts_id: Option<String>,
}

#[derive(Clone, Debug)]
struct SavedTtsAudio {
    tts_id: String,
    translation_item_id: String,
    bytes: Vec<u8>,
    audio_path: Option<String>,
    audio_url: Option<String>,
    file_size: u64,
    format: String,
    sample_rate: u32,
    voice: String,
    model: String,
    created_at: u128,
    completed_at: Option<u128>,
}

#[derive(Debug, PartialEq)]
pub enum TtsAudioReference {
    Url(String),
    Base64(String),
}

struct TtsAudioBytes {
    bytes: Vec<u8>,
    audio_url: Option<String>,
}

pub async fn synthesize_qwen_tts(app: AppHandle, request: TtsInvokeRequest) -> Result<(), String> {
    if request.session_kind != SessionKind::MicInterpretation {
        return Err("TTS 只能用于麦克风同声传译模块。".to_string());
    }

    let settings = load_app_settings();
    let session_id = "mic-tts".to_string();
    let model = settings.qwen_tts.model.clone();
    let voice = settings.qwen_tts.voice.clone();
    let format = normalize_audio_format(&settings.qwen_tts.format);
    let sample_rate = settings.qwen_tts.sample_rate;
    let created_at = now_ms();

    emit_status(
        &app,
        &session_id,
        &request.tts_id,
        &request.translation_item_id,
        "synthesizing",
        &model,
        &voice,
        &format,
        sample_rate,
        None,
        None,
        None,
        created_at,
        None,
        None,
    )?;

    if settings.qwen_tts.endpoint.trim().is_empty() {
        emit_failed(
            &app,
            &session_id,
            &request,
            &model,
            &voice,
            &format,
            sample_rate,
            created_at,
            "TTS API 地址未填写。",
        )?;
        return Ok(());
    }

    if model.trim().is_empty() {
        emit_failed(
            &app,
            &session_id,
            &request,
            &model,
            &voice,
            &format,
            sample_rate,
            created_at,
            "TTS 模型名称未填写。",
        )?;
        return Ok(());
    }

    let Some(api_key) = tts_api_key() else {
        emit_status(
            &app,
            &session_id,
            &request.tts_id,
            &request.translation_item_id,
            "failed",
            &model,
            &voice,
            &format,
            sample_rate,
            None,
            None,
            None,
            created_at,
            Some(now_ms()),
            Some("TTS 密钥未配置。"),
        )?;
        return Ok(());
    };

    let language_type = match request.target_lang.as_str() {
        "zh" | "zh-CN" => "Chinese",
        "en" | "en-US" => "English",
        _ => "Auto",
    };

    let body = json!({
        "model": model,
        "input": {
            "text": request.text,
            "voice": voice,
            "language_type": language_type
        },
        "parameters": {
            "format": format,
            "sample_rate": sample_rate,
            "speed": settings.qwen_tts.speed,
            "volume": settings.qwen_tts.volume
        }
    });

    let client = reqwest::Client::new();
    let response = client
        .post(settings.qwen_tts.endpoint.trim())
        .bearer_auth(api_key)
        .timeout(Duration::from_secs(30))
        .json(&body)
        .send()
        .await;

    let audio = match response {
        Ok(resp) if resp.status().is_success() => match read_tts_audio_bytes(&client, resp).await {
            Ok(audio) => audio,
            Err(error) => {
                emit_failed(
                    &app,
                    &session_id,
                    &request,
                    &model,
                    &voice,
                    &format,
                    sample_rate,
                    created_at,
                    &error,
                )?;
                return Ok(());
            }
        },
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let message = if body.trim().is_empty() {
                format!("Qwen TTS 调用失败：HTTP {status}")
            } else {
                format!("Qwen TTS 调用失败：HTTP {status}: {}", body.trim())
            };
            emit_failed(
                &app,
                &session_id,
                &request,
                &model,
                &voice,
                &format,
                sample_rate,
                created_at,
                &message,
            )?;
            return Ok(());
        }
        Err(error) => {
            emit_failed(
                &app,
                &session_id,
                &request,
                &model,
                &voice,
                &format,
                sample_rate,
                created_at,
                &error.to_string(),
            )?;
            return Ok(());
        }
    };

    let saved = match prepare_tts_audio(
        &request.tts_id,
        &request.translation_item_id,
        &format,
        sample_rate,
        &voice,
        &model,
        created_at,
        audio,
    ) {
        Ok(saved) => saved,
        Err(error) => {
            emit_failed(
                &app,
                &session_id,
                &request,
                &model,
                &voice,
                &format,
                sample_rate,
                created_at,
                &error,
            )?;
            return Ok(());
        }
    };

    register_saved_audio(saved.clone());
    emit_saved_status(&app, &session_id, "audio_saved", &saved, None)?;
    play_saved_tts_audio(app, session_id, saved).await
}

pub async fn retry_tts_playback(
    app: AppHandle,
    request: RetryTtsPlaybackRequest,
) -> Result<(), String> {
    let saved = find_saved_audio(&request.tts_id)
        .ok_or_else(|| format!("当前没有可重播的 TTS 音频：{}", request.tts_id))?;
    play_saved_tts_audio(app, "mic-tts".to_string(), saved).await
}

pub async fn stop_tts_playback(
    app: AppHandle,
    request: StopTtsPlaybackRequest,
) -> Result<(), String> {
    let stopped = stop_current_sink();
    let tts_id = request.tts_id.unwrap_or_else(|| "current".to_string());
    diagnostic(
        "[MIC][TTS]",
        json!({
            "sessionKind": "MicInterpretation",
            "ttsId": tts_id,
            "ttsStatus": if stopped { "stopped" } else { "idle" }
        }),
    );
    if let Some(saved) = find_saved_audio(&tts_id) {
        emit_saved_status(
            &app,
            "mic-tts",
            if stopped { "failed" } else { "completed" },
            &saved,
            Some("TTS 音频播放已停止。"),
        )?;
    }
    Ok(())
}

async fn play_saved_tts_audio(
    app: AppHandle,
    session_id: String,
    saved: SavedTtsAudio,
) -> Result<(), String> {
    emit_saved_status(&app, &session_id, "playing", &saved, None)?;
    match play_tts_audio_bytes(saved.bytes.clone()).await {
        Ok(()) => {
            let mut completed = saved.clone();
            completed.completed_at = Some(now_ms());
            register_saved_audio(completed.clone());
            emit_saved_status(&app, &session_id, "completed", &completed, None)
        }
        Err(error) => emit_saved_status(&app, &session_id, "failed", &saved, Some(&error)),
    }
}

fn emit_failed(
    app: &AppHandle,
    session_id: &str,
    request: &TtsInvokeRequest,
    model: &str,
    voice: &str,
    format: &str,
    sample_rate: u32,
    created_at: u128,
    error: &str,
) -> Result<(), String> {
    emit_status(
        app,
        session_id,
        &request.tts_id,
        &request.translation_item_id,
        "failed",
        model,
        voice,
        format,
        sample_rate,
        None,
        None,
        None,
        created_at,
        Some(now_ms()),
        Some(error),
    )
}

async fn read_tts_audio_bytes(
    client: &reqwest::Client,
    response: reqwest::Response,
) -> Result<TtsAudioBytes, String> {
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = response
        .bytes()
        .await
        .map_err(|error| error.to_string())?
        .to_vec();

    if content_type.contains("application/json") || bytes.first() == Some(&b'{') {
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|error| format!("Qwen TTS 返回格式无效：{error}"))?;
        match extract_audio_reference(&value) {
            Some(TtsAudioReference::Url(url)) => {
                let audio_response = client
                    .get(url.clone())
                    .timeout(Duration::from_secs(30))
                    .send()
                    .await
                    .map_err(|error| error.to_string())?;
                if !audio_response.status().is_success() {
                    return Err(format!(
                        "Qwen TTS 音频下载失败：HTTP {}",
                        audio_response.status()
                    ));
                }
                let bytes = audio_response
                    .bytes()
                    .await
                    .map(|body| body.to_vec())
                    .map_err(|error| error.to_string())?;
                Ok(TtsAudioBytes {
                    bytes,
                    audio_url: Some(url),
                })
            }
            Some(TtsAudioReference::Base64(data)) => {
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(data)
                    .map_err(|error| format!("Qwen TTS base64 音频解码失败：{error}"))?;
                Ok(TtsAudioBytes {
                    bytes,
                    audio_url: None,
                })
            }
            None => Err("TTS 返回音频为空。".to_string()),
        }
    } else {
        Ok(TtsAudioBytes {
            bytes,
            audio_url: None,
        })
    }
}

pub fn extract_audio_reference(value: &Value) -> Option<TtsAudioReference> {
    let candidates = [
        value.pointer("/output/audio/url"),
        value.pointer("/output/audio_url"),
        value.pointer("/output/url"),
        value.pointer("/audio/url"),
        value.pointer("/audio_url"),
        value.pointer("/url"),
    ];
    for candidate in candidates.into_iter().flatten() {
        if let Some(url) = candidate.as_str().filter(|item| !item.trim().is_empty()) {
            return Some(TtsAudioReference::Url(url.to_string()));
        }
    }

    let data_candidates = [
        value.pointer("/output/audio/data"),
        value.pointer("/output/audio/base64"),
        value.pointer("/output/audio"),
        value.pointer("/audio/data"),
        value.pointer("/audio/base64"),
        value.pointer("/data"),
    ];
    for candidate in data_candidates.into_iter().flatten() {
        if let Some(data) = candidate.as_str().filter(|item| !item.trim().is_empty()) {
            return Some(TtsAudioReference::Base64(data.to_string()));
        }
    }
    None
}

fn prepare_tts_audio(
    tts_id: &str,
    translation_item_id: &str,
    format: &str,
    sample_rate: u32,
    voice: &str,
    model: &str,
    created_at: u128,
    audio: TtsAudioBytes,
) -> Result<SavedTtsAudio, String> {
    if tts_id.is_empty()
        || tts_id
            .chars()
            .any(|ch| !(ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'))
    {
        return Err("TTS id 鏃犳晥銆?".to_string());
    }
    let file_size = audio.bytes.len() as u64;
    Ok(SavedTtsAudio {
        tts_id: tts_id.to_string(),
        translation_item_id: translation_item_id.to_string(),
        bytes: audio.bytes,
        audio_path: None,
        audio_url: audio.audio_url,
        file_size,
        format: normalize_audio_format(format),
        sample_rate,
        voice: voice.to_string(),
        model: model.to_string(),
        created_at,
        completed_at: None,
    })
}

#[allow(dead_code)]
pub fn make_tts_audio_path(tts_id: &str, format: &str) -> Result<String, String> {
    let safe_id: String = tts_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect();
    if safe_id.is_empty() {
        return Err("TTS id 无效。".to_string());
    }
    let extension = normalize_audio_format(format);
    Ok(format!("tts_{}_{}.{}", now_ms(), safe_id, extension))
}

fn normalize_audio_format(format: &str) -> String {
    match format
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase()
        .as_str()
    {
        "mp3" => "mp3".to_string(),
        "wav" | "" => "wav".to_string(),
        other => other.to_string(),
    }
}

async fn play_tts_audio_bytes(bytes: Vec<u8>) -> Result<(), String> {
    let settings = load_app_settings();
    let route = TtsPlaybackRoute::from_settings(
        &settings.qwen_tts.output_mode,
        &settings.qwen_tts.virtual_mic_device_name,
    );
    let guard = TTS_PLAYBACK_LOCK
        .get_or_init(|| AsyncMutex::new(()))
        .lock()
        .await;
    let result = tokio::task::spawn_blocking(move || play_tts_audio_bytes_blocking(bytes, route))
        .await
        .map_err(|error| error.to_string())?;
    drop(guard);
    result
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TtsPlaybackRoute {
    PreviewOnly,
    VirtualMicOnly { device_name: String },
    PreviewAndVirtualMic { device_name: String },
}

impl TtsPlaybackRoute {
    fn from_settings(output_mode: &str, virtual_mic_device_name: &str) -> Self {
        let device_name = virtual_mic_device_name.trim().to_string();
        match output_mode.trim() {
            "preview_only" => Self::PreviewOnly,
            "preview_and_virtual_mic" => Self::PreviewAndVirtualMic { device_name },
            _ => Self::VirtualMicOnly { device_name },
        }
    }
}

fn play_tts_audio_bytes_blocking(bytes: Vec<u8>, route: TtsPlaybackRoute) -> Result<(), String> {
    match route {
        TtsPlaybackRoute::PreviewOnly => play_tts_audio_once(bytes, None),
        TtsPlaybackRoute::VirtualMicOnly { device_name } => {
            play_tts_audio_once(bytes, Some(device_name.as_str()))
        }
        TtsPlaybackRoute::PreviewAndVirtualMic { device_name } => {
            let preview_bytes = bytes.clone();
            let preview = std::thread::spawn(move || play_tts_audio_once(preview_bytes, None));
            let virtual_result = play_tts_audio_once(bytes, Some(device_name.as_str()));
            let preview_result = preview
                .join()
                .map_err(|_| "TTS 预览播放线程异常。".to_string())?;
            virtual_result?;
            preview_result
        }
    }
}

fn play_tts_audio_once(bytes: Vec<u8>, output_device_name: Option<&str>) -> Result<(), String> {
    let (_stream, handle) = match output_device_name {
        Some(name) if !name.trim().is_empty() => {
            let device = find_output_device_by_name(name)
                .ok_or_else(|| format!("TTS 虚拟麦输出设备未找到：{name}"))?;
            rodio::OutputStream::try_from_device(&device).map_err(|error| error.to_string())?
        }
        Some(_) => return Err("TTS 虚拟麦输出设备未配置。".to_string()),
        None => rodio::OutputStream::try_default().map_err(|error| error.to_string())?,
    };
    let sink = Arc::new(rodio::Sink::try_new(&handle).map_err(|error| error.to_string())?);
    set_current_sink(Some(sink.clone()));
    let source = rodio::Decoder::new(BufReader::new(Cursor::new(bytes)))
        .map_err(|error| error.to_string())?;
    sink.append(source);
    sink.sleep_until_end();
    set_current_sink(None);
    Ok(())
}

fn find_output_device_by_name(configured_name: &str) -> Option<cpal::Device> {
    let configured = configured_name.trim();
    if configured.is_empty() {
        return None;
    }
    let host = cpal::default_host();
    host.output_devices().ok()?.find(|device| {
        device
            .name()
            .ok()
            .is_some_and(|name| name == configured || name.contains(configured))
    })
}

fn register_saved_audio(saved: SavedTtsAudio) {
    let mut registry = TTS_AUDIO_REGISTRY
        .get_or_init(|| StdMutex::new(HashMap::new()))
        .lock()
        .expect("tts registry lock");
    registry.insert(saved.tts_id.clone(), saved);
}

fn find_saved_audio(tts_id: &str) -> Option<SavedTtsAudio> {
    let registry = TTS_AUDIO_REGISTRY
        .get_or_init(|| StdMutex::new(HashMap::new()))
        .lock()
        .ok()?;
    registry.get(tts_id).cloned()
}

fn set_current_sink(sink: Option<Arc<rodio::Sink>>) {
    if let Ok(mut current) = CURRENT_TTS_SINK.get_or_init(|| StdMutex::new(None)).lock() {
        *current = sink;
    }
}

fn stop_current_sink() -> bool {
    let Ok(mut current) = CURRENT_TTS_SINK.get_or_init(|| StdMutex::new(None)).lock() else {
        return false;
    };
    if let Some(sink) = current.take() {
        sink.stop();
        true
    } else {
        false
    }
}

fn emit_saved_status(
    app: &AppHandle,
    session_id: &str,
    status: &str,
    saved: &SavedTtsAudio,
    error: Option<&str>,
) -> Result<(), String> {
    emit_status(
        app,
        session_id,
        &saved.tts_id,
        &saved.translation_item_id,
        status,
        &saved.model,
        &saved.voice,
        &saved.format,
        saved.sample_rate,
        saved.audio_path.as_deref(),
        saved.audio_url.as_deref(),
        Some(saved.file_size),
        saved.created_at,
        saved.completed_at,
        error,
    )
}

fn emit_status(
    app: &AppHandle,
    session_id: &str,
    tts_id: &str,
    translation_item_id: &str,
    status: &str,
    model: &str,
    voice: &str,
    format: &str,
    sample_rate: u32,
    audio_path: Option<&str>,
    audio_url: Option<&str>,
    file_size: Option<u64>,
    created_at: u128,
    completed_at: Option<u128>,
    error: Option<&str>,
) -> Result<(), String> {
    diagnostic(
        "[MIC][TTS]",
        json!({
            "sessionKind": "MicInterpretation",
            "sessionId": session_id,
            "ttsId": tts_id,
            "translationItemId": translation_item_id,
            "ttsModel": model,
            "voice": voice,
            "format": format,
            "sampleRate": sample_rate,
            "ttsStatus": status,
            "audioPath": audio_path,
            "audioUrl": audio_url,
            "fileSize": file_size,
            "error": error
        }),
    );
    emit_session_event(
        app,
        event_name_for_tts_status(status),
        SessionKind::MicInterpretation,
        session_id,
        TtsStatusPayload {
            tts_id: tts_id.to_string(),
            translation_item_id: translation_item_id.to_string(),
            status: status.to_string(),
            model: model.to_string(),
            voice: voice.to_string(),
            format: format.to_string(),
            sample_rate,
            audio_path: audio_path.map(ToString::to_string),
            audio_url: audio_url.map(ToString::to_string),
            file_size,
            created_at,
            completed_at,
            error: error.map(ToString::to_string),
        },
    )
}

pub fn event_name_for_tts_status(status: &str) -> &'static str {
    match status {
        "playing" => "mic_tts_playing",
        "completed" => "mic_tts_completed",
        "failed" => "mic_tts_error",
        _ => "mic_tts_status",
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{
        event_name_for_tts_status, extract_audio_reference, prepare_tts_audio, TtsAudioBytes,
        TtsAudioReference, TtsPlaybackRoute,
    };

    #[test]
    fn extracts_dashscope_audio_url() {
        let value = serde_json::json!({
            "output": {
                "audio": {
                    "url": "https://example.local/audio.wav"
                }
            }
        });

        assert_eq!(
            extract_audio_reference(&value),
            Some(TtsAudioReference::Url(
                "https://example.local/audio.wav".to_string()
            ))
        );
    }

    #[test]
    fn extracts_base64_audio_payload() {
        let value = serde_json::json!({
            "output": {
                "audio": {
                    "data": "UklGRg=="
                }
            }
        });

        assert_eq!(
            extract_audio_reference(&value),
            Some(TtsAudioReference::Base64("UklGRg==".to_string()))
        );
    }

    #[test]
    fn prepares_tts_audio_without_local_audio_path() {
        let saved = prepare_tts_audio(
            "abc-123",
            "translation-1",
            "mp3",
            24_000,
            "Cherry",
            "qwen3-tts-flash",
            123,
            TtsAudioBytes {
                bytes: vec![1, 2, 3],
                audio_url: None,
            },
        )
        .expect("saved audio");

        assert_eq!(saved.file_size, 3);
        assert_eq!(saved.bytes, vec![1, 2, 3]);
        assert!(saved.audio_path.is_none());
    }

    #[test]
    fn defaults_tts_playback_to_virtual_mic_route() {
        assert_eq!(
            TtsPlaybackRoute::from_settings(
                "virtual_mic_only",
                "CABLE Input (VB-Audio Virtual Cable)"
            ),
            TtsPlaybackRoute::VirtualMicOnly {
                device_name: "CABLE Input (VB-Audio Virtual Cable)".to_string()
            }
        );
    }

    #[test]
    fn maps_playback_statuses_to_specific_events() {
        assert_eq!(event_name_for_tts_status("playing"), "mic_tts_playing");
        assert_eq!(event_name_for_tts_status("completed"), "mic_tts_completed");
        assert_eq!(event_name_for_tts_status("failed"), "mic_tts_error");
        assert_eq!(event_name_for_tts_status("audio_saved"), "mic_tts_status");
    }
}
