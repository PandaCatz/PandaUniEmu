# Roadmap

Current progress: the Phase 1 headless contracts, parser, synthetic core,
capture path, CLI, and verification gates are implemented. Phase 2 now has an
instruction-oriented CPU, mapper-0 CPU bus, hostile reference-log parser, and
generated trace runner. A reproducible sample from the pinned MIT RP2A03
single-step suite covers all 151 documented encodings. The bounded operator-path
CLI and exact external-fixture identity are documented and enforced by SHA-256
before parsing. The reviewed `nestest` pair passes 8,991 rows / 8,990
transitions, including its 76 stable undocumented encodings. Project-owned NROM-128/NROM-256 diagnostics, including a
trainer-bearing case, pass pinned py65 architectural traces through the real
mapper and CLI. Architectural interrupt sampling and reset (edge NMI, level IRQ,
the `I`-flag delay, and the seven-cycle sequences) are implemented. All 190
pinned instruction bus traces match, and the first exact NTSC master-clock and
PPU-dot timing checkpoint covers VBlank edges and the odd-frame skipped dot.
Instruction execution now yields after each live bus cycle, while the
machine-owned boundary advances the scheduler by 12 master ticks / 3 PPU dots
and exposes exact-cycle VBlank events and mapper-bus faults. Hardware
interrupt/reset cycle entry and PPU registers/rendering are next. The host
frontend intentionally follows the verified headless NES path. See `CLAUDE.md`
for evidence and exact next tasks.

Estimates below are ranges for one experienced developer working close to full
time. Part-time work, learning the hardware while implementing it, broad game
compatibility, and polished UX can multiply them. The four-core product is a
multi-year program, not a single-release project.

Every phase has a hard exit gate. A phase is not complete because a title boots;
it is complete when the stated oracle and adversarial checks pass.

## Phase 0 — charter, evidence, and toolchain (1–2 weeks)

Deliverables:

- Freeze supported regions for milestone one: NES NTSC first; PAL/Dendy later.
- Record legal policy for user-owned ROMs/firmware and licenses for every test
  suite or borrowed component.
- Select independent test oracles and document how users supply non-redistributable
  fixtures outside the repository.
- Pin a Rust toolchain after a zero-code compatibility spike. The machine
  currently has Rust/Cargo 1.96.0; dependency versions must be resolved at
  scaffold time rather than copied from the proposal.
- Create CI gates and a benchmark method before performance claims exist.

Exit gate: the architecture decisions in `docs/ARCHITECTURE.md` are accepted,
test data has a legal provenance record, and the first NES acceptance matrix is
written.

## Phase 1 — headless platform skeleton (2–4 weeks)

Deliverables:

- Cargo workspace, core contracts, typed video/audio/input metadata, and a
  deterministic headless runner.
- Standalone iNES/NES 2.0 parser crate with checked arithmetic and size limits.
- Structured diagnostics: emulated time, frame events, audio samples produced,
  buffer fill, underruns, and deterministic seed/configuration.

Exit gate: a synthetic test core runs deterministically through the shared
contract and real headless executable; split/single runs and captured event,
video, and audio hashes match. The minimal host frontend is Phase 3 work.

## Phase 2 — NES vertical slice (3–6 months)

Order:

1. Ricoh 2A03 CPU core: official instructions, interrupts, DMA stalls, and bus
   behavior. Add required unofficial opcode encodings based on compatibility
   evidence, not an unsupported count.
2. Cartridge model and mapper 0 for the first playable NROM gate. After that
   gate, add mappers 2, 1, 3, 7, and 4 as tests justify. The interface must
   support mapper reset, IRQs, PPU address/A12 observation, nametable routing,
   mutable reads, and persistent RAM before MMC3 work begins.
3. Dot-timed NTSC PPU including scroll registers, sprite evaluation, VBlank/NMI
   races, and the odd-frame skipped PPU clock.
