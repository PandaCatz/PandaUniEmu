use crate::{PpuEvent, PpuPosition};

const PALETTE_SIZE: usize = 32;
const OAM_SIZE: usize = 256;

const STATUS_VBLANK: u8 = 0x80;
const CONTROL_NMI_ENABLE: u8 = 0x80;
const CONTROL_INCREMENT_32: u8 = 0x04;
const MASK_RENDERING: u8 = 0x18;
const MASK_GRAYSCALE: u8 = 0x01;
const OAM_ATTRIBUTE_MASK: u8 = 0xe3;
const BACKGROUND_PATTERN_TABLE: u8 = 0x10;
const HORIZONTAL_SCROLL_BITS: u16 = 0x041f;
const VERTICAL_SCROLL_BITS: u16 = 0x7be0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PpuFetchKind {
    Nametable,
    Attribute,
    PatternLow,
    PatternHigh,
    DummyNametable,
}

/// One half of a two-dot background fetch. Sprite-region bus activity is not
/// represented until the sprite pipeline milestone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PpuBusPhase {
    Address,
    Read,
}

/// Observable background-fetch activity for one PPU dot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PpuBusAccess {
    pub position: PpuPosition,
    pub kind: PpuFetchKind,
    pub phase: PpuBusPhase,
    pub address: u16,
    pub value: Option<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingFetch {
    kind: PpuFetchKind,
    address: u16,
    attribute_shift: u8,
}

pub(crate) trait PpuBus {
    fn observe_address(&mut self, address: u16);
    fn peek(&self, address: u16) -> u8;
    fn read(&mut self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}

/// Deterministic NES PPU register and address-space shell.
///
/// This models CPU-visible register side effects, dot-timed background fetches,
/// and scroll transfers. Pixels, sprites, DMA, analog open-bus decay, the
/// dot-exact PPUSTATUS/VBlank race window, PPUMASK's hardware propagation delay,
/// and reset write-ignore warmup are later milestones.
///
/// During rendering, `$2007` models only the documented coarse-X-plus-Y scroll
/// increment. Contended read data/buffering and the hardware's unpredictable
/// write destination are deliberately suppressed and unclaimed. A CPU access
/// coincident with a scheduled scroll increment may currently increment twice;
/// exact CPU/PPU collision timing is also deferred.
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
    palette: [u8; PALETTE_SIZE],
    next_tile_id: u8,
    next_attribute: u8,
    next_pattern_low: u8,
    next_pattern_high: u8,
    pattern_shift_low: u16,
    pattern_shift_high: u16,
    attribute_shift_low: u16,
    attribute_shift_high: u16,
    pending_fetch: Option<PendingFetch>,
    cpu_position: PpuPosition,
}

impl Default for Ppu {
    fn default() -> Self {
        Self::new()
    }
}

