use std::{ sync::Arc, time::Duration};

use ibag::iBag;
use rioc::{LayerChain, LayerResult, SharedLayer};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{to_value, Value};

use crate::{schema::schema::{CallToolResult, ClientCapabilities, ClientRequest, ClientShutdownRequest, Implementation, InitializeParams, InitializeRequest, JSONRPCMessage, JSONRPCRequest, RequestId, RootsCapability, LATEST_PROTOCOL_VERSION}, support::{disruptor::DisruptorWriter, ControlBus}, MCPError};
use crate::schema::client::{build_client_notification, build_client_request};
use crate::schema::json_rpc::{mcp_json_param, mcp_param};
use crate::schema::schema::{CallToolParams, CallToolRequest, ClientNotification, Cursor, InitializedNotification, InitializedNotificationParams, ListToolsRequest, PaginatedParams};

#[derive(Clone)]
pub struct Client {
    connected: bool,
    next_request_id: i64,
    timeout_duration: Option<Duration>,
    notify: Arc<ControlBus>,
    chain: iBag<LayerChain>,
    disruptor: Option<DisruptorWriter>,
}

impl Client {
    pub fn new() -> Self {
        Self {
            connected: false,
            next_request_id: 0,
            timeout_duration: None,
            notify: Arc::new(ControlBus::new()),
            chain: iBag::new(LayerChain::new()),
            disruptor: None,
        }
    }

    pub fn with_timeout(&mut self, duration: Duration) -> &mut Self {
        self.timeout_duration = Some(duration);
        self
    }

