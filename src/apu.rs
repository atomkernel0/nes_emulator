//
// The Audio Processing Unit (APU) is responsible for generating sound in the NES.
// It consists of five channels: two pulse wave channels, one triangle wave channel,
// one noise channel, and one Delta Modulation Channel (DMC) for playing PCM samples.
// This file models the APU and its components.
//

// --- Constants ---

/// Duty cycle sequences for the pulse channels.
/// Each inner array represents a duty cycle, where 1 is high and 0 is low.
/// The four sequences correspond to 12.5%, 25%, 50%, and a negated 25% duty cycle.
const DUTY_SEQUENCES: [[u8; 8]; 4] = [
    [0, 1, 0, 0, 0, 0, 0, 0], // 12.5%
    [0, 1, 1, 0, 0, 0, 0, 0], // 25%
    [0, 1, 1, 1, 1, 0, 0, 0], // 50%
    [1, 0, 0, 1, 1, 1, 1, 1], // 75% (25% negated)
];

/// Sequence of values for the triangle channel's waveform.
/// It steps through these values to generate a triangle wave.
const TRIANGLE_SEQUENCE: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
    13, 14, 15,
];

/// Timer periods for the noise channel, specific to the NTSC video standard.
const NOISE_TIMER_PERIODS_NTSC: [u16; 16] = [
    4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
];

/// Rate table for the DMC, specific to the NTSC video standard.
/// These values determine the playback frequency of samples.
const DMC_RATE_TABLE_NTSC: [u16; 16] = [
    428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
];

/// Lookup table for the length counter.
/// When a value is written to a channel's length counter register,
/// this table is used to determine the actual length.
const LENGTH_COUNTER_TABLE: [u8; 32] = [
    10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14, 12, 16, 24, 18, 48, 20, 96, 22,
    192, 24, 72, 26, 16, 28, 32, 30,
];

// --- Channel sub-structures ---

/// Manages the volume envelope for pulse and noise channels.
/// It can either produce a constant volume or a decaying volume.
#[derive(Default, Copy, Clone)]
struct Envelope {
    start_flag: bool,      // Set when the envelope should restart.
    constant_volume: bool, // True for constant volume, false for decay.
    loop_flag: bool,       // If true, the envelope decay will loop.
    volume: u8,            // The constant volume or envelope period.
    divider_period: u8,
    divider_value: u8,
    decay_level: u8, // The current decay level, from 15 down to 0.
}

impl Envelope {
    /// Clocks the envelope generator.
    fn clock(&mut self) {
        if self.start_flag {
            self.start_flag = false;
            self.decay_level = 15;
            self.divider_value = self.divider_period;
        } else if self.divider_value > 0 {
            self.divider_value -= 1;
        } else {
            self.divider_value = self.divider_period;
            if self.decay_level > 0 {
                self.decay_level -= 1;
            } else if self.loop_flag {
                self.decay_level = 15;
            }
        }
    }

    /// Returns the current output volume of the envelope.
    fn output(&self) -> u8 {
        if self.constant_volume {
            self.volume
        } else {
            self.decay_level
        }
    }
}

/// Manages the frequency sweep for the pulse channels.
/// This unit can periodically adjust the channel's timer period, creating a sweeping pitch effect.
#[derive(Default, Copy, Clone)]
struct SweepUnit {
    enabled: bool,
    negate: bool,      // If true, the sweep decreases the period (increases pitch).
    period: u8,        // The sweep update rate.
    shift: u8,         // The amount to shift the timer period by.
    reload_flag: bool, // Set to reload the divider.
    divider_value: u8,
}

impl SweepUnit {
    /// Clocks the sweep unit.
    fn clock(&mut self, timer_period: &mut u16, is_pulse2: bool) {
        let target_period = self.calculate_target_period(*timer_period, is_pulse2);
        let mute = *timer_period < 8 || target_period > 0x7FF;

        if self.divider_value == 0 && self.enabled && self.shift > 0 && !mute {
            *timer_period = target_period;
        }

        if self.divider_value == 0 || self.reload_flag {
            self.divider_value = self.period;
            self.reload_flag = false;
        } else {
            self.divider_value -= 1;
        }
    }

