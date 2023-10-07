use comat::{cformat_args, cwriteln};
use log::{Level, Metadata, Record};
use std::{
    fs::File,
    io::Write,
    path::PathBuf,
    sync::{Mutex, OnceLock, PoisonError},
    time::Instant,
};

#[derive(Debug)]
pub struct Logger {
    start: Instant,
    file: Mutex<File>,
}

impl Logger {
    pub fn init(level: Level, f: PathBuf) {
        static LOGGER: OnceLock<Logger> = OnceLock::new();
        LOGGER
            .set(Self {
                start: Instant::now(),
                file: Mutex::new(File::create(f).unwrap()),
            })
            .unwrap();
        log::set_logger(LOGGER.get().unwrap())
            .map(|()| log::set_max_level(level.to_level_filter()))
            .unwrap();
    }
}

impl log::Log for Logger {
    fn enabled(&self, _: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        cwriteln!(
            self.file.lock().unwrap_or_else(PoisonError::into_inner),
            "[{} {:bold_blue}:{:blue}{green}@{:yellow}] {}",
            match record.level() {
                Level::Error => cformat_args!("{bold_red}err{reset}"),
                Level::Warn => cformat_args!("{bold_yellow}wrn{reset}"),
                Level::Trace => cformat_args!("{magenta}trc{reset}"),
                Level::Debug => cformat_args!("{green}dbg{reset}"),
                Level::Info => cformat_args!("{blue}inf{reset}"),
            },
            record.file().unwrap_or("<source>"),
            record.line().unwrap_or(0),
            humantime::format_duration(self.start.elapsed()),
            record.args(),
        )
        .unwrap();
    }

    fn flush(&self) {}
}
