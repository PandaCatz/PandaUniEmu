use crate::{CpuBusFault, NesCartridge, NromCpuBus, NtscScheduler, TimedPpuEvent, TimingError};
use cpu_6502::{ClockOutcome, Cpu, CpuError, CpuState};
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachineCycle {
    pub cpu: ClockOutcome,
    pub ppu_event: Option<TimedPpuEvent>,
    pub bus_fault: Option<CpuBusFault>,
}

/// The first machine-owned NES boundary. One call performs one live CPU bus
/// access and advances the fixed-phase NTSC scheduler by the same CPU cycle.
#[derive(Clone, Debug)]
pub struct NesMachine {
    cpu: Cpu,
    bus: NromCpuBus,
    scheduler: NtscScheduler,
}

impl NesMachine {
    #[must_use]
    pub fn new(cartridge: NesCartridge, cpu_state: CpuState) -> Self {
        Self {
            cpu: Cpu::new(cpu_state),
            bus: NromCpuBus::new(cartridge),
            scheduler: NtscScheduler::default(),
        }
    }

    #[must_use]
    pub const fn cpu(&self) -> &Cpu {
        &self.cpu
    }

    #[must_use]
    pub const fn bus(&self) -> &NromCpuBus {
        &self.bus
    }

    #[must_use]
    pub const fn scheduler(&self) -> &NtscScheduler {
        &self.scheduler
    }

    pub fn set_nmi_line(&mut self, asserted: bool) {
        self.cpu.set_nmi_line(asserted);
    }

    pub fn set_irq_line(&mut self, asserted: bool) {
        self.cpu.set_irq_line(asserted);
    }

    /// Clocks one live CPU bus access. Timing is precomputed on a clone and only
    /// committed if the CPU cycle succeeds, so timing overflow cannot leave the
    /// CPU and scheduler in different cycles.
    pub fn clock(&mut self) -> Result<MachineCycle, MachineError> {
        let mut next_scheduler = self.scheduler.clone();
        let ppu_event = next_scheduler.advance_cpu_cycle()?;
        let cpu = self.cpu.clock(&mut self.bus)?;
        self.scheduler = next_scheduler;
        Ok(MachineCycle {
            cpu,
            ppu_event,
            bus_fault: self.bus.take_fault(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MachineError {
    Cpu(CpuError),
    Timing(TimingError),
}

impl Display for MachineError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cpu(error) => write!(formatter, "NES CPU cycle failed: {error}"),
            Self::Timing(error) => write!(formatter, "NES timing cycle failed: {error}"),
        }
    }
}

impl Error for MachineError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Cpu(error) => Some(error),
            Self::Timing(error) => Some(error),
        }
    }
}

impl From<CpuError> for MachineError {
    fn from(error: CpuError) -> Self {
        Self::Cpu(error)
    }
}

