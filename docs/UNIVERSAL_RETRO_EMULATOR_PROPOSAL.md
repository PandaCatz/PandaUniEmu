# Universal Retro Emulator: NES, SNES, Sega Genesis & GBA in Rust

## Overview

Building a universal emulator for the NES, SNES, Sega Genesis, and GBA is one of the most comprehensive systems-programming projects possible. Each console uses fundamentally different CPU architectures, video processors, audio subsystems, and cartridge formats. The only practical approach is a **modular architecture** where each system is an isolated emulation "core" sharing a common frontend for rendering, input, and audio output — similar to how the LibRetro/RetroArch ecosystem separates cores from the frontend.[^1]

This guide covers the full technical depth of each system and how to unite them in Rust.

***

## Architecture: The Universal Emulator Design

Before touching hardware specifics, the design must be right. A universal emulator is best structured as:

```
┌─────────────────────────────────────────────────┐
│                  FRONTEND LAYER                 │
│  (winit + wgpu/SDL2, audio crate, input map)   │
└───────────────────┬─────────────────────────────┘
                    │  Trait-based Core Interface
┌──────────────────┬┴──────────────────┬──────────┐
│   NES Core       │   SNES Core       │  Gen Core │
│   (6502+PPU)     │   (65816+PPU)     │  (68k+Z80)│
└──────────────────┴───────────────────┴──────────┘
```

Define a `Console` trait in Rust that every core implements:

```rust
pub trait Console {
    fn reset(&mut self);
    fn step_frame(&mut self) -> &[u32]; // returns RGBA framebuffer
    fn load_rom(&mut self, data: &[u8]) -> Result<(), EmulatorError>;
    fn set_input(&mut self, port: u8, state: u16);
    fn get_audio_samples(&mut self) -> &[f32];
    fn save_state(&self) -> Vec<u8>;
    fn load_state(&mut self, data: &[u8]) -> Result<(), EmulatorError>;
}
```

This allows the frontend to be completely system-agnostic. The Rust project `moa` (a Sega Genesis emulator written in Rust) demonstrates this pattern with separate crates for CPU, bus, and peripherals.[^2][^3]

***

## System 1: NES (Nintendo Entertainment System)

### Hardware Summary

| Component | Detail |
|-----------|--------|
| CPU | MOS Technology 6502 @ 1.79 MHz (NTSC) |
| PPU | Ricoh 2C02, 256×240 resolution |
| APU | Built into 6502 die, 5 channels |
| RAM | 2 KB main RAM, 256 bytes OAM |
| ROM Format | iNES (.nes), NES 2.0 |
| Clock Ratio | 3 PPU cycles per 1 CPU cycle (NTSC) |

### Step 1: The 6502 CPU

The 6502 is the most documented vintage CPU in existence. Implementation priority:[^4]

1. **Registers**: `A` (accumulator), `X`, `Y` (index), `SP` (stack pointer), `PC` (program counter), `P` (processor status flags: N, V, B, D, I, Z, C)
2. **Addressing Modes** (13 total): Immediate, Zero Page, Zero Page X/Y, Absolute, Absolute X/Y, Indirect, Indexed Indirect, Indirect Indexed, Implicit, Accumulator, Relative, Absolute Indirect[^5]
3. **Official Opcodes**: 151 official opcodes. Implement all of them.
4. **Unofficial/Illegal Opcodes**: ~105 additional. Needed for full compatibility with titles like Battletoads. Blargg's test ROMs test these.[^5]
5. **Cycle Accuracy**: Each instruction consumes a specific number of cycles. Page-crossing adds 1 cycle to certain instructions. This matters because the PPU state depends on the exact cycle count.[^6]

```rust
pub struct Cpu6502 {
    pub a: u8, pub x: u8, pub y: u8,
    pub sp: u8, pub pc: u16,
    pub status: u8, // NVBDIZC
    pub cycles: u64,
}

impl Cpu6502 {
    pub fn step(&mut self, bus: &mut Bus) -> u8 {
        let opcode = bus.read(self.pc);
        self.pc = self.pc.wrapping_add(1);
        self.execute(opcode, bus) // returns cycles consumed
    }
}
```

**Decoupling addressing modes from operations** is the key architectural win — compute the effective address separately from the operation so each addressing mode is reusable across many opcodes.[^7][^5]

**Testing**: Pass `nestest.nes` and all of Blargg's `instr_test-v5` ROMs before proceeding to the PPU.[^8][^9]

### Step 2: The NES Memory Bus

The NES uses memory-mapped I/O. The CPU's 16-bit address space maps as:

| Range | Device |
|-------|--------|
| `$0000–$07FF` | 2 KB RAM (mirrored to $1FFF) |
| `$2000–$2007` | PPU registers (mirrored to $3FFF) |
| `$4000–$4017` | APU and I/O registers |
| `$4020–$FFFF` | Cartridge space (PRG ROM/RAM via mapper) |

```rust
pub struct Bus {
    pub ram: [u8; 2048],
    pub ppu: Ppu,
    pub apu: Apu,
    pub cartridge: Box<dyn Mapper>,
}

impl Bus {
    pub fn read(&mut self, addr: u16) -> u8 { ... }
    pub fn write(&mut self, addr: u16, val: u8) { ... }
}
```