    /// Calculates the target period for the sweep.
    fn calculate_target_period(&self, timer_period: u16, is_pulse2: bool) -> u16 {
        let change = timer_period >> self.shift;
        if self.negate {
            timer_period
                .wrapping_sub(change)
                .wrapping_sub(if is_pulse2 { 0 } else { 1 }) // Pulse 1 has an extra subtraction.
        } else {
            timer_period.wrapping_add(change)
        }
    }

    /// Determines if the sweep unit is currently muting the channel.
    fn is_muting(&self, timer_period: u16, is_pulse2: bool) -> bool {
        timer_period < 8 || self.calculate_target_period(timer_period, is_pulse2) > 0x7FF
    }
}

// --- Channels ---

/// Represents one of the two pulse wave channels.
#[derive(Default, Copy, Clone)]
pub struct PulseChannel {
    enabled: bool,
    is_pulse2: bool, // To distinguish between pulse 1 and 2 for sweep behavior.
    duty_cycle: u8,
    envelope: Envelope,
    sweep: SweepUnit,
    timer_period: u16,
    timer_value: u16,
    length_counter: u8,
    duty_sequencer: u8,
}

impl PulseChannel {
    pub fn new(is_pulse2: bool) -> Self {
        Self {
            is_pulse2,
            ..Default::default()
        }
    }

    /// Clocks the channel's timer.
    fn clock_timer(&mut self) {
        if self.timer_value == 0 {
            self.timer_value = self.timer_period;
            self.duty_sequencer = (self.duty_sequencer + 1) % 8;
        } else {
            self.timer_value -= 1;
        }
    }

    /// Clocks the channel's envelope.
    fn clock_envelope(&mut self) {
        self.envelope.clock();
    }

    /// Clocks the channel's sweep unit.
    fn clock_sweep(&mut self) {
        self.sweep.clock(&mut self.timer_period, self.is_pulse2);
    }

    /// Clocks the channel's length counter.
    fn clock_length_counter(&mut self) {
        if !self.envelope.loop_flag && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    /// Returns the current output sample for this channel.
    pub fn output(&self) -> u8 {
        if !self.enabled
            || self.length_counter == 0
            || self.sweep.is_muting(self.timer_period, self.is_pulse2)
            || DUTY_SEQUENCES[self.duty_cycle as usize][self.duty_sequencer as usize] == 0
        {
            0
        } else {
            self.envelope.output()
        }
    }
}

/// Represents the triangle wave channel.
#[derive(Default, Copy, Clone)]
pub struct TriangleChannel {
    enabled: bool,
    length_counter_halt: bool, // Also the control flag.
    control_flag: bool,
    linear_counter_load: u8,
    linear_counter_value: u8,
    timer_period: u16,
    timer_value: u16,
    length_counter: u8,
    sequencer_step: u8,
}

impl TriangleChannel {
    /// Clocks the channel's timer.
    fn clock_timer(&mut self) {
        if self.timer_value > 0 {
            self.timer_value -= 1;
        } else {
            self.timer_value = self.timer_period;
            if self.length_counter > 0 && self.linear_counter_value > 0 {
                self.sequencer_step = (self.sequencer_step + 1) % 32;
            }
        }
    }

    /// Clocks the linear counter, which controls volume.
    fn clock_linear_counter(&mut self) {
        if self.control_flag {
            self.linear_counter_value = self.linear_counter_load;
        } else if self.linear_counter_value > 0 {
            self.linear_counter_value -= 1;
        }
        if !self.length_counter_halt {
            self.control_flag = false;
        }
    }

    /// Clocks the channel's length counter.
    fn clock_length_counter(&mut self) {
        if !self.length_counter_halt && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    /// Returns the current output sample for this channel.
    pub fn output(&self) -> u8 {
        if !self.enabled
            || self.length_counter == 0
            || self.linear_counter_value == 0
            || self.timer_period < 2
        // Avoids supersonic frequencies.
        {
            0
        } else {
            TRIANGLE_SEQUENCE[self.sequencer_step as usize]
        }
    }
}

/// Represents the noise channel.
#[derive(Copy, Clone)]
pub struct NoiseChannel {
    enabled: bool,
    mode: bool, // False for pseudo-random, true for periodic.
    shift_register: u16,
    envelope: Envelope,
    timer_period: u16,
    timer_value: u16,
    length_counter: u8,
}

impl Default for NoiseChannel {
    fn default() -> Self {
        NoiseChannel {
            enabled: false,
            mode: false,
            shift_register: 1, // Must be initialized to 1.
            envelope: Envelope::default(),
            timer_period: 0,
            timer_value: 0,
            length_counter: 0,
        }
    }
}

impl NoiseChannel {
    /// Clocks the channel's timer.
    fn clock_timer(&mut self) {
        if self.timer_period == 0 {
            return;
        }
        if self.timer_value == 0 {
            self.timer_value = self.timer_period;
            self.clock_shift_register();
        } else {
            self.timer_value -= 1;
        }
    }

