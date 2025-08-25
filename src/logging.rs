use log::LevelFilter;
use simplelog::{Config, SimpleLogger, WriteLogger};
use std::fs;

pub fn init_logger(systemd: bool) {
    if systemd {
        // Log to stdout/stderr so journald captures logs
        SimpleLogger::init(LevelFilter::Info, Config::default()).unwrap();
    } else {
        WriteLogger::init(
            LevelFilter::Info,
            Config::default(),
            fs::File::create("~.oxidots.log").unwrap(),
        )
        .unwrap();
    }
}
