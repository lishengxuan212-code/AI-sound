mod audio_capture;
mod audio_session;
mod caption_state;
mod commands;
mod diagnostics;
mod event_emitter;
mod local_asr_bridge;
mod local_translation_bridge;
mod mic_interpretation_session;
mod secret_redactor;
mod service_manager;
mod settings;
mod system_subtitle_session;
mod tts_bridge;
mod vad_pipeline;

use commands::{
    drain_session_events, ensure_local_services_ready, get_app_settings,
    get_realtime_caption_snapshot, get_secret_status, list_audio_devices, list_tts_voices,
    load_settings, play_tts_audio, probe_local_service_status, replay_tts_audio,
    reset_app_settings, retry_tts, save_app_settings, start_session, stop_session, stop_tts_audio,
    synthesize_tts, test_local_asr_connection, test_local_translation_connection,
    test_tts_connection,
};

pub fn run() {
    tauri::Builder::default()
        .manage(audio_session::SessionRegistry::default())
        .invoke_handler(tauri::generate_handler![
            load_settings,
            get_app_settings,
            save_app_settings,
            reset_app_settings,
            test_local_asr_connection,
            test_local_translation_connection,
            probe_local_service_status,
            ensure_local_services_ready,
            test_tts_connection,
            get_secret_status,
            list_audio_devices,
            drain_session_events,
            get_realtime_caption_snapshot,
            list_tts_voices,
            start_session,
            stop_session,
            synthesize_tts,
            retry_tts,
            play_tts_audio,
            replay_tts_audio,
            stop_tts_audio
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
