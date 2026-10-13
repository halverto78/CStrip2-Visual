//! Cstrip2 DSP Core — Faithful port of Airwindows CStrip2
//! Original algorithm by Chris Johnson (MIT License).
//!
//! Preserves 100% strictly the mathematical formulas, gain staging,
//! filter structures, Class AB compressor, and non-linearities.


use std::sync::atomic::{AtomicU64, Ordering};
#[inline(always)]
fn frexpf_exp(x: f32) -> i32 {
    if x == 0.0 || !x.is_finite() {
        return 0;
    }
    let bits = x.to_bits();
    let mut exp_bits = ((bits >> 23) & 0xff) as i32;
    if exp_bits == 0 {
        // subnormal
        let scaled = x * 16777216.0; // 2^24
        let s_bits = scaled.to_bits();
        exp_bits = (((s_bits >> 23) & 0xff) as i32) - 24;
    }
    exp_bits - 126
}

/// Single-channel Capacitor filter implementing the 6-stage interleaved IIR array
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CapacitorChannel {
    pub iir_highpass_a: f64,
    pub iir_highpass_b: f64,
    pub iir_highpass_c: f64,
    pub iir_highpass_d: f64,
    pub iir_highpass_e: f64,
    pub iir_highpass_f: f64,
    pub iir_lowpass_a: f64,
    pub iir_lowpass_b: f64,
    pub iir_lowpass_c: f64,
    pub iir_lowpass_d: f64,
    pub iir_lowpass_e: f64,
    pub iir_lowpass_f: f64,
}

