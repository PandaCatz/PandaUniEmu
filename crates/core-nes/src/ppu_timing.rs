use std::error::Error;
use std::fmt::{Display, Formatter};

pub const NTSC_MASTER_CLOCK_NUMERATOR_HZ: u64 = 236_250_000;
pub const NTSC_MASTER_CLOCK_DENOMINATOR: u64 = 11;
pub const MASTER_TICKS_PER_CPU_CYCLE: u64 = 12;
pub const MASTER_TICKS_PER_PPU_DOT: u64 = 4;
pub const DOTS_PER_SCANLINE: u16 = 341;
pub const SCANLINES_PER_FRAME: u16 = 262;

/// Visible NTSC picture scanlines. Scanline 240 is post-render, 241-260 are
/// vertical blank, and 261 is the pre-render scanline.
pub const VISIBLE_SCANLINES: u16 = 240;
const VBLANK_START_SCANLINE: u16 = 241;
const PRE_RENDER_SCANLINE: u16 = 261;
const STATUS_TRANSITION_DOT: u16 = 1;
const ODD_FRAME_LAST_DOT: u16 = 339;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PpuPosition {
    /// Completed frames since the deterministic timing epoch.
    pub frame: u64,
    /// Current scanline, in the NESdev 0-261 convention.
    pub scanline: u16,
    /// Current dot, in the NESdev 0-340 convention.
    pub dot: u16,
    /// Whether the current frame is the rendering-shortened odd frame.
    pub odd_frame: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PpuEvent {
    VblankStarted,
    VblankEnded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimedPpuEvent {
    pub master_tick: u64,
    pub event: PpuEvent,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PpuTiming {
    position: PpuPosition,
    rendering_enabled: bool,
    vblank: bool,
}

impl PpuTiming {
    #[must_use]
    pub const fn position(self) -> PpuPosition {
        self.position
    }

    #[must_use]
    pub const fn rendering_enabled(self) -> bool {
        self.rendering_enabled
    }

    pub const fn set_rendering_enabled(&mut self, enabled: bool) {
        self.rendering_enabled = enabled;
    }

    #[must_use]
    pub const fn vblank(self) -> bool {
        self.vblank
    }

    fn clock_dot(&mut self) -> Result<Option<PpuEvent>, TimingError> {
        let event = match (self.position.scanline, self.position.dot) {
            (VBLANK_START_SCANLINE, STATUS_TRANSITION_DOT) => {
                self.vblank = true;
                Some(PpuEvent::VblankStarted)
            }
            (PRE_RENDER_SCANLINE, STATUS_TRANSITION_DOT) => {
                self.vblank = false;
                Some(PpuEvent::VblankEnded)
            }
            _ => None,
        };

        let skips_last_dot = self.rendering_enabled
            && self.position.odd_frame
            && self.position.scanline == PRE_RENDER_SCANLINE
            && self.position.dot == ODD_FRAME_LAST_DOT;

        if skips_last_dot
            || (self.position.scanline == PRE_RENDER_SCANLINE
                && self.position.dot == DOTS_PER_SCANLINE - 1)
        {
            self.position.frame = self
                .position
                .frame
                .checked_add(1)
                .ok_or(TimingError::FrameOverflow)?;
            self.position.scanline = 0;
            self.position.dot = 0;
            self.position.odd_frame = !self.position.odd_frame;
        } else if self.position.dot == DOTS_PER_SCANLINE - 1 {
            self.position.scanline += 1;
            self.position.dot = 0;
        } else {
            self.position.dot += 1;
        }

        debug_assert!(self.position.scanline < SCANLINES_PER_FRAME);
        debug_assert!(self.position.dot < DOTS_PER_SCANLINE);
        Ok(event)
    }
}

/// Deterministic NTSC timing in the shared console master-clock domain.
///
/// The epoch fixes CPU/PPU phase at zero so tests are reproducible. Real reset
/// phase variation and mid-instruction CPU/device interleaving are later
/// machine-level milestones.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NtscScheduler {
    master_ticks: u64,
    ppu: PpuTiming,
}

impl NtscScheduler {
    #[must_use]
    pub const fn master_ticks(&self) -> u64 {
        self.master_ticks
    }

    #[must_use]
    pub const fn ppu(&self) -> &PpuTiming {
        &self.ppu
    }

    pub const fn ppu_mut(&mut self) -> &mut PpuTiming {
        &mut self.ppu
    }

    pub fn advance_cpu_cycles(&mut self, cycles: u16) -> Result<Vec<TimedPpuEvent>, TimingError> {
        let mut next = self.clone();
        let mut events = Vec::new();
        for _ in 0..cycles {
            next.clock_cpu_cycle(&mut events)?;
        }
        *self = next;
        Ok(events)
    }

    fn clock_cpu_cycle(&mut self, events: &mut Vec<TimedPpuEvent>) -> Result<(), TimingError> {
        let end = self
            .master_ticks
            .checked_add(MASTER_TICKS_PER_CPU_CYCLE)
            .ok_or(TimingError::MasterTickOverflow)?;

        let mut next_ppu = self.ppu;
        for dot in 0..(MASTER_TICKS_PER_CPU_CYCLE / MASTER_TICKS_PER_PPU_DOT) {
            let master_tick = self.master_ticks + dot * MASTER_TICKS_PER_PPU_DOT;
            if let Some(event) = next_ppu.clock_dot()? {
                events.push(TimedPpuEvent { master_tick, event });
            }
        }

        self.ppu = next_ppu;
        self.master_ticks = end;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimingError {
    MasterTickOverflow,
    FrameOverflow,
}

impl Display for TimingError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MasterTickOverflow => formatter.write_str("NES master tick counter overflowed"),
            Self::FrameOverflow => formatter.write_str("NES PPU frame counter overflowed"),
        }
    }
}

impl Error for TimingError {}

#[cfg(test)]
mod tests {
    use super::*;

    const DOTS_PER_FULL_FRAME: u64 = 89_342;

    fn clock_until_next_frame(ppu: &mut PpuTiming) -> u64 {
        let starting_frame = ppu.position().frame;
        let mut dots = 0;
        while ppu.position().frame == starting_frame {
            ppu.clock_dot().expect("test frame counter has headroom");
            dots += 1;
        }
        dots
    }

    #[test]
    fn ntsc_cpu_cycle_advances_three_ppu_dots_without_drift() {
        let mut scheduler = NtscScheduler::default();
        assert_eq!(scheduler.advance_cpu_cycles(1), Ok(Vec::new()));
        assert_eq!(scheduler.master_ticks(), MASTER_TICKS_PER_CPU_CYCLE);
        assert_eq!(scheduler.ppu().position().scanline, 0);
        assert_eq!(scheduler.ppu().position().dot, 3);
    }

    #[test]
    fn vblank_edges_occur_at_the_documented_scanline_and_dot() {
        let mut ppu = PpuTiming::default();
        let target = u64::from(VBLANK_START_SCANLINE) * u64::from(DOTS_PER_SCANLINE)
            + u64::from(STATUS_TRANSITION_DOT);
        for dot in 0..=target {
            let event = ppu.clock_dot().expect("first frame has counter headroom");
            assert_eq!(event, (dot == target).then_some(PpuEvent::VblankStarted));
        }
        assert!(ppu.vblank());

        loop {
            let before = ppu.position();
            let event = ppu.clock_dot().expect("first frame has counter headroom");
            if before.scanline == PRE_RENDER_SCANLINE && before.dot == STATUS_TRANSITION_DOT {
                assert_eq!(event, Some(PpuEvent::VblankEnded));
                break;
            }
            assert_eq!(event, None);
        }
        assert!(!ppu.vblank());
    }

    #[test]
    fn odd_frame_skip_only_applies_while_rendering() {
        let mut disabled = PpuTiming::default();
        assert_eq!(clock_until_next_frame(&mut disabled), DOTS_PER_FULL_FRAME);
        assert_eq!(clock_until_next_frame(&mut disabled), DOTS_PER_FULL_FRAME);

        let mut enabled = PpuTiming::default();
        enabled.set_rendering_enabled(true);
        assert_eq!(clock_until_next_frame(&mut enabled), DOTS_PER_FULL_FRAME);
        assert_eq!(
            clock_until_next_frame(&mut enabled),
            DOTS_PER_FULL_FRAME - 1
        );
        assert_eq!(clock_until_next_frame(&mut enabled), DOTS_PER_FULL_FRAME);
    }

    #[test]
    fn scheduler_timestamps_vblank_on_the_exact_master_tick() {
        let mut scheduler = NtscScheduler::default();
        let event_dot = u64::from(VBLANK_START_SCANLINE) * u64::from(DOTS_PER_SCANLINE)
            + u64::from(STATUS_TRANSITION_DOT);
        assert_eq!(event_dot % 3, 0);
        let cycles = u16::try_from(event_dot / 3 + 1).expect("first-frame count fits");
        assert_eq!(
            scheduler.advance_cpu_cycles(cycles),
            Ok(vec![TimedPpuEvent {
                master_tick: event_dot * MASTER_TICKS_PER_PPU_DOT,
                event: PpuEvent::VblankStarted,
            }])
        );
    }

    #[test]
    fn advance_is_failure_atomic_at_counter_overflow() {
        let mut scheduler = NtscScheduler {
            master_ticks: u64::MAX - (MASTER_TICKS_PER_CPU_CYCLE - 1),
            ppu: PpuTiming::default(),
        };
        let before = scheduler.clone();
        assert_eq!(
            scheduler.advance_cpu_cycles(1),
            Err(TimingError::MasterTickOverflow)
        );
        assert_eq!(scheduler, before);
    }

    #[test]
    fn advance_is_failure_atomic_at_frame_overflow() {
        let mut scheduler = NtscScheduler {
            master_ticks: 0,
            ppu: PpuTiming {
                position: PpuPosition {
                    frame: u64::MAX,
                    scanline: PRE_RENDER_SCANLINE,
                    dot: ODD_FRAME_LAST_DOT,
                    odd_frame: true,
                },
                rendering_enabled: true,
                vblank: false,
            },
        };
        let before = scheduler.clone();
        assert_eq!(
            scheduler.advance_cpu_cycles(1),
            Err(TimingError::FrameOverflow)
        );
        assert_eq!(scheduler, before);
    }
}
