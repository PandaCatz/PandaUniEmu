#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{Display, Formatter};

const HEADER_LEN: usize = 16;
const TRAINER_LEN: usize = 512;
const PRG_UNIT: usize = 16 * 1024;
const CHR_UNIT: usize = 8 * 1024;
const PRG_RAM_UNIT: usize = 8 * 1024;
const MAX_PRG_ROM: usize = 64 * 1024 * 1024;
const MAX_CHR_ROM: usize = 32 * 1024 * 1024;
pub const MAX_IMAGE_BYTES: usize = HEADER_LEN + TRAINER_LEN + MAX_PRG_ROM + MAX_CHR_ROM;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ImageFormat {
    INes,
    Nes2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    FourScreen,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Region {
    Ntsc,
    Pal,
    MultiRegion,
    Dendy,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RamSizes {
    pub prg_ram: usize,
    pub prg_nvram: usize,
    pub chr_ram: usize,
    pub chr_nvram: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Cartridge<'a> {
    format: ImageFormat,
    mapper: u16,
    submapper: u8,
    mirroring: Mirroring,
    region: Region,
    battery: bool,
    trainer: Option<&'a [u8]>,
    prg_rom: &'a [u8],
    chr_rom: &'a [u8],
    ram: RamSizes,
}

impl<'a> Cartridge<'a> {
    #[must_use]
    pub const fn format(&self) -> ImageFormat {
        self.format
    }

    #[must_use]
    pub const fn mapper(&self) -> u16 {
        self.mapper
    }

    #[must_use]
    pub const fn submapper(&self) -> u8 {
        self.submapper
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
    pub const fn trainer(&self) -> Option<&'a [u8]> {
        self.trainer
    }

    #[must_use]
    pub const fn prg_rom(&self) -> &'a [u8] {
        self.prg_rom
    }

    #[must_use]
    pub const fn chr_rom(&self) -> &'a [u8] {
        self.chr_rom
    }

    #[must_use]
    pub const fn ram_sizes(&self) -> RamSizes {
        self.ram
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Section {
    Header,
    Trainer,
    PrgRom,
    ChrRom,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    Truncated {
        section: Section,
        needed: usize,
        actual: usize,
    },
    InvalidMagic,
    UnsupportedArchaicHeader,
    UnsupportedConsoleType(u8),
    UnsupportedMiscRoms(u8),
    SizeOverflow(Section),
    ImageTooLarge {
        section: Section,
        size: usize,
        limit: usize,
    },
    UnexpectedTrailingData(usize),
}

impl Display for ParseError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated {
                section,
                needed,
                actual,
            } => write!(
                formatter,
                "truncated {section:?}: need {needed} bytes, got {actual}"
            ),
            Self::InvalidMagic => formatter.write_str("invalid iNES magic"),
            Self::UnsupportedArchaicHeader => formatter
                .write_str("unsupported archaic or dirty iNES header; normalize it before loading"),
            Self::UnsupportedConsoleType(kind) => {
                write!(formatter, "unsupported iNES console type {kind}")
            }
            Self::UnsupportedMiscRoms(count) => {
                write!(
                    formatter,
                    "unsupported NES 2.0 miscellaneous ROM count {count}"
                )
            }
            Self::SizeOverflow(section) => write!(formatter, "{section:?} size overflowed"),
            Self::ImageTooLarge {
                section,
                size,
                limit,
            } => write!(
                formatter,
                "{section:?} size {size} exceeds configured limit {limit}"
            ),
            Self::UnexpectedTrailingData(count) => {
                write!(
                    formatter,
                    "image contains {count} unexplained trailing bytes"
                )
            }
        }
    }
}

impl Error for ParseError {}

