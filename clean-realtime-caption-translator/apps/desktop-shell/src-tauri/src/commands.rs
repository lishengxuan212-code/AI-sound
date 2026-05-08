use crate::audio_session::{SessionKind, SessionRegistry};
use crate::mic_interpretation_session::start_mic_interpretation_session;
use crate::settings::{load_app_settings, AppSettings};
use crate::system_subtitle_session::start_system_subtitle_session;
use crate::tts_bridge::{retry_tts_playback, synthesize_qwen_tts, RetryTtsPlaybackRequest, TtsInvokeRequest};
use tauri::{AppHandle, State};

#[tauri::command(rename_all = "camelCase")]
pub async fn load_settings() -> Result<AppSettings, String> {
    Ok(load_app_settings())
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
pub async fn stop_session(registry: State<'_, SessionRegistry>, session_kind: SessionKind) -> Result<(), String> {
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
