# Nestest Comparison Procedure

Status: specified but not executable yet. The project has neither an integrated
NES CPU bus/trace runner nor reviewed local fixtures.

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
5. Execute exactly one instruction. Stop at the first mismatch and record the
   row number, both states, opcode bytes, and the immediately preceding row.
6. The declared endpoint is the end of the matching reference log. A pass is
   zero mismatches and exactly one executed instruction per parsed row; early
   termination, extra execution, unsupported opcodes, or cycle renormalization
   after row one is a failure.

Status comparison includes all stored processor bits. Any special treatment of
the B/unused bits must be justified against the reviewed reference format and
implemented in the log parser, not hidden in the CPU comparison.

## Required implementation before running

- A mapper-0 CPU bus with PRG mirroring and RAM mirroring.
- A trace-log parser that rejects malformed rows without panicking.
- A runner that can initialize the declared architectural state without
  introducing unrecorded reset cycles.
- A sanitized run-record writer that stores hashes and the first divergence,
  never fixture bytes or operator paths.
