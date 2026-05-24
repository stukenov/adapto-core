use adapto_client_protocol::event::*;
use adapto_client_protocol::patch::*;
use crate::session::LiveSession;
use crate::error::LiveError;
use adapto_auth::rate_limit::RateLimiter;

pub struct EventDispatcher {
    rate_limiter: RateLimiter,
}

impl EventDispatcher {
    pub fn new(rate_limit: u32) -> Self {
        Self {
            rate_limiter: RateLimiter::new(rate_limit),
        }
    }

    /// Dispatch a client message to the appropriate session handler.
    ///
    /// Validates rate limits before forwarding the event, and maps each
    /// payload variant to the correct session method. Heartbeats are
    /// acknowledged directly without touching handler logic.
    pub fn dispatch(
        &mut self,
        session: &mut LiveSession,
        msg: &ClientPayload,
    ) -> Result<ServerPayload, LiveError> {
        self.validate_event(session, &session.id.0.clone())?;

        match msg {
            ClientPayload::Event(event) => {
                let patch = session.handle_event(event)?;
                Ok(ServerPayload::Patch(patch))
            }
            ClientPayload::FormSubmit(form) => {
                let patch = session.handle_form_submit(form)?;
                Ok(ServerPayload::Patch(patch))
            }
            ClientPayload::Navigate(nav) => {
                // Navigation produces a redirect response.
                Ok(ServerPayload::Redirect(RedirectMessage {
                    url: nav.path.clone(),
                    flash: None,
                }))
            }
            ClientPayload::Heartbeat(hb) => {
                Ok(ServerPayload::HeartbeatAck(HeartbeatAck { seq: hb.seq }))
            }
        }
    }

    /// Validate the event against rate limits.
    fn validate_event(
        &mut self,
        _session: &LiveSession,
        session_id: &str,
    ) -> Result<(), LiveError> {
        self.rate_limiter
            .check(session_id)
            .map_err(|_| LiveError::RateLimitExceeded)
    }
}