pub fn parse(image: &[u8]) -> Result<Cartridge<'_>, ParseError> {
    if image.len() < HEADER_LEN {
        return Err(ParseError::Truncated {
            section: Section::Header,
            needed: HEADER_LEN,
            actual: image.len(),
        });
    }
    if image[0..4] != *b"NES\x1a" {
        return Err(ParseError::InvalidMagic);
    }

    let flags6 = image[6];
    let flags7 = image[7];
    let format = if flags7 & 0x0c == 0x08 {
        ImageFormat::Nes2
    } else {
        ImageFormat::INes
    };
    if format == ImageFormat::INes && image[12..16].iter().any(|byte| *byte != 0) {
        return Err(ParseError::UnsupportedArchaicHeader);
    }

    let console_type = flags7 & 0x03;
    if console_type != 0 {
        return Err(ParseError::UnsupportedConsoleType(console_type));
    }

    if format == ImageFormat::Nes2 {
        let misc_roms = image[14] & 0x03;
        if misc_roms != 0 {
            return Err(ParseError::UnsupportedMiscRoms(misc_roms));
        }
    }

    let (mapper, submapper) = match format {
        ImageFormat::INes => (u16::from(flags6 >> 4) | u16::from(flags7 & 0xf0), 0),
        ImageFormat::Nes2 => (
            u16::from(flags6 >> 4) | u16::from(flags7 & 0xf0) | (u16::from(image[8] & 0x0f) << 8),
            image[8] >> 4,
        ),
    };

    let mirroring = if flags6 & 0x08 != 0 {
        Mirroring::FourScreen
    } else if flags6 & 0x01 != 0 {
        Mirroring::Vertical
    } else {
        Mirroring::Horizontal
    };
    let battery = flags6 & 0x02 != 0;
    let has_trainer = flags6 & 0x04 != 0;

    let (prg_size, chr_size) = match format {
        ImageFormat::INes => (
            checked_linear_size(image[4], PRG_UNIT, Section::PrgRom)?,
            checked_linear_size(image[5], CHR_UNIT, Section::ChrRom)?,
        ),
        ImageFormat::Nes2 => (
            nes2_rom_size(image[4], image[9] & 0x0f, PRG_UNIT, Section::PrgRom)?,
            nes2_rom_size(image[5], image[9] >> 4, CHR_UNIT, Section::ChrRom)?,
        ),
    };
    enforce_limit(prg_size, MAX_PRG_ROM, Section::PrgRom)?;
    enforce_limit(chr_size, MAX_CHR_ROM, Section::ChrRom)?;

    let ram = match format {
        ImageFormat::INes => {
            let units = if image[8] == 0 { 1 } else { image[8] };
            let size = checked_linear_size(units, PRG_RAM_UNIT, Section::PrgRom)?;
            RamSizes {
                prg_ram: if battery { 0 } else { size },
                prg_nvram: if battery { size } else { 0 },
                chr_ram: if chr_size == 0 { CHR_UNIT } else { 0 },
                chr_nvram: 0,
            }
        }
        ImageFormat::Nes2 => RamSizes {
            prg_ram: decode_ram_shift(image[10] & 0x0f, Section::PrgRom)?,
            prg_nvram: decode_ram_shift(image[10] >> 4, Section::PrgRom)?,
            chr_ram: decode_ram_shift(image[11] & 0x0f, Section::ChrRom)?,
            chr_nvram: decode_ram_shift(image[11] >> 4, Section::ChrRom)?,
        },
    };

    let region = match format {
        ImageFormat::INes => {
            if image[9] & 0x01 == 0 {
                Region::Ntsc
            } else {
                Region::Pal
            }
        }
        ImageFormat::Nes2 => match image[12] & 0x03 {
            0 => Region::Ntsc,
            1 => Region::Pal,
            2 => Region::MultiRegion,
            3 => Region::Dendy,
            _ => unreachable!(),
        },
    };

    let trainer_start = HEADER_LEN;
    let trainer_end = if has_trainer {
        checked_end(trainer_start, TRAINER_LEN, Section::Trainer)?
    } else {
        trainer_start
    };
    ensure_available(image, trainer_end, Section::Trainer)?;

    let prg_end = checked_end(trainer_end, prg_size, Section::PrgRom)?;
    ensure_available(image, prg_end, Section::PrgRom)?;
    let chr_end = checked_end(prg_end, chr_size, Section::ChrRom)?;
    ensure_available(image, chr_end, Section::ChrRom)?;

    if image.len() != chr_end {
        return Err(ParseError::UnexpectedTrailingData(image.len() - chr_end));
    }

    Ok(Cartridge {
        format,
        mapper,
        submapper,
        mirroring,
        region,
        battery,
        trainer: has_trainer.then(|| &image[trainer_start..trainer_end]),
        prg_rom: &image[trainer_end..prg_end],
        chr_rom: &image[prg_end..chr_end],
        ram,
    })
}

