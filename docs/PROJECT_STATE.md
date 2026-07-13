# Project State

Last updated: 2026-07-13

## Current phase

Phase 1 headless foundation in progress. A Cargo workspace and five functional
crates exist. No console CPU, runtime scheduler, PPU/APU, host frontend, ROMs,
firmware, or test-ROM bytes have been added.

## Shipped this session

- Reviewed the 661-line source proposal and recorded prioritized corrections.
- Completed an independent subagent review and incorporated its surviving
  timing, ownership, mapper, GBA, testing, and scope findings.
- Narrowed the project to first-party NES, GBA, Genesis, and SNES cores.
- Established dependency direction, deterministic timing policy, defensive data
  boundaries, verification gates, roadmap, and initial build order.
- Verified the local Rust and Cargo tools report version 1.96.0 on
  `x86_64-pc-windows-msvc`.
- Added `CLAUDE.md` as the canonical cross-session implementation handoff.
- Implemented shared core contracts, defensive iNES/NES 2.0 parsing, an owning
  mapper-0 cartridge boundary, deterministic synthetic output, capture hashes,
  and a real headless CLI.
- Added a Windows/Linux CI workflow with format, Clippy, debug test, and release
  test gates.
- Published the foundation to `PandaCatz/PandaUniEmu` at commit
  `b7c3182a8672db0bed814951cd9d959fa8eb8f7a`.

## Key decisions

- Build a Rust-native contract rather than freezing the source proposal's
  frame-only `Console` trait.
- Headless execution and test oracles precede a feature-rich frontend.
- Use exact emulated time; never use VSync or wall-clock sleeps inside a core.
- Keep parser crates independent of frontend/runtime dependencies.
- Default core order: NES → GBA → Genesis → SNES.
- Direct `winit`/`wgpu`/`cpal` adapters are the initial frontend direction;
  Bevy is deferred unless later UI needs justify it.
- No original game/firmware/test bytes are committed.

## Verification performed

- Source size: 36,046 bytes, 661 lines.
- Local toolchain commands:
  - `rustc --version --verbose` → 1.96.0
  - `cargo --version --verbose` → 1.96.0
- Primary documentation checked for NES odd-frame timing, SNES variable CPU
  timing, SNES header file locations, GBA LCD/system-cycle timing, mGBA HLE BIOS,
  and current `wgpu` availability.
- Documentation check: six non-empty UTF-8 Markdown files, 31,839 bytes, five
  original local Markdown links resolved, and no mojibake detected.
- `cargo fmt --all -- --check` passed.
- Clippy passed for the workspace, all targets, and all features with warnings
  denied.
- Debug tests: 20 passed, 0 failed.
- Release tests: 20 passed, 0 failed; doc tests passed.
- Release CLI produced tick 30, three video frames, seven audio packets / 28
  audio frames, video hash `2d1f1e3d37030229`, and audio hash
  `b2bdf29fe8dd6d45`; unified ordered event hash `2343096cdf497a5e`.
- GitHub Actions run `29249023446` completed successfully for the published
  foundation on the configured Windows/Linux matrix.

## Next action

Add parser fuzzing and the NES oracle provenance/acceptance matrix, then begin
the trace-first `cpu-6502` crate. Do not add the host frontend before the
headless CPU gate.

## Open decisions

1. Initial license for original code (recommended: MPL-2.0 or GPL-3.0-or-later;
   decide before accepting outside contributions).
2. Whether Windows-only is acceptable for the first playable milestone or Linux
   must pass CI from day one.
3. Whether milestone-one NES supports only NTSC or also PAL/Dendy.
4. Which independently licensed test suites may be downloaded locally and which
   may be represented only by user-provided paths/hashes.
5. Product name; “Universal Retro Emulator” is descriptive working text only.

## Known environment issue

`git` is not installed/on `PATH`. GitHub CLI is authenticated and the public
repository `PandaCatz/PandaUniEmu` is publishing successfully through the
allowlisted Git Data API script. Continue using dry runs before each checkpoint
until Git is installed.
