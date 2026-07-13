# Nestest Comparison Procedure

Status: the bus, bounded log parser, comparison runner, and operator-path CLI
are exercised with generated data. No operator-supplied ROM/reference pair has
been run.

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
- Still required: an operator-supplied fixture pair matching a reviewed identity
  and a sanitized local run record. The absence of an explicit redistribution
  license is recorded; neither file may be added to the repository.

## Local command

```powershell
cargo run --release -p retro-cli -- nes-trace <ROM_PATH> <LOG_PATH>
```

The CLI reads at most 41,488 bytes for the supported NROM image and 4 MiB for the
log. It accepts OS-native paths but never prints either path or raw log rows.
Success prints a single `nes-trace-v1` summary. A mismatch prints only sanitized
architectural state for the first divergence.

Exit statuses are stable: `0` pass, `1` CPU/bus/trace divergence, `2` invalid
arguments, `3` unreadable or oversized input, and `4` malformed or unsupported
fixture data. The no-argument command remains the deterministic synthetic smoke
run used by CI.