fn checked_linear_size(units: u8, unit_size: usize, section: Section) -> Result<usize, ParseError> {
    usize::from(units)
        .checked_mul(unit_size)
        .ok_or(ParseError::SizeOverflow(section))
}

fn nes2_rom_size(
    least_significant: u8,
    most_significant: u8,
    unit_size: usize,
    section: Section,
) -> Result<usize, ParseError> {
    if most_significant != 0x0f {
        let units = (usize::from(most_significant) << 8) | usize::from(least_significant);
        return units
            .checked_mul(unit_size)
            .ok_or(ParseError::SizeOverflow(section));
    }

    let exponent = u32::from(least_significant >> 2);
    let multiplier = usize::from((least_significant & 0x03) * 2 + 1);
    1usize
        .checked_shl(exponent)
        .and_then(|base| base.checked_mul(multiplier))
        .ok_or(ParseError::SizeOverflow(section))
}

fn decode_ram_shift(value: u8, section: Section) -> Result<usize, ParseError> {
    if value == 0 {
        return Ok(0);
    }
    64usize
        .checked_shl(u32::from(value))
        .ok_or(ParseError::SizeOverflow(section))
}

fn enforce_limit(size: usize, limit: usize, section: Section) -> Result<(), ParseError> {
    if size > limit {
        return Err(ParseError::ImageTooLarge {
            section,
            size,
            limit,
        });
    }
    Ok(())
}

fn checked_end(start: usize, len: usize, section: Section) -> Result<usize, ParseError> {
    start
        .checked_add(len)
        .ok_or(ParseError::SizeOverflow(section))
}

