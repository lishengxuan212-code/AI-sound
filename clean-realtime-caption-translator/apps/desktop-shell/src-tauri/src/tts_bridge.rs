use crate::audio_session::SessionKind;
use crate::diagnostics::diagnostic;
use crate::event_emitter::emit_session_event;
use crate::settings::{load_app_settings, tts_api_key};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;
use tauri::AppHandle;
use tokio::sync::Mutex;

static TTS_PLAYBACK_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug, Deserialize)]
pub struct TtsInvokeRequest {
    #[serde(rename = "sessionKind")]
    pub session_kind: SessionKind,
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
    #[serde(rename = "audioPath")]
    audio_path: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RetryTtsPlaybackRequest {
    #[serde(rename = "ttsId")]
    pub tts_id: String,
    #[serde(rename = "translationItemId")]
    pub translation_item_id: String,
    #[serde(rename = "audioPath")]
    pub audio_path: String,
}

#[derive(Debug, PartialEq)]
pub enum TtsAudioReference {
    Url(String),
    Base64(String),
}

pub async fn synthesize_qwen_tts(app: AppHandle, request: TtsInvokeRequest) -> Result<(), String> {
    if request.session_kind != SessionKind::MicInterpretation {
        return Err("TTS is only allowed for MicInterpretation".to_string());
    }

    let settings = load_app_settings();
    let tts_id = uuid::Uuid::new_v4().to_string();
    let session_id = "mic-tts".to_string();
    let model = "qwen-qwen-tts-latest".to_string();

    emit_status(&app, &session_id, &tts_id, &request.translation_item_id, "synthesizing", &model, None, None)?;
    diagnostic(
        "[MIC][TTS]",
        json!({
            "sessionKind": "MicInterpretation",
            "ttsId": tts_id,
            "ttsModel": model,
            "ttsStatus": "synthesizing",
            "textLength": request.text.chars().count()
        }),
    );

    let Some(api_key) = tts_api_key() else {
        emit_status(
            &app,
            &session_id,
            &tts_id,
            &request.translation_item_id,
            "failed",
            &model,
            None,
            Some("Qwen TTS API key missing"),
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
            "voice": settings.qwen_tts.voice,
            "language_type": language_type
        },
        "parameters": {
            "format": settings.qwen_tts.format,
            "sample_rate": settings.qwen_tts.sample_rate,
            "speed": settings.qwen_tts.speed,
            "volume": settings.qwen_tts.volume
        }
    });

    let client = reqwest::Client::new();
    let response = client
        .post(settings.qwen_tts.endpoint)
        .bearer_auth(api_key)
        .timeout(Duration::from_secs(30))
        .json(&body)
        .send()
        .await;

    let audio_path = match response {
        Ok(resp) if resp.status().is_success() => {
            match read_tts_audio_bytes(&client, resp).await.and_then(|bytes| save_tts_audio(&tts_id, &settings.qwen_tts.format, &bytes)) {
                Ok(path) => path,
                Err(error) => {
                    emit_status(&app, &session_id, &tts_id, &request.translation_item_id, "failed", &model, None, Some(&error))?;
                    return Ok(());
                }
            }
        }
        Ok(resp) => {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let message = if body.trim().is_empty() {
                format!("Qwen TTS HTTP {status}")
            } else {
                format!("Qwen TTS HTTP {status}: {}", body.trim())
            };
            emit_status(&app, &session_id, &tts_id, &request.translation_item_id, "failed", &model, None, Some(&message))?;
            return Ok(());
        }
        Err(error) => {
            emit_status(&app, &session_id, &tts_id, &request.translation_item_id, "failed", &model, None, Some(&error.to_string()))?;
            return Ok(());
        }
    };

    let audio_path_string = audio_path.to_string_lossy().to_string();
    emit_status(
        &app,
        &session_id,
        &tts_id,
        &request.translation_item_id,
        "playing",
        &model,
        Some(&audio_path_string),
        None,
    )?;

    match play_tts_audio_path(audio_path.clone()).await {
        Ok(()) => emit_status(
            &app,
            &session_id,
            &tts_id,
            &request.translation_item_id,
            "completed",
            &model,
            Some(&audio_path_string),
            None,
        ),
        Err(error) => emit_status(
            &app,
            &session_id,
            &tts_id,
            &request.translation_item_id,
            "failed",
            &model,
            Some(&audio_path_string),
            Some(&error),
        ),
    }
}

pub async fn retry_tts_playback(app: AppHandle, request: RetryTtsPlaybackRequest) -> Result<(), String> {
    let session_id = "mic-tts".to_string();
    let model = "qwen-qwen-tts-latest".to_string();
    emit_status(
        &app,
        &session_id,
        &request.tts_id,
        &request.translation_item_id,
        "playing",
        &model,
        Some(&request.audio_path),
        None,
    )?;

    match play_tts_audio_path(PathBuf::from(&request.audio_path)).await {
        Ok(()) => emit_status(
            &app,
            &session_id,
            &request.tts_id,
            &request.translation_item_id,
            "completed",
            &model,
            Some(&request.audio_path),
            None,
        ),
        Err(error) => emit_status(
            &app,
            &session_id,
            &request.tts_id,
            &request.translation_item_id,
            "failed",
            &model,
            Some(&request.audio_path),
            Some(&error),
        ),
    }
}