impl CapacitorChannel {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline(always)]
    pub fn process_step(
        &mut self,
        count: usize,
        mut input: f64,
        highpass_amount: f64,
        lowpass_amount: f64,
    ) -> f64 {
        match count {
            0 => {
                self.iir_highpass_a = (self.iir_highpass_a * (1.0 - highpass_amount)) + (input * highpass_amount);
                input -= self.iir_highpass_a;
                self.iir_lowpass_a = (self.iir_lowpass_a * (1.0 - lowpass_amount)) + (input * lowpass_amount);
                input = self.iir_lowpass_a;
                self.iir_highpass_b = (self.iir_highpass_b * (1.0 - highpass_amount)) + (input * highpass_amount);
                input -= self.iir_highpass_b;
                self.iir_lowpass_b = (self.iir_lowpass_b * (1.0 - lowpass_amount)) + (input * lowpass_amount);
                input = self.iir_lowpass_b;
                self.iir_highpass_d = (self.iir_highpass_d * (1.0 - highpass_amount)) + (input * highpass_amount);
                input -= self.iir_highpass_d;
                self.iir_lowpass_d = (self.iir_lowpass_d * (1.0 - lowpass_amount)) + (input * lowpass_amount);
                input = self.iir_lowpass_d;
            }
            1 => {
                self.iir_highpass_a = (self.iir_highpass_a * (1.0 - highpass_amount)) + (input * highpass_amount);
                input -= self.iir_highpass_a;
                self.iir_lowpass_a = (self.iir_lowpass_a * (1.0 - lowpass_amount)) + (input * lowpass_amount);
                input = self.iir_lowpass_a;
                self.iir_highpass_c = (self.iir_highpass_c * (1.0 - highpass_amount)) + (input * highpass_amount);
                input -= self.iir_highpass_c;
                self.iir_lowpass_c = (self.iir_lowpass_c * (1.0 - lowpass_amount)) + (input * lowpass_amount);
                input = self.iir_lowpass_c;
                self.iir_highpass_e = (self.iir_highpass_e * (1.0 - highpass_amount)) + (input * highpass_amount);
                input -= self.iir_highpass_e;
                self.iir_lowpass_e = (self.iir_lowpass_e * (1.0 - lowpass_amount)) + (input * lowpass_amount);
                input = self.iir_lowpass_e;
            }
            2 => {
                self.iir_highpass_a = (self.iir_highpass_a * (1.0 - highpass_amount)) + (input * highpass_amount);
                input -= self.iir_highpass_a;
                self.iir_lowpass_a = (self.iir_lowpass_a * (1.0 - lowpass_amount)) + (input * lowpass_amount);
                input = self.iir_lowpass_a;
                self.iir_highpass_b = (self.iir_highpass_b * (1.0 - highpass_amount)) + (input * highpass_amount);
                input -= self.iir_highpass_b;
                self.iir_lowpass_b = (self.iir_lowpass_b * (1.0 - lowpass_amount)) + (input * lowpass_amount);
                input = self.iir_lowpass_b;
                self.iir_highpass_f = (self.iir_highpass_f * (1.0 - highpass_amount)) + (input * highpass_amount);
                input -= self.iir_highpass_f;
                self.iir_lowpass_f = (self.iir_lowpass_f * (1.0 - lowpass_amount)) + (input * lowpass_amount);
                input = self.iir_lowpass_f;
            }
            3 => {
                self.iir_highpass_a = (self.iir_highpass_a * (1.0 - highpass_amount)) + (input * highpass_amount);
                input -= self.iir_highpass_a;
                self.iir_lowpass_a = (self.iir_lowpass_a * (1.0 - lowpass_amount)) + (input * lowpass_amount);
                input = self.iir_lowpass_a;
                self.iir_highpass_c = (self.iir_highpass_c * (1.0 - highpass_amount)) + (input * highpass_amount);
                input -= self.iir_highpass_c;
                self.iir_lowpass_c = (self.iir_lowpass_c * (1.0 - lowpass_amount)) + (input * lowpass_amount);
                input = self.iir_lowpass_c;
                self.iir_highpass_d = (self.iir_highpass_d * (1.0 - highpass_amount)) + (input * highpass_amount);
                input -= self.iir_highpass_d;
                self.iir_lowpass_d = (self.iir_lowpass_d * (1.0 - lowpass_amount)) + (input * lowpass_amount);
                input = self.iir_lowpass_d;
            }
            4 => {
                self.iir_highpass_a = (self.iir_highpass_a * (1.0 - highpass_amount)) + (input * highpass_amount);
                input -= self.iir_highpass_a;
                self.iir_lowpass_a = (self.iir_lowpass_a * (1.0 - lowpass_amount)) + (input * lowpass_amount);
                input = self.iir_lowpass_a;
                self.iir_highpass_b = (self.iir_highpass_b * (1.0 - highpass_amount)) + (input * highpass_amount);
                input -= self.iir_highpass_b;
                self.iir_lowpass_b = (self.iir_lowpass_b * (1.0 - lowpass_amount)) + (input * lowpass_amount);
                input = self.iir_lowpass_b;
                self.iir_highpass_e = (self.iir_highpass_e * (1.0 - highpass_amount)) + (input * highpass_amount);
                input -= self.iir_highpass_e;
                self.iir_lowpass_e = (self.iir_lowpass_e * (1.0 - lowpass_amount)) + (input * lowpass_amount);
                input = self.iir_lowpass_e;
            }
            _ => {
                self.iir_highpass_a = (self.iir_highpass_a * (1.0 - highpass_amount)) + (input * highpass_amount);
                input -= self.iir_highpass_a;
                self.iir_lowpass_a = (self.iir_lowpass_a * (1.0 - lowpass_amount)) + (input * lowpass_amount);
                input = self.iir_lowpass_a;
                self.iir_highpass_c = (self.iir_highpass_c * (1.0 - highpass_amount)) + (input * highpass_amount);
                input -= self.iir_highpass_c;
                self.iir_lowpass_c = (self.iir_lowpass_c * (1.0 - lowpass_amount)) + (input * lowpass_amount);
                input = self.iir_lowpass_c;
                self.iir_highpass_f = (self.iir_highpass_f * (1.0 - highpass_amount)) + (input * highpass_amount);
                input -= self.iir_highpass_f;
                self.iir_lowpass_f = (self.iir_lowpass_f * (1.0 - lowpass_amount)) + (input * lowpass_amount);
                input = self.iir_lowpass_f;
            }
        }
        input
    }
}

/// Standalone mono Capacitor filter for measurement and tests
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CapacitorFilter {
    pub count: usize,
    pub channel: CapacitorChannel,
}

impl CapacitorFilter {
    pub fn new() -> Self {
        Self::default()
    }

    #[inline(always)]
    pub fn process_sample(&mut self, input: f64, highpass_amount: f64, lowpass_amount: f64) -> f64 {
        self.count += 1;
        if self.count > 5 {
            self.count = 0;
        }
        self.channel.process_step(self.count, input, highpass_amount, lowpass_amount)
    }
}

#[derive(Clone)]
pub struct Cstrip2Dsp {
    pub sample_rate: f64,
    pub last_min_multiplier: f64,

    // Capacitor states
    count: usize,
    cap_l: CapacitorChannel,
    cap_r: CapacitorChannel,

    // EQ states
    last_sample_l: f64,
    last2_sample_l: f64,
    last_sample_r: f64,
    last2_sample_r: f64,

    iir_high_sample_la: f64,
    iir_high_sample_lb: f64,
    iir_high_sample_lc: f64,
    iir_high_sample_ld: f64,
    iir_high_sample_le: f64,
    iir_low_sample_la: f64,
    iir_low_sample_lb: f64,
    iir_low_sample_lc: f64,
    iir_low_sample_ld: f64,
    iir_low_sample_le: f64,
    iir_high_sample_l: f64,
    iir_low_sample_l: f64,

