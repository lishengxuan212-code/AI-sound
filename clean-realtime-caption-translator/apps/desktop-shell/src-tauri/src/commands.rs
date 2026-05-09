use crate::audio_capture::{list_input_devices, list_output_devices, AudioDeviceSummary};
use crate::audio_session::{SessionKind, SessionRegistry};
use crate::caption_state::{
    get_realtime_caption_snapshot as read_caption_snapshot, RealtimeCaptionSnapshot,
};
use crate::event_emitter::{drain_session_events as read_session_events, StoredSessionEvent};
use crate::local_translation_bridge::is_dashscope_provider;
use crate::mic_interpretation_session::start_mic_interpretation_session;
use crate::service_manager::{
    ensure_local_services_ready as ensure_services_ready, probe_local_services, LocalServicesStatus,
};
use crate::settings::{
    get_secret_status as read_secret_status, load_app_settings, reset_app_settings_file,
    save_app_settings_to_file, AppSettings, ConnectionTestResult, SecretStatus,
};
use crate::system_subtitle_session::start_system_subtitle_session;
use crate::tts_bridge::{
    retry_tts_playback, stop_tts_playback, synthesize_qwen_tts, RetryTtsPlaybackRequest,
    StopTtsPlaybackRequest, TtsInvokeRequest,
};
use std::time::Duration;
use tauri::{AppHandle, State};

#[derive(Debug, serde::Serialize)]
pub struct AudioDevicesResult {
    #[serde(rename = "inputDevices")]
    pub input_devices: Vec<AudioDeviceSummary>,
    #[serde(rename = "outputDevices")]
    pub output_devices: Vec<AudioDeviceSummary>,
}

#[derive(Debug, serde::Serialize)]
pub struct TtsVoiceOption {
    pub value: String,
    #[serde(rename = "labelZh")]
    pub label_zh: String,
    #[serde(rename = "languageHint")]
    pub language_hint: String,
}

