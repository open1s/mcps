use disruptor::{Producer, Sequence};
use ibag::iBag;
use log::info;
use rioc::{LayerChain, LayerResult, PayLoad, SharedLayer};
use serde_json::Value;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::schema::{client::{build_client_notification, build_client_request}, schema::{LoggingLevel, SetLevelParams, SetLevelRequest}};
use crate::schema::json_rpc::mcp_json_param;
use crate::schema::schema::{
    CallToolParams, CallToolRequest, ClientNotification, Cursor, InitializedNotification,
    InitializedNotificationParams, ListToolsRequest, PaginatedParams,
};
use crate::{
    schema::schema::{
            CallToolResult, CancelledNotification, CancelledParams, ClientCapabilities,
            ClientRequest, ClientShutdownRequest, Implementation, InitializeParams,
            InitializeRequest, JSONRPCMessage, PingRequest,
            RequestId, RootsCapability, LATEST_PROTOCOL_VERSION,
        },
    support::{
        disruptor::{DisruptorFactory, DisruptorWriter},
    },
    MCPError,
};

pub trait ClientProvider {
    fn client_ping_response(&self, id: RequestId, _params: Option<Value>) -> Result<(), MCPError>;
    fn client_list_roots(&self, id: RequestId, _params: Option<Value>) -> Result<(), MCPError>;
    fn client_sampling_message(&self, id: RequestId, _params: Option<Value>) -> Result<(), MCPError>;
    fn client_logs(&self,params: Option<Value>) -> Result<(), MCPError>;
}


#[derive(Clone)]
pub struct Client<T: ClientProvider + Default + Clone + Send + 'static> {
    is_initialized: bool,
    next_request_id: i64,
    timeout_duration: Option<Duration>,
    chain: iBag<LayerChain>,
    disruptor: Option<DisruptorWriter>,
    cached: Arc<Mutex<Vec<JSONRPCMessage>>>,
    current_request_id: Option<i64>,
    provider: T,
}