    /// Clocks the linear-feedback shift register to generate noise.
    fn clock_shift_register(&mut self) {
        let feedback_bit = if self.mode {
            (self.shift_register >> 6) & 1
        } else {
            (self.shift_register >> 1) & 1
        };
        let feedback = (self.shift_register & 1) ^ feedback_bit;
        self.shift_register >>= 1;
        self.shift_register |= feedback << 14;
    }

    /// Clocks the channel's envelope.
    fn clock_envelope(&mut self) {
        self.envelope.clock();
    }

    /// Clocks the channel's length counter.
    fn clock_length_counter(&mut self) {
        if !self.envelope.loop_flag && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }

    /// Returns the current output sample for this channel.
    pub fn output(&self) -> u8 {
        if !self.enabled || self.length_counter == 0 || (self.shift_register & 1) == 1 {
            0
        } else {
            self.envelope.output()
        }
    }
}

/// Represents the Delta Modulation Channel (DMC).
/// Plays digital samples from memory.
#[derive(Default, Copy, Clone)]
pub struct DmcChannel {
    enabled: bool,
    irq_enabled: bool,
    loop_flag: bool,
    irq_pending: bool,

    timer_period: u16,
    timer_value: u16,

    sample_address: u16, // Start address of the sample.
    sample_length: u16,  // Length of the sample.
    current_address: u16,
    current_length: u16,

    shift_register: u8,
    bits_remaining: u8,
    output_level: u8,
    sample_buffer: Option<u8>, // Holds the next byte of sample data.
}

impl DmcChannel {
    /// Clocks the channel's timer.
    fn clock_timer(&mut self) {
        if self.timer_period == 0 {
            return;
        }
        if self.timer_value > 0 {
            self.timer_value -= 1;
        } else {
            self.timer_value = self.timer_period;
            self.clock_output_unit();
        }
    }

    /// Clocks the output unit, which processes sample bits.
    fn clock_output_unit(&mut self) {
        // Refill shift register if empty
        if self.bits_remaining == 0 {
            self.bits_remaining = 8;
            if let Some(byte) = self.sample_buffer {
                self.shift_register = byte;
                self.sample_buffer = None;
            }
        }

        if self.bits_remaining > 0 {
            // Only adjust output if there's an active sample
            if self.sample_buffer.is_some() || self.current_length > 0 {
                if (self.shift_register & 0x01) == 1 {
                    if self.output_level <= 125 {
                        self.output_level += 2;
                    }
                } else {
                    if self.output_level >= 2 {
                        self.output_level -= 2;
                    }
                }
            }
            self.shift_register >>= 1;
            self.bits_remaining -= 1;
        }
    }

    /// Returns the current output sample for this channel.
    pub fn output(&self) -> u8 {
        self.output_level
    }
}

// --- FrameCounter ---

/// The mode of the frame counter, which controls the timing of APU events.
#[derive(Default, Copy, Clone)]
pub enum FrameCounterMode {
    #[default]
    FourStep, // Divides events into 4 steps.
    FiveStep, // Divides events into 5 steps.
}

/// The frame counter generates clocks for various APU components at specific intervals.
#[derive(Default, Copy, Clone)]
pub struct FrameCounter {
    mode: FrameCounterMode,
    interrupt_inhibit: bool, // Disables frame counter interrupts when set.
    interrupt_flag: bool,    // Set when a frame interrupt occurs.
}

// --- APU ---

/// The main APU structure. It contains all five sound channels and manages their state.
#[derive(Copy, Clone)]
pub struct Apu {
    pulse1: PulseChannel,
    pulse2: PulseChannel,
    triangle: TriangleChannel,
    noise: NoiseChannel,
    dmc: DmcChannel,
    frame_counter: FrameCounter,

