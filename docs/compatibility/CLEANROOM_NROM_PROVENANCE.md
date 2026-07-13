# Clean-room NROM diagnostic provenance

Last updated: 2026-07-13

`tools/generate-cleanroom-nrom.py` constructs three project-owned diagnostic
images and independent instruction-boundary traces. No Nintendo, commercial
game, `nestest`, or operator-supplied bytes are inputs or outputs committed to
the repository. The generated Rust module reconstructs each image in memory.

## Pinned oracle

- Source: <https://github.com/mnaberez/py65>
- Revision: `3138e1b337734a9b2ac1ea90ee7a453514436221`
- License: BSD-3-Clause; retained in `NOTICE`
- `LICENSE.txt` SHA-256:
  `82242fe6c832b58a917269754bc6c0f1ec02993802e78dc897fe9b365605a08b`

| Validated file | SHA-256 |
|---|---|
| `py65/__init__.py` | `4f6a41c619e2f0d6fc48eaf9fbc9ca31729365888c26df8c7750cf4c571ee8fc` |
| `py65/devices/__init__.py` | `4f6a41c619e2f0d6fc48eaf9fbc9ca31729365888c26df8c7750cf4c571ee8fc` |
| `py65/devices/mpu6502.py` | `15d93835b4f279b702270d9a0b417938291347ec57cc12d9e0307cd344d381fe` |
| `py65/utils/__init__.py` | `4f6a41c619e2f0d6fc48eaf9fbc9ca31729365888c26df8c7750cf4c571ee8fc` |
| `py65/utils/conversions.py` | `448b8aa2cc59aa71a6a644a5dd93051ab4b2def2ae720c4991c0e54f31a4eaa9` |
| `py65/utils/devices.py` | `fca3864bebcf1db7dbc13014487ad7091499d85c8cb14ca7288c5fdc63de6a0e` |

The generator reads and validates the license, CPU module, every executed
package initializer, and the two imported py65 utility modules against pinned
hashes before executing code. It creates fresh in-memory modules and directly
compiles only those validated source bytes; importlib, filesystem module search,
and bytecode caches are not in the execution path. It then rejects any extra
py65 module. Cached bytecode and reparse points therefore cannot change the
executed oracle. Missing, oversized, or changed inputs fail before output. The
hashes identify the raw LF bytes served for the exact commit, avoiding checkout
line-ending conversion.

## Reproduction and identities

The normal bounded verification command downloads only the seven files listed
above, caps each response at 1,000,000 bytes, and runs the hostile regressions
plus production check mode:

```powershell
python tools/check-cleanroom-nrom.py
```

CI runs this command on Windows 2025 and Ubuntu 24.04 with Python 3.13.5 from
immutable `actions/setup-python` v6.3.0 commit
`ece7cb06caefa5fff74198d8649806c4678c61a1`. Checkouts do not retain credentials,
and workflow permissions are read-only.

For an intentional regeneration with an already available pinned py65 tree
outside this repository, use the low-level write command and then run the
bounded verification command again:

```powershell
python tools/generate-cleanroom-nrom.py `
  --py65-root <PINNED_PY65_CHECKOUT> `
  --output crates/retro-testkit/src/cleanroom_nrom.rs
```

The checked-in generated module SHA-256 is
`02f88830b4af0d46b3ba542a713c4fddd94f6c9af4f9b49e69d92bc03a3bfab5`.
Two raw-source regenerations must be byte-identical to each other and the
checked-in module. Tests require exact-limit acceptance and reject limit-plus-one,
missing, same-length mutated, stale, trailing, and missing-output inputs without
repairing the target.

| Case | Image SHA-256 | Trace SHA-256 | Rows / transitions | Final state |
|---|---|---|---|---|
| NROM-128 | `bb14da0e0f2d53e36bc92950928a1b16b29d11447baa01eaf7aad24676f14361` | `d93c0825f52ec5ff8739168f763ba33aa84a5a3ddeb06e62191a82b14a488f8d` | 47 / 46 | `PC=C102 A=5A X=01 Y=00 P=25 SP=FD CYC=152` |
| NROM-256 | `adb37217656eb7ad82e68eb72de3e6fb3bb2f6771bd7d829833b8044ec8533d3` | `07404eda6e717db24e44286e853fcc1bb6c3264c5260cfb5e960e185a8cf0612` | 47 / 46 | `PC=C102 A=5A X=01 Y=00 P=25 SP=FD CYC=152` |
| NROM-128 + trainer | `48045390f5a90453675d2e513d6d727d0e7b8fe5d2451137e9ca72c25785b51f` | `88b9cb0ca690e1b4c9dd92109135fc77a2fcdee63d506545844d2dbd769c3e8d` | 47 / 46 | `PC=C102 A=5A X=01 Y=00 P=25 SP=FD CYC=152` |

## Evidence boundary

The diagnostics exercise mapper-0 CPU-side RAM mirrors, NROM-128 PRG mirroring,
NROM-256 bank placement, PRG RAM, ignored PRG-ROM writes, stack/control flow,
branches, and page-cross cycle totals through the real parser, cartridge, CPU
bus, trace runner, and CLI boundary. The trainer is project-owned deterministic
data with byte `i = (37 * i + A7) mod 256`; it yields `$7000=A7` and
`$71FF=82`. Both values affect independently checked trace rows, so correct
header offset and preload are required. Starting CPU state is
explicit; reset is not invoked. Decimal mode stays clear. A Rust integration
test spawns the compiled `retro-cli` executable for all three cases and requires
the exact complete summary, zero exit status, and empty standard error.

This evidence is architectural and intentionally narrow: `bus_order=unchecked`,
`reset=unchecked`, `interrupts=unchecked`, and `nestest=unrun`. It does not
exercise the PPU/APU, CHR reads, DMA, unofficial opcodes, MMC1, gameplay, or a
frontend, and it does not replace the strict 8,990-transition `nestest-v1` gate.
