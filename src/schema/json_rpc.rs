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

use serde_json::Value;

use super::schema::{JSONRPCError, JSONRPCErrorObject, JSONRPCNotification, JSONRPCRequest, JSONRPCResponse, RequestId, JSONRPC_VERSION};

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
    use crate::schema::schema::JSONRPC_VERSION;

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