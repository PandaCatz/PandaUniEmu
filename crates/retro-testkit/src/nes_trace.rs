use core_nes::{CpuBusFault, NesCartridge, NromCpuBus};
use cpu_6502::{Cpu, CpuError, CpuState, decode};
use format_ines::Region;
use format_nestest_log::{ReferenceLog, ReferenceRow};
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceSummary {
    pub rows_matched: usize,
    pub transitions_verified: usize,
    pub final_state: CpuState,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceFailure {
    UnsupportedRegion(Region),
    InitialStateNotRepresentable {
        line: usize,
        expected: CpuState,
        normalized: CpuState,
    },
    StateMismatch {
        line: usize,
        expected: CpuState,
        actual: CpuState,
        previous_expected: Option<CpuState>,
    },
    OpcodeMismatch {
        line: usize,
        pc: u16,
        expected: Vec<u8>,
        actual: Vec<u8>,
    },
    OpcodeLengthMismatch {
        line: usize,
        opcode: u8,
        expected: u8,
        actual: u8,
    },
    Cpu {
        line: usize,
        source: CpuError,
    },
    Bus {
        line: usize,
        source: CpuBusFault,
    },
}

impl Display for TraceFailure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedRegion(region) => {
                write!(
                    formatter,
                    "nestest comparison requires NTSC, got {region:?}"
                )
            }
            Self::InitialStateNotRepresentable { line, .. } => write!(
                formatter,
                "reference CPU state at line {line} cannot be represented"
            ),
            Self::StateMismatch { line, .. } => {
                write!(formatter, "CPU state mismatch at reference line {line}")
            }
            Self::OpcodeMismatch { line, pc, .. } => write!(
                formatter,
                "opcode-byte mismatch at reference line {line}, PC ${pc:04X}"
            ),
            Self::OpcodeLengthMismatch { line, .. } => {
                write!(
                    formatter,
                    "opcode-byte length mismatch at reference line {line}"
                )
            }
            Self::Cpu { line, source } => {
                write!(formatter, "CPU error at reference line {line}: {source}")
            }
            Self::Bus { line, source } => {
                write!(formatter, "bus error at reference line {line}: {source}")
            }
        }
    }
}

impl Error for TraceFailure {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Cpu { source, .. } => Some(source),
            Self::Bus { source, .. } => Some(source),
            _ => None,
        }
    }
}

pub fn compare_nrom_trace(
    cartridge: NesCartridge,
    reference: &ReferenceLog,
) -> Result<TraceSummary, TraceFailure> {
    if cartridge.region() != Region::Ntsc {
        return Err(TraceFailure::UnsupportedRegion(cartridge.region()));
    }
    let rows = reference.rows();
    debug_assert!(!rows.is_empty(), "validated reference logs are nonempty");
    let initial = state_from_row(&rows[0]);
    let mut cpu = Cpu::new(initial);
    if cpu.state() != initial {
        return Err(TraceFailure::InitialStateNotRepresentable {
            line: rows[0].line,
            expected: initial,
            normalized: cpu.state(),
        });
    }
    let mut bus = NromCpuBus::new(cartridge);
    let mut previous_expected = None;

    for (row_index, row) in rows.iter().enumerate() {
        let expected = state_from_row(row);
        let actual = cpu.state();
        if actual != expected {
            return Err(TraceFailure::StateMismatch {
                line: row.line,
                expected,
                actual,
                previous_expected,
            });
        }

        if let Some(instruction) = decode(row.opcode_bytes()[0]) {
            let expected_len = instruction.instruction_bytes();
            let actual_len = row.opcode_bytes().len() as u8;
            if actual_len != expected_len {
                return Err(TraceFailure::OpcodeLengthMismatch {
                    line: row.line,
                    opcode: row.opcode_bytes()[0],
                    expected: expected_len,
                    actual: actual_len,
                });
            }
        }

        let mut actual_opcode = Vec::with_capacity(row.opcode_bytes().len());
        for offset in 0..row.opcode_bytes().len() {
            let address = row.pc.wrapping_add(offset as u16);
            let value = bus.peek(address).map_err(|source| TraceFailure::Bus {
                line: row.line,
                source,
            })?;
            actual_opcode.push(value);
        }
        if actual_opcode != row.opcode_bytes() {
            return Err(TraceFailure::OpcodeMismatch {
                line: row.line,
                pc: row.pc,
                expected: row.opcode_bytes().to_vec(),
                actual: actual_opcode,
            });
        }

        if row_index + 1 == rows.len() {
            break;
        }

        let step_result = cpu.step(&mut bus);
        if let Some(source) = bus.take_fault() {
            return Err(TraceFailure::Bus {
                line: row.line,
                source,
            });
        }
        step_result.map_err(|source| TraceFailure::Cpu {
            line: row.line,
            source,
        })?;
        previous_expected = Some(expected);
    }

    Ok(TraceSummary {
        rows_matched: rows.len(),
        transitions_verified: rows.len().saturating_sub(1),
        final_state: cpu.state(),
    })
}

