//! EQ Curve Measurement & Worker Thread (Latest State Wins)
//!
//! Uses stationary sinusoidal tones at -40 dBFS with coherent integer-cycle DFT
//! on the fundamental frequency to obtain 100% mathematically exact response
//! without spectral leakage from harmonics or non-linearities.

use std::f64::consts::PI;
#[cfg(test)]
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};

use crate::dsp::cstrip2::CapacitorFilter;

pub const NUM_CURVE_POINTS: usize = 128;
pub const NOMINAL_REF_LEVEL: f64 = 0.01; // -40 dBFS

/// Generates the 128 log-spaced frequencies from 20 Hz to 20 kHz
pub fn get_log_frequencies_128() -> [f64; NUM_CURVE_POINTS] {
    let mut freqs = [0.0; NUM_CURVE_POINTS];
    for k in 0..NUM_CURVE_POINTS {
        freqs[k] = 20.0 * 10.0_f64.powf((3.0 * k as f64) / (NUM_CURVE_POINTS - 1) as f64);
    }
    freqs
}

/// Standalone simulation of Cstrip2 3-Band EQ block for pure offline measurement
pub struct Cstrip2EqOnly {
    last_sample_l: f64,
    last2_sample_l: f64,
    flipthree: i32,
    flip: bool,
    triplet_la: f64,
    triplet_lb: f64,
    triplet_lc: f64,
    iir_high_sample_lc: f64,
    iir_high_sample_ld: f64,
    iir_high_sample_le: f64,
    iir_low_sample_lc: f64,
    iir_low_sample_ld: f64,
    iir_low_sample_le: f64,
    iir_high_sample_la: f64,
    iir_high_sample_lb: f64,
    iir_low_sample_la: f64,
    iir_low_sample_lb: f64,
    iir_high_sample_l: f64,
    iir_low_sample_l: f64,
}

impl Cstrip2EqOnly {
    pub fn new() -> Self {
        Self {
            last_sample_l: 0.0,
            last2_sample_l: 0.0,
            flipthree: 0,
            flip: false,
            triplet_la: 0.0,
            triplet_lb: 0.0,
            triplet_lc: 0.0,
            iir_high_sample_lc: 0.0,
            iir_high_sample_ld: 0.0,
            iir_high_sample_le: 0.0,
            iir_low_sample_lc: 0.0,
            iir_low_sample_ld: 0.0,
            iir_low_sample_le: 0.0,
            iir_high_sample_la: 0.0,
            iir_high_sample_lb: 0.0,
            iir_low_sample_la: 0.0,
            iir_low_sample_lb: 0.0,
            iir_high_sample_l: 0.0,
            iir_low_sample_l: 0.0,
        }
    }

