# Universal Retro Emulator

Status: the Phase 1 headless foundation is implemented and the NES vertical
slice is in progress. Shared contracts, a defensive NES image parser, the
parsed-cartridge boundary, synthetic test core, headless CLI, parser fuzzing,
a trace-first 2A03 CPU layer, mapper-0 CPU bus, and generated reference-trace
runner exist. A bounded operator-path trace command and reviewed `nestest`
identity metadata also exist. The strict `nestest-v1` command cryptographically
rejects any unreviewed fixture pair, but the CPU has not yet passed an
operator-supplied independent oracle and playable console emulation does not
exist yet.

This project targets a native Rust application with independently testable NES,
Game Boy Advance, Sega Genesis / Mega Drive, and SNES cores behind one frontend.
The first shippable target is deliberately smaller: a verified NES vertical slice
that loads a user-supplied NROM image, runs deterministically, and produces
measurable video and audio output.

## Project documents

- [ROADMAP.md](ROADMAP.md) defines phases, exit criteria, and realistic effort.
- [BUILD_PATH.md](BUILD_PATH.md) defines the proposed workspace layout and the
  exact order in which to create it.
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) records the corrected design.
- [docs/PROPOSAL_REVIEW.md](docs/PROPOSAL_REVIEW.md) lists corrections to the
  original proposal.
- [docs/PROJECT_STATE.md](docs/PROJECT_STATE.md) is the living handoff for future
  sessions.
- [docs/compatibility/NESTEST_PROVENANCE.md](docs/compatibility/NESTEST_PROVENANCE.md)
  pins the intended external CPU oracle without redistributing it.
- [CLAUDE.md](CLAUDE.md) is the concise agent handoff: completed work, verified
  commands, current limitations, and the exact next-task order.

## Fixed scope

- First-party cores: NES, GBA, Genesis / Mega Drive, and SNES.
- Desktop targets first: Windows and Linux.
- User-supplied ROMs and firmware only; no copyrighted game, firmware, audio,
  graphics, or test-ROM bytes belong in this repository.
- Accuracy before convenience features. Save states, rewind, shaders, library
  browsing, and netplay do not precede a correct NES vertical slice.
- `wgpu` + `winit` is the current presentation direction. Bevy is not required
  for cycle-accurate emulation and will only be considered later for product UI
  if it proves useful.

## Definition of done for the first milestone

The NES milestone is complete only when all of the following are exercised:

1. A defensive iNES/NES 2.0 parser rejects truncated, oversized, and malformed
   inputs without panicking.
2. The CPU trace matches an independent `nestest` oracle for the supported
   instruction set.
3. A headless run produces a stable framebuffer hash and audio sample count.
4. The frontend displays that framebuffer and plays bounded, underrun-measured
   audio from the same deterministic core.
5. Format, lint, unit, integration, parser-adversarial, and headless behavior
   gates pass with warnings denied.

The original input document remains at:
`C:\Users\rezta\Desktop\Universal Retro Emulator  NES, SNES, Sega Genesis & GBA in Rust.md`.
