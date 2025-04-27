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
use std::sync::{Arc, Mutex};
use crate::MCPError;
use disruptor::{Producer, Sequence};
use rioc::{Direction, Layer, LayerBuilder, LayerResult, PayLoad, SharedLayer};
use crate::support::ControlBus;
use crate::transport::disruptor::{DisruptorProcessorCallback, DisruptorWriter};
use super::disruptor::DisruptorFactory;


/// Standard IO transport
pub struct StdioTransport{
    control_bus: Arc<ControlBus>,
    writer: DisruptorWriter,
}

impl StdioTransport {
    pub fn new() -> Self {
        let disruptor = DisruptorFactory::create(Box::new(|e: &PayLoad, _seq: Sequence, _end_of_patch: bool| {
            //write to stdout
            println!("{:?}", e.data.clone().unwrap());
        }));

        StdioTransport {
            control_bus: Arc::new(ControlBus::new()),
            writer: disruptor
       }
    }

    pub fn layer0_tx(&mut self, data: PayLoad) -> Result<(), MCPError> {
        let data = data.data.clone().ok_or_else(|| 
            MCPError::Transport("Payload data is None".to_string()))?;
        
        self.writer.publish(|e| {
            e.data = Some(data);
        });
        Ok(())
    }

    pub fn layer0_rx(&self) -> Result<PayLoad, MCPError> {
        //read from stdin
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)
            .map_err(|e| MCPError::Transport(format!("Failed to read from stdin: {}", e)))?;
        Ok(PayLoad {
            data: Some(input),
            ctx: None,
        })
    }

    pub fn layer0(&self) -> SharedLayer {
        let io = StdioTransport::new();
        let io_cloned = Arc::new(RefCell::new(io));
        let tx_io = io_cloned.clone();
        let rx_io = io_cloned.clone();

        let builder = LayerBuilder::new();
        let layer = builder
            .with_inbound_fn(move |req|{
               let data =  rx_io.borrow_mut().layer0_rx();
               Ok(LayerResult{
                    direction: Direction::Inbound,
                    data: Some(data.unwrap()),
               })
            })
            .with_outbound_fn(move |req|{
                if req.is_none(){
                    return Err("no data to send".to_string());
                }
                let req = req.unwrap();
                tx_io.borrow_mut().layer0_tx(req).unwrap();
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
    use super::*;
    
    #[test]
    fn test_stdio_transport() {
        let mut transport = StdioTransport::new();
        let data = PayLoad {
            data: Some("Hello".to_string()),
            ctx: None,
        };
        let layer = transport.layer0();
        // layer.borrow().handle_inbound(Some(data.clone())).unwrap();
        layer.borrow().handle_outbound(Some(data)).unwrap();
    }
}