    #[inline(always)]
    pub fn process_sample(
        &mut self,
        input_sample_l: f64,
        density_a: f64,
        density_b: f64,
        density_c: f64,
        triplet_intensity: f64,
        iir_amount_a: f64,
        iir_amount_b: f64,
        out_a: f64,
        out_b: f64,
        out_c: f64,
        engage_comp: bool,
    ) -> f64 {
        self.last2_sample_l = self.last_sample_l;
        self.last_sample_l = input_sample_l;
        self.flipthree += 1;
        if self.flipthree < 1 || self.flipthree > 3 {
            self.flipthree = 1;
        }

        let mut high_sample_l;
        let mut bass_sample_l;
        let mut triplet_factor_l;

        match self.flipthree {
            1 => {
                triplet_factor_l = self.last2_sample_l - input_sample_l;
                self.triplet_la += triplet_factor_l;
                self.triplet_lc -= triplet_factor_l;
                triplet_factor_l = self.triplet_la * triplet_intensity;
                self.iir_high_sample_lc = (self.iir_high_sample_lc * (1.0 - iir_amount_a)) + (input_sample_l * iir_amount_a);
                high_sample_l = input_sample_l - self.iir_high_sample_lc;
                self.iir_low_sample_lc = (self.iir_low_sample_lc * (1.0 - iir_amount_b)) + (input_sample_l * iir_amount_b);
                bass_sample_l = self.iir_low_sample_lc;
            }
            2 => {
                triplet_factor_l = self.last2_sample_l - input_sample_l;
                self.triplet_lb += triplet_factor_l;
                self.triplet_la -= triplet_factor_l;
                triplet_factor_l = self.triplet_lb * triplet_intensity;
                self.iir_high_sample_ld = (self.iir_high_sample_ld * (1.0 - iir_amount_a)) + (input_sample_l * iir_amount_a);
                high_sample_l = input_sample_l - self.iir_high_sample_ld;
                self.iir_low_sample_ld = (self.iir_low_sample_ld * (1.0 - iir_amount_b)) + (input_sample_l * iir_amount_b);
                bass_sample_l = self.iir_low_sample_ld;
            }
            _ => {
                triplet_factor_l = self.last2_sample_l - input_sample_l;
                self.triplet_lc += triplet_factor_l;
                self.triplet_lb -= triplet_factor_l;
                triplet_factor_l = self.triplet_lc * triplet_intensity;
                self.iir_high_sample_le = (self.iir_high_sample_le * (1.0 - iir_amount_a)) + (input_sample_l * iir_amount_a);
                high_sample_l = input_sample_l - self.iir_high_sample_le;
                self.iir_low_sample_le = (self.iir_low_sample_le * (1.0 - iir_amount_b)) + (input_sample_l * iir_amount_b);
                bass_sample_l = self.iir_low_sample_le;
            }
        }
        self.triplet_la /= 2.0;
        self.triplet_lb /= 2.0;
        self.triplet_lc /= 2.0;
        high_sample_l += triplet_factor_l;

        if engage_comp {
            self.flip = !self.flip;
        }

        if self.flip {
            self.iir_high_sample_la = (self.iir_high_sample_la * (1.0 - iir_amount_a)) + (high_sample_l * iir_amount_a);
            high_sample_l -= self.iir_high_sample_la;
            self.iir_low_sample_la = (self.iir_low_sample_la * (1.0 - iir_amount_b)) + (bass_sample_l * iir_amount_b);
            bass_sample_l = self.iir_low_sample_la;
        } else {
            self.iir_high_sample_lb = (self.iir_high_sample_lb * (1.0 - iir_amount_a)) + (high_sample_l * iir_amount_a);
            high_sample_l -= self.iir_high_sample_lb;
            self.iir_low_sample_lb = (self.iir_low_sample_lb * (1.0 - iir_amount_b)) + (bass_sample_l * iir_amount_b);
            bass_sample_l = self.iir_low_sample_lb;
        }

        self.iir_high_sample_l = (self.iir_high_sample_l * (1.0 - iir_amount_a)) + (high_sample_l * iir_amount_a);
        high_sample_l -= self.iir_high_sample_l;
        self.iir_low_sample_l = (self.iir_low_sample_l * (1.0 - iir_amount_b)) + (bass_sample_l * iir_amount_b);
        bass_sample_l = self.iir_low_sample_l;

        let mut mid_sample_l = (input_sample_l - bass_sample_l) - high_sample_l;

        // Non-linear waveshaping
        high_sample_l *= density_a + 1.0;
        let mut br = high_sample_l.abs() * 1.57079633;
        if br > 1.57079633 { br = 1.57079633; }
        if density_a > 0.0 { br = br.sin(); } else { br = 1.0 - br.cos(); }
        if high_sample_l > 0.0 { high_sample_l = (high_sample_l * (1.0 - out_a)) + (br * out_a); }
        else { high_sample_l = (high_sample_l * (1.0 - out_a)) - (br * out_a); }

        mid_sample_l *= density_b + 1.0;
        let mut br = mid_sample_l.abs() * 1.57079633;
        if br > 1.57079633 { br = 1.57079633; }
        if density_b > 0.0 { br = br.sin(); } else { br = 1.0 - br.cos(); }
        if mid_sample_l > 0.0 { mid_sample_l = (mid_sample_l * (1.0 - out_b)) + (br * out_b); }
        else { mid_sample_l = (mid_sample_l * (1.0 - out_b)) - (br * out_b); }

        bass_sample_l *= density_c + 1.0;
        let mut br = bass_sample_l.abs() * 1.57079633;
        if br > 1.57079633 { br = 1.57079633; }
        if density_c > 0.0 { br = br.sin(); } else { br = 1.0 - br.cos(); }
        if bass_sample_l > 0.0 { bass_sample_l = (bass_sample_l * (1.0 - out_c)) + (br * out_c); }
        else { bass_sample_l = (bass_sample_l * (1.0 - out_c)) - (br * out_c); }

        mid_sample_l + high_sample_l + bass_sample_l
    }
}

/// Measures the fundamental gain in dB of the Cstrip2 Tone block (Capacitor -> 3-Band EQ) at a target frequency
/// using coherent integer-cycle DFT detection to guarantee 0.0 dB leakage from harmonics.
pub fn measure_coherent_fundamental_gain(
    f_target: f64,
    amp: f64,
    sample_rate: f64,
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
    g: f64,
    engage_comp: bool,
) -> f64 {
    let mut cap = CapacitorFilter::new();
    let mut eq = Cstrip2EqOnly::new();
    let overallscale = sample_rate;

    let lowpass_amount = f * f;
    let highpass_amount = g * g;

    let raw_da = (a * 12.0) - 6.0;
    let raw_db = (b * 12.0) - 6.0;
    let raw_dc = (c * 12.0) - 6.0;

    let density_a = 10.0_f64.powf(raw_da / 20.0) - 1.0;
    let density_b = 10.0_f64.powf(raw_db / 20.0) - 1.0;
    let density_c = 10.0_f64.powf(raw_dc / 20.0) - 1.0;

    let triplet_intensity = -density_a;
    let iir_amount_a = (((d * d * 15.0) + 1.0) * 1000.0) / overallscale;
    let iir_amount_b = (((e * e * 1570.0) + 30.0) * 10.0) / overallscale;
    let out_a = density_a.abs();
    let out_b = density_b.abs();
    let out_c = density_c.abs();

    // Determine coherent frequency with exact integer cycles K over N samples
    // Choose K so measurement duration is at least 30 ms and at least 8 cycles
    let k_cycles = ((0.03_f64 * f_target).ceil() as usize).max(8);
    let ideal_n = (k_cycles as f64) * sample_rate / f_target;
    let n_samples = ideal_n.round() as usize;
    let f_coherent = (k_cycles as f64) * sample_rate / (n_samples as f64);

    let omega = 2.0 * PI * f_coherent / sample_rate;
    let period_samples = (sample_rate / f_coherent).max(1.0);

    // F16: Adaptive settling derived from probe frequency and sample rate.
    // 25 full cycles for low-frequency filters, with a minimum floor of 40 ms (0.040 * sample_rate).
    let settling_cycles = 25.0;
    let cycle_samples = (settling_cycles * period_samples).ceil() as usize;
    let min_samples = (0.040 * sample_rate).ceil() as usize;
    let n_settle = cycle_samples.max(min_samples);

    // 1. Settling phase (Capacitor -> 3-Band EQ)
    for n in 0..n_settle {
        let in_val = amp * (omega * n as f64).sin();
        let cap_val = cap.process_sample(in_val, highpass_amount, lowpass_amount);
        eq.process_sample(
            cap_val, density_a, density_b, density_c, triplet_intensity,
            iir_amount_a, iir_amount_b, out_a, out_b, out_c, engage_comp,
        );
    }

    // 2. Coherent measurement phase (exactly k_cycles integer periods)
    let mut in_re = 0.0;
    let mut in_im = 0.0;
    let mut out_re = 0.0;
    let mut out_im = 0.0;

    for n in 0..n_samples {
        let t = (n + n_settle) as f64;
        let in_val = amp * (omega * t).sin();
        let cap_val = cap.process_sample(in_val, highpass_amount, lowpass_amount);
        let out_val = eq.process_sample(
            cap_val, density_a, density_b, density_c, triplet_intensity,
            iir_amount_a, iir_amount_b, out_a, out_b, out_c, engage_comp,
        );

        let cos_val = (omega * t).cos();
        let sin_val = (omega * t).sin();

        in_re += in_val * cos_val;
        in_im -= in_val * sin_val;
        out_re += out_val * cos_val;
        out_im -= out_val * sin_val;
    }

    let in_mag = (in_re * in_re + in_im * in_im).sqrt();
    let out_mag = (out_re * out_re + out_im * out_im).sqrt();

    if in_mag > 1e-15 {
        20.0 * (out_mag / in_mag).log10()
    } else {
        0.0
    }
}

