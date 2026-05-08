mod audio_capture;
mod audio_session;
mod commands;
mod diagnostics;
mod event_emitter;
mod local_asr_bridge;
mod local_translation_bridge;
mod mic_interpretation_session;
mod secret_redactor;
mod settings;
mod system_subtitle_session;
mod tts_bridge;
mod vad_pipeline;

use commands::{load_settings, retry_tts, start_session, stop_session, synthesize_tts};

pub fn run() {
    tauri::Builder::default()
        .manage(audio_session::SessionRegistry::default())
        .invoke_handler(tauri::generate_handler![
            load_settings,
            start_session,
            stop_session,
            synthesize_tts,
            retry_tts
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