    frame_cycle: u32,
    cycles: u64, // Total APU cycles.
    dmc_read_request: Option<u16>,

    // For audio sampling
    time_counter: u32,
    cycles_per_sample: u32,
}

impl Default for Apu {
    fn default() -> Self {
        Apu {
            pulse1: PulseChannel::new(false),
            pulse2: PulseChannel::new(true),
            triangle: TriangleChannel::default(),
            noise: NoiseChannel::default(),
            dmc: DmcChannel::default(),
            frame_counter: FrameCounter::default(),
            frame_cycle: 0,
            cycles: 0,
            dmc_read_request: None,
            time_counter: 0,
            cycles_per_sample: 0,
        }
    }
}

impl Apu {
    /// Creates a new APU instance.
    pub fn new(sample_rate: f64) -> Self {
        let cpu_clock_rate = 1_789_773.0; // NTSC CPU clock rate
        let cycles_per_sample = (cpu_clock_rate / sample_rate) as u32;

        Apu {
            cycles_per_sample,
            ..Default::default()
        }
    }

    /// Clocks the envelopes and the triangle channel's linear counter.
    /// This is a "quarter frame" event.
    fn clock_envelopes_and_linear_counter(&mut self) {
        self.pulse1.clock_envelope();
        self.pulse2.clock_envelope();
        self.triangle.clock_linear_counter();
        self.noise.clock_envelope();
    }

    /// Clocks the length counters and sweep units.
    /// This is a "half frame" event.
    fn clock_length_counters_and_sweep_units(&mut self) {
        self.pulse1.clock_length_counter();
        self.pulse1.clock_sweep();
        self.pulse2.clock_length_counter();
        self.pulse2.clock_sweep();
        self.triangle.clock_length_counter();
        self.noise.clock_length_counter();
    }

    /// Main clock cycle for the APU. This is called for every CPU cycle.
    pub fn clock(&mut self) {
        self.time_counter += 1;
        self.triangle.clock_timer(); // Triangle timer is clocked at CPU speed.

        self.cycles += 1;
        // Other channels are clocked at half the CPU speed.
        if self.cycles % 2 != 0 {
            self.dmc.clock_timer();
            self.check_dmc_read_request();
            return;
        }

        self.pulse1.clock_timer();
        self.pulse2.clock_timer();
        self.noise.clock_timer();
        self.dmc.clock_timer();

        self.check_dmc_read_request();

        // Frame counter logic
        self.frame_cycle += 1;
        match self.frame_counter.mode {
            FrameCounterMode::FourStep => match self.frame_cycle {
                3729 => self.clock_envelopes_and_linear_counter(),
                7457 => {
                    self.clock_envelopes_and_linear_counter();
                    self.clock_length_counters_and_sweep_units();
                }
                11186 => self.clock_envelopes_and_linear_counter(),
                14915 => {
                    self.clock_envelopes_and_linear_counter();
                    self.clock_length_counters_and_sweep_units();
                    if !self.frame_counter.interrupt_inhibit {
                        self.frame_counter.interrupt_flag = true;
                    }
                    self.frame_cycle = 0;
                }
                _ => {}
            },
            FrameCounterMode::FiveStep => match self.frame_cycle {
                3729 => self.clock_envelopes_and_linear_counter(),
                7457 => {
                    self.clock_envelopes_and_linear_counter();
                    self.clock_length_counters_and_sweep_units();
                }
                11186 => self.clock_envelopes_and_linear_counter(),
                18641 => {
                    // The fifth step, no interrupt.
                    self.clock_envelopes_and_linear_counter();
                    self.clock_length_counters_and_sweep_units();
                    self.frame_cycle = 0;
                }
                _ => {}
            },
        };
    }

    /// Checks if the DMC needs to read a new sample byte from memory.
    fn check_dmc_read_request(&mut self) {
        if self.dmc.sample_buffer.is_none() && self.dmc.current_length > 0 {
            self.dmc_read_request = Some(self.dmc.current_address);
        }
    }

    /// Allows the CPU to see if the DMC is requesting data.
    pub fn dmc_peek_read_request(&self) -> Option<u16> {
        self.dmc_read_request
    }

