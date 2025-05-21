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
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::net::TcpListener;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use crate::config::transport_config::HttpTransportConfig;
use crate::schema::schema::SESSION_ID_KEY;
use crate::support::definition::McpLayer;
use crate::support::shared_memory::{MemoryDuplex, SharedMemory};
use crate::MCPError;
use bytes::BufMut;
use disruptor::{Producer, Sequence};
use rand::rand_core::le;
use rioc::{ChainContext, Direction, Layer, LayerBuilder, LayerResult, PayLoad, SharedLayer};
use crate::support::ControlBus;
use crate::support::disruptor::{DisruptorProcessorCallback, DisruptorWriter};
use crate::support::disruptor::DisruptorFactory;
use ibuf::{MBuf, MPool};

use tiny_http::{Request, Response, SslConfig};
use tiny_http::{Server};


pub struct ServerBuilder {
    config: HttpTransportConfig,
}

impl ServerBuilder {
    pub fn new(config: HttpTransportConfig) -> Self {
        ServerBuilder {
            config,
        }
    }

    pub fn build(&self) -> Result<Server, MCPError> {
        let ssl_config = if self.config.enable_tls {
            let cert_path = self.config.cert_file.as_ref()
                .ok_or_else(|| MCPError::Transport("Missing certificate file".to_string()))?;

            let key_path = self.config.key_file.as_ref()
                .ok_or_else(|| MCPError::Transport("Missing key file".to_string()))?;

            let mut cert_file = File::open(cert_path).map_err(|e| {
                MCPError::Transport(format!("Failed to open certificate file: {:?}", e))
            })?;
            let mut cert_buf = Vec::new();
            cert_file.read_to_end(&mut cert_buf).map_err(|e| {
                MCPError::Transport(format!("Failed to read certificate file: {:?}", e))
            })?;

            let mut key_file = File::open(key_path).map_err(|e| {
                MCPError::Transport(format!("Failed to open key file: {:?}", e))
            })?;
            let mut key_buf = Vec::new();
            key_file.read_to_end(&mut key_buf).map_err(|e| {
                MCPError::Transport(format!("Failed to read key file: {:?}", e))
            })?;

            let config = SslConfig {
                certificate: cert_buf,
                private_key: key_buf,
            };
            Some(config)
        }else {
            None
        };

        let ip = self.config.ip_address.clone();
        let port = self.config.port;
        let bind_addr = format!("{}:{}", ip, port);

        println!("Starting HTTP server on {}", bind_addr);

        match ssl_config {
            Some(config) => {
                Server::https(bind_addr, config).map_err(|e| {
                    MCPError::Transport(format!("Failed to start HTTPS server: {:?}", e))
                })
            },
            None => {
                Server::http(bind_addr).map_err(|e| {
                    MCPError::Transport(format!("Failed to start HTTP server: {:?}", e))
                })
            }
        }
    }
}


/// Standard IO transport
#[derive(Clone)]
pub struct HttpStreamTransport{
    control_bus: Arc<ControlBus>,
    is_server: bool,
    server: Option<Arc<Server>>,
}

impl HttpStreamTransport {
    pub fn new(config: HttpTransportConfig, is_server: bool) -> Self {
        let server = ServerBuilder::new(config).build();

        HttpStreamTransport {
            control_bus: Arc::new(ControlBus::new()),
            server: Some(Arc::new(server.unwrap())),
            is_server,
        }
    }

    pub fn start(&self) -> Result<JoinHandle<()>, MCPError> {
        let server = self.server.as_ref().unwrap();
        let mut rx = self.control_bus.clone_rx().unwrap();
        let server = server.clone();

        let handle = std::thread::spawn(move||{
            loop {
                match rx.try_recv() {
                    Ok(_) => {
                        break;
                    }
                    Err(_) => {
                    }
                }
                let req =  server.recv_timeout(Duration::from_millis(10));
                if let Err(_) = req {
                    continue;
                }
                let req = req.unwrap();
                if  let None = req {
                    continue;
                }
                let req = req.unwrap();

                let response = Response::from_string("Hello World!");
                req.respond(response);
            }
        });
        
        Ok(handle)
    }

    pub fn layer0_tx(&self, data: PayLoad) -> Result<(), MCPError> {
        let mut data = data.data.clone().ok_or_else(|| 
            MCPError::Transport("Payload data is None".to_string()))?;

        // let mut buf = MBuf::with_capacity(1024);
        // let data = data.as_bytes();
        // let len = data.len() as u32;
        // buf.append(&len.to_be_bytes());
        // buf.append(data);

        // self.pipe.write(&buf)
        //     .map_err(|e| MCPError::Transport(format!("Failed to write to shared memory: {}", e)))?;    
        Ok(())
    }

    pub fn layer0_rx(&self) -> Result<PayLoad, MCPError> {
        // let mut len_data = [0; 4];

        // //read len first
        // let bytes_read = self.pipe.read(&mut len_data)
        //     .map_err(|e| MCPError::Transport(format!("Failed to read from shared memory: {}", e)))?;

        // let len = u32::from_be_bytes(len_data);
        // let mut data = vec![0; len as usize];

        
        // let bytes_read = self.pipe.read(&mut data)
        //     .map_err(|e| MCPError::Transport(format!("Failed to read from shared memory: {}", e)))?;

        // let mut ctx = ChainContext{
        //     data: HashMap::new(),
        // };

        // //we need a way 
        // ctx.data.insert(SESSION_ID_KEY.to_owned(), "local".to_string());
        // let payload = String::from_utf8(data).map_err(|e| MCPError::Transport(format!("Failed to convert to string: {}", e)))?;
        // Ok(PayLoad{
        //     data: Some(payload),
        //     ctx: Some(ctx),
        // })
        todo!();
    }
}


impl Drop for HttpStreamTransport {
    fn drop(&mut self) {
       let tx = self.control_bus.clone_tx();
       tx.unwrap().publish(|e|{
          *e = 1;
       });
    }
}


impl McpLayer for HttpStreamTransport {
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
    use crate::config::transport_config::HttpTransportConfig;

    use super::*;

    #[test]
    fn test_http_stream_transport() {
        let server = HttpStreamTransport::new(HttpTransportConfig {
            port: 1212,
            ip_address: "127.0.0.1".to_string(),
            enable_tls: false,
            cert_file: None,
            key_file: None,
        }, true);

        server.start().unwrap();
    }
}