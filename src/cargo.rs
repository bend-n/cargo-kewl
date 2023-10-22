use anyhow::Result;
pub use cargo_metadata::{
    libtest::SuiteEvent, libtest::TestEvent, Message, TestMessage as RawTestMessage,
};
use crossbeam::channel::bounded;
use crossbeam::channel::Receiver;
use serde_derive::Deserialize;
use std::path::Path;
use std::{
    io::Read,
    process::{Command, Stdio},
};

#[derive(Debug)]
pub enum TestMessage {
    CompilerEvent(Message),
    Event(RawTestMessage),
    Finished,
}

pub fn test(at: Option<&Path>) -> Result<Receiver<TestMessage>> {
    let (tx, rx) = bounded(10);
    let mut proc = Command::new("cargo");
    if let Some(at) = at {
        proc.arg("-C");
        proc.arg(at.as_os_str());
    }
    // proc.env("RUSTFLAGS", format!("--diagnostic-width={width}"));
    proc.args([
        "-Zunstable-options",
        "test",
        "--message-format",
        "json",
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

    macro_rules! handle {
        ($n:expr) => {
            let n = $n;
            for &byte in &stdout[..n] {
                match byte {
                    b'\n' => {
                        let val = serde_json::from_slice::<serde_json::Value>(&tmp).unwrap();
                        log::debug!("got val: {}", serde_json::to_string_pretty(&val).unwrap());
                        let event = match serde_json::value::from_value::<Message>(val.clone()) {
                            Err(_) => TestMessage::Event(
                                serde_json::value::from_value::<RawTestMessage>(val).unwrap(),
                            ),
                            Ok(v) => TestMessage::CompilerEvent(v),
                        };
                        tmp.clear();
                        tx.send(event).unwrap();
                    }
                    b => tmp.push(b),
                }
            }
        };
    }
    std::thread::spawn(move || loop {
        handle!(out.read(&mut stdout).unwrap());
        if let Ok(Some(_)) = proc.try_wait() {
            while let Ok(n) = out.read(&mut stdout) {
                if n == 0 {
                    break;
                }
                handle!(n);
            }
            tx.send(TestMessage::Finished).unwrap();
            log::debug!("proc exited, joining thread");
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    });
    Ok(rx)
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
