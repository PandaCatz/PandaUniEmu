#![forbid(unsafe_code)]

use retro_testkit::nes_trace::{
    MAX_NROM_IMAGE_BYTES, MAX_REFERENCE_LOG_BYTES, TraceFailure, TraceInputFailure, TraceSummary,
    compare_nrom_trace_bytes,
};
use retro_testkit::run_synthetic;
use std::ffi::{OsStr, OsString};
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io::{ErrorKind, Read, Write};
use std::path::Path;

pub const EXIT_OK: u8 = 0;
pub const EXIT_TRACE_FAILURE: u8 = 1;
pub const EXIT_USAGE: u8 = 2;
pub const EXIT_INPUT: u8 = 3;
pub const EXIT_FIXTURE: u8 = 4;

const USAGE: &str = "usage: retro-cli [synthetic | nes-trace <ROM_PATH> <LOG_PATH>]";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FixtureKind {
    Rom,
    ReferenceLog,
}

impl Display for FixtureKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rom => formatter.write_str("ROM"),
            Self::ReferenceLog => formatter.write_str("reference log"),
        }
    }
}

#[derive(Debug)]
enum CommandFailure {
    Read {
        fixture: FixtureKind,
        kind: ErrorKind,
    },
    TooLarge {
        fixture: FixtureKind,
        maximum: usize,
    },
    Fixture(TraceInputFailure),
}

impl CommandFailure {
    const fn exit_code(&self) -> u8 {
        match self {
            Self::Read { .. } | Self::TooLarge { .. } => EXIT_INPUT,
            Self::Fixture(TraceInputFailure::Trace(failure)) if is_trace_failure(failure) => {
                EXIT_TRACE_FAILURE
            }
            Self::Fixture(_) => EXIT_FIXTURE,
        }
    }
}

impl Display for CommandFailure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read { fixture, kind } => {
                write!(formatter, "{fixture} could not be read ({kind:?})")
            }
            Self::TooLarge { fixture, maximum } => {
                write!(
                    formatter,
                    "{fixture} exceeds the {maximum}-byte input limit"
                )
            }
            Self::Fixture(source) => Display::fmt(source, formatter),
        }
    }
}

const fn is_trace_failure(failure: &TraceFailure) -> bool {
    matches!(
        failure,
        TraceFailure::StateMismatch { .. }
            | TraceFailure::OpcodeMismatch { .. }
            | TraceFailure::UnsupportedOpcode { .. }
            | TraceFailure::Cpu { .. }
            | TraceFailure::Bus { .. }
    )
}

pub fn execute<I, S>(arguments: I, stdout: &mut dyn Write, stderr: &mut dyn Write) -> u8
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let arguments: Vec<OsString> = arguments.into_iter().map(Into::into).collect();
    if arguments.is_empty() || is_command(&arguments[0], "synthetic") && arguments.len() == 1 {
        return run_synthetic_command(stdout, stderr);
    }
    if arguments.len() == 1
        && (is_command(&arguments[0], "--help") || is_command(&arguments[0], "-h"))
    {
        return write_status(stdout, USAGE, EXIT_OK);
    }
    if arguments.len() == 3 && is_command(&arguments[0], "nes-trace") {
        return run_trace_command(&arguments[1], &arguments[2], stdout, stderr);
    }

    write_status(stderr, USAGE, EXIT_USAGE)
}

fn is_command(value: &OsStr, expected: &str) -> bool {
    value == OsStr::new(expected)
}

fn run_synthetic_command(stdout: &mut dyn Write, stderr: &mut dyn Write) -> u8 {
    match run_synthetic(30) {
        Ok(summary) => {
            let result = writeln!(
                stdout,
                "synthetic-v1 tick={} video_frames={} audio_packets={} audio_frames={} video_hash={:016x} audio_hash={:016x} event_hash={:016x}",
                summary.final_time.ticks(),
                summary.video_frames,
                summary.audio_packets,
                summary.audio_frames,
                summary.video_hash,
                summary.audio_hash,
                summary.event_hash
            );
            if result.is_ok() {
                EXIT_OK
            } else {
                EXIT_TRACE_FAILURE
            }
        }
        Err(error) => write_status(
            stderr,
            &format!("synthetic run failed: {error}"),
            EXIT_TRACE_FAILURE,
        ),
    }
}

