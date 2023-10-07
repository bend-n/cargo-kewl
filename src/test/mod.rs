mod ui;
use anyhow::Result;
use crossbeam::channel::{unbounded, Receiver};
use crossterm::event::{self, Event, KeyCode};
use ratatui::prelude::*;
use ratatui::Terminal;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::cargo;
use crate::cargo::{test, TestEvent, TestMessage, TestResult};
use crate::test::ui::stdout::Stdout;

#[derive(Default, PartialEq, Eq)]
pub enum Screen {
    #[default]
    Main,
    Stdout,
}

enum Test {
    InProgress { name: String },
    Succeeded(TestResult),
    Failed(TestResult),
    Ignored { name: String },
}

impl Test {
    fn name(&self) -> &str {
        let (Self::InProgress { name }
        | Self::Succeeded(TestResult { name, .. })
        | Self::Failed(TestResult { name, .. })
        | Self::Ignored { name }) = self;
        name
    }

    fn stdout(&self) -> Option<&str> {
        match self {
            Self::Succeeded(TestResult { stdout, .. })
            | Self::Failed(TestResult { stdout, .. }) => stdout.as_deref(),
            _ => None,
        }
    }
}

trait VAt {
    fn at(&mut self, which: &str) -> Option<&mut Test>;
}

impl VAt for Vec<Test> {
    fn at(&mut self, which: &str) -> Option<&mut Test> {
        let p = self.iter().position(|t| t.name() == which)?;
        Some(&mut self[p])
    }
}

pub struct TestState {
    tests: Vec<Test>,
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
        let (tx, rx) = unbounded();
        let TestEvent::SuiteStart { test_count } = test(tx, dir)? else {
            panic!("first ev should be suite start")
        };
        Ok(Self {
            test_list: ui::test_list::TestList::default(),
            tests: vec![],
            rx,
            screen: Screen::default(),
            done: false,
            test_count,
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
            };
            match event {
                TestEvent::TestStart { name } => {
                    self.tests.push(Test::InProgress { name });
                }
                TestEvent::TestOk(r) => {
                    let pre = self.tests.at(&r.name).unwrap();
                    *pre = Test::Succeeded(r);
                }
                TestEvent::TestFail(r) => {
                    let pre = self.tests.at(&r.name).unwrap();
                    *pre = Test::Failed(r);
                }
                TestEvent::TestIgnore { name } => {
                    let pre = self.tests.at(&name).unwrap();
                    *pre = Test::Ignored { name };
                }
                TestEvent::SuiteOk { exec_time, .. } | TestEvent::SuiteFail { exec_time, .. } => {
                    self.time += exec_time;
                }
                TestEvent::SuiteStart { test_count } => {
                    log::trace!("have {test_count} tests");
                    self.test_count += test_count;
                }
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