### Step 3: Mappers (Cartridge Hardware)

NES cartridges contain chips that extend the addressable ROM. These are called **mappers**. A `Mapper` trait lets you add new ones without breaking existing code:[^10]

```rust
pub trait Mapper {
    fn prg_read(&self, addr: u16) -> u8;
    fn prg_write(&mut self, addr: u16, val: u8);
    fn chr_read(&self, addr: u16) -> u8;
    fn chr_write(&mut self, addr: u16, val: u8);
    fn mirroring(&self) -> Mirroring;
}
```

Implement in this order to unlock the most games:[^11][^10]

| Mapper | ID | Games Unlocked |
|--------|----|----------------|
| NROM | 0 | Super Mario Bros 1, Donkey Kong, Pac-Man |
| MMC1 | 1 | Zelda, Metroid, Mega Man 2 |
| UxROM | 2 | Mega Man, Castlevania, Contra |
| CNROM | 3 | Arkanoid, Gradius |
| MMC3 | 4 | Super Mario Bros 3, Kirby, Mega Man 6 |
| AxROM | 7 | Battletoads, Wizards & Warriors |

Mappers 0, 1, 2, and 4 alone cover ~80% of the licensed NES library.

### Step 4: The PPU (Picture Processing Unit)

The PPU is the hardest NES component. It renders 256×240 pixels at 60fps (NTSC) by drawing **one dot per cycle**, running 341 cycles × 262 scanlines per frame = 89,342 PPU cycles per frame.[^12]

Key PPU concepts:
- **Pattern Tables**: Two 4KB regions in CHR ROM/RAM at `$0000` and `$1000`. Each 8×8 tile takes 16 bytes (2 bit-planes combined for 2-bit color index)[^13]
- **Nametables**: 2 KB of VRAM mapping which tile goes in each of the 32×30 screen positions
- **Attribute Table**: Sub-region of each nametable storing the upper 2 color bits for 2×2 tile groups (4-bit palette index per tile)
- **OAM**: 256 bytes for 64 sprites, each 4 bytes (Y, tile index, attributes, X)
- **Palettes**: 64-color master palette. Background uses 4 palettes of 4 colors; sprites use 4 palettes of 3 colors (+transparent)
- **VBlank**: Scanlines 241–260 are the vertical blank period. The PPU sets the VBlank flag, triggers NMI to the CPU (if enabled)[^13][^12]

PPU registers accessed by CPU at `$2000–$2007`:
- `PPUCTRL ($2000)`: NMI enable, sprite size, pattern table addresses
- `PPUMASK ($2001)`: Enable/disable rendering layers
- `PPUSTATUS ($2002)`: VBlank flag, sprite 0 hit, sprite overflow
- `OAMADDR ($2003)` / `OAMDATA ($2004)`: Sprite memory access
- `PPUSCROLL ($2005)`: Background scroll position (written twice)
- `PPUADDR ($2006)` / `PPUDATA ($2007)`: VRAM address + data

The internal scroll state uses the **Loopy register** scheme: `v` (current VRAM address), `t` (temporary address), `x` (fine X), `w` (write toggle). Correctly emulating this register is essential for scrolling games.[^12]

### Step 5: The APU

The NES APU has 5 channels:[^14][^15]
- **Pulse 1 & 2**: Square waves with 4 duty cycles (12.5%, 25%, 50%, 75%), sweep unit, envelope, length counter
- **Triangle**: Pure triangle wave, no volume control
- **Noise**: Pseudo-random noise via LFSR (15-bit feedback register)
- **DMC (Delta Modulation Channel)**: Streams 1-bit delta-encoded PCM samples

The APU mixes these channels with a non-linear mixing formula. Output samples are generated at ~44.1 KHz using a downsampling filter applied to the 1.79 MHz internal rate. Use the `cpal` crate for cross-platform audio output and fill a ring buffer.

***

## System 2: SNES (Super Nintendo Entertainment System)

### Hardware Summary

| Component | Detail |
|-----------|--------|
| Main CPU | WDC 65C816 @ 3.58 MHz (NTSC) |
| Audio CPU | Sony SPC700 @ 1.024 MHz |
| Audio DSP | S-DSP (8-channel ADPCM/BRR) |
| PPU | Custom chip, up to 512×448, 8 bg modes |
| RAM | 128 KB WRAM, 64 KB VRAM, 64 KB ARAM |
| ROM Format | SNES ROM (.sfc/.smc), LoROM/HiROM |

The SNES is significantly more complex than the NES, with two separate CPUs and seven distinct background rendering modes.[^16][^17]

### Step 1: The 65C816 CPU

The 65C816 is a 16-bit extension of the 6502:[^17][^16]

- **Backward compatible** with 6502 when `E` (emulation) flag = 1
- In native mode (`E=0`): A, X, Y can be 8 or 16-bit based on `M` and `X` flags in the status register
- New registers: `D` (direct page, replaces zero page), `DBR` (data bank), `PBR` (program bank), 24-bit address space
- New instructions: `MVN`/`MVP` (block move), `PEA`, `PEI`, `PER` (stack push variants), `STP`/`WAI` (stop/wait)
- 24-bit addressing: Full address = (Bank << 16) | 16-bit address. `PBR:PC` for code, `DBR:Addr` for data

