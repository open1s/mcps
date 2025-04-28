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

use std::sync::{Arc, Mutex};
use nbus::{Bus, BusReader};
use disruptor::{BusySpin, MultiProducer, Sequence, SingleConsumerBarrier};
use crate::MCPError;

pub struct ControlBus {
    bus: Arc<Mutex<Bus<i32>>>,
    tx: Option<MultiProducer<i32, SingleConsumerBarrier>>,
}

impl ControlBus {
    pub fn new() -> Self {
        let mut  bus = ControlBus {
            bus: Arc::new(Mutex::new(Bus::new(10))),
            tx: None
        };

        bus.initialize();
        bus
    }

    fn initialize(&mut self) {
        let bus = self.bus.clone();
        let factory = || {
            1
        };

        // Closure for processing events.
        let dispatcher = move |e: &i32, _sequence: Sequence, _end_of_batch: bool| {
            if bus.lock().unwrap().try_broadcast(*e).is_err() {
                eprintln!("Failed to broadcast message: {}", e);
            }
        };

        let size = 64;
        let producer = disruptor::build_multi_producer(size, factory, BusySpin)
            .handle_events_with(dispatcher)
            .build();

        self.tx = Some(producer);
    }

    pub fn clone_rx(&self) -> Result<BusReader<i32>, MCPError> {
        self.bus.lock()
            .map_err(|_| MCPError::Transport("Failed to lock bus".to_string()))
            .map(|mut bus| bus.add_rx())
    }

    pub fn clone_tx(&self) -> Result<MultiProducer<i32, SingleConsumerBarrier>, MCPError> {
        if self.tx.is_none() {
            return Err(MCPError::Transport("No producer".to_string()));
        }

        let tx = self.tx.as_ref().unwrap().clone();
        Ok(tx.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;
    use disruptor::Producer;

    #[test]
    fn test_control_bus() {
        let bus = ControlBus::new();

        let mut reader = bus.clone_rx().unwrap();
        let handle = thread::spawn(move || {
            let mut count = 0;
            loop {
                match reader.try_recv() {
                    Ok(r) => {
                        println!("!!! Received: {}", r);
                        count += 1;
                        if count == 2 {
                            break;
                        }
                    }
                    Err(_) => thread::sleep(Duration::from_millis(100)),
                }
            }
        });

        let mut sender1 = bus.clone_tx().unwrap();
        let handle1 = thread::spawn(move || {
            sender1.publish(|e|{
                *e = 1;
            });
        });

        let mut sender2 = bus.clone_tx().unwrap();
        let handle2 = thread::spawn(move || {
            sender2.publish(|e|{
                *e = 2;
            });
        });

        handle1.join().unwrap();
        handle2.join().unwrap();
        handle.join().unwrap();
    }
}