use crate::schema::schema::LoggingLevel;

pub fn setup_logging(level: LoggingLevel){
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