The SNES main CPU communicates with the APU subsystem via four 8-bit I/O ports (`$2140–$2143`). The SNES CPU uploads programs and data to the SPC700's 64KB RAM through these ports.[^18]

### Step 2: The SNES Memory Map (LoROM vs HiROM)

SNES ROMs use either LoROM or HiROM addressing schemes, declared in the ROM header at `$00FFD5`:[^17]

| Region | LoROM | HiROM |
|--------|-------|-------|
| ROM | Banks `$00–$7D`, `$A0–$FF`, odd 32KB | Banks `$C0–$FF`, full 64KB |
| WRAM | `$7E–$7F` (128 KB) | `$7E–$7F` (128 KB) |
| VRAM | PPU-mapped, not CPU-accessible directly | same |
| Registers | `$2100–$21FF`, `$4000–$43FF` | same |

### Step 3: The SNES PPU

The SNES PPU supports 8 background modes with varying color depths:[^19]

| Mode | BG1 | BG2 | BG3 | BG4 | Notes |
|------|-----|-----|-----|-----|-------|
| 0 | 4-color | 4-color | 4-color | 4-color | 4 backgrounds |
| 1 | 16-color | 16-color | 4-color | — | Most common mode |
| 2 | 16-color | 16-color | — | — | Offset-per-tile |
| 3 | 256-color | 16-color | — | — | |
| 7 | 256-color | EXTBG | — | — | Rotation/scaling |

**Mode 7** is the famous pseudo-3D rotation mode used in Super Mario Kart, F-Zero, and Final Fantasy VI. It applies a 2D affine transform matrix to a single flat tile layer:

\[ \begin{pmatrix} x' \\ y' \end{pmatrix} = \begin{pmatrix} A & B \\ C & D \end{pmatrix} \begin{pmatrix} x - X_0 \\ y - Y_0 \end{pmatrix} + \begin{pmatrix} H \\ V \end{pmatrix} \]

where A, B, C, D are the rotation/scale matrix parameters (signed 8.8 fixed-point), and H, V are the screen center coordinates.[^20][^21]

**HDMA (H-Blank DMA)**: Each scanline can trigger a DMA transfer, allowing per-scanline register changes. This enables raster effects like the wave distortion in Chrono Trigger or the mode 7 perspective in F-Zero. Implementing HDMA is mandatory for many games.[^17]

**Sprites (OBJ layer)**: SNES sprites can be 8×8, 16×16, 32×32, or 64×64. The OBJ attribute table (OAM) holds data for up to 128 sprites. Only 32 sprites per scanline and 34 8×8 sprite "tiles" per scanline can display, with overflow handling hardware.[^17]

### Step 4: The SPC700 + S-DSP (Audio)

The SNES audio subsystem is a self-contained computer:[^22][^23]
- The **SPC700** CPU runs game audio code uploaded by the main CPU
- The **S-DSP** handles 8 ADPCM channels using **BRR (Bit Rate Reduction)** encoding — Nintendo's proprietary format for compressed audio samples
- 64 KB ARAM (Audio RAM) holds both the SPC700 program and BRR sample data

BRR decoding: each 9-byte block decodes to 16 signed 16-bit PCM samples. The first byte is a header with filter and range fields; the remaining 8 bytes pack 16 nibbles.

The DSP features per-voice:
- Pitch (16-bit, pitch tables for note conversion)
- Volume (left/right independently, -128 to +127)
- ADSR/Gain envelope
- FIR echo filter (shared across all voices, configurable 8-tap FIR)
- Gaussian interpolation for smooth pitch changes

Use the existing Rust `snes-apu` crate as a reference implementation, or implement from the SPC700 manual.[^24][^22]

***

## System 3: Sega Genesis / Mega Drive

### Hardware Summary

| Component | Detail |
|-----------|--------|
| Main CPU | Motorola 68000 @ 7.67 MHz |
| Sub CPU | Zilog Z80 @ 3.58 MHz |
| VDP | Custom chip (315-5313), 320×224 / 256×224 |
| FM Audio | Yamaha YM2612 (6 FM channels) |
| PSG Audio | TI SN76489 (3 square + 1 noise) |
| RAM | 64 KB main RAM, 8 KB Z80 RAM, 64 KB VRAM |
| ROM Format | `.md`, `.bin`, `.gen` (raw binary) |

The Genesis uses a dual-CPU design where the Z80 primarily handles audio tasks.[^25][^3]

### Step 1: The Motorola 68000 CPU

The 68000 is a true 16/32-bit processor with a clean, orthogonal instruction set:[^25]

- **Registers**: 8 data registers (`D0–D7`), 8 address registers (`A0–A7`, A7 = SP), `PC`, `SR` (status register with supervisor/user mode)
- **Instruction sizes**: `.B` (byte), `.W` (word, 16-bit), `.L` (long, 32-bit) suffixes on every instruction
- **Addressing modes**: 14 modes including register direct, address register indirect with post-increment/pre-decrement, absolute, PC-relative with offset/index
- **Privilege levels**: Supervisor mode (interrupt handling, I/O) and User mode
- **Exception vectors**: 256 vectors at address `$000000`. Reset vector at `$000004` (initial SSP at `$000000`)

