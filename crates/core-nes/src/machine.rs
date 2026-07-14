use crate::{
    CpuBusFault, NesCartridge, NromCpuBus, NtscScheduler, Ppu, PpuBusAccess, TimedPpuEvent,
    TimingError,
};
use cpu_6502::{Bus, ClockOutcome, Cpu, CpuError, CpuState};
use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachineCycle {
    pub cpu: ClockOutcome,
    pub ppu_event: Option<TimedPpuEvent>,
    /// Background-fetch activity for the three elapsed dots. Sprite/garbage
    /// fetches at dots 257-320 are intentionally absent at this checkpoint.
    pub ppu_accesses: [Option<PpuBusAccess>; 3],
    pub bus_fault: Option<CpuBusFault>,
}

/// The first machine-owned NES boundary. One call performs one live CPU bus
/// access and advances the fixed-phase NTSC scheduler by the same CPU cycle.
#[derive(Clone, Debug)]
pub struct NesMachine {
    cpu: Cpu,
    bus: NromCpuBus,
    ppu: Ppu,
    scheduler: NtscScheduler,
    external_nmi_line: bool,
}

impl NesMachine {
    #[must_use]
    pub fn new(cartridge: NesCartridge, cpu_state: CpuState) -> Self {
        let ppu = Ppu::new();
        Self {
            cpu: Cpu::new(cpu_state),
            bus: NromCpuBus::new(cartridge),
            ppu,
            scheduler: NtscScheduler::default(),
            external_nmi_line: false,
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
    pub const fn ppu(&self) -> &Ppu {
        &self.ppu
    }

    #[must_use]
    pub const fn scheduler(&self) -> &NtscScheduler {
        &self.scheduler
    }

    #[must_use]
    pub fn peek_ppu_memory(&self, address: u16) -> u8 {
        let address = address & 0x3fff;
        if address >= 0x3f00 {
            self.ppu.peek_palette(address)
        } else {
            self.bus.peek_ppu(address)
        }
    }

    pub fn set_nmi_line(&mut self, asserted: bool) {
        self.external_nmi_line = asserted;
        self.cpu
            .set_nmi_line(self.external_nmi_line || self.ppu.nmi_output());
    }

    pub fn set_irq_line(&mut self, asserted: bool) {
        self.cpu.set_irq_line(asserted);
    }

    /// Asserts the front-loader NES reset signal and schedules the seven-cycle
    /// CPU reset sequence. PPU memory and its current VRAM address survive, but
    /// resettable registers and PPU timing return to their deterministic reset
    /// state. The hardware write-ignore warmup window is not modeled yet.
    pub fn begin_reset(&mut self) -> Result<(), MachineError> {
        self.cpu.begin_reset()?;
        self.ppu.reset_registers();
        self.scheduler.reset_ppu_timing();
        self.cpu
            .set_nmi_line(self.external_nmi_line || self.ppu.nmi_output());
        Ok(())
    }

    /// Schedules a CPU-only reset, matching the Famicom/top-loader reset wiring.
    pub fn begin_cpu_reset(&mut self) -> Result<(), MachineError> {
        self.cpu.begin_reset().map_err(MachineError::from)
    }

    /// Clocks one live CPU bus access. The fixed phase advances three PPU dots,
    /// applies their logical event, and then performs the CPU access. CPU, PPU,
    /// and timing are committed together only after that access succeeds.
    pub fn clock(&mut self) -> Result<MachineCycle, MachineError> {
        let mut next_cpu = self.cpu.clone();
        let mut prepared_cpu = next_cpu.prepare_clock()?;

        let mut scheduler_with_rendering = self.scheduler.clone();
        scheduler_with_rendering
            .ppu_mut()
            .set_rendering_enabled(self.ppu.rendering_enabled());
        let (mut next_scheduler, dots) = scheduler_with_rendering.plan_cpu_cycle()?;

        let mut next_ppu = self.ppu.clone();
        let mut ppu_event = None;
        let mut ppu_accesses = [None; 3];
        for (index, dot) in dots.into_iter().enumerate() {
            ppu_accesses[index] = next_ppu.clock_dot(&mut self.bus, dot.position);
            if let Some(event) = dot.event {
                next_ppu.apply_event(event);
                ppu_event = Some(TimedPpuEvent {
                    master_tick: dot.master_tick,
                    event,
                });
            }
        }
        next_ppu.sync_cpu_position(next_scheduler.ppu().position());

        prepared_cpu.set_nmi_line(self.external_nmi_line || next_ppu.nmi_output());
        let cpu = {
            let mut bus = MachineCpuBus {
                nrom: &mut self.bus,
                ppu: &mut next_ppu,
            };
            prepared_cpu.clock(&mut bus)
        };
        next_cpu.set_nmi_line(self.external_nmi_line || next_ppu.nmi_output());
        next_scheduler
            .ppu_mut()
            .set_rendering_enabled(next_ppu.rendering_enabled());

        self.cpu = next_cpu;
        self.ppu = next_ppu;
        self.scheduler = next_scheduler;
        Ok(MachineCycle {
            cpu,
            ppu_event,
            ppu_accesses,
            bus_fault: self.bus.take_fault(),
        })
    }
}

struct MachineCpuBus<'a> {
    nrom: &'a mut NromCpuBus,
    ppu: &'a mut Ppu,
}

impl Bus for MachineCpuBus<'_> {
    fn read(&mut self, address: u16) -> u8 {
        if matches!(address, 0x2000..=0x3fff) {
            let value = self
                .ppu
                .cpu_read_register(self.nrom, (address & 0x0007) as u8);
            self.nrom.observe_open_bus(value);
            value
        } else {
            self.nrom.read(address)
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        if matches!(address, 0x2000..=0x3fff) {
            self.nrom.observe_open_bus(value);
            self.ppu
                .cpu_write_register(self.nrom, (address & 0x0007) as u8, value);
        } else {
            self.nrom.write(address, value);
        }
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
    use cpu_6502::{FLAG_INTERRUPT_DISABLE, FLAG_UNUSED, Interrupt};

    fn cartridge_with_program(program: &[u8]) -> NesCartridge {
        let mut bytes = vec![0; 16 + 16 * 1024 + 8 * 1024];
        bytes[0..4].copy_from_slice(b"NES\x1a");
        bytes[4] = 1;
        bytes[5] = 1;
        bytes[16..16 + 16 * 1024].fill(0xea);
        bytes[16..16 + program.len()].copy_from_slice(program);
        bytes[16 + 0x3ffa..16 + 0x3ffc].copy_from_slice(&0xa000_u16.to_le_bytes());
        bytes[16 + 0x3ffc..16 + 0x3ffe].copy_from_slice(&0x8000_u16.to_le_bytes());
        bytes[16 + 0x3ffe..16 + 0x4000].copy_from_slice(&0x9000_u16.to_le_bytes());
        let parsed = format_ines::parse(&bytes).expect("generated NROM image parses");
        NesCartridge::from_parsed(parsed).expect("generated NROM cartridge is supported")
    }

    fn write_ppu_register(machine: &mut NesMachine, register: u8, value: u8) {
        machine
            .ppu
            .cpu_write_register(&mut machine.bus, register, value);
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
    fn mirrored_ppu_register_write_is_routed_on_its_exact_cpu_cycle() {
        let mut machine = NesMachine::new(
            cartridge_with_program(&[0xa9, 0x80, 0x8d, 0x08, 0x20]),
            CpuState::at(0x8000),
        );

        for _ in 0..5 {
            let cycle = machine.clock().expect("LDA/STA setup cycle succeeds");
            assert_eq!(cycle.bus_fault, None);
        }
        let write = machine.clock().expect("STA write cycle completes safely");
        assert!(matches!(write.cpu, ClockOutcome::InstructionComplete(_)));
        assert_eq!(write.bus_fault, None);
        assert_eq!(machine.ppu().control(), 0x80);
        assert_eq!(
            machine.scheduler().master_ticks(),
            6 * MASTER_TICKS_PER_CPU_CYCLE
        );
    }

    #[test]
    fn ppumask_write_commits_matching_ppu_and_scheduler_rendering_state() {
        let mut machine = NesMachine::new(
            cartridge_with_program(&[0xa9, 0x08, 0x8d, 0x01, 0x20]),
            CpuState::at(0x8000),
        );
        for _ in 0..6 {
            machine.clock().expect("LDA/STA program clocks");
        }
        assert!(machine.ppu().rendering_enabled());
        assert!(machine.scheduler().ppu().rendering_enabled());
    }

    #[test]
    fn current_immediate_ppumask_model_only_enables_later_ppu_dots() {
        let program = [0xa9, 0x08, 0x8d, 0x01, 0x20, 0xea];
        let mut machine = NesMachine::new(cartridge_with_program(&program), CpuState::at(0x8000));
        for _ in 0..5 {
            let cycle = machine.clock().expect("setup cycle clocks");
            assert_eq!(cycle.ppu_accesses, [None; 3]);
        }
        let write_cycle = machine.clock().expect("PPUMASK write cycle clocks");
        assert_eq!(write_cycle.ppu_accesses, [None; 3]);
        assert!(machine.ppu.rendering_enabled());

        let following = machine
            .clock()
            .expect("following cycle clocks rendered dots");
        assert_eq!(following.ppu_accesses[0], None);
        let address = following.ppu_accesses[1].expect("dot 19 drives the attribute address");
        assert_eq!(address.position.dot, 19);
        assert_eq!(address.kind, crate::PpuFetchKind::Attribute);
        assert_eq!(address.phase, crate::PpuBusPhase::Address);
        let read = following.ppu_accesses[2].expect("dot 20 reads the attribute byte");
        assert_eq!(read.position.dot, 20);
        assert_eq!(read.kind, crate::PpuFetchKind::Attribute);
        assert_eq!(read.phase, crate::PpuBusPhase::Read);
    }

    #[test]
    fn every_ppu_register_decodes_identically_at_the_top_mirror() {
        for register in 0_u16..8 {
            let cartridge = cartridge_with_program(&[]);
            let mut expected_bus = NromCpuBus::new(cartridge.clone());
            let mut expected_ppu = Ppu::new();
            let mut mirrored_bus = NromCpuBus::new(cartridge.clone());
            let mut mirrored_ppu = Ppu::new();
            let value = 0x40 | register as u8;

            MachineCpuBus {
                nrom: &mut expected_bus,
                ppu: &mut expected_ppu,
            }
            .write(0x2000 + register, value);
            MachineCpuBus {
                nrom: &mut mirrored_bus,
                ppu: &mut mirrored_ppu,
            }
            .write(0x3ff8 + register, value);
            assert_eq!(mirrored_ppu, expected_ppu, "register {register}");
            assert_eq!(mirrored_bus, expected_bus, "register {register} bus state");
        }

        let cartridge = cartridge_with_program(&[]);
        let mut nrom = NromCpuBus::new(cartridge.clone());
        let mut ppu = Ppu::new();
        MachineCpuBus {
            nrom: &mut nrom,
            ppu: &mut ppu,
        }
        .write(0x4000, 0x5a);
        assert_eq!(
            nrom.take_fault(),
            Some(CpuBusFault::UnsupportedWrite {
                address: 0x4000,
                value: 0x5a,
            })
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
    fn machine_exposes_ppu_address_then_read_before_the_cpu_bus_cycle() {
        let mut machine = NesMachine::new(cartridge_with_program(&[0xea]), CpuState::at(0x8000));
        write_ppu_register(&mut machine, 1, 0x08);

        let cycle = machine
            .clock()
            .expect("first rendered machine cycle clocks");
        assert_eq!(cycle.ppu_accesses[0], None);
        let address = cycle.ppu_accesses[1].expect("dot 1 drives the nametable address");
        assert_eq!(address.position.dot, 1);
        assert_eq!(address.kind, crate::PpuFetchKind::Nametable);
        assert_eq!(address.phase, crate::PpuBusPhase::Address);
        assert_eq!(address.address, 0x2000);
        assert_eq!(address.value, None);
        let read = cycle.ppu_accesses[2].expect("dot 2 reads that nametable byte");
        assert_eq!(read.position.dot, 2);
        assert_eq!(read.kind, crate::PpuFetchKind::Nametable);
        assert_eq!(read.phase, crate::PpuBusPhase::Read);
        assert_eq!(read.address, 0x2000);
        assert_eq!(read.value, Some(0));
        assert_eq!(machine.scheduler().ppu().position().dot, 3);
        assert_eq!(machine.cpu().state().total_cycles, 1);
    }

    #[test]
    fn warm_reset_clocks_seven_machine_cycles_in_lockstep() {
        let mut state = CpuState::at(0x8005);
        state.status &= !FLAG_INTERRUPT_DISABLE;
        let mut machine = NesMachine::new(cartridge_with_program(&[]), state);
        machine.begin_reset().expect("reset scheduling succeeds");

        for cycle_index in 1..=7 {
            let cycle = machine.clock().expect("reset bus cycle succeeds");
            assert_eq!(cycle.bus_fault, None);
            if cycle_index < 7 {
                assert_eq!(cycle.cpu, ClockOutcome::InProgress);
            } else {
                let ClockOutcome::ResetComplete(entry) = cycle.cpu else {
                    panic!("reset must complete on its seventh live bus cycle");
                };
                assert_eq!(entry.cycles, 7);
                assert_eq!(entry.before.pc, 0x8005);
                assert_eq!(entry.after.pc, 0x8000);
                assert_eq!(entry.after.sp, 0xfa);
                assert_ne!(entry.after.status & FLAG_INTERRUPT_DISABLE, 0);
            }
        }

        assert_eq!(machine.cpu().state().total_cycles, 7);
        assert_eq!(
            machine.scheduler().master_ticks(),
            7 * MASTER_TICKS_PER_CPU_CYCLE
        );
        assert_eq!(machine.scheduler().ppu().position().dot, 21);
    }

    #[test]
    fn front_loader_reset_resets_ppu_registers_and_timing_but_not_master_time() {
        let mut machine = NesMachine::new(cartridge_with_program(&[]), CpuState::at(0x8000));
        write_ppu_register(&mut machine, 0, 0x80);
        write_ppu_register(&mut machine, 1, 0x18);
        write_ppu_register(&mut machine, 5, 0xff);
        machine
            .scheduler
            .advance_cpu_cycles(3)
            .expect("test scheduler has headroom");
        let master_ticks = machine.scheduler.master_ticks();

        machine.begin_reset().expect("front-loader reset starts");

        assert_eq!(machine.ppu.control(), 0);
        assert_eq!(machine.ppu.mask(), 0);
        assert_eq!(machine.ppu.temporary_address(), 0);
        assert_eq!(machine.ppu.fine_x(), 0);
        assert!(!machine.ppu.write_toggle());
        assert_eq!(machine.scheduler.master_ticks(), master_ticks);
        assert_eq!(
            machine.scheduler.ppu().position(),
            crate::PpuPosition::default()
        );
        assert!(!machine.scheduler.ppu().rendering_enabled());
    }

    #[test]
    fn cpu_only_reset_preserves_famicom_ppu_state() {
        let mut machine = NesMachine::new(cartridge_with_program(&[]), CpuState::at(0x8000));
        write_ppu_register(&mut machine, 0, 0x80);
        write_ppu_register(&mut machine, 1, 0x18);
        for _ in 0..2 {
            machine
                .clock()
                .expect("rendering pipeline and one NOP instruction clock");
        }
        let before_ppu = machine.ppu.clone();
        let before_bus = machine.bus.clone();

        machine.begin_cpu_reset().expect("CPU-only reset starts");

        assert_eq!(machine.ppu, before_ppu);
        assert_eq!(machine.bus, before_bus);
    }

    #[test]
    fn accepted_irq_enters_automatically_through_the_machine_scheduler() {
        let mut state = CpuState::at(0x8000);
        state.status = FLAG_UNUSED;
        let mut machine = NesMachine::new(cartridge_with_program(&[0xea, 0xea]), state);
        machine.set_irq_line(true);

        assert_eq!(
            machine.clock().expect("NOP fetch succeeds").cpu,
            ClockOutcome::InProgress
        );
        assert!(matches!(
            machine.clock().expect("NOP completion succeeds").cpu,
            ClockOutcome::InstructionComplete(_)
        ));

        for entry_cycle in 1..=7 {
            let cycle = machine.clock().expect("IRQ entry cycle succeeds");
            assert_eq!(cycle.bus_fault, None);
            if entry_cycle < 7 {
                assert_eq!(cycle.cpu, ClockOutcome::InProgress);
            } else {
                let ClockOutcome::InterruptComplete(entry) = cycle.cpu else {
                    panic!("IRQ must complete on its seventh live bus cycle");
                };
                assert_eq!(entry.origin, Interrupt::Irq);
                assert_eq!(entry.kind, Interrupt::Irq);
                assert_eq!(entry.vector, 0xfffe);
                assert_eq!(entry.cycles, 7);
                assert_eq!(entry.before.pc, 0x8001);
                assert_eq!(entry.after.pc, 0x9000);
            }
        }

        assert_eq!(machine.cpu().state().total_cycles, 9);
        assert_eq!(
            machine.scheduler().master_ticks(),
            9 * MASTER_TICKS_PER_CPU_CYCLE
        );
        assert_eq!(machine.scheduler().ppu().position().dot, 27);
    }

    #[test]
    fn timing_overflow_leaves_machine_cpu_bus_and_scheduler_unchanged() {
        let mut machine = NesMachine::new(cartridge_with_program(&[]), CpuState::at(0x8000));
        machine
            .scheduler
            .set_master_ticks_for_test(u64::MAX - MASTER_TICKS_PER_CPU_CYCLE + 1);
        let before_cpu = machine.cpu.clone();
        let before_bus = machine.bus.clone();
        let before_ppu = machine.ppu.clone();
        let before_scheduler = machine.scheduler.clone();

        assert_eq!(
            machine.clock(),
            Err(MachineError::Timing(TimingError::MasterTickOverflow))
        );
        assert_eq!(machine.cpu, before_cpu);
        assert_eq!(machine.bus, before_bus);
        assert_eq!(machine.ppu, before_ppu);
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
                assert_ne!(machine.ppu().status() & 0x80, 0);
                break;
            }
        }
    }

    #[test]
    fn timed_vblank_drives_one_automatic_nmi_entry() {
        let program = [0xa9, 0x80, 0x8d, 0x00, 0x20, 0xea, 0xea];
        let mut machine = NesMachine::new(cartridge_with_program(&program), CpuState::at(0x8000));
        let mut vblank_events = 0;
        let mut nmi_entries = 0;

        for _ in 0..30_000 {
            let cycle = machine.clock().expect("synthetic NROM stream clocks");
            if matches!(
                cycle.ppu_event.map(|event| event.event),
                Some(crate::PpuEvent::VblankStarted)
            ) {
                vblank_events += 1;
            }
            if let ClockOutcome::InterruptComplete(entry) = cycle.cpu {
                nmi_entries += 1;
                assert_eq!(entry.origin, Interrupt::Nmi);
                assert_eq!(entry.kind, Interrupt::Nmi);
                assert_eq!(entry.vector, 0xfffa);
                assert_eq!(entry.after.pc, 0xa000);
                assert_eq!(vblank_events, 1);
                assert!(machine.ppu().nmi_output());
            }
            if matches!(
                cycle.ppu_event.map(|event| event.event),
                Some(crate::PpuEvent::VblankEnded)
            ) {
                assert_eq!(nmi_entries, 1);
                assert!(!machine.ppu().nmi_output());
                return;
            }
        }

        panic!("first VBlank did not finish within the expected frame budget");
    }

    #[test]
    fn status_read_lowers_ppu_nmi_without_erasing_the_cpu_latched_edge() {
        let program = [
            0xa9, 0x80, // LDA #$80
            0x8d, 0x00, 0x20, // STA $2000
            0xad, 0x02, 0x20, // LDA $2002
            0xea,
        ];
        let mut machine = NesMachine::new(cartridge_with_program(&program), CpuState::at(0x8000));
        machine.ppu.apply_event(crate::PpuEvent::VblankStarted);

        let mut saw_status_read = false;
        for _ in 0..24 {
            let cycle = machine.clock().expect("register program clocks");
            if matches!(
                &cycle.cpu,
                ClockOutcome::InstructionComplete(trace) if trace.before.pc == 0x8005
            ) && !saw_status_read
            {
                saw_status_read = true;
                assert_eq!(machine.cpu().state().a, 0x80);
                assert_eq!(machine.ppu().status() & 0x80, 0);
                assert!(!machine.ppu().nmi_output());
            }
            if let ClockOutcome::InterruptComplete(entry) = cycle.cpu {
                assert!(saw_status_read);
                assert_eq!(entry.kind, Interrupt::Nmi);
                assert_eq!(entry.after.pc, 0xa000);
                return;
            }
        }

        panic!("latched NMI was lost after PPUSTATUS lowered the line");
    }
}
