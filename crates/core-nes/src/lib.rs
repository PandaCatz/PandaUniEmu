#![forbid(unsafe_code)]

mod nrom_bus;
mod ppu_timing;

pub use nrom_bus::{CpuBusFault, NromCpuBus};
pub use ppu_timing::{
    DOTS_PER_SCANLINE, MASTER_TICKS_PER_CPU_CYCLE, MASTER_TICKS_PER_PPU_DOT,
    NTSC_MASTER_CLOCK_DENOMINATOR, NTSC_MASTER_CLOCK_NUMERATOR_HZ, NtscScheduler, PpuEvent,
    PpuPosition, PpuTiming, SCANLINES_PER_FRAME, TimedPpuEvent, TimingError, VISIBLE_SCANLINES,
};

use format_ines::{Cartridge, Mirroring, RamSizes, Region};
use std::error::Error;
use std::fmt::{Display, Formatter};

const NROM_128_PRG: usize = 16 * 1024;
const NROM_256_PRG: usize = 32 * 1024;
const NROM_CHR: usize = 8 * 1024;
const NROM_PRG_RAM: usize = 8 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NesCartridge {
    mapper: u16,
    mirroring: Mirroring,
    region: Region,
    battery: bool,
    ram: RamSizes,
    trainer: Option<Vec<u8>>,
    prg_rom: Vec<u8>,
    chr_rom: Vec<u8>,
}

impl NesCartridge {
    pub fn from_parsed(parsed: Cartridge<'_>) -> Result<Self, CartridgeError> {
        if parsed.mapper() != 0 {
            return Err(CartridgeError::UnsupportedMapper(parsed.mapper()));
        }
        if parsed.submapper() != 0 {
            return Err(CartridgeError::UnsupportedSubmapper(parsed.submapper()));
        }
        if !matches!(parsed.prg_rom().len(), NROM_128_PRG | NROM_256_PRG) {
            return Err(CartridgeError::InvalidNromPrgSize(parsed.prg_rom().len()));
        }
        let ram = parsed.ram_sizes();
        if !matches!(parsed.chr_rom().len(), 0 | NROM_CHR) {
            return Err(CartridgeError::InvalidNromChrSize(parsed.chr_rom().len()));
        }
        let chr_memory =
            ram.chr_ram
                .checked_add(ram.chr_nvram)
                .ok_or(CartridgeError::InvalidNromChrMemory {
                    rom: parsed.chr_rom().len(),
                    volatile: ram.chr_ram,
                    nonvolatile: ram.chr_nvram,
                })?;
        if !matches!(
            (parsed.chr_rom().len(), chr_memory),
            (NROM_CHR, 0) | (0, NROM_CHR)
        ) {
            return Err(CartridgeError::InvalidNromChrMemory {
                rom: parsed.chr_rom().len(),
                volatile: ram.chr_ram,
                nonvolatile: ram.chr_nvram,
            });
        }
        let prg_memory =
            ram.prg_ram
                .checked_add(ram.prg_nvram)
                .ok_or(CartridgeError::InvalidNromPrgMemory {
                    volatile: ram.prg_ram,
                    nonvolatile: ram.prg_nvram,
                })?;
        if !matches!(prg_memory, 0 | NROM_PRG_RAM) {
            return Err(CartridgeError::InvalidNromPrgMemory {
                volatile: ram.prg_ram,
                nonvolatile: ram.prg_nvram,
            });
        }
        if parsed.trainer().is_some() && prg_memory == 0 {
            return Err(CartridgeError::TrainerWithoutPrgMemory);
        }

        Ok(Self {
            mapper: parsed.mapper(),
            mirroring: parsed.mirroring(),
            region: parsed.region(),
            battery: parsed.has_battery(),
            ram,
            trainer: parsed.trainer().map(<[u8]>::to_vec),
            prg_rom: parsed.prg_rom().to_vec(),
            chr_rom: parsed.chr_rom().to_vec(),
        })
    }

    #[must_use]
    pub const fn mapper(&self) -> u16 {
        self.mapper
    }

    #[must_use]
    pub const fn mirroring(&self) -> Mirroring {
        self.mirroring
    }

    #[must_use]
    pub const fn region(&self) -> Region {
        self.region
    }

    #[must_use]
    pub const fn has_battery(&self) -> bool {
        self.battery
    }

    #[must_use]
    pub const fn ram_sizes(&self) -> RamSizes {
        self.ram
    }

    #[must_use]
    pub fn trainer(&self) -> Option<&[u8]> {
        self.trainer.as_deref()
    }

    #[must_use]
    pub fn prg_rom(&self) -> &[u8] {
        &self.prg_rom
    }

