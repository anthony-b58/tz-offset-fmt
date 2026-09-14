use std::io::{self, BufRead, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return run_stdin();
    }

    let mut had_error = false;
    for arg in &args {
        match tz_offset_fmt::normalize(arg) {
            Ok(offset) => println!("{arg} -> {offset}"),
            Err(err) => {
                eprintln!("{arg} -> error: {err}");
                had_error = true;
            }
        }
    }

    if had_error {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Batch mode: one offset per line on stdin, e.g. `cut -f3 log.csv | tz-offset-fmt`.
/// Blank lines are skipped so trailing newlines in piped input don't get flagged
/// as parse errors.
fn run_stdin() -> ExitCode {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut had_error = false;
    let mut saw_any = false;

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(err) => {
                eprintln!("error reading stdin: {err}");
                return ExitCode::FAILURE;
            }
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        saw_any = true;
        match tz_offset_fmt::normalize(trimmed) {
            Ok(offset) => {
                let _ = writeln!(out, "{trimmed} -> {offset}");
            }
            Err(err) => {
                eprintln!("{trimmed} -> error: {err}");
                had_error = true;
            }
        }
    }

    if !saw_any {
        eprintln!("usage: tz-offset-fmt <offset> [<offset> ...]");
        eprintln!("       tz-offset-fmt < file    (one offset per line)");
        eprintln!("example: tz-offset-fmt UTC+5:30 -0800 Z PST");
        return ExitCode::FAILURE;
    }

    if had_error {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
