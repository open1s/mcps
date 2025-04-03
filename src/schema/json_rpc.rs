// Copyright (c) { props["inceptionYear"] } { props["copyrightOwner"] }
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN
// THE SOFTWARE.

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The latest supported MCP protocol version
pub const LATEST_PROTOCOL_VERSION: &str = "2024-11-05";
/// The JSON-RPC version used by MCP
pub const JSONRPC_VERSION: &str = "2.0";


/// A uniquely identifying ID for a request in JSON-RPC.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    Number(i64),
}

/// JSON-RPC message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JSONRPCMessage {
    Request(JSONRPCRequest),
    Notification(JSONRPCNotification),
    Response(JSONRPCResponse),
    Error(JSONRPCError),
}

/// A request that expects a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JSONRPCRequest {
    pub jsonrpc: String,
    pub id: RequestId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}


/// A notification which does not expect a response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JSONRPCNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// A successful (non-error) response to a request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JSONRPCResponse {
    pub jsonrpc: String,
    pub id: RequestId,
    pub result: Value,
}

/// A response to a request that indicates an error occurred.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JSONRPCError {
    pub jsonrpc: String,
    pub id: RequestId,
    pub error: JSONRPCErrorObject,
}

/// Error object in a JSON-RPC error response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JSONRPCErrorObject {
    /// The error type that occurred.
    pub code: i32,

    /// A short description of the error.
    pub message: String,

    /// Additional information about the error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Standard JSON-RPC error codes
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
}

/// Base request interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<RequestParams>,
}

/// Request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<RequestMeta>,
    #[serde(flatten)]
    pub extra: DashMap<String, Value>,
}

/// Request metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_token: Option<super::common::ProgressToken>,
}

/// Base notification interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<NotificationParams>,
}

/// Notification parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<DashMap<String, Value>>,
    #[serde(flatten)]
    pub extra: DashMap<String, Value>,
}

/// Base result interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Result {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<DashMap<String, Value>>,
    #[serde(flatten)]
    pub extra: DashMap<String, Value>,
}

/// A response that indicates success but carries no data.
pub type EmptyResult = Result;

// Helper functions for creating JSON-RPC messages
impl JSONRPCRequest {
    /// Create a new JSON-RPC request
    pub fn new(id: RequestId, method: String, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            method,
            params,
        }
    }
}

impl JSONRPCNotification {
    /// Create a new JSON-RPC notification
    pub fn new(method: String, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method,
            params,
        }
    }
}

impl JSONRPCResponse {
    /// Create a new JSON-RPC response
    pub fn new(id: RequestId, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result,
        }
    }
}

impl JSONRPCError {
    /// Create a new JSON-RPC error
    pub fn new(id: RequestId, error_obj: JSONRPCErrorObject) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            error: error_obj,
        }
    }

    /// Create a new JSON-RPC error with details
    pub fn new_with_details(
        id: RequestId,
        code: i32,
        message: String,
        data: Option<Value>,
    ) -> Self {
        Self::new(
            id,
            JSONRPCErrorObject {
                code,
                message,
                data,
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonrpc_request() {
        let req = JSONRPCRequest::new(
            RequestId::String("1".to_string()),
            "method".to_string(),
            Some(Value::String("params".to_string())),
        );

        let json_req = serde_json::to_string(&req).unwrap();
        let expected_json = r#"{"jsonrpc":"2.0","id":"1","method":"method","params":"params"}"#;
        assert_eq!(json_req, expected_json);

        assert_eq!(req.jsonrpc, JSONRPC_VERSION.to_string());
        assert_eq!(req.id, RequestId::String("1".to_string()));
        assert_eq!(req.method, "method".to_string());
        assert_eq!(req.params, Some(Value::String("params".to_string())));
    }
}