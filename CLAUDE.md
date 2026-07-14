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
- Project-owned code and documentation use `GPL-2.0-or-later`; preserve
  separately identified third-party terms in `NOTICE` and never imply that the
  project license covers operator-supplied ROMs or firmware.
- No `unsafe` without a measured release bottleneck, documented invariants,
  focused tests/fuzzing, and a safe baseline.
- Warnings are errors. Do not move to the next layer with a red current gate.

## Current state

The operator-authorized Kevin Horton `nestest` V1.00 pair is stored locally at
the ignored paths `external-fixtures/nestest.nes` and
`external-fixtures/nestest.log`. Both files match the reviewed byte counts and
SHA-256 identities in `docs/compatibility/NESTEST_PROVENANCE.md`; neither file
may be committed or published. The strict release run passes all 8,991 rows and
8,990 transitions, ending at `PC=C66E` after 26,554 cumulative CPU cycles.

The workspace contains seven functional crates:

- `retro-core`: shared deterministic contracts and typed output/input metadata.
- `format-ines`: defensive borrowed parser for iNES and NES 2.0 images.
- `format-nestest-log`: bounded hostile-byte parser for reference CPU trace rows.
- `core-nes`: parsed-cartridge ownership boundary, mapper-0 validation, a
  minimal CPU bus with RAM/PRG mirroring and explicit unsupported-I/O faults,
  the exact NTSC master-clock/PPU-dot timing layer, and a machine-owned
  one-CPU-cycle boundary that composes the live CPU bus with that scheduler.
- `retro-testkit`: deterministic synthetic core, capture hashes, generated
  mapper-0 trace comparison, the `nestest` CPU-only I/O policy selected by the
  strict CLI after fixture identity verification, and pinned clean-room
  NROM-128/NROM-256 cases.
- `retro-cli`: headless synthetic smoke executable plus bounded, sanitized
  operator-path NROM/reference-trace commands. `nestest-v1` enforces the exact
  reviewed fixture size/hash matrix before parsing.
- `cpu-6502`: trace-first 2A03 instruction layer with all 151 documented and the
  76 stable undocumented encodings exercised by `nestest`, explicit addressing,
  flags, cycle totals, stack/control flow, decode metadata, a pinned MIT
  documented-opcode single-step oracle sample, strict ordered bus traces for all
  190 sampled vectors through a live one-cycle `clock` interface, a compatible
  whole-instruction `step` wrapper, and instruction-granular interrupt/reset handling
  (edge-triggered NMI, level-triggered IRQ, the one-instruction `I`-flag delay,
  and the seven-cycle interrupt/reset sequences).

The fuzz project calls both format parsers with arbitrary bytes. The checked-in
launcher generates redistribution-safe seeds and handles the Windows
AddressSanitizer runtime path.

Not implemented: `retro-frontend`, a complete NES machine, PPU/APU/I/O bus
devices, per-cycle hardware interrupt/reset entry and NMI hijacking, DMA, input
hardware, SRAM persistence, save states, rewind, or any GBA/Genesis/SNES code.
The CPU and timing scheduler are connected at one live mapper-bus cycle per
machine call, but there is no PPU register or rendering device yet.

The operator's 31-file NES instruction folder is located at
`%USERPROFILE%\Desktop\panda video\nes`. It was ingested into
`docs/NES_REFERENCE_INTAKE.md` as an external, unattributed AI-generated
scaffold. Use it only as a subsystem checklist; do not publish its originals or
treat its code/claims as an oracle.

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
  and selected generated semantic tests in a trace-first CPU layer.
- Curated 190 data-only vectors from the pinned MIT `SingleStepTests/65x02`
  RP2A03 suite. They independently sample final architectural state, declared
  RAM, and cycle counts for all 151 encodings, all 23 paired page-penalty
  profiles, and all eight branch 2/3/4-cycle profiles. The reproducible curator
  validates bounded upstream data and the upstream license is retained in
  `NOTICE`.
- Implemented an NROM-128/NROM-256 CPU bus, trainer/PRG-memory validation,
  side-effect-free diagnostic reads, and explicit faults for missing devices.
- Added an isolated bounded `nestest`-style log parser, generated end-to-end
  trace comparison, and a second ASan fuzz target. No external fixture was used.