const fn state_from_row(row: &ReferenceRow) -> CpuState {
    CpuState {
        a: row.a,
        x: row.x,
        y: row.y,
        sp: row.sp,
        pc: row.pc,
        status: row.status,
        total_cycles: row.cycles,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cartridge_with_program(program: &[u8], pal: bool) -> NesCartridge {
        let mut bytes = vec![0; 16 + 16 * 1024 + 8 * 1024];
        bytes[0..4].copy_from_slice(b"NES\x1a");
        bytes[4] = 1;
        bytes[5] = 1;
        bytes[9] = u8::from(pal);
        bytes[16..16 + program.len()].copy_from_slice(program);
        let parsed = format_ines::parse(&bytes).expect("generated NROM image parses");
        NesCartridge::from_parsed(parsed).expect("generated NROM cartridge is supported")
    }

    fn parse_log(rows: &str) -> ReferenceLog {
        format_nestest_log::parse(rows.as_bytes()).expect("generated reference log parses")
    }

    #[test]
    fn generated_trace_uses_the_final_row_as_a_verified_end_state() {
        let cartridge = cartridge_with_program(&[0xa9, 0x01, 0xaa, 0xea], false);
        let log = parse_log(
            "C000 A9 01 LDA A:00 X:00 Y:00 P:24 SP:FD CYC:7\n\
             C002 AA TAX A:01 X:00 Y:00 P:24 SP:FD CYC:9\n\
             C003 EA NOP A:01 X:01 Y:00 P:24 SP:FD CYC:11",
        );
        let summary = compare_nrom_trace(cartridge, &log).expect("generated trace matches");
        assert_eq!(summary.rows_matched, 3);
        assert_eq!(summary.transitions_verified, 2);
        assert_eq!(summary.final_state.pc, 0xc003);
        assert_eq!(summary.final_state.x, 1);
        assert_eq!(summary.final_state.total_cycles, 11);
    }

    #[test]
    fn reports_first_state_and_opcode_mismatch() {
        let state_cartridge = cartridge_with_program(&[0xea, 0xea], false);
        let state_log = parse_log(
            "C000 EA NOP A:00 X:00 Y:00 P:24 SP:FD CYC:7\n\
             C001 EA NOP A:01 X:00 Y:00 P:24 SP:FD CYC:9",
        );
        assert!(matches!(
            compare_nrom_trace(state_cartridge, &state_log),
            Err(TraceFailure::StateMismatch { line: 2, .. })
        ));

        let opcode_cartridge = cartridge_with_program(&[0xea], false);
        let opcode_log = parse_log("C000 A8 TAY A:00 X:00 Y:00 P:24 SP:FD CYC:7");
        assert!(matches!(
            compare_nrom_trace(opcode_cartridge, &opcode_log),
            Err(TraceFailure::OpcodeMismatch { line: 1, .. })
        ));

        let length_cartridge = cartridge_with_program(&[0xa9, 0x01], false);
        let length_log = parse_log("C000 A9 LDA A:00 X:00 Y:00 P:24 SP:FD CYC:7");
        assert_eq!(
            compare_nrom_trace(length_cartridge, &length_log),
            Err(TraceFailure::OpcodeLengthMismatch {
                line: 1,
                opcode: 0xa9,
                expected: 2,
                actual: 1,
            })
        );
    }

    #[test]
    fn rejects_unrepresentable_status_and_non_ntsc_cartridge() {
        let cartridge = cartridge_with_program(&[0xea], false);
        let log = parse_log("C000 EA NOP A:00 X:00 Y:00 P:10 SP:FD CYC:7");
        assert!(matches!(
            compare_nrom_trace(cartridge, &log),
            Err(TraceFailure::InitialStateNotRepresentable { line: 1, .. })
        ));

        let pal = cartridge_with_program(&[0xea], true);
        let log = parse_log("C000 EA NOP A:00 X:00 Y:00 P:24 SP:FD CYC:7");
        assert_eq!(
            compare_nrom_trace(pal, &log),
            Err(TraceFailure::UnsupportedRegion(Region::Pal))
        );
    }

    #[test]
    fn fails_on_unsupported_device_access() {
        let cartridge = cartridge_with_program(&[0xad, 0x00, 0x20, 0xea], false);
        let log = parse_log(
            "C000 AD 00 20 LDA A:00 X:00 Y:00 P:24 SP:FD CYC:7\n\
             C003 EA NOP A:00 X:00 Y:00 P:24 SP:FD CYC:11",
        );
        assert_eq!(
            compare_nrom_trace(cartridge, &log),
            Err(TraceFailure::Bus {
                line: 1,
                source: CpuBusFault::UnsupportedRead { address: 0x2000 },
            })
        );
    }
}
