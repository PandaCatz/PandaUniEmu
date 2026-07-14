# NES Acceptance Matrix

Current target: NTSC NES/Famicom base console, documented 2A03 instructions,
mapper 0 cartridge boundary. This document defines gates; it does not claim they
have passed unless the evidence column says so.

| Layer | Required evidence | Current status |
|---|---|---|
| iNES/NES 2.0 parser | Generated valid/malformed tests, every truncation point, configured size limits, dirty-header rejection, and libFuzzer smoke/soak | Unit/adversarial tests pass; Windows and Linux AddressSanitizer smoke completed 10,000 runs through the checked-in launcher |
| Shared scheduler contract | Split and single runs match; reset repeats exact output; future input wins same-timestamp ordering; combined event hash is stable | Passed by synthetic-core tests |
| Documented 2A03 opcodes | Independently checked opcode set plus broad semantics/flags/addressing/cycle cases | A reproducible 190-vector sample from the pinned MIT `SingleStepTests/65x02` RP2A03 suite passes all 151 documented encodings, all 23 paired page-penalty profiles, all eight branch 2/3/4-cycle profiles, and every sampled ordered bus trace byte for byte; the sample is not exhaustive |
| Independent CPU trace | Strict `nestest-v1` output proves the reviewed operator pair matched 8,991 rows / 8,990 transitions for PC, A, X, Y, P, SP, and cumulative cycles through the final end-state row | Passed the exact QMT CRLF pair: 8,991 rows, 8,990 transitions, final `PC=C66E`, and 26,554 cumulative cycles; after identity verification the strict CLI allows writes only to `$4004`-`$4007` and `$4015`, which are the fixture's five terminal APU-register writes, without claiming APU behavior |
| Clean-room mapper-0 integration | Project-owned NROM-128/NROM-256 programs match a pinned, independently generated architectural trace through the real parser, mapper bus, runner, and CLI | Three 47-row / 46-transition py65 traces pass with pinned hashes; the trainer case executes `$7000` and `$71FF` reads after parser slicing and preload; bus order, reset, interrupts, and PPU remain unchecked |
| IRQ/NMI/reset | Dedicated external suites plus focused generated tests cover stack bytes, vectors, B/U bits, masking, and edge timing | Live seven-cycle entry, polling, and hijacking match the pinned transistor-level oracle; the PPU shell drives the same edge-triggered NMI path, while dot-exact PPUSTATUS suppression races remain open |
| Unofficial opcodes | Explicit supported-encoding table and independent suite | The exact 76 stable encodings exercised by the identity-checked `nestest` trace pass; jam and hardware-sensitive unstable encodings remain unsupported |
| Mapper 0 execution | Parsed NROM-128/NROM-256 images map PRG/CHR correctly and run CPU traces without inline re-parsing | CPU RAM/PRG mapping, trainer preload, ROM write behavior, reset vector storage, hostile memory-layout rejection, independent clean-room CPU traces, and NROM CHR-ROM/CHR-RAM PPU routing pass |
| PPU register/address shell | Focused register, address-map, mirroring, buffering, OAM-port, timing, and NMI tests | Deterministic shell passes generated tests for mirrored `$2000-$3FFF` ports, shared scroll/address state, PPUDATA buffering, horizontal/vertical/four-screen nametables, palette aliases, and logical VBlank NMI |
| PPU rendering | Fetch/scroll/sprite/pixel oracles plus dot-exact VBlank/status race suites | Not started; no rendering or cycle-accurate PPU claim |
| APU/DMA/input/gameplay | Headless timing/oracle suites and operator-owned compatibility matrix | Not started |

## CPU milestone boundary

The first CPU crate is instruction-trace oriented. Generated tests and a pinned
independent 190-vector sample exercise architectural state, declared memory
effects, instruction cycle totals, page-crossing penalties, stack behavior, and
the NMOS indirect-JMP wrap quirk across all documented encodings. The sample is
not exhaustive. The strict full mapper trace now passes, including its 76
stable undocumented encodings. Sampled documented-opcode bus order and selected
hardware interrupt polling races are verified. DMA stalls, exhaustive
undocumented-opcode bus order, PPU dot races, and APU-specific gates remain.

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
