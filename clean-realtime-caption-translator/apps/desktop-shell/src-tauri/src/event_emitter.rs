use crate::audio_session::SessionKind;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Debug, Serialize)]
pub struct SessionEvent<T: Serialize> {
    pub r#type: String,
    #[serde(rename = "sessionKind")]
    pub session_kind: SessionKind,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub payload: T,
}

pub fn emit_session_event<T: Serialize>(
    app: &AppHandle,
    event_type: &str,
    session_kind: SessionKind,
    session_id: &str,
    payload: T,
) -> Result<(), String>
where
    T: Clone,
{
    let event = SessionEvent {
        r#type: event_type.to_string(),
        session_kind,
        session_id: session_id.to_string(),
        payload,
    };
    app.emit("session_event", event).map_err(|error| error.to_string())
}
