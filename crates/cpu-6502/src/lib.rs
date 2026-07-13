#![forbid(unsafe_code)]

use std::error::Error;
use std::fmt::{Display, Formatter};

pub const FLAG_CARRY: u8 = 0x01;
pub const FLAG_ZERO: u8 = 0x02;
pub const FLAG_INTERRUPT_DISABLE: u8 = 0x04;
pub const FLAG_DECIMAL: u8 = 0x08;
pub const FLAG_BREAK: u8 = 0x10;
pub const FLAG_UNUSED: u8 = 0x20;
pub const FLAG_OVERFLOW: u8 = 0x40;
pub const FLAG_NEGATIVE: u8 = 0x80;

pub trait Bus {
    fn read(&mut self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CpuState {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub pc: u16,
    pub status: u8,
    pub total_cycles: u64,
}

impl CpuState {
    #[must_use]
    pub const fn at(pc: u16) -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            sp: 0xfd,
            pc,
            status: FLAG_INTERRUPT_DISABLE | FLAG_UNUSED,
            total_cycles: 0,
        }
    }
}

impl Default for CpuState {
    fn default() -> Self {
        Self::at(0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AddressingMode {
    Implied,
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndexedIndirect,
    IndirectIndexed,
    Relative,
}

impl AddressingMode {
    #[must_use]
    pub const fn instruction_bytes(self) -> u8 {
        match self {
            Self::Implied | Self::Accumulator => 1,
            Self::Immediate
            | Self::ZeroPage
            | Self::ZeroPageX
            | Self::ZeroPageY
            | Self::IndexedIndirect
            | Self::IndirectIndexed
            | Self::Relative => 2,
            Self::Absolute | Self::AbsoluteX | Self::AbsoluteY | Self::Indirect => 3,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mnemonic {
    Adc,
    And,
    Asl,
    Bcc,
    Bcs,
    Beq,
    Bit,
    Bmi,
    Bne,
    Bpl,
    Brk,
    Bvc,
    Bvs,
    Clc,
    Cld,
    Cli,
    Clv,
    Cmp,
    Cpx,
    Cpy,
    Dcp,
    Dec,
    Dex,
    Dey,
    Eor,
    Inc,
    Inx,
    Iny,
    Isc,
    Jmp,
    Jsr,
    Lda,
    Lax,
    Ldx,
    Ldy,
    Lsr,
    Nop,
    Ora,
    Pha,
    Php,
    Pla,
    Plp,
    Rol,
    Ror,
    Rti,
    Rts,
    Sax,
    Sbc,
    Sec,
    Sed,
    Sei,
    Slo,
    Sre,
    Sta,
    Stx,
    Sty,
    Tax,
    Tay,
    Tsx,
    Txa,
    Txs,
    Tya,
    Rla,
    Rra,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Instruction {
    pub mnemonic: Mnemonic,
    pub mode: AddressingMode,
    pub base_cycles: u8,
    pub page_cross_cycle: bool,
}

impl Instruction {
    #[must_use]
    pub const fn instruction_bytes(self) -> u8 {
        self.mode.instruction_bytes()
    }
}

macro_rules! instruction {
    ($mnemonic:ident, $mode:ident, $cycles:expr) => {
        Some(Instruction {
            mnemonic: Mnemonic::$mnemonic,
            mode: AddressingMode::$mode,
            base_cycles: $cycles,
            page_cross_cycle: false,
        })
    };
    ($mnemonic:ident, $mode:ident, $cycles:expr, page) => {
        Some(Instruction {
            mnemonic: Mnemonic::$mnemonic,
            mode: AddressingMode::$mode,
            base_cycles: $cycles,
            page_cross_cycle: true,
        })
    };
}

#[must_use]
pub const fn decode(opcode: u8) -> Option<Instruction> {
    match opcode {
        0x00 => instruction!(Brk, Implied, 7),
        0x01 => instruction!(Ora, IndexedIndirect, 6),
        0x03 => instruction!(Slo, IndexedIndirect, 8),
        0x04 => instruction!(Nop, ZeroPage, 3),
        0x05 => instruction!(Ora, ZeroPage, 3),
        0x06 => instruction!(Asl, ZeroPage, 5),
        0x07 => instruction!(Slo, ZeroPage, 5),
        0x08 => instruction!(Php, Implied, 3),
        0x09 => instruction!(Ora, Immediate, 2),
        0x0a => instruction!(Asl, Accumulator, 2),
        0x0c => instruction!(Nop, Absolute, 4),
        0x0d => instruction!(Ora, Absolute, 4),
        0x0e => instruction!(Asl, Absolute, 6),
        0x0f => instruction!(Slo, Absolute, 6),
        0x10 => instruction!(Bpl, Relative, 2),
        0x11 => instruction!(Ora, IndirectIndexed, 5, page),
        0x13 => instruction!(Slo, IndirectIndexed, 8),
        0x14 => instruction!(Nop, ZeroPageX, 4),
        0x15 => instruction!(Ora, ZeroPageX, 4),
        0x16 => instruction!(Asl, ZeroPageX, 6),
        0x17 => instruction!(Slo, ZeroPageX, 6),
        0x18 => instruction!(Clc, Implied, 2),
        0x19 => instruction!(Ora, AbsoluteY, 4, page),
        0x1a => instruction!(Nop, Implied, 2),
        0x1b => instruction!(Slo, AbsoluteY, 7),
        0x1c => instruction!(Nop, AbsoluteX, 4, page),
        0x1d => instruction!(Ora, AbsoluteX, 4, page),
        0x1e => instruction!(Asl, AbsoluteX, 7),
        0x1f => instruction!(Slo, AbsoluteX, 7),
        0x20 => instruction!(Jsr, Absolute, 6),
        0x21 => instruction!(And, IndexedIndirect, 6),
        0x23 => instruction!(Rla, IndexedIndirect, 8),
        0x24 => instruction!(Bit, ZeroPage, 3),
        0x25 => instruction!(And, ZeroPage, 3),
        0x26 => instruction!(Rol, ZeroPage, 5),
        0x27 => instruction!(Rla, ZeroPage, 5),
        0x28 => instruction!(Plp, Implied, 4),
        0x29 => instruction!(And, Immediate, 2),
        0x2a => instruction!(Rol, Accumulator, 2),
        0x2c => instruction!(Bit, Absolute, 4),
        0x2d => instruction!(And, Absolute, 4),
        0x2e => instruction!(Rol, Absolute, 6),
        0x2f => instruction!(Rla, Absolute, 6),
        0x30 => instruction!(Bmi, Relative, 2),
        0x31 => instruction!(And, IndirectIndexed, 5, page),
        0x33 => instruction!(Rla, IndirectIndexed, 8),
        0x34 => instruction!(Nop, ZeroPageX, 4),
        0x35 => instruction!(And, ZeroPageX, 4),
        0x36 => instruction!(Rol, ZeroPageX, 6),
        0x37 => instruction!(Rla, ZeroPageX, 6),
        0x38 => instruction!(Sec, Implied, 2),
        0x39 => instruction!(And, AbsoluteY, 4, page),
        0x3a => instruction!(Nop, Implied, 2),
        0x3b => instruction!(Rla, AbsoluteY, 7),
        0x3c => instruction!(Nop, AbsoluteX, 4, page),
        0x3d => instruction!(And, AbsoluteX, 4, page),
        0x3e => instruction!(Rol, AbsoluteX, 7),
        0x3f => instruction!(Rla, AbsoluteX, 7),
        0x40 => instruction!(Rti, Implied, 6),
        0x41 => instruction!(Eor, IndexedIndirect, 6),
        0x43 => instruction!(Sre, IndexedIndirect, 8),
        0x44 => instruction!(Nop, ZeroPage, 3),
        0x45 => instruction!(Eor, ZeroPage, 3),
        0x46 => instruction!(Lsr, ZeroPage, 5),
        0x47 => instruction!(Sre, ZeroPage, 5),
        0x48 => instruction!(Pha, Implied, 3),
        0x49 => instruction!(Eor, Immediate, 2),
        0x4a => instruction!(Lsr, Accumulator, 2),
        0x4c => instruction!(Jmp, Absolute, 3),
        0x4d => instruction!(Eor, Absolute, 4),
        0x4e => instruction!(Lsr, Absolute, 6),
        0x4f => instruction!(Sre, Absolute, 6),
        0x50 => instruction!(Bvc, Relative, 2),
        0x51 => instruction!(Eor, IndirectIndexed, 5, page),
        0x53 => instruction!(Sre, IndirectIndexed, 8),
        0x54 => instruction!(Nop, ZeroPageX, 4),
        0x55 => instruction!(Eor, ZeroPageX, 4),
        0x56 => instruction!(Lsr, ZeroPageX, 6),
        0x57 => instruction!(Sre, ZeroPageX, 6),
        0x58 => instruction!(Cli, Implied, 2),
        0x59 => instruction!(Eor, AbsoluteY, 4, page),
        0x5a => instruction!(Nop, Implied, 2),
        0x5b => instruction!(Sre, AbsoluteY, 7),
        0x5c => instruction!(Nop, AbsoluteX, 4, page),
        0x5d => instruction!(Eor, AbsoluteX, 4, page),
        0x5e => instruction!(Lsr, AbsoluteX, 7),
        0x5f => instruction!(Sre, AbsoluteX, 7),
        0x60 => instruction!(Rts, Implied, 6),
        0x61 => instruction!(Adc, IndexedIndirect, 6),
        0x63 => instruction!(Rra, IndexedIndirect, 8),
        0x64 => instruction!(Nop, ZeroPage, 3),
        0x65 => instruction!(Adc, ZeroPage, 3),
        0x66 => instruction!(Ror, ZeroPage, 5),
        0x67 => instruction!(Rra, ZeroPage, 5),
        0x68 => instruction!(Pla, Implied, 4),
        0x69 => instruction!(Adc, Immediate, 2),
        0x6a => instruction!(Ror, Accumulator, 2),
        0x6c => instruction!(Jmp, Indirect, 5),
        0x6d => instruction!(Adc, Absolute, 4),
        0x6e => instruction!(Ror, Absolute, 6),
        0x6f => instruction!(Rra, Absolute, 6),
        0x70 => instruction!(Bvs, Relative, 2),
        0x71 => instruction!(Adc, IndirectIndexed, 5, page),
        0x73 => instruction!(Rra, IndirectIndexed, 8),
        0x74 => instruction!(Nop, ZeroPageX, 4),
        0x75 => instruction!(Adc, ZeroPageX, 4),
        0x76 => instruction!(Ror, ZeroPageX, 6),
        0x77 => instruction!(Rra, ZeroPageX, 6),
        0x78 => instruction!(Sei, Implied, 2),
        0x79 => instruction!(Adc, AbsoluteY, 4, page),
        0x7a => instruction!(Nop, Implied, 2),
        0x7b => instruction!(Rra, AbsoluteY, 7),
        0x7c => instruction!(Nop, AbsoluteX, 4, page),
        0x7d => instruction!(Adc, AbsoluteX, 4, page),
        0x7e => instruction!(Ror, AbsoluteX, 7),
        0x7f => instruction!(Rra, AbsoluteX, 7),
        0x80 => instruction!(Nop, Immediate, 2),
        0x81 => instruction!(Sta, IndexedIndirect, 6),
        0x83 => instruction!(Sax, IndexedIndirect, 6),
        0x84 => instruction!(Sty, ZeroPage, 3),
        0x85 => instruction!(Sta, ZeroPage, 3),
        0x86 => instruction!(Stx, ZeroPage, 3),
        0x87 => instruction!(Sax, ZeroPage, 3),
        0x88 => instruction!(Dey, Implied, 2),
        0x8a => instruction!(Txa, Implied, 2),
        0x8c => instruction!(Sty, Absolute, 4),
        0x8d => instruction!(Sta, Absolute, 4),
        0x8e => instruction!(Stx, Absolute, 4),
        0x8f => instruction!(Sax, Absolute, 4),
        0x90 => instruction!(Bcc, Relative, 2),
        0x91 => instruction!(Sta, IndirectIndexed, 6),
        0x94 => instruction!(Sty, ZeroPageX, 4),
        0x95 => instruction!(Sta, ZeroPageX, 4),
        0x96 => instruction!(Stx, ZeroPageY, 4),
        0x97 => instruction!(Sax, ZeroPageY, 4),
        0x98 => instruction!(Tya, Implied, 2),
        0x99 => instruction!(Sta, AbsoluteY, 5),
        0x9a => instruction!(Txs, Implied, 2),
        0x9d => instruction!(Sta, AbsoluteX, 5),
        0xa0 => instruction!(Ldy, Immediate, 2),
        0xa1 => instruction!(Lda, IndexedIndirect, 6),
        0xa2 => instruction!(Ldx, Immediate, 2),
        0xa3 => instruction!(Lax, IndexedIndirect, 6),
        0xa4 => instruction!(Ldy, ZeroPage, 3),
        0xa5 => instruction!(Lda, ZeroPage, 3),
        0xa6 => instruction!(Ldx, ZeroPage, 3),
        0xa7 => instruction!(Lax, ZeroPage, 3),
        0xa8 => instruction!(Tay, Implied, 2),
        0xa9 => instruction!(Lda, Immediate, 2),
        0xaa => instruction!(Tax, Implied, 2),
        0xac => instruction!(Ldy, Absolute, 4),
        0xad => instruction!(Lda, Absolute, 4),
        0xae => instruction!(Ldx, Absolute, 4),
        0xaf => instruction!(Lax, Absolute, 4),
        0xb0 => instruction!(Bcs, Relative, 2),
        0xb1 => instruction!(Lda, IndirectIndexed, 5, page),
        0xb3 => instruction!(Lax, IndirectIndexed, 5, page),
        0xb4 => instruction!(Ldy, ZeroPageX, 4),
        0xb5 => instruction!(Lda, ZeroPageX, 4),
        0xb6 => instruction!(Ldx, ZeroPageY, 4),
        0xb7 => instruction!(Lax, ZeroPageY, 4),
        0xb8 => instruction!(Clv, Implied, 2),
        0xb9 => instruction!(Lda, AbsoluteY, 4, page),
        0xba => instruction!(Tsx, Implied, 2),
        0xbc => instruction!(Ldy, AbsoluteX, 4, page),
        0xbd => instruction!(Lda, AbsoluteX, 4, page),
        0xbe => instruction!(Ldx, AbsoluteY, 4, page),
        0xbf => instruction!(Lax, AbsoluteY, 4, page),
        0xc0 => instruction!(Cpy, Immediate, 2),
        0xc1 => instruction!(Cmp, IndexedIndirect, 6),
        0xc3 => instruction!(Dcp, IndexedIndirect, 8),
        0xc4 => instruction!(Cpy, ZeroPage, 3),
        0xc5 => instruction!(Cmp, ZeroPage, 3),
        0xc6 => instruction!(Dec, ZeroPage, 5),
        0xc7 => instruction!(Dcp, ZeroPage, 5),
        0xc8 => instruction!(Iny, Implied, 2),
        0xc9 => instruction!(Cmp, Immediate, 2),
        0xca => instruction!(Dex, Implied, 2),
        0xcc => instruction!(Cpy, Absolute, 4),
        0xcd => instruction!(Cmp, Absolute, 4),
        0xce => instruction!(Dec, Absolute, 6),
        0xcf => instruction!(Dcp, Absolute, 6),
        0xd0 => instruction!(Bne, Relative, 2),
        0xd1 => instruction!(Cmp, IndirectIndexed, 5, page),
        0xd3 => instruction!(Dcp, IndirectIndexed, 8),
        0xd4 => instruction!(Nop, ZeroPageX, 4),
        0xd5 => instruction!(Cmp, ZeroPageX, 4),
        0xd6 => instruction!(Dec, ZeroPageX, 6),
        0xd7 => instruction!(Dcp, ZeroPageX, 6),
        0xd8 => instruction!(Cld, Implied, 2),
        0xd9 => instruction!(Cmp, AbsoluteY, 4, page),
        0xda => instruction!(Nop, Implied, 2),
        0xdb => instruction!(Dcp, AbsoluteY, 7),
        0xdc => instruction!(Nop, AbsoluteX, 4, page),
        0xdd => instruction!(Cmp, AbsoluteX, 4, page),
        0xde => instruction!(Dec, AbsoluteX, 7),
        0xdf => instruction!(Dcp, AbsoluteX, 7),
        0xe0 => instruction!(Cpx, Immediate, 2),
        0xe1 => instruction!(Sbc, IndexedIndirect, 6),
        0xe3 => instruction!(Isc, IndexedIndirect, 8),
        0xe4 => instruction!(Cpx, ZeroPage, 3),
        0xe5 => instruction!(Sbc, ZeroPage, 3),
        0xe6 => instruction!(Inc, ZeroPage, 5),
        0xe7 => instruction!(Isc, ZeroPage, 5),
        0xe8 => instruction!(Inx, Implied, 2),
        0xe9 => instruction!(Sbc, Immediate, 2),
        0xea => instruction!(Nop, Implied, 2),
        0xeb => instruction!(Sbc, Immediate, 2),
        0xec => instruction!(Cpx, Absolute, 4),
        0xed => instruction!(Sbc, Absolute, 4),
        0xee => instruction!(Inc, Absolute, 6),
        0xef => instruction!(Isc, Absolute, 6),
        0xf0 => instruction!(Beq, Relative, 2),
        0xf1 => instruction!(Sbc, IndirectIndexed, 5, page),
        0xf3 => instruction!(Isc, IndirectIndexed, 8),
        0xf4 => instruction!(Nop, ZeroPageX, 4),
        0xf5 => instruction!(Sbc, ZeroPageX, 4),
        0xf6 => instruction!(Inc, ZeroPageX, 6),
        0xf7 => instruction!(Isc, ZeroPageX, 6),
        0xf8 => instruction!(Sed, Implied, 2),
        0xf9 => instruction!(Sbc, AbsoluteY, 4, page),
        0xfa => instruction!(Nop, Implied, 2),
        0xfb => instruction!(Isc, AbsoluteY, 7),
        0xfc => instruction!(Nop, AbsoluteX, 4, page),
        0xfd => instruction!(Sbc, AbsoluteX, 4, page),
        0xfe => instruction!(Inc, AbsoluteX, 7),
        0xff => instruction!(Isc, AbsoluteX, 7),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StepTrace {
    pub before: CpuState,
    pub after: CpuState,
    pub opcode: u8,
    pub instruction: Instruction,
    pub cycles: u8,
}

/// A hardware interrupt source sampled on the CPU's interrupt lines.
///
/// `Nmi` is edge-triggered and cannot be masked. `Irq` is level-triggered and is
/// only recognized while the interrupt-disable (`I`) flag is clear.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Interrupt {
    Nmi,
    Irq,
}

impl Interrupt {
    #[must_use]
    const fn vector(self) -> u16 {
        match self {
            Self::Nmi => NMI_VECTOR,
            Self::Irq => IRQ_VECTOR,
        }
    }
}

/// The architectural effect of servicing a hardware interrupt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InterruptEntry {
    pub kind: Interrupt,
    pub before: CpuState,
    pub after: CpuState,
    pub vector: u16,
    pub cycles: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CpuError {
    UnsupportedOpcode { pc: u16, opcode: u8 },
    CycleCounterHeadroomExhausted { remaining: u8 },
    CycleOverflow,
}

impl Display for CpuError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedOpcode { pc, opcode } => {
                write!(formatter, "unsupported opcode ${opcode:02X} at ${pc:04X}")
            }
            Self::CycleCounterHeadroomExhausted { remaining } => write!(
                formatter,
                "CPU cycle counter has only {remaining} cycles of safe headroom"
            ),
            Self::CycleOverflow => formatter.write_str("CPU cycle counter overflowed"),
        }
    }
}

impl Error for CpuError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cpu {
    state: CpuState,
    /// Last observed level of the NMI line (`true` when asserted), used to detect
    /// the high-to-low edge that latches an NMI.
    nmi_level: bool,
    /// A latched NMI edge awaiting service. Cleared only when the NMI is serviced
    /// or the CPU is reset.
    nmi_pending: bool,
    /// Current level of the IRQ line (`true` when asserted). IRQ is level-driven,
    /// so it is re-evaluated at every interrupt poll rather than latched.
    irq_level: bool,
    /// The interrupt-disable (`I`) flag value the most recent interrupt poll
    /// observed. This honors the one-instruction delay of `CLI`/`SEI`/`PLP`,
    /// whose flag change lands after the poll.
    poll_interrupt_disable: bool,
}

// Eight is the largest cycle total among the documented instructions and the
// stable undocumented encodings exercised by the accepted nestest trace.
const MAX_INSTRUCTION_CYCLES: u64 = 8;

// Hardware interrupt entry (IRQ, NMI, and the BRK software interrupt) and the
// reset sequence each take seven cycles at the architectural granularity this
// crate models.
const INTERRUPT_CYCLES: u64 = 7;

const NMI_VECTOR: u16 = 0xfffa;
const RESET_VECTOR: u16 = 0xfffc;
const IRQ_VECTOR: u16 = 0xfffe;

impl Cpu {
    #[must_use]
    pub fn new(state: CpuState) -> Self {
        let mut cpu = Self {
            state,
            nmi_level: false,
            nmi_pending: false,
            irq_level: false,
            poll_interrupt_disable: state.status & FLAG_INTERRUPT_DISABLE != 0,
        };
        cpu.normalize_status();
        cpu
    }

    #[must_use]
    pub const fn state(&self) -> CpuState {
        self.state
    }

    pub fn power_on(&mut self, bus: &mut impl Bus) -> Result<(), CpuError> {
        self.state = CpuState::default();
        self.state.pc = read_u16(bus, RESET_VECTOR);
        self.state.total_cycles = 7;
        self.nmi_level = false;
        self.nmi_pending = false;
        self.irq_level = false;
        // The power-on state has the `I` flag set.
        self.poll_interrupt_disable = true;
        Ok(())
    }

    /// Drives the NMI line. NMI is edge-triggered: a deasserted-to-asserted
    /// transition latches a pending NMI that survives until it is serviced or the
    /// CPU is reset.
    pub fn set_nmi_line(&mut self, asserted: bool) {
        if asserted && !self.nmi_level {
            self.nmi_pending = true;
        }
        self.nmi_level = asserted;
    }

    /// Drives the IRQ line. IRQ is level-triggered, so the environment holds the
    /// line asserted for as long as the request stands.
    pub fn set_irq_line(&mut self, asserted: bool) {
        self.irq_level = asserted;
    }

    /// The interrupt that would be serviced before the next instruction, based on
    /// the line state and the most recent interrupt poll. NMI outranks IRQ, and a
    /// level-triggered IRQ is only reported while the polled `I` flag was clear.
    #[must_use]
    pub const fn pending_interrupt(&self) -> Option<Interrupt> {
        if self.nmi_pending {
            Some(Interrupt::Nmi)
        } else if self.irq_level && !self.poll_interrupt_disable {
            Some(Interrupt::Irq)
        } else {
            None
        }
    }

    /// Services a hardware interrupt: pushes the return address and status (with
    /// the `B` flag clear), sets the `I` flag, and loads the source's vector. The
    /// seven-cycle count is reserved before any bus access, so an exhausted cycle
    /// counter leaves the CPU and bus unchanged.
    ///
    /// Callers select `kind` from [`Cpu::pending_interrupt`]. Servicing an NMI
    /// clears its latched edge. Exact per-cycle interrupt bus order and NMI
    /// hijacking of a BRK/IRQ sequence are part of the deferred per-cycle
    /// milestone and are not modeled here.
    pub fn service_interrupt(
        &mut self,
        bus: &mut impl Bus,
        kind: Interrupt,
    ) -> Result<InterruptEntry, CpuError> {
        if self
            .state
            .total_cycles
            .checked_add(INTERRUPT_CYCLES)
            .is_none()
        {
            return Err(CpuError::CycleCounterHeadroomExhausted {
                remaining: (u64::MAX - self.state.total_cycles) as u8,
            });
        }

        let before = self.state;
        let vector = kind.vector();
        self.enter_interrupt_frame(bus, vector, false);
        self.state.total_cycles += INTERRUPT_CYCLES;
        if kind == Interrupt::Nmi {
            self.nmi_pending = false;
        }
        // The `I` flag is now set; the next poll observes it as set.
        self.poll_interrupt_disable = true;
        self.normalize_status();
        Ok(InterruptEntry {
            kind,
            before,
            after: self.state,
            vector,
            cycles: INTERRUPT_CYCLES as u8,
        })
    }

    /// Performs a warm reset. Three suppressed stack accesses decrement `SP` by
    /// three without writing, the `I` flag is set, any latched NMI is dropped, and
    /// `PC` is loaded from the reset vector. `A`, `X`, `Y`, and the other flags are
    /// preserved. The seven-cycle count is reserved before any bus access. Exact
    /// per-cycle reset bus reads are part of the deferred per-cycle milestone.
    pub fn reset(&mut self, bus: &mut impl Bus) -> Result<(), CpuError> {
        if self
            .state
            .total_cycles
            .checked_add(INTERRUPT_CYCLES)
            .is_none()
        {
            return Err(CpuError::CycleCounterHeadroomExhausted {
                remaining: (u64::MAX - self.state.total_cycles) as u8,
            });
        }

        self.state.sp = self.state.sp.wrapping_sub(3);
        self.set_flag(FLAG_INTERRUPT_DISABLE, true);
        self.state.pc = read_u16(bus, RESET_VECTOR);
        self.state.total_cycles += INTERRUPT_CYCLES;
        self.nmi_pending = false;
        self.poll_interrupt_disable = true;
        self.normalize_status();
        Ok(())
    }

    fn enter_interrupt_frame(&mut self, bus: &mut impl Bus, vector: u16, break_flag: bool) {
        self.push_u16(bus, self.state.pc);
        let mut pushed = self.state.status | FLAG_UNUSED;
        if break_flag {
            pushed |= FLAG_BREAK;
        } else {
            pushed &= !FLAG_BREAK;
        }
        self.push(bus, pushed);
        self.set_flag(FLAG_INTERRUPT_DISABLE, true);
        self.state.pc = read_u16(bus, vector);
    }

    pub fn step(&mut self, bus: &mut impl Bus) -> Result<StepTrace, CpuError> {
        if self
            .state
            .total_cycles
            .checked_add(MAX_INSTRUCTION_CYCLES)
            .is_none()
        {
            return Err(CpuError::CycleCounterHeadroomExhausted {
                remaining: (u64::MAX - self.state.total_cycles) as u8,
            });
        }

        let before = self.state;
        let opcode = bus.read(self.state.pc);
        let instruction = decode(opcode).ok_or(CpuError::UnsupportedOpcode {
            pc: self.state.pc,
            opcode,
        })?;
        self.state.pc = self.state.pc.wrapping_add(1);
        let extra_cycles = self.execute(bus, instruction);
        let cycles = instruction
            .base_cycles
            .checked_add(extra_cycles)
            .ok_or(CpuError::CycleOverflow)?;
        self.state.total_cycles = self
            .state
            .total_cycles
            .checked_add(u64::from(cycles))
            .ok_or(CpuError::CycleOverflow)?;
        self.normalize_status();
        // Sample the interrupt poll for this instruction. `CLI`, `SEI`, and `PLP`
        // change the `I` flag on their final cycle, after the poll, so the poll
        // observes the pre-instruction value; every other instruction (including
        // `RTI`, which takes effect immediately) is observed after its change.
        self.poll_interrupt_disable = if matches!(
            instruction.mnemonic,
            Mnemonic::Cli | Mnemonic::Sei | Mnemonic::Plp
        ) {
            before.status & FLAG_INTERRUPT_DISABLE != 0
        } else {
            self.state.status & FLAG_INTERRUPT_DISABLE != 0
        };
        Ok(StepTrace {
            before,
            after: self.state,
            opcode,
            instruction,
            cycles,
        })
    }

    fn execute(&mut self, bus: &mut impl Bus, instruction: Instruction) -> u8 {
        use Mnemonic::{
            Adc, And, Asl, Bcc, Bcs, Beq, Bit, Bmi, Bne, Bpl, Brk, Bvc, Bvs, Clc, Cld, Cli, Clv,
            Cmp, Cpx, Cpy, Dcp, Dec, Dex, Dey, Eor, Inc, Inx, Iny, Isc, Jmp, Jsr, Lax, Lda, Ldx,
            Ldy, Lsr, Nop, Ora, Pha, Php, Pla, Plp, Rla, Rol, Ror, Rra, Rti, Rts, Sax, Sbc, Sec,
            Sed, Sei, Slo, Sre, Sta, Stx, Sty, Tax, Tay, Tsx, Txa, Txs, Tya,
        };

        match instruction.mnemonic {
            Ora | And | Eor | Adc | Lax | Lda | Ldx | Ldy | Cmp | Cpx | Cpy | Sbc | Bit => {
                let (value, page_crossed) = self.read_operand(bus, instruction.mode);
                match instruction.mnemonic {
                    Ora => {
                        self.state.a |= value;
                        self.update_zero_negative(self.state.a);
                    }
                    And => {
                        self.state.a &= value;
                        self.update_zero_negative(self.state.a);
                    }
                    Eor => {
                        self.state.a ^= value;
                        self.update_zero_negative(self.state.a);
                    }
                    Adc => self.adc(value),
                    Sbc => self.adc(value ^ 0xff),
                    Lda => {
                        self.state.a = value;
                        self.update_zero_negative(value);
                    }
                    Lax => {
                        self.state.a = value;
                        self.state.x = value;
                        self.update_zero_negative(value);
                    }
                    Ldx => {
                        self.state.x = value;
                        self.update_zero_negative(value);
                    }
                    Ldy => {
                        self.state.y = value;
                        self.update_zero_negative(value);
                    }
                    Cmp => self.compare(self.state.a, value),
                    Cpx => self.compare(self.state.x, value),
                    Cpy => self.compare(self.state.y, value),
                    Bit => {
                        self.set_flag(FLAG_ZERO, self.state.a & value == 0);
                        self.set_flag(FLAG_OVERFLOW, value & FLAG_OVERFLOW != 0);
                        self.set_flag(FLAG_NEGATIVE, value & FLAG_NEGATIVE != 0);
                    }
                    _ => unreachable!(),
                }
                u8::from(instruction.page_cross_cycle && page_crossed)
            }
            Sax | Sta | Stx | Sty => {
                let (address, _) = self.resolve_address(bus, instruction.mode);
                let value = match instruction.mnemonic {
                    Sta => self.state.a,
                    Stx => self.state.x,
                    Sty => self.state.y,
                    Sax => self.state.a & self.state.x,
                    _ => unreachable!(),
                };
                bus.write(address, value);
                0
            }
            Asl | Lsr | Rol | Ror | Inc | Dec | Dcp | Isc | Slo | Rla | Sre | Rra => {
                if instruction.mode == AddressingMode::Accumulator {
                    self.state.a = self.modify(instruction.mnemonic, self.state.a);
                } else {
                    let (address, _) = self.resolve_address(bus, instruction.mode);
                    let value = bus.read(address);
                    let operation = match instruction.mnemonic {
                        Dcp => Dec,
                        Isc => Inc,
                        Slo => Asl,
                        Rla => Rol,
                        Sre => Lsr,
                        Rra => Ror,
                        other => other,
                    };
                    let result = self.modify(operation, value);
                    bus.write(address, result);
                    match instruction.mnemonic {
                        Dcp => self.compare(self.state.a, result),
                        Isc => self.adc(result ^ 0xff),
                        Slo => {
                            self.state.a |= result;
                            self.update_zero_negative(self.state.a);
                        }
                        Rla => {
                            self.state.a &= result;
                            self.update_zero_negative(self.state.a);
                        }
                        Sre => {
                            self.state.a ^= result;
                            self.update_zero_negative(self.state.a);
                        }
                        Rra => self.adc(result),
                        _ => {}
                    }
                }
                0
            }
            Bcc | Bcs | Beq | Bmi | Bne | Bpl | Bvc | Bvs => {
                let condition = match instruction.mnemonic {
                    Bcc => !self.flag(FLAG_CARRY),
                    Bcs => self.flag(FLAG_CARRY),
                    Beq => self.flag(FLAG_ZERO),
                    Bmi => self.flag(FLAG_NEGATIVE),
                    Bne => !self.flag(FLAG_ZERO),
                    Bpl => !self.flag(FLAG_NEGATIVE),
                    Bvc => !self.flag(FLAG_OVERFLOW),
                    Bvs => self.flag(FLAG_OVERFLOW),
                    _ => unreachable!(),
                };
                self.branch(bus, condition)
            }
            Brk => {
                // BRK skips the padding byte after the opcode, then pushes the
                // return address and status with the `B` flag set and enters the
                // IRQ/BRK vector.
                self.state.pc = self.state.pc.wrapping_add(1);
                self.enter_interrupt_frame(bus, IRQ_VECTOR, true);
                0
            }
            Jmp => {
                self.state.pc = if instruction.mode == AddressingMode::Indirect {
                    let pointer = self.fetch_u16(bus);
                    read_u16_indirect_bug(bus, pointer)
                } else {
                    self.fetch_u16(bus)
                };
                0
            }
            Jsr => {
                let target = self.fetch_u16(bus);
                self.push_u16(bus, self.state.pc.wrapping_sub(1));
                self.state.pc = target;
                0
            }
            Rti => {
                self.state.status = (self.pop(bus) & !FLAG_BREAK) | FLAG_UNUSED;
                self.state.pc = self.pop_u16(bus);
                0
            }
            Rts => {
                self.state.pc = self.pop_u16(bus).wrapping_add(1);
                0
            }
            Pha => {
                self.push(bus, self.state.a);
                0
            }
            Php => {
                self.push(bus, self.state.status | FLAG_BREAK | FLAG_UNUSED);
                0
            }
            Pla => {
                self.state.a = self.pop(bus);
                self.update_zero_negative(self.state.a);
                0
            }
            Plp => {
                self.state.status = (self.pop(bus) & !FLAG_BREAK) | FLAG_UNUSED;
                0
            }
            Clc | Cld | Cli | Clv | Sec | Sed | Sei => {
                match instruction.mnemonic {
                    Clc => self.set_flag(FLAG_CARRY, false),
                    Cld => self.set_flag(FLAG_DECIMAL, false),
                    Cli => self.set_flag(FLAG_INTERRUPT_DISABLE, false),
                    Clv => self.set_flag(FLAG_OVERFLOW, false),
                    Sec => self.set_flag(FLAG_CARRY, true),
                    Sed => self.set_flag(FLAG_DECIMAL, true),
                    Sei => self.set_flag(FLAG_INTERRUPT_DISABLE, true),
                    _ => unreachable!(),
                }
                0
            }
            Dex | Dey | Inx | Iny => {
                let value = match instruction.mnemonic {
                    Dex => {
                        self.state.x = self.state.x.wrapping_sub(1);
                        self.state.x
                    }
                    Dey => {
                        self.state.y = self.state.y.wrapping_sub(1);
                        self.state.y
                    }
                    Inx => {
                        self.state.x = self.state.x.wrapping_add(1);
                        self.state.x
                    }
                    Iny => {
                        self.state.y = self.state.y.wrapping_add(1);
                        self.state.y
                    }
                    _ => unreachable!(),
                };
                self.update_zero_negative(value);
                0
            }
            Tax | Tay | Tsx | Txa | Tya => {
                let value = match instruction.mnemonic {
                    Tax => {
                        self.state.x = self.state.a;
                        self.state.x
                    }
                    Tay => {
                        self.state.y = self.state.a;
                        self.state.y
                    }
                    Tsx => {
                        self.state.x = self.state.sp;
                        self.state.x
                    }
                    Txa => {
                        self.state.a = self.state.x;
                        self.state.a
                    }
                    Tya => {
                        self.state.a = self.state.y;
                        self.state.a
                    }
                    _ => unreachable!(),
                };
                self.update_zero_negative(value);
                0
            }
            Txs => {
                self.state.sp = self.state.x;
                0
            }
            Nop => {
                if instruction.mode == AddressingMode::Implied {
                    0
                } else {
                    let (_, page_crossed) = self.read_operand(bus, instruction.mode);
                    u8::from(instruction.page_cross_cycle && page_crossed)
                }
            }
        }
    }

    fn fetch(&mut self, bus: &mut impl Bus) -> u8 {
        let value = bus.read(self.state.pc);
        self.state.pc = self.state.pc.wrapping_add(1);
        value
    }

    fn fetch_u16(&mut self, bus: &mut impl Bus) -> u16 {
        let low = self.fetch(bus);
        let high = self.fetch(bus);
        u16::from_le_bytes([low, high])
    }

    fn resolve_address(&mut self, bus: &mut impl Bus, mode: AddressingMode) -> (u16, bool) {
        match mode {
            AddressingMode::ZeroPage => (u16::from(self.fetch(bus)), false),
            AddressingMode::ZeroPageX => {
                (u16::from(self.fetch(bus).wrapping_add(self.state.x)), false)
            }
            AddressingMode::ZeroPageY => {
                (u16::from(self.fetch(bus).wrapping_add(self.state.y)), false)
            }
            AddressingMode::Absolute => (self.fetch_u16(bus), false),
            AddressingMode::AbsoluteX => {
                let base = self.fetch_u16(bus);
                let address = base.wrapping_add(u16::from(self.state.x));
                (address, page_crossed(base, address))
            }
            AddressingMode::AbsoluteY => {
                let base = self.fetch_u16(bus);
                let address = base.wrapping_add(u16::from(self.state.y));
                (address, page_crossed(base, address))
            }
            AddressingMode::IndexedIndirect => {
                let pointer = self.fetch(bus).wrapping_add(self.state.x);
                (read_u16_zero_page(bus, pointer), false)
            }
            AddressingMode::IndirectIndexed => {
                let pointer = self.fetch(bus);
                let base = read_u16_zero_page(bus, pointer);
                let address = base.wrapping_add(u16::from(self.state.y));
                (address, page_crossed(base, address))
            }
            _ => unreachable!("mode {mode:?} does not resolve to a data address"),
        }
    }

    fn read_operand(&mut self, bus: &mut impl Bus, mode: AddressingMode) -> (u8, bool) {
        if mode == AddressingMode::Immediate {
            return (self.fetch(bus), false);
        }
        let (address, crossed) = self.resolve_address(bus, mode);
        (bus.read(address), crossed)
    }

    fn modify(&mut self, mnemonic: Mnemonic, value: u8) -> u8 {
        let result = match mnemonic {
            Mnemonic::Asl => {
                self.set_flag(FLAG_CARRY, value & 0x80 != 0);
                value << 1
            }
            Mnemonic::Lsr => {
                self.set_flag(FLAG_CARRY, value & 0x01 != 0);
                value >> 1
            }
            Mnemonic::Rol => {
                let carry = u8::from(self.flag(FLAG_CARRY));
                self.set_flag(FLAG_CARRY, value & 0x80 != 0);
                (value << 1) | carry
            }
            Mnemonic::Ror => {
                let carry = u8::from(self.flag(FLAG_CARRY)) << 7;
                self.set_flag(FLAG_CARRY, value & 0x01 != 0);
                (value >> 1) | carry
            }
            Mnemonic::Inc => value.wrapping_add(1),
            Mnemonic::Dec => value.wrapping_sub(1),
            _ => unreachable!(),
        };
        self.update_zero_negative(result);
        result
    }

    fn adc(&mut self, value: u8) {
        let a = self.state.a;
        let carry = u16::from(self.flag(FLAG_CARRY));
        let sum = u16::from(a) + u16::from(value) + carry;
        let result = sum as u8;
        self.set_flag(FLAG_CARRY, sum > 0xff);
        self.set_flag(
            FLAG_OVERFLOW,
            (!(a ^ value) & (a ^ result) & FLAG_NEGATIVE) != 0,
        );
        self.state.a = result;
        self.update_zero_negative(result);
    }

    fn compare(&mut self, register: u8, value: u8) {
        let result = register.wrapping_sub(value);
        self.set_flag(FLAG_CARRY, register >= value);
        self.update_zero_negative(result);
    }

    fn branch(&mut self, bus: &mut impl Bus, condition: bool) -> u8 {
        let offset = self.fetch(bus) as i8;
        if !condition {
            return 0;
        }
        let before = self.state.pc;
        self.state.pc = self.state.pc.wrapping_add_signed(i16::from(offset));
        1 + u8::from(page_crossed(before, self.state.pc))
    }

    fn push(&mut self, bus: &mut impl Bus, value: u8) {
        bus.write(0x0100 | u16::from(self.state.sp), value);
        self.state.sp = self.state.sp.wrapping_sub(1);
    }

    fn pop(&mut self, bus: &mut impl Bus) -> u8 {
        self.state.sp = self.state.sp.wrapping_add(1);
        bus.read(0x0100 | u16::from(self.state.sp))
    }

    fn push_u16(&mut self, bus: &mut impl Bus, value: u16) {
        let [low, high] = value.to_le_bytes();
        self.push(bus, high);
        self.push(bus, low);
    }

    fn pop_u16(&mut self, bus: &mut impl Bus) -> u16 {
        let low = self.pop(bus);
        let high = self.pop(bus);
        u16::from_le_bytes([low, high])
    }

    fn flag(&self, flag: u8) -> bool {
        self.state.status & flag != 0
    }

    fn set_flag(&mut self, flag: u8, enabled: bool) {
        if enabled {
            self.state.status |= flag;
        } else {
            self.state.status &= !flag;
        }
    }

    fn update_zero_negative(&mut self, value: u8) {
        self.set_flag(FLAG_ZERO, value == 0);
        self.set_flag(FLAG_NEGATIVE, value & FLAG_NEGATIVE != 0);
    }

    fn normalize_status(&mut self) {
        self.state.status = (self.state.status | FLAG_UNUSED) & !FLAG_BREAK;
    }
}

fn read_u16(bus: &mut impl Bus, address: u16) -> u16 {
    let low = bus.read(address);
    let high = bus.read(address.wrapping_add(1));
    u16::from_le_bytes([low, high])
}

fn read_u16_zero_page(bus: &mut impl Bus, address: u8) -> u16 {
    let low = bus.read(u16::from(address));
    let high = bus.read(u16::from(address.wrapping_add(1)));
    u16::from_le_bytes([low, high])
}

fn read_u16_indirect_bug(bus: &mut impl Bus, address: u16) -> u16 {
    let low = bus.read(address);
    let high_address = (address & 0xff00) | u16::from((address as u8).wrapping_add(1));
    let high = bus.read(high_address);
    u16::from_le_bytes([low, high])
}

const fn page_crossed(first: u16, second: u16) -> bool {
    first & 0xff00 != second & 0xff00
}

#[cfg(test)]
mod singlestep_vectors;

#[cfg(test)]
mod tests {
    use super::singlestep_vectors::{Snapshot, UPSTREAM_COMMIT, VECTORS, Vector};
    use super::*;

    struct Ram {
        data: Vec<u8>,
    }

    impl Ram {
        fn new() -> Self {
            Self {
                data: vec![0; 65_536],
            }
        }
    }

    impl Bus for Ram {
        fn read(&mut self, address: u16) -> u8 {
            self.data[usize::from(address)]
        }

        fn write(&mut self, address: u16, value: u8) {
            self.data[usize::from(address)] = value;
        }
    }

    fn cpu_with(mut state: CpuState) -> Cpu {
        state.status |= FLAG_UNUSED;
        Cpu::new(state)
    }

    struct OracleRam {
        data: Vec<u8>,
        known: Vec<bool>,
        vector_name: &'static str,
    }

    impl OracleRam {
        fn from_vector(vector: &Vector) -> Self {
            let mut ram = Self {
                data: vec![0; 65_536],
                known: vec![false; 65_536],
                vector_name: vector.name,
            };
            for &(address, value) in vector.initial_ram {
                let index = usize::from(address);
                if ram.known[index] {
                    assert_eq!(
                        ram.data[index], value,
                        "vector {} has conflicting initial RAM at ${address:04X}",
                        vector.name
                    );
                }
                ram.data[index] = value;
                ram.known[index] = true;
            }
            for &(address, _) in vector.final_ram {
                ram.known[usize::from(address)] = true;
            }
            ram
        }

        fn assert_final(&self, vector: &Vector) {
            for &(address, expected) in vector.final_ram {
                assert_eq!(
                    self.data[usize::from(address)],
                    expected,
                    "vector {} final RAM mismatch at ${address:04X}",
                    vector.name
                );
            }
        }
    }

    impl Bus for OracleRam {
        fn read(&mut self, address: u16) -> u8 {
            assert!(
                self.known[usize::from(address)],
                "vector {} read undeclared RAM at ${address:04X}",
                self.vector_name
            );
            self.data[usize::from(address)]
        }

        fn write(&mut self, address: u16, value: u8) {
            assert!(
                self.known[usize::from(address)],
                "vector {} wrote undeclared RAM at ${address:04X}",
                self.vector_name
            );
            self.data[usize::from(address)] = value;
        }
    }

    const fn state_from_snapshot(snapshot: Snapshot, total_cycles: u64) -> CpuState {
        CpuState {
            pc: snapshot.pc,
            sp: snapshot.sp,
            a: snapshot.a,
            x: snapshot.x,
            y: snapshot.y,
            status: snapshot.status,
            total_cycles,
        }
    }

    const fn is_branch(mnemonic: Mnemonic) -> bool {
        matches!(
            mnemonic,
            Mnemonic::Bcc
                | Mnemonic::Bcs
                | Mnemonic::Beq
                | Mnemonic::Bmi
                | Mnemonic::Bne
                | Mnemonic::Bpl
                | Mnemonic::Bvc
                | Mnemonic::Bvs
        )
    }

    const NESTEST_UNDOCUMENTED_OPCODES: [u8; 76] = [
        0x03, 0x04, 0x07, 0x0c, 0x0f, 0x13, 0x14, 0x17, 0x1a, 0x1b, 0x1c, 0x1f, 0x23, 0x27, 0x2f,
        0x33, 0x34, 0x37, 0x3a, 0x3b, 0x3c, 0x3f, 0x43, 0x44, 0x47, 0x4f, 0x53, 0x54, 0x57, 0x5a,
        0x5b, 0x5c, 0x5f, 0x63, 0x64, 0x67, 0x6f, 0x73, 0x74, 0x77, 0x7a, 0x7b, 0x7c, 0x7f, 0x80,
        0x83, 0x87, 0x8f, 0x97, 0xa3, 0xa7, 0xaf, 0xb3, 0xb7, 0xbf, 0xc3, 0xc7, 0xcf, 0xd3, 0xd4,
        0xd7, 0xda, 0xdb, 0xdc, 0xdf, 0xe3, 0xe7, 0xeb, 0xef, 0xf3, 0xf4, 0xf7, 0xfa, 0xfb, 0xfc,
        0xff,
    ];

    #[test]
    fn decoder_contains_the_documented_and_nestest_undocumented_encodings() {
        assert_eq!(
            (0_u8..=u8::MAX)
                .filter(|opcode| decode(*opcode).is_some())
                .count(),
            227
        );
        for opcode in NESTEST_UNDOCUMENTED_OPCODES {
            assert!(decode(opcode).is_some(), "missing opcode ${opcode:02X}");
        }
    }

    #[test]
    fn every_supported_opcode_executes_from_a_neutral_state() {
        for opcode in 0_u8..=u8::MAX {
            let Some(instruction) = decode(opcode) else {
                continue;
            };
            let mut ram = Ram::new();
            ram.data[0x8000] = opcode;
            let mut cpu = Cpu::new(CpuState::at(0x8000));
            let trace = cpu.step(&mut ram).expect("documented opcode executes");
            assert_eq!(trace.instruction, instruction, "opcode ${opcode:02X}");
            assert!(trace.cycles >= 2, "opcode ${opcode:02X}");
        }
    }

    #[test]
    fn pinned_mit_single_step_vectors_match_all_documented_encodings() {
        assert_eq!(UPSTREAM_COMMIT, "2f6980a2d95757486c7bee24355c360e40e2a224");
        assert_eq!(VECTORS.len(), 190);

        let mut covered = [false; 256];
        let mut cycles_seen = [[false; 8]; 256];
        for vector in VECTORS {
            let instruction = decode(vector.opcode).unwrap_or_else(|| {
                panic!(
                    "MIT vector {} uses unsupported opcode ${:02X}",
                    vector.name, vector.opcode
                )
            });
            covered[usize::from(vector.opcode)] = true;
            cycles_seen[usize::from(vector.opcode)][usize::from(vector.cycles)] = true;

            let initial = state_from_snapshot(vector.initial, 0);
            let mut cpu = Cpu::new(initial);
            assert_eq!(
                cpu.state(),
                initial,
                "vector {} initial status is not canonical",
                vector.name
            );
            let mut ram = OracleRam::from_vector(vector);
            let trace = cpu
                .step(&mut ram)
                .unwrap_or_else(|error| panic!("vector {} failed: {error}", vector.name));
            let expected = state_from_snapshot(vector.final_state, u64::from(vector.cycles));

            assert_eq!(trace.before, initial, "vector {} before state", vector.name);
            assert_eq!(trace.after, expected, "vector {} after state", vector.name);
            assert_eq!(trace.opcode, vector.opcode, "vector {} opcode", vector.name);
            assert_eq!(
                trace.cycles, vector.cycles,
                "vector {} cycle count",
                vector.name
            );
            assert_eq!(cpu.state(), expected, "vector {} CPU state", vector.name);
            ram.assert_final(vector);

            if is_branch(instruction.mnemonic) {
                assert_eq!(instruction.base_cycles, 2);
            }
        }

        assert_eq!(covered.iter().filter(|covered| **covered).count(), 151);
        for opcode in 0_u8..=u8::MAX {
            let expected =
                covered[usize::from(opcode)] || NESTEST_UNDOCUMENTED_OPCODES.contains(&opcode);
            assert_eq!(
                decode(opcode).is_some(),
                expected,
                "opcode-set mismatch at ${opcode:02X}"
            );
            if !covered[usize::from(opcode)] {
                continue;
            }
            let Some(instruction) = decode(opcode) else {
                continue;
            };
            if is_branch(instruction.mnemonic) {
                for cycles in [2_usize, 3, 4] {
                    assert!(
                        cycles_seen[usize::from(opcode)][cycles],
                        "branch ${opcode:02X} lacks a {cycles}-cycle oracle vector"
                    );
                }
            } else if instruction.page_cross_cycle {
                for cycles in [instruction.base_cycles, instruction.base_cycles + 1] {
                    assert!(
                        cycles_seen[usize::from(opcode)][usize::from(cycles)],
                        "opcode ${opcode:02X} lacks a {cycles}-cycle page profile"
                    );
                }
            }
        }
    }

    #[test]
    fn undocumented_nops_consume_operands_and_report_page_cross_cycles() {
        let cases = [
            (0x04, 0x40, 0x00, 0, 2, 3),
            (0x0c, 0x00, 0x20, 0, 3, 4),
            (0x14, 0x40, 0x00, 1, 2, 4),
            (0x1a, 0x00, 0x00, 0, 1, 2),
            (0x80, 0x55, 0x00, 0, 2, 2),
            (0x1c, 0xfe, 0x20, 1, 3, 4),
            (0x1c, 0xff, 0x20, 1, 3, 5),
        ];
        for (opcode, low, high, x, bytes, cycles) in cases {
            let mut ram = Ram::new();
            ram.data[0x8000..0x8003].copy_from_slice(&[opcode, low, high]);
            let mut state = CpuState::at(0x8000);
            state.a = 0x5a;
            state.x = x;
            state.y = 0xa5;
            state.status = FLAG_CARRY | FLAG_OVERFLOW | FLAG_UNUSED;
            let mut cpu = cpu_with(state);
            let trace = cpu.step(&mut ram).expect("undocumented NOP executes");
            assert_eq!(trace.after.pc, 0x8000 + bytes, "opcode ${opcode:02X}");
            assert_eq!(trace.cycles, cycles, "opcode ${opcode:02X}");
            assert_eq!(trace.after.a, state.a, "opcode ${opcode:02X}");
            assert_eq!(trace.after.x, state.x, "opcode ${opcode:02X}");
            assert_eq!(trace.after.y, state.y, "opcode ${opcode:02X}");
            assert_eq!(trace.after.status, state.status, "opcode ${opcode:02X}");
        }
    }

    #[test]
    fn lax_loads_both_registers_and_sax_stores_their_intersection() {
        let mut ram = Ram::new();
        ram.data[0x8000..0x8003].copy_from_slice(&[0xbf, 0xff, 0x20]);
        ram.data[0x2100] = 0x80;
        let mut state = CpuState::at(0x8000);
        state.y = 1;
        let mut cpu = cpu_with(state);
        let trace = cpu.step(&mut ram).expect("LAX executes");
        assert_eq!(trace.cycles, 5);
        assert_eq!(trace.after.a, 0x80);
        assert_eq!(trace.after.x, 0x80);
        assert_ne!(trace.after.status & FLAG_NEGATIVE, 0);

        ram.data[0x8000..0x8002].copy_from_slice(&[0x97, 0x40]);
        let mut state = CpuState::at(0x8000);
        state.a = 0xf3;
        state.x = 0x3f;
        state.y = 2;
        state.status = FLAG_CARRY | FLAG_OVERFLOW | FLAG_UNUSED;
        let mut cpu = cpu_with(state);
        let trace = cpu.step(&mut ram).expect("SAX executes");
        assert_eq!(trace.cycles, 4);
        assert_eq!(ram.data[0x0042], 0x33);
        assert_eq!(trace.after.status, state.status);
    }

    #[test]
    fn undocumented_read_modify_write_combinations_preserve_operation_order() {
        let cases = [
            (0x07, 0x81, 0x01, 0, 0x02, 0x03, FLAG_CARRY),
            (0x27, 0x80, 0xff, FLAG_CARRY, 0x01, 0x01, FLAG_CARRY),
            (0x47, 0x03, 0x02, 0, 0x01, 0x03, FLAG_CARRY),
            (
                0x67,
                0x02,
                0x7f,
                0,
                0x01,
                0x80,
                FLAG_OVERFLOW | FLAG_NEGATIVE,
            ),
            (0xc7, 0x10, 0x0f, 0, 0x0f, 0x0f, FLAG_CARRY | FLAG_ZERO),
            (0xe7, 0x0f, 0x20, FLAG_CARRY, 0x10, 0x10, FLAG_CARRY),
        ];
        let arithmetic_flags = FLAG_CARRY | FLAG_ZERO | FLAG_OVERFLOW | FLAG_NEGATIVE;
        for (opcode, memory, a, status, expected_memory, expected_a, expected_flags) in cases {
            let mut ram = Ram::new();
            ram.data[0x8000..0x8002].copy_from_slice(&[opcode, 0x40]);
            ram.data[0x0040] = memory;
            let mut state = CpuState::at(0x8000);
            state.a = a;
            state.status = status | FLAG_UNUSED;
            let mut cpu = cpu_with(state);
            let trace = cpu.step(&mut ram).expect("combined operation executes");
            assert_eq!(trace.cycles, 5, "opcode ${opcode:02X}");
            assert_eq!(ram.data[0x0040], expected_memory, "opcode ${opcode:02X}");
            assert_eq!(trace.after.a, expected_a, "opcode ${opcode:02X}");
            assert_eq!(
                trace.after.status & arithmetic_flags,
                expected_flags,
                "opcode ${opcode:02X}"
            );
        }
    }

    #[test]
    fn undocumented_rmw_families_have_exact_modes_and_cycles() {
        let families = [
            (Mnemonic::Slo, [0x03, 0x07, 0x0f, 0x13, 0x17, 0x1b, 0x1f]),
            (Mnemonic::Rla, [0x23, 0x27, 0x2f, 0x33, 0x37, 0x3b, 0x3f]),
            (Mnemonic::Sre, [0x43, 0x47, 0x4f, 0x53, 0x57, 0x5b, 0x5f]),
            (Mnemonic::Rra, [0x63, 0x67, 0x6f, 0x73, 0x77, 0x7b, 0x7f]),
            (Mnemonic::Dcp, [0xc3, 0xc7, 0xcf, 0xd3, 0xd7, 0xdb, 0xdf]),
            (Mnemonic::Isc, [0xe3, 0xe7, 0xef, 0xf3, 0xf7, 0xfb, 0xff]),
        ];
        let modes = [
            AddressingMode::IndexedIndirect,
            AddressingMode::ZeroPage,
            AddressingMode::Absolute,
            AddressingMode::IndirectIndexed,
            AddressingMode::ZeroPageX,
            AddressingMode::AbsoluteY,
            AddressingMode::AbsoluteX,
        ];
        let cycles = [8, 5, 6, 8, 6, 7, 7];
        for (mnemonic, opcodes) in families {
            for index in 0..opcodes.len() {
                let instruction = decode(opcodes[index]).expect("opcode decodes");
                assert_eq!(instruction.mnemonic, mnemonic);
                assert_eq!(instruction.mode, modes[index]);
                assert_eq!(instruction.base_cycles, cycles[index]);
                assert!(!instruction.page_cross_cycle);
            }
        }
    }

    #[test]
    fn undocumented_sbc_immediate_alias_matches_the_documented_encoding() {
        let mut states = Vec::new();
        for opcode in [0xe9, 0xeb] {
            let mut ram = Ram::new();
            ram.data[0x8000..0x8002].copy_from_slice(&[opcode, 0x41]);
            let mut state = CpuState::at(0x8000);
            state.a = 0x80;
            state.status = FLAG_CARRY | FLAG_UNUSED;
            let mut cpu = cpu_with(state);
            states.push(cpu.step(&mut ram).expect("SBC executes"));
        }
        assert_eq!(states[0].after, states[1].after);
        assert_eq!(states[0].cycles, states[1].cycles);
    }

    #[test]
    fn eight_cycle_instruction_requires_headroom_before_state_changes() {
        let mut ram = Ram::new();
        ram.data[0x8000..0x8002].copy_from_slice(&[0x03, 0x40]);
        let before_ram = ram.data.clone();
        let mut state = CpuState::at(0x8000);
        state.total_cycles = u64::MAX - 7;
        let mut cpu = cpu_with(state);
        let before = cpu.state();
        assert_eq!(
            cpu.step(&mut ram),
            Err(CpuError::CycleCounterHeadroomExhausted { remaining: 7 })
        );
        assert_eq!(cpu.state(), before);
        assert_eq!(ram.data, before_ram);
    }

    #[test]
    fn load_and_page_cross_cycle_are_reported() {
        let mut ram = Ram::new();
        ram.data[0x8000..0x8003].copy_from_slice(&[0xbd, 0xff, 0x20]);
        ram.data[0x2100] = 0x80;
        let mut state = CpuState::at(0x8000);
        state.x = 1;
        let mut cpu = cpu_with(state);
        let trace = cpu.step(&mut ram).expect("LDA executes");
        assert_eq!(trace.cycles, 5);
        assert_eq!(trace.after.a, 0x80);
        assert_ne!(trace.after.status & FLAG_NEGATIVE, 0);
    }

    #[test]
    fn cycle_overflow_is_reported_before_cpu_or_bus_state_changes() {
        let mut ram = Ram::new();
        ram.data[0x8000..0x8003].copy_from_slice(&[0x8d, 0x00, 0x20]);
        ram.data[0x2000] = 0xa5;
        let mut state = CpuState::at(0x8000);
        state.a = 0x5a;
        state.total_cycles = u64::MAX;
        let mut cpu = cpu_with(state);
        let before = cpu.state();

        assert_eq!(
            cpu.step(&mut ram),
            Err(CpuError::CycleCounterHeadroomExhausted { remaining: 0 })
        );
        assert_eq!(cpu.state(), before);
        assert_eq!(ram.data[0x2000], 0xa5);
    }

    #[test]
    fn adc_sets_carry_and_overflow_and_sbc_uses_inverted_borrow() {
        let mut ram = Ram::new();
        ram.data[0x8000..0x8004].copy_from_slice(&[0x69, 0x50, 0xe9, 0x10]);
        let mut state = CpuState::at(0x8000);
        state.a = 0x50;
        let mut cpu = cpu_with(state);
        cpu.step(&mut ram).expect("ADC executes");
        assert_eq!(cpu.state().a, 0xa0);
        assert_ne!(cpu.state().status & FLAG_OVERFLOW, 0);
        assert_eq!(cpu.state().status & FLAG_CARRY, 0);

        let mut state = cpu.state();
        state.status |= FLAG_CARRY;
        let mut cpu = cpu_with(state);
        cpu.step(&mut ram).expect("SBC executes");
        assert_eq!(cpu.state().a, 0x90);
        assert_ne!(cpu.state().status & FLAG_CARRY, 0);
    }

    #[test]
    fn decimal_flag_is_observable_but_does_not_change_2a03_arithmetic() {
        let mut ram = Ram::new();
        ram.data[0x8000..0x8002].copy_from_slice(&[0x69, 0x55]);
        let mut state = CpuState::at(0x8000);
        state.a = 0x45;
        state.status |= FLAG_DECIMAL;
        let mut cpu = cpu_with(state);
        cpu.step(&mut ram).expect("ADC executes");
        assert_eq!(cpu.state().a, 0x9a);
        assert_ne!(cpu.state().status & FLAG_DECIMAL, 0);
    }

    #[test]
    fn taken_branch_adds_cycles_for_branch_and_page_crossing() {
        let mut ram = Ram::new();
        ram.data[0x80fd..0x80ff].copy_from_slice(&[0xd0, 0x02]);
        let mut cpu = Cpu::new(CpuState::at(0x80fd));
        let trace = cpu.step(&mut ram).expect("BNE executes");
        assert_eq!(trace.after.pc, 0x8101);
        assert_eq!(trace.cycles, 4);
    }

    #[test]
    fn indirect_jmp_reproduces_nmos_page_wrap_quirk() {
        let mut ram = Ram::new();
        ram.data[0x8000..0x8003].copy_from_slice(&[0x6c, 0xff, 0x10]);
        ram.data[0x10ff] = 0x34;
        ram.data[0x1000] = 0x12;
        ram.data[0x1100] = 0x99;
        let mut cpu = Cpu::new(CpuState::at(0x8000));
        cpu.step(&mut ram).expect("JMP executes");
        assert_eq!(cpu.state().pc, 0x1234);
    }

    #[test]
    fn jsr_and_rts_round_trip_through_hardware_stack_order() {
        let mut ram = Ram::new();
        ram.data[0x8000..0x8003].copy_from_slice(&[0x20, 0x00, 0x90]);
        ram.data[0x9000] = 0x60;
        let mut cpu = Cpu::new(CpuState::at(0x8000));
        cpu.step(&mut ram).expect("JSR executes");
        assert_eq!(cpu.state().pc, 0x9000);
        assert_eq!(ram.data[0x01fd], 0x80);
        assert_eq!(ram.data[0x01fc], 0x02);
        cpu.step(&mut ram).expect("RTS executes");
        assert_eq!(cpu.state().pc, 0x8003);
        assert_eq!(cpu.state().sp, 0xfd);
    }

    #[test]
    fn brk_and_rti_preserve_return_address_and_transient_break_bit() {
        let mut ram = Ram::new();
        ram.data[0x8000] = 0x00;
        ram.data[0x9000] = 0x40;
        ram.data[0xfffe] = 0x00;
        ram.data[0xffff] = 0x90;
        let mut cpu = Cpu::new(CpuState::at(0x8000));
        cpu.step(&mut ram).expect("BRK executes");
        assert_eq!(cpu.state().pc, 0x9000);
        assert_eq!(ram.data[0x01fd], 0x80);
        assert_eq!(ram.data[0x01fc], 0x02);
        assert_ne!(ram.data[0x01fb] & FLAG_BREAK, 0);
        assert_eq!(cpu.state().status & FLAG_BREAK, 0);
        cpu.step(&mut ram).expect("RTI executes");
        assert_eq!(cpu.state().pc, 0x8002);
        assert_eq!(cpu.state().status & FLAG_BREAK, 0);
        assert_ne!(cpu.state().status & FLAG_UNUSED, 0);
    }

    #[test]
    fn zero_page_indirect_addresses_wrap_within_zero_page() {
        let mut ram = Ram::new();
        ram.data[0x8000..0x8002].copy_from_slice(&[0xa1, 0xfe]);
        ram.data[0x00ff] = 0x34;
        ram.data[0x0000] = 0x12;
        ram.data[0x1234] = 0x7f;
        let mut state = CpuState::at(0x8000);
        state.x = 1;
        let mut cpu = cpu_with(state);
        cpu.step(&mut ram).expect("LDA indexed-indirect executes");
        assert_eq!(cpu.state().a, 0x7f);
    }

    #[test]
    fn power_on_reads_reset_vector_and_records_seven_cycles() {
        let mut ram = Ram::new();
        ram.data[0xfffc] = 0x78;
        ram.data[0xfffd] = 0x56;
        let mut cpu = Cpu::new(CpuState::at(0x1111));
        cpu.power_on(&mut ram).expect("power on succeeds");
        assert_eq!(cpu.state().pc, 0x5678);
        assert_eq!(cpu.state().sp, 0xfd);
        assert_eq!(cpu.state().total_cycles, 7);
    }

    // Builds a CPU whose status has the interrupt-disable flag clear so IRQ
    // recognition can be exercised.
    fn cpu_with_irq_enabled(pc: u16) -> Cpu {
        let mut state = CpuState::at(pc);
        state.status = FLAG_UNUSED;
        cpu_with(state)
    }

    #[test]
    fn reset_preserves_registers_sets_interrupt_disable_and_decrements_stack() {
        let mut ram = Ram::new();
        ram.data[0xfffc] = 0x00;
        ram.data[0xfffd] = 0xc0;
        let mut state = CpuState::at(0x1234);
        state.a = 0x11;
        state.x = 0x22;
        state.y = 0x33;
        state.sp = 0x80;
        state.status = FLAG_CARRY | FLAG_OVERFLOW | FLAG_UNUSED;
        state.total_cycles = 100;
        let mut cpu = cpu_with(state);
        // A latched NMI must not survive a reset.
        cpu.set_nmi_line(true);
        assert_eq!(cpu.pending_interrupt(), Some(Interrupt::Nmi));

        cpu.reset(&mut ram).expect("reset fits the cycle counter");

        assert_eq!(cpu.state().a, 0x11);
        assert_eq!(cpu.state().x, 0x22);
        assert_eq!(cpu.state().y, 0x33);
        assert_eq!(cpu.state().sp, 0x7d);
        assert_eq!(cpu.state().pc, 0xc000);
        assert_ne!(cpu.state().status & FLAG_INTERRUPT_DISABLE, 0);
        assert_ne!(cpu.state().status & FLAG_CARRY, 0);
        assert_ne!(cpu.state().status & FLAG_OVERFLOW, 0);
        assert_eq!(cpu.state().total_cycles, 107);
        assert_eq!(cpu.pending_interrupt(), None);
    }

    #[test]
    fn irq_is_masked_while_interrupt_disable_is_set() {
        let mut cpu = Cpu::new(CpuState::at(0x8000));
        assert_ne!(cpu.state().status & FLAG_INTERRUPT_DISABLE, 0);
        cpu.set_irq_line(true);
        assert_eq!(cpu.pending_interrupt(), None);
    }

    #[test]
    fn deasserting_the_irq_line_stops_recognition() {
        let mut cpu = cpu_with_irq_enabled(0x8000);
        cpu.set_irq_line(true);
        assert_eq!(cpu.pending_interrupt(), Some(Interrupt::Irq));
        cpu.set_irq_line(false);
        assert_eq!(cpu.pending_interrupt(), None);
    }

    #[test]
    fn irq_enters_its_vector_pushing_status_with_the_break_flag_clear() {
        let mut ram = Ram::new();
        ram.data[0xfffe] = 0x00;
        ram.data[0xffff] = 0x90;
        let mut cpu = cpu_with_irq_enabled(0x8000);
        cpu.set_irq_line(true);
        assert_eq!(cpu.pending_interrupt(), Some(Interrupt::Irq));

        let entry = cpu
            .service_interrupt(&mut ram, Interrupt::Irq)
            .expect("interrupt entry fits the cycle counter");

        assert_eq!(entry.kind, Interrupt::Irq);
        assert_eq!(entry.vector, 0xfffe);
        assert_eq!(entry.cycles, 7);
        // Return address pushed high byte first.
        assert_eq!(ram.data[0x01fd], 0x80);
        assert_eq!(ram.data[0x01fc], 0x00);
        // Pushed status has B clear and bit 5 set.
        assert_eq!(ram.data[0x01fb] & FLAG_BREAK, 0);
        assert_ne!(ram.data[0x01fb] & FLAG_UNUSED, 0);
        assert_eq!(cpu.state().pc, 0x9000);
        assert_eq!(cpu.state().sp, 0xfa);
        assert_ne!(cpu.state().status & FLAG_INTERRUPT_DISABLE, 0);
        assert_eq!(cpu.state().total_cycles, 7);
    }

    #[test]
    fn nmi_is_edge_triggered_and_ignores_interrupt_disable() {
        let mut ram = Ram::new();
        ram.data[0xfffa] = 0x40;
        ram.data[0xfffb] = 0x80;
        // Interrupt-disable is set, which must not mask an NMI.
        let mut cpu = Cpu::new(CpuState::at(0x8000));
        cpu.set_nmi_line(true);
        assert_eq!(cpu.pending_interrupt(), Some(Interrupt::Nmi));

        let entry = cpu
            .service_interrupt(&mut ram, Interrupt::Nmi)
            .expect("interrupt entry fits the cycle counter");

        assert_eq!(entry.vector, 0xfffa);
        assert_eq!(cpu.state().pc, 0x8040);
        assert_eq!(ram.data[0x01fb] & FLAG_BREAK, 0);
        assert_eq!(cpu.pending_interrupt(), None);
    }

    #[test]
    fn nmi_requires_a_new_edge_after_being_serviced() {
        let mut ram = Ram::new();
        ram.data[0xfffa] = 0x00;
        ram.data[0xfffb] = 0x90;
        let mut cpu = Cpu::new(CpuState::at(0x8000));
        cpu.set_nmi_line(true);
        cpu.service_interrupt(&mut ram, Interrupt::Nmi)
            .expect("first NMI is serviced");
        assert_eq!(cpu.pending_interrupt(), None);

        // The line held asserted does not re-latch without a fresh edge.
        cpu.set_nmi_line(true);
        assert_eq!(cpu.pending_interrupt(), None);

        // Deasserting and re-asserting produces a new edge.
        cpu.set_nmi_line(false);
        cpu.set_nmi_line(true);
        assert_eq!(cpu.pending_interrupt(), Some(Interrupt::Nmi));
    }

    #[test]
    fn nmi_outranks_a_simultaneous_irq() {
        let mut cpu = cpu_with_irq_enabled(0x8000);
        cpu.set_irq_line(true);
        cpu.set_nmi_line(true);
        assert_eq!(cpu.pending_interrupt(), Some(Interrupt::Nmi));
    }

    #[test]
    fn cli_delays_irq_recognition_by_one_instruction() {
        let mut ram = Ram::new();
        // CLI then NOP.
        ram.data[0x8000..0x8002].copy_from_slice(&[0x58, 0xea]);
        let mut cpu = Cpu::new(CpuState::at(0x8000));
        cpu.set_irq_line(true);
        assert_eq!(cpu.pending_interrupt(), None);

        cpu.step(&mut ram).expect("CLI executes");
        // The I flag is now clear, but the poll for CLI observed it set, so the
        // IRQ is not recognized until after the following instruction.
        assert_eq!(cpu.state().status & FLAG_INTERRUPT_DISABLE, 0);
        assert_eq!(cpu.pending_interrupt(), None);

        cpu.step(&mut ram).expect("NOP executes");
        assert_eq!(cpu.pending_interrupt(), Some(Interrupt::Irq));
    }

    #[test]
    fn sei_delays_masking_so_one_pending_irq_is_still_serviced() {
        // Follows the driver contract: poll pending_interrupt() before each step
        // and service when it returns Some. The I flag starts set, so the IRQ is
        // masked. CLI clears I, but its effect is delayed one instruction, so SEI
        // is the first instruction whose poll observes I clear. SEI sets I again,
        // yet its masking is itself delayed, so that poll recognizes the pending
        // IRQ and lets exactly one through before the line is masked.
        let mut ram = Ram::new();
        ram.data[0x8000..0x8003].copy_from_slice(&[0x58, 0x78, 0xea]); // CLI, SEI, NOP
        ram.data[0xfffe] = 0x00;
        ram.data[0xffff] = 0x90;
        let mut cpu = Cpu::new(CpuState::at(0x8000)); // power-on default has I set
        cpu.set_irq_line(true);

        assert_eq!(cpu.pending_interrupt(), None); // masked by the I flag
        cpu.step(&mut ram).expect("CLI executes");
        assert_eq!(cpu.pending_interrupt(), None); // CLI's enable is delayed one instruction
        cpu.step(&mut ram).expect("SEI executes");
        assert_ne!(cpu.state().status & FLAG_INTERRUPT_DISABLE, 0); // SEI set I again...
        assert_eq!(cpu.pending_interrupt(), Some(Interrupt::Irq)); // ...yet one IRQ still gets through

        let entry = cpu
            .service_interrupt(&mut ram, Interrupt::Irq)
            .expect("the delayed IRQ is serviced");
        assert_eq!(entry.after.pc, 0x9000);
        // The line stays asserted, but the I flag now masks it.
        cpu.step(&mut ram).expect("NOP executes");
        assert_eq!(cpu.pending_interrupt(), None);
    }

    #[test]
    fn plp_delays_irq_recognition_like_cli() {
        let mut ram = Ram::new();
        ram.data[0x8000..0x8002].copy_from_slice(&[0x28, 0xea]); // PLP then NOP
        // Stack holds a status byte with the I flag clear for PLP to pull.
        ram.data[0x01fe] = FLAG_UNUSED;
        let mut state = CpuState::at(0x8000);
        state.sp = 0xfd;
        let mut cpu = cpu_with(state);
        assert_ne!(cpu.state().status & FLAG_INTERRUPT_DISABLE, 0);
        cpu.set_irq_line(true);

        cpu.step(&mut ram).expect("PLP executes");
        assert_eq!(cpu.state().status & FLAG_INTERRUPT_DISABLE, 0);
        assert_eq!(cpu.pending_interrupt(), None);

        cpu.step(&mut ram).expect("NOP executes");
        assert_eq!(cpu.pending_interrupt(), Some(Interrupt::Irq));
    }

    #[test]
    fn rti_recognizes_a_pending_irq_immediately() {
        let mut ram = Ram::new();
        ram.data[0x8000] = 0x40; // RTI
        // Stack frame: status (I clear), then return PC 0x9000.
        ram.data[0x01fb] = FLAG_UNUSED;
        ram.data[0x01fc] = 0x00;
        ram.data[0x01fd] = 0x90;
        let mut state = CpuState::at(0x8000);
        state.sp = 0xfa;
        let mut cpu = cpu_with(state);
        assert_ne!(cpu.state().status & FLAG_INTERRUPT_DISABLE, 0);
        cpu.set_irq_line(true);

        cpu.step(&mut ram).expect("RTI executes");
        // RTI's I-flag change takes effect immediately, with no one-instruction
        // delay, so the pending IRQ is recognized right away.
        assert_eq!(cpu.state().pc, 0x9000);
        assert_eq!(cpu.state().status & FLAG_INTERRUPT_DISABLE, 0);
        assert_eq!(cpu.pending_interrupt(), Some(Interrupt::Irq));
    }

    #[test]
    fn brk_pushes_break_set_while_irq_entry_pushes_it_clear() {
        let mut brk_ram = Ram::new();
        brk_ram.data[0x8000] = 0x00; // BRK
        brk_ram.data[0xfffe] = 0x00;
        brk_ram.data[0xffff] = 0x90;
        let mut brk_cpu = Cpu::new(CpuState::at(0x8000));
        brk_cpu.step(&mut brk_ram).expect("BRK executes");
        assert_eq!(brk_cpu.state().pc, 0x9000);
        assert_ne!(brk_ram.data[0x01fb] & FLAG_BREAK, 0);

        let mut irq_ram = Ram::new();
        irq_ram.data[0xfffe] = 0x00;
        irq_ram.data[0xffff] = 0x90;
        let mut irq_cpu = cpu_with_irq_enabled(0x8000);
        irq_cpu
            .service_interrupt(&mut irq_ram, Interrupt::Irq)
            .expect("IRQ entry executes");
        assert_eq!(irq_cpu.state().pc, 0x9000);
        assert_eq!(irq_ram.data[0x01fb] & FLAG_BREAK, 0);
    }

    #[test]
    fn interrupt_entry_is_failure_atomic_at_the_cycle_ceiling() {
        let mut ram = Ram::new();
        ram.data[0xfffe] = 0x00;
        ram.data[0xffff] = 0x90;
        let before_ram = ram.data.clone();
        let mut state = CpuState::at(0x8000);
        state.status = FLAG_UNUSED;
        state.total_cycles = u64::MAX - 3;
        let mut cpu = cpu_with(state);
        cpu.set_irq_line(true);
        let before = cpu.state();

        assert_eq!(
            cpu.service_interrupt(&mut ram, Interrupt::Irq),
            Err(CpuError::CycleCounterHeadroomExhausted { remaining: 3 })
        );
        assert_eq!(cpu.state(), before);
        assert_eq!(ram.data, before_ram);
    }

    #[test]
    fn reset_is_failure_atomic_at_the_cycle_ceiling() {
        let mut ram = Ram::new();
        ram.data[0xfffc] = 0x00;
        ram.data[0xfffd] = 0xc0;
        let mut state = CpuState::at(0x1234);
        state.total_cycles = u64::MAX - 3;
        let mut cpu = cpu_with(state);
        let before = cpu.state();

        assert_eq!(
            cpu.reset(&mut ram),
            Err(CpuError::CycleCounterHeadroomExhausted { remaining: 3 })
        );
        assert_eq!(cpu.state(), before);
    }
}
