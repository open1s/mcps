use log::info;
use rioc::{Direction, LayerBuilder, LayerResult, SharedLayer};
use crate::support::definition::McpLayer;

pub struct Tracer;


impl Tracer {
    pub fn new() -> Self {
        Self
    }
}

impl McpLayer for Tracer {
    fn create(&self) -> SharedLayer {
        let builder = LayerBuilder::new();
        let layer = builder
            .with_inbound_fn(move |req|{
                //log the request
                info!("Tracer received inbound request: {:#?}", &req);
                
                return  Ok(LayerResult{
                    direction: Direction::Inbound,
                    data: req,
                })
            })
            .with_outbound_fn(move |req|{
                if req.is_none(){
                    return Err("no data to send".to_string());
                }
                //log the request
                info!("Tracer received outbound request: {:#?}", &req);
                Ok(LayerResult{
                    direction: Direction::Outbound,
                    data: req,
                })
            }).build();
        return layer.unwrap();
    }
}