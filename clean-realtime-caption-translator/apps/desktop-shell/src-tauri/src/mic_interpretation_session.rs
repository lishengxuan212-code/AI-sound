use crate::audio_session::{SessionKind, SessionRegistry};
use crate::diagnostics::diagnostic;
use crate::event_emitter::emit_session_event;
use crate::local_asr_bridge::check_local_asr;
use crate::local_asr_bridge::{stream_audio_to_local_asr, AudioCaptureSource};
use serde_json::json;
use tauri::{AppHandle, State};

pub async fn start_mic_interpretation_session(app: AppHandle, registry: State<'_, SessionRegistry>) -> Result<(), String> {
    let session = registry.start(SessionKind::MicInterpretation).await;
    diagnostic(
        "[MIC][AUDIO]",
        json!({
            "sessionKind": "MicInterpretation",
            "sessionId": session.session_id,
            "event.type": "start_mic_interpretation",
            "note": "Microphone capture boundary initialized; local ASR server must be running."
        }),
    );

    let status = check_local_asr(SessionKind::MicInterpretation).await;
    emit_session_event(
        &app,
        "session_error",
        SessionKind::MicInterpretation,
        &session.session_id,
        json!({
            "message": if status.connected { String::new() } else { status.error.unwrap_or_else(|| "Local ASR unavailable".to_string()) },
            "localAsrConnected": status.connected
        }),
    )?;
    if status.connected {
        let app_for_task = app.clone();
        let session_id = session.session_id.clone();
        let running = session.running.clone();
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build();
            let result = match runtime {
                Ok(runtime) => runtime.block_on(stream_audio_to_local_asr(
                    app_for_task.clone(),
                    SessionKind::MicInterpretation,
                    session_id.clone(),
                    running,
                    AudioCaptureSource::Microphone,
                )),
                Err(error) => Err(error.to_string()),
            };
            if let Err(error) = result {
                let _ = emit_session_event(
                    &app_for_task,
                    "session_error",
                    SessionKind::MicInterpretation,
                    &session_id,
                    json!({ "message": error }),
                );
            }
        });
    }
    Ok(())
}
