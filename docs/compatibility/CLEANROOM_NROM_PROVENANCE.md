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
  `aff1cd260d7d6367ccc9ecb28e6823d54ec7cfd254c27e43ae76c2747a7dc6a1`

| Validated file | SHA-256 |
|---|---|
| `py65/__init__.py` | `9c3218570ae3ad9bc4ca97809f9f24f490ac94b4d2557970b8ebb57dbbb87c7e` |
| `py65/devices/__init__.py` | `9c3218570ae3ad9bc4ca97809f9f24f490ac94b4d2557970b8ebb57dbbb87c7e` |
| `py65/devices/mpu6502.py` | `bdae2b7ef3e2a38519a007412280107f330c6fc6433738364578fe8338e57e7e` |
| `py65/utils/__init__.py` | `9c3218570ae3ad9bc4ca97809f9f24f490ac94b4d2557970b8ebb57dbbb87c7e` |
| `py65/utils/conversions.py` | `8c1a947f24351ced8265dae7e94eb8b13b24f489a4f62f15040af1a597997d44` |
| `py65/utils/devices.py` | `1ab16cbe2c6ae2213452d9ff577dec3856df209e70361e1f1598ffb7cbbf0a1c` |

The generator reads and validates the license, CPU module, every executed
package initializer, and the two imported py65 utility modules against pinned
hashes before executing code. It creates fresh in-memory modules and directly
compiles only those validated source bytes; importlib, filesystem module search,
and bytecode caches are not in the execution path. It then rejects any extra
py65 module. Cached bytecode and reparse points therefore cannot change the
executed oracle. Missing, oversized, or changed inputs fail before output.

## Reproduction and identities

With the pinned py65 checkout available outside this repository:

```powershell
python tools/generate-cleanroom-nrom.py `
  --py65-root <PINNED_PY65_CHECKOUT> `
  --output crates/retro-testkit/src/cleanroom_nrom.rs
```

The checked-in generated module SHA-256 is
`c54fb4ce577aa3331386bd6eb91260869493a5c4fbc89fc409f827497d2c9054`.
Two clean regenerations on 2026-07-13 were byte-identical. A mutated imported
py65 module was rejected before output.

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
explicit; reset is not invoked. Decimal mode stays clear.

This evidence is architectural and intentionally narrow: `bus_order=unchecked`,
`reset=unchecked`, `interrupts=unchecked`, and `nestest=unrun`. It does not
exercise the PPU/APU, CHR reads, DMA, unofficial opcodes, MMC1, gameplay, or a
frontend, and it does not replace the strict 8,990-transition `nestest-v1` gate.
