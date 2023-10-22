use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen, SetTitle,
    },
};
use log::Level as RLevel;
use ratatui::prelude::*;
pub mod cargo;
pub mod compiler;
mod logger;
mod test;
pub mod ui;

#[derive(Parser)]
/// Kewl cargo addon for dashboards
struct Args {
    #[arg(short = 'C')]
    /// Change to DIRECTORY before doing anything
    directory: Option<PathBuf>,
    #[arg(short = 'l')]
    /// Log to LOG_FILE
    log_file: Option<PathBuf>,
    #[arg(default_value = "trace", long = "level")]
    log_level: Level,
}

#[repr(usize)]
#[derive(clap::ValueEnum, Clone, Copy)]
enum Level {
    Error = 1,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<Level> for RLevel {
    fn from(value: Level) -> Self {
        // SAFETY: same
        unsafe { std::mem::transmute(value) }
    }
}
macro_rules! ctext {
    ($fmt:literal $(, $arg:expr)* $(,)?) => {
        <String as ansi_to_tui::IntoText>::into_text(&comat::cformat!($fmt $(, $arg)*)).expect("WHAT")
    };
}
use ctext;

fn main() -> Result<()> {
    let args = if std::env::args().next().unwrap().contains(".cargo/bin") {
        Args::parse_from(std::env::args().skip(1))
    } else {
        Args::parse()
    };
    if let Some(log) = args.log_file {
        logger::Logger::init(args.log_level.into(), log);
    }
    log::info!("startup");
    let mut stdout = std::io::stdout();
    let meta = cargo::meta(
        args.directory
            .as_deref()
            .unwrap_or(&std::env::current_dir()?),
    )?;

    enable_raw_mode()?;
    execute!(
        stdout,
        EnableMouseCapture,
        EnterAlternateScreen,
        SetTitle("testing")
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let h = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        disable_raw_mode().unwrap();
        execute!(std::io::stdout(), DisableMouseCapture, LeaveAlternateScreen).unwrap();
        h(panic);
    }));
    let res = test::run(&mut terminal, args.directory.as_deref(), &meta);
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableMouseCapture,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    res
}
