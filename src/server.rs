use crate::{
    schema::{
        json_rpc::{mcp_from_value, mcp_json_param, mcp_param, mcp_to_value},
        schema::{
            CallToolParams, EmptyResult, Implementation, InitializeParams, InitializeResult, JSONRPCError, JSONRPCMessage, JSONRPCResponse, ListRootsRequest, ListToolsResult, LoggingLevel, LoggingMessageNotification, LoggingMessageParams, RequestId, ServerCapabilities, ServerNotification, ServerRequest, SetLevelParams, TextContent, Tool, ToolResultContent, ToolsCapability, LATEST_PROTOCOL_VERSION, SESSION_ID_KEY
        },
        server::{build_server_notification, build_server_request},
    },
    support::{
        disruptor::{DisruptorFactory, DisruptorWriter}, jobman::JobManager, logging::setup_logging, sessons::SESSION_STORE, ControlBus
    },
    MCPError,
};
use chrono::Utc;
use dashmap::DashMap;
use disruptor::{Producer, Sequence};
use ibag::{iBag};
use log::{info};
use rioc::{ChainContext, JobTask, LayerChain, LayerResult, PayLoad, SharedLayer, TaskEvent};
use serde_json::{json, Value};
use std::{
    collections::HashMap, sync::{Arc, Mutex}, time::Duration
};
use std::cell::RefCell;
use crossbeam::channel::{Receiver, Sender};
use crate::schema::schema::{AudioContent, CallToolResult, CancelledParams, EmbeddedResource, ImageContent, LoadType, ResourceContents,error_codes};
use crate::schema::server::build_server_error;
use crate::support::sessons::{get_current_session, set_session_id, SessionItem};

#[derive(Clone)]
pub struct ServerConfig {
    pub name: String,
    pub version: String,
    pub tools: Vec<Tool>,
    pub timeout: Option<Duration>,
}

impl ServerConfig {
    pub fn new() -> Self {
        Self {
            name: "MCP Server".to_string(),
            version: "1.0.0".to_string(),
            tools: Vec::new(),
            timeout: None,
        }
    }

    pub fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn with_version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    pub fn with_tools(mut self, tools: Tool) -> Self {
        self.tools.push(tools);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone,PartialEq)]
pub enum ServerState {
    Initialized,
    Running,
    Uninitialized,
    Shutdown,
}

pub trait ServerProvider { 
}


pub type ToolHandler = Arc<Box<dyn Fn(Value,Sender<TaskEvent<(LoadType,String),i32>>,Receiver<String>,) -> Result<Value, MCPError> + Send + Sync + 'static>>;