fn run_trace_command(
    rom_path: &OsStr,
    log_path: &OsStr,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
) -> u8 {
    match load_and_compare(rom_path, log_path) {
        Ok(summary) => write_trace_summary(stdout, summary),
        Err(error) => {
            let exit_code = error.exit_code();
            if write_failure(stderr, &error).is_ok() {
                exit_code
            } else {
                EXIT_TRACE_FAILURE
            }
        }
    }
}

fn write_failure(stderr: &mut dyn Write, error: &CommandFailure) -> std::io::Result<()> {
    writeln!(stderr, "nes-trace failed: {error}")?;
    let CommandFailure::Fixture(TraceInputFailure::Trace(failure)) = error else {
        return Ok(());
    };
    match failure {
        TraceFailure::InitialStateNotRepresentable {
            line,
            expected,
            normalized,
        } => writeln!(
            stderr,
            "nes-trace-detail-v1 line={line} expected_pc={:04X} expected_a={:02X} expected_x={:02X} expected_y={:02X} expected_p={:02X} expected_sp={:02X} expected_cycles={} actual_pc={:04X} actual_a={:02X} actual_x={:02X} actual_y={:02X} actual_p={:02X} actual_sp={:02X} actual_cycles={}",
            expected.pc,
            expected.a,
            expected.x,
            expected.y,
            expected.status,
            expected.sp,
            expected.total_cycles,
            normalized.pc,
            normalized.a,
            normalized.x,
            normalized.y,
            normalized.status,
            normalized.sp,
            normalized.total_cycles
        ),
        TraceFailure::StateMismatch {
            line,
            expected,
            actual,
            previous_expected,
        } => writeln!(
            stderr,
            "nes-trace-divergence-v1 line={line} expected_pc={:04X} expected_a={:02X} expected_x={:02X} expected_y={:02X} expected_p={:02X} expected_sp={:02X} expected_cycles={} actual_pc={:04X} actual_a={:02X} actual_x={:02X} actual_y={:02X} actual_p={:02X} actual_sp={:02X} actual_cycles={} previous_expected_pc={}",
            expected.pc,
            expected.a,
            expected.x,
            expected.y,
            expected.status,
            expected.sp,
            expected.total_cycles,
            actual.pc,
            actual.a,
            actual.x,
            actual.y,
            actual.status,
            actual.sp,
            actual.total_cycles,
            previous_expected
                .map(|state| format!("{:04X}", state.pc))
                .unwrap_or_else(|| "none".to_owned())
        ),
        TraceFailure::OpcodeMismatch {
            line,
            pc,
            expected,
            actual,
        } => writeln!(
            stderr,
            "nes-trace-divergence-v1 line={line} pc={pc:04X} expected_opcode={} actual_opcode={}",
            hex_bytes(expected),
            hex_bytes(actual)
        ),
        TraceFailure::OpcodeLengthMismatch {
            line,
            opcode,
            expected,
            actual,
        } => writeln!(
            stderr,
            "nes-trace-detail-v1 line={line} opcode={opcode:02X} expected_length={expected} actual_length={actual}"
        ),
        TraceFailure::UnsupportedOpcode { .. } => Ok(()),
        TraceFailure::Cpu { .. }
        | TraceFailure::Bus { .. }
        | TraceFailure::UnsupportedRegion(_) => Ok(()),
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(encoded, "{byte:02X}");
    }
    encoded
}

