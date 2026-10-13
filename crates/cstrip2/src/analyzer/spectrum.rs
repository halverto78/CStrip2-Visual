use realfft::num_complex::Complex;
use realfft::{RealFftPlanner, RealToComplex};
use std::f32::consts::PI;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

pub const FFT_SIZE: usize = 4096;
pub const NUM_BINS: usize = FFT_SIZE / 2 + 1;
pub const QUEUE_CAPACITY: usize = 8192;
const QUEUE_MASK: usize = QUEUE_CAPACITY - 1;
const QUEUE_SHIFT: u32 = 13; // 2^13 = 8192

#[inline(always)]
pub(crate) fn pack_slot(cycle: u32, sample: f32) -> u64 {
    ((cycle as u64) << 32) | (sample.to_bits() as u64)
}

#[inline(always)]
pub(crate) fn unpack_slot(packed: u64) -> (u32, f32) {
    let cycle = (packed >> 32) as u32;
    let sample = f32::from_bits(packed as u32);
    (cycle, sample)
}

/// Shared lock-free, zero-allocation, realtime-safe audio queue using AtomicU64.
///
/// Implements a Single-Producer Single-Consumer (SPSC) lock-free ring buffer
/// with Release/Acquire synchronization, atomic cycle/generation tags per slot,
/// and sliding-window frame coherence.
///
/// Invariants (F06):
/// 1. Sample + cycle tag are written and published in a single atomic 64-bit store.
/// 2. Happens-before: data store has Release ordering, head store has Release ordering.
/// 3. The consumer validates that every slot in the requested sliding window matches its
///    expected cycle tag (cycle = absolute_position >> QUEUE_SHIFT).
///    Any slot overwrite by a subsequent wrap immediately results in cycle != expected_cycle,
///    preventing any hybrid/torn frames across slot reuse.
/// 4. Sliding window semantics: each valid frame read represents the latest 4096 published
///    samples (head - FFT_SIZE .. head), preserving smooth ~60 Hz spectral updates.
pub struct SpectrumAudioQueue {
    pub(crate) buffer: Box<[AtomicU64; QUEUE_CAPACITY]>,
    pub write_head: AtomicU64,
}

impl SpectrumAudioQueue {
    pub fn new() -> Self {
        let mut v = Vec::with_capacity(QUEUE_CAPACITY);
        // Initialize with sentinel cycle u32::MAX so unwritten slots never match any valid cycle
        let initial_packed = pack_slot(u32::MAX, 0.0);
        for _ in 0..QUEUE_CAPACITY {
            v.push(AtomicU64::new(initial_packed));
        }
        let boxed_slice = v.into_boxed_slice();
        let boxed_array = match boxed_slice.try_into() {
            Ok(arr) => arr,
            Err(_) => unreachable!(),
        };

        Self {
            buffer: boxed_array,
            write_head: AtomicU64::new(0),
        }
    }

    #[inline(always)]
    pub fn push_sample(&self, mono_sample: f32) {
        let pos = self.write_head.load(Ordering::Relaxed);
        let slot = (pos as usize) & QUEUE_MASK;
        let cycle = (pos >> QUEUE_SHIFT) as u32;
        let packed = pack_slot(cycle, mono_sample);

        // 1. Write sample data + cycle tag atomically in slot with Release ordering
        self.buffer[slot].store(packed, Ordering::Release);

        // 2. Publish updated write index with Release ordering
        self.write_head.store(pos.wrapping_add(1), Ordering::Release);
    }

    pub fn read_frame(&self, dest: &mut [f32; FFT_SIZE]) -> bool {
        // Bounded retry loop (up to 3 attempts) in case of rare concurrent overrun
        for _retry in 0..3 {
            // 1. Observe published write position with Acquire ordering
            let head = self.write_head.load(Ordering::Acquire);
            if head < FFT_SIZE as u64 {
                // Not enough samples published yet to form a full frame
                return false;
            }

            // Sliding window: the latest FFT_SIZE published samples
            let start_pos = head - FFT_SIZE as u64;
            let mut valid = true;

            // 2. Read samples and verify per-slot atomic cycle tag
            for i in 0..FFT_SIZE {
                let pos = start_pos + i as u64;
                let slot = (pos as usize) & QUEUE_MASK;
                let expected_cycle = (pos >> QUEUE_SHIFT) as u32;

                let packed = self.buffer[slot].load(Ordering::Acquire);
                let (cycle, sample) = unpack_slot(packed);

                if cycle != expected_cycle {
                    // Slot was overwritten or not yet written for this generation
                    valid = false;
                    break;
                }

                dest[i] = sample;
            }

            if !valid {
                continue;
            }

            // Memory barrier ensuring all reads above complete before checking head_after
            std::sync::atomic::fence(Ordering::Acquire);

            // 3. Check for overrun: verify producer did not advance more than QUEUE_CAPACITY past start_pos
            let head_after = self.write_head.load(Ordering::Acquire);
            if head_after.wrapping_sub(start_pos) <= QUEUE_CAPACITY as u64 {
                return true;
            }
        }

        false
    }
}

/// Precomputed low-bin interference compensation parameters for bins 1..=4
/// due to positive/negative frequency overlap (+f / -f interference) under the Blackman-Harris window.
#[derive(Clone, Copy)]
struct LowBinComp {
    cos_theta: f32,
    sin_theta: f32,
    inv_s_plus: f32,
    inv_s_minus: f32,
}

pub struct SpectrumProcessor {
    pub sample_rate: f32,
    fft: Arc<dyn RealToComplex<f32>>,
    window: [f32; FFT_SIZE],
    window_s1: f32,
    low_bin_comp: [LowBinComp; 4],
    input_scratch: [f32; FFT_SIZE],
    complex_scratch: Vec<Complex<f32>>,

    // Smoothed visual bands (dBFS converted to normalized 0..1 height)
    pub bands_31: [f32; 31],
    pub peak_hold_31: [f32; 31],
    pub bands_63: [f32; 63],
    pub peak_hold_63: [f32; 63],
    pub bands_128: [f32; 128],
    pub peak_hold_128: [f32; 128],
}

impl SpectrumProcessor {
    pub fn new(sample_rate: f32) -> Self {
        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);

