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

use disruptor::{BusySpin, Sequence};
use crate::transport::common::{DisruptorProcessorCallback, DisruptorWriter, HeaderType, PayLoad};

pub struct DisruptorFactory;

impl DisruptorFactory {
    pub fn create(mut f: DisruptorProcessorCallback) -> DisruptorWriter {
        let factory = || {
            PayLoad  {
                hdr: HeaderType::Data,
                data: Some("".to_string())
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
    use crate::transport::common::{DisruptorProcessorCallback, HeaderType, PayLoad};
    use crate::transport::disruptor::DisruptorFactory;

    #[test]
    fn test_disruptor() {
        let f:  DisruptorProcessorCallback = Box::new(|e: &PayLoad,_seq: Sequence,_end_of_patch:bool| {
            println!("{}",e.data().unwrap());
        });

        let mut producer = DisruptorFactory::create(f);

        producer.publish(|e|{
            e.hdr = HeaderType::Data;
            e.data = Some("Hello World".to_string());
        });
    }
}