The 68000 exception model handles interrupts at levels 1–7 via the VDP's interrupt line. The Genesis VDP fires a V-blank interrupt at level 6 and H-blank interrupt at level 4.[^26]

The `moa` Rust project is the most complete open-source Rust M68K emulator and already runs many Genesis games. Using Tom Harte's 68000 test suite for validation is essential.[^3]

### Step 2: The Zilog Z80

The Z80 handles audio commands and can access FM/PSG chip registers:[^25]
- 8 KB dedicated RAM at `$A00000–$A07FFF` (from 68K perspective)
- The 68000 can halt the Z80 and access its bus via `$A11100` (bus request) and `$A11200` (reset)
- Communication typically happens via shared Z80 RAM with a command protocol

Implement all 252 official Z80 instructions (including CB/DD/ED/FD prefixed sets). The Z80's I and R registers, block move/compare instructions, and undocumented flags (bit 3 and 5 of F register) must be correct for passing test suites.

### Step 3: The Genesis VDP

The VDP (315-5313) is the most complex Genesis component:[^27][^28]

- **Planes**: Two scrolling background planes (A and B), a Window plane, and a Sprite plane
- **Resolution**: 320×224 (40-cell mode) or 256×224 (32-cell mode); 224 or 240 active lines
- **Colors**: 9-bit color (3 bits each RGB), 64-color CRAM palette (4 palettes × 16 colors)
- **Tiles**: 8×8 pixels, 4 bits per pixel (16-color per tile from a chosen palette)
- **Sprites**: Up to 80 sprites (320-wide) or 64 sprites (256-wide). Max 20 sprites per line (320-wide)
- **DMA**: Three DMA modes: VRAM fill, VRAM-to-VRAM copy, and 68K memory to VRAM transfer
- **Scroll**: Plane A/B support horizontal scroll modes: whole-screen, per-cell, per-line. Vertical scroll is per-column or whole-screen
- **Shadow/Highlight**: Allows colors to be darkened or brightened per-pixel for lighting effects[^28]

VDP register access: The CPU writes a 2-word control port sequence to set the VRAM address and operation type (read/write VRAM/CRAM/VSRAM). Data reads/writes go through `$C00000` (data port).[^29]

### Step 4: Audio — YM2612 + SN76489

**YM2612 (FM synthesis)**:[^25]
- 6 FM channels, each with 4 operators (sine wave oscillators)
- Each operator has an ADSR envelope, frequency multiplier, detune, total level, key scaling, and feedback settings
- Operators are connected in one of 8 "algorithms" defining the FM modulation topology
- Channel 6 can optionally be switched to PCM (DAC) mode for digitized audio

**SN76489 (PSG)**:
- 3 square wave tone channels + 1 noise channel
- 10-bit frequency dividers for tone channels
- 4-bit volume per channel
- Noise: either periodic or "white" noise, with selectable frequency

Mix both at 44.1 KHz output. The YM2612 runs at 7.67 MHz internally; the sample rate is typically 7670454/144 ≈ 53.27 KHz before downsampling.

***

## System 4: Game Boy Advance (GBA)

### Hardware Summary

| Component | Detail |
|-----------|--------|
| CPU | ARM7TDMI @ 16.78 MHz |
| PPU | Custom, 240×160, 15-bit color |
| Audio | 4 legacy GB channels + 2 DMA PCM channels |
| RAM | 256 KB EWRAM, 32 KB IWRAM, 96 KB VRAM |
| ROM Format | `.gba` (raw binary), requires GBA BIOS |
| BIOS | 16 KB system ROM with SWI (software interrupt) functions |

### Step 1: The ARM7TDMI CPU

The ARM7TDMI is a 32-bit RISC processor with two instruction sets:[^30]

**ARM State (32-bit instructions)**:
- 16 registers: `R0–R12` (general purpose), `R13` (SP), `R14` (LR, link register), `R15` (PC)
- All instructions are 32-bit and conditionally executed (top 4 bits are condition code)
- 3-stage pipeline: fetch, decode, execute. PC always points 8 bytes ahead of executing instruction
- Modes: User, System, IRQ, FIQ, SVC, Abort, Undefined — each mode has banked registers

**Thumb State (16-bit instructions)**:
- Activated when `T` bit in CPSR is set; entered via `BX Rn` where bit 0 of Rn = 1[^30]
- Restricted instruction set: 8 general-purpose registers accessible, no conditional execution (except branches), many operations have limited immediate ranges
- PC points 4 bytes ahead in Thumb state
- Most GBA game code runs in Thumb mode for density in 16-bit bus ROM

**CPSR Flags**: N (negative), Z (zero), C (carry), V (overflow), I (IRQ disable), F (FIQ disable), T (Thumb), M[4:0] (mode).[^30]

**Memory timing**: ARM7TDMI access times vary by memory region. IWRAM is 32-bit wide (fastest), EWRAM is 16-bit (slower), cartridge ROM is 16-bit with configurable waitstates. Get these timings right — many games depend on them.[^31]