        // Blackman-Harris 4-term window
        let mut window = [0.0f32; FFT_SIZE];
        let a0 = 0.35875;
        let a1 = 0.48829;
        let a2 = 0.14128;
        let a3 = 0.01168;
        for i in 0..FFT_SIZE {
            let n = i as f32;
            let n_tot = (FFT_SIZE - 1) as f32;
            window[i] = a0 - a1 * (2.0 * PI * n / n_tot).cos()
                + a2 * (4.0 * PI * n / n_tot).cos()
                - a3 * (6.0 * PI * n / n_tot).cos();
        }
        let window_s1: f32 = window.iter().sum();

        // Precompute low-bin interference compensation parameters for bins 1..=4
        let mut low_bin_comp = [LowBinComp {
            cos_theta: 0.0,
            sin_theta: 0.0,
            inv_s_plus: 0.0,
            inv_s_minus: 0.0,
        }; 4];
        for b in 1..=4 {
            let theta = PI * (b as f32) * ((FFT_SIZE - 1) as f32) / (FFT_SIZE as f32);
            let mut w_2 = 0.0f32;
            for i in 0..FFT_SIZE {
                let m = i as f32 - ((FFT_SIZE - 1) as f32) * 0.5;
                w_2 += window[i] * (2.0 * PI * (2 * b) as f32 * m / (FFT_SIZE as f32)).cos();
            }
            let s_plus = window_s1 + w_2;
            let s_minus = window_s1 - w_2;
            low_bin_comp[b - 1] = LowBinComp {
                cos_theta: theta.cos(),
                sin_theta: theta.sin(),
                inv_s_plus: 1.0 / s_plus,
                inv_s_minus: 1.0 / s_minus,
            };
        }

        Self {
            sample_rate,
            fft,
            window,
            window_s1,
            low_bin_comp,
            input_scratch: [0.0; FFT_SIZE],
            complex_scratch: vec![Complex::new(0.0, 0.0); NUM_BINS],
            bands_31: [0.0; 31],
            peak_hold_31: [0.0; 31],
            bands_63: [0.0; 63],
            peak_hold_63: [0.0; 63],
            bands_128: [0.0; 128],
            peak_hold_128: [0.0; 128],
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
        let nyquist = sample_rate * 0.5;
        self.clear_above_nyquist(nyquist);
    }

    pub fn clear_above_nyquist(&mut self, nyquist: f32) {
        Self::clear_bands_above_nyquist::<31>(&mut self.bands_31, &mut self.peak_hold_31, nyquist);
        Self::clear_bands_above_nyquist::<63>(&mut self.bands_63, &mut self.peak_hold_63, nyquist);
        Self::clear_bands_above_nyquist::<128>(&mut self.bands_128, &mut self.peak_hold_128, nyquist);
    }

    fn clear_bands_above_nyquist<const N: usize>(
        bands: &mut [f32; N],
        peak_hold: &mut [f32; N],
        nyquist: f32,
    ) {
        for k in 0..N {
            let f_high = 20.0 * 10.0_f32.powf((3.0 * (k as f32 + 0.5)) / (N - 1) as f32);
            if f_high > nyquist {
                bands[k] = 0.0;
                peak_hold[k] = 0.0;
            }
        }
    }

    pub fn process_frame(&mut self, queue: &SpectrumAudioQueue, decay: f32) {
        if !queue.read_frame(&mut self.input_scratch) {
            return;
        }
        self.process_buffer_internal(decay);
    }

    pub fn process_raw_buffer(&mut self, input: &[f32; FFT_SIZE], decay: f32) {
        self.input_scratch.copy_from_slice(input);
        self.process_buffer_internal(decay);
    }

    fn process_buffer_internal(&mut self, decay: f32) {
        // Apply window
        for i in 0..FFT_SIZE {
            self.input_scratch[i] *= self.window[i];
        }

        if self
            .fft
            .process(&mut self.input_scratch, &mut self.complex_scratch)
            .is_err()
        {
            return;
        }

        let bin_hz = self.sample_rate / (FFT_SIZE as f32);

        self.update_log_bands::<31>(bin_hz, decay, |s| (&mut s.bands_31, &mut s.peak_hold_31));
        self.update_log_bands::<63>(bin_hz, decay, |s| (&mut s.bands_63, &mut s.peak_hold_63));
        self.update_log_bands::<128>(bin_hz, decay, |s| (&mut s.bands_128, &mut s.peak_hold_128));
    }

    fn update_log_bands<const N: usize>(
        &mut self,
        bin_hz: f32,
        decay: f32,
        get_bands: impl FnOnce(&mut Self) -> (&mut [f32; N], &mut [f32; N]),
    ) {
        let mut raw_mags = [0.0f32; N];
        let nyquist = self.sample_rate * 0.5;
        let window_s1 = self.window_s1;

        for k in 0..N {
            let f_center = 20.0 * 10.0_f32.powf((3.0 * k as f32) / (N - 1) as f32);
            let f_low = 20.0 * 10.0_f32.powf((3.0 * (k as f32 - 0.5).max(0.0)) / (N - 1) as f32);
            let f_high = 20.0 * 10.0_f32.powf((3.0 * (k as f32 + 0.5)) / (N - 1) as f32);

            // Bins beyond Nyquist physically contain no audio information
            if f_center > nyquist || f_low >= nyquist {
                raw_mags[k] = 0.0;
                continue;
            }

            let f_high_clamped = f_high.min(nyquist);
            let bin_start = ((f_low / bin_hz).floor() as usize).clamp(1, NUM_BINS - 1);
            let bin_end = ((f_high_clamped / bin_hz).ceil() as usize).clamp(bin_start, NUM_BINS - 1);

            let mut max_scaled_sq = 0.0f32;
            for b in bin_start..=bin_end {
                let scaled_sq = if b >= 1 && b <= 4 {
                    // Closed-form phase-independent elliptical compensation for low bins
                    // (+f and -f image interference under Blackman-Harris window)
                    let comp = &self.low_bin_comp[b - 1];
                    let rot_re = self.complex_scratch[b].re * comp.cos_theta - self.complex_scratch[b].im * comp.sin_theta;
                    let rot_im = self.complex_scratch[b].re * comp.sin_theta + self.complex_scratch[b].im * comp.cos_theta;
                    let term_re = rot_re * comp.inv_s_plus;
                    let term_im = rot_im * comp.inv_s_minus;
                    4.0 * (term_re * term_re + term_im * term_im)
                } else if b == NUM_BINS - 1 {
                    // Factor of 1 for Nyquist bin
                    let scale = 1.0 / window_s1;
                    self.complex_scratch[b].norm_sqr() * (scale * scale)
                } else {
                    // Factor of 2 for AC bins
                    let scale = 2.0 / window_s1;
                    self.complex_scratch[b].norm_sqr() * (scale * scale)
                };
                if scaled_sq > max_scaled_sq {
                    max_scaled_sq = scaled_sq;
                }
            }
            let peak_mag = max_scaled_sq.sqrt();
            let db = 20.0 * (peak_mag + 1e-9).log10();

            // Normalized to 0..1 for -60 dBFS to +12 dBFS
            let norm = ((db + 60.0) / 72.0).clamp(0.0, 1.0);
            raw_mags[k] = norm;
        }

        let (bands, peak_hold) = get_bands(self);
        for k in 0..N {
            let f_center = 20.0 * 10.0_f32.powf((3.0 * k as f32) / (N - 1) as f32);
            let f_low = 20.0 * 10.0_f32.powf((3.0 * (k as f32 - 0.5).max(0.0)) / (N - 1) as f32);
            if f_center > nyquist || f_low >= nyquist {
                bands[k] = 0.0;
                peak_hold[k] = 0.0;
                continue;
            }

            let target = raw_mags[k];
            if target > bands[k] {
                bands[k] = target; // Fast attack
            } else {
                bands[k] += (target - bands[k]) * decay; // Ballistic release
            }

            if target > peak_hold[k] {
                peak_hold[k] = target;
            } else {
                peak_hold[k] -= 0.003; // Slow peak-hold decay
                if peak_hold[k] < 0.0 {
                    peak_hold[k] = 0.0;
                }
            }
        }
    }
}

