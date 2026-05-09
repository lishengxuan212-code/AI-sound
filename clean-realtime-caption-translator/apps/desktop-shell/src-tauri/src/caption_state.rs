use crate::audio_session::SessionKind;
use serde::Serialize;
use std::sync::{Mutex, OnceLock};

#[derive(Clone, Debug, Default, Serialize)]
pub struct RealtimeCaptionSnapshot {
    #[serde(rename = "systemSessionId")]
    pub system_session_id: String,
    #[serde(rename = "systemUtteranceId")]
    pub system_utterance_id: String,
    #[serde(rename = "systemRawTranscript")]
    pub system_raw_transcript: String,
    #[serde(rename = "systemTranslatedText")]
    pub system_translated_text: String,
    #[serde(rename = "micSessionId")]
    pub mic_session_id: String,
    #[serde(rename = "micUtteranceId")]
    pub mic_utterance_id: String,
    #[serde(rename = "micRawTranscript")]
    pub mic_raw_transcript: String,
    #[serde(rename = "micTranslatedText")]
    pub mic_translated_text: String,
    #[serde(rename = "updatedAt")]
    pub updated_at: u128,
}

static CAPTION_STATE: OnceLock<Mutex<RealtimeCaptionSnapshot>> = OnceLock::new();

pub fn get_realtime_caption_snapshot() -> RealtimeCaptionSnapshot {
    state()
        .lock()
        .map(|snapshot| snapshot.clone())
        .unwrap_or_default()
}

pub fn update_raw_transcript(
    session_kind: SessionKind,
    session_id: &str,
    utterance_id: &str,
    text: &str,
) {
    if let Ok(mut snapshot) = state().lock() {
        match session_kind {
            SessionKind::SystemSubtitle => {
                snapshot.system_session_id = session_id.to_string();
                snapshot.system_utterance_id = utterance_id.to_string();
                snapshot.system_raw_transcript = text.to_string();
            }
            SessionKind::MicInterpretation => {
                if snapshot.mic_utterance_id != utterance_id {
                    snapshot.mic_translated_text.clear();
                }
                snapshot.mic_session_id = session_id.to_string();
                snapshot.mic_utterance_id = utterance_id.to_string();
                snapshot.mic_raw_transcript = text.to_string();
            }
        }
        snapshot.updated_at = now_ms();
    }
}

pub fn update_translated_text(
    session_kind: SessionKind,
    session_id: &str,
    utterance_id: &str,
    text: &str,
) {
    if let Ok(mut snapshot) = state().lock() {
        match session_kind {
            SessionKind::SystemSubtitle => {
                snapshot.system_session_id = session_id.to_string();
                snapshot.system_utterance_id = utterance_id.to_string();
                snapshot.system_translated_text = text.to_string();
            }
            SessionKind::MicInterpretation => {
                snapshot.mic_session_id = session_id.to_string();
                snapshot.mic_utterance_id = utterance_id.to_string();
                snapshot.mic_translated_text = text.to_string();
            }
        }
        snapshot.updated_at = now_ms();
    }
}

fn state() -> &'static Mutex<RealtimeCaptionSnapshot> {
    CAPTION_STATE.get_or_init(|| Mutex::new(RealtimeCaptionSnapshot::default()))
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{get_realtime_caption_snapshot, update_raw_transcript, update_translated_text};
    use crate::audio_session::SessionKind;

    #[test]
    fn mic_raw_transcript_for_new_utterance_clears_previous_mic_translation() {
        update_translated_text(
            SessionKind::MicInterpretation,
            "mic-session",
            "mic-old",
            "old",
        );
        update_raw_transcript(
            SessionKind::MicInterpretation,
            "mic-session",
            "mic-new",
            "new",
        );

        let snapshot = get_realtime_caption_snapshot();

        assert_eq!(snapshot.mic_raw_transcript, "new");
        assert_eq!(snapshot.mic_translated_text, "");
    }
}
