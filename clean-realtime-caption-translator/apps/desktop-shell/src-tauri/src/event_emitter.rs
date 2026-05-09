use crate::audio_session::SessionKind;
use serde::Serialize;
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter, Manager};

const MAX_STORED_EVENTS: usize = 1000;

#[derive(Clone, Debug, Serialize)]
pub struct SessionEvent<T: Serialize> {
    pub r#type: String,
    #[serde(rename = "sessionKind")]
    pub session_kind: SessionKind,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub payload: T,
}

#[derive(Clone, Debug, Serialize)]
pub struct StoredSessionEvent {
    pub sequence: u64,
    pub r#type: String,
    #[serde(rename = "sessionKind")]
    pub session_kind: SessionKind,
    #[serde(rename = "sessionId")]
    pub session_id: String,
    pub payload: Value,
}

static SESSION_EVENT_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static SESSION_EVENT_QUEUE: OnceLock<Mutex<VecDeque<StoredSessionEvent>>> = OnceLock::new();

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
    let payload_value = serde_json::to_value(payload.clone()).map_err(|error| error.to_string())?;
    let event = SessionEvent {
        r#type: event_type.to_string(),
        session_kind,
        session_id: session_id.to_string(),
        payload,
    };
    remember_session_event(StoredSessionEvent {
        sequence: SESSION_EVENT_SEQUENCE.fetch_add(1, Ordering::SeqCst) + 1,
        r#type: event_type.to_string(),
        session_kind,
        session_id: session_id.to_string(),
        payload: payload_value,
    });
    app.emit("session_event", event.clone())
        .map_err(|error| error.to_string())?;
    app.emit(event_type, event.clone())
        .map_err(|error| error.to_string())?;
    for window in app.webview_windows().values() {
        window
            .emit("session_event", event.clone())
            .map_err(|error| error.to_string())?;
        window
            .emit(event_type, event.clone())
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn drain_session_events(after_sequence: u64) -> Vec<StoredSessionEvent> {
    let queue = SESSION_EVENT_QUEUE.get_or_init(|| Mutex::new(VecDeque::new()));
    queue
        .lock()
        .map(|events| {
            events
                .iter()
                .filter(|event| event.sequence > after_sequence)
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

fn remember_session_event(event: StoredSessionEvent) {
    let queue = SESSION_EVENT_QUEUE.get_or_init(|| Mutex::new(VecDeque::new()));
    if let Ok(mut events) = queue.lock() {
        events.push_back(event);
        while events.len() > MAX_STORED_EVENTS {
            events.pop_front();
        }
    }
}