- Pinned Kevin Horton's `nestest` V1.00 identity and hashes. No explicit
  redistribution license was found, so fixtures remain operator-supplied only.
- Added `retro-cli nes-trace <ROM_PATH> <LOG_PATH>` with bounded reads,
  path/content-safe diagnostics, stable exit statuses, and generated real-file
  boundary tests.
- Added strict `retro-cli nestest-v1 <ROM_PATH> <LOG_PATH>` acceptance:
  SHA-256 identity checks precede parsing, generic output says
  `fixture_identity=unchecked`, and strict success requires 8,991 rows / 8,990
  transitions.
- Ran the exact reviewed QMT pair through the release strict path. All 8,991
  rows / 8,990 transitions match through mapper 0, with final `PC=C66E`,
  `A=00`, `X=FF`, `Y=15`, `P=27`, `SP=FD`, and 26,554 cumulative cycles.
- Added the exact 76 stable undocumented encodings exercised by the accepted
  trace: NOP aliases, LAX, SAX, DCP, ISC, SLO, RLA, SRE, RRA, and `$EB` SBC.
  Focused tests cover metadata, operand consumption, page cycles, combined
  operation/flag order, and failure-atomic eight-cycle headroom.
- Kept missing-device behavior explicit: after verifying both exact fixture
  identities, the strict CLI selects a CPU-trace allowlist for writes to
  `$4004`-`$4007` and `$4015`. The reviewed log makes exactly five terminal
  writes to those addresses; the normal NROM bus still faults unimplemented I/O.
- Rebuilt the public GitHub README around verified status, the checked/remaining
  build path, architecture and crate maps, reproducible commands, legal fixture
  boundaries, contributing rules, and an explicit human-directed AI-development
  disclosure. Removed the obsolete operator-local source-document path.
- Added RustCrypto `sha2` 0.11.0 to the CLI runtime and testkit development
  tests, with default features disabled. The reviewed lockfile contains its
  small Rust dependency graph; the crate is MIT OR Apache-2.0 and requires
  Rust 1.85.
- Added a project-owned NROM diagnostic generator and three reproducible
  architectural traces from BSD-3-Clause py65 commit
  `3138e1b337734a9b2ac1ea90ee7a453514436221`. NROM-128, NROM-256, and a
  trainer-bearing NROM-128 case pass through the parser, cartridge, mapper bus,
  runner, and real CLI. The trainer case executes reads from `$7000` and
  `$71FF`. Imported oracle files are hash-pinned; no third-party ROM or operator
  bytes are stored.
- Published the verified foundation as commit
  `b7c3182a8672db0bed814951cd9d959fa8eb8f7a` and its handoff update as commit
  `4515511c154c1e5fe39a45c750bda45a71569ed3`.
- Published the mapper-0 bus/reference-runner checkpoint as commit
  `505a73c02d69f309cad37d7c85e7520d7e5ab6b6`.
- Published the provenance/operator-CLI checkpoint as commit
  `cb4e2de00bb843bef37fa5ef0dc1dc8c08b6a27f`.
- Published the independent single-step-oracle checkpoint as commit
  `e5f3a4d73738e908b0c2d2fce8c372182a9141fc`.
- Published the clean-room NROM mapper-integration checkpoint as commit
  `53c65b20e9d572bfe64bdaf0613481dba87d21a3`.
- Published the trainer-backed NROM and `GPL-2.0-or-later` checkpoint as commit
  `93c696b005f4cddcaca932ba210e95aebeaba44a`.
- Published the automated clean-room evidence checkpoint as commit
  `1fedfd85944c4ca58261cff4f823ace04686533d` and its cross-platform LF fix as
  commit `f6cf9bc38f895ae839495c76f3adb01963966a6b`.
