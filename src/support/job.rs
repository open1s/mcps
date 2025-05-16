#![allow(dead_code)]

use crossbeam::channel::{self, Receiver, Sender};
use may::coroutine::{self, JoinHandle};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use serde_json::Value;

#[derive(Debug)]
pub enum TaskEvent<T, E> {
    Data(T),         // 任务发送的数据项
    Progress((u8,u32)),    // 任务进度更新
    Done,            // 任务正常完成
    Cancelled,       // 任务被取消
    Error(E),        // 任务返回错误
    Panic(String),   // 任务 panic
}

#[derive(Clone)]
pub struct JobTask<T: Send + 'static, E: Send + 'static>  {
    is_cancelled: Arc<AtomicBool>,
    handle: Option<Arc<JoinHandle<()>>>,
    event_rx:  Receiver<TaskEvent<T, E>>,
    _event_tx: Sender<TaskEvent<T, E>>, // 保持 channel 开启
}


impl<T: Send + 'static, E: Send + 'static> JobTask<T, E>  {
    pub fn new<F>(params: Value,task: F) -> Self
    where
        F: FnOnce(Value,Sender<TaskEvent<T, E>>) + Send + 'static,
    {
        let is_cancelled = Arc::new(AtomicBool::new(false));
        let (event_tx, event_rx) = channel::unbounded();

        let flag = is_cancelled.clone();
        let sender = event_tx.clone();

        // 在协程中运行任务
        let handle = unsafe { coroutine::spawn(move || {
            // 检查是否已被取消
            if flag.load(Ordering::Acquire) {
                let _ = sender.send(TaskEvent::Cancelled);
                return;
            }

            // 执行任务并捕获 panic
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                task(params,sender.clone())
            }));

            match result {
                Ok(_) => {
                    // 任务正常完成
                    let _ = sender.send(TaskEvent::Done);
                }
                Err(_) => {
                    // 任务 panic
                    let _ = sender.send(TaskEvent::Panic(format!("panic")));
                }
            }
        }) };

        JobTask {
            is_cancelled: is_cancelled,
            handle: Some(Arc::new(handle)),
            event_rx: event_rx,
            _event_tx: event_tx,
        }
    }

    // 中断任务
    pub fn cancel(&mut self) {
        self.is_cancelled.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            // 强制取消协程（如果标志位未被及时检查）
            unsafe { handle.coroutine().cancel() };
        }
    }

    pub fn try_recv(&self) -> Option<TaskEvent<T, E>> {
        self.event_rx.try_recv().ok()
    }

    pub fn recv(&self) -> Option<TaskEvent<T, E>> {
        self.event_rx.recv().ok()
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Option<TaskEvent<T, E>> {
        self.event_rx.recv_timeout(timeout).ok()
    }
}

impl <T, E>  Drop for JobTask<T, E>
where
    T: Send  + 'static,
    E: Send  + 'static,
{
    fn drop(&mut self) {
        self.cancel(); // 确保任务被清理
    }
}

#[cfg(test)]
mod tests {
    use std::thread;
    use irgo::defer;
    use serde_json::json;
    use super::*;
    #[test]
    fn test_job_task() {
        let params = json!({});
        let mut job:JobTask<String,String> = JobTask::new(params,|params,sender| {
            println!("Hello, world!");
            defer!(println!("Goodbye, world!"));

            loop {
                // 模拟长时间运行的任务
                sender.send(TaskEvent::Data("hi".to_string())).unwrap();
                may::coroutine::yield_now();
                may::coroutine::sleep(std::time::Duration::from_secs(1));
            }
        }
        );

        let cloned_job = job.clone();
        thread::spawn(move||{
            while let Some(event) = cloned_job.recv() {
                match event {
                    TaskEvent::Data(v) => println!("{}", v),
                    TaskEvent::Done => println!("Task completed"),
                    TaskEvent::Cancelled => println!("Task cancelled"),
                    TaskEvent::Error(e) => println!("Error: {}", e),
                    TaskEvent::Panic(p) => println!("Panic: {}", p),
                    TaskEvent::Progress(p) => {
                        println!("Progress: {}", p.0);
                    }
                }
            }
        });

        std::thread::sleep(std::time::Duration::from_secs(5)); 
        assert_eq!(job.is_cancelled.load(Ordering::Relaxed), false);
        job.cancel();
        assert_eq!(job.is_cancelled.load(Ordering::Relaxed), true);
        println!("Job cancelled!");
        std::thread::sleep(std::time::Duration::from_secs(3)); 
        println!("Main thread finished.");
    }
}