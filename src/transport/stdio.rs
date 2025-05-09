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


use std::cell::RefCell;
use std::io::Read;
use std::path::Path;
use std::sync::{Arc, Mutex};
use crate::support::definition::McpLayer;
use crate::support::shared_memory::{MemoryDuplex, SharedMemory};
use crate::MCPError;
use bytes::BufMut;
use disruptor::{Producer, Sequence};
use rand::rand_core::le;
use rioc::{Direction, Layer, LayerBuilder, LayerResult, PayLoad, SharedLayer};
use crate::support::ControlBus;
use crate::support::disruptor::{DisruptorProcessorCallback, DisruptorWriter};
use crate::support::disruptor::DisruptorFactory;
use ibuf::{MBuf, MPool};

/// Standard IO transport
#[derive(Clone)]
pub struct StdioTransport{
    control_bus: Arc<ControlBus>,
    pipe: Arc<MemoryDuplex>,
    is_server: bool,
}

impl StdioTransport {
    pub fn new(path: impl AsRef<Path>, is_server: bool) -> Self {
        let duplex = if is_server {
            //create a new shared memory for server
            MemoryDuplex::create(path,128000)
                .expect("Failed to create shared memory for server")
        } else {
            MemoryDuplex::open(path)
                .expect("Failed to open shared memory for client")
        };

        StdioTransport {
            control_bus: Arc::new(ControlBus::new()),
            is_server,
            pipe: Arc::new(duplex),
        }
    }

    pub fn layer0_tx(&self, data: PayLoad) -> Result<(), MCPError> {
        let mut data = data.data.clone().ok_or_else(|| 
            MCPError::Transport("Payload data is None".to_string()))?;

        let mut buf = MBuf::with_capacity(1024);
        let data = data.as_bytes();
        let len = data.len() as u32;
        buf.append(&len.to_be_bytes());
        buf.append(data);

        self.pipe.write(&buf)
            .map_err(|e| MCPError::Transport(format!("Failed to write to shared memory: {}", e)))?;    
        Ok(())
    }

    pub fn layer0_rx(&self) -> Result<PayLoad, MCPError> {
        let mut len_data = [0; 4];

        //read len first
        let bytes_read = self.pipe.read(&mut len_data)
            .map_err(|e| MCPError::Transport(format!("Failed to read from shared memory: {}", e)))?;

        let len = u32::from_be_bytes(len_data);
        let mut data = vec![0; len as usize];

        
        let bytes_read = self.pipe.read(&mut data)
            .map_err(|e| MCPError::Transport(format!("Failed to read from shared memory: {}", e)))?;

        let payload = String::from_utf8(data).map_err(|e| MCPError::Transport(format!("Failed to convert to string: {}", e)))?;
        Ok(PayLoad{
            data: Some(payload),
            ctx: None,
        })
    }
}


impl McpLayer for StdioTransport {
    fn create(&self) -> SharedLayer {
        let io = self.clone();
        let tx_io = io.clone();
        let rx_io = io.clone();

        let builder = LayerBuilder::new();
        let layer = builder
            .with_inbound_fn(move |req|{
                let data = rx_io.layer0_rx();
                return  Ok(LayerResult{
                    direction: Direction::Inbound,
                    data: Some(data.unwrap()),
                })
            })
            .with_outbound_fn(move |req|{
                if req.is_none(){
                    return Err("no data to send".to_string());
                }
                let req = req.unwrap();
                tx_io.layer0_tx(req).unwrap();
                Ok(LayerResult{
                    direction: Direction::Outbound,
                    data: None,
                })
            }).build();
        return layer.unwrap();    
    }
}


#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;
    use tungstenite::client;

    use super::*;
    
    #[test]
    fn test_stdio_transport() {
        
        let server = thread::spawn(move ||{
            let mut server_transport = StdioTransport::new("abc", true);

            let layer = server_transport.create();
            let result = layer.borrow().handle_inbound(None).unwrap();
            println!("server recieved: {:?}", result);

            let data = PayLoad {
                data: Some("Hello from server".to_string()),
                ctx: None,
            };

            let result =  layer.borrow().handle_outbound(Some(data)).unwrap();
            println!("server sent: {:?}", result);
        });


        let client = thread::spawn(move ||{
            thread::sleep(Duration::new(1, 0));
            let mut client_transport = StdioTransport::new("abc", false);
             let layer = client_transport.create();

            let data = PayLoad {
                data: Some("Hello from client".to_string()),
                ctx: None,
            };

            let result =  layer.borrow().handle_outbound(Some(data)).unwrap();
            println!("client sent: {:?}", result);

            let result = layer.borrow().handle_inbound(None).unwrap();
            println!("client recieved: {:?}", result);
        });

        client.join().unwrap();
        server.join().unwrap();
    }
}