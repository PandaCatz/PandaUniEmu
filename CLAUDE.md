# Claude Project Handoff

This is the canonical cross-session handoff for Claude and other coding agents.
Read this file, `ROADMAP.md`, and `docs/ARCHITECTURE.md` before changing code.
Update this file at the end of every implementation session.

## Mission

Build deterministic, independently testable Rust cores for NES, GBA, Genesis /
Mega Drive, and SNES behind one frontend. Current scope is the Phase 1 headless
foundation and then the NTSC NES vertical slice.

## Non-negotiable rules

- Run and measure before claiming behavior works.
- Core simulation uses emulated integer time. It never sleeps, reads wall time,
  opens host devices, or waits for VSync.
- Format crates accept bytes and return validated structures. Runtime cores do
  not reinterpret raw images.
- All external data is hostile: checked arithmetic, size limits, non-panicking
  errors, truncation/oversize tests, and fuzz targets when available.
- Keep original ROMs, firmware, test-ROM bytes, copyrighted assets, and operator
  paths out of the repository and logs.
- No `unsafe` without a measured release bottleneck, documented invariants,
  focused tests/fuzzing, and a safe baseline.
- Warnings are errors. Do not move to the next console with a red current gate.

## Current state

Phase 1 is in progress. The repository now has a Cargo workspace containing:

- `retro-core`: shared deterministic contracts and typed output/input metadata.
- `format-ines`: defensive borrowed parser for iNES and NES 2.0 images.
- `core-nes`: parsed-cartridge ownership boundary; mapper 0 only, no CPU yet.
- `retro-testkit`: deterministic synthetic core, capture sink, and stable hashes.
- `retro-cli`: real headless executable for the synthetic core.

Not implemented yet: `retro-frontend`, `cpu-6502`, NES bus/mapper devices,
scheduler, PPU, APU, input hardware, SRAM persistence, save states, rewind, or
any GBA/Genesis/SNES code.

## Completed work

- Reviewed and corrected the original 661-line proposal.
- Created architecture, roadmap, build path, and project-state documents.
- Pinned Rust 1.96.0 and added Windows/Linux CI gates.
- Scaffolded the five functional Phase 1/headless crates listed above.
- Implemented exact emulated timestamps, typed video/audio packets, and a
  deadline-based core API.
- Implemented defensive iNES/NES 2.0 parsing with no runtime/frontend dependency.
- Implemented a deterministic synthetic run used by tests and the CLI.

## Required commands

Run from `H:\claaaude\universal-retro-emulator`:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test --release --workspace
cargo run --release -p retro-cli
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/publish-github.ps1 -Message '<verified milestone>' -WhatIf
```

After all gates and the dry run pass, remove `-WhatIf` to publish the allowlisted
snapshot to `PandaCatz/PandaUniEmu`. This is used because Git is not installed;
it creates a normal non-force commit through GitHub's Git Data API.

Record the observed results below. A command listed here is not evidence unless
its result is also recorded.

## Latest verified results

Verified on Windows x86-64 with Rust/Cargo 1.96.0 on 2026-07-13:

- `cargo fmt --all -- --check` — pass.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` — pass.
- `cargo test --workspace --all-targets --all-features` — 20 passed, 0 failed.
- `cargo test --release --workspace` — 20 passed, 0 failed; doc tests passed.
- `cargo run --release -p retro-cli` — pass with:
  - final tick: `30`
  - video: `3` frames, hash `2d1f1e3d37030229`
  - audio: `7` packets / `28` frames, hash `b2bdf29fe8dd6d45`
  - ordered event stream hash: `2343096cdf497a5e`

## Next tasks, in order

1. Publish this verified foundation to `PandaCatz/PandaUniEmu`.
2. Add a parser fuzz target after selecting/installing the fuzzing toolchain.
3. Add an acceptance matrix and legal provenance record for external NES tests.
4. Implement `cpu-6502` with a trace-first bus interface and generated unit tests.
5. Run the operator-supplied `nestest` oracle without committing its ROM/log.
6. Design the active cartridge/mapper contract, then add mapper 0.
7. Add the first master-clock scheduler and PPU timing oracle.
8. Only then resolve and spike `winit`/`wgpu`/`cpal` for `retro-frontend`.

## Decisions still open

- Project source license before accepting outside contributions.
- Whether Linux must be a release target or only a CI target initially.
- Exact external test suites and their local acquisition/provenance procedure.
- Final product name.

## Honest limitations

The synthetic core proves the shared contract/headless capture path only. It is
not console emulation. `core-nes` proves the parser-to-runtime boundary only and
must not be described as a playable or executing NES core.
