use crate::NesCartridge;
use cpu_6502::Bus;
use format_ines::Mirroring;
use std::fmt::{Display, Formatter};

const CPU_RAM_SIZE: usize = 2 * 1024;
const PRG_RAM_START: u16 = 0x6000;
const PRG_RAM_SIZE: usize = 8 * 1024;
const PRG_ROM_START: u16 = 0x8000;
const TRAINER_START_IN_PRG_RAM: usize = 0x1000;
const CHR_RAM_SIZE: usize = 8 * 1024;
const NAMETABLE_SIZE: usize = 4 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CpuBusFault {
    UnsupportedRead { address: u16 },
    UnsupportedWrite { address: u16, value: u8 },
}

impl Display for CpuBusFault {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedRead { address } => {
                write!(formatter, "unsupported NES CPU read at ${address:04X}")
            }
            Self::UnsupportedWrite { address, value } => write!(
                formatter,
                "unsupported NES CPU write of ${value:02X} at ${address:04X}"
            ),
        }
    }
}

impl std::error::Error for CpuBusFault {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NromCpuBus {
    cartridge: NesCartridge,
    cpu_ram: [u8; CPU_RAM_SIZE],
    prg_ram: Vec<u8>,
    chr_ram: Vec<u8>,
    nametables: [u8; NAMETABLE_SIZE],
    ppu_address_bus: u16,
    open_bus: u8,
    fault: Option<CpuBusFault>,
}

impl NromCpuBus {
    #[must_use]
    pub fn new(cartridge: NesCartridge) -> Self {
        let ram_sizes = cartridge.ram_sizes();
        let prg_memory = ram_sizes.prg_ram + ram_sizes.prg_nvram;
        debug_assert!(matches!(prg_memory, 0 | PRG_RAM_SIZE));
        let mut prg_ram = vec![0; prg_memory];
        let chr_ram = if cartridge.chr_rom().is_empty() {
            vec![0; CHR_RAM_SIZE]
        } else {
            Vec::new()
        };
        if let Some(trainer) = cartridge.trainer() {
            debug_assert_eq!(trainer.len(), 512);
            prg_ram[TRAINER_START_IN_PRG_RAM..TRAINER_START_IN_PRG_RAM + trainer.len()]
                .copy_from_slice(trainer);
        }
        Self {
            cartridge,
            cpu_ram: [0; CPU_RAM_SIZE],
            prg_ram,
            chr_ram,
            nametables: [0; NAMETABLE_SIZE],
            ppu_address_bus: 0,
            open_bus: 0,
            fault: None,
        }
    }

    #[must_use]
    pub const fn cartridge(&self) -> &NesCartridge {
        &self.cartridge
    }

    #[must_use]
    pub const fn cpu_ram(&self) -> &[u8; CPU_RAM_SIZE] {
        &self.cpu_ram
    }

    #[must_use]
    pub fn prg_ram(&self) -> &[u8] {
        &self.prg_ram
    }

    pub fn take_fault(&mut self) -> Option<CpuBusFault> {
        self.fault.take()
    }

    pub(crate) const fn observe_open_bus(&mut self, value: u8) {
        self.open_bus = value;
    }

    pub(crate) fn peek_ppu(&self, address: u16) -> u8 {
        let address = address & 0x3fff;
        match address {
            0x0000..=0x1fff if self.chr_ram.is_empty() => {
                self.cartridge.chr_rom()[usize::from(address)]
            }
            0x0000..=0x1fff => self.chr_ram[usize::from(address)],
            0x2000..=0x3eff => self.nametables[self.nametable_index(address)],
            _ => unreachable!("palette RAM is internal to the PPU"),
        }
    }

    fn nametable_index(&self, address: u16) -> usize {
        let mirrored = if address >= 0x3000 {
            address - 0x1000
        } else {
            address
        };
        let offset = usize::from(mirrored - 0x2000);
        let logical_table = offset / 0x400;
        let physical_table = match self.cartridge.mirroring() {
            Mirroring::Horizontal => logical_table >> 1,
            Mirroring::Vertical => logical_table & 1,
            Mirroring::FourScreen => logical_table,
        };
        physical_table * 0x400 + (offset & 0x3ff)
    }

    pub fn peek(&self, address: u16) -> Result<u8, CpuBusFault> {
        match address {
            0x0000..=0x1fff => Ok(self.cpu_ram[usize::from(address) & (CPU_RAM_SIZE - 1)]),
            PRG_RAM_START..=0x7fff if !self.prg_ram.is_empty() => {
                Ok(self.prg_ram[usize::from(address - PRG_RAM_START)])
            }
            PRG_ROM_START..=u16::MAX => {
                let offset = usize::from(address - PRG_ROM_START);
                let index = if self.cartridge.prg_rom().len() == 16 * 1024 {
                    offset & 0x3fff
                } else {
                    offset
                };
                Ok(self.cartridge.prg_rom()[index])
            }
            _ => Err(CpuBusFault::UnsupportedRead { address }),
        }
    }

    fn record_fault(&mut self, fault: CpuBusFault) {
        if self.fault.is_none() {
            self.fault = Some(fault);
        }
    }
}

impl crate::ppu::PpuBus for NromCpuBus {
    fn observe_address(&mut self, address: u16) {
        self.ppu_address_bus = address & 0x3fff;
    }

