use crate::{NesCartridge, PpuEvent};
use format_ines::Mirroring;

const CHR_SIZE: usize = 8 * 1024;
const NAMETABLE_SIZE: usize = 4 * 1024;
const PALETTE_SIZE: usize = 32;
const OAM_SIZE: usize = 256;

const STATUS_VBLANK: u8 = 0x80;
const CONTROL_NMI_ENABLE: u8 = 0x80;
const CONTROL_INCREMENT_32: u8 = 0x04;
const MASK_RENDERING: u8 = 0x18;
const MASK_GRAYSCALE: u8 = 0x01;
const OAM_ATTRIBUTE_MASK: u8 = 0xe3;

/// Deterministic NES PPU register and address-space shell.
///
/// This models CPU-visible register side effects and logical PPU memory
/// routing. Rendering fetches, sprites, DMA, analog open-bus decay, and the
/// dot-exact PPUSTATUS/VBlank race window, and reset write-ignore warmup are
/// later milestones.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ppu {
    control: u8,
    mask: u8,
    status: u8,
    oam_address: u8,
    oam: [u8; OAM_SIZE],
    vram_address: u16,
    temporary_address: u16,
    fine_x: u8,
    write_toggle: bool,
    data_buffer: u8,
    io_latch: u8,
    chr: Vec<u8>,
    chr_writable: bool,
    nametables: [u8; NAMETABLE_SIZE],
    palette: [u8; PALETTE_SIZE],
    mirroring: Mirroring,
}

impl Ppu {
    #[must_use]
    pub fn new(cartridge: &NesCartridge) -> Self {
        let chr_writable = cartridge.chr_rom().is_empty();
        let chr = if chr_writable {
            vec![0; CHR_SIZE]
        } else {
            cartridge.chr_rom().to_vec()
        };
        debug_assert_eq!(chr.len(), CHR_SIZE);

        Self {
            control: 0,
            mask: 0,
            status: 0,
            oam_address: 0,
            oam: [0; OAM_SIZE],
            vram_address: 0,
            temporary_address: 0,
            fine_x: 0,
            write_toggle: false,
            data_buffer: 0,
            io_latch: 0,
            chr,
            chr_writable,
            nametables: [0; NAMETABLE_SIZE],
            palette: [0; PALETTE_SIZE],
            mirroring: cartridge.mirroring(),
        }
    }

    #[must_use]
    pub const fn control(&self) -> u8 {
        self.control
    }

    #[must_use]
    pub const fn mask(&self) -> u8 {
        self.mask
    }

    #[must_use]
    pub const fn status(&self) -> u8 {
        self.status
    }

    #[must_use]
    pub const fn oam_address(&self) -> u8 {
        self.oam_address
    }

    #[must_use]
    pub const fn oam(&self) -> &[u8; OAM_SIZE] {
        &self.oam
    }

    #[must_use]
    pub const fn vram_address(&self) -> u16 {
        self.vram_address
    }

    #[must_use]
    pub const fn temporary_address(&self) -> u16 {
        self.temporary_address
    }

    #[must_use]
    pub const fn fine_x(&self) -> u8 {
        self.fine_x
    }

    #[must_use]
    pub const fn write_toggle(&self) -> bool {
        self.write_toggle
    }

    #[must_use]
    pub const fn data_buffer(&self) -> u8 {
        self.data_buffer
    }

    #[must_use]
    pub const fn io_latch(&self) -> u8 {
        self.io_latch
    }

    #[must_use]
    pub const fn rendering_enabled(&self) -> bool {
        self.mask & MASK_RENDERING != 0
    }

    #[must_use]
    pub const fn nmi_output(&self) -> bool {
        self.status & STATUS_VBLANK != 0 && self.control & CONTROL_NMI_ENABLE != 0
    }

    #[must_use]
    pub fn peek_memory(&self, address: u16) -> u8 {
        let address = address & 0x3fff;
        match address {
            0x0000..=0x1fff => self.chr[usize::from(address)],
            0x2000..=0x3eff => self.nametables[self.nametable_index(address)],
            0x3f00..=0x3fff => self.palette[Self::palette_index(address)],
            _ => unreachable!("14-bit PPU address mapping is exhaustive"),
        }
    }

    pub(crate) fn apply_event(&mut self, event: PpuEvent) {
        match event {
            PpuEvent::VblankStarted => self.status |= STATUS_VBLANK,
            PpuEvent::VblankEnded => self.status &= !STATUS_VBLANK,
        }
    }

    pub(crate) fn reset_registers(&mut self) {
        self.control = 0;
        self.mask = 0;
        self.temporary_address = 0;
        self.fine_x = 0;
        self.write_toggle = false;
        self.data_buffer = 0;
    }

