use disruptor::{BusySpin, MultiProducer, Sequence, SingleConsumerBarrier};
use rioc::PayLoad;

pub type DisruptorProcessorCallback = Box<dyn FnMut(&PayLoad, Sequence, bool) + Send>;
pub type DisruptorWriter = MultiProducer<PayLoad, SingleConsumerBarrier>;

pub struct DisruptorFactory;

impl DisruptorFactory {
    pub fn create(mut f: DisruptorProcessorCallback) -> DisruptorWriter {
        let factory = || {
            PayLoad  {
                data: None,
                ctx: None,
            }
        };

        // Closure for processing events.
        let processor = move |e: &PayLoad, sequence: Sequence, end_of_batch: bool| {
            f(e, sequence, end_of_batch);
        };

        let size = 64;
        let producer = disruptor::build_multi_producer(size, factory, BusySpin)
            .handle_events_with(processor)
            .build();
        producer
    }
}


#[cfg(test)]
mod tests {
    use disruptor::{Producer, Sequence};
    use rioc::PayLoad;
    use crate::transport::disruptor::DisruptorFactory;

    use super::DisruptorProcessorCallback;

    #[test]
    fn test_disruptor() {
        let f:  DisruptorProcessorCallback = Box::new(|e: &PayLoad,_seq: Sequence,_end_of_patch:bool| {
            println!("{:?}",e.data.clone().unwrap());
        });

        let mut producer = DisruptorFactory::create(f);

        producer.publish(|e|{
            e.data = Some("Hello".to_string());
        });
    }
}