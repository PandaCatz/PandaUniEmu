# Test and Oracle Provenance

Last updated: 2026-07-13

No original game, firmware, copyrighted reference-output, or third-party test
ROM bytes may be committed. External fixtures remain operator-supplied local
files even if redistribution would be permitted; publish only reviewed source
metadata, hashes, and sanitized results.

| Fixture or oracle | Repository status | Provenance rule | Current use |
|---|---|---|---|
| Generated iNES/NES 2.0 byte vectors | Committed as Rust test construction code | Original project code; contains no Nintendo data | Parser validity, truncation, size, dirty-header, and mapper-boundary tests |
| Generated reference-log rows | Committed as Rust string construction code | Original project data; not copied from an external log | Parser boundaries and generated mapper-0 trace comparison |
| `cargo-fuzz` generated corpus/artifacts | Ignored | Generated from arbitrary bytes; minimize and inspect before promoting a regression case into source | Both format parsers; 10,000-run Windows ASan smoke per target passed 2026-07-13 |
| Synthetic core output | Committed as algorithms and numeric hashes | Original project code | Deterministic scheduling, reset, split-run, input ordering, video/audio/event hashes |
| `nestest.nes` | Never commit | Kevin Horton V1.00 identity is pinned in `compatibility/NESTEST_PROVENANCE.md`; no explicit redistribution license was found; operator supplies a lawful local copy | Strict CLI enforces exact byte count/SHA-256 before parsing; external comparison not run |
| Matching `nestest` reference log | Never commit | QMT CRLF and archival LF size/hash identities are pinned; neither is redistributed because no explicit license was found | Strict CLI accepts only the two reviewed raw encodings and requires 8,991/8,990 on success; external comparison not run |
| Blargg CPU/interrupt test ROMs | Local-only until each archive's terms are reviewed | Record upstream URL/revision, archive hash, individual ROM hash, and execution result | Planned timing, interrupt, reset, and unofficial-opcode gates |
| Operator-owned commercial images | Never commit | Use only from the operator's own dumps; record opaque local identifier/hash and observed result | Later compatibility testing only |

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

Strict identity uses RustCrypto `sha2` 0.11.0 in `retro-cli` only, with default
features disabled. The crate declares Rust 1.85 and `MIT OR Apache-2.0`; the
workspace lockfile pins the reviewed resolution. The resolved normal graph is
`sha2`, `digest`, `cfg-if`, target-specific `cpufeatures`, `block-buffer`,
`crypto-common`, `hybrid-array`, and `typenum` (plus target lockfile entries).
This dependency is pure Rust but is not claimed to be unsafe-free: optimized
hardware backends contain reviewed intrinsic calls.