- Implemented architectural interrupt and reset handling in `cpu-6502` on the
  instruction-stepped model, fully additive so the instruction path and the
  `nestest` trace are unchanged. Added edge-triggered NMI and level-triggered IRQ
  line inputs, a `pending_interrupt` poll with NMI-over-IRQ priority and IRQ
  masked by the `I` flag, `service_interrupt` for the seven-cycle frame (return
  address and status pushed with `B` clear, `I` set, NMI `$FFFA`/IRQ `$FFFE`
  vector), a shared frame helper reused by `BRK` (with `B` set), the
  one-instruction `I`-flag delay for `CLI`/`SEI`/`PLP` versus immediate `RTI`,
  and a warm `reset` (SP −3 without writing, `I` set, latched NMI dropped, `PC`
  from `$FFFC`, registers preserved). Both entry paths reserve seven cycles
  before any bus access. Added 14 focused tests and left per-cycle interrupt/reset
  bus order, dummy reads/writes, RMW double-writes, sub-instruction poll timing,
  and NMI hijacking explicitly deferred. A four-lens adversarial review found no
  real P0-P2 defect. Shipped as commit `e6b32c9a2e7e65367f11075a72ea5936727e3f27`;
  GitHub Actions CI passed all six Windows/Ubuntu jobs.
- Completed per-cycle instruction bus accuracy (published 2026-07-14 in
  `04ae6d017e38296fc82dc1b2ebf1eb06d8eb6b63`).
  Extended the
  SingleStepTests curator to emit each vector's ordered per-cycle bus trace
  (`bus_cycles` + a `CycleKind` enum) and regenerated `singlestep_vectors.rs`
  deterministically offline (new SHA-256
  `a53a81800b37bbfb5f5101785974bbed9070c103a09d206988101a79969922fa`). Added a
  `RecordingRam` bus and a coverage checkpoint comparing the CPU's per-cycle bus
  trace to the oracle (measured baseline 74/190). Implemented all remaining
  classes:
  two-cycle implied/accumulator instructions now perform the dummy read of the
  byte after the opcode (74->96), and read-modify-write instructions write the
  original value back before the modified value so zero-page and absolute RMW
  modes match (96->108); indexed-addressing dummy reads bring the checkpoint to
  166; and stack, control-flow, branch, and BRK sequences bring it to 190. The
  temporary threshold is gone: a strict test now requires every sampled ordered
  bus trace to match. The final-state, clean-room, and strict `nestest-v1` state
  oracles remained unregressed after each increment.
- Inventoried the operator's 31-file NES instruction folder and added the safe
  project-owned intake at `docs/NES_REFERENCE_INTAKE.md`. No source note was
  copied because the folder has no author/license metadata and contains
  simplified or unsafe scaffolds.
- Added the first exact NTSC timing checkpoint in `core-nes`: rational master
  clock 236.25 MHz / 11, 12 master ticks per CPU cycle, 4 per PPU dot, 341 dots
  per scanline, 262 scanlines per frame, VBlank edges at 241/1 and 261/1, and
  the rendering-dependent odd-frame jump from 261/339 to 0/0. Checked counters
  are failure-atomic. Six focused timing tests pass. This is a timing layer, not
  a PPU implementation.
- Implemented live instruction-cycle execution and machine timing integration.
  `Cpu::clock` performs exactly one live bus read/write per successful call and
  reports progress, instruction completion, or an unsupported opcode after its
  consumed fetch cycle; `Cpu::step` now loops that interface. Typed microstate covers all
  227 decoded encodings. The strict 190-vector gate now checks exactly one bus
  access per call and byte-for-byte ordered traces; a differential test matches
  the retained test-only whole-instruction implementation for all 227 encodings.
  A mutation-between-cycles test proves operand reads are live rather than
  replayed. Interrupt/reset whole operations reject an active instruction
  without changing CPU or bus state.
- Added `core-nes::NesMachine::clock`, which composes `Cpu`, `NromCpuBus`, and
  `NtscScheduler`. Each successful CPU bus cycle advances 12 master ticks / 3
  PPU dots, reports unsupported mapper-I/O on the exact causing cycle, and
  exposes timed VBlank events. Scheduler overflow is preflighted so CPU and
  timing state cannot diverge. Five focused machine tests cover NOP completion,
  an exact-cycle `$2000` write fault, unsupported-fetch lockstep, overflow
  atomicity, and the VBlank-start delivery cycle.