### Step 2: The GBA Memory Map

| Region | Address | Size | Notes |
|--------|---------|------|-------|
| BIOS ROM | `$00000000` | 16 KB | Readable only during BIOS execution |
| EWRAM | `$02000000` | 256 KB | External work RAM, 2-cycle |
| IWRAM | `$03000000` | 32 KB | Internal work RAM, 1-cycle |
| I/O Registers | `$04000000` | 1 KB | Hardware control |
| Palette RAM | `$05000000` | 1 KB | BG + OBJ palettes |
| VRAM | `$06000000` | 96 KB | Tile data + frame buffers |
| OAM | `$07000000` | 1 KB | Object (sprite) attributes |
| Game Pak ROM | `$08000000` | Up to 32 MB | ROM (mirrored at $0A/0C) |
| Game Pak SRAM | `$0E000000` | Up to 64 KB | Save data |

### Step 3: The GBA PPU

The GBA PPU supports 6 display modes:[^31]

| Mode | Type | BG0 | BG1 | BG2 | BG3 |
|------|------|-----|-----|-----|-----|
| 0 | Tile | Reg | Reg | Reg | Reg |
| 1 | Tile | Reg | Reg | Aff | — |
| 2 | Tile | — | — | Aff | Aff |
| 3 | Bitmap | — | — | 240×160 16bpp | — |
| 4 | Bitmap | — | — | 240×160 8bpp×2 | — |
| 5 | Bitmap | — | — | 160×128 16bpp×2 | — |

- **Regular backgrounds** use tile-based rendering similar to SNES Mode 1
- **Affine backgrounds** support rotation and scaling via a 2×2 matrix (same concept as SNES Mode 7)
- **Bitmap modes** (3, 4, 5) write pixel data directly to VRAM — widely used in homebrew and 3D games
- **Sprites (OBJ)**: 128 sprites with sizes from 8×8 to 64×64, affine transform support, semi-transparent blending

**Key registers** (I/O at `$04000000`):
- `DISPCNT ($000)`: Display control, BG mode selection, enable layers
- `DISPSTAT ($004)`: V-blank/H-blank status and interrupt enables
- `VCOUNT ($006)`: Current scanline counter
- `BG0CNT–BG3CNT ($008–$00E)`: Background control (tile/map base, size, priority)
- `WIN0H/WIN1H ($040–$044)`: Window horizontal bounds
- `BLDCNT ($050)`: Alpha blending / brightness control register

### Step 4: DMA, Timers, and Interrupts

**DMA**: 4 DMA channels, each with source, destination, word count, and control registers. DMA channel 3 is the most general; channels 1 and 2 can be triggered by audio FIFO empty events for streaming PCM audio.[^32]

**Timers**: 4 cascading timers (TM0–TM3). Each is a 16-bit counter with prescalers of 1, 64, 256, or 1024 CPU cycles. Timers 0 and 1 drive the two PCM audio FIFO channels (one timer per channel).

**Interrupt Controller**: The `IE` register (`$04000200`) enables interrupts, `IF` (`$04000202`) flags pending ones. Writing 1 to an `IF` bit acknowledges the interrupt. The BIOS vectors all interrupts through `$03007FFC` (an IWRAM pointer to the user IRQ handler).[^31]

### Step 5: GBA Audio

GBA audio mixes legacy channels with modern PCM:
- **4 legacy channels**: Square 1 (with sweep), Square 2, Wave (4-bit PCM, 32-byte wavetable), Noise — identical to Game Boy
- **2 Direct Sound (PCM) channels**: FIFO-based DMA streaming. Each FIFO holds 32 bytes; DMA refills when half-empty. Timer 0 or 1 drives the sample rate. The CPU writes signed 8-bit samples to `FIFO_A ($040000A0)` or `FIFO_B ($040000A4)`

The sound hardware is often considered the GBA's weakest area — game developers frequently drove DMA at 18,157 Hz or 36,314 Hz for the PCM channels.

