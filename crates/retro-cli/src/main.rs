#![forbid(unsafe_code)]

use std::io::{stderr, stdout};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut stdout = stdout().lock();
    let mut stderr = stderr().lock();
    ExitCode::from(retro_cli::execute(
        std::env::args_os().skip(1),
        &mut stdout,
        &mut stderr,
    ))
}
