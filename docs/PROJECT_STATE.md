# Project State

Last updated: 2026-07-13

## Current phase

The Phase 1 headless foundation is implemented. The Phase 2 NTSC NES vertical
slice has passed the independent CPU trace and is now at the interrupt and
per-cycle bus-order boundary. Seven functional workspace crates exist. The
reviewed QMT `nestest` pair passes 8,991 rows / 8,990
transitions through the mapper-0 CPU bus, including the exact 76 stable
undocumented encodings it exercises. A pinned MIT single-step sample provides
independent instruction-boundary evidence across all 151 documented encodings.
Project-owned NROM-128/NROM-256 diagnostics also match
pinned py65 architectural traces through the mapper and CLI, including a
trainer-bearing NROM-128 case that reads both preload endpoints. There is no
PPU/APU, mapper 1, complete NES machine, host
frontend, or playable emulation.

## Implemented this session

- Obtained the operator-authorized pair from the author-hosted source into the
  ignored `external-fixtures/` boundary and verified both reviewed sizes and
  SHA-256 identities before parsing. Neither file is allowlisted for publishing.
- Fixed the first real trace boundary by implementing the exact 76 stable
  undocumented encodings exercised by the fixture, including operand-consuming
  NOP aliases, LAX/SAX, six combined RMW families, the `$EB` SBC alias, exact
  page policies, and failure-atomic eight-cycle headroom.
- Added focused tests for decode-set identity, NOP lengths/page cycles, LAX/SAX,
  combined operation and flag order, RMW mode/cycle matrices, the SBC alias,
  and eight-cycle overflow protection.
- After verifying both exact fixture identities, the strict CLI selects a
  CPU-only allowlist for writes to `$4004`-`$4007` and `$4015`. The reviewed log
  makes exactly five terminal writes to those addresses. The normal NROM bus
  still rejects unimplemented I/O, and a focused test proves `$4014` remains a
  fault under the special policy.
- The release strict path passes all 8,991 rows / 8,990 transitions, ending at
  `PC=C66E`, `A=00`, `X=FF`, `Y=15`, `P=27`, `SP=FD`, and 26,554 cycles.
- Rebuilt the public GitHub README with evidence-backed status, a checkbox build
  path, architecture/workspace summaries, runnable verification commands,
  fixture/legal boundaries, contribution rules, and an explicit
  human-directed AI-development disclosure. All 16 local Markdown links resolve.

- Automated clean-room evidence regeneration without committing upstream files:
  a bounded downloader fetches only the py65 license and six executed source
  files at the exact revision, and the generator validates every raw LF hash
  before compiling source bytes in memory.
- Replaced Windows-checkout-dependent py65 hashes with the raw pinned-revision
  byte identities. This changed only the recorded oracle-source identity in the
  generated module; all three project-owned image and trace hashes remain the
  same. The module now has SHA-256
  `02f88830b4af0d46b3ba542a713c4fddd94f6c9af4f9b49e69d92bc03a3bfab5`.
- Added generator `--check`, same-handle bounded reads, and six stdlib regression
  tests covering two identical generations, exact-limit acceptance,
  limit-plus-one, missing, same-length changed, stale, truncated, trailing, and
  missing-output rejection without repair.
- Added a Rust integration test that spawns the compiled `retro-cli` process for
  NROM-128, NROM-256, and trainer-bearing NROM-128 files and requires exit zero,
  empty standard error, and the complete 47-row/46-transition/152-cycle summary.
- Hardened CI to read-only contents permission and credential-free checkouts;
  added a separate pinned Python 3.13.5 clean-room evidence matrix on Windows
  and Ubuntu. Replaced publisher path patterns with individually enumerated
  paths. Local generator verification, format, warnings-denied clippy, 69 debug
  tests, 69 release tests, release doc tests, and the release app passed.
- Fresh adversarial review found two P2 defense-in-depth gaps: Ubuntu-only
  generator automation and publisher path patterns broader than the reviewed
  snapshot. Adding the Windows evidence leg and fully enumerating the publisher
  fixed both. Re-review found no remaining P0-P2 issues. Initial checkpoint
  `1fedfd85944c4ca58261cff4f823ace04686533d` then exposed CRLF conversion of
  the generated module only on the Windows checkout. Root `.gitattributes`
  now enforces canonical LF text across platforms; focused re-review found no
  P0-P2 issue, and the deletion-safe 54-file preview passed.
- Adopted `GPL-2.0-or-later` for project-owned code and documentation, added the
  canonical GPLv2 text at the repository root, declared the SPDX expression in
  every Rust package, and documented that `NOTICE` material and operator ROMs
  retain their own legal status.
- Added `format-nestest-log`, a separate ASCII byte parser with total-size,
  line-size, and row-count limits; strict state fields; opcode bounds; and
  non-panicking errors that do not echo hostile input.
