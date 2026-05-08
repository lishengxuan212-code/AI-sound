use crate::audio_session::SessionKind;
use crate::event_emitter::emit_session_event;
use crate::settings::{load_app_settings, tts_api_key};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;
use tauri::AppHandle;

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
    error: Option<String>,
}

pub async fn synthesize_qwen_tts(app: AppHandle, request: TtsInvokeRequest) -> Result<(), String> {
    if request.session_kind != SessionKind::MicInterpretation {
        return Err("TTS is only allowed for MicInterpretation".to_string());
    }

    let settings = load_app_settings();
    let tts_id = uuid::Uuid::new_v4().to_string();
    let session_id = "mic-tts".to_string();
    let model = "qwen-qwen-tts-latest".to_string();

    emit_status(&app, &session_id, &tts_id, &request.translation_item_id, "synthesizing", &model, None)?;

    let Some(api_key) = tts_api_key() else {
        emit_status(
            &app,
            &session_id,
            &tts_id,
            &request.translation_item_id,
            "failed",
            &model,
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

    match response {
        Ok(resp) if resp.status().is_success() => {
            emit_status(&app, &session_id, &tts_id, &request.translation_item_id, "completed", &model, None)
        }
        Ok(resp) => emit_status(
            &app,
            &session_id,
            &tts_id,
            &request.translation_item_id,
            "failed",
            &model,
            Some(&format!("Qwen TTS HTTP {}", resp.status())),
        ),
        Err(error) => emit_status(
            &app,
            &session_id,
            &tts_id,
            &request.translation_item_id,
            "failed",
            &model,
            Some(&error.to_string()),
        ),
    }
}

fn emit_status(
    app: &AppHandle,
    session_id: &str,
    tts_id: &str,
    translation_item_id: &str,
    status: &str,
    model: &str,
    error: Option<&str>,
) -> Result<(), String> {
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
            error: error.map(ToString::to_string),
        },
    )
}
