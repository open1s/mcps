use std::{ sync::Arc, time::Duration};

use disruptor::{Producer};
use ibag::iBag;
use log::info;
use rioc::{LayerChain, LayerResult, PayLoad, SharedLayer};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

use crate::{schema::json_rpc::{JSONRPCMessage, JSONRPCRequest, RequestId, LATEST_PROTOCOL_VERSION}, support::{disruptor::{DisruptorFactory, DisruptorWriter}, ControlBus}, MCPError};


#[derive(Clone)]
pub struct Client {
    next_request_id: i64,
    connected: bool,
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

    pub fn with_timeout(mut self, duration: Duration) -> Self {
        self.timeout_duration = Some(duration);
        self
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
        let initial_request = JSONRPCRequest::new(
            self.next_request_id(),
            "initialize".to_string(),
            Some(serde_json::json!({
                "protocol_version": LATEST_PROTOCOL_VERSION,
            }))
        );

        let message = JSONRPCMessage::Request(initial_request);

        let payload = rioc::PayLoad{
            data: Some(serde_json::to_string(&message).unwrap()),
            ctx: None
        };

        //send initial request to server
        self.handle_outbound(Some(payload));

        //wait for response
        let response = self.recieve_with_timeout::<JSONRPCMessage>()?;
        match response {
            JSONRPCMessage::Response(response) => {
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
    
    pub fn call_tool<P: Serialize + Send + Sync, R: DeserializeOwned + Send + Sync>(
        &mut self,
        tool_name: &str,
        params: &P,
    ) -> Result<R, MCPError> {
        let tool_call_request = JSONRPCRequest::new(
            self.next_request_id(),
            tool_name.to_string(),
            Some(serde_json::json!({
                "name": tool_name,
                "parameters": serde_json::to_value(params)?,
            })),
        );

        let message = JSONRPCMessage::Request(tool_call_request);
        let payload = rioc::PayLoad{
            data: Some(serde_json::to_string(&message).unwrap()),
            ctx: None
        };
        //send tool call request to server
        self.handle_outbound(Some(payload));

        //wait for response 
        let response = self.recieve_with_timeout::<JSONRPCMessage>()?;
        match response {
            JSONRPCMessage::Response(resp) => {
                let result = resp.result;
                let result = result.get("result").ok_or_else(||{
                    MCPError::Protocol("Missing 'result' field in response".to_string())
                })?;
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
        let shutdown_request = JSONRPCRequest::new(
            self.next_request_id(),
            "shutdown".to_string(),
            None
        );

        let message = JSONRPCMessage::Request(shutdown_request);
        let payload = rioc::PayLoad{
            data: Some(serde_json::to_string(&message).unwrap()),
            ctx: None
        };

         //send initial request to server
         self.handle_outbound(Some(payload));

         //wait for response
         let response = self.recieve_with_timeout::<JSONRPCMessage>()?;
         match response {
             JSONRPCMessage::Response(_) => {
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


    // pub fn start(&mut self) -> Result<(), MCPError> {
    //     if self.connected {
    //         return Err(MCPError::Protocol("Client already initialized".to_string()));
    //     }
    //     self.connected = true;

    //     let client = self.clone();
    //     let disruptor = DisruptorFactory::create(move |e: &PayLoad, _seq: Sequence, _end_of_patch: bool| {
    //         if let Some(data) = &e.data {
    //             info!("Received message: {:?}", data);
    //             match serde_json::from_str::<JSONRPCMessage>(&data) {
    //                 Ok(message) => {
    //                     if let Err(err) = client.handle_message(message) {
    //                         log::error!("handle_message failed: {}", err);
    //                     }
    //                 }
    //                 Err(err) => {
    //                     log::error!("Failed to parse JSONRPCMessage: {}", err);
    //                 }
    //             }
    //         }
    //     });

    //     self.disruptor = Some(disruptor);   
    //     Ok(())
    // }


    pub fn handle_outbound(&self, message: Option<rioc::PayLoad>) -> Result<(),String>{
        self.chain.with_read(|layer|{
            layer.handle_outbound(message);
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

    // pub fn publish(&self, message: PayLoad) {
    //     self.disruptor.clone().unwrap().publish(|e| {
    //         e.data = message.data;
    //     });
    // }

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
        let client_cloned = self.clone();
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

    pub fn next_request_id(&mut self) -> RequestId {
        self.next_request_id += 1;
        let id = self.next_request_id;
        RequestId::Number(id)
    }
}


#[cfg(test)]
mod tests {
    use crate::transport::stdio;

    use super::*;
    #[test]
    fn test_next_request_id() {
        let mut client = Client::new();
        let layer0 = stdio::StdioTransport::new().layer0();

        client.add_transport_layer(layer0);
        client.build();

        let d = client.handle_inbound();
        println!("{:?}", d);
    }
}