- Added `NromCpuBus` with 2 KiB RAM mirroring, NROM-128/NROM-256 PRG mapping,
  an optional 8 KiB PRG-memory window, trainer preload at `$7000-$71FF`, ignored
  ROM writes, side-effect-free diagnostic peeks, and first-fault reporting for
  unimplemented PPU/APU/I/O addresses.
- Tightened mapper-0 cartridge validation so unsupported PRG-memory layouts and
  trainers without a PRG-memory window fail before machine construction.
- Added a generated reference runner that initializes from row one, verifies
  representable status bits, compares state/cycles/opcode bytes before every
  transition, checks opcode length, uses the final row as a verified end-state
  sentinel, and stops on the first CPU or bus fault.
- Added a second cargo-fuzz target and redistribution-safe trace seed. The
  Windows launcher now runs both the iNES and trace-log parsers.
- Reviewed Kevin Horton's `nestest` V1.00 identity, immutable mirror pin,
  expected hashes, and 8,991-row convention. No explicit redistribution license
  was found; no external fixture was downloaded, committed, or run.
- Added `retro-cli nes-trace <ROM_PATH> <LOG_PATH>` with bounded reads,
  OS-native path handling, sanitized first-divergence details, stable exit
  statuses, and generated real-filesystem boundary tests.
- Added a byte-oriented testkit entry point that parses each boundary once and
  preserves image, cartridge, log, and trace failure layers.
- Added a strict `nestest-v1` CLI profile that checks raw size/SHA-256 identity
  before parsing, accepts only the QMT CRLF or pinned LF log paired with the
  reviewed ROM, and requires 8,991 rows / 8,990 transitions for success.
- Marked generic `nes-trace` output as `fixture_identity=unchecked`; it remains a
  development harness and cannot be cited as independent acceptance evidence.
- Added RustCrypto `sha2` 0.11.0 to the CLI runtime and testkit development
  tests, with default features disabled. Its MIT/Apache-2.0 licensing, Rust
  1.85 MSRV, old inapplicable advisory, resolved dependency tree, and lockfile
  were reviewed.
- Curated 190 data-only RP2A03 vectors from MIT-licensed
  `SingleStepTests/65x02` commit
  `2f6980a2d95757486c7bee24355c360e40e2a224`. The bounded reproducible curator
  selects expected cycle profiles without calculating expected results, and
  `NOTICE` retains the upstream license.
- Added a public-surface test covering all 151 documented opcode encodings, all
  23 page-penalty encodings with crossed/non-crossed profiles, and all eight
  branches with 2/3/4-cycle profiles. It checks final architectural state,
  declared RAM, and instruction cycles, but not bus access order.
- Added `tools/generate-cleanroom-nrom.py`, which builds project-owned NROM-128
  and NROM-256 diagnostics and traces them with hash-pinned BSD-3-Clause py65
  commit `3138e1b337734a9b2ac1ea90ee7a453514436221`.
- Added two 41-row / 40-transition mapper-0 integration cases through the real
  parser, cartridge, CPU bus, trace runner, and CLI. They exercise CPU RAM
  mirrors, both NROM PRG layouts, PRG RAM, ignored ROM writes, stack/control
  flow, branches, and page-cross cycles without importing game or test-ROM data.
- Extended the clean-room suite to three 47-row / 46-transition cases. The new
  trainer-bearing NROM-128 image proves the full header offset, parser slice,
  owned-cartridge copy, `$7000-$71FF` preload, CPU-read, trace, and CLI path.

The mapper-bus/reference-runner checkpoint passed fresh adversarial review, a
deletion-safe 39-file publisher preview, and the Windows/Linux CI matrix. The
new provenance/operator-CLI changes passed fresh adversarial review after all
three P2 findings were fixed. The deletion-safe 42-file publisher preview and
the Windows/Linux GitHub Actions matrix also passed. The strict identity changes
passed local verification, fresh adversarial review with no actionable P0-P2
findings, a deletion-safe 43-file publisher preview, and the Windows/Linux
GitHub Actions matrix.

The single-step-oracle changes pass local verification and a clean regeneration
produced the exact checked-in SHA-256
`5e8341f1b5b17a3f08835bf81674b6fe01b682d9500a4204540de462a09eeddb`.
Fresh adversarial review found one P1: fractional JSON numbers could be rounded
during integer conversion. The curator now accepts only CLR integer types; a
same-size hostile chunk proved rejection, and re-review found no remaining
P0-P2 issues. A deletion-safe 46-file publisher preview passed and excluded all
local operator fixtures. Checkpoint
`e5f3a4d73738e908b0c2d2fce8c372182a9141fc` is published, and GitHub Actions run
`29262489825` passed the Windows/Linux stable and fuzz matrix.