/// Computes the complete 128-point Tone response curve (Capacitor -> 3-Band EQ) in dB
pub fn compute_tone_curve_128(
    sample_rate: f64,
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
    f: f64,
    g: f64,
    engage_comp: bool,
) -> [f32; NUM_CURVE_POINTS] {
    let freqs = get_log_frequencies_128();
    let mut curve = [0.0f32; NUM_CURVE_POINTS];

    for (k, &freq) in freqs.iter().enumerate() {
        let gain_db = measure_coherent_fundamental_gain(
            freq,
            NOMINAL_REF_LEVEL,
            sample_rate,
            a,
            b,
            c,
            d,
            e,
            f,
            g,
            engage_comp,
        );
        curve[k] = gain_db as f32;
    }

    curve
}

/// Backward compatibility wrapper for pure EQ curve (with neutral Capacitor: LowCap=1.0, HiCap=0.0)
pub fn compute_eq_curve_128(
    sample_rate: f64,
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    e: f64,
) -> [f32; NUM_CURVE_POINTS] {
    compute_tone_curve_128(sample_rate, a, b, c, d, e, 1.0, 0.0, true)
}

/// Request snapshot sent to the curve worker
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurveParamsSnapshot {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
    pub g: f64,
    pub sample_rate: f64,
    pub engage_comp: bool,
}

struct SharedState {
    pending: Option<(u64, CurveParamsSnapshot)>,
    latest_requested_gen: u64,
    shutdown: bool,
}

struct WorkerShared {
    state: Mutex<SharedState>,
    condvar: Condvar,
    latest_curve: Mutex<Arc<[f32; NUM_CURVE_POINTS]>>,
    published_gen: AtomicU64,
    #[cfg(test)]
    test_hook: Mutex<Option<Arc<dyn Fn(usize, u64) + Send + Sync>>>,
    #[cfg(test)]
    test_pre_commit_hook: Mutex<Option<Arc<dyn Fn(u64) + Send + Sync>>>,
    #[cfg(test)]
    publication_log: Mutex<Vec<u64>>,
}