    iir_high_sample_ra: f64,
    iir_high_sample_rb: f64,
    iir_high_sample_rc: f64,
    iir_high_sample_rd: f64,
    iir_high_sample_re: f64,
    iir_low_sample_ra: f64,
    iir_low_sample_rb: f64,
    iir_low_sample_rc: f64,
    iir_low_sample_rd: f64,
    iir_low_sample_re: f64,
    iir_high_sample_r: f64,
    iir_low_sample_r: f64,

    triplet_la: f64,
    triplet_lb: f64,
    triplet_lc: f64,
    triplet_factor_l: f64,
    triplet_ra: f64,
    triplet_rb: f64,
    triplet_rc: f64,
    triplet_factor_r: f64,
    flipthree: i32,

    // ButterComp states
    control_apos_l: f64,
    control_aneg_l: f64,
    control_bpos_l: f64,
    control_bneg_l: f64,
    targetpos_l: f64,
    targetneg_l: f64,
    avg_la: f64,
    nvg_la: f64,

    control_apos_r: f64,
    control_aneg_r: f64,
    control_bpos_r: f64,
    control_bneg_r: f64,
    targetpos_r: f64,
    targetneg_r: f64,
    avg_ra: f64,
    nvg_ra: f64,

    flip: bool,

    // Dither seeds
    fpd_l: u32,
    fpd_r: u32,
}

/// Generates distinct, non-zero initial pseudo-random seeds for Left and Right channels,
/// compliant with original Airwindows specification (both >= 16386, distinct, non-zero).
/// Uses SplitMix64 on a global instance sequence counter.
fn generate_initial_seeds() -> (u32, u32) {
    static INSTANCE_SEED_COUNTER: AtomicU64 = AtomicU64::new(0x9e3779b97f4a7c15);

    fn splitmix64(mut x: u64) -> u64 {
        x = x.wrapping_add(0x9e3779b97f4a7c15);
        x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
        x ^ (x >> 31)
    }

    let seq = INSTANCE_SEED_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut state_l = splitmix64(seq.wrapping_mul(2));
    let mut state_r = splitmix64(seq.wrapping_mul(2).wrapping_add(1));

    let mut fpd_l = (state_l & 0xFFFFFFFF) as u32;
    let mut fpd_r = (state_r & 0xFFFFFFFF) as u32;

    while fpd_l < 16386 {
        state_l = splitmix64(state_l);
        fpd_l = (state_l & 0xFFFFFFFF) as u32;
    }
    while fpd_r < 16386 || fpd_r == fpd_l {
        state_r = splitmix64(state_r);
        fpd_r = (state_r & 0xFFFFFFFF) as u32;
    }

    (fpd_l, fpd_r)
}

impl Cstrip2Dsp {
    pub fn new(sample_rate: f64) -> Self {
        let (fpd_l, fpd_r) = generate_initial_seeds();
        Self::new_with_seeds(sample_rate, fpd_l, fpd_r)
    }

    /// Construct instance with specific seeds (useful for reproducible testing)
    pub fn new_with_seeds(sample_rate: f64, fpd_l: u32, fpd_r: u32) -> Self {
        Self {
            sample_rate,
            last_min_multiplier: 1.0,
            count: 0,
            cap_l: CapacitorChannel::new(),
            cap_r: CapacitorChannel::new(),

            last_sample_l: 0.0,
            last2_sample_l: 0.0,
            last_sample_r: 0.0,
            last2_sample_r: 0.0,

            iir_high_sample_la: 0.0,
            iir_high_sample_lb: 0.0,
            iir_high_sample_lc: 0.0,
            iir_high_sample_ld: 0.0,
            iir_high_sample_le: 0.0,
            iir_low_sample_la: 0.0,
            iir_low_sample_lb: 0.0,
            iir_low_sample_lc: 0.0,
            iir_low_sample_ld: 0.0,
            iir_low_sample_le: 0.0,
            iir_high_sample_l: 0.0,
            iir_low_sample_l: 0.0,

            iir_high_sample_ra: 0.0,
            iir_high_sample_rb: 0.0,
            iir_high_sample_rc: 0.0,
            iir_high_sample_rd: 0.0,
            iir_high_sample_re: 0.0,
            iir_low_sample_ra: 0.0,
            iir_low_sample_rb: 0.0,
            iir_low_sample_rc: 0.0,
            iir_low_sample_rd: 0.0,
            iir_low_sample_re: 0.0,
            iir_high_sample_r: 0.0,
            iir_low_sample_r: 0.0,

            triplet_la: 0.0,
            triplet_lb: 0.0,
            triplet_lc: 0.0,
            triplet_factor_l: 0.0,
            triplet_ra: 0.0,
            triplet_rb: 0.0,
            triplet_rc: 0.0,
            triplet_factor_r: 0.0,
            flipthree: 0,

            control_apos_l: 1.0,
            control_aneg_l: 1.0,
            control_bpos_l: 1.0,
            control_bneg_l: 1.0,
            targetpos_l: 1.0,
            targetneg_l: 1.0,
            avg_la: 0.0,
            nvg_la: 0.0,

            control_apos_r: 1.0,
            control_aneg_r: 1.0,
            control_bpos_r: 1.0,
            control_bneg_r: 1.0,
            targetpos_r: 1.0,
            targetneg_r: 1.0,
            avg_ra: 0.0,
            nvg_ra: 0.0,

            flip: false,

            fpd_l,
            fpd_r,
        }
    }

