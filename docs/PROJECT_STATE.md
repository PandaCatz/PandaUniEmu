# Project State

Last updated: 2026-07-13

## Current phase

The Phase 1 headless foundation is implemented. The Phase 2 NTSC NES vertical
slice is at the independent CPU-trace boundary. Seven functional workspace
crates exist. The mapper-0 CPU bus and a generated trace runner now work, but no
operator-supplied `nestest` pair has been run. There is no PPU/APU, complete NES
machine, host frontend, or playable emulation.

## Implemented this session

- Added `format-nestest-log`, a separate ASCII byte parser with total-size,
  line-size, and row-count limits; strict state fields; opcode bounds; and
  non-panicking errors that do not echo hostile input.
- Added `NromCpuBus` with 2 KiB RAM mirroring, NROM-128/NROM-256 PRG mapping,
  an optional 8 KiB PRG-memory window, trainer preload at `$7000-$71FF`, ignored
  ROM writes, side-effect-free diagnostic peeks, and first-fault reporting for
  unimplemented PPU/APU/I/O addresses.
- Tightened mapper-0 cartridge validation so unsupported PRG-memory layouts and
  trainers without a PRG-memory window fail before machine construction.
- Added a generated reference runner that initializes from row one, verifies
  representable status bits, compares state/cycles/opcode bytes before every
  transition, checks opcode length, uses the final row as a verified end-state
  sentinel, and stops on the first CPU or bus fault.
- Added a second cargo-fuzz target and redistribution-safe trace seed. The
  Windows launcher now runs both the iNES and trace-log parsers.
- Reviewed Kevin Horton's `nestest` V1.00 identity, immutable mirror pin,
  expected hashes, and 8,991-row convention. No explicit redistribution license
  was found; no external fixture was downloaded, committed, or run.
- Added `retro-cli nes-trace <ROM_PATH> <LOG_PATH>` with bounded reads,
  OS-native path handling, sanitized first-divergence details, stable exit
  statuses, and generated real-filesystem boundary tests.
- Added a byte-oriented testkit entry point that parses each boundary once and
  preserves image, cartridge, log, and trace failure layers.

The mapper-bus/reference-runner checkpoint passed fresh adversarial review, a
deletion-safe 39-file publisher preview, and the Windows/Linux CI matrix. The
new provenance/operator-CLI changes passed fresh adversarial review after all
three P2 findings were fixed. The deletion-safe 42-file publisher preview also
passed; GitHub checkpoint and CI are still pending.

## Verification performed

Verified locally on Windows x86-64 with Rust/Cargo 1.96.0 and
nightly-2026-07-12 on 2026-07-13:

- `cargo fmt --all -- --check` passed.
- Clippy passed for the workspace, all targets, and all features with warnings
  denied.
- Debug tests: 58 passed, 0 failed.
- Release tests: 58 passed, 0 failed; doc tests passed.
- Both parser fuzz targets completed 10,000 AddressSanitizer executions with no
  crash. Generated seeds contain no third-party ROM or reference-log bytes.
- The release CLI retained tick 30, video hash `2d1f1e3d37030229`, audio hash
  `b2bdf29fe8dd6d45`, and event hash `2343096cdf497a5e`.
- The release CLI help path and generated operator-file path were exercised; the
  latter is also covered by unit tests because no external fixture is present.
- The release binary matched a three-row generated trace and returned status `1`
  for an unsupported opcode on a one-row final sentinel, without printing paths.

Mapper-0 bus/reference-runner checkpoint
`505a73c02d69f309cad37d7c85e7520d7e5ab6b6` is published. GitHub Actions run
`29254844214` passed stable tests and both 10,000-run parser ASan fuzz targets on
Windows 2025 and Ubuntu 24.04.

## Key decisions

- External ROMs and reference logs always remain operator-supplied and
  uncommitted; only source metadata, hashes, and sanitized results may be stored.
- The reviewed public `nestest` distribution has no located explicit license;
  public availability is not treated as permission to redistribute it.
- `format-nestest-log` has no CPU, NES runtime, Bevy, or frontend dependency.
- Unimplemented NES I/O records an explicit bus fault. Returning deterministic
  latched data is only a way to finish the current CPU call safely, not a claim
  that the device access is supported.
- The runner uses the first reference row's raw cycle convention and never
  renormalizes later rows.
- The current milestone remains instruction-oriented, not bus-cycle accurate.

## Next action

Obtain an operator-supplied pair matching a reviewed identity in
`compatibility/NESTEST_PROVENANCE.md`, record local hashes under the ignored
`external-fixtures/` directory, and run the new CLI. Fix every divergence before
moving to interrupt or PPU work. Generated tests are not independent proof.

## Open decisions

1. Initial source license before accepting outside contributions.
2. Whether Linux is a release target or a CI-only target for the first build.
3. Exact independently licensed external CPU test suite and acquisition source.
4. Final product name.

## Environment notes

`git` is not installed/on `PATH`. Publishing uses the authenticated, allowlisted
Git Data API script, preserves remote-only files by default, and requires a dry
run. On x64 Windows, use `tools/run-fuzz.ps1` so the Visual Studio AddressSanitizer
runtime is placed on the child process path.