#[derive(Clone)]
pub struct Server {
    config: ServerConfig,
    tool_handlers: Arc<Mutex<HashMap<String, ToolHandler>>>,
    notify: Arc<ControlBus>,
    chain: iBag<LayerChain>,
    disruptor: Option<DisruptorWriter>,
    is_initialized: bool,
    current_request_id: Option<i64>,
    cached: Arc<Mutex<Vec<JSONRPCMessage>>>,
    next_request_id: i64,
    timeout_duration: Option<Duration>,
    state: ServerState,
    job_manager: Arc<Mutex<RefCell<JobManager>>>,
}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        //check if config/log4rs.yaml exists
        Self {
            config,
            tool_handlers: Arc::new(Mutex::new(HashMap::new())),
            notify: Arc::new(ControlBus::new()),
            chain: iBag::new(LayerChain::new()),
            disruptor: None,
            is_initialized: false,
            next_request_id: 0,
            current_request_id: None,
            cached: Arc::new(Mutex::new(Vec::new())),
            timeout_duration: None,
            state: ServerState::Uninitialized,
            job_manager: Arc::new(Mutex::new(RefCell::new(JobManager::new()))),
        }
    }

    fn cache_response(&self, message: JSONRPCMessage) {
        let mut cached = self.cached.lock().unwrap();
        cached.push(message);
    }

    fn pop_response(&self) -> Option<JSONRPCMessage> {
        let mut cached = self.cached.lock().unwrap();
        if cached.is_empty() {
            return None;
        }
        Some(cached.remove(0))
    }

    pub fn with_timeout(&mut self, duration: Duration) -> &mut Self {
        self.timeout_duration = Some(duration);
        self
    }

    pub fn serve(&self) -> Result<(), MCPError> {
        let _ = self.handle_inbound();
        Ok(())
    }

    pub fn list_roots(&mut self) -> Result<Value, MCPError> {
        let request = ListRootsRequest::new();

        let req = ServerRequest::ListRootsRequest(request);
        let request_id = self.next_request_id();
        let req = build_server_request(request_id, req);
        let payload = rioc::PayLoad {
            data: mcp_json_param(&req),
            ctx: None,
        };

        //send initial request to server
        let _ = self.handle_outbound(Some(payload));
        
        //wait for response
        let result = self.recieve_with_timeout()?;
        if let JSONRPCMessage::Response(response) = result {
            return Ok(response.result);
        }
        Err(MCPError::Transport("Failed to get roots list".to_string()))
    }

    pub fn recieve_with_timeout(&mut self) -> Result<JSONRPCMessage, MCPError> {
        if self.timeout_duration.is_none() {
            //receive forever
            loop {
                let result = self.try_recieve();
                if result.is_ok() {
                    return result;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        } else {
            let timeout_duration = self.timeout_duration.unwrap();
            let start_time = std::time::Instant::now();

            while start_time.elapsed() < timeout_duration {
                let result = self.try_recieve();
                if result.is_ok() {
                    return result;
                }
                std::thread::sleep(Duration::from_millis(300));
            }
            return Err(MCPError::Transport("Timeout".to_string()));
        }
    }

    pub fn try_recieve(&mut self) -> Result<JSONRPCMessage, MCPError> {
        // Check if there is any cached message
        if let Some(message) = self.pop_response() {
            return Ok(message);
        }

        Err(MCPError::Transport("No cached message".to_string()))
    }

    fn next_request_id(&mut self) -> RequestId {
        self.next_request_id += 1;
        let id = self.next_request_id;
        self.current_request_id = Some(id);
        RequestId::Number(id)
    }

    pub fn register_tool_handler<F>(&self, tool_name: String, handler: F) -> Result<(), MCPError>
    where
        F: Fn(Value,Sender<TaskEvent<(LoadType,String),i32>>,Receiver<String>) -> Result<Value, MCPError> + Send  + Sync + 'static,
    {
        //check if the tool exists
        if !self.config.tools.iter().any(|tool| tool.name == tool_name) {
            return Err(MCPError::Transport(format!(
                "Tool {} not found in server config",
                tool_name
            )));
        }

        //register the tool handler
        let mut handlers = match self.tool_handlers.try_lock() {
            Ok(handlers) => handlers,
            Err(_) => {
                return Err(MCPError::Transport(
                    "Failed to lock tool handlers".to_string(),
                ))
            }
        };

        let handler: Arc<Box<dyn Fn(Value, Sender<TaskEvent<(LoadType,String), i32>>,Receiver<String>) -> Result<Value, MCPError> + Send + Sync>> =
            Arc::new(Box::new(handler));
        handlers.insert(tool_name, handler);

        Ok(())
    }

    pub fn start(&mut self) -> Result<(), MCPError> {
        if self.is_initialized {
            return Err(MCPError::Transport(
                "Server already initialized".to_string(),
            ));
        }
        self.is_initialized = true;
        let mut server = self.clone();
        let disruptor =
            DisruptorFactory::create(move |e: &PayLoad, _seq: Sequence, _end_of_patch: bool| {
                if let Some(data) = &e.data {
                    info!("Received message: {:?}", data);

                    let ctx = e.ctx.clone();

                    match serde_json::from_str::<JSONRPCMessage>(&data) {
                        Ok(message) => {
                            //setup session if needed
                            match ctx {
                                None => {},
                                Some(ref ss) => {
                                    let session_id = ss.data.get(SESSION_ID_KEY);
                                    if let Some(sid) = session_id {
                                        //try find session
                                        if let Some(session) = SESSION_STORE.get_session(sid) {
                                           //find debug level
                                           let debug_level = session.get_item("debug_level");
                                           if  let Some(debug_level) = debug_level {
                                               let level = LoggingLevel::from(debug_level.as_str());
                                               setup_logging(&level);
                                           }
                                        }
                                    }
                                }
                            }

                            if let Err(err) = server.handle_message(ctx,message) {
                                log::error!("handle_message failed: {}", err);
                            }
                        }
                        Err(err) => {
                            log::error!("Failed to parse JSONRPCMessage: {}", err);
                        }
                    }
                }
            });

        self.disruptor = Some(disruptor);


        //start job manager
        let mut notify_rx = self.notify.clone_rx();
        let server = self.clone();
        let job_manager = self.job_manager.clone();
        std::thread::spawn(move || {
            loop{
                //server.job_manager.start();
                let jobs = job_manager.lock().unwrap().borrow_mut().polling();

                match jobs {
                    Ok(gn) => {
                        for payload in gn {
                            match &payload.1 {
                               LoadType::Text => {
                                    let tool_result = CallToolResult {
                                        is_error: Some(false),
                                        content: vec![ToolResultContent::Text(TextContent {
                                            r#type: "text".to_string(),
                                            text: payload.2.data.unwrap(),
                                            annotations: None,
                                        })],
                                    };
                                    let response = JSONRPCResponse::new(
                                        payload.0,
                                        serde_json::to_value(tool_result).map_err(MCPError::Serialization).unwrap(),
                                    );
                                    let response = serde_json::to_string(&response).map_err(MCPError::Serialization).unwrap();

                                    server.handle_outbound(Some(PayLoad {
                                        data: Some(response),
                                        ctx: payload.2.ctx,
                                    })).expect("panic");
                                }
                               LoadType::Audio => {
                                   let tool_result = CallToolResult {
                                       is_error: Some(false),
                                       content: vec![ToolResultContent::Audio(AudioContent {
                                           r#type: "audio".to_string(),
                                           data: payload.2.data.unwrap(),
                                           annotations: None,
                                           mime_type: "audio/mpeg".to_string(),
                                       })],
                                   };
                                   let response = JSONRPCResponse::new(
                                       payload.0,
                                       serde_json::to_value(tool_result).map_err(MCPError::Serialization).unwrap(),
                                   );
                                   let response = serde_json::to_string(&response).map_err(MCPError::Serialization).unwrap();

                                   server.handle_outbound(Some(PayLoad {
                                       data: Some(response),
                                       ctx: payload.2.ctx,
                                   })).expect("panic");
                               }
                               LoadType::Image => {
                                   let tool_result = CallToolResult {
                                       is_error: Some(false),
                                       content: vec![ToolResultContent::Image(ImageContent {
                                           r#type: "image".to_string(),
                                           data: payload.2.data.unwrap(),
                                           annotations: None,
                                           mime_type: "image/png".to_string(),
                                       })],
                                   };
                                   let response = JSONRPCResponse::new(
                                       payload.0,
                                       serde_json::to_value(tool_result).map_err(MCPError::Serialization).unwrap(),
                                   );
                                   let response = serde_json::to_string(&response).map_err(MCPError::Serialization).unwrap();

                                   server.handle_outbound(Some(PayLoad {
                                       data: Some(response),
                                       ctx: payload.2.ctx,
                                   })).expect("panic");
                               }
                               LoadType::Embedded => {
                                   let content = payload.2.data.unwrap();
                                   let result  =  serde_json::from_str::<ResourceContents>(&content);
                                   if  let Err(err) = result {
                                       return Err(err);
                                   }

                                   let resource = result.unwrap();
                                   let tool_result = CallToolResult {
                                       is_error: Some(false),
                                       content: vec![ToolResultContent::Resource(EmbeddedResource {
                                           r#type: "resource".to_string(),
                                           annotations: None,
                                           resource,
                                       })],
                                   };

                                   let response = JSONRPCResponse::new(
                                       payload.0,
                                       serde_json::to_value(tool_result).map_err(MCPError::Serialization).unwrap(),
                                   );
                                   let response = serde_json::to_string(&response).map_err(MCPError::Serialization).unwrap();

                                   server.handle_outbound(Some(PayLoad {
                                       data: Some(response),
                                       ctx: payload.2.ctx,
                                   })).expect("panic");
                               }
                            }
                        }
                    },
                    Err(e) => {
                        log::error!("Error polling job manager: {}", e);
                    }
                }

                match &mut notify_rx {
                    Ok(reader) => {
                        let event  = reader.try_recv();
                        if let Ok(_) = event {
                            break Ok(());
                        }
                    }
                    Err(_) => {}
                }
            }
        });

        Ok(())
    }

    pub fn send_log(&self,level: LoggingLevel, message: &str) {
        //get current session
        let mut max_level = LoggingLevel::Info;
        if let Some(ref mut s) = Self::current_session() {
            let s = s.get_item("debug_level");
            if let Some(s) = s {
                max_level = LoggingLevel::from(s.as_str());
            }
        }else {
            return;
        }

        if level <  max_level {
            return;
        }

        let log_message = LoggingMessageNotification::new(LoggingMessageParams{
            level,
            logger: Some("Mcp Server 1.0".to_string()),
            data: json!(message),
        });

        let notify = ServerNotification::LoggingMessageNotification(log_message);
        let notify = build_server_notification(notify);
        let notify = serde_json::to_string(&notify).map_err(MCPError::Serialization);
        if  let Err(_) = notify {
            return;
        }
        let  notify = notify.unwrap();
        if let Err(_) = self.handle_outbound(Some(rioc::PayLoad {
            data: Some(notify),
            ctx: None,
        })) {}
    }

    pub fn info(&self, message: &str) {
        self.send_log(LoggingLevel::Info, message);
    }

    pub fn debug(&self, message: &str) {
        self.send_log(LoggingLevel::Debug, message);
    }

    pub fn error(&self, message: &str) {
        self.send_log(LoggingLevel::Error, message);
    }

    pub fn warning(&self, message: &str) {
        self.send_log(LoggingLevel::Warning, message);
    }

    pub fn notice(&self, message: &str) {
        self.send_log(LoggingLevel::Warning, message);
    }

    pub fn critical(&self, message: &str) {
        self.send_log(LoggingLevel::Critical, message);
    }

    pub fn emergency(&self, message: &str) {
        self.send_log(LoggingLevel::Emergency, message);
    }

    pub fn alert(&self, message: &str) {
        self.send_log(LoggingLevel::Alert, message);
    }

    pub fn stop(&self) -> Result<(), MCPError> {
        let tx = self.notify.clone_tx();
        tx.unwrap().publish(|e|{
            *e = 1;
        });
        Ok(())
    }

    fn publish(&self, message: PayLoad) {
        self.disruptor.clone().unwrap().publish(|e| {
            e.data = message.data;
            e.ctx = message.ctx;
        });
    }

    fn handle_outbound(&self, message: Option<rioc::PayLoad>) -> Result<(), String> {
        self.chain.with_read(|layer| {
            let _ = layer.handle_outbound(message);
        });
        Ok(())
    }

    fn handle_inbound(&self) -> Result<(), String> {
        // let mut result: Result<LayerResult,String> = Err("".to_string());
        self.chain.with_read(|layer| {
            let _ = layer.handle_inbound(None);
        });
        Ok(())
    }

    fn handle_message(&mut self, ctx: Option<ChainContext> ,message: JSONRPCMessage) -> Result<(), MCPError> {
        match message {
            JSONRPCMessage::Request(req) => {
                let id = req.id.clone();
                let method = req.method.clone();
                let params = req.params.clone();

                //restore session id if exist
                let mut session_id = "local".to_string();
                if let Some(ref ctx) = ctx {
                    if let Some(session) = ctx.data.get(SESSION_ID_KEY) {
                        session_id = session.to_string();
                        set_session_id(session_id.clone());
                    }
                }else{
                    set_session_id(session_id.clone());
                }

                match method.as_str() {
                    "initialize" => {
                        info!("Received initialize request");
                        if let Err(e) = self.handle_initialize(id.clone(), params) {
                            log::error!("Failed to handle initialize request: {}", e);
                            self.response_with_error(id,error_codes::INVALID_REQUEST, "Failed to handle initialize request".to_string(),None);
                        }
                        //create and store the session
                        let mut session_id = "local".to_string();
                        if let Some(ctx) = ctx {
                            if let Some(session) = ctx.data.get(SESSION_ID_KEY) {
                                session_id = session.to_string();
                                SESSION_STORE.create_session(session_id, 60*30);
                            }
                        }else {
                            SESSION_STORE.create_session(session_id, 60*30);
                        } 
                    }
                    "ping" => {
                        info!("Received ping request");
                        if let Err(e) = self.handle_ping(id, params) {
                            log::error!("Failed to handle ping request: {}", e);
                        }
                    }
                    "tools/list" => {
                        info!("Received tools/list request");
                        if let Err(e) = self.check_state(id.clone()) {
                            log::error!("Failed to check state: {}", e);
                            self.response_with_error(id,error_codes::INVALID_REQUEST, "Cannot list tools at current state.  Please initialize the session first".to_string(),None);
                        
                            return Err(e);
                        }

                        if let Err(e) = self.handle_list_tools(id.clone(), params) {
                            log::error!("Failed to handle tools/list request: {}", e);
                            self.response_with_error(id,error_codes::INVALID_REQUEST, "Failed to list tools".to_string(),None);
                        }
                    }
                    "tools/call" => {
                        info!("Received tools/call request");
                        self.info("begin handle log");
                        if let Err(e) = self.check_state(id.clone()) {
                            log::error!("Failed to check state: {}", e);
                            self.response_with_error(id,error_codes::INVALID_REQUEST, "Cannot list tools at current state.  Please initialize the session first".to_string(),None);
                            return Err(e);
                        }

                        if let Err(e) = self.handle_tool_call(ctx,id.clone(), params) {
                            log::error!("Failed to handle tools/call request: {}", e);
                            self.response_with_error(id,error_codes::INVALID_REQUEST, "Failed to call tool".to_string(),None);
                        }
                    }
                    "shutdown" => {
                        info!("Received shutdown request");
                        if let Err(e) = self.check_state(id.clone()) {
                            log::error!("Failed to check state: {}", e);
                            return Err(e);
                        }

                        if let Err(e) = self.handle_shutdown(id, params) {
                            log::error!("Failed to handle shutdown request: {}", e);
                        }
                        let tx = self.notify.clone_tx();

                        if let Ok(mut tx) = tx {
                            tx.publish(|e| *e = 1);
                        }
                    }
                    "logging/setLevel" => {
                        info!("Received logging/setLevel request");
                        if let Err(e) = self.check_state(id.clone()) {
                            log::error!("Failed to check state: {}", e);
                            self.response_with_error(id,error_codes::INVALID_REQUEST, "Cannot set logging level at current state.  Please initialize the session first".to_string(),None);
                            return Err(e);
                        }

                        if let Err(e) = self.handle_set_level(id,session_id, params) {
                            log::error!("Failed to handle logging/setLevel request: {}", e);
                        }
                    }
                    _ => {
                        info!("Received unsupported method: {}", method);
                        if let Err(e) = self.handle_unsupported(id.clone(), params) {
                            log::error!("Failed to handle unsupported method: {}", e);
                            self.response_with_error(id,error_codes::INVALID_REQUEST, "Unsupported method".to_string(),None);
                        }
                    }
                }
            }
            JSONRPCMessage::Notification(notify) => {
                let method = notify.method.clone();
                let params = notify.params.clone();
                match method.as_str() {
                    "notifications/cancelled" => {
                        info!("Received notifications/cancelled request");
                        if let Err(e) = self.handle_cancelled(params) {
                            log::error!("Failed to handle notifications/cancelled request: {}", e);
                        }
                    }
                    "notifications/roots/list_changed" => {
                        info!("Received notifications/roots/list_changed request");
                        if let Err(e) = self.handle_roots_list_changed(params) {
                            log::error!(
                                "Failed to handle notifications/roots/list_changed request: {}",
                                e
                            );
                        }
                    }
                    "notifications/initialized" => {
                        info!("Received notifications/initialized request");
                        if let Err(e) = self.handle_initialize_notification(params) {
                            log::error!(
                                "Failed to handle notifications/initialized request: {}",
                                e
                            );
                        }
                    }
                    "notifications/progress" => {
                        info!("Received notifications/progress request");
                        if let Err(e) = self.handle_progress_notification(params) {
                            log::error!("Failed to handle notifications/progress request: {}", e);
                        }
                    }
                    _ => {
                        info!("Received unsupported method: {}", method);
                    }
                }
            }
            JSONRPCMessage::Error(_) => {
                self.cache_response(message);
            }
            JSONRPCMessage::Response(_) => {
                self.cache_response(message);
            }
        }

        Ok(())
    }


    fn check_state(&self,id: RequestId) -> Result<(), MCPError> {
        if self.state != ServerState::Running {
            //send error response
            let error = JSONRPCMessage::Error(JSONRPCError::new_with_details(
                id.clone(),
                error_codes::INVALID_REQUEST,
                "Server not initialized".to_string(),
                None,
            ));

            let response = JSONRPCResponse::new(id, mcp_param(&error).unwrap());
            //handle outbound
            let response = serde_json::to_string(&response).map_err(MCPError::Serialization)?;
            if let Err(e) = self.handle_outbound(Some(rioc::PayLoad {
                data: Some(response),
                ctx: None,
            })) {
                log::error!("Failed to send error response: {}", e);
            }


            return Err(MCPError::Transport(
                "Server not initialized".to_string(),
            ));
        }
        Ok(())
    }

    fn handle_initialize(&mut self, id: RequestId, params: Option<Value>) -> Result<(), MCPError> {
        let mut client_params = None;
        if let Some(params) = params {
            client_params = mcp_from_value::<InitializeParams>(params);
        }

        info!("Received initialize params: {:?}", client_params);

        let capabilities = ServerCapabilities {
            experimental: None,
            logging: Some(Value::Bool(false)),
            prompts: None,
            resources: None,
            tools: if !self.config.tools.is_empty() {
                Some(ToolsCapability {
                    list_changed: Some(false),
                })
            } else {
                None
            },
        };

        let server_info = Implementation {
            name: self.config.name.clone(),
            version: self.config.version.clone(),
        };

        //just use server capabilities
        let init_result = InitializeResult {
            protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
            capabilities,
            server_info,
            instructions: None,
        };

        let response = JSONRPCResponse::new(id, mcp_to_value(init_result)?);

        //handle outbound
        let response = serde_json::to_string(&response).map_err(MCPError::Serialization)?;
        if let Err(e) = self.handle_outbound(Some(rioc::PayLoad {
            data: Some(response),
            ctx: None,
        })) {
            log::error!("Failed to send initialize response: {}", e);
        }

        self.state = ServerState::Initialized;

        Ok(())
    }

    fn handle_list_tools(&self, id: RequestId, _params: Option<Value>) -> Result<(), MCPError> {
        let tools_list = ListToolsResult {
            next_cursor: None,
            tools: self.config.tools.clone(),
        };

        let response = JSONRPCResponse::new(id, mcp_to_value(tools_list)?);

        //handle outbound
        let response = serde_json::to_string(&response).map_err(MCPError::Serialization)?;
        if let Err(e) = self.handle_outbound(Some(rioc::PayLoad {
            data: Some(response),
            ctx: None,
        })) {
            log::error!("Failed to send list tools response: {}", e);
        }

        Ok(())
    }

    fn handle_tool_call(&mut self, ctx: Option<ChainContext>, id: RequestId, params: Option<Value>) -> Result<(), MCPError> {
        let params = params.ok_or_else(|| {
            MCPError::Transport("Missing parameters in tools/call request".to_string())
        })?;

        //parse the parameters as CallToolParams
        let call_params: CallToolParams = serde_json::from_value(params.clone())
            .map_err(|e| MCPError::Transport(format!("Invalid tools/call parameters: {}", e)))?;

        //get the tool by name
        let tool_name = call_params.name.clone();

        //convert arguments to JSON value if exists,otherwise use null
        let tool_params = match call_params.arguments {
            Some(args) => serde_json::to_value(args).unwrap_or(Value::Null),
            None => Value::Null,
        };

        let result = self.execute_tool(tool_name, tool_params);
        match result {
            Ok(job) => {
                self.job_manager.lock().unwrap().borrow_mut().add_job(id,(ctx,job))
            }
            Err(e) => {
                let error = JSONRPCMessage::Error(JSONRPCError::new_with_details(
                    id,
                    -32000,
                    format!("Tool execution failed: {}", e),
                    None,
                ));

                let error = serde_json::to_string(&error).map_err(MCPError::Serialization)?;
                if let Err(e) = self.handle_outbound(Some(rioc::PayLoad {
                    data: Some(error),
                    ctx: None,
                })) {
                    log::error!("Failed to send tool call error response: {}", e);
                }
            }
        }

        Ok(())
    }

    fn handle_shutdown(&self, id: RequestId, _params: Option<Value>) -> Result<(), MCPError> {
        let response = JSONRPCResponse::new(id, serde_json::json!({}));

        //handle outbound
        let response = serde_json::to_string(&response).map_err(MCPError::Serialization)?;
        if let Err(e) = self.handle_outbound(Some(rioc::PayLoad {
            data: Some(response),
            ctx: None,
        })) {
            log::error!("Failed to send shutdown response: {}", e);
        }

        Ok(())
    }

    fn handle_ping(&self, id: RequestId, _params: Option<Value>) -> Result<(), MCPError> {
        //get the current time as string
        let timestamp = Utc::now().to_rfc3339();
        let extra = DashMap::new();
        extra.insert("timestamp".to_string(), mcp_to_value(timestamp).unwrap());
        let result = EmptyResult {
            _meta: None,
            extra: Some(extra),
        };

        let response = JSONRPCResponse::new(id, serde_json::json!(result));

        //handle outbound
        let response = serde_json::to_string(&response).map_err(MCPError::Serialization)?;
        if let Err(e) = self.handle_outbound(Some(rioc::PayLoad {
            data: Some(response),
            ctx: None,
        })) {
            log::error!("Failed to send ping response: {}", e);
        }

        Ok(())
    }

    fn handle_unsupported(
        &self,
        id: RequestId,
        _params: Option<Value>,
    ) -> Result<(), MCPError> {
        info!("Received unsupported method : {:?}", id);
        Ok(())
    }

    fn execute_tool(&self, tool: String, params: Value) -> Result<JobTask<(LoadType,String),i32,String>, MCPError> {
        let handlers = self.tool_handlers.lock().unwrap();
        if let Some(handler) = handlers.get(&tool).cloned() {
            let job: JobTask<(LoadType,String),i32,String> = JobTask::new(params,move |params,sender,receiver| {
                let _result = handler(params,sender,receiver);
            });

            return Ok(job);
        } else {
            return Err(MCPError::Transport(format!(
                "No handler found for tool: {}",
                tool
            )));
        }
    }

    pub fn add_transport_layer(&mut self, layer: SharedLayer) {
        self.chain.with(|chain| {
            chain.add_layer(layer);
        })
    }

    pub fn add_protocol_layer(&mut self, layer: SharedLayer) {
        self.chain.with(|chain| {
            chain.add_layer(layer);
        })
    }

    pub fn build(&mut self) {
        let server_cloned = self.clone();
        let builder = rioc::LayerBuilder::new();

        let layer = builder
            .with_inbound_fn(move |req| {
                log::info!("Received request: {:?}", req);
                //call back to server to handle the request
                if let Some(data) = req {
                    server_cloned.publish(data);
                }
                Ok(rioc::LayerResult {
                    direction: rioc::Direction::Inbound,
                    data: None,
                })
            })
            .with_outbound_fn(move |req| {
                log::info!("Received response: {:?}", req);
                Ok(LayerResult {
                    direction: rioc::Direction::Outbound,
                    data: req,
                })
            })
            .build();

        self.chain.with(|chain| {
            chain.add_layer(layer.unwrap());
        });
    }

    fn handle_cancelled(&self, params: Option<Value>) -> Result<Value, MCPError> {
        let params = params.ok_or_else(|| {
            MCPError::Transport("Missing parameters in tools/call request".to_string())
        })?;

        //parse the parameters as CallToolParams
        let call_params: CancelledParams = serde_json::from_value(params.clone())
            .map_err(|e| MCPError::Transport(format!("Invalid Cancellation parameters: {}", e)))?;

        let request_id = call_params.request_id;
        self.job_manager.lock().unwrap().borrow_mut().cancel_job(request_id);

        Ok(Value::Null)
    }

    fn handle_roots_list_changed(&self, _params: Option<Value>) -> Result<Value, MCPError> {
        Ok(Value::Null)
    }

    fn handle_initialize_notification(&mut self, params: Option<Value>) -> Result<Value, MCPError> {
        info!("Received initialize notification: {:?}", params);
        self.state = ServerState::Running;
        Ok(Value::Null)
    }

    fn handle_progress_notification(&mut self, params: Option<Value>) -> Result<Value, MCPError> {
        info!("Received progress notification: {:?}", params);
        Ok(Value::Null)
    }
    
    fn handle_set_level(&self, id: RequestId, session_id: String,params: Option<Value>) -> Result<Value, MCPError> {
        info!("Received set level request: req: {:?} {:?}",id, params);

        let params = params.ok_or_else(|| {
            MCPError::Transport("Missing parameters in logging/setLevel request".to_string())
        })?;

        //parse the parameters as CallToolParams
        let params: SetLevelParams = serde_json::from_value(params.clone())
            .map_err(|e| MCPError::Transport(format!("Invalid set level parameters: {}", e)))?;
        let level = params.level;
        setup_logging(&level);

        //get session id
        let s = SESSION_STORE.get_session(&session_id);
        if let Some(mut s) = s {
            s.set_item("debug_level".to_string(),level.to_string());
        }
        
        //response empty 
        let result = EmptyResult::new();
        let response = JSONRPCResponse::new(id, serde_json::json!(result));

        //handle outbound
        let response = serde_json::to_string(&response).map_err(MCPError::Serialization)?;
        if let Err(e) = self.handle_outbound(Some(rioc::PayLoad {
            data: Some(response),
            ctx: None,
        })) {
            log::error!("Failed to send empty response: {}", e);
        }


        Ok(Value::Null)
    }

    fn response_with_error(&self,
                           id: RequestId,
                           code: i32,
                           message: String,
                           data: Option<Value>){
        let error = build_server_error(id,code,message, data);
        let error = serde_json::to_string(&error).map_err(MCPError::Serialization);
        if  let Err(_) = error {
            return;
        }
        let  error = error.unwrap();
        if let Err(_) = self.handle_outbound(Some(rioc::PayLoad {
            data: Some(error),
            ctx: None,
        })) {}
    }
    pub fn current_session() -> Option<SessionItem> {
        let sid = get_current_session();
        let s = SESSION_STORE.get_session(&sid);
        return s;
    }
}
