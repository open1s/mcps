use std::collections::HashMap;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The latest supported MCP protocol version
pub const LATEST_PROTOCOL_VERSION: &str = "2025-03-26";
/// The JSON-RPC version used by MCP
pub const JSONRPC_VERSION: &str = "2.0";
/// MCP session identifier key in the context data 
pub const SESSION_ID_KEY: &str = "sessionId";

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadType {
    Text = 1,
    Audio = 2,
    Image = 3,
    Embedded = 4,
}


/// Standard JSON-RPC error codes
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
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

/// The sender or recipient of messages and data in a conversation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
}

/// Annotations for objects
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotations {
    /// Describes who the intended customer of this object or data is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<Role>>,

    /// Describes how important this data is for operating the server.
    /// A value of 1 means "most important," and indicates that the data is
    /// effectively required, while 0 means "least important," and indicates that
    /// the data is entirely optional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f32>,
}

/// Base for objects that include optional annotations for the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Annotated {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

/// Base result interface
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Result {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<DashMap<String, Value>>,
    #[serde(flatten)]
    pub extra: Option<DashMap<String, Value>>,
}

pub type EmptyResult = Result;

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

/// A progress token, used to associate progress notifications with the original request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProgressToken {
    String(String),
    Number(i64),
}

/// Request metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress_token: Option<ProgressToken>,
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

/// A uniquely identifying ID for a request in JSON-RPC.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    Number(i64),
}

/// An opaque token used to represent a cursor for pagination.
pub type Cursor = String;

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


pub enum JSONRPCBatchRequest {
    Request(JSONRPCRequest),
    Notification(JSONRPCNotification),
}

/// A notification which can be sent by either side to indicate that it is cancelling a previously-issued request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelledNotification {
    pub method: String,
    pub params: CancelledParams,
}

/// Parameters for cancelled notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelledParams {
    /// The ID of the request to cancel.
    pub request_id: RequestId,

    /// An optional string describing the reason for the cancellation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Describes an implementation of MCP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Implementation {
    pub name: String,
    pub version: String,
}

/// This request is sent from the client to the server when it first connects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    pub method: String,
    pub params: InitializeParams,
}

/// Parameters for initialize request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    /// The latest version of the Model Context Protocol that the client supports.
    #[serde(rename="protocolVersion")]
    pub protocol_version: String,

    /// Client capabilities
    pub capabilities: ClientCapabilities,

    /// Client information
    #[serde(rename="clientInfo")]
    pub client_info: Implementation,
}

/// Roots capability
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsCapability {
    /// Whether the client supports notifications for changes to the roots list.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename="listChanged")]
    pub list_changed: Option<bool>,
}

/// Client capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCapabilities {
    /// Experimental, non-standard capabilities that the client supports.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<DashMap<String, Value>>,

    /// Present if the client supports listing roots.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,

    /// Present if the client supports sampling from an LLM.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<Value>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializedNotificationParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<HashMap<String, String>>,
}

/// This notification is sent from the client to the server after initialization has finished.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializedNotification {
    pub method: String,
    pub params: InitializedNotificationParams,
}

/// A ping, issued by either the server or the client, to check that the other party is still alive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingRequest {
    pub method: String,
}

/// An out-of-band notification used to inform the receiver of a progress update for a long-running request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressNotification {
    pub method: String,
    pub params: ProgressParams,
}

/// Parameters for progress notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressParams {
    /// The progress token which was given in the initial request.
    pub progress_token: ProgressToken,

    /// The progress thus far.
    pub progress: f64,

    /// Total number of items to process (or total progress required), if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedRequest {
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Cursor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResult{
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _meta: Option<HashMap<String, Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "nextCursor")]
    pub next_cursor: Option<String>,
}

/// Sent from the client to request a list of resources the server has.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResourcesRequest {
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<PaginatedParams>,
}

/// Parameters for paginated requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedParams {
    /// An opaque token representing the current pagination position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<Cursor>,
}

/// A known resource that the server is capable of reading.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// The URI of this resource.
    pub uri: String,

    /// A human-readable name for this resource.
    pub name: String,

    /// A description of what this resource represents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The MIME type of this resource, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// The size of the raw resource content, in bytes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

/// The server's response to a resources/list request from the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResourcesResult {
    /// An opaque token representing the pagination position after the last returned result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,

    /// The list of resources
    pub resources: Vec<Resource>,
}

/// Sent from the client to request a list of resource templates the server has.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResourceTemplatesRequest {
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<PaginatedParams>,
}

/// Sent from the client to the server, to read a specific resource URI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResourceRequest {
    pub method: String,
    pub params: ReadResourceParams,
}

/// Parameters for read resource request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResourceParams {
    /// The URI of the resource to read.
    pub uri: String,
}

/// The server's response to a resources/read request from the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResourceResult {
    pub contents: Vec<ResourceContent>,
}



/// Binary resource contents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobResourceContents {
    /// The URI of this resource.
    pub uri: String,

    /// The MIME type of this resource, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// A base64-encoded string representing the binary data of the item.
    pub blob: String,
}

