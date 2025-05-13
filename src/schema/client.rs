//EmptyResultCopyright (c) { props["inceptionYear"] } { props["copyrightOwner"] }
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

use crate::schema::json_rpc::mcp_param;
use super::schema::{CallToolParams, CallToolRequest, CancelledNotification, ClientNotification, ClientRequest, ClientShutdownRequest, CompleteRequest, GetPromptRequest, InitializeParams, InitializeRequest, InitializedNotification, InitializedNotificationParams, JSONRPCNotification, JSONRPCRequest, ListPromptsRequest, ListResourceTemplatesRequest, ListResourcesRequest, ListToolsRequest, PaginatedParams, PingRequest, ProgressNotification, ReadResourceRequest, RequestId, RootsListChangedNotification, SetLevelRequest, SubscribeRequest, UnsubscribeRequest};

impl InitializeRequest {
    /// Create a new InitializeRequest
    pub fn new(
        params: InitializeParams,
    ) -> Self {
        InitializeRequest {
            method: "initialize".to_string(),
            params
        }
    }
}

impl ListToolsRequest {
    pub fn new(params :Option<PaginatedParams>) -> Self {
        Self {
            method:"tools/list".to_string(),
            params
        }
    }
}



impl InitializedNotification {
    pub fn new(params: InitializedNotificationParams) -> Self {
        Self {
            method: "notifications/initialized".to_string(),
            params
        }
    }
}

impl CallToolRequest{
    pub fn new(params :CallToolParams) -> Self {
        Self{
            method: "tools/call".to_string(),
            params
        }
    }
}


impl ClientShutdownRequest {
    pub fn new() -> Self {
        Self {
            method: "shutdown".to_string(),
        }
    }
}




pub fn build_client_request(id: RequestId,param: ClientRequest) -> JSONRPCRequest {
    match param {
        ClientRequest::Initialize(req) => {
            JSONRPCRequest::new(id,req.method,mcp_param(&req.params))
        }
        ClientRequest::ListTools(req) => {
            JSONRPCRequest::new(id,req.method,mcp_param(&req.params))
        }
        ClientRequest::CallTool(req) => {
            JSONRPCRequest::new(id,req.method,mcp_param(&req.params))
        }
        ClientRequest::Ping(req) => {
            JSONRPCRequest::new(id,req.method,None)
        }
        ClientRequest::GetPrompt(req) => {
            JSONRPCRequest::new(id,req.method,mcp_param(&req.params))
        }
        ClientRequest::Complete(req) => {
            JSONRPCRequest::new(id,req.method,mcp_param(&req.params))
        }
        ClientRequest::Subscribe(req) => {
            JSONRPCRequest::new(id,req.method,mcp_param(&req.params))
        }
        ClientRequest::Unsubscribe(req) => {
            JSONRPCRequest::new(id,req.method,mcp_param(&req.params))
        }
        ClientRequest::ListResources(req) => {
            JSONRPCRequest::new(id,req.method,mcp_param(&req.params))
        }
        ClientRequest::ListResourceTemplates(req) => {
            JSONRPCRequest::new(id,req.method,mcp_param(&req.params))
        }
        ClientRequest::ReadResource(req) => {
            JSONRPCRequest::new(id,req.method,mcp_param(&req.params))
        }
        ClientRequest::ListPrompts(req) => {
            JSONRPCRequest::new(id,req.method,mcp_param(&req.params))
        }
        ClientRequest::SetLevel(req) => {
            JSONRPCRequest::new(id,req.method,mcp_param(&req.params))
        }
        ClientRequest::Shutdown(req) => {
            JSONRPCRequest::new(id,req.method,None)
        }
    }
}

pub fn build_client_notification(param: ClientNotification) -> JSONRPCNotification {
    match  param {
        ClientNotification::Cancelled(notification) => {
            JSONRPCNotification::new(
                notification.method,
                mcp_param(&notification.params)
            )
        }
        ClientNotification::Initialized(notification) => {
            JSONRPCNotification::new(
                notification.method,
                mcp_param(&notification.params)
            )
        }
        ClientNotification::ProgressNotification(notification) => {
            JSONRPCNotification::new(
                notification.method,
                mcp_param(&notification.params)
            )
        }
        ClientNotification::RootsListChanged(notification) => {
            JSONRPCNotification::new(
                notification.method,
                None
            )
        }
    }
}