pub fn norm_to_dbfs(norm: f32) -> f32 {
    norm * 72.0 - 60.0
}

pub fn dbfs_to_norm(db: f32) -> f32 {
    ((db + 60.0) / 72.0).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn generate_sine_buffer_with_phase(
        freq_hz: f32,
        sample_rate: f32,
        amp: f32,
        phase_rad: f32,
    ) -> [f32; FFT_SIZE] {
        let mut buf = [0.0f32; FFT_SIZE];
        for i in 0..FFT_SIZE {
            let t = i as f32 / sample_rate;
            buf[i] = amp * (2.0 * PI * freq_hz * t + phase_rad).sin();
        }
        buf
    }

    fn generate_sine_buffer(freq_hz: f32, sample_rate: f32, amp: f32) -> [f32; FFT_SIZE] {
        generate_sine_buffer_with_phase(freq_hz, sample_rate, amp, 0.0)
    }

    fn find_target_band<const N: usize>(freq_hz: f32) -> usize {
        let k = ((freq_hz / 20.0).log10() / 3.0 * (N - 1) as f32).round() as usize;
        k.clamp(0, N - 1)
    }

    fn max_band_dbfs<const N: usize>(bands: &[f32; N]) -> f32 {
        let max_norm = bands.iter().copied().fold(0.0f32, f32::max);
        norm_to_dbfs(max_norm)
    }

    #[test]
    fn test_f10_coherent_sine_calibration_across_resolutions() {
        let sample_rate = 44100.0;

        // Test coherent bins: low, mid, high, near-Nyquist
        let test_bins = [
            5,    // Low: 53.83 Hz
            93,   // Mid: 1001.29 Hz
            900,  // High: 9689.94 Hz
            1800, // Near-Nyquist: 19379.88 Hz
        ];

        for &bin in &test_bins {
            let mut proc = SpectrumProcessor::new(sample_rate);
            let freq = bin as f32 * sample_rate / FFT_SIZE as f32;
            let sine = generate_sine_buffer(freq, sample_rate, 1.0); // 0 dBFS peak

            proc.process_raw_buffer(&sine, 0.0);

            let k31 = find_target_band::<31>(freq);
            let k63 = find_target_band::<63>(freq);
            let k128 = find_target_band::<128>(freq);

            let target_db31 = norm_to_dbfs(proc.bands_31[k31]);
            let target_db63 = norm_to_dbfs(proc.bands_63[k63]);
            let target_db128 = norm_to_dbfs(proc.bands_128[k128]);

            let db31 = max_band_dbfs(&proc.bands_31);
            let db63 = max_band_dbfs(&proc.bands_63);
            let db128 = max_band_dbfs(&proc.bands_128);

            // Verify specific target bands directly
            assert!(
                target_db31.abs() < 0.01,
                "Bin {} ({} Hz) target 31[{}] = {:.4} dBFS, expected 0.0 dBFS",
                bin, freq, k31, target_db31
            );
            assert!(
                target_db63.abs() < 0.01,
                "Bin {} ({} Hz) target 63[{}] = {:.4} dBFS, expected 0.0 dBFS",
                bin, freq, k63, target_db63
            );
            assert!(
                target_db128.abs() < 0.01,
                "Bin {} ({} Hz) target 128[{}] = {:.4} dBFS, expected 0.0 dBFS",
                bin, freq, k128, target_db128
            );

            // 0 dBFS expected: error must be extremely small (< 0.01 dB)
            assert!(
                db31.abs() < 0.01,
                "Bin {} ({} Hz) 31-band peak = {:.4} dBFS, expected 0.0 dBFS",
                bin, freq, db31
            );
            assert!(
                db63.abs() < 0.01,
                "Bin {} ({} Hz) 63-band peak = {:.4} dBFS, expected 0.0 dBFS",
                bin, freq, db63
            );
            assert!(
                db128.abs() < 0.01,
                "Bin {} ({} Hz) 128-band peak = {:.4} dBFS, expected 0.0 dBFS",
                bin, freq, db128
            );

            // Cross-resolution discrepancy must be essentially zero (< 0.001 dB)
            let diff_31_63 = (db31 - db63).abs();
            let diff_63_128 = (db63 - db128).abs();
            assert!(
                diff_31_63 < 0.001,
                "Diff 31 vs 63 bands = {:.6} dB", diff_31_63
            );
            assert!(
                diff_63_128 < 0.001,
                "Diff 63 vs 128 bands = {:.6} dB", diff_63_128
            );
        }
    }

    #[test]
    fn test_f10_low_bins_phase_and_amplitude_invariance() {
        let sample_rates = [44100.0f32, 48000.0f32, 96000.0f32];
        let test_bins = [1usize, 2, 3, 4];
        let phases_deg = [0.0f32, 45.0, 90.0, 135.0, 180.0];
        let test_cases = [
            (1.0f32, 0.0f32),
            (0.5f32, -6.0205999f32),
            (0.25f32, -12.0411998f32),
        ];

        for &sr in &sample_rates {
            let bin_hz = sr / FFT_SIZE as f32;
            for &bin in &test_bins {
                let freq = bin as f32 * bin_hz;
                let k31 = find_target_band::<31>(freq);
                let k63 = find_target_band::<63>(freq);
                let k128 = find_target_band::<128>(freq);

                for &(amp, expected_db) in &test_cases {
                    for &phase_deg in &phases_deg {
                        let phase_rad = phase_deg * PI / 180.0;
                        let mut proc = SpectrumProcessor::new(sr);
                        let sine = generate_sine_buffer_with_phase(freq, sr, amp, phase_rad);
                        proc.process_raw_buffer(&sine, 0.0);

                        let target_db31 = norm_to_dbfs(proc.bands_31[k31]);
                        let target_db63 = norm_to_dbfs(proc.bands_63[k63]);
                        let target_db128 = norm_to_dbfs(proc.bands_128[k128]);

                        let peak_db31 = max_band_dbfs(&proc.bands_31);
                        let peak_db63 = max_band_dbfs(&proc.bands_63);
                        let peak_db128 = max_band_dbfs(&proc.bands_128);

                        assert!(
                            (target_db31 - expected_db).abs() < 0.01,
                            "SR {} Bin {} ({} Hz, phase {} deg, amp {}) 31[{}] = {:.4} dBFS, expected {:.4} dBFS",
                            sr, bin, freq, phase_deg, amp, k31, target_db31, expected_db
                        );
                        assert!(
                            (target_db63 - expected_db).abs() < 0.01,
                            "SR {} Bin {} ({} Hz, phase {} deg, amp {}) 63[{}] = {:.4} dBFS, expected {:.4} dBFS",
                            sr, bin, freq, phase_deg, amp, k63, target_db63, expected_db
                        );
                        assert!(
                            (target_db128 - expected_db).abs() < 0.01,
                            "SR {} Bin {} ({} Hz, phase {} deg, amp {}) 128[{}] = {:.4} dBFS, expected {:.4} dBFS",
                            sr, bin, freq, phase_deg, amp, k128, target_db128, expected_db
                        );

                        assert!(
                            (peak_db31 - expected_db).abs() < 0.01,
                            "SR {} Bin {} peak 31 = {:.4} dBFS, expected {:.4} dBFS",
                            sr, bin, peak_db31, expected_db
                        );
                        assert!(
                            (peak_db63 - expected_db).abs() < 0.01,
                            "SR {} Bin {} peak 63 = {:.4} dBFS, expected {:.4} dBFS",
                            sr, bin, peak_db63, expected_db
                        );
                        assert!(
                            (peak_db128 - expected_db).abs() < 0.01,
                            "SR {} Bin {} peak 128 = {:.4} dBFS, expected {:.4} dBFS",
                            sr, bin, peak_db128, expected_db
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn test_f10_amplitude_linearity() {
        let sample_rate = 44100.0;
        let bin = 93; // 1001.29 Hz
        let freq = bin as f32 * sample_rate / FFT_SIZE as f32;

        let test_cases = [
            (1.0f32, 0.0f32),
            (0.5f32, -6.0205999f32),
            (0.25f32, -12.0411998f32),
        ];

        for &(amp, expected_db) in &test_cases {
            let mut proc = SpectrumProcessor::new(sample_rate);
            let sine = generate_sine_buffer(freq, sample_rate, amp);
            proc.process_raw_buffer(&sine, 0.0);

            let k31 = find_target_band::<31>(freq);
            let k63 = find_target_band::<63>(freq);
            let k128 = find_target_band::<128>(freq);

            let target_db31 = norm_to_dbfs(proc.bands_31[k31]);
            let target_db63 = norm_to_dbfs(proc.bands_63[k63]);
            let target_db128 = norm_to_dbfs(proc.bands_128[k128]);

            let db31 = max_band_dbfs(&proc.bands_31);
            let db63 = max_band_dbfs(&proc.bands_63);
            let db128 = max_band_dbfs(&proc.bands_128);

            assert!(
                (target_db31 - expected_db).abs() < 0.01,
                "Amp {} in target 31[{}] = {:.4} dBFS, expected {:.4} dBFS",
                amp, k31, target_db31, expected_db
            );
            assert!(
                (target_db63 - expected_db).abs() < 0.01,
                "Amp {} in target 63[{}] = {:.4} dBFS, expected {:.4} dBFS",
                amp, k63, target_db63, expected_db
            );
            assert!(
                (target_db128 - expected_db).abs() < 0.01,
                "Amp {} in target 128[{}] = {:.4} dBFS, expected {:.4} dBFS",
                amp, k128, target_db128, expected_db
            );

            assert!(
                (db31 - expected_db).abs() < 0.01,
                "Amp {} in 31 bands = {:.4} dBFS, expected {:.4} dBFS",
                amp, db31, expected_db
            );
            assert!(
                (db63 - expected_db).abs() < 0.01,
                "Amp {} in 63 bands = {:.4} dBFS, expected {:.4} dBFS",
                amp, db63, expected_db
            );
            assert!(
                (db128 - expected_db).abs() < 0.01,
                "Amp {} in 128 bands = {:.4} dBFS, expected {:.4} dBFS",
                amp, db128, expected_db
            );
        }
    }

    #[test]
    fn test_f10_sample_rates_calibration() {
        let sample_rates = [44100.0, 48000.0, 96000.0];

        for &sr in &sample_rates {
            let mut proc = SpectrumProcessor::new(sr);
            let bin = 100;
            let freq = bin as f32 * sr / FFT_SIZE as f32;
            let sine = generate_sine_buffer(freq, sr, 1.0);

            proc.process_raw_buffer(&sine, 0.0);

            let k31 = find_target_band::<31>(freq);
            let k63 = find_target_band::<63>(freq);
            let k128 = find_target_band::<128>(freq);

            let target_db31 = norm_to_dbfs(proc.bands_31[k31]);
            let target_db63 = norm_to_dbfs(proc.bands_63[k63]);
            let target_db128 = norm_to_dbfs(proc.bands_128[k128]);

            let db31 = max_band_dbfs(&proc.bands_31);
            let db63 = max_band_dbfs(&proc.bands_63);
            let db128 = max_band_dbfs(&proc.bands_128);

            assert!(
                target_db31.abs() < 0.01,
                "SR {} target 31[{}] = {:.4} dBFS, expected 0.0 dBFS", sr, k31, target_db31
            );
            assert!(
                target_db63.abs() < 0.01,
                "SR {} target 63[{}] = {:.4} dBFS, expected 0.0 dBFS", sr, k63, target_db63
            );
            assert!(
                target_db128.abs() < 0.01,
                "SR {} target 128[{}] = {:.4} dBFS, expected 0.0 dBFS", sr, k128, target_db128
            );

            assert!(
                db31.abs() < 0.01,
                "SR {} 31 bands = {:.4} dBFS, expected 0.0 dBFS", sr, db31
            );
            assert!(
                db63.abs() < 0.01,
                "SR {} 63 bands = {:.4} dBFS, expected 0.0 dBFS", sr, db63
            );
            assert!(
                db128.abs() < 0.01,
                "SR {} 128 bands = {:.4} dBFS, expected 0.0 dBFS", sr, db128
            );
        }
    }

    #[test]
    fn test_f10_silence_produces_zero() {
        let sample_rates = [44100.0, 48000.0, 96000.0];
        for &sr in &sample_rates {
            let mut proc = SpectrumProcessor::new(sr);
            let silence = [0.0f32; FFT_SIZE];
            proc.process_raw_buffer(&silence, 0.0);

            for k in 0..31 {
                assert_eq!(proc.bands_31[k], 0.0, "SR {} 31[{}] must be 0 on silence", sr, k);
                assert_eq!(proc.peak_hold_31[k], 0.0, "SR {} peak_hold 31[{}] must be 0", sr, k);
            }
            for k in 0..63 {
                assert_eq!(proc.bands_63[k], 0.0, "SR {} 63[{}] must be 0 on silence", sr, k);
                assert_eq!(proc.peak_hold_63[k], 0.0, "SR {} peak_hold 63[{}] must be 0", sr, k);
            }
            for k in 0..128 {
                assert_eq!(proc.bands_128[k], 0.0, "SR {} 128[{}] must be 0 on silence", sr, k);
                assert_eq!(proc.peak_hold_128[k], 0.0, "SR {} peak_hold 128[{}] must be 0", sr, k);
            }
        }
    }

    #[test]
    fn test_f10_non_coherent_tone_scalloping_bound() {
        let sample_rate = 44100.0;
        let mut proc = SpectrumProcessor::new(sample_rate);

        // 1000.0 Hz is not integer multiple of 44100 / 4096 (10.7666 Hz)
        let sine = generate_sine_buffer(1000.0, sample_rate, 1.0);
        proc.process_raw_buffer(&sine, 0.0);

        let db31 = max_band_dbfs(&proc.bands_31);
        let db63 = max_band_dbfs(&proc.bands_63);
        let db128 = max_band_dbfs(&proc.bands_128);

        // Blackman-Harris 4-term maximum scalloping loss is 0.83 dB
        assert!(
            db31 >= -0.83 && db31 <= 0.05,
            "31 bands off-bin peak = {:.4} dBFS (outside [-0.83, 0.05])", db31
        );
        assert!(
            db63 >= -0.83 && db63 <= 0.05,
            "63 bands off-bin peak = {:.4} dBFS (outside [-0.83, 0.05])", db63
        );
        assert!(
            db128 >= -0.83 && db128 <= 0.05,
            "128 bands off-bin peak = {:.4} dBFS (outside [-0.83, 0.05])", db128
        );

        // The peak band between resolutions should agree within 0.1 dB even with off-bin tone
        assert!(
            (db31 - db63).abs() < 0.1,
            "Diff 31 vs 63 = {:.4} dB", (db31 - db63).abs()
        );
        assert!(
            (db63 - db128).abs() < 0.1,
            "Diff 63 vs 128 = {:.4} dB", (db63 - db128).abs()
        );
    }

    #[test]
    fn test_f13_no_energy_above_nyquist_at_various_sample_rates() {
        let test_rates = [32000.0, 44100.0, 48000.0, 96000.0];

        for &sr in &test_rates {
            let mut proc = SpectrumProcessor::new(sr);
            let nyquist = sr * 0.5;

            // Fill buffer with broadband white noise / impulse
            let mut noise_buf = [0.0f32; FFT_SIZE];
            for i in 0..FFT_SIZE {
                noise_buf[i] = if i % 2 == 0 { 0.9 } else { -0.9 };
            }
            proc.process_raw_buffer(&noise_buf, 0.0);

            // Check 31 bands
            for k in 0..31 {
                let f_center = 20.0 * 10.0_f32.powf((3.0 * k as f32) / 30.0);
                if f_center > nyquist {
                    assert_eq!(
                        proc.bands_31[k], 0.0,
                        "SR {} band 31[{}] (center {:.1} Hz > Nyquist {:.1} Hz) has energy {:.4}",
                        sr, k, f_center, nyquist, proc.bands_31[k]
                    );
                    assert_eq!(
                        proc.peak_hold_31[k], 0.0,
                        "SR {} peak_hold 31[{}] > Nyquist has energy", sr, k
                    );
                }
            }

            // Check 63 bands
            for k in 0..63 {
                let f_center = 20.0 * 10.0_f32.powf((3.0 * k as f32) / 62.0);
                if f_center > nyquist {
                    assert_eq!(
                        proc.bands_63[k], 0.0,
                        "SR {} band 63[{}] (center {:.1} Hz > Nyquist {:.1} Hz) has energy {:.4}",
                        sr, k, f_center, nyquist, proc.bands_63[k]
                    );
                }
            }

            // Check 128 bands
            for k in 0..128 {
                let f_center = 20.0 * 10.0_f32.powf((3.0 * k as f32) / 127.0);
                if f_center > nyquist {
                    assert_eq!(
                        proc.bands_128[k], 0.0,
                        "SR {} band 128[{}] (center {:.1} Hz > Nyquist {:.1} Hz) has energy {:.4}",
                        sr, k, f_center, nyquist, proc.bands_128[k]
                    );
                }
            }
        }
    }

    #[test]
    fn test_f13_sample_rate_switch_clears_stale_bands_above_nyquist() {
        // Start at 44.1 kHz (Nyquist = 22050 Hz)
        let mut proc = SpectrumProcessor::new(44100.0);
        let sine_18k = generate_sine_buffer(18000.0, 44100.0, 1.0);
        proc.process_raw_buffer(&sine_18k, 0.0);

        // Verify band near 18k is non-zero
        let mut has_18k_energy = false;
        for k in 0..128 {
            let f = 20.0 * 10.0_f32.powf((3.0 * k as f32) / 127.0);
            if (f - 18000.0).abs() < 1000.0 && proc.bands_128[k] > 0.0 {
                has_18k_energy = true;
            }
        }
        assert!(has_18k_energy, "Should have energy near 18 kHz at 44.1 kHz SR");

        // Now switch host sample rate to 32 kHz (Nyquist = 16000 Hz)
        proc.set_sample_rate(32000.0);

        // All bands > 16 kHz must be immediately cleared to 0.0 with no stale data
        for k in 0..128 {
            let f = 20.0 * 10.0_f32.powf((3.0 * k as f32) / 127.0);
            if f > 16000.0 {
                assert_eq!(
                    proc.bands_128[k], 0.0,
                    "Band at {:.1} Hz must be 0.0 after switching to 32 kHz SR", f
                );
                assert_eq!(proc.peak_hold_128[k], 0.0);
            }
        }
    }

    #[test]
    fn test_f13_partial_band_down_switch_clears_stale_data_and_repopulates() {
        // Start at 44.1 kHz (Nyquist = 22050 Hz)
        let mut proc = SpectrumProcessor::new(44100.0);

        // 17 kHz tone at 44.1 kHz:
        // In 31 bands: band 29 has f_low ~ 14158.9 Hz, f_center ~ 15886.6 Hz, f_high ~ 17825.0 Hz.
        // The 17 kHz tone falls into band 29.
        let sine_17k = generate_sine_buffer(17000.0, 44100.0, 1.0);
        proc.process_raw_buffer(&sine_17k, 0.0);

        assert!(
            proc.bands_31[29] > 0.7,
            "Band 29 must capture 17 kHz tone at 44.1 kHz, got {:.4}",
            proc.bands_31[29]
        );
        assert!(
            proc.peak_hold_31[29] > 0.7,
            "Peak hold 29 must capture 17 kHz tone at 44.1 kHz"
        );

        // Switch host sample rate down to 32 kHz (Nyquist = 16000 Hz).
        // Since band 29 has f_high ~ 17825.0 Hz > 16000 Hz, its previous energy came from
        // frequencies above the new Nyquist limit. It must be immediately cleared!
        proc.set_sample_rate(32000.0);

        assert_eq!(
            proc.bands_31[29], 0.0,
            "Band 29 must be cleared immediately upon down-switch to 32 kHz SR"
        );
        assert_eq!(
            proc.peak_hold_31[29], 0.0,
            "Peak hold 29 must be cleared immediately upon down-switch to 32 kHz SR"
        );

        // Verify that processing silence at 32 kHz does NOT revive stale 17 kHz energy
        let silence = [0.0f32; FFT_SIZE];
        proc.process_raw_buffer(&silence, 0.0);
        assert_eq!(
            proc.bands_31[29], 0.0,
            "Band 29 must remain 0.0 on silence after down-switch"
        );
        assert_eq!(
            proc.peak_hold_31[29], 0.0,
            "Peak hold 29 must remain 0.0 on silence after down-switch"
        );

        // Now process a 15 kHz tone at 32 kHz (valid audio below 16 kHz Nyquist).
        // Band 29 must repopulate normally from the new valid audio!
        let sine_15k = generate_sine_buffer(15000.0, 32000.0, 1.0);
        proc.process_raw_buffer(&sine_15k, 0.0);

        assert!(
            proc.bands_31[29] > 0.7,
            "Band 29 must repopulate with valid 15 kHz tone at 32 kHz, got {:.4}",
            proc.bands_31[29]
        );
        assert!(
            proc.peak_hold_31[29] > 0.7,
            "Peak hold 29 must repopulate with valid 15 kHz tone at 32 kHz"
        );

        // Band 30 (center = 20000 Hz > 16000 Hz) must remain 0.0
        assert_eq!(proc.bands_31[30], 0.0);
        assert_eq!(proc.peak_hold_31[30], 0.0);
    }

    // =========================================================================
    // F06: TESTS DIRIGIDOS A LA COHERENCIA Y ORDENACIÓN DEL RING BUFFER
    // =========================================================================

    // Test 1: Deterministic adversarial test of Sol's exact interleaving race condition
    #[test]
    fn test_f06_sol_adversarial_deterministic_interleaving() {
        let queue = SpectrumAudioQueue::new();

        // 1. Fill first wrap (8192 samples, cycle 0 for all slots 0..8191)
        for i in 0..8192 {
            queue.push_sample(i as f32);
        }
        assert_eq!(queue.write_head.load(Ordering::Acquire), 8192);

        // 2. Advance to 12288 (slots 0..4095 overwritten with cycle 1)
        for i in 8192..12288 {
            queue.push_sample(i as f32);
        }
        assert_eq!(queue.write_head.load(Ordering::Acquire), 12288);

        // 3. Recreate Sol's exact interleaving:
        // Producer is at absolute pos 12288. Physical slot is 4096 (12288 & 8191 = 4096).
        // Producer writes new sample (99999.0) with cycle 1 into slot 4096:
        let slot_4096 = 4096;
        let new_cycle = 1u32;
        let new_sample = 99999.0f32;
        queue.buffer[slot_4096].store(pack_slot(new_cycle, new_sample), Ordering::Release);
        // Adversarial pause: Producer pauses BEFORE write_head.store(12289, Release)!
        // Therefore write_head remains at 12288!

        // 4. Consumer calls read_frame:
        let mut dest = [0.0f32; FFT_SIZE];
        let ok = queue.read_frame(&mut dest);

        // Assertions:
        // A hybrid frame (new sample 99999.0 mixed with old cycle 0 samples) is strictly forbidden!
        if ok {
            let has_new_sample = dest.iter().any(|&v| (v - 99999.0).abs() < 1e-4);
            let has_old_cycle0 = dest.iter().any(|&v| v < 8192.0);
            assert!(
                !(has_new_sample && has_old_cycle0),
                "SOL ADVERSARIAL FAILURE: Hybrid frame accepted with new sample and old cycle 0 samples!"
            );

            // With sliding semantics at head=12288, the valid window is 8192..12287 (all cycle 1)
            for (i, &v) in dest.iter().enumerate() {
                let exp = (8192 + i) as f32;
                assert_eq!(v, exp, "Mismatch at sample {} in valid window", i);
            }
        }

        // 5. Explicit check on old window:
        // If a consumer attempts to read the old window covering pos 4096 (which expects cycle 0):
        // Slot 4096 now holds cycle 1. The cycle check MUST fail!
        let packed = queue.buffer[4096].load(Ordering::Acquire);
        let (cycle, _) = unpack_slot(packed);
        assert_eq!(cycle, 1, "Slot 4096 holds cycle 1");
        let expected_cycle_for_old = (4096 >> QUEUE_SHIFT) as u32; // 0
        assert_ne!(
            cycle, expected_cycle_for_old,
            "Slot 4096 must detect generation mismatch (1 != 0)"
        );
    }

    // Test 2: Sliding window exact positions across non-aligned heads
    #[test]
    fn test_f06_sliding_window_exact_positions() {
        let queue = SpectrumAudioQueue::new();
        let targets = [4096usize, 4100, 5000, 7000, 8191, 8192, 9000, 12287];
        let mut current_samples = 0usize;
        let mut dest = [0.0f32; FFT_SIZE];

        for &target in &targets {
            while current_samples < target {
                queue.push_sample((current_samples + 1) as f32);
                current_samples += 1;
            }

            let head = queue.write_head.load(Ordering::Acquire);
            assert_eq!(head as usize, target);

            assert!(
                queue.read_frame(&mut dest),
                "read_frame must succeed at non-aligned head {}",
                target
            );

            let start = target - FFT_SIZE;
            for i in 0..FFT_SIZE {
                let exp = (start + i + 1) as f32;
                assert_eq!(
                    dest[i], exp,
                    "Sample {} mismatch at head {}: expected {}, got {}",
                    i, target, exp, dest[i]
                );
            }
        }
    }

    // Test 3: Short 2ms burst across 4096 boundary detected via sliding window
    #[test]
    fn test_f06_boundary_burst_immediate_detection() {
        let queue = SpectrumAudioQueue::new();
        let mut proc = SpectrumProcessor::new(44100.0);

        // Pre-fill silence up to 4050 samples
        for _ in 0..4050 {
            queue.push_sample(0.0);
        }

        // Short 2ms burst of 1 kHz tone at 0 dBFS (~88 samples at 44.1 kHz)
        // Position: 4050..4138, directly straddling the 4096 boundary
        for i in 0..88 {
            let t = i as f32 / 44100.0;
            let s = (2.0 * PI * 1000.0 * t).sin();
            queue.push_sample(s);
        }

        // Push 1500 samples of post-burst silence (simulating ~2 GUI frames of audio advance)
        // Total head is now 5638.
        for _ in 0..1500 {
            queue.push_sample(0.0);
        }

        // Read sliding frame at head = 5638 (captures samples 1542..5637, placing burst near window center)
        let mut frame = [0.0f32; FFT_SIZE];
        assert!(
            queue.read_frame(&mut frame),
            "Sliding window must read frame at head = 5638 without waiting for 8192"
        );

        // Process through SpectrumProcessor with decay = 0.0 (instantaneous response)
        proc.process_raw_buffer(&frame, 0.0);

        // In 31 bands, band 15 (center ~1000 Hz) captures the 1 kHz burst energy (~ -25.5 dBFS)
        let band_15 = proc.bands_31[15];
        assert!(
            band_15 > 0.3,
            "Sliding window must detect 1 kHz burst across boundary (expected ~ -25 dBFS / height > 0.3), got {:.4}",
            band_15
        );
    }

    // Test 4: Wraparound multi-cycle coherence
    #[test]
    fn test_f06_wraparound_multi_cycle_coherence() {
        let queue = SpectrumAudioQueue::new();
        let mut dest = [0.0f32; FFT_SIZE];

        // Push 50,000 samples (~6 full wraps of 8192-capacity buffer)
        for i in 0..50_000 {
            queue.push_sample((i + 1) as f32);

            // Periodically verify frame coherence across arbitrary points including boundary wraps
            if i >= FFT_SIZE && i % 2500 == 0 {
                assert!(queue.read_frame(&mut dest));
                let head = queue.write_head.load(Ordering::Acquire) as usize;
                let start = head - FFT_SIZE;
                for s in 0..FFT_SIZE {
                    let exp = (start + s + 1) as f32;
                    assert_eq!(
                        dest[s], exp,
                        "Wraparound mismatch at head {}, sample index {}: expected {}, got {}",
                        head, s, exp, dest[s]
                    );
                }
            }
        }
    }

    // Test 5: Overrun handling with latest window and zero tearing
    #[test]
    fn test_f06_overrun_latest_window_no_tearing() {
        use std::sync::atomic::AtomicBool;
        use std::sync::Arc;
        use std::thread;

        let queue = Arc::new(SpectrumAudioQueue::new());
        let running = Arc::new(AtomicBool::new(true));

        // Fast producer thread pushes 80,000 samples
        let q_prod = Arc::clone(&queue);
        let run_prod = Arc::clone(&running);
        let prod_h = thread::spawn(move || {
            for i in 0..80_000 {
                q_prod.push_sample((i + 1) as f32);
            }
            run_prod.store(false, Ordering::Release);
        });

        // Slow consumer sleeps between reads to deliberately provoke overruns
        let q_cons = Arc::clone(&queue);
        let run_cons = Arc::clone(&running);
        let cons_h = thread::spawn(move || {
            let mut dest = [0.0f32; FFT_SIZE];
            let mut valid_frames = 0;

            while run_cons.load(Ordering::Acquire) || valid_frames < 10 {
                thread::sleep(std::time::Duration::from_millis(2));
                if q_cons.read_frame(&mut dest) {
                    // Check internal frame continuity: dest[s] == dest[0] + s
                    let first = dest[0];
                    for (s, &val) in dest.iter().enumerate() {
                        let exp = first + s as f32;
                        assert_eq!(
                            val, exp,
                            "Torn frame during overrun: sample {} is {}, expected {}",
                            s, val, exp
                        );
                    }
                    valid_frames += 1;
                }
            }
            assert!(valid_frames > 0, "Consumer must have read valid frames despite overruns");
        });

        prod_h.join().unwrap();
        cons_h.join().unwrap();
    }

    // Test 6: Concurrent stress test across multiple scheduling regimes
    #[test]
    fn test_f06_concurrent_stress_various_rates() {
        use std::sync::atomic::AtomicBool;
        use std::sync::Arc;
        use std::thread;

        for regime in 0..3 {
            let queue = Arc::new(SpectrumAudioQueue::new());
            let running = Arc::new(AtomicBool::new(true));

            let q_prod = Arc::clone(&queue);
            let run_prod = Arc::clone(&running);
            let prod_h = thread::spawn(move || {
                for i in 0..60_000 {
                    q_prod.push_sample((i + 1) as f32);
                    if regime == 1 && i % 1000 == 0 {
                        thread::yield_now();
                    } else if regime == 2 && i % 100 == 0 {
                        thread::yield_now();
                    }
                }
                run_prod.store(false, Ordering::Release);
            });

            let q_cons = Arc::clone(&queue);
            let run_cons = Arc::clone(&running);
            let cons_h = thread::spawn(move || {
                let mut dest = [0.0f32; FFT_SIZE];
                let mut attempted = 0usize;
                let mut valid = 0usize;
                let mut retried_or_discarded = 0usize;
                let mut hybrids = 0usize;

                while run_cons.load(Ordering::Acquire) || valid < 10 {
                    attempted += 1;
                    if q_cons.read_frame(&mut dest) {
                        valid += 1;
                        let first = dest[0];
                        for (s, &val) in dest.iter().enumerate() {
                            let exp = first + s as f32;
                            if (val - exp).abs() > 1e-4 {
                                hybrids += 1;
                            }
                        }
                    } else {
                        retried_or_discarded += 1;
                    }
                    thread::yield_now();
                }

                assert_eq!(hybrids, 0, "Regime {}: ZERO hybrid frames allowed, saw {}", regime, hybrids);
                assert!(valid > 0, "Regime {}: Consumer must read valid frames", regime);
                println!(
                    "Regime {}: attempted={}, valid={}, retried/discarded={}, hybrids={}",
                    regime, attempted, valid, retried_or_discarded, hybrids
                );
            });

            prod_h.join().unwrap();
            cons_h.join().unwrap();
        }
    }

    // Test 7: Publication order (datos y ciclo visibles al observar head)
    #[test]
    fn test_f06_publication_order_data_visible_on_head_observation() {
        use std::sync::atomic::AtomicBool;
        use std::sync::Arc;
        use std::thread;

        let queue = Arc::new(SpectrumAudioQueue::new());
        let running = Arc::new(AtomicBool::new(true));

        let total_samples = 40_000u64;

        let q_prod = Arc::clone(&queue);
        let run_prod = Arc::clone(&running);
        let prod_handle = thread::spawn(move || {
            for i in 0..total_samples {
                let sample_val = (i + 1) as f32 * 0.25;
                q_prod.push_sample(sample_val);
                if i % 1000 == 0 {
                    thread::yield_now();
                }
            }
            run_prod.store(false, Ordering::Release);
        });

        let q_cons = Arc::clone(&queue);
        let run_cons = Arc::clone(&running);
        let cons_handle = thread::spawn(move || {
            let mut last_observed_pos = 0u64;
            let mut checks = 0usize;

            while run_cons.load(Ordering::Acquire) || last_observed_pos < total_samples {
                let head = q_cons.write_head.load(Ordering::Acquire);
                if head > last_observed_pos {
                    let sample_idx = head - 1;
                    let slot = (sample_idx as usize) & QUEUE_MASK;
                    let packed = q_cons.buffer[slot].load(Ordering::Acquire);
                    let (cycle, val) = unpack_slot(packed);
                    let exp_cycle = (sample_idx >> QUEUE_SHIFT) as u32;
                    let exp_val = (sample_idx + 1) as f32 * 0.25;

                    assert_eq!(
                        cycle, exp_cycle,
                        "Cycle mismatch at pos {}: expected {}, got {}",
                        sample_idx, exp_cycle, cycle
                    );
                    assert_eq!(
                        val, exp_val,
                        "Publication before write race at pos {}: expected {}, got {}",
                        sample_idx, exp_val, val
                    );
                    last_observed_pos = head;
                    checks += 1;
                }
                std::hint::spin_loop();
            }
            assert!(checks > 0, "Consumer must have observed head advancements");
        });

        prod_handle.join().unwrap();
        cons_handle.join().unwrap();
    }

    // Test 8: Memory ordering happens-before & boundary rollover
    #[test]
    fn test_f06_memory_ordering_happens_before() {
        let queue = SpectrumAudioQueue::new();

        // 1. Initial state: head == 0, read_frame must return false (no partial/stale read)
        let mut dest = [999.0f32; FFT_SIZE];
        assert!(!queue.read_frame(&mut dest));
        assert_eq!(dest[0], 999.0); // Untouched

        // 2. Partial frame: push FFT_SIZE - 1 samples
        for i in 0..(FFT_SIZE - 1) {
            queue.push_sample((i + 1) as f32);
        }
        assert!(!queue.read_frame(&mut dest), "Partial frame must not be published");

        // 3. Complete exact frame: push the 4096th sample
        queue.push_sample(4096.0);
        assert!(queue.read_frame(&mut dest), "Full frame must be published");
        for i in 0..FFT_SIZE {
            assert_eq!(dest[i], (i + 1) as f32, "Sample {} content mismatch", i);
        }

        // 4. Double buffer / sliding rollover: push second frame
        for i in 0..FFT_SIZE {
            queue.push_sample((i + 10001) as f32);
        }
        assert!(queue.read_frame(&mut dest));
        for i in 0..FFT_SIZE {
            assert_eq!(dest[i], (i + 10001) as f32, "Sample {} content mismatch in frame 2", i);
        }
    }
}
