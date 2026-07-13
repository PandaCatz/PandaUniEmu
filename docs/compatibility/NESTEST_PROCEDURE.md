# Nestest Comparison Procedure

Status: passed on 2026-07-13 with the exact reviewed QMT CRLF pair: 8,991 rows,
8,990 transitions, final `PC=C66E`, and 26,554 cumulative CPU cycles.

## Fixture boundary

The operator supplies `nestest.nes` and its matching reference log outside the
repository. The reviewed identity and unresolved redistribution status are in
`NESTEST_PROVENANCE.md`. Before the first run, verify and record the SHA-256 of
both local files in an ignored `external-fixtures/` run record. Never copy or
publish either fixture in this repository.

## Comparison convention

1. Parse the image through `format-ines`; do not reinterpret its raw header in
   the CPU harness.
2. Map it through the active mapper-0 bus implementation.
3. Parse the first reference row and initialize PC, A, X, Y, P, SP, and the
   cumulative CPU-cycle counter to that row's pre-instruction values. This
   avoids silently assuming a reset preamble or a zero/seven-cycle convention.
4. Before each instruction, compare PC, A, X, Y, P, SP, and cumulative CPU
   cycles with the corresponding reference row. Opcode bytes are diagnostic
   context and must match bytes read through the mapped CPU bus.
5. For every row except the final row, execute exactly one instruction. Stop at
   the first mismatch. State mismatches report both architectural states and the
   preceding expected PC; opcode mismatches report the PC and expected/actual
   bytes; CPU and bus faults report their line and structured cause. Diagnostics
   never include a raw row or operator path.
6. Treat the final row as the expected post-state sentinel for the preceding
   transition; compare its state and opcode bytes but do not execute it. A pass
   is zero mismatches and exactly `row_count - 1` verified transitions. Early
   termination, extra execution, unsupported opcodes, or cycle renormalization
   after row one is a failure.

Status comparison includes all stored processor bits. Any special treatment of
the B/unused bits must be justified against the reviewed reference format and
implemented in the log parser, not hidden in the CPU comparison.

## Implementation and run prerequisites

- Implemented: mapper-0 CPU bus with PRG and internal-RAM mirroring.
- Implemented: isolated bounded trace-log parser with malformed-input tests and
  a dedicated fuzz target.
- Implemented: runner initialization from the first row without unrecorded
  reset cycles, plus generated state/opcode/cycle/fault comparison tests.
- Implemented: bounded operator-path CLI with sanitized diagnostics and distinct
  exit statuses.
- Implemented: the 76 stable undocumented encodings exercised by the fixture.
- Implemented: after verifying both exact fixture identities, the strict CLI
  selects a CPU-only allowlist for `$4004`-`$4007` and `$4015` writes. The
  reviewed log performs exactly five terminal writes there. The normal NROM bus
  still faults unimplemented I/O; this policy is not evidence of APU behavior.

The absence of an explicit redistribution license remains recorded; neither
fixture file may be added to the repository or publisher snapshot.

## Local command

```powershell
cargo run --release -p retro-cli -- nestest-v1 <ROM_PATH> <LOG_PATH>
```

The CLI reads at most 41,488 bytes for the supported NROM image and 4 MiB for the
log. It accepts OS-native paths but never prints either path or raw log rows.
Identity is checked against `NESTEST_PROVENANCE.md` before parsing. Success
prints one `nestest-v1` line with the reviewed fixture ID, hashes, log variant,
byte counts, 8,991 matched rows, 8,990 transitions, and final architectural
state. A mismatch prints only sanitized architectural context for the first
divergence. The generic `nes-trace` command prints
`fixture_identity=unchecked` and must not be cited as acceptance evidence.

Exit statuses are stable: `0` verified pass, `1` CPU/bus/trace divergence or an
impossible strict-summary invariant, `2` invalid arguments, `3` unreadable or
oversized input, `4` identity-approved bytes rejected as malformed/unsupported,
and `5` raw bytes outside the reviewed identity matrix. Identity failures omit
observed hashes, sizes, paths, and contents. The no-argument command remains the
deterministic synthetic smoke run used by CI.