/// Resource content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResourceContent {
    Text(TextResourceContents),
    Blob(BlobResourceContents),
}

/// An optional notification from the server to the client, informing it that the list of resources it can read from has changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceListChangedNotification {
    pub method: String,
}

/// Parameters for subscribe request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeParams {
    /// The URI of the resource to subscribe to.
    pub uri: String,
}

/// Sent from the client to request resources/updated notifications from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeRequest {
    pub method: String,
    pub params: SubscribeParams,
}


/// Sent from the client to request cancellation of resources/updated notifications from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsubscribeRequest {
    pub method: String,
    pub params: UnsubscribeParams,
}

/// Parameters for unsubscribe request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsubscribeParams {
    /// The URI of the resource to unsubscribe from.
    pub uri: String,
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

/// The contents of a specific resource or sub-resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResourceContents {
    Text(TextResourceContents),
    Blob(BlobResourceContents),
}


/// Text resource contents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextResourceContents {
    /// The URI of this resource.
    pub uri: String,

    /// The MIME type of this resource, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    /// The text of the item.
    pub text: String,
}

/// Sent from the client to request a list of prompts and prompt templates the server has.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPromptsRequest {
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<PaginatedParams>,
}

/// Describes an argument that a prompt can accept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    /// The name of the argument.
    pub name: String,

    /// A human-readable description of the argument.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Whether this argument must be provided.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

/// A prompt or prompt template that the server offers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    /// The name of the prompt or prompt template.
    pub name: String,

    /// An optional description of what this prompt provides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// A list of arguments to use for templating the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
}

/// The server's response to a prompts/list request from the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPromptsResult {
    /// An opaque token representing the pagination position after the last returned result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,

    /// The list of prompts
    pub prompts: Vec<Prompt>,
}

/// Used by the client to get a prompt provided by the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPromptRequest {
    pub method: String,
    pub params: GetPromptParams,
}

/// Parameters for get prompt request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPromptParams {
    /// The name of the prompt or prompt template.
    pub name: String,

    /// Arguments to use for templating the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<DashMap<String, String>>,
}

/// Text provided to or from an LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    pub r#type: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

/// An image provided to or from an LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    pub r#type: String,
    pub data: String,
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioContent {
    pub r#type: String,
    pub data: String,
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

/// The contents of a resource, embedded into a prompt or tool call result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedResource {
    pub r#type: String,
    pub resource: ResourceContents,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

/// Content of a prompt message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PromptMessageContent {
    Text(TextContent),
    Image(ImageContent),
    Audio(AudioContent),
    Resource(EmbeddedResource),
}

/// Describes a message returned as part of a prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    pub role: Role,
    #[serde(flatten)]
    pub content: PromptMessageContent,
}

/// The server's response to a prompts/get request from the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPromptResult {
    /// An optional description for the prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The prompt messages
    pub messages: Vec<PromptMessage>,
}

/// An optional notification from the server to the client, informing it that the list of prompts it offers has changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptListChangedNotification {
    pub method: String,
}

/// Sent from the client to request a list of tools the server has.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListToolsRequest {
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<PaginatedParams>,
}

/// The server's response to a tools/list request from the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListToolsResult {
    /// An opaque token representing the pagination position after the last returned result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,

    /// The list of tools
    pub tools: Vec<Tool>,
}

/// Definition for a tool the client can call.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// The name of the tool.
    pub name: String,

    /// A human-readable description of the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// A JSON Schema object defining the expected parameters for the tool.
    pub input_schema: ToolInputSchema,
}

/// JSON Schema for tool input
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInputSchema {
    pub r#type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<DashMap<String, Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

/// Represents a root directory or file that the server can operate on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Root {
    /// The URI identifying the root. This must start with file:// for now.
    pub uri: String,

    /// An optional name for the root.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
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

/// The server's response to a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolResult {
    pub content: Vec<ToolResultContent>,

    /// Whether the tool call ended in an error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

/// Used by the client to invoke a tool provided by the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolRequest {
    pub method: String,
    pub params: CallToolParams,
}

/// Parameters for call tool request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolParams {
    /// The name of the tool to call
    pub name: String,

    /// Arguments for the tool
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<DashMap<String, Value>>,
}

/// An optional notification from the server to the client, informing it that the list of tools it offers has changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolListChangedNotification {
    pub method: String,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolAnnotations{
    #[serde(rename="destructiveHint")]
    pub destructive_hint: bool,

    #[serde(rename="idempotentHint")]
    pub idempotent_hint: bool,

    #[serde(rename="openWorldHint")]
    pub open_world_hint: bool,

    #[serde(rename="readOnlyHint")]
    pub read_only_hint: bool,

    pub title: String,
}