    pub(crate) fn cpu_read_register(&mut self, register: u8) -> u8 {
        let value = match register & 0x07 {
            2 => {
                let value = (self.status & 0xe0) | (self.io_latch & 0x1f);
                self.status &= !STATUS_VBLANK;
                self.write_toggle = false;
                value
            }
            4 => self.oam[usize::from(self.oam_address)],
            7 => self.read_data(),
            _ => self.io_latch,
        };
        self.io_latch = value;
        value
    }

    pub(crate) fn cpu_write_register(&mut self, register: u8, value: u8) {
        self.io_latch = value;
        match register & 0x07 {
            0 => {
                self.control = value;
                self.temporary_address =
                    (self.temporary_address & !0x0c00) | (u16::from(value & 0x03) << 10);
            }
            1 => self.mask = value,
            2 => {}
            3 => self.oam_address = value,
            4 => {
                let stored = if self.oam_address & 0x03 == 2 {
                    value & OAM_ATTRIBUTE_MASK
                } else {
                    value
                };
                self.oam[usize::from(self.oam_address)] = stored;
                self.oam_address = self.oam_address.wrapping_add(1);
            }
            5 => self.write_scroll(value),
            6 => self.write_address(value),
            7 => {
                self.write_memory(self.vram_address, value);
                self.increment_vram_address();
            }
            _ => unreachable!("three-bit register mapping is exhaustive"),
        }
    }

    fn write_scroll(&mut self, value: u8) {
        if self.write_toggle {
            self.temporary_address = (self.temporary_address & !0x73e0)
                | (u16::from(value & 0xf8) << 2)
                | (u16::from(value & 0x07) << 12);
        } else {
            self.temporary_address = (self.temporary_address & !0x001f) | u16::from(value >> 3);
            self.fine_x = value & 0x07;
        }
        self.write_toggle = !self.write_toggle;
    }

    fn write_address(&mut self, value: u8) {
        if self.write_toggle {
            self.temporary_address = (self.temporary_address & 0x7f00) | u16::from(value);
            self.vram_address = self.temporary_address;
        } else {
            self.temporary_address =
                (self.temporary_address & 0x00ff) | (u16::from(value & 0x3f) << 8);
        }
        self.write_toggle = !self.write_toggle;
    }

    fn read_data(&mut self) -> u8 {
        let address = self.vram_address & 0x3fff;
        let value = if address >= 0x3f00 {
            let color_mask = if self.mask & MASK_GRAYSCALE != 0 {
                0x30
            } else {
                0x3f
            };
            let palette_value = self.peek_memory(address) & color_mask;
            self.data_buffer = self.peek_memory(address - 0x1000);
            (self.io_latch & 0xc0) | palette_value
        } else {
            let buffered = self.data_buffer;
            self.data_buffer = self.peek_memory(address);
            buffered
        };
        self.increment_vram_address();
        value
    }

    fn increment_vram_address(&mut self) {
        let increment = if self.control & CONTROL_INCREMENT_32 != 0 {
            32
        } else {
            1
        };
        self.vram_address = self.vram_address.wrapping_add(increment) & 0x7fff;
    }

