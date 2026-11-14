//! The only place the lab starts a process.
//!
//! Only the programs in `ALLOWED_PROGRAMS` may be run, matched by file name. Nothing is run through
//! a shell, `sudo` is not in the list, output is capped, and every call has a timeout. Tests use
//! `MockRunner`, which never starts anything and is labelled as a mock.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Programs the lab may start, by file name.
pub const ALLOWED_PROGRAMS: &[&str] = &["qemu-system-aarch64", "qemu-img"];

/// Output cap per stream, in bytes.
pub const MAX_OUTPUT_BYTES: usize = 1 << 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunStatus {
    Exited(i32),
    Timeout,
    NotFound,
    NotAllowed,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunResult {
    pub status: RunStatus,
    pub stdout: String,
    pub stderr: String,
    pub truncated: bool,
}

impl RunResult {
    pub fn ok(&self) -> bool {
        self.status == RunStatus::Exited(0)
    }
}

pub trait ProcessRunner {
    fn run(&self, program: &str, args: &[String], timeout: Duration) -> RunResult;
    /// True for test doubles. A mock result is never evidence of a real run.
    fn is_mock(&self) -> bool;
}

pub fn program_allowed(program: &str) -> bool {
    let name = Path::new(program)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    ALLOWED_PROGRAMS.contains(&name)
}

#[derive(Debug, Default)]
pub struct RealRunner;

fn read_capped<R: Read>(mut r: R) -> (String, bool) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    let mut truncated = false;
    loop {
        match r.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                if buf.len() + n > MAX_OUTPUT_BYTES {
                    buf.extend_from_slice(&chunk[..MAX_OUTPUT_BYTES - buf.len()]);
                    truncated = true;
                    break;
                }
                buf.extend_from_slice(&chunk[..n]);
            }
        }
    }
    (String::from_utf8_lossy(&buf).into_owned(), truncated)
}

impl ProcessRunner for RealRunner {
    fn run(&self, program: &str, args: &[String], timeout: Duration) -> RunResult {
        let fail = |status: RunStatus| RunResult {
            status,
            stdout: String::new(),
            stderr: String::new(),
            truncated: false,
        };
        if !program_allowed(program) {
            return fail(RunStatus::NotAllowed);
        }
        let mut child = match Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return fail(RunStatus::NotFound),
            Err(e) => return fail(RunStatus::Failed(e.kind().to_string())),
        };
        let out = child.stdout.take();
        let err = child.stderr.take();
        let t_out = std::thread::spawn(move || out.map(read_capped).unwrap_or_default());
        let t_err = std::thread::spawn(move || err.map(read_capped).unwrap_or_default());
        let start = Instant::now();
        let status = loop {
            match child.try_wait() {
                Ok(Some(s)) => break RunStatus::Exited(s.code().unwrap_or(-1)),
                Ok(None) if start.elapsed() >= timeout => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break RunStatus::Timeout;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                Err(e) => break RunStatus::Failed(e.kind().to_string()),
            }
        };
        let (stdout, t1) = t_out.join().unwrap_or_default();
        let (stderr, t2) = t_err.join().unwrap_or_default();
        RunResult {
            status,
            stdout,
            stderr,
            truncated: t1 || t2,
        }
    }

    fn is_mock(&self) -> bool {
        false
    }
}

/// A test double. It answers from a table keyed by the full command line and never starts anything.
#[derive(Debug, Default)]
pub struct MockRunner {
    answers: BTreeMap<String, RunResult>,
    pub calls: RefCell<Vec<String>>,
}

impl MockRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn answer(mut self, command_line: &str, result: RunResult) -> Self {
        self.answers.insert(command_line.to_string(), result);
        self
    }

    pub fn exits(self, command_line: &str, code: i32, stdout: &str) -> Self {
        self.answer(
            command_line,
            RunResult {
                status: RunStatus::Exited(code),
                stdout: stdout.to_string(),
                stderr: String::new(),
                truncated: false,
            },
        )
    }
}

impl ProcessRunner for MockRunner {
    fn run(&self, program: &str, args: &[String], _timeout: Duration) -> RunResult {
        let name = Path::new(program)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(program);
        let line = std::iter::once(name.to_string())
            .chain(args.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ");
        self.calls.borrow_mut().push(line.clone());
        self.answers.get(&line).cloned().unwrap_or(RunResult {
            status: RunStatus::NotFound,
            stdout: String::new(),
            stderr: String::new(),
            truncated: false,
        })
    }

    fn is_mock(&self) -> bool {
        true
    }
}
