# Nestest Comparison Procedure

Status: the bus, bounded log parser, and comparison runner are exercised with
generated data. No reviewed operator-supplied ROM/reference pair has been run.

## Fixture boundary

The operator supplies `nestest.nes` and its matching reference log outside the
repository. Before the first run, record the upstream URL, revision/date,
license or redistribution terms, and SHA-256 of both files in an ignored
`external-fixtures/` run record. Do not acquire, copy, or publish either file
until those details have been independently verified.

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
   the first mismatch and record the row number, both states, opcode bytes, and
   the immediately preceding row.
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
- Still required for an external run: reviewed fixture source/revision/license,
  a local CLI, and a sanitized run record containing hashes and the first
  divergence but never fixture bytes or operator paths.
