//! Shell-out reformatter backend: pipe stripped source through an external
//! formatter (`prettier`, `gofmt`, `black`, ...).
//!
//! The command is an argv vector — no shell, no interpolation — so a
//! checked-in `contasty.toml` cannot smuggle shell metacharacters. Stripped
//! source goes in on stdin; formatted source is read from stdout. Any failure
//! (spawn error, non-zero exit, timeout, non-UTF-8 output) is logged at `warn`
//! and yields `None`, so the caller keeps the unformatted text. Stripping
//! correctness never depends on a formatter being installed.

use std::io::{Read, Write};
use std::process::{Child, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Wall-clock budget for one reformat invocation before the child is killed.
const TIMEOUT: Duration = Duration::from_secs(10);
/// How often the wait loop polls the child for completion.
const POLL: Duration = Duration::from_millis(5);

/// Background reader draining the child's stdout into a buffer.
type StdoutReader = JoinHandle<std::io::Result<Vec<u8>>>;

/// Run `argv` with `source` on stdin, returning its stdout on success or `None`
/// (with a logged warning) on any failure.
pub(super) fn run(argv: &[String], source: &str) -> Option<String> {
    let Some((program, rest)) = argv.split_first() else {
        log::warn!("reformat: empty command; keeping unformatted output");
        return None;
    };
    let mut child = match std::process::Command::new(program)
        .args(rest)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            log::warn!("reformat: cannot spawn `{program}`: {err}; keeping unformatted output");
            return None;
        }
    };
    feed_stdin(&mut child, source);
    let reader = take_stdout(&mut child);
    wait_for_output(child, program, reader)
}

/// Write `source` to the child's stdin from a detached thread so a formatter
/// that streams output (filling the stdout pipe) cannot deadlock us.
fn feed_stdin(child: &mut Child, source: &str) {
    if let Some(mut stdin) = child.stdin.take() {
        let buf = source.as_bytes().to_vec();
        std::thread::spawn(move || {
            // A formatter that exits without consuming all input yields a broken
            // pipe; that is the child's prerogative, so the error is ignored.
            let _ = stdin.write_all(&buf);
        });
    }
}

/// Drain the child's stdout in a thread so we can poll for the timeout without
/// the child blocking on a full pipe.
fn take_stdout(child: &mut Child) -> Option<StdoutReader> {
    child.stdout.take().map(|mut stdout| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            stdout.read_to_end(&mut buf).map(|_| buf)
        })
    })
}

/// Poll the child until it exits or the timeout elapses, then collect output.
fn wait_for_output(
    mut child: Child,
    program: &str,
    reader: Option<StdoutReader>,
) -> Option<String> {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return finish(status, program, reader),
            Ok(None) => {}
            Err(err) => {
                log::warn!("reformat: `{program}` wait failed: {err}; keeping unformatted output");
                return None;
            }
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            log::warn!(
                "reformat: `{program}` timed out after {}s; keeping unformatted output",
                TIMEOUT.as_secs()
            );
            return None;
        }
        std::thread::sleep(POLL);
    }
}

/// Turn a finished child's exit status and captured stdout into formatted text,
/// warning and returning `None` on a non-zero exit or non-UTF-8 output.
fn finish(
    status: std::process::ExitStatus,
    program: &str,
    reader: Option<StdoutReader>,
) -> Option<String> {
    if !status.success() {
        log::warn!("reformat: `{program}` exited with {status}; keeping unformatted output");
        return None;
    }
    let bytes = reader?.join().ok()?.ok()?;
    let Ok(text) = String::from_utf8(bytes) else {
        log::warn!("reformat: `{program}` emitted non-UTF-8 output; keeping unformatted output");
        return None;
    };
    Some(text)
}

#[cfg(test)]
#[path = "shellout_tests.rs"]
mod tests;
