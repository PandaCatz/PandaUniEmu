# Review of the Source Proposal

Reviewed 2026-07-13. Source:
`C:\Users\rezta\Desktop\Universal Retro Emulator  NES, SNES, Sega Genesis & GBA in Rust.md`

## Verdict

The proposal is a good orientation document and correctly chooses a modular
frontend plus isolated cores. It should not be used as an implementation spec
unchanged. Several statements are inaccurate, oversimplified, stale, or unsafe
as engineering policy. The project documents in this folder supersede those
parts.

## Corrections required before implementation

| Priority | Source line(s) | Problem | Corrected direction |
|---|---:|---|---|
| Critical | 428–436 | `while self.ppu.scanline < 262` can fail to terminate after the scanline counter wraps to zero, and instruction-at-a-time catch-up cannot express every DMA stall, bus edge, or interrupt race. | Stop on an explicit frame-complete event. Use exact scheduled events and advance at the granularity required by each observable hardware interaction. |
| Critical | 101–106, 428–436 | One example makes `Bus` own the PPU/APU, while another accesses `self.ppu`/`self.apu` beside `self.bus`; the ownership and mutable-borrow model is contradictory. | Define one machine owner for mutable console state. CPUs/devices receive narrow bus/device contexts during scheduled work; avoid `Rc<RefCell<_>>` as the default escape hatch. |
| Critical | 443–448 | The CPU/PPU ratio table treats unlike clocks as comparable and says GBA has four PPU cycles per CPU cycle. | Use console master/system cycles. GBA renders one LCD dot per four system cycles; it is not a PPU clock four times the CPU. SNES CPU accesses take variable master clocks. |
| Critical | 204, 466 | LoROM and HiROM are both described with `$00FFD5`; this confuses CPU addresses with file offsets and omits ExHiROM/copier headers. | Score candidate internal headers at file offsets near `$7FC0`, `$FFC0`, and `$40FFC0`, adjusted for a possible 512-byte copier header. The map-mode byte is candidate-header offset `+0x15`. Validate checksum, reset vector, sizes, and map consistency. |
| Critical | 30–38 | `step_frame() -> &[u32]` and `get_audio_samples() -> &[f32]` omit dimensions, formats, timestamps, ownership/back-pressure, interlace/variable modes, and sub-frame execution. | Run to an emulated deadline/event and emit typed video/audio packets with metadata through caller-owned buffers/sinks. |
| Critical | 118–125 | The mapper API cannot represent mutable reads, reset, mapper IRQs, PPU address/A12 observation, nametable routing, expansion audio, or persistent RAM. It therefore cannot implement the later MMC3 claim. | Design a cartridge device interface from mapper-specific behaviors and scheduler events; keep PRG/CHR mapping, IRQ, mirroring/nametable, persistence, and observation capabilities explicit. |
| High | 456 | Sleeping or locking simulation to VSync is offered as the pacing model. Host refresh commonly differs from console timing and sleep granularity is not a clock. | Keep simulation deterministic; use audio-buffer occupancy and bounded resampling feedback for real-time pacing, with VSync only for presentation. |
| High | 547 | Deriving `Serialize`/`Deserialize` and writing `bincode` is presented as durable state design and said to enable netplay. | Use an explicit, versioned, bounded state schema with identity and invariant checks. Netplay requires a separately proven deterministic protocol. |
| High | 545 | `get_unchecked` is recommended before profiling, and test-ROM success is treated as bounds proof. | Begin safe. Measure release builds; isolate and document `unsafe` only if it materially improves a demonstrated bottleneck, then fuzz and test its safety invariants. |
| High | 416 | The text names an mGBA `gba_bios` replacement and otherwise suggests stubbing SWIs. | Distinguish optional user-supplied 16 KiB firmware, a validated open implementation, and HLE BIOS services. mGBA has a built-in HLE BIOS implementation; blindly stubbing SWIs is not compatibility. |
| Medium | 181, 443–447 | SNES CPU speed is stated simply as 3.58 MHz and a `~4` PPU ratio. | 3.58 MHz is the fast 6-master-clock case; accesses can consume 6, 8, or 12 master clocks. Schedule in the master-clock domain. |
| Medium | 53 | The APU is said to be built into a “6502 die.” | Describe the Ricoh 2A03/2A07 as a 6502-derived CPU core integrated with the APU; it is not a stock MOS 6502 package. |
| Medium | 64–65 | Counts blur instructions and opcode encodings; compatibility claims for unofficial opcodes/test coverage are too broad. | Say 56 documented NMOS 6502 instructions represented by 151 documented opcode encodings, then implement undocumented encodings based on tests and title evidence. Name exactly which suite covers what. |
| Medium | 139 | A mapper set is claimed to cover about 80% without a reproducible dataset. | Track coverage from a versioned cartridge database or remove the percentage. Mapper priority can remain a practical hypothesis. |
| Medium | 215–230 | The prose says eight SNES background modes but the table shows only 0–3 and 7. | Either label it a partial table or document modes 0–7 and their hires/offset/color-math constraints. |
| Medium | 276 | `.B`, `.W`, and `.L` suffixes are said to apply to every 68000 instruction. | Many instructions accept a size; others have fixed or encoded forms. Implement from the programmer reference and test encodings rather than this rule. |
| Medium | 292 | “252 official Z80 instructions” is presented as a stable count. | Counts vary by whether mnemonics, prefixed encodings, aliases, and undocumented forms are counted. Define the supported opcode matrix and verify every encoding. |
| Medium | 352, 365, 372, 404 | GBA details overstate Thumb's low-register restriction, flatten EWRAM access to “2-cycle,” omit EEPROM/Flash/RTC save hardware, and imply one fixed timer per Direct Sound channel. | Model the documented high-register Thumb forms; bus width/sequential waitstates; SRAM/EEPROM/Flash/Flash1M/GPIO policy; and independent timer 0/1 selection for Direct Sound A and B. |
| Medium | 481 | `wgpu = "22"` is stale and the whole dependency block is unverified as a set. | Resolve a compatible set during scaffold. `wgpu` 30 exists as of this review; latest is not automatically best, so pin only after a build spike. |
| Medium | 521–534 | The order calls Genesis the most complex dual-CPU system and estimates NES at 2–4 months without defining compatibility. | Use NES → GBA → Genesis → SNES for increasing scheduler/device complexity, with milestone-specific gates. Treat estimates as ranges tied to declared compatibility tiers. |