    #[must_use]
    pub fn chr_rom(&self) -> &[u8] {
        &self.chr_rom
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CartridgeError {
    UnsupportedMapper(u16),
    UnsupportedSubmapper(u8),
    InvalidNromPrgSize(usize),
    InvalidNromChrSize(usize),
    InvalidNromChrMemory {
        rom: usize,
        volatile: usize,
        nonvolatile: usize,
    },
    InvalidNromPrgMemory {
        volatile: usize,
        nonvolatile: usize,
    },
    TrainerWithoutPrgMemory,
}

impl Display for CartridgeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedMapper(mapper) => {
                write!(formatter, "NES mapper {mapper} is not implemented")
            }
            Self::UnsupportedSubmapper(submapper) => {
                write!(
                    formatter,
                    "NES mapper 0 submapper {submapper} is not implemented"
                )
            }
            Self::InvalidNromPrgSize(size) => write!(
                formatter,
                "NROM PRG ROM must be 16 KiB or 32 KiB, got {size} bytes"
            ),
            Self::InvalidNromChrSize(size) => write!(
                formatter,
                "NROM CHR ROM must be absent or 8 KiB, got {size} bytes"
            ),
            Self::InvalidNromChrMemory {
                rom,
                volatile,
                nonvolatile,
            } => write!(
                formatter,
                "NROM requires either 8 KiB CHR ROM or 8 KiB CHR memory, got {rom} ROM, {volatile} volatile, and {nonvolatile} nonvolatile bytes"
            ),
            Self::InvalidNromPrgMemory {
                volatile,
                nonvolatile,
            } => write!(
                formatter,
                "NROM PRG memory must total 0 or 8 KiB, got {volatile} volatile and {nonvolatile} nonvolatile bytes"
            ),
            Self::TrainerWithoutPrgMemory => {
                formatter.write_str("an NROM trainer requires an 8 KiB PRG memory window")
            }
        }
    }
}

impl Error for CartridgeError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(mapper: u8) -> Vec<u8> {
        let mut bytes = vec![0; 16 + NROM_128_PRG + NROM_CHR];
        bytes[0..4].copy_from_slice(b"NES\x1a");
        bytes[4] = 1;
        bytes[5] = 1;
        bytes[6] = mapper << 4;
        bytes
    }

    #[test]
    fn owns_validated_mapper_zero_data() {
        let bytes = image(0);
        let parsed = format_ines::parse(&bytes).expect("generated image parses");
        let cartridge = NesCartridge::from_parsed(parsed).expect("mapper 0 is supported");
        assert_eq!(cartridge.mapper(), 0);
        assert_eq!(cartridge.prg_rom().len(), NROM_128_PRG);
        assert_eq!(cartridge.chr_rom().len(), NROM_CHR);
    }

    #[test]
    fn rejects_mapper_before_machine_construction() {
        let bytes = image(1);
        let parsed = format_ines::parse(&bytes).expect("generated image parses");
        assert_eq!(
            NesCartridge::from_parsed(parsed),
            Err(CartridgeError::UnsupportedMapper(1))
        );
    }

    #[test]
    fn rejects_nes2_prg_memory_that_cannot_fit_the_nrom_cpu_window() {
        let mut bytes = image(0);
        bytes[7] = 0x08;
        bytes[10] = 0x08;
        let parsed = format_ines::parse(&bytes).expect("generated NES 2.0 image parses");

        assert_eq!(
            NesCartridge::from_parsed(parsed),
            Err(CartridgeError::InvalidNromPrgMemory {
                volatile: 16 * 1024,
                nonvolatile: 0,
            })
        );
    }

    #[test]
    fn accepts_only_one_eight_kibibyte_chr_backing_store() {
        fn nes2_image(chr_banks: u8, chr_memory_shift: u8) -> Vec<u8> {
            let mut bytes = vec![0; 16 + NROM_128_PRG + usize::from(chr_banks) * NROM_CHR];
            bytes[0..4].copy_from_slice(b"NES\x1a");
            bytes[4] = 1;
            bytes[5] = chr_banks;
            bytes[7] = 0x08;
            bytes[11] = chr_memory_shift;
            bytes
        }

        let valid_ram = nes2_image(0, 7);
        let parsed = format_ines::parse(&valid_ram).expect("generated CHR-RAM image parses");
        assert!(NesCartridge::from_parsed(parsed).is_ok());

        for bytes in [nes2_image(0, 0), nes2_image(0, 8), nes2_image(1, 7)] {
            let parsed = format_ines::parse(&bytes).expect("generated NES 2.0 image parses");
            assert!(matches!(
                NesCartridge::from_parsed(parsed),
                Err(CartridgeError::InvalidNromChrMemory { .. })
            ));
        }
    }
}