/// A request from the client to the server, to enable or adjust logging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetLevelRequest {
    pub method: String,
    pub params: SetLevelParams,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash,PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LoggingLevel {
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

impl ToString for LoggingLevel {
    fn to_string(&self) -> String {
        let str = match self { 
            LoggingLevel::Info => "info",
            LoggingLevel::Notice => {
                "notice"
            },
            LoggingLevel::Warning => "warning",
            LoggingLevel::Error => "error",
            LoggingLevel::Critical => "critical",
            LoggingLevel::Alert => "alert",
            LoggingLevel::Emergency => "emergency",
            LoggingLevel::Debug => "debug",
        };

        str.to_string()
    }
}

impl From<&str> for LoggingLevel {
    fn from(level_str: &str) -> Self {
        let l = match level_str.to_lowercase().as_str() {
            "debug" => Self::Debug,
            "info" => Self::Info,
            "error" => Self::Error,
            "notice" => Self::Notice,
            "warn" => Self::Warning,
            "critical" => Self::Critical,
            "alert" => Self::Alert,
            "emergency" => Self::Emergency,
            _ => Self::Info,
        };
        l
    }
}

/// Parameters for set level request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetLevelParams {
    /// The level of logging that the client wants to receive from the server.
    pub level: LoggingLevel,
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

/// Hints to use for model selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelHint {
    /// A hint for a model name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
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

/// Message content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Audio(AudioContent),
    Text(TextContent),
    Image(ImageContent),
}

/// Describes a message issued to or received from an LLM API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingMessage {
    pub role: Role,
    #[serde(flatten)]
    pub content: MessageContent,
}

/// Include context options
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IncludeContext {
    None,
    ThisServer,
    AllServers,
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

/// Known stop reasons
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KnownStopReason {
    EndTurn,
    StopSequence,
    MaxTokens,
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

/// A request from the client to the server, to ask for completion options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteRequest {
    pub method: String,
    pub params: CompleteParams,
}

/// Reference to a prompt or resource
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Reference {
    Prompt(PromptReference),
    Resource(ResourceReference),
}

/// Identifies a prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptReference {
    pub r#type: String,

    /// The name of the prompt or prompt template
    pub name: String,
}

/// A reference to a resource or resource template definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceReference {
    pub r#type: String,

    /// The URI or URI template of the resource.
    pub uri: String,
}

/// Argument information for completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgumentInfo {
    /// The name of the argument
    pub name: String,

    /// The value of the argument to use for completion matching.
    pub value: String,
}

/// Parameters for complete request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteParams {
    /// Reference to a prompt or resource
    #[serde(rename = "ref")]
    pub ref_: Reference,

    /// The argument's information
    pub argument: ArgumentInfo,
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

/// The client's response to a roots/list request from the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListRootsResult {
    pub _meta: Option<HashMap<String, String>>,
    pub roots: Vec<Root>,
}


/// A notification from the client to the server, informing it that the list of roots has changed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsListChangedNotification {
    pub method: String,
}

/// A template description for resources available on the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceTemplate {
    /// A URI template (according to RFC 6570) that can be used to construct resource URIs.
    pub uri_template: String,

    /// A human-readable name for the type of resource this template refers to.
    pub name: String,

    /// A description of what this template is for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The MIME type for all resources that match this template.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

/// The server's response to a resources/templates/list request from the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResourceTemplatesResult {
    /// An opaque token representing the pagination position after the last returned result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<Cursor>,

    /// The list of resource templates
    pub resource_templates: Vec<ResourceTemplate>,
}

/// Result of a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolCallResult {
    /// The result of the tool call
    pub result: Value,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JSONRPCBatchResponse {
    #[serde(flatten)]
    response: Vec<JSONRPCBatchResponseEnum>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JSONRPCBatchResponseEnum {
    Response(JSONRPCResponse),
    Error(JSONRPCError),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientShutdownRequest {
    pub method: String,
}

/// JSON-RPC message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ClientRequest {
    Initialize(InitializeRequest),
    Ping(PingRequest),
    ListResources(ListResourcesRequest),
    ListResourceTemplates(ListResourceTemplatesRequest),
    ReadResource(ReadResourceRequest),
    Subscribe(SubscribeRequest),
    Unsubscribe(UnsubscribeRequest),
    ListPrompts(ListPromptsRequest),
    GetPrompt(GetPromptRequest),
    ListTools(ListToolsRequest),
    CallTool(CallToolRequest),
    SetLevel(SetLevelRequest),
    Complete(CompleteRequest),
    Shutdown(ClientShutdownRequest),
}


pub enum ClientNotification {
    Cancelled(CancelledNotification),
    Initialized(InitializedNotification),
    ProgressNotification(ProgressNotification),
    RootsListChanged(RootsListChangedNotification),
}

pub enum ClientResult {
    Empty(EmptyResult),
    CreateMessage(CreateMessageResult),
    ListRootsResult(ListRootsResult)
}

pub enum ServerRequest{
    Ping(PingRequest),
    CreateMessageRequest(CreateMessageRequest),
    ListRootsRequest(ListRootsRequest),
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

pub enum ServerResult{
    Result(Result),
    EmptyResult(EmptyResult),
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


pub enum ClientRequst {
    CreateMessageResult(CreateMessageResult),
    ListRootsResult(ListRootsResult),
    ListToolsResult(ListToolsResult),
    CallToolResult(CallToolResult)
}