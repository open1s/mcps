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
use crate::schema::schema::{EmptyResult, JSONRPCError};
use super::{json_rpc::mcp_param, schema::{JSONRPCNotification, JSONRPCRequest, ListRootsRequest, LoggingMessageNotification, LoggingMessageParams, RequestId, ServerNotification, ServerRequest}};

impl ListRootsRequest {
    pub fn new() -> Self {
        Self {
            method: "roots/list".to_string(),
        }
    }
}

impl LoggingMessageNotification{
    pub fn new(params : LoggingMessageParams) -> Self {
        Self {
            method: "notifications/message".to_string(),
            params,
        }
    }
}

impl EmptyResult {
    pub fn new() -> Self {
        EmptyResult{
            _meta: None,
            extra: None,
        }
    }
}


pub fn build_server_request(id: RequestId, param: ServerRequest) -> JSONRPCRequest {
    match param {
        ServerRequest::Ping(req) => {
            JSONRPCRequest::new(id,req.method,None)
        },
        ServerRequest::CreateMessageRequest(req) => {
            JSONRPCRequest::new(id,req.method,mcp_param(&req.params))
        },
        ServerRequest::ListRootsRequest(req) => {
            JSONRPCRequest::new(id,req.method,None)
        },
    }
}

pub fn build_server_notification(param: ServerNotification) -> JSONRPCNotification {
    match param {
        ServerNotification::CancelledNotification(req) => {
            JSONRPCNotification::new(
                req.method,
                mcp_param(&req.params)
            )
        },
        ServerNotification::ProgressNotification(req) => {
            JSONRPCNotification::new(
                req.method,
                mcp_param(&req.params)
            )
        },
        ServerNotification::ResourceListChangedNotification(req) => {
            JSONRPCNotification::new(
                req.method,
                None
            )
        },
        ServerNotification::ResourceUpdatedNotification(req) => {
            JSONRPCNotification::new(
                req.method,
                mcp_param(&req.params)
            )
        },
        ServerNotification::PromptListChangedNotification(req) => {
            JSONRPCNotification::new(
                req.method,
                None
            )
        },
        ServerNotification::ToolListChangedNotification(req) => {
            JSONRPCNotification::new(
                req.method,
                None
            )
        },
        ServerNotification::LoggingMessageNotification(req) => {
            JSONRPCNotification::new(
                req.method,
                mcp_param(&req.params)
            )
        },
    }
}


pub fn build_server_error(
    id: RequestId,
    code: i32,
    message: String,
    data: Option<Value>,
)  -> JSONRPCError {
    JSONRPCError::new_with_details(id, code,message,data)
}

















