    fn write_memory(&mut self, address: u16, value: u8) {
        let address = address & 0x3fff;
        match address {
            0x0000..=0x1fff if self.chr_writable => self.chr[usize::from(address)] = value,
            0x0000..=0x1fff => {}
            0x2000..=0x3eff => {
                let index = self.nametable_index(address);
                self.nametables[index] = value;
            }
            0x3f00..=0x3fff => self.palette[Self::palette_index(address)] = value & 0x3f,
            _ => unreachable!("14-bit PPU address mapping is exhaustive"),
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
        let physical_table = match self.mirroring {
            Mirroring::Horizontal => logical_table >> 1,
            Mirroring::Vertical => logical_table & 1,
            Mirroring::FourScreen => logical_table,
        };
        physical_table * 0x400 + (offset & 0x3ff)
    }

    fn palette_index(address: u16) -> usize {
        let mut index = usize::from(address - 0x3f00) & 0x1f;
        if matches!(index, 0x10 | 0x14 | 0x18 | 0x1c) {
            index -= 0x10;
        }
        index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cartridge(flags6: u8, chr_ram: bool) -> NesCartridge {
        let chr_len = if chr_ram { 0 } else { CHR_SIZE };
        let mut bytes = vec![0; 16 + 16 * 1024 + chr_len];
        bytes[0..4].copy_from_slice(b"NES\x1a");
        bytes[4] = 1;
        bytes[5] = u8::from(!chr_ram);
        bytes[6] = flags6;
        if chr_ram {
            bytes[10] = 0x07;
        }
        let parsed = format_ines::parse(&bytes).expect("generated NROM image parses");
        NesCartridge::from_parsed(parsed).expect("generated NROM cartridge is supported")
    }

    fn set_address(ppu: &mut Ppu, address: u16) {
        ppu.cpu_write_register(6, (address >> 8) as u8);
        ppu.cpu_write_register(6, address as u8);
    }

    #[test]
    fn scroll_address_and_status_share_the_hardware_write_toggle() {
        let mut ppu = Ppu::new(&cartridge(0, false));
        ppu.cpu_write_register(0, 0x03);
        ppu.cpu_write_register(5, 0xad);
        assert_eq!(ppu.temporary_address() & 0x0c1f, 0x0c15);
        assert_eq!(ppu.fine_x(), 5);
        assert!(ppu.write_toggle());

        ppu.cpu_write_register(5, 0x76);
        assert_eq!(ppu.temporary_address() & 0x73e0, 0x61c0);
        assert!(!ppu.write_toggle());

        ppu.cpu_write_register(6, 0xff);
        assert!(ppu.write_toggle());
        ppu.apply_event(PpuEvent::VblankStarted);
        ppu.cpu_write_register(2, 0x1b);
        assert_eq!(ppu.cpu_read_register(2), 0x9b);
        assert_eq!(ppu.status() & STATUS_VBLANK, 0);
        assert!(!ppu.write_toggle());

        ppu.cpu_write_register(6, 0xff);
        ppu.cpu_write_register(6, 0xff);
        assert_eq!(ppu.vram_address(), 0x3fff);
    }

    #[test]
    fn data_reads_buffer_non_palette_and_refill_from_palette_shadow() {
        let mut ppu = Ppu::new(&cartridge(0, true));
        set_address(&mut ppu, 0x0004);
        ppu.cpu_write_register(7, 0xa5);
        set_address(&mut ppu, 0x0004);
        assert_eq!(ppu.cpu_read_register(7), 0);
        assert_eq!(ppu.cpu_read_register(7), 0xa5);

        set_address(&mut ppu, 0x2f00);
        ppu.cpu_write_register(7, 0x2c);
        set_address(&mut ppu, 0x3f00);
        ppu.cpu_write_register(7, 0x15);
        set_address(&mut ppu, 0x3f00);
        ppu.cpu_write_register(2, 0xc0);
        assert_eq!(ppu.cpu_read_register(7), 0xd5);
        assert_eq!(ppu.data_buffer(), 0x2c);

        ppu.cpu_write_register(1, MASK_GRAYSCALE);
        set_address(&mut ppu, 0x3f00);
        ppu.cpu_write_register(2, 0xc0);
        assert_eq!(ppu.cpu_read_register(7), 0xd0);
        assert_eq!(ppu.peek_memory(0x3f00), 0x15);
    }

    #[test]
    fn data_increment_wraps_and_chr_rom_writes_are_nonfatal_and_ignored() {
        let mut rom_ppu = Ppu::new(&cartridge(0, false));
        set_address(&mut rom_ppu, 0x1fff);
        let before = rom_ppu.peek_memory(0x1fff);
        rom_ppu.cpu_write_register(7, before.wrapping_add(1));
        assert_eq!(rom_ppu.peek_memory(0x1fff), before);
        assert_eq!(rom_ppu.vram_address(), 0x2000);

        rom_ppu.cpu_write_register(0, CONTROL_INCREMENT_32);
        set_address(&mut rom_ppu, 0x3fff);
        rom_ppu.cpu_write_register(7, 0x22);
        assert_eq!(rom_ppu.vram_address(), 0x401f);
        assert_eq!(rom_ppu.vram_address() & 0x3fff, 0x001f);
    }

    #[test]
    fn horizontal_vertical_and_four_screen_nametables_route_distinctly() {
        let cases = [
            (0, [0_usize, 0, 1, 1]),
            (1, [0_usize, 1, 0, 1]),
            (8, [0_usize, 1, 2, 3]),
        ];
        for (flags6, physical) in cases {
            let mut ppu = Ppu::new(&cartridge(flags6, false));
            for (table, value) in [0x11_u8, 0x22, 0x33, 0x44].into_iter().enumerate() {
                let address = 0x2000 + (table as u16) * 0x400 + 0x3ff;
                ppu.write_memory(address, value);
            }
            for table in 0..4 {
                let expected_table = physical[table];
                let last_writer = physical
                    .iter()
                    .rposition(|candidate| *candidate == expected_table)
                    .expect("physical table is present");
                let address = 0x2000 + (table as u16) * 0x400 + 0x3ff;
                assert_eq!(ppu.peek_memory(address), 0x11 + (last_writer as u8) * 0x11);
            }
            assert_eq!(ppu.peek_memory(0x3000), ppu.peek_memory(0x2000));
            assert_eq!(ppu.peek_memory(0x3eff), ppu.peek_memory(0x2eff));
        }
    }

    #[test]
    fn palette_repeats_and_universal_background_entries_alias_both_ways() {
        let mut ppu = Ppu::new(&cartridge(0, false));
        ppu.write_memory(0x3f10, 0xff);
        assert_eq!(ppu.peek_memory(0x3f00), 0x3f);
        assert_eq!(ppu.peek_memory(0x3f20), 0x3f);
        ppu.write_memory(0x3f04, 0x2a);
        assert_eq!(ppu.peek_memory(0x3f14), 0x2a);
        assert_eq!(ppu.peek_memory(0x3ff4), 0x2a);
    }

    #[test]
    fn oam_and_mask_have_their_basic_register_semantics() {
        let mut ppu = Ppu::new(&cartridge(0, false));
        ppu.cpu_write_register(3, 0xff);
        ppu.cpu_write_register(4, 0x55);
        ppu.cpu_write_register(4, 0xaa);
        assert_eq!(ppu.oam()[0xff], 0x55);
        assert_eq!(ppu.oam()[0], 0xaa);
        assert_eq!(ppu.oam_address(), 1);
        ppu.cpu_write_register(3, 0xff);
        assert_eq!(ppu.cpu_read_register(4), 0x55);
        assert_eq!(ppu.oam_address(), 0xff);

        ppu.cpu_write_register(3, 0x02);
        ppu.cpu_write_register(4, 0xff);
        assert_eq!(ppu.oam()[2], OAM_ATTRIBUTE_MASK);
        ppu.cpu_write_register(3, 0x02);
        assert_eq!(ppu.cpu_read_register(4), OAM_ATTRIBUTE_MASK);

        ppu.cpu_write_register(1, 0x08);
        assert!(ppu.rendering_enabled());
        ppu.cpu_write_register(1, 0x00);
        assert!(!ppu.rendering_enabled());
    }

    #[test]
    fn write_only_register_reads_return_the_distinct_ppu_io_latch() {
        for register in [0_u8, 1, 3, 5, 6] {
            let mut ppu = Ppu::new(&cartridge(0, false));
            ppu.cpu_write_register(2, 0xa7);
            assert_eq!(ppu.cpu_read_register(register), 0xa7);
            assert_eq!(ppu.io_latch(), 0xa7);
        }
    }

    #[test]
    fn vblank_and_control_drive_a_logical_nmi_output() {
        let mut ppu = Ppu::new(&cartridge(0, false));
        ppu.apply_event(PpuEvent::VblankStarted);
        assert!(!ppu.nmi_output());
        ppu.cpu_write_register(0, CONTROL_NMI_ENABLE);
        assert!(ppu.nmi_output());
        ppu.cpu_write_register(0, 0);
        assert!(!ppu.nmi_output());
        ppu.cpu_write_register(0, CONTROL_NMI_ENABLE);
        assert!(ppu.nmi_output());
        let _ = ppu.cpu_read_register(2);
        assert!(!ppu.nmi_output());
        ppu.apply_event(PpuEvent::VblankEnded);
        assert_eq!(ppu.status() & STATUS_VBLANK, 0);
    }

    #[test]
    fn reset_clears_resettable_registers_but_preserves_memory_v_and_status() {
        let mut ppu = Ppu::new(&cartridge(0, true));
        ppu.cpu_write_register(0, 0x87);
        ppu.cpu_write_register(1, 0x19);
        ppu.cpu_write_register(3, 0x20);
        ppu.cpu_write_register(4, 0x5a);
        set_address(&mut ppu, 0x0010);
        ppu.cpu_write_register(7, 0xa5);
        set_address(&mut ppu, 0x0010);
        let _ = ppu.cpu_read_register(7);
        ppu.cpu_write_register(5, 0xff);
        ppu.apply_event(PpuEvent::VblankStarted);
        let preserved_v = ppu.vram_address();

        ppu.reset_registers();

        assert_eq!(ppu.control(), 0);
        assert_eq!(ppu.mask(), 0);
        assert_eq!(ppu.temporary_address(), 0);
        assert_eq!(ppu.fine_x(), 0);
        assert!(!ppu.write_toggle());
        assert_eq!(ppu.data_buffer(), 0);
        assert_eq!(ppu.vram_address(), preserved_v);
        assert_ne!(ppu.status() & STATUS_VBLANK, 0);
        assert_eq!(ppu.oam_address(), 0x21);
        assert_eq!(ppu.oam()[0x20], 0x5a);
        assert_eq!(ppu.peek_memory(0x0010), 0xa5);
    }
}
