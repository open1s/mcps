use std::time::Duration;

use rioc::{LayerResult, PayLoad};

use crate::schema::json_rpc::{JSONRPCMessage, JSONRPCRequest, RequestId, LATEST_PROTOCOL_VERSION};



pub struct Client {
    next_request_id: i64,
    connectioned: bool,
    timeout_duration: Option<Duration>
}

impl Client {
    pub fn new() -> Self {
        Self {
            connectioned: false,
            next_request_id: 1,
            timeout_duration: None
        }
    }

    pub fn with_timeout(mut self, duration: Duration) -> Self {
        self.timeout_duration = Some(duration);
        self
    }

    pub fn connect(&mut self) {
        self.connectioned = true;
    }

    pub fn initialize(&mut self) {
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

        // self.handle_outbound(Some(payload));
        self.connectioned = true;

        //TODO: 处理超时
    }


    // pub fn call_tool(&mut self, tool_name: &str, tool_args: serde_json::Value) -> Result<LayerResult,String>{
    //     let tool_call_request = JSONRPCRequest::new(
    //         self.next_request_id(),
    //         "tool_call".to_string(),
    //         Some(tool_args)
    //     );

    //     let message = JSONRPCMessage::Request(tool_call_request);
    //     let payload = rioc::PayLoad{
    //         data: Some(serde_json::to_string(&message).unwrap()),
    //         ctx: None
    //     };

    //     // self.handle_outbound(Some(payload))
    // }





    // pub fn handle_inbound(&self, message: Option<rioc::PayLoad>) -> Result<LayerResult,String>{
    //     Ok(())
    // }


    // pub fn handle_outbound(&self, message: Option<rioc::PayLoad>) -> Result<LayerResult,String>{
    //     Ok(())
    // }

    pub fn next_request_id(&mut self) -> RequestId {
        self.next_request_id += 1;
        let id = self.next_request_id;
        RequestId::Number(id)
    }
}