    /// Provides the DMC with sample data from the bus.
    pub fn dmc_provide_data(&mut self, data: u8) {
        self.dmc.sample_buffer = Some(data);
        self.dmc_read_request = None;
        self.dmc.current_address = self.dmc.current_address.wrapping_add(1);
        if self.dmc.current_address == 0 {
            self.dmc.current_address = 0x8000;
        }

        self.dmc.current_length -= 1;
        if self.dmc.current_length == 0 {
            if self.dmc.loop_flag {
                self.dmc.current_address = self.dmc.sample_address;
                self.dmc.current_length = self.dmc.sample_length;
            } else if self.dmc.irq_enabled {
                self.dmc.irq_pending = true;
            }
        }
    }

    /// Handles CPU writes to APU registers.
    pub fn cpu_write(&mut self, addr: u16, data: u8) {
        match addr {
            0x4000..=0x4003 => Self::write_pulse_register(&mut self.pulse1, addr, data),
            0x4004..=0x4007 => Self::write_pulse_register(&mut self.pulse2, addr, data),
            0x4008..=0x400B => Self::write_triangle_register(&mut self.triangle, addr, data),
            0x400C..=0x400F => Self::write_noise_register(&mut self.noise, addr, data),
            0x4010..=0x4013 => Self::write_dmc_register(&mut self.dmc, addr, data),
            0x4015 => {
                // Status register write
                self.pulse1.enabled = (data & 0x01) != 0;
                self.pulse2.enabled = (data & 0x02) != 0;
                self.triangle.enabled = (data & 0x04) != 0;
                self.noise.enabled = (data & 0x08) != 0;
                self.dmc.enabled = (data & 0x10) != 0;
                // Disabling a channel clears its length counter.
                if !self.pulse1.enabled {
                    self.pulse1.length_counter = 0;
                }
                if !self.pulse2.enabled {
                    self.pulse2.length_counter = 0;
                }
                if !self.triangle.enabled {
                    self.triangle.length_counter = 0;
                }
                if !self.noise.enabled {
                    self.noise.length_counter = 0;
                }
                if !self.dmc.enabled {
                    self.dmc.current_length = 0;
                } else {
                    // Re-enabling DMC restarts the sample if it was empty.
                    if self.dmc.current_length == 0 {
                        self.dmc.current_address = self.dmc.sample_address;
                        self.dmc.current_length = self.dmc.sample_length;
                    }
                }
                self.dmc.irq_pending = false;
            }
            0x4017 => {
                // Frame counter control
                self.frame_counter.mode = if data & 0x80 == 0 {
                    FrameCounterMode::FourStep
                } else {
                    FrameCounterMode::FiveStep
                };
                self.frame_counter.interrupt_inhibit = (data & 0x40) != 0;
                self.frame_cycle = 0;
                // 5-step mode gets an immediate clocking of half- and quarter-frame units.
                if matches!(self.frame_counter.mode, FrameCounterMode::FiveStep) {
                    self.clock_envelopes_and_linear_counter();
                    self.clock_length_counters_and_sweep_units();
                }
            }
            _ => {}
        }
    }

    fn write_pulse_register(p: &mut PulseChannel, addr: u16, data: u8) {
        match addr & 0x03 {
            0 => {
                // Duty, loop, constant volume, volume/period
                p.duty_cycle = data >> 6;
                p.envelope.loop_flag = (data >> 5) & 1 == 1;
                p.envelope.constant_volume = (data >> 4) & 1 == 1;
                p.envelope.divider_period = data & 0x0F;
                p.envelope.volume = p.envelope.divider_period;
            }
            1 => {
                // Sweep unit control
                p.sweep.enabled = (data >> 7) & 1 == 1;
                p.sweep.period = (data >> 4) & 0x07;
                p.sweep.negate = (data >> 3) & 1 == 1;
                p.sweep.shift = data & 0x07;
                p.sweep.reload_flag = true;
            }
            2 => {
                // Timer low bits
                p.timer_period = (p.timer_period & 0xFF00) | data as u16;
            }
            3 => {
                // Timer high bits, length counter load
                p.timer_period = (p.timer_period & 0x00FF) | (((data & 0x07) as u16) << 8);
                if p.enabled {
                    p.length_counter = LENGTH_COUNTER_TABLE[(data >> 3) as usize];
                }
                p.envelope.start_flag = true;
                p.duty_sequencer = 0;
            }
            _ => unreachable!(),
        }
    }

