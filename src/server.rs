use std::{collections::HashMap, sync::{Arc, Mutex}, time::Duration};
use disruptor::{Producer, Sequence};
use ibag::iBag;
use log::info;
use rioc::{LayerChain, LayerResult, PayLoad, SharedLayer};
use serde_json::Value;

use crate::{schema::schema::{CallToolParams, CallToolResult, Implementation, InitializeResult, JSONRPCError, JSONRPCMessage, JSONRPCResponse, ListToolsResult, RequestId, ServerCapabilities, TextContent, Tool, ToolInputSchema, ToolResultContent, ToolsCapability, LATEST_PROTOCOL_VERSION}, support::{definition::McpLayer, disruptor::{DisruptorFactory, DisruptorWriter}, ControlBus}, transport::stdio::StdioTransport, MCPError};



#[derive(Clone)]
pub struct ServerConfig {
    pub name: String,
    pub version: String,
    pub tools: Vec<Tool>,
    pub timeout: Option<Duration>
}

impl ServerConfig {
    pub fn new() -> Self {
        Self {
            name: "MCP Server".to_string(),
            version: "1.0.0".to_string(),
            tools: Vec::new(),
            timeout: None
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

pub type ToolHandler = Box<dyn Fn(Value)->Result<Value,MCPError> + Send>;

#[derive(Clone)]
pub struct Server {
    config: ServerConfig,
    tool_handlers: Arc<Mutex<HashMap<String, ToolHandler>>>,
    notify: Arc<ControlBus>,
    chain: iBag<LayerChain>,
    disruptor: Option<DisruptorWriter>,
    is_initialized: bool,
}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            tool_handlers: Arc::new(Mutex::new(HashMap::new())),
            notify: Arc::new(ControlBus::new()),
            chain: iBag::new(LayerChain::new()),
            disruptor: None,
            is_initialized: false,
       }
    }

    pub fn register_tool_handler<F>(&self, tool_name: String, handler: F) -> Result<(), MCPError>
    where F: Fn(Value) -> Result<Value, MCPError> + Send + Sync + 'static,
    {
        //check if the tool exists
        if !self.config.tools.iter().any(|tool| tool.name == tool_name)  {
           return Err(MCPError::Transport(format!("Tool {} not found in server config", tool_name)));
        }

        //register the tool handler
        let mut handlers = match self.tool_handlers.try_lock(){
            Ok(handlers) => handlers,
            Err(_) => return Err(MCPError::Transport("Failed to lock tool handlers".to_string())),
        };

        let handler = Box::new(handler);
        handlers.insert(tool_name, handler );

        Ok(())  
    }


