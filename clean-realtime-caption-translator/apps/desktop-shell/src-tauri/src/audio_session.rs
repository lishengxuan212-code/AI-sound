use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum SessionKind {
    SystemSubtitle,
    MicInterpretation,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioSession {
    pub session_id: String,
    pub session_kind: SessionKind,
    pub is_running: bool,
    #[serde(skip)]
    pub running: Arc<AtomicBool>,
}

#[derive(Default)]
pub struct SessionRegistry {
    sessions: Mutex<HashMap<SessionKind, AudioSession>>,
}

impl SessionRegistry {
    pub async fn start(&self, session_kind: SessionKind) -> AudioSession {
        let session = AudioSession {
            session_id: uuid::Uuid::new_v4().to_string(),
            session_kind,
            is_running: true,
            running: Arc::new(AtomicBool::new(true)),
        };
        self.sessions.lock().await.insert(session_kind, session.clone());
        session
    }

    pub async fn stop(&self, session_kind: SessionKind) {
        if let Some(session) = self.sessions.lock().await.get_mut(&session_kind) {
            session.is_running = false;
            session.running.store(false, Ordering::SeqCst);
        }
    }
}