/// Dedicated background curve worker implementing monotonic request generations and latest-state-wins
pub struct CurveWorker {
    shared: Arc<WorkerShared>,
    #[cfg(test)]
    pub(crate) abort_early: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl CurveWorker {
    pub fn new() -> Self {
        let initial_curve = Arc::new([0.0f32; NUM_CURVE_POINTS]);
        let shared = Arc::new(WorkerShared {
            state: Mutex::new(SharedState {
                pending: None,
                latest_requested_gen: 0,
                shutdown: false,
            }),
            condvar: Condvar::new(),
            latest_curve: Mutex::new(initial_curve),
            published_gen: AtomicU64::new(0),
            #[cfg(test)]
            test_hook: Mutex::new(None),
            #[cfg(test)]
            test_pre_commit_hook: Mutex::new(None),
            #[cfg(test)]
            publication_log: Mutex::new(Vec::new()),
        });

        #[cfg(test)]
        let abort_early = Arc::new(AtomicBool::new(true));
        #[cfg(test)]
        let abort_early_clone = Arc::clone(&abort_early);
        let shared_clone = Arc::clone(&shared);

        let thread_handle = thread::spawn(move || {
            let mut last_computed_params: Option<CurveParamsSnapshot> = None;

            loop {
                // Wait for a pending request safely synchronized with condvar
                let (req_gen, next_request) = {
                    let mut lock = shared_clone.state.lock().unwrap();
                    while lock.pending.is_none() && !lock.shutdown {
                        lock = shared_clone.condvar.wait(lock).unwrap();
                    }

                    if lock.shutdown {
                        break;
                    }

                    lock.pending.take().unwrap() // Coalescing: only latest state is taken!
                };

                // If identical to last computed and already published, skip redundant recalculation
                if Some(next_request) == last_computed_params {
                    // Check generation under state lock before updating published_gen
                    let state_lock = shared_clone.state.lock().unwrap();
                    if req_gen == state_lock.latest_requested_gen {
                        shared_clone.published_gen.store(req_gen, Ordering::Release);
                    }
                    continue;
                }

                // Compute the 128 points
                let freqs = get_log_frequencies_128();
                let mut curve = [0.0f32; NUM_CURVE_POINTS];
                let mut aborted = false;

                for (k, &freq) in freqs.iter().enumerate() {
                    #[cfg(test)]
                    {
                        // Test hook callback (if registered)
                        let hook_opt = {
                            let hook_lock = shared_clone.test_hook.lock().unwrap();
                            hook_lock.clone()
                        };
                        if let Some(hook) = hook_opt {
                            hook(k, req_gen);
                        }
                    }

                    // Check if superseded while computing
                    #[cfg(test)]
                    let should_check_abort = abort_early_clone.load(Ordering::Relaxed);
                    #[cfg(not(test))]
                    let should_check_abort = true;

                    if should_check_abort {
                        let is_stale = {
                            let lock = shared_clone.state.lock().unwrap();
                            lock.latest_requested_gen > req_gen
                        };
                        if is_stale {
                            aborted = true;
                            break;
                        }
                    }

                    let gain_db = measure_coherent_fundamental_gain(
                        freq,
                        NOMINAL_REF_LEVEL,
                        next_request.sample_rate,
                        next_request.a,
                        next_request.b,
                        next_request.c,
                        next_request.d,
                        next_request.e,
                        next_request.f,
                        next_request.g,
                        next_request.engage_comp,
                    );
                    curve[k] = gain_db as f32;
                }

                if aborted {
                    continue;
                }

                #[cfg(test)]
                {
                    // Test hook called right after completing all 128 points, BEFORE acquiring commit lock
                    let pre_commit_hook_opt = {
                        let hook_lock = shared_clone.test_pre_commit_hook.lock().unwrap();
                        hook_lock.clone()
                    };
                    if let Some(hook) = pre_commit_hook_opt {
                        hook(req_gen);
                    }
                }

                // F12 Atomic Commit Protocol:
                // Hold state lock while verifying req_gen == latest_requested_gen AND publishing to latest_curve!
                // This makes `validar generación -> publicar curva` strictly indivisible with respect to request_update().
                {
                    let state_lock = shared_clone.state.lock().unwrap();
                    if req_gen == state_lock.latest_requested_gen {
                        {
                            let mut curve_lock = shared_clone.latest_curve.lock().unwrap();
                            *curve_lock = Arc::new(curve);
                        }
                        shared_clone.published_gen.store(req_gen, Ordering::Release);
                        #[cfg(test)]
                        shared_clone.publication_log.lock().unwrap().push(req_gen);
                        last_computed_params = Some(next_request);
                    }
                    // If req_gen != state_lock.latest_requested_gen:
                    // Stale result is discarded immediately while holding state lock.
                    // It is NEVER published to latest_curve!
                }
            }
        });

        Self {
            shared,
            #[cfg(test)]
            abort_early,
            thread_handle: Some(thread_handle),
        }
    }

    /// Notify worker with new parameter snapshot. Overwrites any previous pending request.
    /// Returns the monotonic generation assigned to this request.
    pub fn request_update(&self, params: CurveParamsSnapshot) -> u64 {
        let mut lock = self.shared.state.lock().unwrap();
        lock.latest_requested_gen = lock.latest_requested_gen.wrapping_add(1);
        let gen = lock.latest_requested_gen;
        lock.pending = Some((gen, params)); // Overwrite: latest state wins, no backlog!
        self.shared.condvar.notify_one();
        gen
    }

    /// Retrieve the latest complete coherent curve frame for rendering (instantaneous read)
    pub fn get_latest_curve(&self) -> Arc<[f32; NUM_CURVE_POINTS]> {
        let lock = self.shared.latest_curve.lock().unwrap();
        Arc::clone(&lock)
    }

    /// Retrieve the monotonic generation of the currently published curve
    pub fn get_published_gen(&self) -> u64 {
        self.shared.published_gen.load(Ordering::Acquire)
    }

    /// Set an optional test hook called at point `k` for request generation `gen`.
    #[cfg(test)]
    pub(crate) fn set_test_hook<F>(&self, hook: F)
    where
        F: Fn(usize, u64) + Send + Sync + 'static,
    {
        let mut lock = self.shared.test_hook.lock().unwrap();
        *lock = Some(Arc::new(hook));
    }

    /// Clear test hook
    #[cfg(test)]
    pub(crate) fn clear_test_hook(&self) {
        let mut lock = self.shared.test_hook.lock().unwrap();
        *lock = None;
    }

    /// Retrieve list of published generations (for deterministic test observation)
    #[cfg(test)]
    pub fn get_publication_log(&self) -> Vec<u64> {
        let lock = self.shared.publication_log.lock().unwrap();
        lock.clone()
    }

    /// Clear publication log
    #[cfg(test)]
    pub fn clear_publication_log(&self) {
        let mut lock = self.shared.publication_log.lock().unwrap();
        lock.clear();
    }

    /// Set an optional test hook called after 128 points are computed, BEFORE taking commit lock.
    #[cfg(test)]
    pub(crate) fn set_pre_commit_hook<F>(&self, hook: F)
    where
        F: Fn(u64) + Send + Sync + 'static,
    {
        let mut lock = self.shared.test_pre_commit_hook.lock().unwrap();
        *lock = Some(Arc::new(hook));
    }

    /// Clear pre-commit hook
    #[cfg(test)]
    pub(crate) fn clear_pre_commit_hook(&self) {
        let mut lock = self.shared.test_pre_commit_hook.lock().unwrap();
        *lock = None;
    }
}

impl Drop for CurveWorker {
    fn drop(&mut self) {
        // F09: Graceful shutdown correctly synchronized with the condvar
        {
            let mut lock = self.shared.state.lock().unwrap();
            lock.shutdown = true;
        } // Must release mutex before join to avoid deadlock!

        self.shared.condvar.notify_all();
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Barrier};
    use std::time::Duration;

