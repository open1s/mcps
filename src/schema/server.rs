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

use std::{collections::HashMap};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{client::{CancelledNotification, GetPromptResult, ListPromptsResult, ListResourceTemplatesResult, ListResourcesResult, ListToolsResult, PingRequest, ProgressNotification, ReadResourceResult}, common::{
    AudioContent, EmbeddedResource, ImageContent, Implementation, LoggingLevel, Role, TextContent
}, json_rpc};


/// Server capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    /// Experimental, non-standard capabilities that the server supports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<DashMap<String, Value>>,

    /// Present if the server supports sending log messages to the client.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<Value>,

    /// Present if the server offers any prompt templates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,

    /// Present if the server offers any resources to read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,

    /// Present if the server offers any tools to call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
}

/// Prompts capability
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptsCapability {
    /// Whether this server supports notifications for changes to the prompt list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Resources capability
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesCapability {
    /// Whether this server supports subscribing to resource updates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,

    /// Whether this server supports notifications for changes to the resource list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// Tools capability
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCapability {
    /// Whether this server supports notifications for changes to the tool list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

/// After receiving an initialize request from the client, the server sends this response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// The version of the Model Context Protocol that the server wants to use.
    pub protocol_version: String,

    /// Server capabilities
    pub capabilities: ServerCapabilities,

    /// Server information
    pub server_info: Implementation,

    /// Instructions describing how to use the server and its features.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

/// A notification from the server to the client, informing it that a resource has changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUpdatedNotification {
    pub method: String,
    pub params: ResourceUpdatedParams,
}

/// Parameters for resource updated notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUpdatedParams {
    /// The URI of the resource that has been updated.
    pub uri: String,
}

/// An optional notification from the server to the client, informing it that the list of resources it can read from has changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceListChangedNotification {
    pub method: String,
}

/// An optional notification from the server to the client, informing it that the list of prompts it offers has changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptListChangedNotification {
    pub method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAnnotations{
    pub destructiveHint: bool,
    pub idempotentHint: bool,
    pub openWorldHint: bool,
    pub readOnlyHint: bool,
    pub title: String,
}


/// An optional notification from the server to the client, informing it that the list of tools it offers has changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolListChangedNotification {
    pub method: String,
}

/// Notification of a log message passed from server to client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingMessageNotification {
    pub method: String,
    pub params: LoggingMessageParams,
}

/// Parameters for logging message notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingMessageParams {
    /// The severity of this log message.
    pub level: LoggingLevel,

    /// An optional name of the logger issuing this message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logger: Option<String>,

    /// The data to be logged, such as a string message or an object.
    pub data: Value,
}

/// A request from the server to sample an LLM via the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessageRequest {
    pub method: String,
    pub params: CreateMessageParams,
}

/// Parameters for create message request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessageParams {
    /// The messages to sample from
    pub messages: Vec<SamplingMessage>,

    /// The server's preferences for which model to select.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_preferences: Option<ModelPreferences>,

    /// An optional system prompt the server wants to use for sampling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,

    /// A request to include context from one or more MCP servers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_context: Option<IncludeContext>,

    /// Temperature for sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// The maximum number of tokens to sample.
    pub max_tokens: u32,

    /// Stop sequences for sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    /// Optional metadata to pass through to the LLM provider.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// Include context options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IncludeContext {
    None,
    ThisServer,
    AllServers,
}

/// The client's response to a sampling/create_message request from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessageResult {
    /// The role of the message
    pub role: Role,

    /// The content of the message
    #[serde(flatten)]
    pub content: MessageContent,

    /// The name of the model that generated the message.
    pub model: String,

    /// The reason why sampling stopped, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<StopReason>,
}

/// Message content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Audio(AudioContent),
    Text(TextContent),
    Image(ImageContent),
}

/// Stop reason
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StopReason {
    /// Known stop reasons
    #[serde(rename_all = "camelCase")]
    Known(KnownStopReason),
    /// Custom stop reason
    Custom(String),
}

/// Known stop reasons
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KnownStopReason {
    EndTurn,
    StopSequence,
    MaxTokens,
}

/// Describes a message issued to or received from an LLM API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingMessage {
    pub role: Role,
    #[serde(flatten)]
    pub content: MessageContent,
}

/// The server's preferences for model selection, requested of the client during sampling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPreferences {
    /// Optional hints to use for model selection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hints: Option<Vec<ModelHint>>,

    /// How much to prioritize cost when selecting a model.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "costPriority")]
    pub cost_priority: Option<f32>,

    /// How much to prioritize sampling speed (latency) when selecting a model.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "speedPriority")]
    pub speed_priority: Option<f32>,

    /// How much to prioritize intelligence and capabilities when selecting a model.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "intelligencePriority")]
    pub intelligence_priority: Option<f32>,
}

/// Hints to use for model selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelHint {
    /// A hint for a model name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// The server's response to a completion/complete request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteResult {
    _meta: Option<HashMap<String, String>>,
    pub completion: CompletionInfo,
}

/// Completion information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionInfo {
    /// An array of completion values.
    pub values: Vec<String>,

    /// The total number of completion options available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u32>,

    /// Indicates whether there are additional completion options beyond those provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
}

/// Sent from the server to request a list of root URIs from the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRootsRequest {
    pub method: String,
}

/// The server's response to a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolResult {
    pub content: Vec<ToolResultContent>,

    /// Whether the tool call ended in an error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Tool result content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    Audio(AudioContent),
    Text(TextContent),
    Image(ImageContent),
    Resource(EmbeddedResource),
}

/// Result of a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallResult {
    /// The result of the tool call
    pub result: Value,
}

pub enum ServerNotification{
    CancelledNotification(CancelledNotification),
    ProgressNotification(ProgressNotification),
    ResourceListChangedNotification(ResourceListChangedNotification),
    ResourceUpdatedNotification(ResourceUpdatedNotification),
    PromptListChangedNotification(PromptListChangedNotification),
    ToolListChangedNotification(ToolListChangedNotification),
    LoggingMessageNotification(LoggingMessageNotification),
}

pub enum ServerRequest{
    Ping(PingRequest),
    CreateMessageRequest(CreateMessageRequest),
    ListRootsRequest(ListRootsRequest),
}

pub enum ServerResult{
    Result(json_rpc::Result),
    InitializeRusult(InitializeResult),
    ListResourcesResult(ListResourcesResult),
    ListResourceTemplatesResult(ListResourceTemplatesResult),
    ReadResourceResult(ReadResourceResult),
    ListPromptsResult(ListPromptsResult),
    GetPromptResult(GetPromptResult),
    ListToolsResult(ListToolsResult),
    CallToolResult(CallToolResult),
    CompleteResult(CompleteResult),   
}