impl Ppu {
    #[must_use]
    pub const fn new() -> Self {
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
            palette: [0; PALETTE_SIZE],
            next_tile_id: 0,
            next_attribute: 0,
            next_pattern_low: 0,
            next_pattern_high: 0,
            pattern_shift_low: 0,
            pattern_shift_high: 0,
            attribute_shift_low: 0,
            attribute_shift_high: 0,
            pending_fetch: None,
            cpu_position: PpuPosition {
                frame: 0,
                scanline: 0,
                dot: 0,
                odd_frame: false,
            },
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
    /// Returns the current PPUMASK rendering bits. The automatic fetch schedule
    /// currently applies changes immediately; hardware's 3-4-dot propagation
    /// delay remains explicit future work.
    pub const fn rendering_enabled(&self) -> bool {
        self.mask & MASK_RENDERING != 0
    }

    #[must_use]
    pub const fn nmi_output(&self) -> bool {
        self.status & STATUS_VBLANK != 0 && self.control & CONTROL_NMI_ENABLE != 0
    }

    #[must_use]
    pub fn peek_palette(&self, address: u16) -> u8 {
        self.palette[Self::palette_index(address & 0x3fff)]
    }

    #[must_use]
    pub const fn background_pattern_shifters(&self) -> (u16, u16) {
        (self.pattern_shift_low, self.pattern_shift_high)
    }

    #[must_use]
    pub const fn background_attribute_shifters(&self) -> (u16, u16) {
        (self.attribute_shift_low, self.attribute_shift_high)
    }

    fn peek_memory(&self, bus: &impl PpuBus, address: u16) -> u8 {
        let address = address & 0x3fff;
        match address {
            0x0000..=0x3eff => bus.peek(address),
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
        self.next_tile_id = 0;
        self.next_attribute = 0;
        self.next_pattern_low = 0;
        self.next_pattern_high = 0;
        self.pattern_shift_low = 0;
        self.pattern_shift_high = 0;
        self.attribute_shift_low = 0;
        self.attribute_shift_high = 0;
        self.pending_fetch = None;
        self.cpu_position = PpuPosition::default();
    }

    pub(crate) fn cpu_read_register(&mut self, bus: &mut impl PpuBus, register: u8) -> u8 {
        let value = match register & 0x07 {
            2 => {
                let value = (self.status & 0xe0) | (self.io_latch & 0x1f);
                self.status &= !STATUS_VBLANK;
                self.write_toggle = false;
                value
            }
            4 => self.oam[usize::from(self.oam_address)],
            7 if self.cpu_vram_is_contended() => {
                self.increment_cpu_vram_address();
                self.io_latch
            }
            7 => self.read_data(bus),
            _ => self.io_latch,
        };
        self.io_latch = value;
        value
    }

    pub(crate) fn cpu_write_register(&mut self, bus: &mut impl PpuBus, register: u8, value: u8) {
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
                if !self.cpu_vram_is_contended() {
                    self.write_memory(bus, self.vram_address, value);
                }
                self.increment_cpu_vram_address();
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

    fn read_data(&mut self, bus: &mut impl PpuBus) -> u8 {
        let address = self.vram_address & 0x3fff;
        let value = if address >= 0x3f00 {
            let color_mask = if self.mask & MASK_GRAYSCALE != 0 {
                0x30
            } else {
                0x3f
            };
            let palette_value = self.peek_memory(bus, address) & color_mask;
            let shadow_address = address - 0x1000;
            bus.observe_address(shadow_address);
            self.data_buffer = bus.read(shadow_address);
            (self.io_latch & 0xc0) | palette_value
        } else {
            let buffered = self.data_buffer;
            bus.observe_address(address);
            self.data_buffer = bus.read(address);
            buffered
        };
        self.increment_cpu_vram_address();
        value
    }

    fn increment_cpu_vram_address(&mut self) {
        if self.cpu_vram_is_contended() {
            self.increment_horizontal();
            self.increment_vertical();
            return;
        }
        let increment = if self.control & CONTROL_INCREMENT_32 != 0 {
            32
        } else {
            1
        };
        self.vram_address = self.vram_address.wrapping_add(increment) & 0x7fff;
    }

    const fn cpu_vram_is_contended(&self) -> bool {
        self.rendering_enabled()
            && (self.cpu_position.scanline < 240 || self.cpu_position.scanline == 261)
    }

    pub(crate) const fn sync_cpu_position(&mut self, position: PpuPosition) {
        self.cpu_position = position;
    }

    pub(crate) fn clock_dot(
        &mut self,
        bus: &mut impl PpuBus,
        position: PpuPosition,
    ) -> Option<PpuBusAccess> {
        let rendering_scanline = position.scanline < 240 || position.scanline == 261;
        if !self.rendering_enabled() || !rendering_scanline {
            self.pending_fetch = None;
            return None;
        }

        if matches!(position.dot, 2..=257 | 322..=337) {
            self.shift_background();
        }
        if (position.dot >= 9 && position.dot <= 257 && position.dot % 8 == 1)
            || matches!(position.dot, 329 | 337)
        {
            self.reload_background();
        }

        let access = if let Some(pending) = self.pending_fetch.take() {
            let value = bus.read(pending.address);
            self.complete_fetch(pending, value);
            Some(PpuBusAccess {
                position,
                kind: pending.kind,
                phase: PpuBusPhase::Read,
                address: pending.address,
                value: Some(value),
            })
        } else if let Some((kind, address, attribute_shift)) = self.fetch_start(position.dot) {
            bus.observe_address(address);
            self.pending_fetch = Some(PendingFetch {
                kind,
                address,
                attribute_shift,
            });
            Some(PpuBusAccess {
                position,
                kind,
                phase: PpuBusPhase::Address,
                address,
                value: None,
            })
        } else {
            None
        };

        if (position.dot <= 256 && position.dot >= 8 && position.dot.is_multiple_of(8))
            || matches!(position.dot, 328 | 336)
        {
            self.increment_horizontal();
        }
        if position.dot == 256 {
            self.increment_vertical();
        }
        if position.dot == 257 {
            self.vram_address = (self.vram_address & !HORIZONTAL_SCROLL_BITS)
                | (self.temporary_address & HORIZONTAL_SCROLL_BITS);
        }
        if position.scanline == 261 && matches!(position.dot, 280..=304) {
            self.vram_address = (self.vram_address & !VERTICAL_SCROLL_BITS)
                | (self.temporary_address & VERTICAL_SCROLL_BITS);
        }

        access
    }

    fn fetch_start(&self, dot: u16) -> Option<(PpuFetchKind, u16, u8)> {
        if matches!(dot, 337 | 339) {
            return Some((PpuFetchKind::DummyNametable, self.nametable_address(), 0));
        }
        if !((1..=256).contains(&dot) || (321..=336).contains(&dot)) {
            return None;
        }
        match (dot - 1) % 8 {
            0 => Some((PpuFetchKind::Nametable, self.nametable_address(), 0)),
            2 => Some((
                PpuFetchKind::Attribute,
                self.attribute_address(),
                self.attribute_shift(),
            )),
            4 => Some((PpuFetchKind::PatternLow, self.pattern_address(0), 0)),
            6 => Some((PpuFetchKind::PatternHigh, self.pattern_address(8), 0)),
            _ => None,
        }
    }

    fn complete_fetch(&mut self, pending: PendingFetch, value: u8) {
        match pending.kind {
            PpuFetchKind::Nametable => self.next_tile_id = value,
            PpuFetchKind::Attribute => {
                self.next_attribute = (value >> pending.attribute_shift) & 0x03;
            }
            PpuFetchKind::PatternLow => self.next_pattern_low = value,
            PpuFetchKind::PatternHigh => self.next_pattern_high = value,
            PpuFetchKind::DummyNametable => {}
        }
    }

    const fn nametable_address(&self) -> u16 {
        0x2000 | (self.vram_address & 0x0fff)
    }

    const fn attribute_address(&self) -> u16 {
        0x23c0
            | (self.vram_address & 0x0c00)
            | ((self.vram_address >> 4) & 0x0038)
            | ((self.vram_address >> 2) & 0x0007)
    }

    const fn attribute_shift(&self) -> u8 {
        (((self.vram_address >> 4) & 4) | (self.vram_address & 2)) as u8
    }

    const fn pattern_address(&self, plane: u16) -> u16 {
        let table = if self.control & BACKGROUND_PATTERN_TABLE != 0 {
            0x1000
        } else {
            0
        };
        table | (self.next_tile_id as u16) << 4 | ((self.vram_address >> 12) & 0x0007) | plane
    }

    fn increment_horizontal(&mut self) {
        if self.vram_address & 0x001f == 31 {
            self.vram_address &= !0x001f;
            self.vram_address ^= 0x0400;
        } else {
            self.vram_address += 1;
        }
    }

    fn increment_vertical(&mut self) {
        if self.vram_address & 0x7000 != 0x7000 {
            self.vram_address += 0x1000;
            return;
        }
        self.vram_address &= !0x7000;
        let mut coarse_y = (self.vram_address & 0x03e0) >> 5;
        if coarse_y == 29 {
            coarse_y = 0;
            self.vram_address ^= 0x0800;
        } else if coarse_y == 31 {
            coarse_y = 0;
        } else {
            coarse_y += 1;
        }
        self.vram_address = (self.vram_address & !0x03e0) | (coarse_y << 5);
    }

    fn shift_background(&mut self) {
        self.pattern_shift_low <<= 1;
        self.pattern_shift_high <<= 1;
        self.attribute_shift_low <<= 1;
        self.attribute_shift_high <<= 1;
    }

    fn reload_background(&mut self) {
        self.pattern_shift_low =
            (self.pattern_shift_low & 0xff00) | u16::from(self.next_pattern_low);
        self.pattern_shift_high =
            (self.pattern_shift_high & 0xff00) | u16::from(self.next_pattern_high);
        self.attribute_shift_low = (self.attribute_shift_low & 0xff00)
            | if self.next_attribute & 1 != 0 {
                0xff
            } else {
                0
            };
        self.attribute_shift_high = (self.attribute_shift_high & 0xff00)
            | if self.next_attribute & 2 != 0 {
                0xff
            } else {
                0
            };
    }

    fn write_memory(&mut self, bus: &mut impl PpuBus, address: u16, value: u8) {
        let address = address & 0x3fff;
        bus.observe_address(address);
        match address {
            0x0000..=0x3eff => bus.write(address, value),
            0x3f00..=0x3fff => self.palette[Self::palette_index(address)] = value & 0x3f,
            _ => unreachable!("14-bit PPU address mapping is exhaustive"),
        }
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
    use crate::{NesCartridge, nrom_bus::NromCpuBus};

    const TEST_CHR_SIZE: usize = 8 * 1024;

    fn cartridge(flags6: u8, chr_ram: bool) -> NesCartridge {
        let chr_len = if chr_ram { 0 } else { TEST_CHR_SIZE };
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

    fn devices(flags6: u8, chr_ram: bool) -> (Ppu, NromCpuBus) {
        (Ppu::new(), NromCpuBus::new(cartridge(flags6, chr_ram)))
    }

    fn set_address(ppu: &mut Ppu, bus: &mut NromCpuBus, address: u16) {
        ppu.cpu_write_register(bus, 6, (address >> 8) as u8);
        ppu.cpu_write_register(bus, 6, address as u8);
    }

    fn position(scanline: u16, dot: u16) -> PpuPosition {
        PpuPosition {
            frame: 0,
            scanline,
            dot,
            odd_frame: false,
        }
    }

    #[test]
    fn scroll_address_and_status_share_the_hardware_write_toggle() {
        let (mut ppu, mut bus) = devices(0, false);
        ppu.cpu_write_register(&mut bus, 0, 0x03);
        ppu.cpu_write_register(&mut bus, 5, 0xad);
        assert_eq!(ppu.temporary_address() & 0x0c1f, 0x0c15);
        assert_eq!(ppu.fine_x(), 5);
        assert!(ppu.write_toggle());

        ppu.cpu_write_register(&mut bus, 5, 0x76);
        assert_eq!(ppu.temporary_address() & 0x73e0, 0x61c0);
        assert!(!ppu.write_toggle());

        ppu.cpu_write_register(&mut bus, 6, 0xff);
        assert!(ppu.write_toggle());
        ppu.apply_event(PpuEvent::VblankStarted);
        ppu.cpu_write_register(&mut bus, 2, 0x1b);
        assert_eq!(ppu.cpu_read_register(&mut bus, 2), 0x9b);
        assert_eq!(ppu.status() & STATUS_VBLANK, 0);
        assert!(!ppu.write_toggle());

        ppu.cpu_write_register(&mut bus, 6, 0xff);
        ppu.cpu_write_register(&mut bus, 6, 0xff);
        assert_eq!(ppu.vram_address(), 0x3fff);
    }

    #[test]
    fn data_reads_buffer_non_palette_and_refill_from_palette_shadow() {
        let (mut ppu, mut bus) = devices(0, true);
        set_address(&mut ppu, &mut bus, 0x0004);
        ppu.cpu_write_register(&mut bus, 7, 0xa5);
        set_address(&mut ppu, &mut bus, 0x0004);
        assert_eq!(ppu.cpu_read_register(&mut bus, 7), 0);
        assert_eq!(ppu.cpu_read_register(&mut bus, 7), 0xa5);

        set_address(&mut ppu, &mut bus, 0x2f00);
        ppu.cpu_write_register(&mut bus, 7, 0x2c);
        set_address(&mut ppu, &mut bus, 0x3f00);
        ppu.cpu_write_register(&mut bus, 7, 0x15);
        set_address(&mut ppu, &mut bus, 0x3f00);
        ppu.cpu_write_register(&mut bus, 2, 0xc0);
        assert_eq!(ppu.cpu_read_register(&mut bus, 7), 0xd5);
        assert_eq!(ppu.data_buffer(), 0x2c);

        ppu.cpu_write_register(&mut bus, 1, MASK_GRAYSCALE);
        set_address(&mut ppu, &mut bus, 0x3f00);
        ppu.cpu_write_register(&mut bus, 2, 0xc0);
        assert_eq!(ppu.cpu_read_register(&mut bus, 7), 0xd0);
        assert_eq!(ppu.peek_palette(0x3f00), 0x15);
    }

    #[test]
    fn data_increment_wraps_and_chr_rom_writes_are_nonfatal_and_ignored() {
        let (mut rom_ppu, mut bus) = devices(0, false);
        set_address(&mut rom_ppu, &mut bus, 0x1fff);
        let before = bus.peek_ppu(0x1fff);
        rom_ppu.cpu_write_register(&mut bus, 7, before.wrapping_add(1));
        assert_eq!(bus.peek_ppu(0x1fff), before);
        assert_eq!(rom_ppu.vram_address(), 0x2000);

        rom_ppu.cpu_write_register(&mut bus, 0, CONTROL_INCREMENT_32);
        set_address(&mut rom_ppu, &mut bus, 0x3fff);
        rom_ppu.cpu_write_register(&mut bus, 7, 0x22);
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
            let (mut ppu, mut bus) = devices(flags6, false);
            for (table, value) in [0x11_u8, 0x22, 0x33, 0x44].into_iter().enumerate() {
                let address = 0x2000 + (table as u16) * 0x400 + 0x3ff;
                ppu.write_memory(&mut bus, address, value);
            }
            for table in 0..4 {
                let expected_table = physical[table];
                let last_writer = physical
                    .iter()
                    .rposition(|candidate| *candidate == expected_table)
                    .expect("physical table is present");
                let address = 0x2000 + (table as u16) * 0x400 + 0x3ff;
                assert_eq!(bus.peek_ppu(address), 0x11 + (last_writer as u8) * 0x11);
            }
            assert_eq!(bus.peek_ppu(0x3000), bus.peek_ppu(0x2000));
            assert_eq!(bus.peek_ppu(0x3eff), bus.peek_ppu(0x2eff));
        }
    }

    #[test]
    fn palette_repeats_and_universal_background_entries_alias_both_ways() {
        let (mut ppu, mut bus) = devices(0, false);
        ppu.write_memory(&mut bus, 0x3f10, 0xff);
        assert_eq!(ppu.peek_palette(0x3f00), 0x3f);
        assert_eq!(ppu.peek_palette(0x3f20), 0x3f);
        ppu.write_memory(&mut bus, 0x3f04, 0x2a);
        assert_eq!(ppu.peek_palette(0x3f14), 0x2a);
        assert_eq!(ppu.peek_palette(0x3ff4), 0x2a);
    }

    #[test]
    fn oam_and_mask_have_their_basic_register_semantics() {
        let (mut ppu, mut bus) = devices(0, false);
        ppu.cpu_write_register(&mut bus, 3, 0xff);
        ppu.cpu_write_register(&mut bus, 4, 0x55);
        ppu.cpu_write_register(&mut bus, 4, 0xaa);
        assert_eq!(ppu.oam()[0xff], 0x55);
        assert_eq!(ppu.oam()[0], 0xaa);
        assert_eq!(ppu.oam_address(), 1);
        ppu.cpu_write_register(&mut bus, 3, 0xff);
        assert_eq!(ppu.cpu_read_register(&mut bus, 4), 0x55);
        assert_eq!(ppu.oam_address(), 0xff);

        ppu.cpu_write_register(&mut bus, 3, 0x02);
        ppu.cpu_write_register(&mut bus, 4, 0xff);
        assert_eq!(ppu.oam()[2], OAM_ATTRIBUTE_MASK);
        ppu.cpu_write_register(&mut bus, 3, 0x02);
        assert_eq!(ppu.cpu_read_register(&mut bus, 4), OAM_ATTRIBUTE_MASK);

        ppu.cpu_write_register(&mut bus, 1, 0x08);
        assert!(ppu.rendering_enabled());
        ppu.cpu_write_register(&mut bus, 1, 0x00);
        assert!(!ppu.rendering_enabled());
    }

    #[test]
    fn write_only_register_reads_return_the_distinct_ppu_io_latch() {
        for register in [0_u8, 1, 3, 5, 6] {
            let (mut ppu, mut bus) = devices(0, false);
            ppu.cpu_write_register(&mut bus, 2, 0xa7);
            assert_eq!(ppu.cpu_read_register(&mut bus, register), 0xa7);
            assert_eq!(ppu.io_latch(), 0xa7);
        }
    }

    #[test]
    fn vblank_and_control_drive_a_logical_nmi_output() {
        let (mut ppu, mut bus) = devices(0, false);
        ppu.apply_event(PpuEvent::VblankStarted);
        assert!(!ppu.nmi_output());
        ppu.cpu_write_register(&mut bus, 0, CONTROL_NMI_ENABLE);
        assert!(ppu.nmi_output());
        ppu.cpu_write_register(&mut bus, 0, 0);
        assert!(!ppu.nmi_output());
        ppu.cpu_write_register(&mut bus, 0, CONTROL_NMI_ENABLE);
        assert!(ppu.nmi_output());
        let _ = ppu.cpu_read_register(&mut bus, 2);
        assert!(!ppu.nmi_output());
        ppu.apply_event(PpuEvent::VblankEnded);
        assert_eq!(ppu.status() & STATUS_VBLANK, 0);
    }

    #[test]
    fn reset_clears_resettable_registers_but_preserves_memory_v_and_status() {
        let (mut ppu, mut bus) = devices(0, true);
        ppu.cpu_write_register(&mut bus, 0, 0x87);
        ppu.cpu_write_register(&mut bus, 1, 0x19);
        ppu.cpu_write_register(&mut bus, 3, 0x20);
        ppu.cpu_write_register(&mut bus, 4, 0x5a);
        ppu.sync_cpu_position(position(240, 0));
        set_address(&mut ppu, &mut bus, 0x0010);
        ppu.cpu_write_register(&mut bus, 7, 0xa5);
        set_address(&mut ppu, &mut bus, 0x0010);
        let _ = ppu.cpu_read_register(&mut bus, 7);
        ppu.cpu_write_register(&mut bus, 5, 0xff);
        ppu.apply_event(PpuEvent::VblankStarted);
        ppu.mask = MASK_RENDERING;
        assert!(ppu.clock_dot(&mut bus, position(0, 1)).is_some());
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
        assert_eq!(bus.peek_ppu(0x0010), 0xa5);
        assert_eq!(ppu.background_pattern_shifters(), (0, 0));
        assert_eq!(ppu.background_attribute_shifters(), (0, 0));
        assert_eq!(ppu.clock_dot(&mut bus, position(0, 2)), None);
    }

    #[test]
    fn visible_tile_fetch_matches_numeric_address_and_phase_oracle() {
        let (mut ppu, mut bus) = devices(0, true);
        ppu.mask = MASK_RENDERING;
        ppu.control = BACKGROUND_PATTERN_TABLE;
        ppu.vram_address = 0x73a5;
        PpuBus::write(&mut bus, 0x23a5, 0x2d);
        PpuBus::write(&mut bus, 0x23f9, 0xe4);
        PpuBus::write(&mut bus, 0x12d7, 0xaa);
        PpuBus::write(&mut bus, 0x12df, 0x55);

        let accesses: Vec<_> = (1..=8)
            .map(|dot| {
                ppu.clock_dot(&mut bus, position(0, dot))
                    .expect("each fetch half-cycle is observable")
            })
            .collect();
        let actual: Vec<_> = accesses
            .iter()
            .map(|access| (access.kind, access.phase, access.address, access.value))
            .collect();
        assert_eq!(
            actual,
            vec![
                (PpuFetchKind::Nametable, PpuBusPhase::Address, 0x23a5, None),
                (
                    PpuFetchKind::Nametable,
                    PpuBusPhase::Read,
                    0x23a5,
                    Some(0x2d)
                ),
                (PpuFetchKind::Attribute, PpuBusPhase::Address, 0x23f9, None),
                (
                    PpuFetchKind::Attribute,
                    PpuBusPhase::Read,
                    0x23f9,
                    Some(0xe4)
                ),
                (PpuFetchKind::PatternLow, PpuBusPhase::Address, 0x12d7, None),
                (
                    PpuFetchKind::PatternLow,
                    PpuBusPhase::Read,
                    0x12d7,
                    Some(0xaa)
                ),
                (
                    PpuFetchKind::PatternHigh,
                    PpuBusPhase::Address,
                    0x12df,
                    None
                ),
                (
                    PpuFetchKind::PatternHigh,
                    PpuBusPhase::Read,
                    0x12df,
                    Some(0x55)
                ),
            ]
        );
        assert_eq!(ppu.vram_address(), 0x73a6);

        let reload = ppu
            .clock_dot(&mut bus, position(0, 9))
            .expect("next nametable address phase starts at dot 9");
        assert_eq!(reload.address, 0x23a6);
        assert_eq!(ppu.background_pattern_shifters(), (0x00aa, 0x0055));
    }

    #[test]
    fn continuous_dot_249_through_257_trace_matches_fetch_and_scroll_oracle() {
        let (mut ppu, mut bus) = devices(0, true);
        ppu.mask = MASK_RENDERING;
        ppu.vram_address = 0x73bf;
        ppu.temporary_address = 0x56f2;
        PpuBus::write(&mut bus, 0x23bf, 0x2d);
        PpuBus::write(&mut bus, 0x23ff, 0xe4);
        PpuBus::write(&mut bus, 0x02d7, 0xaa);
        PpuBus::write(&mut bus, 0x02df, 0x55);

        let accesses: Vec<_> = (249..=256)
            .map(|dot| {
                ppu.clock_dot(&mut bus, position(0, dot))
                    .expect("the last visible tile fetch occupies every dot")
            })
            .collect();
        assert_eq!(accesses[0].address, 0x23bf);
        assert_eq!(accesses[7].kind, PpuFetchKind::PatternHigh);
        assert_eq!(accesses[7].phase, PpuBusPhase::Read);
        assert_eq!(accesses[7].address, 0x02df);
        assert_eq!(accesses[7].value, Some(0x55));
        assert_eq!(ppu.vram_address(), 0x0c00);
        assert_eq!(ppu.clock_dot(&mut bus, position(0, 257)), None);
        assert_eq!(ppu.vram_address(), 0x0c12);
        assert_eq!(ppu.background_pattern_shifters(), (0x00aa, 0x0055));
        assert_eq!(ppu.clock_dot(&mut bus, position(261, 280)), None);
        assert_eq!(ppu.vram_address(), 0x56f2);
    }

    #[test]
    fn horizontal_and_vertical_helpers_cover_every_hardware_wrap_branch() {
        let mut ppu = Ppu::new();

        ppu.vram_address = 0x001e;
        ppu.increment_horizontal();
        assert_eq!(ppu.vram_address, 0x001f);
        ppu.increment_horizontal();
        assert_eq!(ppu.vram_address, 0x0400);

        for (before, after) in [
            (0x6000, 0x7000),
            (0x7380, 0x03a0),
            (0x73a0, 0x0800),
            (0x7bc0, 0x0be0),
            (0x7be0, 0x0800),
        ] {
            ppu.vram_address = before;
            ppu.increment_vertical();
            assert_eq!(ppu.vram_address, after, "vertical wrap from ${before:04X}");
        }
    }

    #[test]
    fn attribute_quadrants_and_pattern_extremes_have_exact_addresses() {
        for (vram_address, shift, expected) in [
            (0x0000, 0, (0x0000, 0x0000)),
            (0x0002, 2, (0x00ff, 0x0000)),
            (0x0040, 4, (0x0000, 0x00ff)),
            (0x0042, 6, (0x00ff, 0x00ff)),
        ] {
            let (mut ppu, mut bus) = devices(0, true);
            ppu.mask = MASK_RENDERING;
            ppu.vram_address = vram_address;
            assert_eq!(ppu.attribute_shift(), shift);
            PpuBus::write(&mut bus, ppu.attribute_address(), 0xe4);
            for dot in 1..=9 {
                let _ = ppu.clock_dot(&mut bus, position(0, dot));
            }
            assert_eq!(ppu.background_attribute_shifters(), expected);
        }

        let mut ppu = Ppu::new();
        ppu.next_tile_id = 0xff;
        ppu.vram_address = 0x7000;
        ppu.control = 0;
        assert_eq!(ppu.pattern_address(0), 0x0ff7);
        assert_eq!(ppu.pattern_address(8), 0x0fff);
        ppu.control = BACKGROUND_PATTERN_TABLE;
        assert_eq!(ppu.pattern_address(0), 0x1ff7);
        assert_eq!(ppu.pattern_address(8), 0x1fff);
        ppu.next_tile_id = 0;
        assert_eq!(ppu.pattern_address(0), 0x1007);
    }

    #[test]
    fn pre_render_vertical_copy_repeats_and_observes_t_changes_through_dot_304() {
        let (mut ppu, mut bus) = devices(0, true);
        ppu.mask = MASK_RENDERING;
        ppu.vram_address = 0x0412;
        ppu.temporary_address = 0x12e0;
        ppu.clock_dot(&mut bus, position(261, 280));
        assert_eq!(ppu.vram_address, 0x16f2);

        ppu.temporary_address = 0x6b40;
        ppu.clock_dot(&mut bus, position(261, 304));
        assert_eq!(ppu.vram_address, 0x6f52);
        ppu.temporary_address = 0x0000;
        ppu.clock_dot(&mut bus, position(261, 305));
        assert_eq!(ppu.vram_address, 0x6f52);
    }

    #[test]
    fn prefetch_and_dummy_fetches_match_numeric_dot_oracle() {
        let (mut ppu, mut bus) = devices(0, true);
        ppu.mask = MASK_RENDERING;
        ppu.control = BACKGROUND_PATTERN_TABLE;
        ppu.vram_address = 0x56f2;
        for (address, value) in [
            (0x26f2, 0x2d),
            (0x26f3, 0x7a),
            (0x27ec, 0),
            (0x12d5, 1),
            (0x12dd, 2),
            (0x17a5, 3),
            (0x17ad, 4),
        ] {
            PpuBus::write(&mut bus, address, value);
        }

        let accesses: Vec<_> = (321..=340)
            .map(|dot| {
                ppu.clock_dot(&mut bus, position(0, dot))
                    .expect("prefetch and dummy phases cover every dot")
            })
            .collect();
        let addresses: Vec<_> = accesses.iter().map(|access| access.address).collect();
        assert_eq!(
            addresses,
            vec![
                0x26f2, 0x26f2, 0x27ec, 0x27ec, 0x12d5, 0x12d5, 0x12dd, 0x12dd, 0x26f3, 0x26f3,
                0x27ec, 0x27ec, 0x17a5, 0x17a5, 0x17ad, 0x17ad, 0x26f4, 0x26f4, 0x26f4, 0x26f4,
            ]
        );
        assert_eq!(ppu.vram_address(), 0x56f4);
        assert_eq!(accesses[16].kind, PpuFetchKind::DummyNametable);
        assert_eq!(accesses[18].kind, PpuFetchKind::DummyNametable);
    }

    #[test]
    fn odd_frame_skip_still_completes_last_dummy_fetch_on_next_frame_dot_zero() {
        let (mut ppu, mut bus) = devices(0, true);
        ppu.mask = MASK_RENDERING;
        ppu.vram_address = 0x56f4;
        PpuBus::write(&mut bus, 0x26f4, 0xa5);

        let address = ppu
            .clock_dot(&mut bus, position(261, 339))
            .expect("dummy address phase starts");
        let mut next_frame = position(0, 0);
        next_frame.frame = 1;
        next_frame.odd_frame = true;
        let read = ppu
            .clock_dot(&mut bus, next_frame)
            .expect("skipped dot 340 leaves the data phase for dot zero");
        assert_eq!(address.phase, PpuBusPhase::Address);
        assert_eq!(read.phase, PpuBusPhase::Read);
        assert_eq!(read.position, next_frame);
        assert_eq!(read.value, Some(0xa5));
    }

    #[test]
    fn rendering_gate_and_contended_ppudata_increment_are_explicit() {
        let (mut ppu, mut bus) = devices(0, true);
        ppu.vram_address = 0x73bf;
        assert_eq!(ppu.clock_dot(&mut bus, position(0, 256)), None);
        assert_eq!(ppu.vram_address(), 0x73bf);

        ppu.cpu_write_register(&mut bus, 1, 0x10);
        ppu.sync_cpu_position(position(0, 64));
        let bus_before_write = bus.clone();
        ppu.cpu_write_register(&mut bus, 7, 0x44);
        assert_eq!(ppu.vram_address(), 0x0c00);
        assert_eq!(bus, bus_before_write);

        ppu.vram_address = 0x73bf;
        ppu.io_latch = 0xa5;
        ppu.data_buffer = 0x5a;
        let bus_before_read = bus.clone();
        assert_eq!(ppu.cpu_read_register(&mut bus, 7), 0xa5);
        assert_eq!(ppu.vram_address(), 0x0c00);
        assert_eq!(ppu.data_buffer(), 0x5a);
        assert_eq!(bus, bus_before_read);
        assert!(ppu.clock_dot(&mut bus, position(0, 1)).is_some());
    }

    #[test]
    fn either_rendering_enable_bit_produces_the_same_background_bus_schedule() {
        let mut traces = Vec::new();
        for mask in [0x08, 0x10, 0x18] {
            let (mut ppu, mut bus) = devices(0, true);
            ppu.mask = mask;
            traces.push(
                (1..=8)
                    .map(|dot| {
                        let access = ppu
                            .clock_dot(&mut bus, position(0, dot))
                            .expect("rendering fetch phase exists");
                        (access.kind, access.phase, access.address)
                    })
                    .collect::<Vec<_>>(),
            );
        }
        assert_eq!(traces[0], traces[1]);
        assert_eq!(traces[1], traces[2]);
    }

    #[test]
    fn background_trace_is_idle_in_sprite_and_non_render_scanline_windows() {
        let (mut ppu, mut bus) = devices(0, true);
        ppu.mask = MASK_RENDERING;
        assert_eq!(ppu.clock_dot(&mut bus, position(239, 0)), None);
        assert!(ppu.clock_dot(&mut bus, position(239, 1)).is_some());

        ppu.pending_fetch = None;
        for dot in 257..=320 {
            assert_eq!(ppu.clock_dot(&mut bus, position(239, dot)), None);
        }
        assert!(ppu.clock_dot(&mut bus, position(239, 321)).is_some());

        ppu.pending_fetch = None;
        for dot in [0, 1, 256, 321, 337, 340] {
            assert_eq!(ppu.clock_dot(&mut bus, position(240, dot)), None);
        }
        assert!(ppu.clock_dot(&mut bus, position(261, 1)).is_some());
    }
}
