# NES Reference Intake

## Operator source

The operator supplied a local folder of 31 Markdown notes at:

```text
%USERPROFILE%\Desktop\panda video\nes
```

The folder was inventoried on 2026-07-14 (173,662 bytes total). It is a
conversational AI-generated roadmap and code scaffold. No author, source,
license, or attribution metadata appears in the files, and `new34.md` and
`new35.md` are byte-identical. The originals therefore remain outside the
repository and must not be copied or published verbatim unless their provenance
and redistribution rights are established.

This document is the project-owned intake: it records what is useful, what needs
independent verification, and where a future agent can find the operator's local
source.

## How the notes may be used

Use the folder as a subsystem checklist and topic index. Do not use it as a
hardware specification, test oracle, or source-code dependency. When it conflicts
with checked project evidence, pinned independent test vectors, defensive input
rules, or the architecture record, those project controls win.

Accepted sequencing guidance:

1. Keep cartridge parsing, CPU logic, machine devices, and the host frontend in
   separate layers.
2. Establish headless CPU and mapper evidence before graphics/audio claims.
3. Establish an exact CPU/PPU clock domain and dot timing before implementing
   PPU registers, fetches, and rendering.
4. Add DMA, APU, input, persistence, mappers, and frontend behavior only with
   focused tests and explicit timing/error boundaries.

Important cautions found during intake:

- `new14.md`'s eager operand-fetch abstraction hides observable bus cycles and
  must not replace the verified CPU bus sequence.
- `new17.md` and several mapper/save examples use unchecked indexing or
  panicking I/O; all external boundaries in this project return errors.
- `new21.md` proposes fake VBlank behavior, `new25.md` services NMI immediately,
  and `new24.md` performs DMA as a bulk copy. Those are scaffolds, not acceptable
  emulation behavior.
- `new26.md` records 262 scanlines, 341 dots, and VBlank at scanline 241 dot 1,
  but omits the rendering-dependent odd-frame skipped dot. The project timing
  implementation follows independently checked NESdev timing instead.
- Assertions such as “fully functional,” “structurally perfect,” or broad game
  compatibility are not evidence and must not appear in project claims.

## Indexed topics

NES roadmap and CPU foundation:

- `new9.md`-`new18.md`: high-level roadmap, iNES, CPU/bus scaffolding, status and
  addressing, opcode organization, `nestest`, and NROM mapping.
- `new11.md` is the main timing note: it identifies later DMC DMA, 513/514-cycle
  OAM DMA, and the need for sub-instruction CPU/PPU synchronization, but provides
  no bus-cycle oracle.

NES PPU, mapper, input, persistence, and APU:

- `new21.md`-`new27.md`: preliminary PPU registers, memory, OAM/DMA, scrolling,
  scanline/dot timing, background attributes, and sprites.
- `new29.md`-`new32.md`: frame output, mapper abstraction/MMC1, controllers,
  PRG RAM, and battery persistence.
- `new33.md`-`new35.md`: introductory APU channel sketches; `new35.md` duplicates
  `new34.md`.

Future SNES material, outside the current milestone:

- `new36.md`-`new42.md`: 65C816, SNES bus/mapping, cartridge heuristics, PPU,
  backgrounds, and DMA.

## Current application

The old recorded CPU task is already complete in the working tree: all 190
pinned RP2A03 vectors now match their ordered instruction bus traces. The first
timing checkpoint built after this intake is deliberately smaller than a PPU:

- exact NTSC rational master-clock metadata;
- 12 master ticks per CPU cycle and 4 per PPU dot;
- 341 dots per scanline and 262 scanlines per frame;
- VBlank set/clear timing events at scanline 241 dot 1 and pre-render scanline
  261 dot 1;
- the one-dot odd-frame shortening only while rendering is enabled; and
- checked, failure-atomic timing counters.

The later machine checkpoints now make the CPU yield at every live bus cycle and
add a deterministic PPU register/address/background-fetch shell: mirrored CPU
ports, shared scroll/address state, buffered blanking-time data access, NROM CHR,
nametable/palette mirroring, basic OAM ports, logical VBlank-driven NMI, two-dot
background fetch phases, shifter reloads, and scroll transfers. The numeric
fetch/scroll cases were independently derived from the NESdev references below,
not from the local notes. Pixels, sprites, exact PPUMASK propagation,
PPUSTATUS/VBlank races, contended `$2007` data behavior/collisions, DMA, APU, and
a complete machine still do not exist. The local notes remain a checklist, not
an oracle.

## Timing references

- <https://www.nesdev.org/wiki/Clock_rate>
- <https://www.nesdev.org/wiki/PPU_rendering>
- <https://www.nesdev.org/wiki/PPU_scrolling>
- <https://www.nesdev.org/wiki/PPU_registers>
