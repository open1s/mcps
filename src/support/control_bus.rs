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

use std::sync::{mpsc, Arc, Mutex};
use std::sync::mpsc::{SyncSender, TryRecvError};
use bus::{Bus, BusReader};
use crate::MCPError;

pub struct ControlBus {
    bus: Arc<Mutex<Bus<i32>>>,
    tx: Mutex<Option<SyncSender<i32>>>,
    is_running: Arc<Mutex<bool>>,
}

impl ControlBus {
    pub fn new() -> Self {
        Self {
            bus: Arc::new(Mutex::new(Bus::new(10))),
            tx: Mutex::new(None),
            is_running: Arc::new(Mutex::new(false)),
        }
    }

    pub fn start(&self) {
        let mut is_running = self.is_running.lock().unwrap();
        if *is_running {
            return;
        }
        *is_running = true;
        drop(is_running); // Release the lock early

        let (tx, rx) = mpsc::sync_channel(100);
        *self.tx.lock().unwrap() = Some(tx);

        let bus = self.bus.clone();
        let running_flag = self.is_running.clone();

        std::thread::spawn(move || {
            while *running_flag.lock().unwrap() {
                match rx.try_recv() {
                    Ok(msg) => {
                        if let Ok(mut bus) = bus.lock() {
                            if bus.try_broadcast(msg).is_err() {
                                eprintln!("Failed to broadcast message: {}", msg);
                            }
                        }
                    }
                    Err(TryRecvError::Empty) => {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(TryRecvError::Disconnected) => break,
                }
            }
        });
    }

    pub fn stop(&self) {
        *self.is_running.lock().unwrap() = false;
        *self.tx.lock().unwrap() = None;
    }

    pub fn clone_rx(&self) -> Result<BusReader<i32>, MCPError> {
        self.bus.lock()
            .map_err(|_| MCPError::Transport("Failed to lock bus".to_string()))
            .map(|mut bus| bus.add_rx())
    }

    pub fn clone_tx(&self) -> Result<SyncSender<i32>, MCPError> {
        self.tx.lock()
            .map_err(|_| MCPError::Transport("Failed to lock tx".to_string()))
            .and_then(|tx| tx.as_ref()
                .map(|t| t.clone())
                .ok_or(MCPError::Transport("No tx channel".to_string())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_control_bus() {
        let bus = ControlBus::new();
        bus.start();

        let mut reader = bus.clone_rx().unwrap();
        let handle = thread::spawn(move || {
            let mut count = 0;
            loop {
                match reader.try_recv() {
                    Ok(r) => {
                        println!("Received: {}", r);
                        count += 1;
                        if count == 2 {
                            break;
                        }
                    }
                    Err(_) => thread::sleep(Duration::from_millis(100)),
                }
            }
        });

        let sender1 = bus.clone_tx().unwrap();
        let handle1 = thread::spawn(move || {
            sender1.send(1).unwrap();
        });

        let sender2 = bus.clone_tx().unwrap();
        let handle2 = thread::spawn(move || {
            sender2.send(3).unwrap();
        });

        handle1.join().unwrap();
        handle2.join().unwrap();
        handle.join().unwrap();

        bus.stop();
    }
}