    // =========================================================================
    // F12 TESTS: DETERMINISTIC VERIFICATION OF STALE WORK DISCARD & COALESCING
    // =========================================================================

    #[test]
    fn test_f12_stale_publication_deterministic() {
        let worker = CurveWorker::new();
        // In this test, let A finish computing its points without early abort
        // to verify that publication rejection discards A even when computation completes.
        worker.abort_early.store(false, Ordering::Relaxed);

        let initial_gen = worker.get_published_gen();
        assert_eq!(initial_gen, 0);

        let snap_a = CurveParamsSnapshot {
            a: 0.8, b: 0.2, c: 0.7, d: 0.6, e: 0.4, f: 0.8, g: 0.2,
            sample_rate: 44100.0, engage_comp: false,
        };
        let snap_b = CurveParamsSnapshot {
            a: 0.2, b: 0.8, c: 0.3, d: 0.4, e: 0.6, f: 0.3, g: 0.8,
            sample_rate: 44100.0, engage_comp: false,
        };

        let barrier_paused = Arc::new(Barrier::new(2));
        let barrier_resume = Arc::new(Barrier::new(2));

        let b_paused = Arc::clone(&barrier_paused);
        let b_resume = Arc::clone(&barrier_resume);
        let hook_fired = Arc::new(AtomicBool::new(false));
        let h_fired = Arc::clone(&hook_fired);

        // Hook pauses worker at point k = 5 of generation 1
        worker.set_test_hook(move |k, gen| {
            if gen == 1 && k == 5 && !h_fired.swap(true, Ordering::SeqCst) {
                b_paused.wait();
                b_resume.wait();
            }
        });

        // 1. Request A begins (generation 1)
        let gen_a = worker.request_update(snap_a);
        assert_eq!(gen_a, 1);

        // 2. Wait until worker is paused in the middle of computing Request A
        barrier_paused.wait();

        // 3. Send Request B (generation 2) while worker is paused on Request A
        let gen_b = worker.request_update(snap_b);
        assert_eq!(gen_b, 2);

        // 4. Unpause worker to allow Request A to finish
        barrier_resume.wait();

        // Wait until generation 2 finishes and publishes
        let mut waited = 0;
        while worker.get_published_gen() < gen_b && waited < 200 {
            thread::sleep(Duration::from_millis(15));
            waited += 1;
        }

        // 5 & 6. Verify that Request A was NEVER published as visible cache
        let published_gen = worker.get_published_gen();
        assert_eq!(
            published_gen, gen_b,
            "Visible result must correspond to Request B (gen {}), got {}",
            gen_b, published_gen
        );

        // 7. Verify published curve data corresponds to B
        let expected_curve_b = compute_tone_curve_128(
            snap_b.sample_rate, snap_b.a, snap_b.b, snap_b.c, snap_b.d, snap_b.e,
            snap_b.f, snap_b.g, snap_b.engage_comp,
        );
        let actual_curve = worker.get_latest_curve();
        for i in 0..NUM_CURVE_POINTS {
            assert!(
                (actual_curve[i] - expected_curve_b[i]).abs() < 1e-4,
                "Point {} mismatch: published curve must be B", i
            );
        }

        worker.clear_test_hook();
    }

    #[test]
    fn test_f12_multiple_requests_coalescing() {
        let worker = CurveWorker::new();
        worker.abort_early.store(false, Ordering::Relaxed);

        let snap_a = CurveParamsSnapshot {
            a: 0.9, b: 0.1, c: 0.5, d: 0.5, e: 0.5, f: 1.0, g: 0.0,
            sample_rate: 44100.0, engage_comp: false,
        };
        let snap_b = CurveParamsSnapshot {
            a: 0.1, b: 0.9, c: 0.5, d: 0.5, e: 0.5, f: 1.0, g: 0.0,
            sample_rate: 44100.0, engage_comp: false,
        };
        let snap_c = CurveParamsSnapshot {
            a: 0.5, b: 0.5, c: 0.9, d: 0.2, e: 0.8, f: 0.6, g: 0.4,
            sample_rate: 44100.0, engage_comp: false,
        };

        let barrier_paused = Arc::new(Barrier::new(2));
        let barrier_resume = Arc::new(Barrier::new(2));
        let b_paused = Arc::clone(&barrier_paused);
        let b_resume = Arc::clone(&barrier_resume);
        let hook_fired = Arc::new(AtomicBool::new(false));
        let h_fired = Arc::clone(&hook_fired);

        worker.set_test_hook(move |k, gen| {
            if gen == 1 && k == 5 && !h_fired.swap(true, Ordering::SeqCst) {
                b_paused.wait();
                b_resume.wait();
            }
        });

        // 1. Send Request A
        let gen_a = worker.request_update(snap_a);
        assert_eq!(gen_a, 1);

        // 2. Wait until worker is paused on A
        barrier_paused.wait();

        // 3. Send Request B then immediately Request C while worker is computing A
        let gen_b = worker.request_update(snap_b);
        let gen_c = worker.request_update(snap_c);
        assert_eq!(gen_b, 2);
        assert_eq!(gen_c, 3);

        // 4. Resume worker
        barrier_resume.wait();

        // Wait until generation 3 is published
        let mut waited = 0;
        while worker.get_published_gen() < gen_c && waited < 200 {
            thread::sleep(Duration::from_millis(15));
            waited += 1;
        }

        // 5. Verify neither A nor B was ever published after C; result must be C
        assert_eq!(worker.get_published_gen(), gen_c);

        let expected_curve_c = compute_tone_curve_128(
            snap_c.sample_rate, snap_c.a, snap_c.b, snap_c.c, snap_c.d, snap_c.e,
            snap_c.f, snap_c.g, snap_c.engage_comp,
        );
        let actual_curve = worker.get_latest_curve();
        for i in 0..NUM_CURVE_POINTS {
            assert!(
                (actual_curve[i] - expected_curve_c[i]).abs() < 1e-4,
                "Point {} mismatch: published curve must be C", i
            );
        }

        worker.clear_test_hook();
    }

