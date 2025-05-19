use std::{fs::OpenOptions, sync::{Arc, Mutex}};
use log::{Level, LevelFilter, Log, Metadata, Record};
use crate::schema::schema::LoggingLevel;
use std::io::Write;
pub trait Appender: Send + Sync{
    fn append(&self, record: &Record);
}

#[derive(Debug, Clone)]
pub struct ConsoleAppender;
impl Appender for ConsoleAppender {
    fn append(&self, record: &Record) {
        let color_code = match record.level() {
            Level::Error => "\x1b[31m",    // 红色
            Level::Warn  => "\x1b[33m",    // 黄色
            Level::Info  => "\x1b[90m",    // 绿色
            Level::Debug => "\x1b[90m",    // 蓝色
            Level::Trace => "\x1b[90m",    // 灰色
        };

        let reset = "\x1b[0m";
        println!(
            "{}[{}] {} {}",
            color_code,
            record.level(),
            format!("{} -> {}", record.metadata().target(), record.args()),
            reset
        );
    }
}

#[derive(Debug, Clone)]
pub struct FileAppender {
    file: Arc<Mutex<std::fs::File>>,
}

impl FileAppender {
    pub fn new(path: &str) -> Self {
        //if parent directory does not exist, create it
        if let Some(parent_dir) = std::path::Path::new(path).parent() {
            if !parent_dir.exists() {
                std::fs::create_dir_all(parent_dir).ok();
            }
        }
        let file = OpenOptions::new().create(true).append(true).open(path).unwrap();
        Self { file: Arc::new(Mutex::new(file)) }
    }
}

impl Appender for FileAppender {
    fn append(&self, record: &Record) {
        let mut file = self.file.lock().unwrap();
        writeln!(file, "[{}] {} -> {}", record.level(),record.metadata().target(), record.args()).ok();
    }
}


pub struct McpInterceptorLogger {
    appenders: Vec<Arc<dyn Appender>>,
    level_filter: LevelFilter,
}

impl McpInterceptorLogger {
    pub fn new(appenders: Vec<Arc<dyn Appender>>, level_filter: LevelFilter) -> Self {
        Self { appenders, level_filter }
    }

    pub fn init()  {
        let mut appenders: Vec<Arc<dyn Appender>> = vec![];

        appenders.push(Arc::new(ConsoleAppender));
        appenders.push(Arc::new(FileAppender::new("log/requests.log")));
        let logger = McpInterceptorLogger::new(appenders, LevelFilter::Info);

        log::set_boxed_logger(Box::new(logger)).unwrap();
        log::set_max_level(log::LevelFilter::Warn);
    }
}

impl Log for McpInterceptorLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level_filter
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            for appender in &self.appenders {
                appender.append(record);
            }
        }
    }

    fn flush(&self) {}
}


pub fn setup_logging(level: &LoggingLevel){
    let log_level = match level {
        LoggingLevel::Debug => log::LevelFilter::Debug,
        LoggingLevel::Info => log::LevelFilter::Info,
        LoggingLevel::Error => log::LevelFilter::Error,
        LoggingLevel::Notice => log::LevelFilter::Info,
        LoggingLevel::Warning => log::LevelFilter::Warn,
        LoggingLevel::Critical => log::LevelFilter::Error,
        LoggingLevel::Alert => log::LevelFilter::Warn,
        LoggingLevel::Emergency => log::LevelFilter::Warn,
    };

    log::set_max_level(log_level);
}