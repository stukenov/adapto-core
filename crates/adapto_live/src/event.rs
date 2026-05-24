use adapto_client_protocol::event::*;
use crate::error::LiveError;

/// Validate a client event's structure and security properties.
pub fn validate_client_event(event: &ClientEvent) -> Result<(), LiveError> {
    if event.session.is_empty() {
        return Err(LiveError::InvalidEvent("empty session".to_string()));
    }
    if event.handler.is_empty() {
        return Err(LiveError::InvalidEvent("empty handler".to_string()));
    }
    Ok(())
}

/// Validate a form submission event.
pub fn validate_form_event(event: &FormSubmitEvent) -> Result<(), LiveError> {
    if event.session.is_empty() {
        return Err(LiveError::InvalidEvent("empty session".to_string()));
    }
    if event.handler.is_empty() {
        return Err(LiveError::InvalidEvent("empty handler".to_string()));
    }
    Ok(())
}

/// Extract action arguments from the event payload as a single JSON value.
pub fn extract_action_args(event: &ClientEvent) -> serde_json::Value {
    serde_json::Value::Object(
        event
            .payload
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
    )
}
