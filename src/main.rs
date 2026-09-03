use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: tz-offset-fmt <offset> [<offset> ...]");
        eprintln!("example: tz-offset-fmt UTC+5:30 -0800 Z PST");
        return ExitCode::FAILURE;
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