    fn write_triangle_register(t: &mut TriangleChannel, addr: u16, data: u8) {
        match addr & 0x03 {
            0 => {
                // Control, linear counter load
                t.length_counter_halt = (data >> 7) & 1 == 1;
                t.control_flag = t.length_counter_halt;
                t.linear_counter_load = data & 0x7F;
            }
            1 => {} // Unused
            2 => {
                // Timer low
                t.timer_period = (t.timer_period & 0xFF00) | data as u16;
            }
            3 => {
                // Timer high, length counter
                if t.enabled {
                    t.length_counter = LENGTH_COUNTER_TABLE[(data >> 3) as usize];
                }
                t.timer_period = (t.timer_period & 0x00FF) | (((data & 0x07) as u16) << 8);
                t.control_flag = true;
            }
            _ => unreachable!(),
        }
    }

    fn write_noise_register(n: &mut NoiseChannel, addr: u16, data: u8) {
        match addr {
            0x400C => {
                // Envelope
                n.envelope.loop_flag = (data >> 5) & 1 == 1;
                n.envelope.constant_volume = (data >> 4) & 1 == 1;
                n.envelope.divider_period = data & 0x0F;
                n.envelope.volume = n.envelope.divider_period;
            }
            0x400E => {
                // Mode and period
                n.mode = (data >> 7) & 1 == 1;
                n.timer_period = NOISE_TIMER_PERIODS_NTSC[(data & 0x0F) as usize];
            }
            0x400F => {
                // Length counter
                if n.enabled {
                    n.length_counter = LENGTH_COUNTER_TABLE[(data >> 3) as usize];
                }
                n.envelope.start_flag = true;
            }
            _ => {}
        }
    }

    fn write_dmc_register(dmc: &mut DmcChannel, addr: u16, data: u8) {
        match addr {
            0x4010 => {
                // IRQ, loop, frequency
                dmc.irq_enabled = (data >> 7) & 1 == 1;
                if !dmc.irq_enabled {
                    dmc.irq_pending = false;
                }
                dmc.loop_flag = (data >> 6) & 1 == 1;
                dmc.timer_period = DMC_RATE_TABLE_NTSC[(data & 0x0F) as usize] / 2;
            }
            0x4011 => {
                // Output level
                dmc.output_level = data & 0x7F;
            }
            0x4012 => {
                // Sample address
                dmc.sample_address = 0xC000 + (data as u16 * 64);
            }
            0x4013 => {
                // Sample length
                dmc.sample_length = (data as u16 * 16) + 1;
            }
            _ => unreachable!(),
        }
    }

    /// Handles CPU reads from APU registers.
    pub fn cpu_read(&mut self, addr: u16) -> u8 {
        match addr {
            0x4015 => {
                // Status register
                let mut status = 0u8;
                if self.pulse1.length_counter > 0 {
                    status |= 0x01;
                }
                if self.pulse2.length_counter > 0 {
                    status |= 0x02;
                }
                if self.triangle.length_counter > 0 {
                    status |= 0x04;
                }
                if self.noise.length_counter > 0 {
                    status |= 0x08;
                }
                if self.dmc.current_length > 0 {
                    status |= 0x10;
                }
                if self.dmc.irq_pending {
                    status |= 0x80;
                }

                self.frame_counter.interrupt_flag = false;
                status
            }
            _ => 0,
        }
    }

    /// Mixes the output of all channels into a single audio sample.
    fn get_output_sample(&self) -> f32 {
        // Mixing formulas are approximations.
        let pulse_out = 0.00752 * (self.pulse1.output() + self.pulse2.output()) as f32;
        let tnd_out = 0.00851 * self.triangle.output() as f32
            + 0.00494 * self.noise.output() as f32
            + 0.00335 * self.dmc.output() as f32;

        pulse_out + tnd_out
    }

    /// Called by the audio system to get a new sample when ready.
    pub fn collect_audio_sample(&mut self) -> Option<f32> {
        if self.time_counter >= self.cycles_per_sample {
            self.time_counter -= self.cycles_per_sample;
            Some(self.get_output_sample())
        } else {
            None
        }
    }
}