## Important omissions

- Region variants and timing: NTSC/PAL/Dendy NES, PAL/NTSC Genesis and SNES,
  region selection, and clock phase/power-on behavior.
- Defensive parsing: truncation, checked arithmetic, maximum sizes, copier
  headers, ExHiROM, save-device detection, bad endianness, and unsupported chips.
- Cartridge hardware scope: SNES enhancement chips and GBA EEPROM/flash/RTC are
  large compatibility requirements, not minor loader details.
- Test provenance and redistribution rights. Many common test ROMs are not
  automatically safe to commit; keep user-supplied assets outside the repo.
- Power/reset state, open-bus behavior, DMA stalls, bus conflicts/arbitration,
  interrupt races, and exact device event ordering.
- Genesis 3/6-button handshake, TMSS/region behavior, and any SVP scope.
- `nestest` is one trace oracle, not proof of every official/unofficial opcode;
  Tonc demos are smoke fixtures rather than self-verifying PPU goldens.
- Durable state compatibility, atomic persistent saves, fuzzing, long-run drift
  tests, and release-profile performance evidence.
- Clear compatibility tiers. “Playable” and “full compatibility” require a
  versioned test matrix and observable criteria.

## Reference-quality issue

The proposal relies heavily on blogs, Reddit, YouTube, Hacker News, and project
summaries. Those are useful discovery aids but should not be normative hardware
specifications. Prefer platform manuals, NESdev/SNESdev, GBATEK, upstream source,
and independently generated trace/golden comparisons. Record the exact revision
used because community documentation changes.

Primary references used for the most important corrections:

- NES PPU timing: <https://www.nesdev.org/wiki/PPU_frame_timing>
- SNES timing: <https://snes.nesdev.org/wiki/Timing>
- SNES ROM header: <https://snes.nesdev.org/wiki/ROM_header>
- SNES ROM file formats: <https://snes.nesdev.org/wiki/ROM_file_formats>
- GBA LCD timing: <https://mgba-emu.github.io/gbatek/#lcd-dimensions-and-timings>
- mGBA upstream: <https://github.com/mgba-emu/mgba>
- Current `wgpu` crate documentation: <https://docs.rs/crate/wgpu/latest>
