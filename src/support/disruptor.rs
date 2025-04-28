use disruptor::{BusySpin, MultiProducer, Sequence, SingleConsumerBarrier};
use rioc::PayLoad;

pub type DisruptorProcessorCallback = Box<dyn FnMut(&PayLoad, Sequence, bool) + Send>;
pub type DisruptorWriter = MultiProducer<PayLoad, SingleConsumerBarrier>;

pub struct DisruptorFactory;

impl DisruptorFactory {
    pub fn create(mut f: impl FnMut(&PayLoad, Sequence, bool) + Send + 'static) -> DisruptorWriter {
        let factory = || PayLoad {
            data: None,
            ctx: None,
        };
    
        let processor = move |e: &PayLoad, sequence: Sequence, end_of_batch: bool| {
            f(e, sequence, end_of_batch);
        };
    
        disruptor::build_multi_producer(64, factory, BusySpin)
            .handle_events_with(processor)
            .build()
    }
}


#[cfg(test)]
mod tests {
    use disruptor::{Producer, Sequence};
    use rioc::PayLoad;
    use super::DisruptorFactory;

    #[test]
    fn test_disruptor() {
        // let f:  DisruptorProcessorCallback = Box::new(|e: &PayLoad,_seq: Sequence,_end_of_patch:bool| {
        //     println!("{:?}",e.data.clone().unwrap());
        // });

        let mut producer = DisruptorFactory::create(|e: &PayLoad, _seq: Sequence, _end_of_patch:bool| {
            println!("{:?}",e.data.clone().unwrap());
        });

        producer.publish(|e|{
            e.data = Some("Hello".to_string());
        });
    }
}