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

#![allow(unused)]

use std::ops::DerefMut;
use std::sync::Arc;
use crate::MCPError;
use crate::transport::common::{CloseCallback, ErrorCallback, HeaderType, IoProvider, MessageCallback, PayLoad, Transport};
use serde::{de::DeserializeOwned, Serialize};
use crate::support::ControlBus;

/// Standard IO transport
pub struct StdioTransport {
    event_bus: ControlBus,
    provider: Box<dyn IoProvider + 'static>,
    is_connected: bool,
    on_close: Option<CloseCallback>,
    on_error: Option<ErrorCallback>,
    on_message: Option<MessageCallback>,
}

impl StdioTransport {
    /// Create a new stdio transport using stdin and stdout
    pub fn new(provider: impl IoProvider + 'static) -> Self {
        Self {
            event_bus: ControlBus::new(),
            provider: Box::new(provider),
            is_connected: false,
            on_close: None,
            on_error: None,
            on_message: None,
        }
    }

    fn handle_error(&self, error: &MCPError) {
        if let Some(callback) = &self.on_error {
            callback(error);
        }
    }
}

// Implement Clone for StdioTransport
// impl Clone for StdioTransport {
//     fn clone(&self) -> Self {
//         // Create a new instance with its own reader but sharing the same writer channel
//         StdioTransport {
//             provider: self.provider.clone(),
//             event_bus: self.event_bus.clone(),
//             is_connected: self.is_connected,
//             on_close: None,
//             on_error: None,
//             on_message: None,
//         }
//     }
// }

impl  Transport for StdioTransport {
    fn start(&mut self) -> Result<(), MCPError> {
        if self.is_connected {
            return Ok(());
        }

        self.is_connected = true;
        Ok(())
    }

    fn send<T: Serialize + Send + Sync>(&mut self, message: &T) -> Result<(), MCPError> {
        if !self.is_connected {
            let error = MCPError::Transport("Transport not connected".to_string());
            self.handle_error(&error);
            return Err(error);
        }

        let json = match serde_json::to_string(message) {
            Ok(json) => json,
            Err(e) => {
                let error = MCPError::Serialization(e);
                self.handle_error(&error);
                return Err(error);
            }
        };

        let data = PayLoad  {
            hdr: HeaderType::Data,
            data: Some(json),
        };

        self.provider.write(&data).unwrap();
        Ok(())
    }

    fn receive<T: DeserializeOwned + Send + Sync>(&mut self) -> Result<T, MCPError> {
        if !self.is_connected {
            let error = MCPError::Transport("Transport not connected".to_string());
            self.handle_error(&error);
            return Err(error);
        }

        match self.provider.read() {
            Ok(payload) => {
                if let Some(callback) = &self.on_message {
                    callback(&payload.data.clone().unwrap());
                }

                match serde_json::from_str(payload.clone().data.unwrap().as_str()) {
                    Ok(parsed) => Ok(parsed),
                    Err(e) => {
                        let error = MCPError::Serialization(e);
                        self.handle_error(&error);
                        Err(error)
                    }
                }
            }
            Err(e) => {
                let error = MCPError::Transport(format!("Failed to read: {}", e));
                self.handle_error(&error);
                Err(error)
            }
        }
    }

    // fn receive_event(&mut self) -> Result<i32, MCPError> {
    //     let evt = self.event_bus.make_reader().try_recv();
    //     match evt {
    //         Ok(evt) => Ok(evt),
    //         Err(_) => {
    //             let error = MCPError::Transport("Failed to receive event".to_string());
    //             Err(error)
    //         }
    //     }
    // }

    fn close(&mut self) -> Result<(), MCPError> {
        if !self.is_connected {
            return Ok(());
        }

        self.is_connected = false;

        if let Some(callback) = &self.on_close {
            callback();
        }

        Ok(())
    }

    fn set_on_close(&mut self, callback: Option<CloseCallback>) {
        self.on_close = callback;
    }

    fn set_on_error(&mut self, callback: Option<ErrorCallback>) {
        self.on_error = callback;
    }