    fn peek(&self, address: u16) -> u8 {
        self.peek_ppu(address)
    }

    fn read(&mut self, address: u16) -> u8 {
        self.peek_ppu(address)
    }

    fn write(&mut self, address: u16, value: u8) {
        let address = address & 0x3fff;
        match address {
            0x0000..=0x1fff if !self.chr_ram.is_empty() => {
                self.chr_ram[usize::from(address)] = value;
            }
            0x0000..=0x1fff => {}
            0x2000..=0x3eff => {
                let index = self.nametable_index(address);
                self.nametables[index] = value;
            }
            _ => unreachable!("palette RAM is internal to the PPU"),
        }
    }
}

impl Bus for NromCpuBus {
    fn read(&mut self, address: u16) -> u8 {
        match self.peek(address) {
            Ok(value) => {
                self.open_bus = value;
                value
            }
            Err(fault) => {
                self.record_fault(fault);
                self.open_bus
            }
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        self.open_bus = value;
        match address {
            0x0000..=0x1fff => {
                self.cpu_ram[usize::from(address) & (CPU_RAM_SIZE - 1)] = value;
            }
            PRG_RAM_START..=0x7fff if !self.prg_ram.is_empty() => {
                self.prg_ram[usize::from(address - PRG_RAM_START)] = value;
            }
            PRG_ROM_START..=u16::MAX => {}
            _ => self.record_fault(CpuBusFault::UnsupportedWrite { address, value }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cartridge(prg_banks: u8, trainer: bool) -> NesCartridge {
        let trainer_len = usize::from(trainer) * 512;
        let prg_len = usize::from(prg_banks) * 16 * 1024;
        let mut bytes = vec![0; 16 + trainer_len + prg_len + 8 * 1024];
        bytes[0..4].copy_from_slice(b"NES\x1a");
        bytes[4] = prg_banks;
        bytes[5] = 1;
        bytes[6] = u8::from(trainer) << 2;
        if trainer {
            bytes[16..16 + 512].fill(0x5a);
        }
        let prg_start = 16 + trainer_len;
        for (index, byte) in bytes[prg_start..prg_start + prg_len].iter_mut().enumerate() {
            *byte = (index >> 14) as u8 + 0x10;
        }
        let parsed = format_ines::parse(&bytes).expect("generated NROM image parses");
        NesCartridge::from_parsed(parsed).expect("generated NROM cartridge is supported")
    }

    #[test]
    fn mirrors_internal_ram_every_two_kibibytes() {
        let mut bus = NromCpuBus::new(cartridge(1, false));
        bus.write(0x0002, 0xa5);
        assert_eq!(bus.read(0x0802), 0xa5);
        assert_eq!(bus.read(0x1002), 0xa5);
        assert_eq!(bus.read(0x1802), 0xa5);
    }

    #[test]
    fn mirrors_nrom_128_and_maps_nrom_256_without_mirroring() {
        let bus128 = NromCpuBus::new(cartridge(1, false));
        assert_eq!(bus128.peek(0x8000), Ok(0x10));
        assert_eq!(bus128.peek(0xc000), Ok(0x10));

        let bus256 = NromCpuBus::new(cartridge(2, false));
        assert_eq!(bus256.peek(0x8000), Ok(0x10));
        assert_eq!(bus256.peek(0xc000), Ok(0x11));
    }

    #[test]
    fn maps_prg_memory_and_preloads_trainer_at_seven_thousand() {
        let mut bus = NromCpuBus::new(cartridge(1, true));
        assert_eq!(bus.peek(0x7000), Ok(0x5a));
        assert_eq!(bus.peek(0x71ff), Ok(0x5a));
        bus.write(0x6000, 0xc3);
        assert_eq!(bus.peek(0x6000), Ok(0xc3));
    }

    #[test]
    fn unsupported_io_records_the_first_fault_without_panicking() {
        let mut bus = NromCpuBus::new(cartridge(1, false));
        bus.write(0x2000, 0x44);
        assert_eq!(bus.read(0x2001), 0x44);
        assert_eq!(
            bus.take_fault(),
            Some(CpuBusFault::UnsupportedWrite {
                address: 0x2000,
                value: 0x44,
            })
        );
        assert_eq!(bus.take_fault(), None);
    }

    #[test]
    fn rom_writes_are_ignored_and_reset_vector_uses_nrom_mirroring() {
        let mut cartridge = cartridge(1, false);
        cartridge.prg_rom[0x3ffc] = 0x34;
        cartridge.prg_rom[0x3ffd] = 0x12;
        let mut bus = NromCpuBus::new(cartridge);
        let before = bus.peek(0x8000).expect("ROM address is mapped");
        bus.write(0x8000, before.wrapping_add(1));
        assert_eq!(bus.peek(0x8000), Ok(before));

        let mut cpu = cpu_6502::Cpu::new(cpu_6502::CpuState::default());
        cpu.power_on(&mut bus).expect("power-on cycle count fits");
        assert_eq!(cpu.state().pc, 0x1234);
        assert_eq!(cpu.state().total_cycles, 7);
    }
}
