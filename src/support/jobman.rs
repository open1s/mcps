use std::{sync::{Arc}};
use dashmap::DashMap;
use log::warn;
use rioc::{ChainContext, JobTask, PayLoad, TaskEvent};
use crate::schema::schema::{LoadType, RequestId};

#[derive(Clone)]
pub struct JobManager {
    jobs: Arc<DashMap<RequestId, (Option<ChainContext>, JobTask<(LoadType, String), i32, String>)>>,
}

impl JobManager {
    pub fn new() -> Self {
        JobManager { jobs: Arc::new(DashMap::new()) }
    }

    pub fn add_job(&mut self,req: RequestId, job: (Option<ChainContext>,JobTask<(LoadType,String),i32,String>)) {
        self.jobs.insert(req, job);
    }

    pub fn cancel_job(&mut self, req: RequestId) {
        let job  = self.jobs.remove(&req);
        if let Some(mut job) = job {
            job.1.1.cancel()
        } else {
            warn!("No job found with request {:?}", req);
        }
    }

    pub fn cancel_all_jobs(&mut self) {
        for mut job in self.jobs.iter_mut() {
            job.value_mut().1.cancel();
        }
        self.jobs.clear();
    }

    pub fn polling(&mut self) -> Result<Vec<(RequestId,LoadType,PayLoad)>, String> {
        let mut to_remove = Vec::new();
        let mut payloads = vec![];

        for mut entry in self.jobs.iter_mut() {
            let req = entry.key().clone();
            let (ctx, job) = entry.value_mut();
            if let Some(event) = job.try_recv() {
                match event {
                    TaskEvent::Data(data) => {
                        let payload = PayLoad {
                            data: Some(data.1),
                            ctx: ctx.clone(),
                        };
                        payloads.push((req.clone(), data.0, payload));
                    },
                    TaskEvent::Done => {
                        to_remove.push(req.clone());
                    },
                    _ => {}
                }
            }
        }

        for req in to_remove {
            self.jobs.remove(&req);
        }

        Ok(payloads)
    }
}

impl Drop for JobManager {
    fn drop(&mut self) {
        self.cancel_all_jobs();
    }
}