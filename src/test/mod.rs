mod ui;
use anyhow::Result;
use cargo_metadata::libtest::SuiteEvent;
use cargo_metadata::TestMessage as RTestMessage;
use crossbeam::channel::Receiver;
use crossterm::event::{self, Event, KeyCode};
use ratatui::prelude::*;
use ratatui::Terminal;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::cargo;
use crate::cargo::{test, TestEvent, TestMessage};
use crate::test::ui::stdout::Stdout;

#[derive(Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Main,
    Stdout,
}

pub struct TestState {
    tests: Vec<TestEvent>, // use the event like a state (ok => in progress, ..)
    test_list: ui::test_list::TestList,
    rx: Receiver<TestMessage>,
    screen: Screen,
    test_count: usize,
    stdout: Stdout,
    time: f32,
    done: bool,
}

impl TestState {
    pub fn new(dir: Option<&Path>) -> Result<Self> {
        log::info!("initializing test state");
        let rx = test(dir)?;
        Ok(Self {
            test_list: ui::test_list::TestList::default(),
            tests: vec![],
            rx,
            screen: Screen::default(),
            done: false,
            test_count: 0,
            time: 0.,
            stdout: Stdout::default(),
        })
    }

    pub fn recv(&mut self) {
        if self.done {
            return;
        }
        let deadline = Instant::now() + Duration::from_millis(50);
        while let Ok(event) = self.rx.recv_deadline(deadline) {
            log::debug!("got event {event:?}");
            let event = match event {
                TestMessage::Event(e) => e,
                TestMessage::Finished => {
                    self.done = true;
                    return;
                }
                TestMessage::CompilerEvent(c) => {
                    return;
                }
            };
            match event {
                RTestMessage::Test(t) => match t {
                    TestEvent::Started { name } => {
                        self.tests.push(TestEvent::Started { name });
                    }
                    t => {
                        let i = self
                            .tests
                            .iter()
                            .position(|o| o.name() == t.name())
                            .unwrap();
                        self.tests[i] = t;
                    }
                },
                RTestMessage::Suite(s) => match s {
                    SuiteEvent::Ok { exec_time, .. } | SuiteEvent::Failed { exec_time, .. } => {
                        self.time += exec_time;
                    }
                    SuiteEvent::Started { test_count } => {
                        log::trace!("have {test_count} tests");
                        self.test_count += test_count;
                    }
                },
                RTestMessage::Bench { .. } => unreachable!("not applicable"),
            };
        }
    }
}

pub fn run<B: Backend>(
    terminal: &mut Terminal<B>,
    dir: Option<&Path>,
    meta: &cargo::Metadata,
) -> Result<()> {
    let mut state = TestState::new(dir)?;
    loop {
        terminal.draw(|f| ui::ui(f, &mut state, &meta))?;
        if event::poll(Duration::from_millis(5))? {
            if let Event::Key(key) = event::read()? {
                match state.screen {
                    Screen::Main => match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Down | KeyCode::Char('s') => state.test_list.next(),
                        KeyCode::Up | KeyCode::Char('w') => state.test_list.prev(),
                        KeyCode::Right | KeyCode::Char('d')
                            if state.test_list.stdout(&state).is_some() =>
                        {
                            state.screen = Screen::Stdout;
                            state.stdout.scroll = 0;
                            state.stdout.lines = u16::try_from(
                                state.test_list.stdout(&state).unwrap().lines().count(),
                            )?;
                        }
                        _ => {}
                    },
                    Screen::Stdout => match key.code {
                        KeyCode::Char('q') => return Ok(()),
                        KeyCode::Down | KeyCode::Char('s') => state.stdout.incr(),
                        KeyCode::Up | KeyCode::Char('w') => state.stdout.decr(),
                        KeyCode::Left | KeyCode::Char('a') => {
                            state.screen = Screen::Main;
                            state.stdout.scroll = 0;
                        }
                        _ => {}
                    },
                }
            }
        }
        state.recv();
    }
}
