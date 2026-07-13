# Project State

Last updated: 2026-07-13

## Current phase

The Phase 1 headless foundation is implemented. The Phase 2 NTSC NES vertical
slice is in progress at the CPU-verification boundary. Six functional workspace
crates exist. There is no running NES machine, PPU/APU, host frontend, or
playable emulation. No original ROM, firmware, or test-ROM bytes are committed.

## Shipped locally this session

- Added a separate `cargo-fuzz` project whose `parse_ines` target passes arbitrary
  bytes directly to the parser.
- Added `tools/run-fuzz.ps1`, which locates the Visual Studio x64 AddressSanitizer
  runtime on Windows and gives a specific installation error when it is absent.
- Added a bounded Linux fuzz-smoke CI job pinned to cargo-fuzz 0.13.2 and
  nightly-2026-07-12.
- Added `docs/TEST_PROVENANCE.md` and the NES acceptance matrix.
- Added `cpu-6502`, a trace-first documented-opcode instruction layer with a
  hostile-error boundary for illegal opcodes and 12 selected generated tests.
- Corrected the roadmap so the headless gate precedes the frontend and the first
  playable compatibility target is mapper-0/NROM before broader mapper work.

This checkpoint was published as commit
`a01aac5e9c287770197ebb8b79f0095b87ebbabb` after the full gates, two independent
reviews, and a deletion-safe publisher dry run passed.

## Key decisions

- Milestone one is NTSC NES/Famicom; PAL and Dendy are later compatibility work.
- Headless execution and independent test oracles precede the host frontend.
- The current CPU is deliberately instruction-trace oriented. It does not yet
  claim bus-cycle ordering, IRQ/NMI sampling, DMA interaction, unofficial
  opcodes, or `nestest` equivalence.
- External test ROMs and logs remain operator-supplied and uncommitted even when
  redistribution terms would permit publication; store only hashes and results.
- Direct `winit`/`wgpu`/`cpal` adapters remain the frontend direction after the
  deterministic headless NROM gate.

## Verification performed

Verified locally on Windows x86-64 with Rust/Cargo 1.96.0 on 2026-07-13:

- Format check passed.
- Clippy passed for the workspace, all targets, and all features with warnings
  denied.
- Debug tests: 32 passed, 0 failed.
- Release tests: 32 passed, 0 failed; doc tests passed.
- The Windows ASan fuzz launcher completed 10,000 `parse_ines` executions with
  generated full-image seeds, a 64 KiB mutation limit, and no crash. The local
  rustc nightly commit was `be8e82435` (2026-07-11). The original launch failure
  was a missing `PATH` entry for the installed
  `clang_rt.asan_dynamic-x86_64.dll`; the launcher now discovers it.
- The release CLI produced tick 30, three video frames, seven audio packets / 28
  audio frames, video hash `2d1f1e3d37030229`, audio hash
  `b2bdf29fe8dd6d45`, and event hash `2343096cdf497a5e`.

GitHub Actions run `29252492924` passed all four jobs for checkpoint
`a01aac5e9c287770197ebb8b79f0095b87ebbabb`: stable format/Clippy/debug/release/
CLI gates and seeded ASan fuzz smoke on both Windows 2025 and Ubuntu 24.04.

## Next action

Implement the minimal mapper-0 CPU bus, defensive reference-log parser, and
trace runner before comparing an operator-supplied `nestest` ROM/log. Record
only fixture hashes and the first divergence outside the repository.

## Open decisions

1. Initial source license before accepting outside contributions.
2. Whether Linux is a release target or a CI-only target for the first build.
3. Exact independently licensed external suites and local acquisition process.
4. Final product name.

## Environment notes

`git` is not installed/on `PATH`. GitHub CLI is authenticated, and publishing
uses the allowlisted Git Data API script. Always dry-run the snapshot before a
non-force checkpoint. Windows cargo-fuzz also needs the Visual Studio C++
AddressSanitizer runtime; use `tools/run-fuzz.ps1` rather than launching the
generated executable directly.
