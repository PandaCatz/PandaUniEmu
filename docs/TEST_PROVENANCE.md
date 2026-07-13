# Test and Oracle Provenance

Last updated: 2026-07-13

No original game, firmware, copyrighted reference-output, or third-party test
ROM bytes may be committed. External fixtures remain operator-supplied local
files even if redistribution would be permitted; publish only reviewed source
metadata, hashes, and sanitized results.

| Fixture or oracle | Repository status | Provenance rule | Current use |
|---|---|---|---|
| Generated iNES/NES 2.0 byte vectors | Committed as Rust test construction code | Original project code; contains no Nintendo data | Parser validity, truncation, size, dirty-header, and mapper-boundary tests |
| `cargo-fuzz` generated corpus/artifacts | Ignored | Generated from arbitrary bytes; minimize and inspect before promoting a regression case into source | `format_ines::parse` no-panic/error-boundary fuzzing; 10,000-run Windows ASan smoke passed 2026-07-13 |
| Synthetic core output | Committed as algorithms and numeric hashes | Original project code | Deterministic scheduling, reset, split-run, input ordering, video/audio/event hashes |
| `nestest.nes` | Never commit by default | Operator supplies a legally obtained local copy; record source/revision/license and hash in a noncommitted run record before use | Planned 2A03 instruction trace oracle; acquisition source still unverified |
| Matching `nestest` reference log | Never commit | Record source/revision/license and hash; compare under `compatibility/NESTEST_PROCEDURE.md` | Planned PC/register/status/cycle comparison; runner absent |
| Blargg CPU/interrupt test ROMs | Local-only until each archive's terms are reviewed | Record upstream URL/revision, archive hash, individual ROM hash, and execution result | Planned timing, interrupt, reset, and unofficial-opcode gates |
| Operator-owned commercial images | Never commit | Use only from the operator's own dumps; record opaque local identifier/hash and observed result | Later compatibility testing only |

## Run-record requirements

Every external-oracle run records, without copying the fixture:

- date, command, host target, Rust version, build profile, and core commit;
- upstream/source URL and exact revision where applicable;
- SHA-256 of the local fixture and reference output;
- selected region/configuration and starting state;
- first divergence with instruction number, PC, registers, status, and cycles;
- pass/fail and any explicitly unsupported behavior.

Local run records containing operator paths or fixture hashes belong under
`external-fixtures/`, which is ignored. Sanitized aggregate results may be
committed under `docs/compatibility/` after review.
