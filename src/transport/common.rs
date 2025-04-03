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

use disruptor::{MultiProducer, Sequence, SingleConsumerBarrier};
use lombok::{Builder, Getter, GetterMut, Setter};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use crate::MCPError;

#[derive(Debug,Clone,Serialize, Deserialize,PartialEq)]
pub enum HeaderType {
    Data = 1,
    Close = 2
}

#[derive(Debug, Clone,Default, Serialize, Deserialize,Builder,Getter,GetterMut,Setter)]
pub struct PayLoad  {
    pub hdr: HeaderType,
    pub data: Option<String>,
}

impl Default for HeaderType {
    fn default() -> Self {
         HeaderType::Data
    }
}

impl PayLoad {
    pub fn type_(&self) -> HeaderType {
        self.hdr.clone()
    }

    pub fn data(&self) -> Option<String> {
        if self.data.is_none() {
            return None;
        }

        Some(self.data.clone().unwrap())
    }
}

pub trait IoProvider: Sync + Send {
    fn read(&self) -> Result<PayLoad, MCPError>;
    fn write(&mut self, data: &PayLoad) -> Result<(), MCPError>;
}

/// Type alias for a closure that is called when an error occurs
pub type ErrorCallback = Box<dyn Fn(&MCPError) + Send + Sync>;

/// Type alias for a closure that is called when a message is received
pub type MessageCallback = Box<dyn Fn(&str) + Send + Sync>;

/// Type alias for a closure that is called when the connection is closed
pub type CloseCallback = Box<dyn Fn() + Send + Sync>;

pub type DisruptorProcessorCallback = Box<dyn FnMut(&PayLoad, Sequence, bool) + Send>;

pub type DisruptorWriter = MultiProducer<PayLoad, SingleConsumerBarrier>;

pub trait Transport: Send + Sync {
    /// Start processing messages
    fn start(&mut self) -> Result<(), MCPError>;

    /// Send a message
    fn send<T: Serialize + Send + Sync>(&mut self, message: &T) -> Result<(), MCPError>;

    /// Receive a message
    fn receive<T: DeserializeOwned + Send + Sync>(&mut self) -> Result<T, MCPError>;

    fn receive_event(&mut self) -> Result<i32, MCPError>;

    /// Close the connection
    fn close(&mut self) -> Result<(), MCPError>;

    /// Set callback for when the connection is closed
    fn set_on_close(&mut self, callback: Option<CloseCallback>);

    /// Set callback for when an error occurs
    fn set_on_error(&mut self, callback: Option<ErrorCallback>);

    /// Set callback for when a message is received
    fn set_on_message<F>(&mut self, callback: Option<F>)
    where
        F: Fn(&str) + Send + Sync + 'static;
}