# Build Path

This is the execution order for turning the plan into code. The shared contract,
iNES parser, NES cartridge boundary, testkit, CLI, parser fuzz targets,
trace-first CPU layer, mapper-0 CPU bus, generated trace runner, and bounded
operator-path trace command now exist. Independent CPU-oracle verification,
PPU/APU/runtime scheduling, and the frontend remain work, not claims of
implementation.

## 1. Scaffold only the shared contracts and NES slice

Proposed workspace:

```text
universal-retro-emulator/
├── Cargo.toml
├── rust-toolchain.toml
├── deny.toml
├── README.md
├── ROADMAP.md
├── BUILD_PATH.md
├── docs/
│   ├── ARCHITECTURE.md
│   ├── PROPOSAL_REVIEW.md
│   ├── PROJECT_STATE.md
│   ├── decisions/
│   └── compatibility/
├── crates/
│   ├── retro-core/          # contracts and value types; no window/GPU/audio API
│   ├── retro-testkit/       # headless runner, traces, hashes, fixture discovery
│   ├── retro-cli/           # deterministic headless executable
│   ├── retro-frontend/      # winit/wgpu/cpal/gilrs adapters
│   ├── format-ines/         # bytes -> validated cartridge; no frontend deps
│   ├── format-nestest-log/  # hostile reference bytes -> validated trace rows
│   ├── cpu-6502/
│   └── core-nes/
├── tests/
│   ├── generated/           # repository-safe generated fixtures
│   └── manifests/           # hashes/metadata, only when redistribution permits
├── benches/
└── tools/
```

Do not create empty crates for all four systems on day one. Add these only when
their phase begins:

```text
format-gba  cpu-arm7tdmi  core-gba
format-genesis  cpu-m68000  cpu-z80  core-genesis
format-snes  cpu-w65c816  cpu-spc700  core-snes
```

## 2. Establish the contracts with a synthetic core

The contract must represent:

- immutable system metadata and runtime-selected region/timing;
- typed input ports rather than one universal `u16` bitfield;
- video frames with width, height, pitch, pixel format, aspect, field, and
  emulated timestamp;
- interleaved audio with rate, channel layout, sample count, and timestamp;
- execution until a target emulated time or event, not only `step_frame()`;
- structured reset/power events and explicit errors;
- deterministic configuration and capability queries.

The frontend owns host clocks and devices. Cores own emulated time and never
sleep, wait for VSync, open windows, or call host audio APIs.

## 3. Build parser boundaries before machine execution

For every format crate:

1. Accept `&[u8]` or a bounded reader and return validated immutable structures.
2. Check magic/signatures as evidence, not as the only validity rule.
3. Use checked offset/length/count arithmetic and explicit maximum sizes.
4. Reject truncation, impossible mappings, unsupported hardware, and trailing
   structures that cannot be explained.
5. Keep raw input and copyrighted fixture bytes out of logs and snapshots.
6. Unit-test valid generated samples, every truncation point, oversized fields,
   integer-boundary cases, and random input with no panics.

The core consumes parsed cartridges; it must not reinterpret raw ROM bytes.

## 4. Implement CPU with trace-first verification

- Keep execution state separate from the system bus interface.
- Define wrapping arithmetic and status-flag behavior explicitly.
- Compare each step against an independent oracle using PC, opcode bytes,
  registers, flags, memory effects, and elapsed cycles.
- Verify interrupt entry, reset, DMA stalls, page crossings, read-modify-write
  bus behavior, and known hardware quirks separately.
- Avoid dynamic allocation, logging, and trait-object calls in the instruction
  hot path unless measurements show they are acceptable.

## 5. Add a master-time scheduler and devices

Use exact integer clock domains or rational conversion against a console master
clock. Device events carry deterministic ordering. A frame boundary is an output
event, not the simulation clock. The first scheduler tests cover simultaneous
events, long-run drift, overflow horizons, and reset determinism.

Then add cartridge mapping, PPU/VDP, APU/audio, DMA, timers, and controllers one
at a time. Each component gets a headless oracle before frontend integration.

## 6. Connect the minimal frontend

- Upload validated frame data to a `wgpu` texture and preserve intended aspect.
- Feed `cpal` from a bounded single-producer/single-consumer buffer.
- Pace from audio-buffer occupancy with a small resampling correction; present
  with VSync when available without changing emulated time.
- Keep frontend input sampling deterministic by timestamping/latching it at
  core-defined boundaries.
- Record underruns, overruns, frame drops, emulation speed, and latency.

Dependency versions in the source proposal are examples from 2024, not a lock
file. As of 2026-07-13, `wgpu` 30 exists; compatibility must be proven together
with the selected Rust toolchain and the other frontend crates before pinning.

## 7. Add states only after normal execution is stable

Do not serialize internal Rust structs directly as the permanent file format.
Define an envelope containing magic, format version, core/system ID, core build
compatibility, region, ROM identity, payload length, and checksum. Decode with
strict limits into a temporary state, validate invariants, then commit it
atomically. Rewind uses the same validated snapshot model with a memory budget.

Netplay is a separate deterministic protocol project; save-state support does
not imply netplay readiness.

## 8. Required gates before each phase closes

Run using the pinned toolchain:

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
cargo test --release --workspace
```

Add, when configured:

```powershell
cargo deny check
cargo audit
cargo fuzz run <parser-target>
```

Also run the relevant real binary or headless core and save only concise,
redistributable evidence: command, toolchain, fixture identity/hash, observed
result, and date. Performance gates use release builds and measured budgets.

## 9. First implementation issue list

1. Completed: freeze the first milestone to NTSC and approve the core contract.
2. Completed: scaffold the headless crates, parser, CLI, CI, and fuzz smoke gate.
3. Completed: implement generated hostile-input parser tests and a trace-first
   documented-opcode CPU layer.
4. Completed: connect a minimal mapper-0 CPU bus and implement a defensive,
   independently fuzzed reference-log parser plus generated trace runner.
5. Completed: pin the intended `nestest` V1.00 distribution identity, record its
   unresolved redistribution status, and add a bounded sanitized local CLI.
6. Match against an operator-supplied `nestest` oracle and close every declared
   semantic/cycle divergence.
7. Add interrupt entry, DMA-stall behavior, the first master-clock scheduler,
   and a dot-timed PPU oracle.
8. Reach the headless NROM video/audio gate, then resolve and spike
   `winit`/`wgpu`/`cpal` for the minimal frontend.