- Fresh adversarial review found and resolved machine lockstep on unsupported
  opcode fetches: the fetch now consumes one CPU cycle and returns a typed clock
  outcome, so the scheduler advances and cannot replay a side-effectful read at
  frozen time. It also strengthened all-227 per-call bus regression coverage,
  made CPU overflow tests detect reads, added machine timing-overflow atomicity,
  and asserted VBlank delivery call count, CPU cycles, master ticks, and PPU
  position. The lack of an independent ordered-bus oracle for the 76 stable
  undocumented encodings remains documented, not promoted to a verified claim.
- Fresh technical and claims reviews found three bounded issues: missing
  `FrameOverflow` atomicity coverage, wording that accidentally extended the
  190 documented-vector bus oracle to undocumented opcodes, and publication of
  an absolute operator path. Added the missing test, narrowed the claim and
  explicitly left undocumented bus order open, and replaced the path with the
  non-identifying `%USERPROFILE%` form. The provenance ledger was also brought
  forward to the sanitized July 14 strict run. Fresh fix-only re-review found no
  remaining P0-P2 issue.
- Published the verified 56-file CPU bus-trace/NTSC timing checkpoint as commit
  `04ae6d017e38296fc82dc1b2ebf1eb06d8eb6b63`. The final publisher preview found
  no missing managed remote files, validated every allowlisted file through its
  in-root handle, preserved the parent tree and all non-allowlisted remote files,
  and made no write before the explicit publish call.

## Required commands

Run from `H:\claaaude\universal-retro-emulator`:

```powershell
python tools/check-cleanroom-nrom.py
cargo run --release -p retro-cli -- nestest-v1 external-fixtures\nestest.nes external-fixtures\nestest.log
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

Verified on Windows x86-64 with Rust/Cargo 1.96.0 on 2026-07-14:

- Format check passed.
- Clippy passed for the workspace, all targets, and all features with warnings
  denied.
- Debug tests: 106 passed, 0 failed.
- Release tests: 106 passed, 0 failed; doc tests passed.
- `cpu-6502`: 38 tests passed, including live per-cycle access, mutation,
  227-encoding differential, boundary rejection, and the 190-vector oracle.
- `core-nes`: 20 tests passed, including all scheduler and machine-cycle tests.
- All six new NTSC timing tests passed: exact 3:1 CPU/PPU progression, VBlank
  edges and master timestamp, 89,342/89,341-dot frame alternation, no shortened
  frame while rendering is disabled, and failure-atomic overflow.
- The vectors regenerate deterministically offline to SHA-256
  `a53a81800b37bbfb5f5101785974bbed9070c103a09d206988101a79969922fa`.
- Every one of the 190 pinned ordered instruction bus traces matches exactly;
  the final-state oracle passes too.
- The clean-room verifier downloaded the pinned py65 source/license, reproduced
  all three cases at generated SHA-256
  `02f88830b4af0d46b3ba542a713c4fddd94f6c9af4f9b49e69d92bc03a3bfab5`, and
  passed its six Python tests.
- Windows AddressSanitizer fuzz smoke: 10,000 executions per parser completed
  with no crash.
  cargo-fuzz is pinned at 0.13.2; CI pins nightly-2026-07-12, while the local
  run used rustc nightly commit `be8e82435` dated 2026-07-11.
- Release CLI: final tick `30`; video `3` frames, hash `2d1f1e3d37030229`;
  audio `7` packets / `28` frames, hash `b2bdf29fe8dd6d45`; ordered event hash
  `2343096cdf497a5e`.
- Strict `nestest-v1`: the exact reviewed QMT pair matched 8,991 rows / 8,990
  transitions with final `PC=C66E`, `A=00`, `X=FF`, `Y=15`, `P=27`, `SP=FD`,
  and 26,554 cumulative cycles. All 76 stable undocumented encodings exercised
  by the fixture passed.
- Strict full-trace checkpoint `7f1e20f07a93c432fb0639ea8630474c4e9e26b7`
  is published. GitHub Actions run `29273142006` passed all six Windows/Ubuntu
  test, 10,000-run fuzz, and clean-room evidence jobs. No ignored fixture or
  local run record was published.
- Mapper-0 bus/reference-runner checkpoint
  `505a73c02d69f309cad37d7c85e7520d7e5ab6b6` is published. GitHub Actions run
  `29254844214` passed all four jobs: stable tests and both 10,000-run parser
  ASan fuzz targets on Windows 2025 and Ubuntu 24.04.
- Provenance/operator-CLI checkpoint
  `cb4e2de00bb843bef37fa5ef0dc1dc8c08b6a27f` is published. GitHub Actions run
  `29257679328` passed all four stable/fuzz jobs on Windows 2025 and Ubuntu
  24.04. No external fixture was found or run.
- Strict fixture-identity checkpoint
  `8bfdec36fc866a2f1c3b37d88e304a7e7ef96e10` is published. GitHub Actions run
  `29259546369` passed all four stable/fuzz jobs on Windows 2025 and Ubuntu
  24.04. It passed fresh adversarial review with no actionable P0-P2 findings
  and a deletion-safe 43-file publisher preview. At that checkpoint its
  accepted-fixture path was unrun; the later operator-authorized local run is
  recorded above without publishing either fixture.
- Independent single-step-oracle checkpoint
  `e5f3a4d73738e908b0c2d2fce8c372182a9141fc` is published. It passes all 190
  pinned vectors and a
  clean regeneration reproduced generated-file SHA-256
  `5e8341f1b5b17a3f08835bf81674b6fe01b682d9500a4204540de462a09eeddb`.
  Fresh adversarial review found one P1 fractional-number validation defect;
  integer-type enforcement fixed it, a same-size hostile chunk proved rejection,
  and re-review found no remaining P0-P2 issues. A deletion-safe 46-file
  publisher preview subsequently passed and excluded all local operator
  fixtures. GitHub Actions run `29262489825` passed all four stable/fuzz jobs on
  Windows 2025 and Ubuntu 24.04.
- The clean-room NROM generated module reproduced byte-for-byte twice at
  SHA-256 `64b66bef80d0d07f9da4664cdf9d4ef133e070994f375a2d3071a6bda142e6c5`.
  A mutated imported py65 module was rejected before output. The release CLI
  matched the NROM-128 case across 41 rows / 40 transitions, ending at
  `PC=C102`, `A=5A`, and 128 cycles. Both parser fuzz targets completed 10,000
  Windows AddressSanitizer runs without a crash.
- Fresh adversarial review found two P1 trust-boundary defects: Python cache
  bytecode could bypass source hashes, and the publisher could follow an
  allowlisted reparse point outside the workspace. The generator now compiles
  only hash-validated source bytes into fresh in-memory modules, with no
  filesystem import/cache path. The publisher now rejects reparse points and
  validates the final in-root target from the same locked handle it reads.
  Focused re-review found no remaining P0-P2 issue. A guarded, deletion-safe
  49-file publisher preview passed and excluded operator fixtures.
- Clean-room NROM mapper-integration checkpoint
  `53c65b20e9d572bfe64bdaf0613481dba87d21a3` is published. GitHub Actions run
  `29265895004` passed stable format/lint/debug/release/app gates and both
  10,000-run parser fuzz jobs on Windows 2025 and Ubuntu 24.04.
- The trainer extension supersedes the generated module at SHA-256
  `c54fb4ce577aa3331386bd6eb91260869493a5c4fbc89fc409f827497d2c9054`.
  Two clean regenerations were byte-identical. All three cases match 47 rows /
  46 transitions and 152 cycles; the release CLI exercised the trainer case
  through real files. Format, warnings-denied clippy, 68 debug tests, 68 release
  tests, doc tests, and both 10,000-run Windows ASan parser fuzz gates passed.
- Trainer/license checkpoint `93c696b005f4cddcaca932ba210e95aebeaba44a`
  is published. GitHub Actions run `29267749389` passed all four stable/fuzz
  jobs on Windows 2025 and Ubuntu 24.04, including the release app and 10,000
  executions of each parser fuzz target.
- The clean-room automation extension corrected checkout-line-ending-dependent
  py65 hashes to raw LF source hashes and regenerated the module at SHA-256
  `02f88830b4af0d46b3ba542a713c4fddd94f6c9af4f9b49e69d92bc03a3bfab5`.
  The downloader and generator cap each external file at 1,000,000 bytes and
  validate all seven hashes before oracle execution. Six hostile/deterministic
  Python tests and a spawned compiled-CLI test for all three cases pass.
- CI now has read-only repository permissions, credential-free checkouts, and
  a separate Windows/Ubuntu clean-room evidence matrix with immutable
  checkout/setup-python pins and Python 3.13.5. The publisher now accepts only
  individually enumerated paths. Local format, warnings-denied clippy, 69 debug
  tests, 69 release tests, release doc tests, and the release app pass.
- Fresh review found two P2 defense-in-depth gaps: the evidence job was
  Ubuntu-only and publisher path patterns were broader than the reviewed
  snapshot. The Windows evidence leg and fully enumerated publisher inventory
  fixed both; re-review found no remaining P0-P2 issues. The first published
  six-job run then exposed Windows checkout CRLF conversion of the generated
  LF module. Root `.gitattributes` now enforces LF text on every platform; a
  focused re-review found no P0-P2 issue. The deletion-safe 54-file preview
  passed. Commit `f6cf9bc38f895ae839495c76f3adb01963966a6b` is published,
  and GitHub Actions run `29270030204` passed all six Windows/Ubuntu
  test, 10,000-run fuzz, and clean-room evidence jobs.
- An operator-owned mapper-1 image was identified and retained only under the
  ignored `external-fixtures/` directory. Its header is valid, but the current
  NROM-only trace boundary correctly rejected it before emulation. MMC1 remains
  a later compatibility target, not a working feature.

## Next tasks, in order

1. Curate a minimal reproducible harness from the pinned MIT `perfect6502`
   source recorded in `docs/compatibility/PERFECT6502_PROVENANCE.md`; use it to
   verify and implement live hardware IRQ/NMI/reset bus entry, second-to-last
   cycle polling, and NMI hijacking without weakening the current 190-vector
   and full-trace gates.
2. Add the PPU register/address-space shell to the existing machine boundary,
   route mirrored
   `$2000-$3FFF` CPU accesses, and connect the verified VBlank timeline to the
   CPU NMI line. Continue with fetch, scroll, sprite, and rendering oracles.
3. Add DMA/APU/input and reach a deterministic headless NROM video/audio
   checkpoint.
4. Add a tested MMC1 implementation, including serial writes, PRG/CHR banking,
   mirroring, reset, and PRG RAM, for the supplied mapper-1 target.
5. Only then resolve and spike `winit`/`wgpu`/`cpal` for `retro-frontend`.

## Decisions still open

- Whether Linux is a release target or a CI-only target initially.
- Additional independently licensed interrupt/bus suites and acquisition process.
- Final product name.

## License decision

- On 2026-07-13, project-owned code and documentation were licensed as
  `GPL-2.0-or-later`: recipients may use GPL version 2 or any later GPL version.
- The canonical GPLv2 terms are in `LICENSE`; Cargo metadata declares the SPDX
  expression for every workspace crate and the fuzz package.
- `NOTICE` retains the licenses and attribution for third-party test material.
  Commercial ROMs, firmware, and operator-supplied files are excluded from the
  repository and are not relicensed.
- The normalized `LICENSE` contents matched GitHub's canonical GPL-2.0 template
  (`SHA-256 8177f97513213526df2cf6184d8ff986c675afb514d4e68a404010521b880643`).
  Format, warnings-denied clippy, 68 debug tests, 68 release tests, release doc
  tests, and an independent P0-P2 license review all passed.

## Honest limitations

The synthetic core proves only the shared contract/headless capture path. The
CPU now has independent instruction-boundary samples across all documented
encodings and passes the full mapper-0 `nestest` architectural trace, including
its 76 stable undocumented encodings. All 190 sampled ordered instruction bus
traces match through a live one-cycle interface, but the sample is not exhaustive,
and hardware interrupt/reset entry bus order, sub-instruction polling, and NMI
hijacking remain unchecked. The machine boundary advances the NTSC scheduler
after each successful instruction bus cycle and exposes timed events/faults; it
is not connected to PPU registers or rendering. DMA, APU, input, gameplay, and mapper 1 remain
unimplemented. PPU/APU/I/O are intentionally faulted outside the strict CLI's
reviewed trace-write allowlist. This is not a playable NES emulator.