    pub fn recieve_bytes_with_timeout(&self) -> Result<Vec<u8>, MCPError> {
        if self.timeout_duration.is_none() {
            return self.try_recieve_bytes();
        } else {
            let timeout_duration = self.timeout_duration.unwrap();
            let start_time = std::time::Instant::now();

            while start_time.elapsed() < timeout_duration {
                let result = self.try_recieve_bytes();
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

    pub fn recieve_with_timeout<R: DeserializeOwned + Send + Sync>(&self) -> Result<R, MCPError> {
        if self.timeout_duration.is_none() {
            return self.try_recieve::<R>();
        } else {
            let timeout_duration = self.timeout_duration.unwrap();
            let start_time = std::time::Instant::now();

            while start_time.elapsed() < timeout_duration {
                let result = self.try_recieve::<R>();
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


    pub fn try_recieve_bytes(&self) -> Result<Vec<u8>, MCPError> {
        let mut recieved: Option<String> = None;
        self.chain.with_read(|layer| {
            let req = layer.handle_inbound(None);
            if let Ok(req) = req {
                recieved = req.data.unwrap().data;
            }
        });
        if let Some(data) = recieved {
            Ok(data.into_bytes())
        }else {
            Err(MCPError::Transport("No data received".to_string()))
        }
    }

    pub fn try_recieve<R: DeserializeOwned + Send + Sync>(&self) -> Result<R, MCPError> {
        let mut recieved: Option<String> = None;
        self.chain.with_read(|layer| {
            let req = layer.handle_inbound(None);
            if let Ok(req) = req {
                recieved = req.data.unwrap().data;
            }
        });
        if let Some(data) = recieved {
            // Deserialize the received data
            match serde_json::from_str::<R>(&data) {
                Ok(deserialized) => Ok(deserialized),
                Err(err) => Err(MCPError::Serialization(err)),
            }
        } else {
            Err(MCPError::Transport("No data received".to_string()))
        }
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
               roots: Some(RootsCapability{
                 list_changed: Some(false),
               }),
               sampling: None,
            },
        };

        let initial_request = InitializeRequest::new(initial_params);
        let client_request = ClientRequest::Initialize(initial_request);

        let request_id = self.next_request_id();
        let req = build_client_request(request_id.clone(), client_request);

        let payload = rioc::PayLoad{
            data: mcp_json_param(&req),
            ctx: None
        };

        //send initial request to server
        let _ = self.handle_outbound(Some(payload));

        //wait for response
        let response = self.recieve_with_timeout::<JSONRPCMessage>()?;
        match response {
            JSONRPCMessage::Response(response) => {
                //response with notification
                let notification = InitializedNotification::new(
                   InitializedNotificationParams{
                        _meta: None,
                    }
                );

                let notification = ClientNotification::Initialized(notification);
                let notify = build_client_notification(notification);
                let payload = rioc::PayLoad{
                    data: mcp_json_param(&notify),
                    ctx: None
                };
                //send initial request to server
                let _ = self.handle_outbound(Some(payload));

                assert!(response.id == request_id);

                Ok(response.result)
            }
            JSONRPCMessage::Error(error) => {
                Err(MCPError::Protocol(format!("Error: {:?}", error)))
            }
            _ => {
                Err(MCPError::Protocol("Invalid response".to_string()))
            }
        }
    }
    
    pub fn list_tool(&mut self, cursor: Option<Cursor>) -> Result<Value, MCPError> {
        let list_tool_req = ListToolsRequest::new(Some(
            PaginatedParams{
                cursor,
            }
        ));

        let req = ClientRequest::ListTools(list_tool_req);
        let request_id = self.next_request_id();
        let req = build_client_request(request_id.clone(), req);
        let payload = rioc::PayLoad{
            data: mcp_json_param(&req),
            ctx: None
        };

        //send initial request to server
        let _ = self.handle_outbound(Some(payload));

        //wait for response 
        let response = self.recieve_with_timeout::<JSONRPCMessage>()?;
        match response {
            JSONRPCMessage::Response(resp) => {
                assert!(resp.id == request_id);
                Ok(resp.result)
            }
            JSONRPCMessage::Error(error) => {
                Err(MCPError::Protocol(format!("Error: {:?}", error)))
            }
            _ => {
                Err(MCPError::Protocol("Invalid response".to_string()))
            }
        }
    }

    pub fn call_tool(
        &mut self,
        params: CallToolParams,
    ) -> Result<CallToolResult, MCPError> {
        let call_tool_req = CallToolRequest::new(
            params
        );

        let req = ClientRequest::CallTool(call_tool_req);
        let request_id = self.next_request_id();
        let req = build_client_request(request_id.clone(), req);


        let payload = rioc::PayLoad{
            data: mcp_json_param(&req),
            ctx: None
        };
        //send tool call request to server
        let _ = self.handle_outbound(Some(payload));

        //wait for response 
        let response = self.recieve_with_timeout::<JSONRPCMessage>()?;
        match response {
            JSONRPCMessage::Response(resp) => {
                assert_eq!(resp.id,request_id);
                let result = resp.result;
                serde_json::from_value(result.clone()).map_err(MCPError::Serialization)
            }
            JSONRPCMessage::Error(error) => {
                Err(MCPError::Protocol(format!("Tool call failed {:?}",error)))
            }
            _ => {
                Err(MCPError::Protocol("Unexpected response type".to_string()))
            }
        }
    }


    pub fn shutdown(&mut self) -> Result<(),MCPError> {
        let shutdown_req = ClientShutdownRequest::new();

        let req = ClientRequest::Shutdown(shutdown_req);
        let request_id = self.next_request_id();
        let req = build_client_request(request_id.clone(), req);
        let payload = rioc::PayLoad{
            data: Some(serde_json::to_string(&req).unwrap()),
            ctx: None
        };

         //send initial request to server
        let _ = self.handle_outbound(Some(payload));

         //wait for response
         let response = self.recieve_with_timeout::<JSONRPCMessage>()?;
         match response {
             JSONRPCMessage::Response(resp) => {
                 assert_eq!(resp.id,request_id);
                 Ok(())
             }
             JSONRPCMessage::Error(error) => {
                 Err(MCPError::Protocol(format!("Error: {:?}", error)))
             }
             _ => {
                 Err(MCPError::Protocol("Invalid response".to_string()))
             }
         }
    }


    pub fn execute_session<F, Fut, R>(&mut self, f: F) -> Result<R, MCPError>
    where
    F: FnOnce(&mut Self) -> Result<R, MCPError> + Send,
    R: Send,{
        self.initialize()?;

        let result = f(self);
        let shutdown_result = self.shutdown();

        // Return the function result if it succeeded, otherwise return the function error
        match result {
            Ok(r) => {
                // If the function succeeded but shutdown failed, return the shutdown error
                if let Err(e) = shutdown_result {
                    Err(e)
                } else {
                    Ok(r)
                }
            }
            Err(e) => {
                // If both the function and shutdown failed, prefer the function error
                Err(e)
            }
        }
    }

    fn handle_outbound(&self, message: Option<rioc::PayLoad>) -> Result<(),String>{
        self.chain.with_read(|layer|{
            let _ =  layer.handle_outbound(message);
        });
        Ok(())
    }

    pub fn handle_inbound(&self) -> Result<LayerResult,String> {
        let mut result: Result<LayerResult,String> = Err("Failed to get result".to_string());
        self.chain.with_read(|layer|{
            result = layer.handle_inbound(None);
        });
        result
    }

    pub fn add_transport_layer(&mut self, layer: SharedLayer) {
        self.chain.with(|chain|{
                chain.add_layer(layer);
            }
        )
    }

    pub fn add_trace_layer(&mut self, layer: SharedLayer) {
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

    pub fn next_request_id(&mut self) -> RequestId {
        self.next_request_id += 1;
        let id = self.next_request_id;
        RequestId::Number(id)
    }

    pub fn build(&mut self) {
        let builder = rioc::LayerBuilder::new();

        let layer = builder
            .with_inbound_fn(move |req| {
                log::info!("Received request: {:?}", req);
                //call back to server to handle the request
                if let Some(ref data) = req {
                    // client_cloned.publish(data.clone());
                    Ok(rioc::LayerResult {
                        direction: rioc::Direction::Inbound,
                        data: req,
                    })
                } else {
                    Err("Failed to get request data".to_string())
                }
                
            })
            .with_outbound_fn(move |req| {
                log::info!("Sending response: {:?}", req);
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


#[cfg(test)]
mod tests {

    use dashmap::DashMap;

    use crate::{executor::ServerExecutor, init_log, schema::schema::{Tool, ToolInputSchema}, server::{Server, ServerConfig}, support::definition::McpLayer, transport::{stdio, trace}};

    use super::*;
    #[test]
    fn test_next_request_id() {
        let mut client = Client::new();
        let layer0 = stdio::StdioTransport::new("abc",true).create();

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
                input_schema: ToolInputSchema{
                    r#type: "object".to_string(),
                    properties: None,
                    required: None,
                },
                description: None,
            });

        let mut server = Server::new(config);
        let _ = server.register_tool_handler("test_tool".to_string(), move |input|{
            println!("Tool called with input: {:?}", input);
            Ok(serde_json::json!({
                "result": "hello mcp client",
            }))
        });

        let server_transport = stdio::StdioTransport::new("abc",true).create();
        server.add_transport_layer(server_transport);
        server.start().unwrap();
        server.build();
        
        //new server executor
        let mut server_executor = ServerExecutor::new();
        let _ = server_executor.start(server);


        //init client
        let mut client = Client::new();
        let client = client.with_timeout(Duration::from_secs(3));
        let layer0 = stdio::StdioTransport::new("abc",false).create();
        client.add_transport_layer(layer0);

        //for debugging
        client.add_trace_layer(trace::Tracer::new().create());

        client.build();

        let init_result =  client.initialize().unwrap();

        if let Some(server_info) = init_result.get("serverInfo"){
            if let (Some(server_name), Some(server_version),Some(protocol_version)) = (
                server_info.get("name"), 
                server_info.get("version"), 
                init_result.get("protocolVersion")){
                println!("Connnected to server: {} v{} with protocol version {}", 
                    server_name.as_str().unwrap(), 
                    server_version.as_str().unwrap(), 
                    protocol_version.as_str().unwrap());
            }
        }
        
        // list tools
        let list_tool_result = client.list_tool(Some("0".to_string())).unwrap();
        if let Some(tools) = list_tool_result.get("tools") {
            println!("Tools: {:?}", tools);
        }
        
        
        let toolcall_result = client.call_tool(
            CallToolParams{
                name: "test_tool".to_string(),
                arguments: None,
            }
        );
        println!("Tools/call {:?}", toolcall_result);
        
        client.shutdown();
        server_executor.stop();
    }
}