use crate::session::LiveSession;
use crate::error::LiveError;
use adapto_runtime::types::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<SessionId, LiveSession>>>,
    max_sessions_per_user: usize,
}

impl SessionManager {
    pub fn new(max_sessions_per_user: usize) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            max_sessions_per_user,
        }
    }

    /// Add a new session, enforcing the per-user session limit.
    pub fn add(&self, session: LiveSession) -> Result<(), LiveError> {
        let mut sessions = self.sessions.write().unwrap();

        if let Some(ref user_id) = session.user_id {
            let count = sessions
                .values()
                .filter(|s| s.user_id.as_ref() == Some(user_id))
                .count();
            if count >= self.max_sessions_per_user {
                return Err(LiveError::MaxSessionsExceeded);
            }
        }

        sessions.insert(session.id.clone(), session);
        Ok(())
    }

    /// Execute a closure with mutable access to a session.
    pub fn with_session<F, R>(&self, id: &SessionId, f: F) -> Result<R, LiveError>
    where
        F: FnOnce(&mut LiveSession) -> R,
    {
        let mut sessions = self.sessions.write().unwrap();
        let session = sessions
            .get_mut(id)
            .ok_or_else(|| LiveError::SessionNotFound(id.0.clone()))?;
        Ok(f(session))
    }

    /// Remove and return a session.
    pub fn remove(&self, id: &SessionId) -> Option<LiveSession> {
        let mut sessions = self.sessions.write().unwrap();
        sessions.remove(id)
    }

    /// Count all active sessions.
    pub fn count(&self) -> usize {
        let sessions = self.sessions.read().unwrap();
        sessions.len()
    }

    /// Count active sessions belonging to a specific user.
    pub fn count_for_user(&self, user_id: &UserId) -> usize {
        let sessions = self.sessions.read().unwrap();
        sessions
            .values()
            .filter(|s| s.user_id.as_ref() == Some(user_id))
            .count()
    }

    /// Remove all expired sessions, returning the number removed.
    pub fn cleanup_expired(&self, timeout: std::time::Duration) -> usize {
        let mut sessions = self.sessions.write().unwrap();
        let before = sessions.len();
        sessions.retain(|_, s| !s.is_expired(timeout));
        before - sessions.len()
    }

    /// Check whether a session exists.
    pub fn has(&self, id: &SessionId) -> bool {
        let sessions = self.sessions.read().unwrap();
        sessions.contains_key(id)
    }
}