    #[test]
    fn test_f12_early_abort_when_superseded() {
        let worker = CurveWorker::new();
        // Enable early abort (default)
        worker.abort_early.store(true, Ordering::Relaxed);

        let snap_a = CurveParamsSnapshot {
            a: 0.8, b: 0.2, c: 0.5, d: 0.5, e: 0.5, f: 1.0, g: 0.0,
            sample_rate: 44100.0, engage_comp: false,
        };
        let snap_b = CurveParamsSnapshot {
            a: 0.2, b: 0.8, c: 0.5, d: 0.5, e: 0.5, f: 1.0, g: 0.0,
            sample_rate: 44100.0, engage_comp: false,
        };

        let barrier_paused = Arc::new(Barrier::new(2));
        let barrier_resume = Arc::new(Barrier::new(2));
        let b_paused = Arc::clone(&barrier_paused);
        let b_resume = Arc::clone(&barrier_resume);
        let hook_fired = Arc::new(AtomicBool::new(false));
        let h_fired = Arc::clone(&hook_fired);

        let points_computed_a = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let pts_a = Arc::clone(&points_computed_a);

        worker.set_test_hook(move |k, gen| {
            if gen == 1 {
                pts_a.fetch_add(1, Ordering::SeqCst);
                if k == 10 && !h_fired.swap(true, Ordering::SeqCst) {
                    b_paused.wait();
                    b_resume.wait();
                }
            }
        });

        // 1. Send Request A
        worker.request_update(snap_a);

        // 2. Wait until worker is paused at k = 10
        barrier_paused.wait();

        // 3. Send Request B
        let gen_b = worker.request_update(snap_b);

        // 4. Resume worker
        barrier_resume.wait();

        // Wait until B is published
        let mut waited = 0;
        while worker.get_published_gen() < gen_b && waited < 200 {
            thread::sleep(Duration::from_millis(15));
            waited += 1;
        }

        // Verify that Request A aborted early: it did NOT compute all 128 points!
        let total_a_points = points_computed_a.load(Ordering::SeqCst);
        assert!(
            total_a_points < NUM_CURVE_POINTS,
            "Request A must abort early when superseded; computed {} of {} points",
            total_a_points, NUM_CURVE_POINTS
        );

        assert_eq!(worker.get_published_gen(), gen_b);
        worker.clear_test_hook();
    }

    #[test]
    fn test_f12_toctou_adversarial_pre_commit_race() {
        let worker = CurveWorker::new();
        // Disable abort_early so Request A finishes all 128 points and reaches pre-commit hook
        worker.abort_early.store(false, Ordering::Relaxed);

        let snap_a = CurveParamsSnapshot {
            a: 0.8, b: 0.2, c: 0.7, d: 0.6, e: 0.4, f: 0.8, g: 0.2,
            sample_rate: 44100.0, engage_comp: false,
        };
        let snap_b = CurveParamsSnapshot {
            a: 0.2, b: 0.8, c: 0.3, d: 0.4, e: 0.6, f: 0.3, g: 0.8,
            sample_rate: 44100.0, engage_comp: false,
        };

        let barrier_at_pre_commit = Arc::new(Barrier::new(2));
        let barrier_allow_resume = Arc::new(Barrier::new(2));

        let b_pre = Arc::clone(&barrier_at_pre_commit);
        let b_res = Arc::clone(&barrier_allow_resume);

        // Pre-commit hook: pauses worker after 128 points of generation 1, BEFORE acquiring state lock
        worker.set_pre_commit_hook(move |gen| {
            if gen == 1 {
                b_pre.wait();
                b_res.wait();
            }
        });

        // 1. Send Request A (generation 1)
        let gen_a = worker.request_update(snap_a);
        assert_eq!(gen_a, 1);

        // 2. Wait until worker completes all 128 points and reaches pre-commit hook
        barrier_at_pre_commit.wait();

        // 3. At this precise moment:
        // - A has completed all 128 points
        // - A has NOT acquired commit lock yet
        // - NO curve has been published yet
        assert_eq!(worker.get_publication_log(), Vec::<u64>::new());
        assert_eq!(worker.get_published_gen(), 0);

        // 4. Send Request B (generation 2) while A is paused before commit!
        let gen_b = worker.request_update(snap_b);
        assert_eq!(gen_b, 2);

        // B is now officially requested in state.
        // Publication log still has NOT recorded 1:
        assert_eq!(worker.get_publication_log(), Vec::<u64>::new());

        // 5. Unpause A to let it proceed to commit block
        barrier_allow_resume.wait();

        // 6. Wait until generation 2 finishes and publishes
        let mut waited = 0;
        while worker.get_published_gen() < gen_b && waited < 200 {
            thread::sleep(Duration::from_millis(15));
            waited += 1;
        }

        // 7. Verification:
        // Generation 2 must be published
        assert_eq!(worker.get_published_gen(), gen_b);

        // CRITICAL OBSERVATION:
        // Request A was NEVER published! Not even transiently!
        let log = worker.get_publication_log();
        assert_eq!(
            log,
            vec![gen_b],
            "Publication log must contain ONLY Request B (gen {}). Got: {:?}",
            gen_b,
            log
        );
        assert!(!log.contains(&gen_a), "Request A must NEVER appear in publication log!");

        // Published curve data must match B exactly
        let expected_curve_b = compute_tone_curve_128(
            snap_b.sample_rate, snap_b.a, snap_b.b, snap_b.c, snap_b.d, snap_b.e,
            snap_b.f, snap_b.g, snap_b.engage_comp,
        );
        let actual_curve = worker.get_latest_curve();
        for i in 0..NUM_CURVE_POINTS {
            assert!(
                (actual_curve[i] - expected_curve_b[i]).abs() < 1e-4,
                "Point {} mismatch: published curve must be B", i
            );
        }

        worker.clear_pre_commit_hook();
    }