impl <T: ClientProvider + Default + Clone + Send + 'static> Client<T> {
    pub fn new() -> Self {
        Self {
            is_initialized: false,
            next_request_id: 0,
            timeout_duration: None,
            chain: iBag::new(LayerChain::new()),
            disruptor: None,
            cached: Arc::new(Mutex::new(Vec::new())),
            current_request_id: None,
            provider: T::default(),
        }
    }

    fn cached_response(&self, response: JSONRPCMessage) -> Result<(), MCPError> {
        self.cached.lock().unwrap().push(response);
        Ok(())
    }

    fn pop_response(&self) -> Option<JSONRPCMessage> {
        self.cached.lock().unwrap().pop()
    }

    pub fn serve(&self) -> Result<(), MCPError> {
        let _ = self.handle_inbound();
        Ok(())
    } 

    pub fn start(&mut self) -> Result<(), MCPError> {
        if self.is_initialized {
            return Err(MCPError::Transport(
                "Client already initialized".to_string(),
            ));
        }
        self.is_initialized = true;

        let mut client = self.clone();
        let disruptor =
            DisruptorFactory::create(move |e: &PayLoad, _seq: Sequence, _end_of_patch: bool| {
                if let Some(data) = &e.data {
                    info!("Client Received message: {:?}", data);
                    match serde_json::from_str::<JSONRPCMessage>(&data) {
                        Ok(message) => {
                            if let Err(err) = client.handle_message(message) {
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

    fn publish(&self, message: PayLoad) {
        self.disruptor.clone().unwrap().publish(|e| {
            *e = message;
        });
    }

    fn handle_message(&mut self, response: JSONRPCMessage) -> Result<(), MCPError> {
        match &response {
            JSONRPCMessage::Response(_) => {
                let _ = self.cached_response(response);
                Ok(())
            }
            JSONRPCMessage::Request(req) => {
                let id = req.id.clone();
                let method = req.method.clone();
                let params = req.params.clone();
                match method.as_str() {
                    "ping" => {
                        info!("Received ping request");
                        self.provider.client_ping_response(id, params)?;
                    }
                    "roots/list" => {
                        info!("Received roots/list request");
                        self.provider.client_list_roots(id, params)?;
                    }
                    "sampling/createMessage" => {
                        info!("Received sampling/createMessage request");
                        self.provider.client_sampling_message(id, params)?;
                    }
                    _ => {
                        info!("Received unsupported method: {}", method);
                        if let Err(e) = self.handle_unsupported(id, params) {
                            log::error!("Failed to handle unsupported method: {}", e);
                        }
                    }
                }

                Ok(())
            }
            JSONRPCMessage::Notification(params) => {
                let params = params.params.clone();
                let _ = self.provider.client_logs(params);
                Ok(())
            }
            JSONRPCMessage::Error(_) => {
                let _ = self.cached_response(response);
                Ok(())
            }
        }
    }

    fn handle_unsupported(
        &self,
        id: RequestId,
        _params: Option<Value>,
    ) -> Result<(), MCPError> {
        info!("Received unsupported method : {:?}", id);
        Ok(())
    }

    pub fn with_timeout(&mut self, duration: Duration) -> &mut Self {
        self.timeout_duration = Some(duration);
        self
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
                // Wait a bit before trying again
                // Need polling for data, not sleeping
                // This is a hack, but it works for now
                // maybe use wait for notify?
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

    pub fn initialize(&mut self) -> Result<Value, MCPError> {
        let initial_params = InitializeParams {
            protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
            client_info: Implementation {
                name: "MCP Client".to_string(),
                version: "1.0.0".to_string(),
            },
            capabilities: ClientCapabilities {
                experimental: None,
                roots: Some(RootsCapability {
                    list_changed: Some(false),
                }),
                sampling: None,
            },
        };

        let initial_request = InitializeRequest::new(initial_params);
        let client_request = ClientRequest::Initialize(initial_request);

        let request_id = self.next_request_id();
        let req = build_client_request(request_id.clone(), client_request);

        let payload = rioc::PayLoad {
            data: mcp_json_param(&req),
            ctx: None,
        };

        //send initial request to server
        let _ = self.handle_outbound(Some(payload));

        //wait for response
        let response = self.recieve_with_timeout()?;
        match response {
            JSONRPCMessage::Response(response) => {
                //response with notification
                let notification =
                    InitializedNotification::new(InitializedNotificationParams { _meta: None });

                let notification = ClientNotification::Initialized(notification);
                let notify = build_client_notification(notification);
                let payload = rioc::PayLoad {
                    data: mcp_json_param(&notify),
                    ctx: None,
                };
                //send initial request to server
                let _ = self.handle_outbound(Some(payload));

                assert!(response.id == request_id);

                Ok(response.result)
            }
            JSONRPCMessage::Error(error) => Err(MCPError::Protocol(format!("Error: {:?}", error))),
            _ => Err(MCPError::Protocol("Invalid response".to_string())),
        }
    }

    pub fn list_tool(&mut self, cursor: Option<Cursor>) -> Result<Value, MCPError> {
        let list_tool_req = ListToolsRequest::new(Some(PaginatedParams { cursor }));

        let req = ClientRequest::ListTools(list_tool_req);
        let request_id = self.next_request_id();
        let req = build_client_request(request_id.clone(), req);
        let payload = rioc::PayLoad {
            data: mcp_json_param(&req),
            ctx: None,
        };

        //send initial request to server
        let _ = self.handle_outbound(Some(payload));

        //wait for response
        let response = self.recieve_with_timeout()?;
        match response {
            JSONRPCMessage::Response(resp) => {
                assert!(resp.id == request_id);
                Ok(resp.result)
            }
            JSONRPCMessage::Error(error) => Err(MCPError::Protocol(format!("Error: {:?}", error))),
            _ => Err(MCPError::Protocol("Invalid response".to_string())),
        }
    }

    pub fn call_tool(&mut self, params: CallToolParams) -> Result<CallToolResult, MCPError> {
        let call_tool_req = CallToolRequest::new(params);

        let req = ClientRequest::CallTool(call_tool_req);
        let request_id = self.next_request_id();
        let req = build_client_request(request_id.clone(), req);

        let payload = rioc::PayLoad {
            data: mcp_json_param(&req),
            ctx: None,
        };
        //send tool call request to server
        let _ = self.handle_outbound(Some(payload));

        //wait for response
        let response = self.recieve_with_timeout()?;
        match response {
            JSONRPCMessage::Response(resp) => {
                assert_eq!(resp.id, request_id);
                let result = resp.result;
                serde_json::from_value(result.clone()).map_err(MCPError::Serialization)
            }
            JSONRPCMessage::Error(error) => {
                Err(MCPError::Protocol(format!("Tool call failed {:?}", error)))
            }
            _ => Err(MCPError::Protocol("Unexpected response type".to_string())),
        }
    }

    pub fn shutdown(&mut self) -> Result<(), MCPError> {
        let shutdown_req = ClientShutdownRequest::new();

        let req = ClientRequest::Shutdown(shutdown_req);
        let request_id = self.next_request_id();
        let req = build_client_request(request_id.clone(), req);
        let payload = rioc::PayLoad {
            data: Some(serde_json::to_string(&req).unwrap()),
            ctx: None,
        };

        //send initial request to server
        let _ = self.handle_outbound(Some(payload));

        //wait for response
        let response = self.recieve_with_timeout()?;
        match response {
            JSONRPCMessage::Response(resp) => {
                assert_eq!(resp.id, request_id);
                Ok(())
            }
            JSONRPCMessage::Error(error) => Err(MCPError::Protocol(format!("Error: {:?}", error))),
            _ => Err(MCPError::Protocol("Invalid response".to_string())),
        }
    }

    pub fn ping(&mut self) -> Result<(), MCPError> {
        let ping_req = PingRequest::new();

        let request_id = self.next_request_id();
        let req = ClientRequest::Ping(ping_req);
        let req = build_client_request(request_id.clone(), req);
        let payload = rioc::PayLoad {
            data: mcp_json_param(&req),
            ctx: None,
        };

        //send initial request to server
        let _ = self.handle_outbound(Some(payload));

        //wait for response
        let response = self.recieve_with_timeout()?;
        match response {
            JSONRPCMessage::Response(resp) => {
                assert_eq!(resp.id, request_id);
                Ok(())
            }
            JSONRPCMessage::Error(error) => Err(MCPError::Protocol(format!("Error: {:?}", error))),
            _ => Err(MCPError::Protocol("Invalid response".to_string())),
        }
    }

    pub fn cancel(&mut self) -> Result<(), MCPError> {
        let params = CancelledParams {
            request_id: RequestId::Number(self.current_request_id.unwrap()),
            reason: Some("client cancelled".to_string()),
        };

        let cancelled_req = CancelledNotification::new(params);

        let req = ClientNotification::Cancelled(cancelled_req);
        let notify = build_client_notification(req);
        let payload = rioc::PayLoad {
            data: mcp_json_param(&notify),
            ctx: None,
        };

        //send initial request to server
        let _ = self.handle_outbound(Some(payload));
        Ok(())
    }

    fn handle_outbound(&self, message: Option<rioc::PayLoad>) -> Result<(), String> {
        self.chain.with_read(|layer| {
            let _ = layer.handle_outbound(message);
        });
        Ok(())
    }

    fn handle_inbound(&self) -> Result<LayerResult, String> {
        let mut result: Result<LayerResult, String> = Err("Failed to get result".to_string());
        self.chain.with_read(|layer| {
            result = layer.handle_inbound(None);
        });
        result
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

    fn next_request_id(&mut self) -> RequestId {
        self.next_request_id += 1;
        let id = self.next_request_id;
        self.current_request_id = Some(id);
        RequestId::Number(id)
    }

    pub fn set_log_level(&mut self, level: LoggingLevel) {
        let request = SetLevelRequest::new(SetLevelParams{
            level,
        });

        let request_id = self.next_request_id();
        let req = ClientRequest::SetLevel(request);
        let req = build_client_request(request_id.clone(), req);
        let payload = rioc::PayLoad {
            data: mcp_json_param(&req),
            ctx: None,
        };
        //send initial request to server
        let _ = self.handle_outbound(Some(payload));

        //wait for response
        let _ = self.recieve_with_timeout();
    }

    pub fn build(&mut self) {
        let client_cloned = self.clone();
        let builder = rioc::LayerBuilder::new();

        let layer = builder
            .with_inbound_fn(move |req| {
                log::info!("Received request: {:?}", req);
                //call back to server to handle the request
                if let Some(ref data) = req {
                    client_cloned.publish(data.clone());
                    Ok(rioc::LayerResult {
                        direction: rioc::Direction::Inbound,
                        data: None,
                    })
                } else {
                    Err("Failed to get request data".to_string())
                }
            })
            .with_outbound_fn(move |req| {
                log::info!("Sending response: {:?}", req);
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
}

#[cfg(test)]
mod tests {
    use log::warn;
    use rioc::TaskEvent;
    use crate::{
        executor::{ClientExecutor, ServerExecutor},
        init_log,
        schema::schema::{Tool, ToolInputSchema},
        server::{Server, ServerConfig},
        support::definition::McpLayer,
        transport::{stdio, trace},
    };
    use crate::schema::schema::LoggingMessageParams;
    use crate::support::logging::{setup_logging};

    #[derive(Clone, Default)]
    pub struct TestClientService;

    impl ClientProvider for TestClientService {
        fn client_ping_response(&self, _id: RequestId, _params: Option<Value>) -> Result<(), MCPError> {
            Ok(())
        }

        fn client_list_roots(&self, _id: RequestId, _params: Option<Value>) -> Result<(), MCPError> {
            Ok(())
        }

        fn client_sampling_message(&self, _id: RequestId, _params: Option<Value>) -> Result<(), MCPError> {
            Ok(())
        }

        fn client_logs(&self, params: Option<Value>) -> Result<(), MCPError> {
            let params = serde_json::from_value::<LoggingMessageParams>(
                params.unwrap()
            );

            if let Ok(message) = params {
                warn!("Log notification: {:?}", serde_json::to_string(&message.data));
            }
            Ok(())
        }
    }

    use super::*;
    #[test]
    fn test_next_request_id() {
        let mut client = Client::<TestClientService>::new();
        let layer0 = stdio::StdioTransport::new("abc", true).create();

        client.add_transport_layer(layer0);
        client.build();

        let d = client.handle_inbound();
        println!("{:?}", d);
    }

    #[test]
    fn test_client() {
        //init log
        init_log();

        //init dummy server
        let config = ServerConfig::new()
            .with_name("MCP Server")
            .with_version("1.0.0")
            .with_tools(Tool {
                name: "test_tool".to_string(),
                input_schema: ToolInputSchema {
                    r#type: "object".to_string(),
                    properties: None,
                    required: None,
                },
                description: None,
            });

        let mut server = Server::new(config);
        let _ = server.register_tool_handler("test_tool".to_string(), move |_input,sender,_receiver| {
            let _ = sender.send(TaskEvent::Data((crate::schema::schema::LoadType::Text, "hello mcp".to_string())));
            Ok(serde_json::json!({
                "result": "hello mcp client",
            }))
        });

        let server_transport = stdio::StdioTransport::new("abc", true).create();
        server.add_transport_layer(server_transport);
        let _ = server.start();
        server.build();

        //new server executor
        let mut server_executor = ServerExecutor::new();
        let _ = server_executor.start(server);

        //init client
        let mut client = Client::<TestClientService>::new();
        let client = client.with_timeout(Duration::from_secs(2));
        let layer0 = stdio::StdioTransport::new("abc", false).create();
        client.add_transport_layer(layer0);

        //for debugging
        client.add_protocol_layer(trace::Tracer::new().create());
        client.start().unwrap();
        client.build();

        let mut client_executor = ClientExecutor::new();
        let _ = client_executor.start(client.clone());

        let init_result = client.initialize();
        if let Err(err) = init_result {
            return;
        }
        let init_result = init_result.unwrap();

        if let Some(server_info) = init_result.get("serverInfo") {
            if let (Some(server_name), Some(server_version), Some(protocol_version)) = (
                server_info.get("name"),
                server_info.get("version"),
                init_result.get("protocolVersion"),
            ) {
                println!(
                    "Connnected to server: {} v{} with protocol version {}",
                    server_name.as_str().unwrap(),
                    server_version.as_str().unwrap(),
                    protocol_version.as_str().unwrap()
                );
            }
        }

        //set level
        client.set_log_level(LoggingLevel::Info);

        // list tools
        let list_tool_result = client.list_tool(Some("0".to_string())).unwrap();
        if let Some(tools) = list_tool_result.get("tools") {
            println!("Tools: {:?}", tools);
        }

        let toolcall_result = client.call_tool(CallToolParams {
            name: "test_tool".to_string(),
            arguments: None,
        });
        println!("Tools/call {:?}", toolcall_result);
        let _= client.cancel();

        let _= client.ping();

        let _= client.shutdown();
        let _= client_executor.stop();
        let _= server_executor.stop();
    }
    
    #[test]
    pub fn test_setup_logging(){
        init_log();
        info!("Starting test");
        setup_logging(&LoggingLevel::Info);
        info!("Starting test");
        setup_logging(&LoggingLevel::Warning);
        info!("Starting test");
        warn!("Starting test>");
    }
}