4. APU including frame counter, DMC DMA interaction, nonlinear mixing, and a
   band-limited/resampled host output path.
5. Input, battery-backed RAM, reset/power state, and deterministic replay.

First playable gate: documented CPU behavior passes an independent trace,
mapper 0 executes, and one operator-owned NROM image reaches a measurable
headless video/audio checkpoint. Phase exit gate: relevant CPU, PPU, APU, and
supported-mapper suites pass; a curated operator-owned compatibility set runs
through representative scenes; headless goldens are stable on supported hosts;
malformed cartridges return errors.

## Phase 3 — frontend baseline (1–2 months)

Deliverables:

- ROM picker, per-system input mapping, integer scaling/aspect handling,
  fullscreen, audio device selection, pause/reset, and SRAM persistence.
- Audio-driven pacing with bounded resampling feedback. VSync is presentation,
  not the emulated clock.
- Crash-safe configuration and saves using atomic replace semantics.

Deferred: save states, rewind, shaders, achievements, netplay, scraping, and a
large library UI.

Exit gate: NES behavior is unchanged under frontend load, audio underruns are
measured, and a release build meets its documented frame/audio budgets.

## Phase 4 — GBA core (6–12 months)

Order: defensive cartridge and SRAM/EEPROM/Flash/RTC detection; ARM7TDMI ARM and
Thumb execution; width/sequential-wait-state-aware bus and prefetch;
IRQ/timers/DMA; PPU modes and effects; audio; then either validated HLE BIOS
services or an optional user-supplied 16 KiB BIOS.

The scheduler uses GBA system cycles. LCD output advances one dot per four
system cycles; the PPU is not a separate clock running four times faster than
the CPU.

Exit gate: CPU suites, `gba-tests`, and selected permissively licensed homebrew
fixtures pass; save-type behavior and bad-input handling are verified; real BIOS
and HLE modes are clearly distinguished in compatibility reports.

## Phase 5 — Genesis / Mega Drive core (9–18 months)

Order: defensive cartridge parser and region model; 68000; basic VDP; Z80 bus
arbitration; PSG; YM2612; DMA, interrupts, sprites, and timing edge cases.
Schedule devices in a common master-clock domain rather than using a rounded
CPU-to-VDP multiplier.

Exit gate: independent 68000/Z80 instruction tests pass, VDP/audio regression
captures match documented oracles, and dual-CPU ordering is deterministic.

## Phase 6 — SNES core (12–24+ months)

Order: header scoring and cartridge mapping; variable-speed 65C816 bus; DMA and
HDMA; baseline PPU modes; SPC700/S-DSP; remaining PPU modes and edge behavior;
then enhancement chips as separately scoped subprojects.

Ship a declared base-hardware compatibility tier before claiming broad SNES
support. SuperFX, SA-1, DSP-family, S-DD1, SPC7110, and other cartridge hardware
each require their own parser, scheduler, tests, and milestone.

Exit gate: CPU/APU suites and frame/audio goldens pass for the declared tier;
LoROM, HiROM, ExHiROM, copier-header, malformed, truncated, and oversized inputs
are covered; unsupported coprocessors fail explicitly rather than misbehave.

## Phase 7 — product hardening (ongoing)

- Versioned save-state schema with compatibility tests and strict decode limits.
- Rewind only after state snapshots are stable and bounded.
- Debugger, trace comparison, frame/audio capture, and per-core diagnostics.
- Accessibility, localization, packaging, updater policy, and signed releases.
- Fuzz parser/state boundaries and run long deterministic soak tests.

Exit gate: claims in UI and docs match the tested compatibility matrix; release
artifacts contain no original-game bytes; performance claims include release
measurements.

## Program-level stop rules

Stop and fix the current layer when an oracle diverges. Do not compensate for a
CPU or scheduler defect in renderer code, add `unsafe` to hide a measured design
problem, or start the next console while the current phase's gate is red.