async fn read_tts_audio_bytes(client: &reqwest::Client, response: reqwest::Response) -> Result<Vec<u8>, String> {
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = response.bytes().await.map_err(|error| error.to_string())?.to_vec();

    if content_type.contains("application/json") || bytes.first() == Some(&b'{') {
        let value: Value = serde_json::from_slice(&bytes).map_err(|error| format!("Qwen TTS JSON parse failed: {error}"))?;
        match extract_audio_reference(&value) {
            Some(TtsAudioReference::Url(url)) => {
                let audio_response = client.get(url).timeout(Duration::from_secs(30)).send().await.map_err(|error| error.to_string())?;
                if !audio_response.status().is_success() {
                    return Err(format!("Qwen TTS audio download failed: HTTP {}", audio_response.status()));
                }
                audio_response.bytes().await.map(|body| body.to_vec()).map_err(|error| error.to_string())
            }
            Some(TtsAudioReference::Base64(data)) => base64::engine::general_purpose::STANDARD
                .decode(data)
                .map_err(|error| format!("Qwen TTS base64 audio decode failed: {error}")),
            None => Err("Qwen TTS response did not contain audio data".to_string()),
        }
    } else {
        Ok(bytes)
    }
}

pub fn extract_audio_reference(value: &Value) -> Option<TtsAudioReference> {
    let candidates = [
        value.pointer("/output/audio/url"),
        value.pointer("/output/audio_url"),
        value.pointer("/audio/url"),
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
        value.pointer("/audio/data"),
        value.pointer("/data"),
    ];
    for candidate in data_candidates.into_iter().flatten() {
        if let Some(data) = candidate.as_str().filter(|item| !item.trim().is_empty()) {
            return Some(TtsAudioReference::Base64(data.to_string()));
        }
    }
    None
}

fn save_tts_audio(tts_id: &str, format: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    let path = make_tts_audio_path(tts_id, format)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(&path, bytes).map_err(|error| error.to_string())?;
    Ok(path)
}

pub fn make_tts_audio_path(tts_id: &str, format: &str) -> Result<PathBuf, String> {
    let safe_id: String = tts_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect();
    if safe_id.is_empty() {
        return Err("Invalid TTS id".to_string());
    }
    let normalized_format = format.trim().trim_start_matches('.').to_ascii_lowercase();
    let extension = match normalized_format.as_str() {
        "mp3" => "mp3".to_string(),
        "wav" | "" => "wav".to_string(),
        other => other.to_string(),
    };
    Ok(std::env::current_dir()
        .map_err(|error| error.to_string())?
        .join("tts-audio")
        .join(format!("{safe_id}.{extension}")))
}

async fn play_tts_audio_path(path: PathBuf) -> Result<(), String> {
    let guard = TTS_PLAYBACK_LOCK.get_or_init(|| Mutex::new(())).lock().await;
    let result = tokio::task::spawn_blocking(move || {
        let (_stream, handle) = rodio::OutputStream::try_default().map_err(|error| error.to_string())?;
        let sink = rodio::Sink::try_new(&handle).map_err(|error| error.to_string())?;
        let file = File::open(path).map_err(|error| error.to_string())?;
        let source = rodio::Decoder::new(BufReader::new(file)).map_err(|error| error.to_string())?;
        sink.append(source);
        sink.sleep_until_end();
        Ok::<(), String>(())
    })
    .await
    .map_err(|error| error.to_string())?;
    drop(guard);
    result
}

fn emit_status(
    app: &AppHandle,
    session_id: &str,
    tts_id: &str,
    translation_item_id: &str,
    status: &str,
    model: &str,
    audio_path: Option<&str>,
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
            "ttsStatus": status,
            "audioPath": audio_path,
            "error": error
        }),
    );
    emit_session_event(
        app,
        "mic_tts_status",
        SessionKind::MicInterpretation,
        session_id,
        TtsStatusPayload {
            tts_id: tts_id.to_string(),
            translation_item_id: translation_item_id.to_string(),
            status: status.to_string(),
            model: model.to_string(),
            audio_path: audio_path.map(ToString::to_string),
            error: error.map(ToString::to_string),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::{extract_audio_reference, make_tts_audio_path, TtsAudioReference};

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
            Some(TtsAudioReference::Url("https://example.local/audio.wav".to_string()))
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
    fn makes_safe_tts_audio_path() {
        let path = make_tts_audio_path("abc-123", "wav").expect("path");
        assert!(path.ends_with("tts-audio\\abc-123.wav") || path.ends_with("tts-audio/abc-123.wav"));
    }
}
