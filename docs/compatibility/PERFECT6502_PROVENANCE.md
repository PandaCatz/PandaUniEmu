# perfect6502 Oracle Provenance

Last reviewed: 2026-07-14

## Purpose and claim boundary

`perfect6502` is the planned independent oracle for hardware IRQ/NMI/reset bus
order, sub-instruction interrupt polling, and NMI hijacking. It is not yet
integrated, and no current compatibility or cycle-accuracy claim relies on it.
The existing 190-vector oracle remains authoritative only for its sampled
instruction executions.

## Pinned source

- Repository: <https://github.com/mist64/perfect6502>
- Commit: `09fc542877a84318291aa42dab143a3e2c3db974`
- Archive: <https://codeload.github.com/mist64/perfect6502/zip/09fc542877a84318291aa42dab143a3e2c3db974>
- Archive SHA-256:
  `594553a873d66a13e88c134495c9f55e064a36ba4670b07fba71f5047a77bdf5`
- License at the pinned revision: MIT; copyright and permission text must be
  preserved if source is imported.

The archive hash and license header were independently downloaded and checked
on Windows x86-64 on 2026-07-14. The repository currently stores metadata only,
not upstream source, binaries, traces, or generated results.

## Integration rules

1. Import only the minimum simulator/netlist files required by a reproducible
   harness, preserving the upstream MIT notice and recording every imported
   file hash in `NOTICE`.
2. Keep the oracle out of production crates. A test/tool harness may emit
   project-owned data-only results for specifically documented line/bus cases.
3. Pin inputs, initial state, line transitions, observed address/data/RW values,
   and the exact phase convention. Do not infer missing cycles by eye.
4. Compare the live Rust CPU trace numerically against the oracle for IRQ, NMI,
   reset, second-to-last-cycle polling, and BRK/IRQ NMI-hijack cases.
5. Record disagreements as open behavior until the phase convention and oracle
   result are independently reproduced. Never weaken an expected trace merely
   to make the implementation pass.