**BIOS Requirement**: The GBA BIOS (`$00000000–$00003FFF`) handles boot, SWI (software interrupt) calls for math routines, decompression, and audio playback. You need a legally obtained BIOS dump or the open-source replacement `gba_bios` (the mGBA team's open-source replacement). Without it, you must stub out SWI calls.[^31]

***

## Timing and Synchronization

This is the most subtle and critical aspect of emulation. Every component runs on the same master clock and must stay synchronized.[^33][^34][^6]

### The Catch-Up Method (Recommended for Rust)

```rust
// NES example: 3 PPU cycles per CPU cycle
fn run_frame(&mut self) {
    while self.ppu.scanline < 262 {
        let cpu_cycles = self.cpu.step(&mut self.bus);
        // Catch PPU up to CPU
        for _ in 0..(cpu_cycles * 3) {
            self.ppu.tick(&mut self.bus);
        }
        // Catch APU up
        self.apu.tick_n(cpu_cycles);
    }
}
```

The key insight: run the CPU one instruction, then run every other component forward by the proportional number of cycles. The ratios per system:[^35][^34]

| System | CPU Cycles | PPU Cycles | Notes |
|--------|------------|------------|-------|
| NES (NTSC) | 1 | 3 | APU same as CPU |
| SNES (NTSC) | 1 | ~4 | Variable, depends on instruction |
| Genesis (NTSC) | 1 | ~10 | Based on master clock 53.69 MHz |
| GBA | 1 | 4 | Based on 16.78 MHz CPU, 4 PPU cycles |

For frame-level timing, compute cycles per frame from clock speed and refresh rate:[^34]
- **NES**: 1,789,773 CPU cycles/sec ÷ 60.098 fps = ~29,780 cycles/frame
- **SNES**: 3,579,545 ÷ 60.098 = ~59,561 cycles/frame
- **Genesis**: 7,670,454 ÷ 59.924 (NTSC) = ~128,013 cycles/frame
- **GBA**: 16,777,216 ÷ 59.727 = ~280,896 cycles/frame

Use `std::thread::sleep` + monotonic timestamps (`std::time::Instant`) or lock to vsync via `winit`'s event loop to pace the emulator to real time.

***

## ROM Formats and Loading

| System | Format | Header |
|--------|--------|--------|
| NES | iNES (`.nes`) | 16-byte: magic `NES\x1A`, PRG/CHR size, mapper number, flags[^36] |
| NES | NES 2.0 (`.nes`) | Extended iNES with submapper, battery, extra RAM info |
| SNES | Raw (`.sfc`, `.smc`) | ROM header at `$00FFD5` (LoROM) or `$00FFD5` (HiROM) |
| Sega Genesis | Raw (`.md`, `.bin`, `.gen`) | "SEGA" magic at offset `$100` |
| GBA | Raw (`.gba`) | Nintendo logo at `$004`, game title at `$0A0`, checksum at `$0BD` |

For NES, parse the iNES header to get PRG ROM size (16 KB units), CHR ROM size (8 KB units), mapper number (bits 4–7 of byte 6 + bits 4–7 of byte 7), and mirroring type.[^36][^37]

***

## Frontend: Rendering, Input, Audio

### Recommended Rust Stack

```toml
[dependencies]
winit = "0.30"         # Cross-platform window and event loop
wgpu = "22"            # GPU rendering (WebGPU backend)
cpal = "0.15"          # Cross-platform audio output
gilrs = "0.10"         # Gamepad input
```

The display pipeline: each console's PPU writes into an `[u32; WIDTH * HEIGHT]` RGBA framebuffer. Upload this as a texture to wgpu each frame and render a fullscreen quad.[^38][^39]

For audio: use `cpal` with a ring buffer. The emulator fills the ring buffer during `step_frame`, and a separate audio callback thread drains it at the hardware sample rate. Use `rubato` crate for sample rate conversion if the emulated sample rate doesn't match the host's.

### Input Mapping

```rust
pub struct InputState {
    pub buttons: u16, // bitfield per controller
}
// NES: A, B, Select, Start, Up, Down, Left, Right
// SNES: B, Y, Select, Start, Up, Down, Left, Right, A, X, L, R
// Genesis: A, B, C, Start, Up, Down, Left, Right, X, Y, Z, Mode
// GBA: A, B, Select, Start, Up, Down, Left, Right, L, R
```

***

## Test ROMs and Validation

Never claim an emulator component is working without running the appropriate test ROMs:[^8]

| System | Test Suite | Tests |
|--------|------------|-------|
| NES CPU | `nestest.nes` | All official + illegal opcodes with expected log |
| NES | Blargg's `instr_test-v5` | Timing and behavior per instruction |
| NES PPU | Blargg's `ppu_vbl_nmi` | VBlank timing and NMI behavior |
| NES APU | Blargg's `apu_mixer` | Channel output accuracy |
| SNES | Blargg's `blargg_apu_test` | SPC700 accuracy |
| Genesis | `68000-opcodes` test suites | M68K instruction accuracy |
| GBA | `armwrestler`, `gba-tests` | ARM7TDMI instruction set |
| GBA | Tonc demos | PPU mode accuracy |

***

## Development Order (Recommended)

Follow this sequence to maintain momentum and catch errors early:

1. **NES First**: Smallest and best-documented. Establish your core patterns here.
   - 6502 CPU → Bus → Mapper 0 → PPU backgrounds → PPU sprites → APU → More mappers
2. **GBA Second**: ARM7TDMI is very well-documented with modern tools.
   - ARM + Thumb CPU → Memory map → PPU modes 0/3 → DMA + Timers → Audio
3. **SNES Third**: Similar concepts to NES but much more complex.
   - 65C816 CPU → Memory map → PPU Mode 1 → SPC700 APU → Mode 7 → HDMA
4. **Genesis Last**: Most complex dual-CPU system.
   - 68000 CPU → VDP basics → Z80 → YM2612 FM → Full VDP → DMA

***

## Rust-Specific Considerations

**Trait objects for mappers and cores**: Use `Box<dyn Mapper>` and `Box<dyn Console>` to allow dynamic dispatch at runtime when loading different ROM types.

**Borrow checker with shared bus**: The classic emulator challenge — CPU, PPU, and APU all need mutable access to the bus. Solutions:
- Pass the bus as `&mut Bus` through each tick function (most Rust-idiomatic)
- Use `Rc<RefCell<Bus>>` for interior mutability (simpler architecture, slight overhead)
- Split the bus into separate sub-structs and pass only what's needed

**unsafe for performance hotpaths**: PPU pixel rendering running at millions of pixels per frame can benefit from `unsafe` slice indexing (`get_unchecked`) after bounds checking is validated by test ROMs.

**State serialization**: Use `serde` with `bincode` for save states. Derive `Serialize`/`Deserialize` on all CPU, PPU, and APU state structs. This also enables netplay and rewind features.

**Existing Rust References**:
- `moa`: Rust Sega Genesis emulator (68K + Z80)[^3]
- `emu-rs/snes-apu`: Rust SNES SPC700 APU emulator[^22]
- `dustinbowers/nes-emulator`: Rust NES emulator[^5]
- `RustRetro/RustRetro`: Plugin-based multi-system Rust emulator framework[^2]

***

## Key Reference Documents

| System | Document |
|--------|----------|
| NES | NESdev Wiki (nesdev.org) — the definitive NES technical reference |
| NES | Blargg test ROM documentation |
| SNES | SNESdev Wiki (snes.nesdev.org)[^17] |
| SNES | SPC700 APU Manual[^24] |
| Genesis | Sega Technical Overview v1.00 (1991)[^26] |
| Genesis | genvdp.txt (VDP register reference)[^40] |
| GBA | GBATEK (Martin Korth) — the GBA hardware bible |
| GBA | gbadoc (mgba-emu.github.io)[^31] |
| GBA | Tonc GBA Programming tutorial |
| ARM | ARM7TDMI Technical Reference Manual (ARM DDI 0029E) |

***

## Conclusion

A universal NES/SNES/Genesis/GBA emulator is a multi-year project at the level of a production Rust application. The recommended path is to achieve playable NES emulation first (2–4 months of dedicated work), establish the frontend and trait architecture, then add each subsequent system as a new core. Accuracy should be validated continuously with test ROMs at every step — getting the fundamentals precisely right before adding the next component is the single most important discipline in emulator development. The Rust type system and memory model are strong allies here, eliminating entire classes of bugs that plague C/C++ emulators, and the growing ecosystem of Rust emulation crates provides solid reference material.[^4][^5]

---

## References

1. [RetroArch](https://wiki.batocera.org/emulators:retroarch) - [Ubiquitous with retro-gaming.] RetroArch RetroArch (formerly SSNES), is a ubiquitous frontend that ...

2. [GitHub - RustRetro/RustRetro: A WIP multisystem emulator](https://github.com/RustRetro/RustRetro) - A WIP multisystem emulator. Contribute to RustRetro/RustRetro development by creating an account on ...

3. [transistorfet/moa: An emulator for various m68k ...](https://github.com/transistorfet/moa) - An emulator for various m68k and z80 based computers, written in Rust. Currently it has support for ...

4. [Writing your own NES emulator Part 1 - overview](https://yizhang82.dev/nes-emu-overview) - 1. Start from CPU first. And make sure it's really solid. · 2. Add NES rom support (and mapper 0) · ...

5. [NES Console Emulator - Dustin Bowers](https://dustinbowers.com/projects/nes-emulator) - A Rust-based endeavor to emulate the Nintendo Entertainment System.

6. [CPU timing and better instruction implementation?](https://www.reddit.com/r/EmuDev/comments/a7kr9h/cpu_timing_and_better_instruction_implementation/) - CPU timing and better instruction implementation?

7. [Writing an NES emulator: Part 1 - The 6502 CPU](https://analog-hors.github.io/site/pones-p1/) - After an immense amount of time of not sticking with a project, I finally decided to pick up writing...

8. [GitHub - christopherpow/nes-test-roms: Collection of test ROMs for testing a NES emulator.](https://github.com/christopherpow/nes-test-roms/) - Collection of test ROMs for testing a NES emulator. - christopherpow/nes-test-roms

9. [How to use blargg's instr_test-v5](https://forums.nesdev.org/viewtopic.php?t=17733)

10. [NES emulation journal: Implementing mappers](https://snoozetime.github.io/2019/01/07/nes-emu-journal4.html) - The easiest mapper to implement after NROM is Uxrom. It provide bank-switching capabilities. The PPU...

11. [about mappers – mmc1 and mmc3 - emudev](https://emudev.de/nes-emulator/about-mappers-mmc1-and-mmc3/) - The MMC1 mapper is a mapper, that offers banking for PRG and CHR ROM. more space on the cartridge. I...

12. [Releasing NES emulator source](https://medium.com/@BadFoolPrototype/releasing-nes-emulator-source-b41bf2a8e376) - Hello everyone!

13. [Nintendo Entertainment System Hardware Emulation](https://fpga.mit.edu/videos/2019/team18/report.pdf) - A full block diagram of our implementation of the PPU can be found in ​Diagram 1​. Communications be...

14. [NES Emulator Part #6: APU - Sounds, Beeps & Bloops](https://www.youtube.com/watch?v=72dI7dB3ZvQ) - In this video I look at how to start adding audio to the NES emulation. But before I can, I show an ...

15. [charming sound – the apu](https://emudev.de/nes-emulator/charming-sound-the-apu/)

16. [Super NES Programming - Wikibooks, open books for an open world](https://en.wikibooks.org/wiki/Super_NES_Programming)

17. [SNESdev Wiki](https://snes.nesdev.org/wiki/SNESdev_Wiki) - SNES Development Wiki

18. [Differences of SPCPlayer and SPC7000](https://forums.nesdev.org/viewtopic.php?t=20043)

19. [backgrounds, modes and tests](https://emudev.de/q00-snes/backgrounds-modes-and-tests/)

20. [Proof of concept: SNES Mode 7 at HD/4K resolutions, help ...](https://www.reddit.com/r/emulation/comments/b2m7db/proof_of_concept_snes_mode_7_at_hd4k_resolutions/) - The mode 7 transformations (incl. HDMA) are performed at the output resolution. Thats is 25 (5x5) or...

21. [BSNES (emulator) mod allows for HD rendering of Mode7 games](https://www.resetera.com/threads/bsnes-emulator-mod-allows-for-hd-rendering-of-mode7-games.111715/) - DerKoun on Reddit has put together a mod for byuu's bsnes emulator that lets it render Mode7 games a...

22. [GitHub - emu-rs/snes-apu: A Super Nintendo audio unit emulator.](https://github.com/emu-rs/snes-apu) - A Super Nintendo audio unit emulator. Contribute to emu-rs/snes-apu development by creating an accou...

23. [GitHub - Herringway/spc700: D port of blargg's snes_spc library](https://github.com/Herringway/spc700) - D port of blargg's snes_spc library. Contribute to Herringway/spc700 development by creating an acco...

24. [spc700_apu_manual.txt](http://snesmusic.org/files/spc700_apu_manual.txt)

25. [SEGA Genesis: Building a ROM - nameless algorithm](https://namelessalgorithm.com/genesis/blog/genesis/) - SEGA Genesis programming

26. [Full text of "Sega Genesis Manual: Genesis Technical Overview v1.00 (1991)(Sega)(US)"](https://archive.org/stream/Genesis_Technical_Overview_v1.00_1991_Sega_US/Genesis_Technical_Overview_v1.00_1991_Sega_US_djvu.txt)

27. [Genesis-Plus-GX/core/vdp_ctrl.c at master · ekeeke/Genesis-Plus-GX](https://github.com/ekeeke/Genesis-Plus-GX/blob/master/core/vdp_ctrl.c) - An enhanced port of Genesis Plus - accurate & portable Sega 8/16 bit emulator - ekeeke/Genesis-Plus-...

28. [Sega Genesis/Mega Drive VDP Graphics Guide v1.2a (03/14/17)](https://megacatstudios.com/ko/blogs/retro-development/sega-genesis-mega-drive-vdp-graphics-guide-v1-2a-03-14-17) - Mega Cat Studios OVERVIEW OF VDP CONCEPTS VDP

29. [VDP Registers - Nameless Algorithm: SEGA Genesis](https://namelessalgorithm.com/genesis/blog/vdp/) - SEGA Genesis programming

30. [gbadoc](https://gbadev.net/gbadoc/cpu.html) - Documents the workings of the Game Boy Advance hardware

31. [Draw mode](https://mgba-emu.github.io/gbdoc/) - Open Game Boy Documentation Project

32. [GitHub - mara-kr/GBA: GameBoy Advance Zedboard Implementation](https://github.com/mara-kr/GBA) - GameBoy Advance Zedboard Implementation. Contribute to mara-kr/GBA development by creating an accoun...

33. [How to handle proper timing in an emulator.](https://www.reddit.com/r/EmuDev/comments/7kqsy6/how_to_handle_proper_timing_in_an_emulator/)

34. [How do you emulate specific cpu speeds?](https://www.reddit.com/r/EmuDev/comments/4o2t6k/how_do_you_emulate_specific_cpu_speeds/)

35. [CPU, PPU timing -> 'catch up' method](https://www.reddit.com/r/EmuDev/comments/10m9had/cpu_ppu_timing_catch_up_method/)

36. [iNES - NESdev Wiki](https://www.nesdev.org/wiki/INES) - The .NES file format (file name suffix .nes) is the de facto standard for distribution of NES binary...

37. [Emulating a NES Cartridge](https://www.youtube.com/watch?v=qS95e1TnqUs) - Now that we've seen what cartridge hardware looks like, we'll see what it takes to emulate a basic c...

38. [I agree wgpu is great, but wgpu isn't a substitute for SDL. ...](https://news.ycombinator.com/item?id=29207360) - The Rust project that replaces SDL is called winit. One can use wgpu with winit if they prefer, wgpu...

39. [GitHub - What42Pizza/Wgpu-Template: My personal template for making Rust games with Winit + Wgpu](https://github.com/What42Pizza/Wgpu-Template) - My personal template for making Rust games with Winit + Wgpu - What42Pizza/Wgpu-Template

40. [genvdp.txt](http://jiggawatt.org/genvdp.txt)

