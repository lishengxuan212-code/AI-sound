use crate::audio_session::SessionKind;
use crate::diagnostics::diagnostic;
use crate::event_emitter::emit_session_event;
use crate::settings::{load_app_settings, tts_api_key};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;
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
    audio_path: PathBuf,
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

    let saved = match save_tts_audio(
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
    match play_tts_audio_path(saved.audio_path.clone()).await {
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

fn save_tts_audio(
    tts_id: &str,
    translation_item_id: &str,
    format: &str,
    sample_rate: u32,
    voice: &str,
    model: &str,
    created_at: u128,
    audio: TtsAudioBytes,
) -> Result<SavedTtsAudio, String> {
    let path = make_tts_audio_path(tts_id, format)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(&path, &audio.bytes).map_err(|error| error.to_string())?;
    Ok(SavedTtsAudio {
        tts_id: tts_id.to_string(),
        translation_item_id: translation_item_id.to_string(),
        audio_path: path,
        audio_url: audio.audio_url,
        file_size: audio.bytes.len() as u64,
        format: normalize_audio_format(format),
        sample_rate,
        voice: voice.to_string(),
        model: model.to_string(),
        created_at,
        completed_at: None,
    })
}

pub fn make_tts_audio_path(tts_id: &str, format: &str) -> Result<PathBuf, String> {
    let safe_id: String = tts_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect();
    if safe_id.is_empty() {
        return Err("TTS id 无效。".to_string());
    }
    let extension = normalize_audio_format(format);
    Ok(std::env::current_dir()
        .map_err(|error| error.to_string())?
        .join("generated-audio")
        .join("tts")
        .join(format!("tts_{}_{}.{}", now_ms(), safe_id, extension)))
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

async fn play_tts_audio_path(path: PathBuf) -> Result<(), String> {
    let guard = TTS_PLAYBACK_LOCK
        .get_or_init(|| AsyncMutex::new(()))
        .lock()
        .await;
    let result = tokio::task::spawn_blocking(move || {
        let (_stream, handle) =
            rodio::OutputStream::try_default().map_err(|error| error.to_string())?;
        let sink = Arc::new(rodio::Sink::try_new(&handle).map_err(|error| error.to_string())?);
        set_current_sink(Some(sink.clone()));
        let file = File::open(path).map_err(|error| error.to_string())?;
        let source =
            rodio::Decoder::new(BufReader::new(file)).map_err(|error| error.to_string())?;
        sink.append(source);
        sink.sleep_until_end();
        set_current_sink(None);
        Ok::<(), String>(())
    })
    .await
    .map_err(|error| error.to_string())?;
    drop(guard);
    result
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
    let audio_path = saved.audio_path.to_string_lossy().to_string();
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
        Some(&audio_path),
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
        event_name_for_tts_status, extract_audio_reference, make_tts_audio_path, TtsAudioReference,
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
    fn makes_timestamped_generated_audio_path() {
        let path = make_tts_audio_path("abc-123", "wav").expect("path");
        let text = path.to_string_lossy();
        assert!(text.contains("generated-audio"));
        assert!(text.contains("tts"));
        assert!(text.contains("tts_"));
        assert!(text.ends_with("_abc-123.wav"));
    }

    #[test]
    fn maps_playback_statuses_to_specific_events() {
        assert_eq!(event_name_for_tts_status("playing"), "mic_tts_playing");
        assert_eq!(event_name_for_tts_status("completed"), "mic_tts_completed");
        assert_eq!(event_name_for_tts_status("failed"), "mic_tts_error");
        assert_eq!(event_name_for_tts_status("audio_saved"), "mic_tts_status");
    }
}
