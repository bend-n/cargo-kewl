use anyhow::{bail, Result};
use crossbeam::channel::Sender;
use serde_derive::Deserialize;
use std::path::Path;
use std::{
    error::Error,
    io::Read,
    process::{Command, Stdio},
};
#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum Type {
    Test,
    Suite,
}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum Event {
    Ok,
    #[serde(rename = "started")]
    Start,
    #[serde(rename = "failed")]
    Fail,
    #[serde(rename = "ignored")]
    Ignore,
}

#[derive(Deserialize, Debug)]
// todo: figure out if theres a cool serde trick
struct TestRaw {
    #[serde(rename = "type")]
    ty: Type,
    event: Event,
    name: Option<String>,
    passed: Option<usize>,
    failed: Option<usize>,
    ignored: Option<usize>,
    measured: Option<usize>,
    filtered_out: Option<usize>,
    test_count: Option<usize>,
    stdout: Option<String>,
    exec_time: Option<f32>,
}

#[derive(Debug, PartialEq)]
pub struct TestResult {
    pub name: String,
    pub exec_time: f32,
    pub stdout: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum TestEvent {
    SuiteStart {
        test_count: usize,
    },
    SuiteOk {
        failed: usize,
        passed: usize,
        ignored: usize,
        measured: usize,
        filtered_out: usize,
        exec_time: f32,
    },
    SuiteFail {
        passed: usize,
        failed: usize,
        ignored: usize,
        measured: usize,
        filtered_out: usize,
        exec_time: f32,
    },
    TestStart {
        name: String,
    },
    TestOk(TestResult),
    TestFail(TestResult),
    TestIgnore {
        name: String,
    },
}

#[derive(Debug)]
pub enum TestMessage {
    Event(TestEvent),
    Finished,
}

pub fn test(to: Sender<TestMessage>, at: Option<&Path>) -> Result<TestEvent> {
    let mut proc = Command::new("cargo");
    if let Some(at) = at {
        proc.arg("-C");
        proc.arg(at.as_os_str());
    }
    proc.args([
        "-Zunstable-options",
        "test",
        "--",
        "-Zunstable-options",
        "--report-time",
        "--show-output",
        "--format",
        "json",
    ]);
    log::trace!("running {proc:?}");
    let mut proc = proc.stdout(Stdio::piped()).stderr(Stdio::null()).spawn()?;
    let mut out = proc.stdout.take().unwrap();
    let mut tmp = Vec::with_capacity(32);
    let mut stdout = [0; 4096];
    loop {
        let n = out.read(&mut stdout)?;
        for &byte in &stdout[..n] {
            match byte {
                b'\n' => {
                    log::debug!("got first event, returning");
                    let event = parse_test(std::str::from_utf8(&tmp).unwrap()).unwrap();
                    tmp.clear();
                    log::debug!("spawning thread");
                    std::thread::spawn(move || loop {
                        let n = out.read(&mut stdout).unwrap();
                        for &byte in &stdout[..n] {
                            match byte {
                                b'\n' => {
                                    let event =
                                        parse_test(std::str::from_utf8(&tmp).unwrap()).unwrap();
                                    tmp.clear();
                                    to.send(TestMessage::Event(event)).unwrap();
                                }
                                b => tmp.push(b),
                            }
                        }
                        if let Ok(Some(_)) = proc.try_wait() {
                            to.send(TestMessage::Finished).unwrap();
                            log::debug!("proc exited, joining thread");
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    });
                    return Ok(event);
                }
                b => tmp.push(b),
            }
        }
        if let Some(exit) = proc.try_wait()? {
            log::trace!("process died, we die");
            bail!("process exited too early ({exit})");
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

#[derive(Debug)]
struct Should(&'static str);
impl std::fmt::Display for Should {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "should have had a {}, dang", self.0)
    }
}
impl Error for Should {}

fn parse_test(s: &str) -> Result<TestEvent> {
    let raw = serde_json::from_str::<TestRaw>(s)?;
    log::trace!("got raw event {raw:?}");
    macro_rules! take {
        ($thing:ident { $($holds:ident),+ $(?= $($opt:ident),+)?}) => {
            $thing {
                $($holds: raw.$holds.ok_or(Should(stringify!($holds)))?,)+
                $($($opt: raw.$opt),+)?
            }
        };
        ($thing:ident($inner:ident { $($holds:ident),+ $(?= $($opt:ident),+)? })) => {
            $thing(take!($inner { $($holds),+ $(?= $($opt),+)? }))
        }
    }
    use TestEvent::*;
    Ok(match raw.ty {
        Type::Test => match raw.event {
            Event::Start => take!(TestStart { name }),
            Event::Ok => take!(TestOk(TestResult {
                name,
                exec_time ?= stdout
            })),
            Event::Fail => take!(TestFail(TestResult {
                name,
                exec_time ?= stdout
            })),
            Event::Ignore => take!(TestIgnore { name }),
        },
        Type::Suite => match raw.event {
            Event::Start => take!(SuiteStart { test_count }),
            Event::Ok => take!(SuiteOk {
                failed,
                passed,
                ignored,
                measured,
                filtered_out,
                exec_time
            }),
            Event::Fail => {
                take!(SuiteFail {
                    failed,
                    passed,
                    ignored,
                    measured,
                    filtered_out,
                    exec_time
                })
            }
            Event::Ignore => panic!("ignore suite???"),
        },
    })
}

#[derive(Deserialize)]
pub struct Package {
    pub name: String,
}

#[derive(Deserialize)]
pub struct Metadata {
    pub package: Package,
}

pub fn meta(at: &Path) -> Result<Metadata> {
    Ok(toml::from_str(&std::fs::read_to_string(
        at.join("Cargo.toml"),
    )?)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_output() {
        macro_rules! run {
            ($($input:literal parses to $output:expr),+) => {
                $(assert_eq!(parse_test($input).unwrap(), $output);)+
            };
        }
        run![
            r#"{ "type": "suite", "event": "started", "test_count": 2 }"# parses to TestEvent::SuiteStart { test_count: 2},
            r#"{ "type": "test", "event": "started", "name": "fail" }"# parses to TestEvent::TestStart { name: "fail".into() },
            r#"{ "type": "test", "name": "fail", "event": "ok", "exec_time": 0.000003428, "stdout": "hello world" }"# parses to TestEvent::TestOk(TestResult { name: "fail".into(), exec_time: 0.000003428, stdout: Some("hello world".into()) }),
            r#"{ "type": "test", "event": "started", "name": "nope" }"# parses to TestEvent::TestStart { name: "nope".into() },
            r#"{ "type": "test", "name": "nope", "event": "ignored" }"# parses to TestEvent::TestIgnore { name: "nope".into() },
            r#"{ "type": "suite", "event": "ok", "passed": 1, "failed": 0, "ignored": 1, "measured": 0, "filtered_out": 0, "exec_time": 0.000684028 }"# parses to TestEvent::SuiteOk { passed: 1, failed: 0, ignored: 1, measured: 0, filtered_out: 0, exec_time: 0.000684028 }
        ];
        r#"
        { "type": "suite", "event": "started", "test_count": 1 }
        { "type": "test", "event": "started", "name": "fail" }
        { "type": "test", "name": "fail", "event": "failed", "exec_time": 0.000081092, "stdout": "thread 'fail' panicked at src/main.rs:3:5:\nexplicit panic\nnote: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n" }
        { "type": "suite", "event": "failed", "passed": 0, "failed": 1, "ignored": 0, "measured": 0, "filtered_out": 0, "exec_time": 0.000731068 }
        "#;
    }
}
