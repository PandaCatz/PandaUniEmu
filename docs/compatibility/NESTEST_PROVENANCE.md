# Nestest Fixture Provenance

Last reviewed: 2026-07-13

## Identified reference pair

The intended CPU oracle is Kevin Horton's `nestest` V1.00, dated 2004-09-06.
The [author-hosted documentation](https://www.qmtpro.com/~nes/misc/nestest.txt)
identifies the version, date, author, `$C000` automation entry point, and test
scope. The [NESdev emulator-test index](https://www.nesdev.org/wiki/Emulator_tests)
identifies Kevin Horton as the author and describes the known-good log as
Nintendulator-generated.

Author-hosted distribution references:

- [`nestest.nes`](https://www.qmtpro.com/~nes/misc/nestest.nes)
- [`nestest.log`](https://www.qmtpro.com/~nes/misc/nestest.log)
- [`nestest.txt`](https://www.qmtpro.com/~nes/misc/nestest.txt)

For immutable identity, use the `other/` directory at archival mirror commit
[`a215f7ea9c90e9bd4c22a74ee824fc6405533b16`](https://github.com/christopherpow/nes-test-roms/tree/a215f7ea9c90e9bd4c22a74ee824fc6405533b16/other).
The modern reference log was updated by that commit; the ROM was introduced by
commit `97720008e51db15dd281a2a1e64d4c65cf1bca4c` and is unchanged at the pin.

## Expected identities

These values identify the reviewed public distributions; they are not evidence
that the files have been run locally.

| File | Encoding | Bytes | SHA-256 |
|---|---|---:|---|
| QMT `nestest.nes` | Binary | 24,592 | `f67d55fd6b3cf0bad1cc85f1df0d739c65b53e79cecb7fea8f77ec0eadab0004` |
| QMT `nestest.log` | CRLF | 868,158 | `627c8e180b1a924dfa705c5dc6958fad7ab75a62de556173caf880ccc1337540` |
| Pinned mirror `nestest.log` | LF | 859,167 | `442c4dd5539c7e88b3fd73c7b732a7eadbd22b47c2cd9e58397ef147f64f6f8f` |
| QMT `nestest.txt` | Text | 17,774 | `8291241ba9a0885b9a604a4685101a1473e22b3aa070bc828e3b8c342d7f71fb` |

The two log files contain the same 8,991 logical rows and differ by line
endings. Normalizing the QMT log to LF produces the pinned-log SHA-256 above.
Do not normalize the operator file before recording its original hash.

## Redistribution decision

No explicit license grant was found in the author documentation, the archival
mirror, or the mirrored file history. Public availability is not a
redistribution license. The exact Nintendulator build used to create the modern
log is also undocumented.

Therefore this repository stores only provenance metadata, expected hashes,
and sanitized results. It does not store, vendor, automatically download, or
redistribute the ROM or log. The operator must provide locally obtained files
and is responsible for confirming that their use is lawful.