## Verification performed

Verified locally on Windows x86-64 with Rust/Cargo 1.96.0 and
nightly-2026-07-12 on 2026-07-13:

- `cargo fmt --all -- --check` passed.
- Clippy passed for the workspace, all targets, and all features with warnings
  denied.
- Debug tests: 76 passed, 0 failed.
- Release tests: 76 passed, 0 failed; doc tests passed.
- Both parser fuzz targets completed 10,000 AddressSanitizer executions with no
  crash. Generated seeds contain no third-party ROM or reference-log bytes.
- The release CLI retained tick 30, video hash `2d1f1e3d37030229`, audio hash
  `b2bdf29fe8dd6d45`, and event hash `2343096cdf497a5e`.
- The strict release `nestest-v1` command matched all 8,991 rows / 8,990
  transitions of the exact reviewed QMT pair, including all 76 stable
  undocumented encodings used by the fixture. It ended at `PC=C66E`, `A=00`,
  `X=FF`, `Y=15`, `P=27`, `SP=FD`, and 26,554 cumulative cycles.
- Strict full-trace checkpoint `7f1e20f07a93c432fb0639ea8630474c4e9e26b7`
  is published. GitHub Actions run `29273142006` passed all six Windows/Ubuntu
  test, 10,000-run fuzz, and clean-room evidence jobs. No ignored fixture or
  local run record was published.
- The release CLI help path and generated operator-file path were exercised; the
  latter is also covered by unit tests because no external fixture is present.
- The release binary matched a three-row generated trace and returned status `1`
  for an unsupported opcode on a one-row final sentinel, without printing paths.
- The release `nestest-v1` command rejected a same-size generated ROM with status
  `5` before parsing and did not expose the operator paths or hostile log marker.
- All 190 pinned independent single-step vectors passed. A clean curation from
  the pinned upstream commit was byte-identical, and a short cached chunk was
  rejected before JSON parsing. A same-size chunk containing fractional numeric
  state was rejected rather than rounded.
- The clean-room NROM module regenerated twice to exact SHA-256
  `64b66bef80d0d07f9da4664cdf9d4ef133e070994f375a2d3071a6bda142e6c5`.
  Mutating an imported py65 module caused rejection before output. The release
  CLI matched the NROM-128 case across all 41 rows / 40 transitions and ended at
  `PC=C102`, `A=5A`, and 128 cycles.
- The trainer extension supersedes that module at exact SHA-256
  `c54fb4ce577aa3331386bd6eb91260869493a5c4fbc89fc409f827497d2c9054`.
  Two clean regenerations were byte-identical. All three cases match 47 rows /
  46 transitions and 152 cycles. The release CLI ran the trainer case from real
  files, with `$7000=A7` and `$71FF=82` affecting checked trace rows.
- Fresh adversarial review found two P1 trust-boundary defects. The generator
  could execute cached Python bytecode after validating source, and the
  publisher could follow an allowlisted reparse point outside the workspace.
  The generator now compiles only hash-validated source bytes into fresh
  in-memory modules, bypassing filesystem import/cache resolution; an injected
  external cache produced the exact expected output while a changed source
  failed before output. The publisher now rejects reparse points, locks each
  opened file, validates its final in-root path from the same handle, and reads
  that handle. Focused re-review found no remaining P0-P2 issue in either fix.
- An operator-owned mapper-1 image was moved under ignored local fixtures. Its
  iNES header was inspected as 16 PRG banks, CHR RAM, battery-backed mapper 1;
  the release NROM-only trace command returned exit `3` at its bounded input
  layer and did not attempt emulation.

Mapper-0 bus/reference-runner checkpoint
`505a73c02d69f309cad37d7c85e7520d7e5ab6b6` is published. GitHub Actions run
`29254844214` passed stable tests and both 10,000-run parser ASan fuzz targets on
Windows 2025 and Ubuntu 24.04.

Provenance/operator-CLI checkpoint
`cb4e2de00bb843bef37fa5ef0dc1dc8c08b6a27f` is published. GitHub Actions run
`29257679328` passed the same four-job stable/fuzz matrix on Windows 2025 and
Ubuntu 24.04.

Strict fixture-identity checkpoint
`8bfdec36fc866a2f1c3b37d88e304a7e7ef96e10` is published. GitHub Actions run
`29259546369` passed the same four-job stable/fuzz matrix on Windows 2025 and
Ubuntu 24.04. No external fixture was found or run.

Independent single-step-oracle checkpoint
`e5f3a4d73738e908b0c2d2fce8c372182a9141fc` is published. GitHub Actions run
`29262489825` passed the same four-job stable/fuzz matrix on Windows 2025 and
Ubuntu 24.04. No operator ROM or ignored local record was published.

