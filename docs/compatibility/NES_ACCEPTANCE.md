# NES Acceptance Matrix

Current target: NTSC NES/Famicom base console, documented 2A03 instructions,
mapper 0 cartridge boundary. This document defines gates; it does not claim they
have passed unless the evidence column says so.

| Layer | Required evidence | Current status |
|---|---|---|
| iNES/NES 2.0 parser | Generated valid/malformed tests, every truncation point, configured size limits, dirty-header rejection, and libFuzzer smoke/soak | Unit/adversarial tests pass; Windows AddressSanitizer smoke completed 10,000 runs through the checked-in launcher; Linux CI smoke pending |
| Shared scheduler contract | Split and single runs match; reset repeats exact output; future input wins same-timestamp ordering; combined event hash is stable | Passed by synthetic-core tests |
| Documented 2A03 opcodes | Independently checked opcode metadata plus exhaustive semantics/flags/addressing/cycle cases | The canonical set of 151 encodings decodes and selected generated semantic tests pass; independent metadata and instruction trace are absent |
| Independent CPU trace | Operator-supplied `nestest` trace matches PC, A, X, Y, P, SP, and cumulative cycles through the final end-state row under `NESTEST_PROCEDURE.md` | Bounded parser, bus, and generated comparison pass; local CLI and reviewed external fixture absent |
| IRQ/NMI/reset | Dedicated external suites plus focused generated tests cover stack bytes, vectors, B/U bits, masking, and edge timing | Generated instruction-level tests planned; external suite absent |
| Unofficial opcodes | Explicit supported-encoding table and independent suite | Out of current milestone scope |
| Mapper 0 execution | Parsed NROM-128/NROM-256 images map PRG/CHR correctly and run CPU traces without inline re-parsing | CPU RAM/PRG mapping, trainer preload, ROM write behavior, reset vector, hostile memory-layout rejection, and generated trace pass; PPU-side CHR bus absent |
| PPU/APU/gameplay | Headless timing/oracle suites and operator-owned compatibility matrix | Not started |

## CPU milestone boundary

The first CPU crate is instruction-trace oriented. Selected generated tests
exercise architectural state, memory effects, instruction cycle totals,
page-crossing penalties, stack behavior, and the NMOS indirect-JMP wrap quirk.
The decoding table and full behavior have not yet been independently verified,
so accuracy is not claimed. Dummy reads/writes, interrupt sampling races, DMA
stalls, and exact bus access order remain required before PPU/APU integration.

## Fuzz commands

Run from the project root with the installed nightly toolchain:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/run-fuzz.ps1 -Runs 10000
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/run-fuzz.ps1 -Runs 1000000 -MaxTotalTimeSeconds 60
```

On x64 Windows/MSVC, cargo-fuzz links the Visual Studio AddressSanitizer runtime
but does not always place its directory on `PATH`. The launcher enforces the
`x86_64-pc-windows-msvc` host, discovers the installed x64 runtime, adds that
directory only for the child process, and fails with an installation instruction
if the C++ AddressSanitizer component is missing. Other Windows architectures
are not yet supported by this launcher.
The launcher also creates original zero-filled valid/truncated image seeds and a
generated valid reference row, then sets a 64 KiB mutation limit. A clean CI
checkout therefore exercises both `format_ines::parse` and
`format_nestest_log::parse`. On 2026-07-13 both targets completed 10,000 runs
locally with AddressSanitizer enabled. This bounded job is a smoke gate, not
evidence of a long fuzz soak.

CI uses bounded deterministic unit/adversarial tests. Longer fuzzing is a local
or scheduled security gate because libFuzzer duration is intentionally open
ended.
