use crate::error::ProtocolError;
use crate::event::ClientMessage;
use crate::patch::ServerMessage;

/// Decode a JSON string into a [`ClientMessage`].
///
/// This is the primary entry point for the server when receiving
/// a WebSocket text frame from the client.
pub fn decode_client_message(data: &str) -> Result<ClientMessage, ProtocolError> {
    serde_json::from_str(data).map_err(|e| ProtocolError::Serialization(e.to_string()))
}

/// Encode a [`ServerMessage`] into a JSON string for transmission.
///
/// The resulting string is ready to send as a WebSocket text frame.
pub fn encode_server_message(msg: &ServerMessage) -> Result<String, ProtocolError> {
    serde_json::to_string(msg).map_err(|e| ProtocolError::Serialization(e.to_string()))
}

/// Decode a JSON string into a [`ServerMessage`].
///
/// Used by the client runtime (or tests) to parse incoming server frames.
pub fn decode_server_message(data: &str) -> Result<ServerMessage, ProtocolError> {
    serde_json::from_str(data).map_err(|e| ProtocolError::Serialization(e.to_string()))
}

/// Encode a [`ClientMessage`] into a JSON string for transmission.
///
/// Used by the client runtime to serialize outgoing event frames.
pub fn encode_client_message(msg: &ClientMessage) -> Result<String, ProtocolError> {
    serde_json::to_string(msg).map_err(|e| ProtocolError::Serialization(e.to_string()))
}
