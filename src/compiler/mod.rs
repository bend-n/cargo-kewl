//! compiler output ui
use anyhow::Result;
use cargo_metadata::{diagnostic::Diagnostic, CompilerMessage, Message, PackageId};
use crossbeam::channel::Receiver;
use crossterm::event::{self, Event, KeyCode};
use ratatui::prelude::*;
use std::{
    ops::ControlFlow,
    time::{Duration, Instant},
};

use crate::{
    cargo::{self, TestMessage},
    ui::SList,
};

mod ui;

const BUILT_SCRIPT: u8 = 1;
const BUILD_SCRIPT_EXECUTED: u8 = 2;
const FINISHED: u8 = 4;

struct Crate {
    pid: PackageId,
    /// bitflag, see above
    state: u8,
}

struct State {
    compiled: SList,
    crates: Vec<Crate>,
    diagnostics: Vec<String>,
    rx: Receiver<TestMessage>,
    failed: bool,
}

impl State {
    fn new(rx: Receiver<TestMessage>) -> Self {
        Self {
            compiled: SList::default(),
            diagnostics: vec![],
            crates: vec![],
            failed: false,
            rx,
        }
    }

    fn recv(&mut self) -> RecvStatus {
        let deadline = Instant::now() + Duration::from_millis(50);
        while let Ok(event) = self.rx.recv_deadline(deadline) {
            match event {
                TestMessage::CompilerEvent(e) => match e {
                    Message::BuildFinished(b) => {
                        return match b.success {
                            true => RecvStatus::Finished,
                            false => RecvStatus::Failed,
                        }
                    }
                    Message::BuildScriptExecuted(f) => {
                        let p = self
                            .crates
                            .iter()
                            .position(|Crate { pid, .. }| pid == &f.package_id)
                            .unwrap();
                        self.crates[p].state |= BUILD_SCRIPT_EXECUTED;
                    }
                    Message::CompilerArtifact(c) => {
                        self.compiled.itemc += 1;
                        if c.target.name == "build-script-build" {
                            self.crates.push(Crate {
                                pid: c.package_id,
                                state: BUILT_SCRIPT,
                            });
                        } else {
                            match self
                                .crates
                                .iter()
                                .position(|Crate { pid, .. }| pid == &c.package_id)
                            {
                                None => self.crates.push(Crate {
                                    pid: c.package_id,
                                    state: FINISHED,
                                }),
                                Some(n) => self.crates[n].state |= FINISHED,
                            }
                        }
                    }
                    Message::CompilerMessage(CompilerMessage {
                        message:
                            Diagnostic {
                                rendered: Some(rendered),
                                ..
                            },
                        ..
                    }) => {
                        if self.diagnostics.contains(&rendered) {
                            continue;
                        };
                        self.diagnostics.push(rendered);
                    }
                    // Message::CompilerMessage(CompilerMessage { message, .. }) => {
                    //     let mut h = ahash::AHasher::default();
                    //     message.hash(&mut h);
                    //     let v = h.finish();
                    //     log::trace!("got {message}");
                    //     if self.diagnostics.iter().all(|&(_, hash)| (hash != v)) {
                    //         if let Some(span) = message.spans.first() {
                    //             let f = std::fs::read_to_string(at.join(span.file_name.clone()))
                    //                 .unwrap();
                    //             let mut e = lerr::Error::new(&f);
                    //             e.message(format!(
                    //                 "{}: {}",
                    //                 match message.level {
                    //                     DiagnosticLevel::Help =>
                    //                         cformat_args!("{green}help{reset}"),
                    //                     DiagnosticLevel::Note => cformat_args!("{cyan}note{reset}"),
                    //                     DiagnosticLevel::Warning =>
                    //                         cformat_args!("{yellow}nit{reset}"),
                    //                     _ => cformat_args!("{red}error{reset}"),
                    //                 },
                    //                 message.message
                    //             ));
                    //             for span in message.spans {
                    //                 e.label((
                    //                     span.byte_start as usize..span.byte_end as usize,
                    //                     span.label.unwrap_or("here".to_string()),
                    //                 ));
                    //             }
                    //             self.diagnostics.push((e.to_string(), v));
                    //             continue;
                    //         } else {
                    //             let mut e = lerr::Error::new("\n");
                    //             e.message(format!(
                    //                 "{}: {}",
                    //                 match message.level {
                    //                     DiagnosticLevel::Help =>
                    //                         cformat_args!("{green}help{reset}"),
                    //                     DiagnosticLevel::Note => cformat_args!("{cyan}note{reset}"),
                    //                     _ => cformat_args!("{red}error{reset}"),
                    //                 },
                    //                 message.message
                    //             ));
                    //             self.diagnostics.push((e.to_string(), v));
                    //             continue;
                    //         }
                    //     }
                    _ => {}
                },
                e => unreachable!("got bad event {e:?}"),
            }
        }
        RecvStatus::None
    }
}

enum RecvStatus {
    Finished,
    Failed,
    None,
}

pub fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    meta: &cargo::Metadata,
    rx: Receiver<TestMessage>,
) -> Result<ControlFlow<(), Receiver<TestMessage>>> {
    print!("\x1b]0;compiling {}\x07", meta.package.name);
    let mut state = State::new(rx);
    loop {
        terminal.draw(|f| ui::ui(f, &mut state, meta))?;
        if event::poll(Duration::from_millis(5))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(ControlFlow::Break(())),
                    KeyCode::Down | KeyCode::Char('s') => state.compiled.next(),
                    KeyCode::Up | KeyCode::Char('w') => state.compiled.prev(),
                    _ => {}
                }
            }
        }
        if state.failed {
            continue;
        }
        match state.recv() {
            RecvStatus::Failed => state.failed = true,
            RecvStatus::None => {}
            RecvStatus::Finished => return Ok(ControlFlow::Continue(state.rx)),
        };
    }
}
