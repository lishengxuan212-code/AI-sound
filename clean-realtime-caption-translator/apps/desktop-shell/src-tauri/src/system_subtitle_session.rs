use crate::audio_session::{SessionKind, SessionRegistry};
use crate::diagnostics::diagnostic;
use crate::event_emitter::emit_session_event;
use crate::local_asr_bridge::{check_local_asr, stream_audio_to_local_asr, AudioCaptureSource};
use serde_json::json;
use tauri::{AppHandle, State};

pub async fn start_system_subtitle_session(
    app: AppHandle,
    registry: State<'_, SessionRegistry>,
) -> Result<(), String> {
    let status = check_local_asr(SessionKind::SystemSubtitle).await;
    if !status.connected {
        return Err(status
            .error
            .unwrap_or_else(|| "本地 ASR 服务未就绪，请等待服务启动完成。".to_string()));
    }

    let session = registry.start(SessionKind::SystemSubtitle).await;
    diagnostic(
        "[SYS][AUDIO]",
        json!({
            "sessionKind": "SystemSubtitle",
            "sessionId": session.session_id,
            "event.type": "start_system_subtitle",
            "note": "系统音频采集边界已初始化，本地 ASR 服务必须先启动。"
        }),
    );

    emit_session_event(
        &app,
        "session_error",
        SessionKind::SystemSubtitle,
        &session.session_id,
        json!({
            "message": if status.connected { String::new() } else { status.error.unwrap_or_else(|| "本地 ASR 服务未启动，请先运行 scripts/start_local_asr.ps1。".to_string()) },
            "localAsrConnected": status.connected
        }),
    )?;
    let app_for_task = app.clone();
    let session_id = session.session_id.clone();
    let running = session.running.clone();
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let result = match runtime {
            Ok(runtime) => runtime.block_on(stream_audio_to_local_asr(
                app_for_task.clone(),
                SessionKind::SystemSubtitle,
                session_id.clone(),
                running,
                AudioCaptureSource::SystemLoopback,
            )),
            Err(error) => Err(error.to_string()),
        };
        if let Err(error) = result {
            let _ = emit_session_event(
                &app_for_task,
                "session_error",
                SessionKind::SystemSubtitle,
                &session_id,
                json!({ "message": error }),
            );
        }
    });
    Ok(())
}
