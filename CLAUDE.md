# Claude Project Handoff

This is the canonical cross-session handoff for Claude and other coding agents.
Read this file, `ROADMAP.md`, and `docs/ARCHITECTURE.md` before changing code.
Update this file at the end of every implementation session.

## Mission

Build deterministic, independently testable Rust cores for NES, GBA, Genesis /
Mega Drive, and SNES behind one frontend. Current scope is the NTSC NES vertical
slice on top of the completed Phase 1 headless foundation.

## Non-negotiable rules

- Run and measure before claiming behavior works.
- Core simulation uses emulated integer time. It never sleeps, reads wall time,
  opens host devices, or waits for VSync.
- Format crates accept bytes and return validated structures. Runtime cores do
  not reinterpret raw images.
- All external data is hostile: checked arithmetic, size limits, non-panicking
  errors, truncation/oversize tests, and fuzz targets where applicable.
- Keep original ROMs, firmware, test-ROM bytes, copyrighted assets, and operator
  paths out of the repository and logs.
- No `unsafe` without a measured release bottleneck, documented invariants,
  focused tests/fuzzing, and a safe baseline.
- Warnings are errors. Do not move to the next layer with a red current gate.

## Current state

The workspace contains seven functional crates:

- `retro-core`: shared deterministic contracts and typed output/input metadata.
- `format-ines`: defensive borrowed parser for iNES and NES 2.0 images.
- `format-nestest-log`: bounded hostile-byte parser for reference CPU trace rows.
- `core-nes`: parsed-cartridge ownership boundary, mapper-0 validation, and a
  minimal CPU bus with RAM/PRG mirroring and explicit unsupported-I/O faults.
- `retro-testkit`: deterministic synthetic core, capture hashes, and generated
  mapper-0 CPU reference-trace comparison.
- `retro-cli`: headless synthetic smoke executable plus bounded, sanitized
  operator-path NROM/reference-trace command.
- `cpu-6502`: trace-first documented 2A03 instruction layer with explicit
  addressing, flags, cycle totals, stack/control flow, and decode metadata.

The fuzz project calls both format parsers with arbitrary bytes. The checked-in
launcher generates redistribution-safe seeds and handles the Windows
AddressSanitizer runtime path.

Not implemented: `retro-frontend`, a complete NES machine, PPU/APU/I/O bus
devices, per-bus-cycle CPU sequencing, IRQ/NMI sampling, DMA, scheduler, input
hardware, SRAM persistence, save states, rewind, or any GBA/Genesis/SNES code.

## Completed work

- Reviewed and corrected the original 661-line proposal.
- Created the architecture, roadmap, build path, and living project-state docs.
- Pinned Rust 1.96.0 and added Windows/Linux CI gates.
- Implemented exact emulated timestamps, typed video/audio packets, and a
  deadline-based core API.
- Implemented defensive iNES/NES 2.0 parsing with no runtime/frontend dependency.
- Implemented a deterministic synthetic run used by tests and the CLI.
- Added parser fuzzing, an NES acceptance matrix, and external-test provenance.
- Implemented decode entries for the canonical 151 documented opcode encodings
  and selected generated semantic tests in a trace-first CPU layer. The metadata
  and full behavior still need an independent `nestest` comparison.
- Implemented an NROM-128/NROM-256 CPU bus, trainer/PRG-memory validation,
  side-effect-free diagnostic reads, and explicit faults for missing devices.
- Added an isolated bounded `nestest`-style log parser, generated end-to-end
  trace comparison, and a second ASan fuzz target. No external fixture was used.
- Pinned Kevin Horton's `nestest` V1.00 identity and hashes. No explicit
  redistribution license was found, so fixtures remain operator-supplied only.
- Added `retro-cli nes-trace <ROM_PATH> <LOG_PATH>` with bounded reads,
  path/content-safe diagnostics, stable exit statuses, and generated real-file
  boundary tests.
- Published the verified foundation as commit
  `b7c3182a8672db0bed814951cd9d959fa8eb8f7a` and its handoff update as commit
  `4515511c154c1e5fe39a45c750bda45a71569ed3`.
- Published the mapper-0 bus/reference-runner checkpoint as commit
  `505a73c02d69f309cad37d7c85e7520d7e5ab6b6`.
- Published the provenance/operator-CLI checkpoint as commit
  `cb4e2de00bb843bef37fa5ef0dc1dc8c08b6a27f`.

## Required commands

Run from `H:\claaaude\universal-retro-emulator`:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test --release --workspace
cargo run --release -p retro-cli
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/run-fuzz.ps1 -Runs 10000
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/publish-github.ps1 -Message '<verified milestone>' -WhatIf
```

After all gates and the dry run pass, remove `-WhatIf` to publish the allowlisted
snapshot to `PandaCatz/PandaUniEmu`. Git is not installed, so the script creates
a normal non-force commit through GitHub's Git Data API. Existing remote files
are preserved by default; deletion requires the explicit
`-DeleteMissingManagedFiles` switch and review of its preview.

## Latest verified results

Verified on Windows x86-64 with Rust/Cargo 1.96.0 on 2026-07-13:

- Format check passed.
- Clippy passed for the workspace, all targets, and all features with warnings
  denied.
- Debug tests: 58 passed, 0 failed.
- Release tests: 58 passed, 0 failed; doc tests passed.
- Windows AddressSanitizer fuzz smoke: 10,000 executions per parser completed
  with no crash.
  cargo-fuzz is pinned at 0.13.2; CI pins nightly-2026-07-12, while the local
  run used rustc nightly commit `be8e82435` dated 2026-07-11.
- Release CLI: final tick `30`; video `3` frames, hash `2d1f1e3d37030229`;
  audio `7` packets / `28` frames, hash `b2bdf29fe8dd6d45`; ordered event hash
  `2343096cdf497a5e`.
- Mapper-0 bus/reference-runner checkpoint
  `505a73c02d69f309cad37d7c85e7520d7e5ab6b6` is published. GitHub Actions run
  `29254844214` passed all four jobs: stable tests and both 10,000-run parser
  ASan fuzz targets on Windows 2025 and Ubuntu 24.04.
- Provenance/operator-CLI checkpoint
  `cb4e2de00bb843bef37fa5ef0dc1dc8c08b6a27f` is published. GitHub Actions run
  `29257679328` passed all four stable/fuzz jobs on Windows 2025 and Ubuntu
  24.04. No external fixture was found or run.

## Next tasks, in order

1. Obtain an operator-supplied ROM/log matching a reviewed identity in
   `docs/compatibility/NESTEST_PROVENANCE.md`, record local hashes in the ignored
   run record, and run the new CLI without committing either fixture.
2. Fix every observed architectural-state or cycle divergence and rerun the
   complete external trace until it passes.
3. Add focused IRQ, NMI, reset, and bus-access-order tests, then implement the
   missing interrupt sampling and per-cycle behavior.
4. Add the first master-clock scheduler and dot-timed PPU oracle.
5. Reach a deterministic headless NROM video/audio checkpoint.
6. Only then resolve and spike `winit`/`wgpu`/`cpal` for `retro-frontend`.

## Decisions still open

- Project source license before accepting outside contributions.
- Whether Linux is a release target or a CI-only target initially.
- Exact independently licensed external test suites and acquisition process.
- Final product name.

## Honest limitations

The synthetic core proves only the shared contract/headless capture path. The
CPU and mapper-0 bus currently prove generated instruction-level behavior only;
they have no independent oracle result and are not bus-cycle accurate. PPU/APU/
I/O are intentionally faulted rather than simulated. This is not a playable NES
emulator.