    /// Reset DSP filter state while preserving continuing RNG seed state
    pub fn reset(&mut self, sample_rate: f64) {
        let saved_fpd_l = self.fpd_l;
        let saved_fpd_r = self.fpd_r;
        *self = Self::new_with_seeds(sample_rate, saved_fpd_l, saved_fpd_r);
    }

    /// Get current (fpd_l, fpd_r) seeds
    pub fn get_fpd_seeds(&self) -> (u32, u32) {
        (self.fpd_l, self.fpd_r)
    }

    /// Process a block of stereo audio samples in-place
    pub fn process_stereo(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        a: f64, // Treble: 0..1 (default 0.5)
        b: f64, // Mid: 0..1 (default 0.5)
        c: f64, // Bass: 0..1 (default 0.5)
        d: f64, // TrebFrq: 0..1 (default 0.5)
        e: f64, // BassFrq: 0..1 (default 0.5)
        f: f64, // LowCap: 0..1 (default 1.0)
        g: f64, // HiCap: 0..1 (default 0.0)
        h: f64, // Compres: 0..1 (default 0.0)
        i: f64, // CompSpd: 0..1 (default 0.0)
        j: f64, // Output: 0..1 (default 0.33)
    ) {
        let overallscale = self.sample_rate;
        let compscale = (1.0 / 44100.0) * overallscale;

        let fp_old = 0.618033988749894848204586; // golden ratio!
        let fp_new = 1.0 - fp_old;

        // EQ setup
        let mut density_a = (a * 12.0) - 6.0;
        let mut density_b = (b * 12.0) - 6.0;
        let mut density_c = (c * 12.0) - 6.0;
        let engage_eq = (density_a != 0.0) || (density_b != 0.0) || (density_c != 0.0);

        density_a = 10.0_f64.powf(density_a / 20.0) - 1.0;
        density_b = 10.0_f64.powf(density_b / 20.0) - 1.0;
        density_c = 10.0_f64.powf(density_c / 20.0) - 1.0;

        let triplet_intensity = -density_a;
        let iir_amount_a = (((d * d * 15.0) + 1.0) * 1000.0) / overallscale;
        let iir_amount_b = (((e * e * 1570.0) + 30.0) * 10.0) / overallscale;
        let out_a = density_a.abs();
        let out_b = density_b.abs();
        let out_c = density_c.abs();

        // Capacitor setup
        let lowpass_amount = f * f;
        let highpass_amount = g * g;
        let engage_capacitor = (lowpass_amount < 1.0) || (highpass_amount > 0.0);

        // ButterComp setup
        let inputgain = (h.powi(4) * 35.0) + 1.0;
        let mut compoutgain = inputgain;
        compoutgain -= 1.0;
        compoutgain /= 1.2;
        compoutgain += 1.0;

        let mut divisor = (0.008 * (i * i)) + 0.0004;
        divisor /= compscale;
        let remainder = divisor;
        divisor = 1.0 - divisor;
        let engage_comp = inputgain > 1.0;

        // Output setup
        let outputgain = j * 3.0; // 0..3
        let mut density = outputgain - 1.0;
        if density < 0.0 {
            density = 0.0;
        }
        let mut phattity = density - 1.0;
        if density > 1.0 {
            density = 1.0;
        }
        if phattity < 0.0 {
            phattity = 0.0;
        }

        let mut min_mult = 1.0f64;
        let len = left.len().min(right.len());
        for k in 0..len {
            let mut input_sample_l = left[k] as f64;
            let mut input_sample_r = right[k] as f64;

            if input_sample_l.abs() < 1.18e-23 {
                input_sample_l = (self.fpd_l as f64) * 1.18e-17;
            }
            if input_sample_r.abs() < 1.18e-23 {
                input_sample_r = (self.fpd_r as f64) * 1.18e-17;
            }

            // 1. Capacitor
            if engage_capacitor {
                self.count += 1;
                if self.count > 5 {
                    self.count = 0;
                }
                input_sample_l = self.cap_l.process_step(self.count, input_sample_l, highpass_amount, lowpass_amount);
                input_sample_r = self.cap_r.process_step(self.count, input_sample_r, highpass_amount, lowpass_amount);
            }

            // 2. ButterComp
            if engage_comp {
                self.flip = !self.flip;

                // Left channel
                input_sample_l *= inputgain;
                let mut inputpos = (input_sample_l * fp_old) + (self.avg_la * fp_new) + 1.0;
                self.avg_la = input_sample_l;
                if inputpos < 0.0 {
                    inputpos = 0.0;
                }
                let mut outputpos = inputpos / 2.0;
                if outputpos > 1.0 {
                    outputpos = 1.0;
                }
                inputpos *= inputpos;
                self.targetpos_l *= divisor;
                self.targetpos_l += inputpos * remainder;
                let calcpos = (1.0 / self.targetpos_l).powi(2);

                let mut inputneg = (-input_sample_l * fp_old) + (self.nvg_la * fp_new) + 1.0;
                self.nvg_la = -input_sample_l;
                if inputneg < 0.0 {
                    inputneg = 0.0;
                }
                let mut outputneg = inputneg / 2.0;
                if outputneg > 1.0 {
                    outputneg = 1.0;
                }
                inputneg *= inputneg;
                self.targetneg_l *= divisor;
                self.targetneg_l += inputneg * remainder;
                let calcneg = (1.0 / self.targetneg_l).powi(2);

                if input_sample_l > 0.0 {
                    if self.flip {
                        self.control_apos_l *= divisor;
                        self.control_apos_l += calcpos * remainder;
                    } else {
                        self.control_bpos_l *= divisor;
                        self.control_bpos_l += calcpos * remainder;
                    }
                } else {
                    if self.flip {
                        self.control_aneg_l *= divisor;
                        self.control_aneg_l += calcneg * remainder;
                    } else {
                        self.control_bneg_l *= divisor;
                        self.control_bneg_l += calcneg * remainder;
                    }
                }

                let totalmultiplier_l = if self.flip {
                    (self.control_apos_l * outputpos) + (self.control_aneg_l * outputneg)
                } else {
                    (self.control_bpos_l * outputpos) + (self.control_bneg_l * outputneg)
                };

                input_sample_l *= totalmultiplier_l;
                input_sample_l /= compoutgain;

                // Right channel
                input_sample_r *= inputgain;
                let mut inputpos_r = (input_sample_r * fp_old) + (self.avg_ra * fp_new) + 1.0;
                self.avg_ra = input_sample_r;
                if inputpos_r < 0.0 {
                    inputpos_r = 0.0;
                }
                let mut outputpos_r = inputpos_r / 2.0;
                if outputpos_r > 1.0 {
                    outputpos_r = 1.0;
                }
                inputpos_r *= inputpos_r;
                self.targetpos_r *= divisor;
                self.targetpos_r += inputpos_r * remainder;
                let calcpos_r = (1.0 / self.targetpos_r).powi(2);

                let mut inputneg_r = (-input_sample_r * fp_old) + (self.nvg_ra * fp_new) + 1.0;
                self.nvg_ra = -input_sample_r;
                if inputneg_r < 0.0 {
                    inputneg_r = 0.0;
                }
                let mut outputneg_r = inputneg_r / 2.0;
                if outputneg_r > 1.0 {
                    outputneg_r = 1.0;
                }
                inputneg_r *= inputneg_r;
                self.targetneg_r *= divisor;
                self.targetneg_r += inputneg_r * remainder;
                let calcneg_r = (1.0 / self.targetneg_r).powi(2);

                if input_sample_r > 0.0 {
                    if self.flip {
                        self.control_apos_r *= divisor;
                        self.control_apos_r += calcpos_r * remainder;
                    } else {
                        self.control_bpos_r *= divisor;
                        self.control_bpos_r += calcpos_r * remainder;
                    }
                } else {
                    if self.flip {
                        self.control_aneg_r *= divisor;
                        self.control_aneg_r += calcneg_r * remainder;
                    } else {
                        self.control_bneg_r *= divisor;
                        self.control_bneg_r += calcneg_r * remainder;
                    }
                }

                let totalmultiplier_r = if self.flip {
                    (self.control_apos_r * outputpos_r) + (self.control_aneg_r * outputneg_r)
                } else {
                    (self.control_bpos_r * outputpos_r) + (self.control_bneg_r * outputneg_r)
                };

                input_sample_r *= totalmultiplier_r;
                input_sample_r /= compoutgain;

                let mult_sample = totalmultiplier_l.min(totalmultiplier_r);
                if mult_sample < min_mult {
                    min_mult = mult_sample;
                }
            }

            // 3. 3-Band EQ
            if engage_eq {
                self.last2_sample_l = self.last_sample_l;
                self.last_sample_l = input_sample_l;
                self.last2_sample_r = self.last_sample_r;
                self.last_sample_r = input_sample_r;

                self.flipthree += 1;
                if self.flipthree < 1 || self.flipthree > 3 {
                    self.flipthree = 1;
                }

                let mut high_sample_l;
                let mut mid_sample_l;
                let mut bass_sample_l;

                let mut high_sample_r;
                let mut mid_sample_r;
                let mut bass_sample_r;

                match self.flipthree {
                    1 => {
                        self.triplet_factor_l = self.last2_sample_l - input_sample_l;
                        self.triplet_la += self.triplet_factor_l;
                        self.triplet_lc -= self.triplet_factor_l;
                        self.triplet_factor_l = self.triplet_la * triplet_intensity;
                        self.iir_high_sample_lc = (self.iir_high_sample_lc * (1.0 - iir_amount_a)) + (input_sample_l * iir_amount_a);
                        high_sample_l = input_sample_l - self.iir_high_sample_lc;
                        self.iir_low_sample_lc = (self.iir_low_sample_lc * (1.0 - iir_amount_b)) + (input_sample_l * iir_amount_b);
                        bass_sample_l = self.iir_low_sample_lc;

                        self.triplet_factor_r = self.last2_sample_r - input_sample_r;
                        self.triplet_ra += self.triplet_factor_r;
                        self.triplet_rc -= self.triplet_factor_r;
                        self.triplet_factor_r = self.triplet_ra * triplet_intensity;
                        self.iir_high_sample_rc = (self.iir_high_sample_rc * (1.0 - iir_amount_a)) + (input_sample_r * iir_amount_a);
                        high_sample_r = input_sample_r - self.iir_high_sample_rc;
                        self.iir_low_sample_rc = (self.iir_low_sample_rc * (1.0 - iir_amount_b)) + (input_sample_r * iir_amount_b);
                        bass_sample_r = self.iir_low_sample_rc;
                    }
                    2 => {
                        self.triplet_factor_l = self.last2_sample_l - input_sample_l;
                        self.triplet_lb += self.triplet_factor_l;
                        self.triplet_la -= self.triplet_factor_l;
                        self.triplet_factor_l = self.triplet_lb * triplet_intensity;
                        self.iir_high_sample_ld = (self.iir_high_sample_ld * (1.0 - iir_amount_a)) + (input_sample_l * iir_amount_a);
                        high_sample_l = input_sample_l - self.iir_high_sample_ld;
                        self.iir_low_sample_ld = (self.iir_low_sample_ld * (1.0 - iir_amount_b)) + (input_sample_l * iir_amount_b);
                        bass_sample_l = self.iir_low_sample_ld;

                        self.triplet_factor_r = self.last2_sample_r - input_sample_r;
                        self.triplet_rb += self.triplet_factor_r;
                        self.triplet_ra -= self.triplet_factor_r;
                        self.triplet_factor_r = self.triplet_rb * triplet_intensity;
                        self.iir_high_sample_rd = (self.iir_high_sample_rd * (1.0 - iir_amount_a)) + (input_sample_r * iir_amount_a);
                        high_sample_r = input_sample_r - self.iir_high_sample_rd;
                        self.iir_low_sample_rd = (self.iir_low_sample_rd * (1.0 - iir_amount_b)) + (input_sample_r * iir_amount_b);
                        bass_sample_r = self.iir_low_sample_rd;
                    }
                    _ => {
                        self.triplet_factor_l = self.last2_sample_l - input_sample_l;
                        self.triplet_lc += self.triplet_factor_l;
                        self.triplet_lb -= self.triplet_factor_l;
                        self.triplet_factor_l = self.triplet_lc * triplet_intensity;
                        self.iir_high_sample_le = (self.iir_high_sample_le * (1.0 - iir_amount_a)) + (input_sample_l * iir_amount_a);
                        high_sample_l = input_sample_l - self.iir_high_sample_le;
                        self.iir_low_sample_le = (self.iir_low_sample_le * (1.0 - iir_amount_b)) + (input_sample_l * iir_amount_b);
                        bass_sample_l = self.iir_low_sample_le;

                        self.triplet_factor_r = self.last2_sample_r - input_sample_r;
                        self.triplet_rc += self.triplet_factor_r;
                        self.triplet_rb -= self.triplet_factor_r;
                        self.triplet_factor_r = self.triplet_rc * triplet_intensity;
                        self.iir_high_sample_re = (self.iir_high_sample_re * (1.0 - iir_amount_a)) + (input_sample_r * iir_amount_a);
                        high_sample_r = input_sample_r - self.iir_high_sample_re;
                        self.iir_low_sample_re = (self.iir_low_sample_re * (1.0 - iir_amount_b)) + (input_sample_r * iir_amount_b);
                        bass_sample_r = self.iir_low_sample_re;
                    }
                }

                self.triplet_la /= 2.0;
                self.triplet_lb /= 2.0;
                self.triplet_lc /= 2.0;
                high_sample_l += self.triplet_factor_l;

                self.triplet_ra /= 2.0;
                self.triplet_rb /= 2.0;
                self.triplet_rc /= 2.0;
                high_sample_r += self.triplet_factor_r;

                if self.flip {
                    self.iir_high_sample_la = (self.iir_high_sample_la * (1.0 - iir_amount_a)) + (high_sample_l * iir_amount_a);
                    high_sample_l -= self.iir_high_sample_la;
                    self.iir_low_sample_la = (self.iir_low_sample_la * (1.0 - iir_amount_b)) + (bass_sample_l * iir_amount_b);
                    bass_sample_l = self.iir_low_sample_la;

                    self.iir_high_sample_ra = (self.iir_high_sample_ra * (1.0 - iir_amount_a)) + (high_sample_r * iir_amount_a);
                    high_sample_r -= self.iir_high_sample_ra;
                    self.iir_low_sample_ra = (self.iir_low_sample_ra * (1.0 - iir_amount_b)) + (bass_sample_r * iir_amount_b);
                    bass_sample_r = self.iir_low_sample_ra;
                } else {
                    self.iir_high_sample_lb = (self.iir_high_sample_lb * (1.0 - iir_amount_a)) + (high_sample_l * iir_amount_a);
                    high_sample_l -= self.iir_high_sample_lb;
                    self.iir_low_sample_lb = (self.iir_low_sample_lb * (1.0 - iir_amount_b)) + (bass_sample_l * iir_amount_b);
                    bass_sample_l = self.iir_low_sample_lb;

                    self.iir_high_sample_rb = (self.iir_high_sample_rb * (1.0 - iir_amount_a)) + (high_sample_r * iir_amount_a);
                    high_sample_r -= self.iir_high_sample_rb;
                    self.iir_low_sample_rb = (self.iir_low_sample_rb * (1.0 - iir_amount_b)) + (bass_sample_r * iir_amount_b);
                    bass_sample_r = self.iir_low_sample_rb;
                }

                self.iir_high_sample_l = (self.iir_high_sample_l * (1.0 - iir_amount_a)) + (high_sample_l * iir_amount_a);
                high_sample_l -= self.iir_high_sample_l;
                self.iir_low_sample_l = (self.iir_low_sample_l * (1.0 - iir_amount_b)) + (bass_sample_l * iir_amount_b);
                bass_sample_l = self.iir_low_sample_l;

                self.iir_high_sample_r = (self.iir_high_sample_r * (1.0 - iir_amount_a)) + (high_sample_r * iir_amount_a);
                high_sample_r -= self.iir_high_sample_r;
                self.iir_low_sample_r = (self.iir_low_sample_r * (1.0 - iir_amount_b)) + (bass_sample_r * iir_amount_b);
                bass_sample_r = self.iir_low_sample_r;

                mid_sample_l = (input_sample_l - bass_sample_l) - high_sample_l;
                mid_sample_r = (input_sample_r - bass_sample_r) - high_sample_r;

                // Drive / saturation section
                high_sample_l *= density_a + 1.0;
                let mut br_hl = high_sample_l.abs() * 1.57079633;
                if br_hl > 1.57079633 { br_hl = 1.57079633; }
                if density_a > 0.0 { br_hl = br_hl.sin(); } else { br_hl = 1.0 - br_hl.cos(); }
                if high_sample_l > 0.0 { high_sample_l = (high_sample_l * (1.0 - out_a)) + (br_hl * out_a); }
                else { high_sample_l = (high_sample_l * (1.0 - out_a)) - (br_hl * out_a); }

                high_sample_r *= density_a + 1.0;
                let mut br_hr = high_sample_r.abs() * 1.57079633;
                if br_hr > 1.57079633 { br_hr = 1.57079633; }
                if density_a > 0.0 { br_hr = br_hr.sin(); } else { br_hr = 1.0 - br_hr.cos(); }
                if high_sample_r > 0.0 { high_sample_r = (high_sample_r * (1.0 - out_a)) + (br_hr * out_a); }
                else { high_sample_r = (high_sample_r * (1.0 - out_a)) - (br_hr * out_a); }

                mid_sample_l *= density_b + 1.0;
                let mut br_ml = mid_sample_l.abs() * 1.57079633;
                if br_ml > 1.57079633 { br_ml = 1.57079633; }
                if density_b > 0.0 { br_ml = br_ml.sin(); } else { br_ml = 1.0 - br_ml.cos(); }
                if mid_sample_l > 0.0 { mid_sample_l = (mid_sample_l * (1.0 - out_b)) + (br_ml * out_b); }
                else { mid_sample_l = (mid_sample_l * (1.0 - out_b)) - (br_ml * out_b); }

                mid_sample_r *= density_b + 1.0;
                let mut br_mr = mid_sample_r.abs() * 1.57079633;
                if br_mr > 1.57079633 { br_mr = 1.57079633; }
                if density_b > 0.0 { br_mr = br_mr.sin(); } else { br_mr = 1.0 - br_mr.cos(); }
                if mid_sample_r > 0.0 { mid_sample_r = (mid_sample_r * (1.0 - out_b)) + (br_mr * out_b); }
                else { mid_sample_r = (mid_sample_r * (1.0 - out_b)) - (br_mr * out_b); }

                bass_sample_l *= density_c + 1.0;
                let mut br_bl = bass_sample_l.abs() * 1.57079633;
                if br_bl > 1.57079633 { br_bl = 1.57079633; }
                if density_c > 0.0 { br_bl = br_bl.sin(); } else { br_bl = 1.0 - br_bl.cos(); }
                if bass_sample_l > 0.0 { bass_sample_l = (bass_sample_l * (1.0 - out_c)) + (br_bl * out_c); }
                else { bass_sample_l = (bass_sample_l * (1.0 - out_c)) - (br_bl * out_c); }

                bass_sample_r *= density_c + 1.0;
                let mut br_br = bass_sample_r.abs() * 1.57079633;
                if br_br > 1.57079633 { br_br = 1.57079633; }
                if density_c > 0.0 { br_br = br_br.sin(); } else { br_br = 1.0 - br_br.cos(); }
                if bass_sample_r > 0.0 { bass_sample_r = (bass_sample_r * (1.0 - out_c)) + (br_br * out_c); }
                else { bass_sample_r = (bass_sample_r * (1.0 - out_c)) - (br_br * out_c); }

                input_sample_l = mid_sample_l + high_sample_l + bass_sample_l;
                input_sample_r = mid_sample_r + high_sample_r + bass_sample_r;
            }

            // 4. Output Stage
            if outputgain != 1.0 {
                input_sample_l *= outputgain;
                input_sample_r *= outputgain;
                let raw_sample_l = input_sample_l;
                let raw_sample_r = input_sample_r;

                if input_sample_l > 1.0 { input_sample_l = 1.0; }
                if input_sample_l < -1.0 { input_sample_l = -1.0; }
                if input_sample_r > 1.0 { input_sample_r = 1.0; }
                if input_sample_r < -1.0 { input_sample_r = -1.0; }

                let phat_sample_l = (input_sample_l * 1.57079633).sin();
                let phat_sample_r = (input_sample_r * 1.57079633).sin();

                input_sample_l *= 1.2533141373155;
                input_sample_r *= 1.2533141373155;

                let abs_l = input_sample_l.abs();
                let abs_r = input_sample_r.abs();
                let dist_sample_l = (input_sample_l * abs_l).sin() / if abs_l == 0.0 { 1.0 } else { abs_l };
                let dist_sample_r = (input_sample_r * abs_r).sin() / if abs_r == 0.0 { 1.0 } else { abs_r };

                input_sample_l = dist_sample_l;
                input_sample_r = dist_sample_r;

                if density < 1.0 {
                    input_sample_l = (raw_sample_l * (1.0 - density)) + (dist_sample_l * density);
                    input_sample_r = (raw_sample_r * (1.0 - density)) + (dist_sample_r * density);
                }
                if phattity > 0.0 {
                    input_sample_l = (input_sample_l * (1.0 - phattity)) + (phat_sample_l * phattity);
                    input_sample_r = (input_sample_r * (1.0 - phattity)) + (phat_sample_r * phattity);
                }
            }

            // 5. 32-bit Dither
            let expon_l = frexpf_exp(input_sample_l as f32);
            self.fpd_l ^= self.fpd_l << 13;
            self.fpd_l ^= self.fpd_l >> 17;
            self.fpd_l ^= self.fpd_l << 5;
            input_sample_l += (self.fpd_l as f64 - 2147483647.0) * 5.5e-36 * (2.0_f64).powi(expon_l + 62);

            let expon_r = frexpf_exp(input_sample_r as f32);
            self.fpd_r ^= self.fpd_r << 13;
            self.fpd_r ^= self.fpd_r >> 17;
            self.fpd_r ^= self.fpd_r << 5;
            input_sample_r += (self.fpd_r as f64 - 2147483647.0) * 5.5e-36 * (2.0_f64).powi(expon_r + 62);

            left[k] = input_sample_l as f32;
            right[k] = input_sample_r as f32;
        }

        self.last_min_multiplier = if engage_comp { min_mult } else { 1.0 };
    }
}
