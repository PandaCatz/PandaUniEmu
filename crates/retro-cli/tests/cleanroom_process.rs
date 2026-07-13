#![forbid(unsafe_code)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct FixtureDirectory(PathBuf);

impl FixtureDirectory {
    fn new() -> Self {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "panda-uni-emu-process-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("unique process fixture directory is created");
        Self(path)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, bytes).expect("project-owned process fixture is written");
        path
    }
}

impl Drop for FixtureDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn cleanroom_cases_pass_through_the_compiled_cli_process() {
    assert_eq!(retro_testkit::cleanroom_nrom::CASES.len(), 3);
    let directory = FixtureDirectory::new();

    for case in retro_testkit::cleanroom_nrom::CASES {
        let rom = directory.write(&format!("{}.nes", case.name), &case.image());
        let log = directory.write(&format!("{}.log", case.name), case.trace.as_bytes());
        let output = Command::new(env!("CARGO_BIN_EXE_retro-cli"))
            .arg("nes-trace")
            .arg(&rom)
            .arg(&log)
            .output()
            .expect("compiled CLI process starts");
        let stdout = String::from_utf8(output.stdout).expect("CLI stdout is UTF-8");
        let stderr = String::from_utf8(output.stderr).expect("CLI stderr is UTF-8");

        assert_eq!(output.status.code(), Some(0), "{}: {stderr}", case.name);
        assert_eq!(
            stdout,
            format!(
                "nes-trace-v1 fixture_identity=unchecked rows_matched={} transitions_verified={} \
                 final_pc=C102 final_a=5A final_x=01 final_y=00 final_p=25 final_sp=FD \
                 final_cycles={}\n",
                case.rows, case.transitions, case.final_cycles
            ),
            "{} stdout",
            case.name
        );
        assert!(stderr.is_empty(), "{} stderr: {stderr}", case.name);
    }
}
