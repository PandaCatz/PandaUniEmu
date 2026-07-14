# Test and Oracle Provenance

Last updated: 2026-07-14

No original game, firmware, copyrighted reference-output, or third-party test
ROM bytes may be committed. External fixtures remain operator-supplied local
files even if redistribution would be permitted; publish only reviewed source
metadata, hashes, and sanitized results.

| Fixture or oracle | Repository status | Provenance rule | Current use |
|---|---|---|---|
| Generated iNES/NES 2.0 byte vectors | Committed as Rust test construction code | Original project code; contains no Nintendo data | Parser validity, truncation, size, dirty-header, and mapper-boundary tests |
| Generated reference-log rows | Committed as Rust string construction code | Original project data; not copied from an external log | Parser boundaries and generated mapper-0 trace comparison |
| Clean-room NROM diagnostics | Project-owned generator and generated Rust data committed; images reconstructed in memory | Original diagnostic program traced by BSD-3-Clause py65 commit `3138e1b337734a9b2ac1ea90ee7a453514436221`; raw LF source hashes, bounded downloader/check procedure, and evidence boundary in `compatibility/CLEANROOM_NROM_PROVENANCE.md` | Three independent 47-row traces cover NROM-128, NROM-256, and trainer slicing/preload through the mapper runner and a spawned CLI process; automated regeneration detects stale output, but bus order, reset, interrupts, PPU, and `nestest` remain unchecked |
| Curated RP2A03 single-step vectors | 190 data-only Rust vectors committed with `NOTICE`; reproducible curator is `tools/curate-nes6502-vectors.ps1` | MIT-licensed `SingleStepTests/65x02`, pinned at commit `2f6980a2d95757486c7bee24355c360e40e2a224`; generated-file SHA-256 `a53a81800b37bbfb5f5101785974bbed9070c103a09d206988101a79969922fa` | Independent architectural state, declared RAM, cycle counts, and ordered instruction bus reads/writes for all 151 documented encodings; paired page-penalty and branch profiles; all 190 sampled traces match through the live one-cycle interface, but hardware interrupt/reset entry is outside this oracle |
| `perfect6502` transistor-level oracle | Upstream files remain in a local temp cache; the repository contains only pinned metadata, an opt-in hash-checking acquisition/build verifier, and a project-owned harness | `mist64/perfect6502` commit `09fc542877a84318291aa42dab143a3e2c3db974`, archive SHA-256 `594553a873d66a13e88c134495c9f55e064a36ba4670b07fba71f5047a77bdf5`; simulator C is MIT, while required `netlist_6502.h` has a file-specific CC BY-NC-SA 3.0 notice and attribution requirement | Locally executed evidence for the exact seven-cycle IRQ/NMI/reset bus sequences, second-to-last-cycle polling across NOP and all three branch paths, and the BRK/IRQ NMI-hijack vector-lock boundary; no upstream source, netlist, or generated binary is published or required at runtime |
| `cargo-fuzz` generated corpus/artifacts | Ignored | Generated from arbitrary bytes; minimize and inspect before promoting a regression case into source | Both format parsers; 10,000-run Windows ASan smoke per target passed 2026-07-14 |
| Synthetic core output | Committed as algorithms and numeric hashes | Original project code | Deterministic scheduling, reset, split-run, input ordering, video/audio/event hashes |
| `nestest.nes` | Never commit | Kevin Horton V1.00 identity is pinned in `compatibility/NESTEST_PROVENANCE.md`; no explicit redistribution license was found; operator supplies a lawful local copy | The strict release CLI verified the exact reviewed ROM/log pair on 2026-07-14: 8,991 rows and 8,990 transitions, ending at `PC=C66E` after 26,554 cycles |
| Matching `nestest` reference log | Never commit | QMT CRLF and archival LF size/hash identities are pinned; neither is redistributed because no explicit license was found | The successful strict run used the reviewed QMT CRLF identity; only sanitized aggregate state/counts are committed |
| Operator-supplied AI NES notes | Originals remain external; only `NES_REFERENCE_INTAKE.md` is committed | No author, source, license, or attribution metadata was found; do not copy code/text verbatim or use it as an oracle | Topic/checklist input only; project architecture, pinned vectors, defensive boundaries, and independently checked timing remain authoritative |
| Blargg CPU/interrupt test ROMs | Local-only until each archive's terms are reviewed | Record upstream URL/revision, archive hash, individual ROM hash, and execution result | Planned timing, interrupt, reset, and unofficial-opcode gates |
| Operator-owned commercial images | Never commit | Use only from the operator's own dumps; record opaque local identifier/hash and observed result | Later compatibility testing only |

The curator downloads only the first 65,536 bytes of each required pinned
upstream opcode file, accepts complete brace-balanced records, selects fixed
cycle profiles without calculating expected results, and emits data-only Rust.
A clean offline regeneration on 2026-07-14 produced the exact checked-in
SHA-256 above.

## Run-record requirements

Every external-oracle run records, without copying the fixture:

- date, command, host target, Rust version, build profile, and core commit;
- upstream/source URL and exact revision where applicable;
- SHA-256 of the local fixture and reference output;
- selected region/configuration and starting state;
- first divergence with its line/category and relevant sanitized state, opcode,
  CPU, or bus context emitted by the CLI;
- pass/fail and any explicitly unsupported behavior.

Local run records containing operator paths or fixture hashes belong under
`external-fixtures/`, which is ignored. Sanitized aggregate results may be
committed under `docs/compatibility/` after review.

## Hash implementation dependency

Strict identity uses RustCrypto `sha2` 0.11.0 in `retro-cli`; testkit development
tests use the same pinned crate to verify generated image and trace hashes.
Default features are disabled. The crate declares Rust 1.85 and
`MIT OR Apache-2.0`; the workspace lockfile pins the reviewed resolution. The
resolved normal graph is
`sha2`, `digest`, `cfg-if`, target-specific `cpufeatures`, `block-buffer`,
`crypto-common`, `hybrid-array`, and `typenum` (plus target lockfile entries).
This dependency is pure Rust but is not claimed to be unsafe-free: optimized
hardware backends contain reviewed intrinsic calls.
