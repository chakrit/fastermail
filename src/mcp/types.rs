use serde::{Deserialize, Serialize};

// --- JSON-RPC 2.0 Types ---

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    #[allow(dead_code)]
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

impl JsonRpcResponse {
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: serde_json::Value, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError { code, message }),
        }
    }
}

// --- Error Codes ---

pub const PARSE_ERROR: i32 = -32700;
#[allow(dead_code)]
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
#[allow(dead_code)]
pub const INVALID_PARAMS: i32 = -32602;
#[allow(dead_code)]
pub const INTERNAL_ERROR: i32 = -32603;

// --- MCP Types ---

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: serde_json::Value,
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    #[serde(rename = "serverInfo")]
    pub server_info: ServerInfo,
}

#[derive(Debug, Serialize)]
pub struct ServerCapabilities {
    pub tools: ToolsCapability,
}

#[derive(Debug, Serialize)]
pub struct ToolsCapability {
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

#[derive(Debug, Serialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Serialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ToolsListResult {
    pub tools: Vec<Tool>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct ToolCallParams {
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ToolCallResult {
    pub content: Vec<ToolContent>,
    #[serde(rename = "isError")]
    pub is_error: bool,
}

#[derive(Debug, Serialize)]
pub struct ToolContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

impl ToolCallResult {
    pub fn text(text: String) -> Self {
        Self {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text,
            }],
            is_error: false,
        }
    }

    pub fn error(text: String) -> Self {
        Self {
            content: vec![ToolContent {
                content_type: "text".to_string(),
                text,
            }],
            is_error: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn success_response_omits_error_field() {
        let resp = JsonRpcResponse::success(serde_json::json!(1), serde_json::json!({"ok": true}));

        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
        assert_eq!(resp.id, serde_json::json!(1));
    }

    #[test]
    fn error_response_omits_result_field() {
        let resp = JsonRpcResponse::error(
            serde_json::json!(1),
            METHOD_NOT_FOUND,
            "Method not found".to_string(),
        );

        assert!(resp.result.is_none());
        assert!(resp.error.is_some());
        assert_eq!(
            resp.error.as_ref().expect("error should be present").code,
            -32601
        );
    }

    #[test]
    fn tool_result_text_sets_is_error_false() {
        let result = ToolCallResult::text("ok".to_string());

        assert!(!result.is_error);
    }

    #[test]
    fn tool_result_error_sets_is_error_true() {
        let result = ToolCallResult::error("fail".to_string());

        assert!(result.is_error);
    }

    #[test]
    fn tool_call_params_defaults_to_empty_when_missing() {
        let params: ToolCallParams = serde_json::from_value(serde_json::json!({}))
            .expect("should parse empty params");

        assert!(params.name.is_empty());
        assert_eq!(params.arguments, serde_json::Value::default());
    }

    #[test]
    fn initialize_params_defaults_when_fields_missing() {
        let params: InitializeParams = serde_json::from_value(serde_json::json!({}))
            .expect("should parse with defaults");

        assert!(params.protocol_version.is_empty());
        assert!(params.client_info.name.is_empty());
    }
}