fn load_and_compare(rom_path: &OsStr, log_path: &OsStr) -> Result<TraceSummary, CommandFailure> {
    let image = read_bounded(rom_path, FixtureKind::Rom, MAX_NROM_IMAGE_BYTES)?;
    let reference = read_bounded(log_path, FixtureKind::ReferenceLog, MAX_REFERENCE_LOG_BYTES)?;
    compare_nrom_trace_bytes(&image, &reference).map_err(CommandFailure::Fixture)
}

fn read_bounded(
    path: &OsStr,
    fixture: FixtureKind,
    maximum: usize,
) -> Result<Vec<u8>, CommandFailure> {
    let file = File::open(Path::new(path)).map_err(|source| CommandFailure::Read {
        fixture,
        kind: source.kind(),
    })?;
    let mut bytes = Vec::new();
    file.take((maximum as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|source| CommandFailure::Read {
            fixture,
            kind: source.kind(),
        })?;
    if bytes.len() > maximum {
        return Err(CommandFailure::TooLarge { fixture, maximum });
    }
    Ok(bytes)
}

fn write_trace_summary(stdout: &mut dyn Write, summary: TraceSummary) -> u8 {
    let state = summary.final_state;
    let result = writeln!(
        stdout,
        "nes-trace-v1 rows_matched={} transitions_verified={} final_pc={:04X} final_a={:02X} final_x={:02X} final_y={:02X} final_p={:02X} final_sp={:02X} final_cycles={}",
        summary.rows_matched,
        summary.transitions_verified,
        state.pc,
        state.a,
        state.x,
        state.y,
        state.status,
        state.sp,
        state.total_cycles
    );
    if result.is_ok() {
        EXIT_OK
    } else {
        EXIT_TRACE_FAILURE
    }
}

fn write_status(writer: &mut dyn Write, message: &str, status: u8) -> u8 {
    if writeln!(writer, "{message}").is_ok() {
        status
    } else {
        EXIT_TRACE_FAILURE
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct FixtureDirectory(PathBuf);

    impl FixtureDirectory {
        fn new() -> Self {
            let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "panda-uni-emu-cli-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("unique fixture directory is created");
            Self(path)
        }

        fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
            let path = self.0.join(name);
            fs::write(&path, bytes).expect("generated fixture is written");
            path
        }
    }

    impl Drop for FixtureDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn generated_image(program: &[u8]) -> Vec<u8> {
        let mut bytes = vec![0; 16 + 16 * 1024 + 8 * 1024];
        bytes[0..4].copy_from_slice(b"NES\x1a");
        bytes[4] = 1;
        bytes[5] = 1;
        bytes[16..16 + program.len()].copy_from_slice(program);
        bytes
    }

    fn run(arguments: Vec<OsString>) -> (u8, String, String) {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let status = execute(arguments, &mut stdout, &mut stderr);
        (
            status,
            String::from_utf8(stdout).expect("stdout is UTF-8"),
            String::from_utf8(stderr).expect("stderr is UTF-8"),
        )
    }

    #[test]
    fn zero_arguments_preserve_the_synthetic_smoke_contract() {
        let (status, stdout, stderr) = run(Vec::new());
        assert_eq!(status, EXIT_OK);
        assert!(stdout.starts_with("synthetic-v1 tick=30 "));
        assert!(stderr.is_empty());
    }

    #[test]
    fn help_and_invalid_usage_do_not_echo_arguments() {
        let (status, stdout, stderr) = run(vec![OsString::from("--help")]);
        assert_eq!(status, EXIT_OK);
        assert_eq!(stdout.trim_end(), USAGE);
        assert!(stderr.is_empty());

        let marker = "private-operator-path";
        let (status, stdout, stderr) = run(vec![OsString::from(marker)]);
        assert_eq!(status, EXIT_USAGE);
        assert!(stdout.is_empty());
        assert_eq!(stderr.trim_end(), USAGE);
        assert!(!stderr.contains(marker));
    }

    #[test]
    fn generated_operator_files_run_through_the_real_cli_boundary() {
        let directory = FixtureDirectory::new();
        let rom = directory.write("generated.nes", &generated_image(&[0xa9, 0x01, 0xaa, 0xea]));
        let log = directory.write(
            "generated.log",
            b"C000 A9 01 LDA A:00 X:00 Y:00 P:24 SP:FD CYC:7\n\
              C002 AA TAX A:01 X:00 Y:00 P:24 SP:FD CYC:9\n\
              C003 EA NOP A:01 X:01 Y:00 P:24 SP:FD CYC:11",
        );
        let (status, stdout, stderr) = run(vec![
            OsString::from("nes-trace"),
            rom.into_os_string(),
            log.into_os_string(),
        ]);
        assert_eq!(status, EXIT_OK);
        assert!(stdout.starts_with("nes-trace-v1 rows_matched=3 transitions_verified=2 "));
        assert!(stderr.is_empty());
    }

    #[test]
    fn missing_input_and_hostile_log_contents_are_not_echoed() {
        let directory = FixtureDirectory::new();
        let private_marker = "secret-rom-location";
        let missing = directory.0.join(private_marker);
        let log = directory.write("generated.log", b"invalid");
        let (status, _, stderr) = run(vec![
            OsString::from("nes-trace"),
            missing.into_os_string(),
            log.into_os_string(),
        ]);
        assert_eq!(status, EXIT_INPUT);
        assert!(!stderr.contains(private_marker));

        let rom = directory.write("generated.nes", &generated_image(&[0xea]));
        let hostile_marker = "DO-NOT-ECHO-THIS";
        let hostile = directory.write("hostile.log", hostile_marker.as_bytes());
        let (status, _, stderr) = run(vec![
            OsString::from("nes-trace"),
            rom.into_os_string(),
            hostile.into_os_string(),
        ]);
        assert_eq!(status, EXIT_FIXTURE);
        assert!(!stderr.contains(hostile_marker));
    }

    #[test]
    fn oversized_reads_are_bounded_and_classified() {
        let directory = FixtureDirectory::new();
        let path = directory.write("oversized.bin", b"12345");
        assert!(matches!(
            read_bounded(path.as_os_str(), FixtureKind::ReferenceLog, 4),
            Err(CommandFailure::TooLarge {
                fixture: FixtureKind::ReferenceLog,
                maximum: 4
            })
        ));
    }

    #[test]
    fn malformed_fixtures_and_trace_divergence_have_distinct_statuses() {
        let directory = FixtureDirectory::new();
        let mut unsupported = generated_image(&[0xea]);
        unsupported[6] = 0x10;
        let rom = directory.write("unsupported.nes", &unsupported);
        let log = directory.write(
            "generated.log",
            b"C000 EA NOP A:00 X:00 Y:00 P:24 SP:FD CYC:7",
        );
        let (status, _, _) = run(vec![
            OsString::from("nes-trace"),
            rom.into_os_string(),
            log.into_os_string(),
        ]);
        assert_eq!(status, EXIT_FIXTURE);

        let rom = directory.write("valid.nes", &generated_image(&[0xea, 0xea]));
        let log = directory.write(
            "mismatch.log",
            b"C000 EA NOP A:00 X:00 Y:00 P:24 SP:FD CYC:7\n\
              C001 EA NOP A:01 X:00 Y:00 P:24 SP:FD CYC:9",
        );
        let (status, stdout, stderr) = run(vec![
            OsString::from("nes-trace"),
            rom.into_os_string(),
            log.into_os_string(),
        ]);
        assert_eq!(status, EXIT_TRACE_FAILURE);
        assert!(stdout.is_empty());
        assert!(stderr.contains("CPU state mismatch at reference line 2"));
        assert!(stderr.contains("nes-trace-divergence-v1 line=2"));
        assert!(stderr.contains("expected_a=01"));
        assert!(stderr.contains("actual_a=00"));
        assert!(!stderr.contains("mismatch.log"));
    }
}