    fn set_on_message<F>(&mut self, callback: Option<F>)
    where
        F: Fn(&str) + Send + Sync + 'static,
    {
        self.on_message = callback.map(|f| Box::new(f) as Box<dyn Fn(&str) + Send + Sync>);
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io;
    use std::os::fd::AsRawFd;
    use super::*;
    use std::sync::{Arc};
    use crate::transport::stdio_disruptor::StdioDisruptorProvider;

    // Simple implementation of AsyncRead for testing
    struct MockSyncIoProvider {
        data: Arc<Vec<String>>,
    }

    impl MockSyncIoProvider {
        fn new(data: Vec<String>) -> Self {
            Self {
                data: Arc::new(data),
            }
        }
    }

    impl IoProvider for MockSyncIoProvider {
        fn read(&self) -> Result<PayLoad, MCPError> {
            if self.data.is_empty() {
                return Err(MCPError::Transport("No data".to_string()));
            }

            let data = PayLoad::builder()
                .data(Some(self.data[0].clone()))
                .hdr(HeaderType::Data)
                .build();
            Ok(data)
        }

        fn write(&mut self, data: &PayLoad) -> Result<(), MCPError> {
            println!("{}", data.data.as_ref().unwrap());
            Ok(())
        }
    }

    // Basic test for StdioTransport
    #[test]
    fn test_send_receive() {
        // Test message
        let test_message = r#"{"id":1,"jsonrpc":"2.0","method":"test","params":{}}"#;

        // Create a mock reader with test data
        let mock_provider = MockSyncIoProvider::new(vec![test_message.to_string() + "\n"]);

        // Create a transport with the mock reader
        let mut transport = StdioTransport::new(mock_provider);

        // Start the transport
        transport.start();

        // Define a simple struct to deserialize into
        #[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq)]
        struct TestMessage {
            id: u32,
            jsonrpc: String,
            method: String,
            params: serde_json::Value,
        }

        // Receive the message
        let message: TestMessage = transport.receive().unwrap();

        // Verify the message
        assert_eq!(message.id, 1);
        assert_eq!(message.jsonrpc, "2.0");
        assert_eq!(message.method, "test");

        // Send a response
        let response = TestMessage {
            id: 1,
            jsonrpc: "2.0".to_string(),
            method: "response".to_string(),
            params: serde_json::json!({}),
        };

        transport.send(&response);

        // Close the transport
        transport.close();

        // Verify the transport is closed
        assert!(!transport.is_connected);
    }

    // Test concurrent operations with two separate transports
    #[test]
    fn test_concurrent_operations() {
        // Test message
        let test_message1 = r#"{"id":1,"jsonrpc":"2.0","method":"test1","params":{}}"#;
        let test_message2 = r#"{"id":2,"jsonrpc":"2.0","method":"test2","params":{}}"#;

        // Create two separate transports with their own mock readers
        let mock_io1 = MockSyncIoProvider::new(vec![test_message1.to_string() + "\n"]);
        let mock_io2 = MockSyncIoProvider::new(vec![test_message2.to_string() + "\n"]);

        let mut transport1 = StdioTransport::new(mock_io1);
        let mut transport2 = StdioTransport::new(mock_io2);

        // Start both transports
        transport1.start().unwrap();
        transport2.start().unwrap();

        // Define a simple struct to deserialize into
        #[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq)]
        struct TestMessage {
            id: u32,
            jsonrpc: String,
            method: String,
            params: serde_json::Value,
        }

        // Spawn tasks to receive messages concurrently
        let s = std::thread::scope(|s| {
            let h1 = s.spawn(|| {
                let message: TestMessage = transport1.receive().unwrap();
                message
            });
            let h2 =  s.spawn(|| {
                let message: TestMessage = transport2.receive().unwrap();
                message
            });

            let result1 = h1.join().unwrap();
            let result2 = h2.join().unwrap();

            // Verify the results
            assert_eq!(result1.id, 1);
            assert_eq!(result1.method, "test1");
            assert_eq!(result2.id, 2);
            assert_eq!(result2.method, "test2");
        });
    }

    #[test]
    fn test_disruptor() {
        let io1 = StdioDisruptorProvider::new();
        let mut transport1 = StdioTransport::new(io1.clone());

        transport1.start();

        #[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq)]
        struct TestMessage {
            id: u32,
            jsonrpc: String,
            method: String,
            params: serde_json::Value,
        }

        let message = TestMessage {
            id: 1,
            jsonrpc: "2.0".to_string(),
            method: "test".to_string(),
            params: serde_json::json!({}),
        };

        transport1.send(&message).unwrap();
        transport1.send(&message).unwrap();
        transport1.send(&message).unwrap();
        transport1.send(&message).unwrap();


        //read from stdin
        let input_file = File::open("test_input.txt").unwrap();
        // 备份原始 stdin
        let original_stdin = io::stdin().as_raw_fd();

        // 使用 unsafe 操作重定向 stdin
        unsafe {
            libc::dup2(input_file.as_raw_fd(), original_stdin);
        }

        let line = io1.clone().read().unwrap();
        println!("{}", line.data().unwrap());
    }

    #[test]
    fn test_stdio() {
        //read from stdin
        let input_file = File::open("test_input.txt").unwrap();
        // 备份原始 stdin
        let original_stdin = io::stdin().as_raw_fd();

        // 使用 unsafe 操作重定向 stdin
        unsafe {
            libc::dup2(input_file.as_raw_fd(), original_stdin);
        }

        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer).unwrap();
        println!("{}", buffer);
    }
}