#[tauri::command(rename_all = "camelCase")]
pub async fn load_settings() -> Result<AppSettings, String> {
    Ok(load_app_settings())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_app_settings() -> Result<AppSettings, String> {
    Ok(load_app_settings())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn save_app_settings(settings: AppSettings) -> Result<AppSettings, String> {
    save_app_settings_to_file(settings)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn reset_app_settings() -> Result<AppSettings, String> {
    reset_app_settings_file()
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_secret_status() -> Result<SecretStatus, String> {
    Ok(read_secret_status())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn list_audio_devices() -> Result<AudioDevicesResult, String> {
    Ok(AudioDevicesResult {
        input_devices: list_input_devices(),
        output_devices: list_output_devices(),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub async fn drain_session_events(after_sequence: u64) -> Result<Vec<StoredSessionEvent>, String> {
    Ok(read_session_events(after_sequence))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn get_realtime_caption_snapshot() -> Result<RealtimeCaptionSnapshot, String> {
    Ok(read_caption_snapshot())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn list_tts_voices() -> Result<Vec<TtsVoiceOption>, String> {
    Ok(vec![
        TtsVoiceOption {
            value: "Cherry".to_string(),
            label_zh: "Cherry - 女声，中文自然".to_string(),
            language_hint: "zh".to_string(),
        },
        TtsVoiceOption {
            value: "Serena".to_string(),
            label_zh: "Serena - 女声，中英双语".to_string(),
            language_hint: "zh/en".to_string(),
        },
        TtsVoiceOption {
            value: "Ethan".to_string(),
            label_zh: "Ethan - 男声，中英双语".to_string(),
            language_hint: "zh/en".to_string(),
        },
        TtsVoiceOption {
            value: "Chelsie".to_string(),
            label_zh: "Chelsie - 女声，英文".to_string(),
            language_hint: "en".to_string(),
        },
    ])
}

#[tauri::command(rename_all = "camelCase")]
pub async fn test_local_asr_connection() -> Result<ConnectionTestResult, String> {
    let status = probe_local_services().await;
    Ok(ConnectionTestResult {
        ok: status.asr_ok,
        message: status.message,
    })
}

#[tauri::command(rename_all = "camelCase")]
pub async fn test_local_translation_connection() -> Result<ConnectionTestResult, String> {
    let settings = load_app_settings();
    if is_dashscope_provider(&settings.local_translation.provider) {
        if settings.local_translation.endpoint.trim().is_empty() {
            return Ok(ConnectionTestResult {
                ok: false,
                message: "百炼 API 地址未配置。".to_string(),
            });
        }
        if settings.local_translation.api_key.trim().is_empty() {
            return Ok(ConnectionTestResult {
                ok: false,
                message: "百炼 API 地址已配置，请填写 API Key 后再测试。".to_string(),
            });
        }
        if settings.local_translation.model_name.trim().is_empty() {
            return Ok(ConnectionTestResult {
                ok: false,
                message: "百炼 API 地址已配置，请填写模型名称后再测试。".to_string(),
            });
        }
        return Ok(ConnectionTestResult {
            ok: true,
            message: "百炼 API 配置项已填写，实时翻译将直接调用 API。".to_string(),
        });
    }

    if settings.local_translation.endpoint.trim().is_empty() {
        return Ok(ConnectionTestResult {
            ok: false,
            message: "endpoint 未配置。".to_string(),
        });
    }
    if settings.local_translation.model_path.trim().is_empty()
        && settings.local_translation.model_name.trim().is_empty()
    {
        return Ok(ConnectionTestResult {
            ok: false,
            message: "本地翻译模型未配置或未安装。".to_string(),
        });
    }
    let endpoint = settings
        .local_translation
        .endpoint
        .trim()
        .trim_end_matches('/');
    let health_url = format!("{endpoint}/health");
    let client = reqwest::Client::new();
    match client
        .get(health_url)
        .timeout(Duration::from_secs(3))
        .send()
        .await
    {
        Ok(response) if response.status().is_success() => Ok(ConnectionTestResult {
            ok: true,
            message: "连接成功。".to_string(),
        }),
        Ok(_) | Err(_) => Ok(ConnectionTestResult {
            ok: false,
            message: "本地翻译服务未启动。".to_string(),
        }),
    }
}

#[tauri::command(rename_all = "camelCase")]
pub async fn probe_local_service_status() -> Result<LocalServicesStatus, String> {
    Ok(probe_local_services().await)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn ensure_local_services_ready() -> Result<LocalServicesStatus, String> {
    ensure_services_ready().await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn test_tts_connection() -> Result<ConnectionTestResult, String> {
    let settings = load_app_settings();
    if settings.qwen_tts.endpoint.trim().is_empty() {
        return Ok(ConnectionTestResult {
            ok: false,
            message: "TTS API 地址未填写。".to_string(),
        });
    }
    if settings.qwen_tts.model.trim().is_empty() {
        return Ok(ConnectionTestResult {
            ok: false,
            message: "TTS 模型名称未填写。".to_string(),
        });
    }
    if !settings.qwen_tts.has_api_key {
        return Ok(ConnectionTestResult {
            ok: false,
            message: "TTS 密钥未配置。".to_string(),
        });
    }
    Ok(ConnectionTestResult {
        ok: true,
        message: "连接成功。".to_string(),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub async fn start_session(
    app: AppHandle,
    registry: State<'_, SessionRegistry>,
    session_kind: SessionKind,
) -> Result<(), String> {
    match session_kind {
        SessionKind::SystemSubtitle => start_system_subtitle_session(app, registry).await,
        SessionKind::MicInterpretation => start_mic_interpretation_session(app, registry).await,
    }
}

#[tauri::command(rename_all = "camelCase")]
pub async fn stop_session(
    registry: State<'_, SessionRegistry>,
    session_kind: SessionKind,
) -> Result<(), String> {
    registry.stop(session_kind).await;
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
pub async fn synthesize_tts(app: AppHandle, request: TtsInvokeRequest) -> Result<(), String> {
    synthesize_qwen_tts(app, request).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn retry_tts(app: AppHandle, request: RetryTtsPlaybackRequest) -> Result<(), String> {
    retry_tts_playback(app, request).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn play_tts_audio(
    app: AppHandle,
    request: RetryTtsPlaybackRequest,
) -> Result<(), String> {
    retry_tts_playback(app, request).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn replay_tts_audio(
    app: AppHandle,
    request: RetryTtsPlaybackRequest,
) -> Result<(), String> {
    retry_tts_playback(app, request).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn stop_tts_audio(app: AppHandle, request: StopTtsPlaybackRequest) -> Result<(), String> {
    stop_tts_playback(app, request).await
}
