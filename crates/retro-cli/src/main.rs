#![forbid(unsafe_code)]

use retro_testkit::run_synthetic;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run_synthetic(30) {
        Ok(summary) => {
            println!(
                "synthetic-v1 tick={} video_frames={} audio_packets={} audio_frames={} video_hash={:016x} audio_hash={:016x} event_hash={:016x}",
                summary.final_time.ticks(),
                summary.video_frames,
                summary.audio_packets,
                summary.audio_frames,
                summary.video_hash,
                summary.audio_hash,
                summary.event_hash
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("synthetic run failed: {error}");
            ExitCode::FAILURE
        }
    }
}
