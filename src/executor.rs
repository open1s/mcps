use disruptor::Producer;
use log::{info, warn};

use crate::{client::{Client, ClientProvider}, server::Server, support::ControlBus};


pub struct ServerExecutor{
    bus: ControlBus,
    started: bool,
}

pub struct ClientExecutor{
    bus: ControlBus,
    started: bool,
}



impl ServerExecutor {
    pub fn new() -> Self {
        ServerExecutor {
            bus: ControlBus::new(),
            started: false,
        }
    }

    pub fn stop(&self) {
        let mut tx = self.bus.clone_tx().unwrap();
        let _ = tx.publish(|e|{
            *e = 1;
        });
    }

    pub fn start(&mut self, server: Server) -> Result<String, String> {
        if self.started {
            return Err("Server already started".to_string());
        }

        self.started = true;
        let mut rx = self.bus.clone_rx().unwrap();
        let _ignored = std::thread::spawn(move || {
           loop {
                let envent = rx.try_recv();
                match envent {
                    Ok(r) => {
                        warn!("!!! Received: {}", r);
                        if r == 1 {
                            let _ = server.stop();
                            break;
                        }
                    }
                    Err(_) => {}        
                }

                let _ = server.serve();
           }
        });

        // handle.join().map_err(|e| format!("Error executing server: {:?}", e))?;

        Ok("Server started".to_string())
    }
}

impl ClientExecutor {
    pub fn new() -> Self {
        ClientExecutor {
            bus: ControlBus::new(),
            started: false,
        }
    }

    pub fn stop(&self) {
        let mut tx = self.bus.clone_tx().unwrap();
        let _ = tx.publish(|e|{
            *e = 1;
        });
    }

    pub fn start<T:  Default + ClientProvider + Clone + Send + 'static>(&mut self, client: Client<T>) -> Result<String, String> {
        if self.started {
            return Err("Client already started".to_string());
        }

        self.started = true;
        let mut rx = self.bus.clone_rx().unwrap();
        let _handle = std::thread::spawn(move || {
           loop {
                let envent = rx.try_recv();
                match envent {
                    Ok(r) => {
                        info!("Received: {}", r);
                        if r == 1 {
                            break;
                        }
                    }
                    Err(_) => {}        
                }

                let _ = client.serve();
           }
        });

        // handle.join().map_err(|e| format!("Error executing client: {:?}", e))?;

        Ok("Client started".to_string())
    }
}