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

#![allow(unused_variables)]

use std::io::Write;
use disruptor::{Producer, Sequence};
use crate::MCPError;
use crate::transport::common::{DisruptorProcessorCallback, DisruptorWriter, PayLoad, IoProvider, HeaderType};
use crate::transport::disruptor::DisruptorFactory;

#[derive(Clone)]
pub struct StdioDisruptorProvider {
    disruptor: DisruptorWriter
}


impl StdioDisruptorProvider {
    pub fn new() -> Self {
        Self {
            disruptor: Self::create()
        }
    }
    pub fn create() -> DisruptorWriter {
        let f:  DisruptorProcessorCallback = Box::new(|e: &PayLoad,_seq: Sequence,_end_of_patch:bool| {
            println!("{}",e.data().unwrap());
            std::io::stdout().flush().unwrap();
        });

        let disruptor = DisruptorFactory::create(f.into());
        disruptor
    }
}

impl Default for StdioDisruptorProvider { fn default() -> Self { Self::new() } }

impl IoProvider for StdioDisruptorProvider {
    fn read(&self) -> Result<PayLoad, MCPError> {
        //read from stdin
        std::io::stdout().flush().unwrap();
        let mut input = String::new();
        let result  =std::io::stdin().read_line(&mut input);
        if result.is_err() {
            return Err(MCPError::Transport("Error reading from stdin".to_string()));
        }
        let payload = PayLoad::builder().data(Some(input)).hdr(HeaderType::Data).build();

        Ok(payload)
    }

    fn write(&mut self, data: &PayLoad) -> Result<(), MCPError> {
        //write to disruptor
        self.disruptor.publish(|e|{
            e.data = Some(data.data().unwrap().to_string());
        });
        Ok(())
    }
}