    #[test]
    fn test_f12_toctou_reverse_order_valid_publication() {
        let worker = CurveWorker::new();

        let snap_a = CurveParamsSnapshot {
            a: 0.8, b: 0.2, c: 0.7, d: 0.6, e: 0.4, f: 0.8, g: 0.2,
            sample_rate: 44100.0, engage_comp: false,
        };
        let snap_b = CurveParamsSnapshot {
            a: 0.2, b: 0.8, c: 0.3, d: 0.4, e: 0.6, f: 0.3, g: 0.8,
            sample_rate: 44100.0, engage_comp: false,
        };

        // 1. Request A is issued
        let gen_a = worker.request_update(snap_a);
        assert_eq!(gen_a, 1);

        // 2. A runs and completes commit legitimately before B enters
        let mut waited = 0;
        while worker.get_published_gen() < gen_a && waited < 200 {
            thread::sleep(Duration::from_millis(15));
            waited += 1;
        }
        assert_eq!(worker.get_published_gen(), gen_a);
        assert_eq!(worker.get_publication_log(), vec![gen_a]);

        // 3. Now Request B is issued
        let gen_b = worker.request_update(snap_b);
        assert_eq!(gen_b, 2);

        // 4. B runs and publishes legitimately
        let mut waited = 0;
        while worker.get_published_gen() < gen_b && waited < 200 {
            thread::sleep(Duration::from_millis(15));
            waited += 1;
        }
        assert_eq!(worker.get_published_gen(), gen_b);

        // In reverse order, both publications were valid: [1, 2]
        assert_eq!(worker.get_publication_log(), vec![gen_a, gen_b]);
    }

    #[test]
    fn test_f12_identical_request_reuse() {
        let worker = CurveWorker::new();

        let snap_a = CurveParamsSnapshot {
            a: 0.5, b: 0.5, c: 0.5, d: 0.5, e: 0.5, f: 0.5, g: 0.5,
            sample_rate: 44100.0, engage_comp: false,
        };

        let gen_1 = worker.request_update(snap_a);
        let mut waited = 0;
        while worker.get_published_gen() < gen_1 && waited < 200 {
            thread::sleep(Duration::from_millis(15));
            waited += 1;
        }
        assert_eq!(worker.get_published_gen(), gen_1);
        assert_eq!(worker.get_publication_log(), vec![gen_1]);

        // Send identical request
        let gen_2 = worker.request_update(snap_a);
        let mut waited = 0;
        while worker.get_published_gen() < gen_2 && waited < 200 {
            thread::sleep(Duration::from_millis(15));
            waited += 1;
        }
        assert_eq!(worker.get_published_gen(), gen_2);
        // Identical request reused the existing curve, so publication_log didn't push a redundant recalculation
        assert_eq!(worker.get_publication_log(), vec![gen_1]);
    }


    // =========================================================================
    // F16 TESTS: ADAPTIVE SETTLING ACCURACY AGAINST HIGH-PRECISION REFERENCE
    // =========================================================================