fn ensure_available(image: &[u8], needed: usize, section: Section) -> Result<(), ParseError> {
    if image.len() < needed {
        return Err(ParseError::Truncated {
            section,
            needed,
            actual: image.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(prg_units: u8, chr_units: u8, flags6: u8, flags7: u8) -> Vec<u8> {
        let mut bytes = vec![0; HEADER_LEN];
        bytes[0..4].copy_from_slice(b"NES\x1a");
        bytes[4] = prg_units;
        bytes[5] = chr_units;
        bytes[6] = flags6;
        bytes[7] = flags7;
        if flags6 & 0x04 != 0 {
            bytes.extend((0..TRAINER_LEN).map(|value| (value & 0xff) as u8));
        }
        bytes.resize(bytes.len() + usize::from(prg_units) * PRG_UNIT, 0xa5);
        bytes.resize(bytes.len() + usize::from(chr_units) * CHR_UNIT, 0x5a);
        bytes
    }

    #[test]
    fn parses_basic_ines_nrom() {
        let bytes = image(1, 1, 0x01, 0);
        let cartridge = parse(&bytes).expect("valid generated image");
        assert_eq!(cartridge.format(), ImageFormat::INes);
        assert_eq!(cartridge.mapper(), 0);
        assert_eq!(cartridge.mirroring(), Mirroring::Vertical);
        assert_eq!(cartridge.region(), Region::Ntsc);
        assert_eq!(cartridge.prg_rom().len(), PRG_UNIT);
        assert_eq!(cartridge.chr_rom().len(), CHR_UNIT);
        assert_eq!(cartridge.ram_sizes().prg_ram, PRG_RAM_UNIT);
    }

    #[test]
    fn trainer_is_sliced_before_prg_rom() {
        let bytes = image(1, 0, 0x04, 0);
        let cartridge = parse(&bytes).expect("valid generated trainer image");
        let trainer = cartridge.trainer().expect("trainer flag set");
        assert_eq!(trainer.len(), TRAINER_LEN);
        assert_eq!(trainer[0], 0);
        assert_eq!(trainer[511], 255);
        assert_eq!(cartridge.prg_rom()[0], 0xa5);
    }

    #[test]
    fn parses_nes2_mapper_submapper_and_ram() {
        let mut bytes = image(1, 1, 0x10, 0x28);
        bytes[8] = 0x43;
        bytes[10] = 0x87;
        bytes[11] = 0x65;
        let cartridge = parse(&bytes).expect("valid generated NES 2.0 image");
        assert_eq!(cartridge.format(), ImageFormat::Nes2);
        assert_eq!(cartridge.mapper(), 0x321);
        assert_eq!(cartridge.submapper(), 4);
        assert_eq!(cartridge.ram_sizes().prg_ram, 8 * 1024);
        assert_eq!(cartridge.ram_sizes().prg_nvram, 16 * 1024);
        assert_eq!(cartridge.ram_sizes().chr_ram, 2 * 1024);
        assert_eq!(cartridge.ram_sizes().chr_nvram, 4 * 1024);
    }

    #[test]
    fn parses_nes2_exponent_multiplier_size() {
        let mut bytes = image(0, 0, 0, 0x08);
        bytes[4] = (14 << 2) | 1;
        bytes[9] = 0x0f;
        bytes.resize(HEADER_LEN + (1 << 14) * 3, 0xcc);
        let cartridge = parse(&bytes).expect("valid exponent encoded image");
        assert_eq!(cartridge.prg_rom().len(), (1 << 14) * 3);
    }

    #[test]
    fn rejects_every_truncation_point() {
        let bytes = image(1, 1, 0x04, 0);
        for end in 0..bytes.len() {
            assert!(parse(&bytes[..end]).is_err(), "accepted length {end}");
        }
        assert!(parse(&bytes).is_ok());
    }

    #[test]
    fn rejects_bad_magic_and_trailing_data() {
        let mut bytes = image(1, 0, 0, 0);
        bytes[0] = b'B';
        assert_eq!(parse(&bytes), Err(ParseError::InvalidMagic));

        let mut bytes = image(1, 0, 0, 0);
        bytes.push(0);
        assert_eq!(parse(&bytes), Err(ParseError::UnexpectedTrailingData(1)));
    }

    #[test]
    fn rejects_dirty_diskdude_header_instead_of_inventing_mapper_fields() {
        let mut bytes = image(1, 1, 0, 0);
        bytes[7..16].copy_from_slice(b"DiskDude!");
        assert_eq!(parse(&bytes), Err(ParseError::UnsupportedArchaicHeader));
    }

    #[test]
    fn rejects_unsupported_console_and_misc_roms() {
        let console = image(1, 0, 0, 0x01);
        assert_eq!(parse(&console), Err(ParseError::UnsupportedConsoleType(1)));

        let mut misc = image(1, 0, 0, 0x08);
        misc[14] = 1;
        assert_eq!(parse(&misc), Err(ParseError::UnsupportedMiscRoms(1)));
    }

    #[test]
    fn rejects_oversized_exponent_encoding_without_allocating_payload() {
        let mut bytes = image(0, 0, 0, 0x08);
        bytes[4] = 63 << 2;
        bytes[9] = 0x0f;
        assert!(matches!(
            parse(&bytes),
            Err(ParseError::ImageTooLarge {
                section: Section::PrgRom,
                ..
            }) | Err(ParseError::SizeOverflow(Section::PrgRom))
        ));
    }

    #[test]
    fn arbitrary_small_inputs_do_not_panic() {
        let mut state = 0x4d59_5df4_d0f3_3173_u64;
        for len in 0..256 {
            let mut bytes = vec![0; len];
            for byte in &mut bytes {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                *byte = (state >> 32) as u8;
            }
            let _ = parse(&bytes);
        }
    }
}
