# Universal Retro Emulator

Status: the Phase 1 headless foundation is implemented and the NES vertical
slice is in progress. Shared contracts, a defensive NES image parser, the
parsed-cartridge boundary, synthetic test core, headless CLI, parser fuzzing,
a trace-first 2A03 CPU layer, mapper-0 CPU bus, generated reference-trace
runner, and pinned MIT single-step oracle sample exist. A bounded operator-path
trace command and reviewed `nestest`
identity metadata also exist. The strict `nestest-v1` command cryptographically
rejects any unreviewed fixture pair. All 151 documented encodings pass the
curated instruction-boundary sample, but the full operator-supplied mapper trace
has not run and playable console emulation does not exist yet.

Project-owned NROM-128 and NROM-256 diagnostics also pass independent pinned
py65 architectural traces through the parser, mapper bus, runner, and real CLI.
A third NROM-128 case executes reads from both ends of a 512-byte trainer,
covering parser offset and PRG-RAM preload integration. This is mapper-0
evidence, not proof of reset timing, bus-cycle order, PPU/APU behavior, MMC1
support, or gameplay.

The clean-room generator is also checked automatically from bounded raw files
at the exact pinned py65 revision. Hostile missing, oversized, changed, and
stale inputs are rejected, and all three cases run through a spawned release
CLI process in the test suite. This improves reproducibility and process-boundary
evidence; the strict external `nestest-v1` gate remains unrun.

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
- [docs/compatibility/CLEANROOM_NROM_PROVENANCE.md](docs/compatibility/CLEANROOM_NROM_PROVENANCE.md)
  records the reproducible project-owned mapper-0 diagnostic evidence.
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

## License

Copyright (C) 2026 PandaCatz and contributors.

Except for separately identified third-party material, this project's source
code and documentation are licensed under the GNU General Public License,
version 2 or (at your option) any later version (`GPL-2.0-or-later`). See
[LICENSE](LICENSE) for the complete terms and [NOTICE](NOTICE) for retained
third-party licenses and attribution.

This license does not grant rights to commercial ROMs, firmware, game assets,
or other operator-supplied files. Those files are not part of this repository.

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