    pub fn start(&mut self) -> Result<(), MCPError> {
        if self.is_initialized {
            return Err(MCPError::Transport("Server already initialized".to_string()));
        }
        self.is_initialized = true;

        let server = self.clone();
        let disruptor = DisruptorFactory::create(move |e: &PayLoad, _seq: Sequence, _end_of_patch: bool| {
            if let Some(data) = &e.data {
                info!("Received message: {:?}", data);
                match serde_json::from_str::<JSONRPCMessage>(&data) {
                    Ok(message) => {
                        if let Err(err) = server.handle_message(message) {
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
        Ok(())
    }

    pub fn publish(&self, message: PayLoad) {
        self.disruptor.clone().unwrap().publish(|e| {
            e.data = message.data;
        });
    }

    pub fn handle_outbound(&self, message: Option<rioc::PayLoad>) -> Result<(),String>{
        self.chain.with_read(|layer|{
           let _ = layer.handle_outbound(message);
        });
        Ok(())
    }

    pub fn handle_inbound(&self) -> Result<(),String> {
        // let mut result: Result<LayerResult,String> = Err("".to_string());
        self.chain.with_read(|layer|{
           let _ = layer.handle_inbound(None);
        });
        Ok(())
    }

    pub fn handle_message(&self, message: JSONRPCMessage) -> Result<(), MCPError> {
        match message {
            JSONRPCMessage::Request(req) => {
                let id = req.id.clone();
                let method = req.method.clone();
                let params = req.params.clone();

                match method.as_str() {
                    "initialize" => {
                        info!("Received initialize request");
                        if let Err(e) = self.handle_initialize(id, params) {
                            log::error!("Failed to handle initialize request: {}", e);
                        }
                    }
                    "tools/list" => {
                        info!("Received tools/list request");
                        if let Err(e) = self.handle_list_tools(id, params) {
                            log::error!("Failed to handle tools/list request: {}", e);
                        }
                    }
                    "tools/call" => {
                        info!("Received tools/call request");
                        if let Err(e) = self.handle_tool_call(id, params) {
                            log::error!("Failed to handle tools/call request: {}", e);
                        }
                    }
                    "shutdown" => {
                        info!("Received shutdown request");
                        if let Err(e) = self.handle_shutdown(id, params)  {
                            log::error!("Failed to handle shutdown request: {}", e);
                        }
                        let mut tx = self.notify.clone_tx();

                        if let Ok(mut tx) = tx {
                            tx.publish(|e| *e = 1);
                        }
                    }
                    _ => {
                        info!("Received unsupported method: {}", method);
                        if let Err(e) = self.handle_unsupported(id, params) {
                            log::error!("Failed to handle unsupported method: {}", e);
                        }
                    }
                }
            }
            _ => {
                info!("Unsupported message type: {:?}", message);
            }
        }

        Ok(())
    }

    pub fn handle_initialize(&self, id: RequestId, params: Option<Value>) -> Result<(), MCPError> {
        let capabilities =  ServerCapabilities {
            experimental: None,
            logging: None,
            prompts: None,
            resources: None,
            tools: if !self.config.tools.is_empty() {
                Some(ToolsCapability{
                    list_changed: Some(false),
                })
            }else{
                None
            },
        };


        let server_info = Implementation {
            name: self.config.name.clone(),
            version: self.config.version.clone(),
        };


        let init_result = InitializeResult {
            protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
            capabilities,
            server_info,
            instructions: None,
        };


        let response = JSONRPCResponse::new(
            id,
            serde_json::to_value(init_result).map_err(MCPError::Serialization)?,
        );

        //handle outbound
        let response = serde_json::to_string(&response).map_err(MCPError::Serialization)?;
        if let Err(e) = self.handle_outbound(Some(rioc::PayLoad{
            data: Some(response),
            ctx: None,
        })) {
            log::error!("Failed to send initialize response: {}", e);
        }

        Ok(())
    }
    
    pub fn handle_list_tools(&self, id: RequestId, params: Option<Value>) -> Result<(), MCPError> {
        let tools_list = ListToolsResult {
            next_cursor: None,
            tools: self.config.tools.clone(),
        };

        let response = JSONRPCResponse::new(
            id,
            serde_json::to_value(tools_list).map_err(MCPError::Serialization)?,
        );

        //handle outbound
        let response = serde_json::to_string(&response).map_err(MCPError::Serialization)?;
        if let Err(e) = self.handle_outbound(Some(rioc::PayLoad{
            data: Some(response),
            ctx: None,
        })) {
            log::error!("Failed to send list tools response: {}", e);
        }

        Ok(())
    }
    
    pub fn handle_tool_call(&self, id: RequestId, params: Option<Value>) -> Result<(), MCPError> {
        let params = params.ok_or_else(|| {
            MCPError::Transport("Missing parameters in tools/call request".to_string())
        })?;

        //parse the parameters as CallToolParams
        let call_params: CallToolParams = serde_json::from_value(params.clone()).map_err(|e| {
            MCPError::Transport(format!("Invalid tools/call parameters: {}", e))
        })?;

        //get the tool by name
        let tool_name = call_params.name.clone();

        //convert arguments to JSON value if exists,otherwise use null
        let tool_params = match call_params.arguments {
            Some(args) => serde_json::to_value(args).unwrap_or(Value::Null),
            None => Value::Null,
        };

        let result = self.execute_tool(tool_name, tool_params);
        match result {
            Ok(result) => {
                //send the result back to the client
                let tool_result = CallToolResult{
                    is_error: Some(false),
                    content: vec![ToolResultContent::Text(
                        TextContent{
                            r#type: "text".to_string(),
                            text: serde_json::to_string_pretty(&result).unwrap_or_else(|_|format!("{:?}", result)),
                            annotations: None,
                        }
                    )]
                };


                let response = JSONRPCResponse::new(
                    id,
                    serde_json::to_value(tool_result).map_err(MCPError::Serialization)?,
                );
                let response = serde_json::to_string(&response).map_err(MCPError::Serialization)?;
                if let Err(e) = self.handle_outbound(Some(rioc::PayLoad{
                    data: Some(response),
                    ctx: None,
                })) {
                    log::error!("Failed to send tool call response: {}", e);
                }
            }
            Err(e) => {
                let error = JSONRPCMessage::Error(
                    JSONRPCError::new_with_details(
                        id, 
                        -32000, 
                        format!("Tool execution failed: {}", e), 
                        None)
                );
                
                let error = serde_json::to_string(&error).map_err(MCPError::Serialization)?;
                if let Err(e) = self.handle_outbound(Some(rioc::PayLoad{
                    data: Some(error),
                    ctx: None,
                })) {
                    log::error!("Failed to send tool call error response: {}", e);
                }
            }
        }

        Ok(())
    }
    
    pub fn handle_shutdown(&self, id: RequestId, _params: Option<Value>) -> Result<(), MCPError> {
        let response = JSONRPCResponse::new(
            id,
            serde_json::json!({}),
        );

        //handle outbound
        let response = serde_json::to_string(&response).map_err(MCPError::Serialization)?;
        if let Err(e) = self.handle_outbound(Some(rioc::PayLoad{
            data: Some(response),
            ctx: None,
        })) {
            log::error!("Failed to send shutdown response: {}", e);
        }

        Ok(())
    }
    
    pub fn handle_unsupported(&self, id: RequestId, _params: Option<Value>) -> Result<(), MCPError> {
        info!("Received unsupported method : {:?}", id);
        Ok(())
    }

    pub fn execute_tool(&self, tool: String, params: Value) -> Result<Value, MCPError>  {
        let handlers = self.tool_handlers.lock().unwrap();
        if let Some(handler) = handlers.get(&tool) {
            let result = handler(params);
            return result;
        } else {
            return Err(MCPError::Transport(format!("No handler found for tool: {}", tool)));
        }
    }

    pub fn add_transport_layer(&mut self, layer: SharedLayer) {
        self.chain.with(|chain|{
                chain.add_layer(layer);
            }
        )
    }

    pub fn add_protocol_layer(&mut self, layer: SharedLayer) {
        self.chain.with(|chain|{
                chain.add_layer(layer);
            }
        )
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
                Ok(LayerResult{
                    direction: rioc::Direction::Outbound,
                    data: req,
                })
            })
            .build();

        self.chain.with(|chain|{
            chain.add_layer(layer.unwrap());
        });
    }
}



#[test]
fn test_server() {
    crate::init_log();
    let config = ServerConfig::new()
        .with_name("MCP Server")
        .with_version("1.0.0")
        .with_tools(Tool {
            name: "test_tool".to_string(),
            input_schema: ToolInputSchema{
                r#type: "object".to_string(),
                properties: None,
                required: None,
            },
            description: None,
        });

    let mut server = Server::new(config);
    let _ = server.register_tool_handler("test_tool".to_string(), |params| {
        Ok(Value::String("Hello from test tool".to_string()))
    });
       
    //build stdio as transport layer
    let stdio = StdioTransport::new("abc", true);
    let layer0 = stdio.create();
    server.add_transport_layer(layer0);
    let _ =  server.start();   

    server.build();

    let _ = server.handle_inbound();
}