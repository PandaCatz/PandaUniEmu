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

The workspace contains six functional crates:

- `retro-core`: shared deterministic contracts and typed output/input metadata.
- `format-ines`: defensive borrowed parser for iNES and NES 2.0 images.
- `core-nes`: parsed-cartridge ownership boundary and mapper-0 image validation;
  no active CPU bus or running machine yet.
- `retro-testkit`: deterministic synthetic core, capture sink, and stable hashes.
- `retro-cli`: real headless executable for the synthetic core.
- `cpu-6502`: trace-first documented 2A03 instruction layer with explicit
  addressing, flags, cycle totals, stack/control flow, and decode metadata.

The fuzz project calls `format_ines::parse` with arbitrary bytes. The checked-in
launcher handles the Windows AddressSanitizer runtime path.

Not implemented: `retro-frontend`, an active NES bus/mapper machine,
per-bus-cycle CPU sequencing, IRQ/NMI sampling, DMA, scheduler, PPU, APU, input
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
- Published the verified foundation as commit
  `b7c3182a8672db0bed814951cd9d959fa8eb8f7a` and its handoff update as commit
  `4515511c154c1e5fe39a45c750bda45a71569ed3`.

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
- Debug tests: 32 passed, 0 failed.
- Release tests: 32 passed, 0 failed; doc tests passed.
- Windows AddressSanitizer fuzz smoke: 10,000 executions completed with no crash.
  cargo-fuzz is pinned at 0.13.2; CI pins nightly-2026-07-12, while the local
  run used rustc nightly commit `be8e82435` dated 2026-07-11.
- Release CLI: final tick `30`; video `3` frames, hash `2d1f1e3d37030229`;
  audio `7` packets / `28` frames, hash `b2bdf29fe8dd6d45`; ordered event hash
  `2343096cdf497a5e`.
- CPU/fuzz checkpoint `a01aac5e9c287770197ebb8b79f0095b87ebbabb`
  is published. GitHub Actions run `29252492924` passed all four jobs: stable
  tests on Windows/Linux and seeded ASan fuzz smoke on Windows/Linux.

## Next tasks, in order

1. Implement the minimal mapper-0 CPU bus and a defensive reference-log parser
   plus trace runner.
2. Run an operator-supplied `nestest` ROM/log without committing either; fix
   every architectural-state or cycle divergence.
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
CPU currently proves generated instruction-level behavior only; it has no
independent oracle result and is not bus-cycle accurate. `core-nes` proves only
the parser-to-runtime ownership boundary. This is not a playable NES emulator.
