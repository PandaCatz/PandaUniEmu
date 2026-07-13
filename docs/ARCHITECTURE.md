# Architecture Baseline

## System boundary

The product is one frontend plus four independently testable machine cores. The
shared layer supplies value types, diagnostics, and host adapters; it does not
pretend the consoles share a bus, timing model, pixel format, controller, or
save-state schema.

```text
user-owned image -> format parser -> validated cartridge -> console core
                                                       -> timed video/audio events
host input -> timestamp/latch adapter -> console core  -> frontend presenters
```

## Dependency direction

```text
format-*  ─┐
cpu-*     ─┼─> core-* ─> retro-core contracts <─ retro-testkit / retro-cli
devices   ─┘                                  <─ retro-frontend
```

Rules:

- `format-*` takes bytes and returns validated data. It has no `wgpu`, `winit`,
  `cpal`, `gilrs`, Bevy, or console-runtime dependency.
- `core-*` owns emulated state and time. It depends on contracts and its parsed
  data/CPU components, never on host devices.
- `retro-frontend` depends on contracts; no core calls into it.
- `retro-testkit` can run every core without a window, GPU, or audio device.

## Ownership model

One top-level machine value owns all mutable state for a running console:
processor(s), scheduler, cartridge devices, RAM, video, audio, DMA/timers, and
controllers. A device does not also live inside a separately borrowed `Bus`.
Scheduled work receives a narrow context exposing only the reads, writes, IRQs,
and event scheduling it needs. This avoids the contradictory ownership in the
source examples and keeps runtime borrow failures out of deterministic code.

Cartridges are active devices, not only byte arrays. Their interface must allow
mapper/coproc reset, mutable reads where hardware requires them, CPU/PPU address
observation, nametable routing, IRQ state/events, persistent memory, and optional
audio. Each console defines its own cartridge contract rather than forcing all
formats through the NES mapper vocabulary.

## Core contract direction

The source proposal's `Console` trait is useful as a sketch but too narrow as a
lasting ABI. A corrected contract should use owned output buffers or caller-owned
sinks and expose these concepts without fixing the final Rust spelling yet:

```text
CoreInfo
  system, supported regions, ports, native video modes, audio clock/capabilities

Core
  power(config, validated_cartridge)
  reset(kind)
  set_input(port, typed_state, emulated_time)
  run_until(deadline_or_event, output_sink) -> RunOutcome
  snapshot() -> versioned logical state
  restore(validated_snapshot) -> Result
```

`RunOutcome` reports the exact emulated timestamp and why execution stopped.
Video/audio packets include their own metadata. This supports mid-frame events,
variable resolutions, interlacing, debugging, rollback experiments, and headless
tests without making a host frame the fundamental unit of emulation.

## Time model

Each console chooses an exact integer master-clock domain or exact rational
conversion. Scheduled events have explicit tie-breaking order. Long-run tests
must prove no accumulated floating-point drift.

- NES NTSC: CPU and PPU derive from the common master clock; model the 3:1 PPU
  relation plus phase-sensitive races and the odd-frame skipped PPU clock.
- GBA: one 16,777,216 Hz system-cycle domain; an LCD dot spans four cycles and a
  frame spans 280,896 cycles.
- Genesis: derive 68000, Z80, VDP, and audio events from the console master clock;
  do not use a rounded `~10` multiplier.
- SNES: schedule master cycles; a 65C816 bus cycle consumes 6, 8, or 12 master
  clocks depending on access and configuration. `3.58 MHz` is only the fast case.

The frontend never changes core clocks. It adapts host presentation and audio
rates around the core's deterministic output.

## Save and persistent data

Battery RAM/EEPROM/flash persistence and save states are different products.
Persistent cartridge memory is stored by its device rules and written
atomically. Save states use an explicitly versioned logical schema, strict size
limits, invariant validation, ROM identity, and compatibility tests.

Direct `serde`/`bincode` derivation over implementation structs is acceptable
only for disposable experiments. It is not the durable public format because
field layout and enum changes silently couple files to source structure.

## Determinism

- No wall clock, host thread timing, or unseeded randomness inside a core.
- State-changing device events use explicit order.
- Input is latched at defined emulated times.
- Rendering and audio packetization consume simulation state downstream.
- A replay is identified by core version, cartridge identity, configuration,
  initial state, and timestamped input stream.

## Safety and performance

All external formats, states, firmware, configuration, and mod input are hostile
boundaries. They return errors rather than panic. `unsafe` is allowed only after
a release-profile measurement identifies a material bottleneck, a safe baseline
exists, invariants are documented, and tests/fuzzing exercise the wrapper.

Compatibility and performance are reported from evidence, not from a game
opening or a render inspected by eye.
