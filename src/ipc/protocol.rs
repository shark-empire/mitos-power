//! Wire protocol: newline-delimited JSON (NDJSON) over a Unix domain
//! socket. Every line client->server is a `ClientMessage`; every line
//! server->client is a `ServerMessage`. See docs/ipc.md "Wire format".

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClientMessage {
    /// A method call. `params` shape depends on `method` -- see
    /// ipc::messages and docs/ipc.md.
    Request {
        id: String,
        method: String,
        #[serde(default)]
        params: serde_json::Value,
    },
    /// Add event names to this connection's subscription set. `"*"`
    /// subscribes to everything.
    Subscribe { events: Vec<String> },
    Unsubscribe { events: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ServerMessage {
    /// Reply to exactly one `Request`, correlated by `id`.
    Response {
        id: String,
        ok: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        result: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorPayload>,
    },
    /// Unsolicited push, sent to every connection subscribed to `event`.
    Event {
        event: String,
        data: serde_json::Value,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
}

pub fn ok_response(id: &str, result: serde_json::Value) -> ServerMessage {
    ServerMessage::Response { id: id.to_string(), ok: true, result: Some(result), error: None }
}

pub fn error_response(id: &str, code: &str, message: &str) -> ServerMessage {
    ServerMessage::Response {
        id: id.to_string(),
        ok: false,
        result: None,
        error: Some(ErrorPayload { code: code.to_string(), message: message.to_string() }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_round_trips() {
        let line = r#"{"kind":"request","id":"abc123","method":"GetBattery","params":{"id":"BAT0"}}"#;
        let msg: ClientMessage = serde_json::from_str(line).unwrap();
        match msg {
            ClientMessage::Request { id, method, params } => {
                assert_eq!(id, "abc123");
                assert_eq!(method, "GetBattery");
                assert_eq!(params["id"], "BAT0");
            }
            _ => panic!("expected Request"),
        }
    }

    #[test]
    fn request_without_params_defaults_to_null() {
        let line = r#"{"kind":"request","id":"1","method":"Ping"}"#;
        let msg: ClientMessage = serde_json::from_str(line).unwrap();
        match msg {
            ClientMessage::Request { params, .. } => assert!(params.is_null()),
            _ => panic!("expected Request"),
        }
    }

    #[test]
    fn error_response_serializes_without_result_field() {
        let msg = error_response("42", "NOT_FOUND", "battery 'BAT9' not found");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(!json.contains("\"result\""));
        assert!(json.contains("\"code\":\"NOT_FOUND\""));
    }
}