    #[test]
    fn test_f16_settling_accuracy_against_long_reference() {
        let test_freqs = [20.0, 30.0, 50.0, 100.0, 1000.0, 5000.0, 10000.0, 20000.0];
        let sample_rates = [44100.0, 48000.0, 96000.0];

        // 4 demanding configurations per audit requirements:
        let test_cases = [
            ("Case 1 (Flat)", (0.5, 0.5, 0.5, 0.5, 0.5, 1.0, 0.0)),
            ("Case 2 (Cutoffs agresivos)", (0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5)),
            ("Case 3 (EQ fuerte)", (0.9, 0.2, 0.8, 0.9, 0.1, 1.0, 0.0)),
            ("Case 4 (Combinación)", (0.8, 0.3, 0.7, 0.8, 0.2, 0.7, 0.3)),
        ];

        let mut max_err_all = 0.0f64;

        for &(case_name, (a, b, c, d, e, f, g)) in &test_cases {
            for &sr in &sample_rates {
                for &freq in &test_freqs {
                    if freq >= sr * 0.5 {
                        continue;
                    }

                    // Long reference: 80,000 samples settling
                    let mut cap_ref = CapacitorFilter::new();
                    let mut eq_ref = Cstrip2EqOnly::new();
                    let lowpass_amount = f * f;
                    let highpass_amount = g * g;
                    let raw_da = (a * 12.0) - 6.0;
                    let raw_db = (b * 12.0) - 6.0;
                    let raw_dc = (c * 12.0) - 6.0;
                    let density_a = 10.0_f64.powf(raw_da / 20.0) - 1.0;
                    let density_b = 10.0_f64.powf(raw_db / 20.0) - 1.0;
                    let density_c = 10.0_f64.powf(raw_dc / 20.0) - 1.0;
                    let triplet_intensity = -density_a;
                    let iir_amount_a = (((d * d * 15.0) + 1.0) * 1000.0) / sr;
                    let iir_amount_b = (((e * e * 1570.0) + 30.0) * 10.0) / sr;
                    let out_a = density_a.abs();
                    let out_b = density_b.abs();
                    let out_c = density_c.abs();

                    let k_cycles = ((0.03_f64 * freq).ceil() as usize).max(8);
                    let ideal_n = (k_cycles as f64) * sr / freq;
                    let n_samples = ideal_n.round() as usize;
                    let f_coherent = (k_cycles as f64) * sr / (n_samples as f64);
                    let omega = 2.0 * PI * f_coherent / sr;

                    let n_settle_ref = 80_000usize;
                    for n in 0..n_settle_ref {
                        let in_val = NOMINAL_REF_LEVEL * (omega * n as f64).sin();
                        let cap_val = cap_ref.process_sample(in_val, highpass_amount, lowpass_amount);
                        eq_ref.process_sample(
                            cap_val, density_a, density_b, density_c, triplet_intensity,
                            iir_amount_a, iir_amount_b, out_a, out_b, out_c, false,
                        );
                    }
                    let mut in_re = 0.0;
                    let mut in_im = 0.0;
                    let mut out_re = 0.0;
                    let mut out_im = 0.0;
                    for n in 0..n_samples {
                        let t = (n + n_settle_ref) as f64;
                        let in_val = NOMINAL_REF_LEVEL * (omega * t).sin();
                        let cap_val = cap_ref.process_sample(in_val, highpass_amount, lowpass_amount);
                        let out_val = eq_ref.process_sample(
                            cap_val, density_a, density_b, density_c, triplet_intensity,
                            iir_amount_a, iir_amount_b, out_a, out_b, out_c, false,
                        );
                        let cos_val = (omega * t).cos();
                        let sin_val = (omega * t).sin();
                        in_re += in_val * cos_val;
                        in_im -= in_val * sin_val;
                        out_re += out_val * cos_val;
                        out_im -= out_val * sin_val;
                    }
                    let ref_gain = 20.0 * ((out_re * out_re + out_im * out_im).sqrt() / (in_re * in_re + in_im * in_im).sqrt()).log10();

                    // Measure using production function
                    let actual_gain = measure_coherent_fundamental_gain(
                        freq, NOMINAL_REF_LEVEL, sr, a, b, c, d, e, f, g, false,
                    );

                    let err = (actual_gain - ref_gain).abs();
                    if err > max_err_all {
                        max_err_all = err;
                    }

                    assert!(
                        err < 0.05,
                        "{}: error at {} Hz, SR {} exceeds 0.05 dB: actual={:.4}, ref={:.4}, err={:.5} dB",
                        case_name, freq, sr, actual_gain, ref_gain, err
                    );
                }
            }
        }

        println!("Maximum F16 settling error against 80,000-sample reference: {:.5} dB", max_err_all);
        assert!(max_err_all < 0.05, "Maximum error must be under 0.05 dB");
    }

    #[test]
    fn test_f16_adaptive_settling_properties() {
        // Verify settling samples at representative frequencies
        let sr_44k = 44100.0;
        let sr_96k = 96000.0;

        let calc_settle = |freq: f64, sr: f64| {
            let k_cycles = ((0.03_f64 * freq).ceil() as usize).max(8);
            let ideal_n = (k_cycles as f64) * sr / freq;
            let n_samples = ideal_n.round() as usize;
            let f_coherent = (k_cycles as f64) * sr / (n_samples as f64);
            let period_samples = (sr / f_coherent).max(1.0);
            let cycle_samples = (25.0 * period_samples).ceil() as usize;
            let min_samples = (0.040 * sr).ceil() as usize;
            cycle_samples.max(min_samples)
        };

        // At 20 Hz, 44.1k: 25 cycles is > 55,000 samples
        let n_20_44 = calc_settle(20.0, sr_44k);
        assert!(n_20_44 > 50_000, "20 Hz at 44.1k must have > 50,000 samples settling, got {}", n_20_44);

        // At 20 Hz, 96k: 25 cycles is > 110,000 samples
        let n_20_96 = calc_settle(20.0, sr_96k);
        assert!(n_20_96 > 110_000, "20 Hz at 96k must have > 110,000 samples settling, got {}", n_20_96);

        // At 10 kHz, settling respects the 40 ms time floor
        let n_10k_44 = calc_settle(10000.0, sr_44k);
        let n_10k_96 = calc_settle(10000.0, sr_96k);
        assert_eq!(n_10k_44, (0.040 * sr_44k).ceil() as usize);
        assert_eq!(n_10k_96, (0.040 * sr_96k).ceil() as usize);
    }
}