impl From<TimingError> for MachineError {
    fn from(error: TimingError) -> Self {
        Self::Timing(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MASTER_TICKS_PER_CPU_CYCLE;

    fn cartridge_with_program(program: &[u8]) -> NesCartridge {
        let mut bytes = vec![0; 16 + 16 * 1024 + 8 * 1024];
        bytes[0..4].copy_from_slice(b"NES\x1a");
        bytes[4] = 1;
        bytes[5] = 1;
        bytes[16..16 + 16 * 1024].fill(0xea);
        bytes[16..16 + program.len()].copy_from_slice(program);
        let parsed = format_ines::parse(&bytes).expect("generated NROM image parses");
        NesCartridge::from_parsed(parsed).expect("generated NROM cartridge is supported")
    }

    #[test]
    fn each_machine_call_clocks_one_cpu_cycle_and_three_ppu_dots() {
        let mut machine = NesMachine::new(cartridge_with_program(&[0xea]), CpuState::at(0x8000));

        let first = machine.clock().expect("opcode fetch succeeds");
        assert_eq!(first.cpu, ClockOutcome::InProgress);
        assert_eq!(first.ppu_event, None);
        assert_eq!(machine.cpu().state().total_cycles, 1);
        assert_eq!(
            machine.scheduler().master_ticks(),
            MASTER_TICKS_PER_CPU_CYCLE
        );
        assert_eq!(machine.scheduler().ppu().position().dot, 3);

        let second = machine.clock().expect("NOP dummy read succeeds");
        let ClockOutcome::InstructionComplete(trace) = second.cpu else {
            panic!("NOP must complete on cycle two");
        };
        assert_eq!(trace.cycles, 2);
        assert_eq!(machine.cpu().state().total_cycles, 2);
        assert_eq!(
            machine.scheduler().master_ticks(),
            2 * MASTER_TICKS_PER_CPU_CYCLE
        );
        assert_eq!(machine.scheduler().ppu().position().dot, 6);
    }

    #[test]
    fn unsupported_io_fault_is_reported_on_its_exact_cpu_cycle() {
        let mut machine = NesMachine::new(
            cartridge_with_program(&[0x8d, 0x00, 0x20]),
            CpuState::at(0x8000),
        );

        for _ in 0..3 {
            let cycle = machine.clock().expect("STA setup cycle succeeds");
            assert_eq!(cycle.bus_fault, None);
        }
        let write = machine.clock().expect("STA write cycle completes safely");
        assert!(matches!(write.cpu, ClockOutcome::InstructionComplete(_)));
        assert_eq!(
            write.bus_fault,
            Some(CpuBusFault::UnsupportedWrite {
                address: 0x2000,
                value: 0,
            })
        );
        assert_eq!(
            machine.scheduler().master_ticks(),
            4 * MASTER_TICKS_PER_CPU_CYCLE
        );
    }

    #[test]
    fn unsupported_opcode_fetch_keeps_cpu_and_scheduler_in_lockstep() {
        let mut machine = NesMachine::new(cartridge_with_program(&[0x02]), CpuState::at(0x8000));

        let cycle = machine.clock().expect("unsupported fetch is observable");
        assert_eq!(
            cycle.cpu,
            ClockOutcome::UnsupportedOpcode {
                pc: 0x8000,
                opcode: 0x02,
            }
        );
        assert_eq!(cycle.bus_fault, None);
        assert_eq!(machine.cpu().state().pc, 0x8001);
        assert_eq!(machine.cpu().state().total_cycles, 1);
        assert_eq!(
            machine.scheduler().master_ticks(),
            MASTER_TICKS_PER_CPU_CYCLE
        );
        assert_eq!(machine.scheduler().ppu().position().dot, 3);
    }

    #[test]
    fn timing_overflow_leaves_machine_cpu_bus_and_scheduler_unchanged() {
        let mut machine = NesMachine::new(cartridge_with_program(&[]), CpuState::at(0x8000));
        machine
            .scheduler
            .set_master_ticks_for_test(u64::MAX - MASTER_TICKS_PER_CPU_CYCLE + 1);
        let before_cpu = machine.cpu.clone();
        let before_bus = machine.bus.clone();
        let before_scheduler = machine.scheduler.clone();

        assert_eq!(
            machine.clock(),
            Err(MachineError::Timing(TimingError::MasterTickOverflow))
        );
        assert_eq!(machine.cpu, before_cpu);
        assert_eq!(machine.bus.cpu_ram(), before_bus.cpu_ram());
        assert_eq!(machine.bus.prg_ram(), before_bus.prg_ram());
        assert_eq!(machine.bus.take_fault(), before_bus.clone().take_fault());
        assert_eq!(machine.scheduler, before_scheduler);
    }

    #[test]
    fn vblank_edge_is_visible_on_the_cpu_cycle_that_crosses_its_dot() {
        let mut machine = NesMachine::new(cartridge_with_program(&[]), CpuState::at(0x8000));
        let expected_tick = 328_728;
        let mut calls = 0_u64;

        loop {
            let cycle = machine.clock().expect("NOP stream clocks");
            calls += 1;
            if let Some(event) = cycle.ppu_event {
                assert_eq!(event.master_tick, expected_tick);
                assert_eq!(event.event, crate::PpuEvent::VblankStarted);
                assert_eq!(calls, expected_tick / MASTER_TICKS_PER_CPU_CYCLE + 1);
                assert_eq!(machine.cpu().state().total_cycles, calls);
                assert_eq!(
                    machine.scheduler().master_ticks(),
                    calls * MASTER_TICKS_PER_CPU_CYCLE
                );
                assert_eq!(machine.scheduler().ppu().position().scanline, 241);
                assert_eq!(machine.scheduler().ppu().position().dot, 4);
                break;
            }
        }
    }
}
