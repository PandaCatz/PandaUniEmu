#![forbid(unsafe_code)]

use format_ines::{Cartridge, Mirroring, RamSizes, Region};
use std::error::Error;
use std::fmt::{Display, Formatter};

const NROM_128_PRG: usize = 16 * 1024;
const NROM_256_PRG: usize = 32 * 1024;
const NROM_CHR: usize = 8 * 1024;

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
        if !matches!(parsed.chr_rom().len(), 0 | NROM_CHR) {
            return Err(CartridgeError::InvalidNromChrSize(parsed.chr_rom().len()));
        }

        Ok(Self {
            mapper: parsed.mapper(),
            mirroring: parsed.mirroring(),
            region: parsed.region(),
            battery: parsed.has_battery(),
            ram: parsed.ram_sizes(),
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
}
