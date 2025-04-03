#![allow(unused_mut)]

use std::sync::{Arc, Mutex};
use bus::{Bus, BusReader};

#[derive(Clone)]
pub struct ControlBus {
    bus: Arc<Mutex<Bus<i32>>>
}

impl ControlBus {
    pub fn new() -> Self {
        Self {
            bus: Arc::new(Mutex::new(Bus::new(10)))
        }
    }

    pub fn make_reader(&mut self) -> BusReader<i32> {
        let mut bus = self.bus.lock().unwrap();
        let mut reader = bus.add_rx();
        reader
    }

    pub fn try_broadcast(&mut self, event: i32) -> Result<(),i32> {
        let mut bus = self.bus.lock().unwrap();
        bus.try_broadcast(event)
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;
    use super::*;

    #[test]
    fn test_control_bus() {
        let mut bus = ControlBus::new();

        let mut reader = bus.make_reader();
        let handle = thread::spawn(move || {
            let mut count = 0;
            loop{
                let result = reader.try_recv();
                match result {
                    Ok(r) => {
                        println!("{}", r);
                        count += 1;
                        if count == 2 {
                            break;
                        }
                    }
                    Err(_) => {
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
        });

        let mut bus1 = bus.clone();

        let handle1 = thread::spawn(move || {
            let _ =  bus1.try_broadcast(1).unwrap();
        });

        let handle2 = thread::spawn(move || {
            let _ =  bus.try_broadcast(1).unwrap();
        });


        thread::sleep(Duration::from_millis(100));

        handle.join().unwrap();
        handle1.join().unwrap();
    }
}