The clean-room NROM checkpoint passed a deletion-safe 49-file publisher preview.
No managed remote file was missing, non-allowlisted remote files remain
preserved, and no operator fixture was included.

Clean-room NROM mapper-integration checkpoint
`53c65b20e9d572bfe64bdaf0613481dba87d21a3` is published. GitHub Actions run
`29265895004` passed all four stable/fuzz jobs on Windows 2025 and Ubuntu 24.04,
including format, warnings-denied lint, debug/release tests, the release app,
and 10,000 executions of each parser fuzz target.

Trainer-backed NROM and license checkpoint
`93c696b005f4cddcaca932ba210e95aebeaba44a` is published. GitHub Actions run
`29267749389` passed all four stable/fuzz jobs on Windows 2025 and Ubuntu 24.04,
including format, warnings-denied lint, debug/release tests, the release app,
and 10,000 executions of each parser fuzz target.

Automated clean-room evidence checkpoint
`1fedfd85944c4ca58261cff4f823ace04686533d` added the bounded generator and
six-job matrix, but GitHub Actions run `29269716230` correctly failed the new
Windows evidence leg because checkout had converted the generated LF module to
CRLF. The other five jobs passed. Cross-platform LF fix
`f6cf9bc38f895ae839495c76f3adb01963966a6b` is published; GitHub Actions run
`29270030204` passed all six jobs: stable tests, the release app, both
10,000-run parser fuzz targets, and clean-room regeneration on Windows 2025 and
Ubuntu 24.04.

## Key decisions

- External ROMs and reference logs always remain operator-supplied and
  uncommitted; only source metadata, hashes, and sanitized results may be stored.
- The reviewed public `nestest` distribution has no located explicit license;
  public availability is not treated as permission to redistribute it.
- Only strict `nestest-v1` output can satisfy the independent trace gate. Generic
  `nes-trace-v1 fixture_identity=unchecked` output is generated/development
  evidence regardless of row count.
- `format-nestest-log` has no CPU, NES runtime, Bevy, or frontend dependency.
- Unimplemented NES I/O records an explicit bus fault. Returning deterministic
  latched data is only a way to finish the current CPU call safely, not a claim
  that the device access is supported. The only exception is the
  strict CLI's reviewed CPU-trace write allowlist. The verified log's five
  terminal APU writes are discarded without modeling APU state.
- The runner uses the first reference row's raw cycle convention and never
  renormalizes later rows.
- The current milestone remains instruction-oriented, not bus-cycle accurate.
- The clean-room NROM cases are independent mapper-0 architectural evidence,
  not evidence for reset timing, interrupts, bus order, PPU/APU behavior, MMC1,
  or gameplay.
- The pinned MIT single-step sample is independent architectural evidence, not
  exhaustive state coverage. The strict full mapper-0 `nestest` trace is now a
  separate passing oracle.
- The supplied mapper-1 image is a future MMC1 compatibility target. Accepting
  it before serial banking, CHR/nametable routing, PPU/APU, and mapper tests
  would be a false compatibility claim.

## Next action

Add focused IRQ, NMI, reset, and bus-access-order tests, then implement missing
interrupt sampling and per-cycle behavior. Do not start scheduler/PPU work
until that gate passes. After the mapper-0 whole-machine video/audio gate,
implement and verify MMC1 for the supplied operator target.

## Open decisions

1. Whether Linux is a release target or a CI-only target for the first build.
2. Additional independently licensed interrupt/bus suite and acquisition source.
3. Final product name.

## License decision

On 2026-07-13 the project selected `GPL-2.0-or-later` for project-owned source
code and documentation. This keeps GPL version 2 available while allowing a
recipient to choose a later GPL version. The root `LICENSE` contains the full
GPLv2 terms; package manifests carry the SPDX expression; `NOTICE` preserves
separate third-party licenses. No project license applies to commercial ROMs,
firmware, or operator-supplied fixtures, which remain outside the repository.
The normalized license text matched GitHub's canonical GPL-2.0 template at
SHA-256 `8177f97513213526df2cf6184d8ff986c675afb514d4e68a404010521b880643`.
Cargo metadata confirmed the SPDX expression for all seven workspace crates and
the fuzz package; all current external Rust dependencies report
`MIT OR Apache-2.0`. Format, warnings-denied clippy, 68 debug tests, 68 release
tests, release doc tests, and a fresh P0-P2 license review passed. The publisher
dry-run validated all 50 allowlisted files without making a GitHub write.

## Environment notes

`git` is not installed/on `PATH`. Publishing uses the authenticated, allowlisted
Git Data API script, preserves remote-only files by default, and requires a dry
run. On x64 Windows, use `tools/run-fuzz.ps1` so the Visual Studio AddressSanitizer
runtime is placed on the child process path.
