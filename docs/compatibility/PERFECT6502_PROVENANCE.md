# perfect6502 Oracle Provenance

Last reviewed: 2026-07-14

## Purpose and claim boundary

`perfect6502` is the independently executed oracle for hardware IRQ/NMI/reset
bus order, sub-instruction interrupt polling, and NMI hijacking. The project
publishes only its own verifier and harness; upstream files and the generated
oracle executable remain in the operator's local temp cache. The finished
emulator does not download, compile, link, or run this oracle.

## Pinned source

- Repository: <https://github.com/mist64/perfect6502>
- Commit: `09fc542877a84318291aa42dab143a3e2c3db974`
- Archive: <https://codeload.github.com/mist64/perfect6502/zip/09fc542877a84318291aa42dab143a3e2c3db974>
- Archive SHA-256:
  `594553a873d66a13e88c134495c9f55e064a36ba4670b07fba71f5047a77bdf5`
- Repository-level license and simulator C files: MIT.
- Required `netlist_6502.h`: file-specific CC BY-NC-SA 3.0 notice, copyright
  Greg James, Brian Silverman, and Barry Silverman; it requires attribution to
  Greg James and `www.visual6502.org`, noncommercial use, and ShareAlike.

The archive hash, root license, simulator source headers, and netlist header
were independently downloaded and checked on Windows x86-64 on 2026-07-14. The
file-specific netlist notice takes precedence over the incomplete MIT-only
description recorded during initial intake. The repository stores metadata and
a project-owned external-source harness only, not upstream source, binaries, or
netlist data.

The verifier additionally pins these required file SHA-256 values:

| File | SHA-256 |
|---|---|
| `perfect6502.c` | `cac56dab1d6a08852361870191d9d5f633450939c14b7e5505e26da78146bbbf` |
| `perfect6502.h` | `15ab13035b71d5008bd14d993b34656df088d760d18308a7fb64d7b28c53d340` |
| `netlist_sim.c` | `19d1e30504fb13c27d79f8c8f01df5d080b30b621a60a548b1b84c614d7caed2` |
| `netlist_sim.h` | `fe483a7f43f973dfc388b02410a711ceb5b492ab08970429cc16a7cd0caf70bb` |
| `types.h` | `484747d5c63f0b4c1c8ed897ea52606bd7521b08f78a469582603e85a678f3bc` |
| `netlist_6502.h` | `7a5a28f64a0d464d18faecd3d715d96549bc5da8f05e6f468ef3dae97ef0f340` |
| `LICENSE` | `29f44f6af3005961e76e712a8b0f36faf4d8c3d8e2592ca191876154adff2179` |

## Integration rules

1. Keep all upstream simulator and netlist files outside this repository. The
   acquisition script must verify the complete pinned archive hash before use.
   Do not redistribute `netlist_6502.h` under the project's GPL license.
2. Keep the oracle out of production crates. A project-owned test/tool harness
   may compile against the external checkout and emit short factual bus
   measurements for specifically documented line/bus cases.
3. Pin inputs, initial state, line transitions, observed address/data/RW values,
   and the exact phase convention. Do not infer missing cycles by eye.
4. Compare the live Rust CPU trace numerically against the oracle for IRQ, NMI,
   reset, second-to-last-cycle polling, and BRK/IRQ NMI-hijack cases.
5. Record disagreements as open behavior until the phase convention and oracle
   result are independently reproduced. Never weaken an expected trace merely
   to make the implementation pass.

## Reproduction and measured cases

On Windows with Visual Studio C++ Build Tools installed, explicitly review and
accept the external netlist's noncommercial license before running:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tools/verify-perfect6502.ps1 -Acquire -AcceptNonCommercialLicense
```

`-Acquire` permits a download only when the pinned archive is absent. The script
verifies the archive before extraction, verifies every required source file,
builds in `%TEMP%`, and rejects output that does not match the curated traces.
Without `-AcceptNonCommercialLicense`, it stops before acquisition or use.

The harness applies line transitions during the clock-low half-cycle, services
memory during the following high half-cycle, and records one row after each
complete low/high cycle. The verified cases are:

- exact seven-access IRQ and NMI entry: two current-PC reads, three stack
  writes, and two vector reads;
- exact seven-read warm reset: two current-PC reads, three stack reads, and two
  reset-vector reads;
- an IRQ stable before the penultimate-cycle sample is accepted, while the same
  transition during the final cycle waits for the following instruction;
- the not-taken, taken same-page, and taken page-cross branch poll positions;
- BRK and IRQ selection of the NMI vector through the low-PC push, with the
  original IRQ vector retained when NMI arrives after vector lock.

The opt-in acquisition path and all curated comparisons were executed from a
fresh cache on Windows x86-64 on 2026-07-14. The corresponding Rust tests use
the measured bus sequences as short factual expected values; they contain no
upstream source or netlist data.

Any future redistribution of upstream material must preserve both the MIT
notices and the netlist's CC BY-NC-SA 3.0 notice/attribution and must be reviewed
for GPL compatibility and the noncommercial restriction first.
