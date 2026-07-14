# PandaUniEmu

**A deterministic, evidence-first universal retro-emulator project written in
Rust.** PandaUniEmu is the working repository name; the long-term goal is one
native frontend backed by independently testable NES, Game Boy Advance, Sega
Genesis / Mega Drive, and SNES cores.

[![CI](https://github.com/PandaCatz/PandaUniEmu/actions/workflows/ci.yml/badge.svg)](https://github.com/PandaCatz/PandaUniEmu/actions/workflows/ci.yml)
![Rust 1.96](https://img.shields.io/badge/Rust-1.96-orange?logo=rust)
[![GPL-2.0-or-later](https://img.shields.io/badge/license-GPL--2.0--or--later-blue)](LICENSE)
![AI-assisted](https://img.shields.io/badge/development-AI--assisted-8A2BE2)

> [!IMPORTANT]
> PandaUniEmu is a research-stage, headless emulator foundation. The NES CPU
> and mapper-0 trace checkpoints are real and verified, and a first NTSC
> master-clock/PPU-dot timing model now advances through a machine-owned cycle
> boundary. PPU registers and rendering, APU, input hardware, a complete NES
> machine, and the graphical frontend are not yet implemented. Instruction
> execution yields after every live bus cycle and all 190 sampled instruction
> traces match. Live IRQ, NMI, and reset entry, second-to-last-cycle polling,
> and NMI hijacking are also verified against a pinned transistor-level oracle.
> The project does not currently play games.

## Current status

| Area | Status | Evidence |
|---|---|---|
| Shared deterministic contracts | Complete | Split/single runs, reset, timed input, and capture hashes are tested |
| Defensive NES image parsing | Complete for the current boundary | iNES/NES 2.0 validation, truncation/oversize tests, and parser fuzzing |
| NES 2A03 CPU architecture | Verified checkpoint | All 151 documented encodings pass a pinned 190-vector MIT oracle sample |
| Stable undocumented CPU encodings | Verified checkpoint | The exact 76 encodings exercised by `nestest` pass |
| Independent full CPU trace | Passed | 8,991 rows / 8,990 transitions, final `PC=C66E`, 26,554 cycles |
| CPU cycle stepping and bus order | Verified checkpoint | Each successful `clock` call performs one live read/write; all 190 pinned vectors match byte for byte; `step` remains as a compatibility wrapper |
| IRQ, NMI, and reset | Verified live-cycle checkpoint | Seven bus cycles per entry, second-to-last-cycle polling, branch paths, and BRK/IRQ NMI hijacking match the pinned external transistor oracle |
| Mapper 0 CPU integration | Verified checkpoint | NROM-128, NROM-256, PRG RAM, trainer preload, and clean-room traces |
| NTSC timing foundation | Verified checkpoint | Exact 12:4 master-clock divisors, VBlank edges, 341×262 geometry, and rendering-dependent odd-frame shortening |
| Machine CPU/timing boundary | Verified checkpoint | One CPU bus cycle advances exactly 12 master ticks / 3 PPU dots; VBlank events and exact-cycle bus faults are observable |
| Quality gates | Passing | Workspace debug/release tests, doc tests, warnings-denied Clippy, clean-room checks, strict `nestest`, and parser fuzzing |
| PPU registers/rendering, APU, input, DMA | Not implemented | Next active NES milestones |
| Native frontend | Not implemented | Intentionally deferred until the headless NES core is verified |
| GBA, Genesis / Mega Drive, SNES | Planned | No implementation claims yet |

Published checkpoints and their Windows/Ubuntu CI evidence are available in the
[commit history](https://github.com/PandaCatz/PandaUniEmu/commits/main/) and
[Actions](https://github.com/PandaCatz/PandaUniEmu/actions).

## Why this project is different

- **Deterministic by design.** Core simulation advances in exact emulated time;
  it does not depend on wall-clock sleeps, VSync, or host-device timing.
- **Parsers are isolated trust boundaries.** Format crates accept hostile bytes
  and return validated structures without depending on runtime or frontend code.
- **Evidence before compatibility claims.** Independent traces, pinned vectors,
  generated clean-room cases, fuzzing, and warnings-denied builds define each
  checkpoint.
- **One frontend, separate machines.** Console cores share host contracts—not
  buses, clocks, pixel formats, controllers, or save-state layouts.
- **Failure is explicit.** Missing devices, unsupported mappers, and malformed
  inputs return structured errors instead of silently pretending to work.

## Build path

- [x] Freeze the architecture, legal policy, evidence rules, and Rust toolchain.
- [x] Build deterministic shared contracts, a synthetic core, capture hashing,
  and a real headless CLI.
- [x] Add defensive iNES/NES 2.0 and reference-log parser boundaries with fuzz
  targets.
- [x] Implement and independently sample all 151 documented 2A03 opcode
  encodings.
- [x] Implement mapper-0 CPU addressing, PRG RAM, trainer preload, and
  clean-room NROM integration traces.
- [x] Pass the identity-checked `nestest` V1.00 architectural trace,
  including its 76 stable undocumented encodings.
- [x] Add architectural reset/IRQ/NMI behavior and match all 190 sampled
  instruction bus-access traces.
- [x] Add the first exact NTSC master-clock scheduler and PPU-dot timing oracle.
- [x] Make instruction execution cycle-steppable, retain the whole-instruction
  wrapper, and connect each successful CPU bus cycle to the NTSC scheduler.
- [x] Verify hardware interrupt/reset entry bus order, sub-instruction polling,
  and NMI hijacking against an independent oracle.
- [ ] Add PPU registers, address-space mapping, fetches, and rendering on the
  verified dot timeline.
- [ ] Add APU, controller input, DMA interactions, and deterministic replay.
- [ ] Reach a deterministic headless NROM video/audio checkpoint.
- [ ] Add a tested MMC1 implementation for the operator-owned compatibility
  target.
- [ ] Build the minimal `winit` / `wgpu` / `cpal` frontend without changing core
  behavior.
- [ ] Begin the GBA, Genesis / Mega Drive, and SNES cores only after the NES
  vertical slice closes its gates.

See [BUILD_PATH.md](BUILD_PATH.md) for the implementation order and
[ROADMAP.md](ROADMAP.md) for phase gates and realistic program-level scope.

## Architecture

```text
user-owned bytes
      │
      ▼
format parser ──► validated cartridge ──► console core ──► timed video/audio
                                               ▲
host input ──► timestamp/latch adapter ─────────┘
```

The current workspace contains seven crates:

| Crate | Responsibility |
|---|---|
| `retro-core` | Shared deterministic contracts and typed media/input metadata |
| `format-ines` | Defensive borrowed iNES and NES 2.0 parser |
| `format-nestest-log` | Bounded parser for hostile CPU-reference logs |
| `cpu-6502` | Trace-first Ricoh 2A03 instruction layer |
| `core-nes` | Validated NES cartridge ownership and mapper-0 CPU bus |
| `retro-testkit` | Synthetic core, hashes, clean-room fixtures, and trace comparison |
| `retro-cli` | Headless smoke, generated trace, and strict `nestest-v1` commands |

The dependency and ownership rules are documented in
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Quick start

### Requirements

- Rust/Cargo 1.96.0. The checked-in `rust-toolchain.toml` selects it
  automatically through `rustup`.
- Windows or Linux for the currently tested host environments.
- Visual Studio C++ AddressSanitizer components only if running the Windows fuzz
  gate.

### Build and run the deterministic smoke test

```powershell
git clone https://github.com/PandaCatz/PandaUniEmu.git
cd PandaUniEmu
cargo run --release -p retro-cli
```

The default command runs the project-owned synthetic core. It does not require
or load a ROM.

### Run the standard verification gates

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test --release --workspace
cargo run --release -p retro-cli
```

On Windows, both parser fuzz targets can be exercised with:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/run-fuzz.ps1 -Runs 10000
```

The clean-room NROM evidence gate is:

```powershell
python tools/check-cleanroom-nrom.py
```

The strict external trace requires operator-supplied files matching the exact
identities in
[docs/compatibility/NESTEST_PROVENANCE.md](docs/compatibility/NESTEST_PROVENANCE.md):

```powershell
$romPath = 'external-fixtures\nestest.nes'
$logPath = 'external-fixtures\nestest.log'
cargo run --release -p retro-cli -- nestest-v1 $romPath $logPath
```

External ROMs and logs are never required for ordinary builds or CI.

Developers can optionally reproduce the transistor-level CPU interrupt oracle
in a local temp cache. This is not required to build or run PandaUniEmu:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/verify-perfect6502.ps1 -Acquire -AcceptNonCommercialLicense
```

The required netlist is downloaded directly from its pinned upstream source and
is governed by CC BY-NC-SA 3.0. It is never copied into this repository or a
PandaUniEmu release. See
[the oracle provenance record](docs/compatibility/PERFECT6502_PROVENANCE.md).

## Built with AI

PandaUniEmu is built with substantial AI assistance under human direction. AI
coding agents help analyze architecture, write and revise code, construct
tests, review diffs, and maintain project records.

AI output is treated as untrusted until it passes the same review and evidence
standards as any other contribution: independent oracles, focused regression
tests, warnings-denied linting, debug and release test suites, fuzzing where
applicable, and adversarial diff review. The human operator owns project scope,
legal decisions, fixture authorization, and publication.

## ROMs, fixtures, and clean-room policy

- Use only ROMs and firmware you are legally entitled to use.
- Commercial game data, firmware, audio, artwork, and operator-supplied test
  fixtures are not part of this repository and are not relicensed by it.
- The public `nestest` distribution has no explicit redistribution license
  recorded by this project, so its files remain operator-supplied and ignored.
- Project-owned clean-room NROM diagnostics contain no commercial game data.
- Third-party test material and licenses are recorded in [NOTICE](NOTICE) and
  [docs/TEST_PROVENANCE.md](docs/TEST_PROVENANCE.md).

## Documentation

- [CLAUDE.md](CLAUDE.md) — concise cross-session handoff and exact next tasks.
- [docs/PROJECT_STATE.md](docs/PROJECT_STATE.md) — living implementation and
  verification record.
- [ROADMAP.md](ROADMAP.md) — phased product roadmap and stop rules.
- [BUILD_PATH.md](BUILD_PATH.md) — ordered implementation path.
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — system boundaries, ownership,
  timing, determinism, and safety rules.
- [docs/compatibility/NES_ACCEPTANCE.md](docs/compatibility/NES_ACCEPTANCE.md) —
  NES acceptance matrix.
- [docs/CPU_6502.md](docs/CPU_6502.md) — current CPU evidence and limitations.

## Contributing

Contributions should preserve the project's evidence-first discipline:

1. Keep format parsing isolated from runtime and frontend dependencies.
2. Add focused tests and an independent oracle when one is available.
3. Treat all external bytes as hostile and return errors instead of panicking.
4. Keep simulation deterministic and rendering downstream of emulated state.
5. Run format, warnings-denied clippy, debug tests, and release tests before
   proposing a checkpoint.
6. Never commit operator ROMs, firmware, reference logs, or copyrighted game
   assets.

## License

Copyright (C) 2026 PandaCatz and contributors.

Project-owned source code and documentation are licensed under the GNU General
Public License, version 2 or (at your option) any later version
(`GPL-2.0-or-later`). See [LICENSE](LICENSE) for the complete terms and
[NOTICE](NOTICE) for separately licensed third-party material.

This license does not grant rights to commercial ROMs, firmware, game assets,
or other operator-supplied files.
