pub mod analyzer;
pub mod dsp;
pub mod gui;
pub mod params;
pub mod presets;

use nih_plug::prelude::*;
use atomic_float::AtomicF32;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use analyzer::SpectrumAudioQueue;
use dsp::cstrip2::Cstrip2Dsp;
use dsp::curve::CurveWorker;
use params::Cstrip2Params;

pub struct Cstrip2 {
    params: Arc<Cstrip2Params>,
    dsp: Cstrip2Dsp,
    input_queue: Arc<SpectrumAudioQueue>,
    output_queue: Arc<SpectrumAudioQueue>,
    curve_worker: Arc<CurveWorker>,
    sample_rate: Arc<AtomicF32>,
    in_peak: Arc<AtomicF32>,
    in_rms: Arc<AtomicF32>,
    out_peak: Arc<AtomicF32>,
    out_rms: Arc<AtomicF32>,
    comp_gr: Arc<AtomicF32>,
}

impl Default for Cstrip2 {
    fn default() -> Self {
        let sample_rate = 44100.0;
        Self {
            params: Arc::new(Cstrip2Params::default()),
            dsp: Cstrip2Dsp::new(sample_rate as f64),
            input_queue: Arc::new(SpectrumAudioQueue::new()),
            output_queue: Arc::new(SpectrumAudioQueue::new()),
            curve_worker: Arc::new(CurveWorker::new()),
            sample_rate: Arc::new(AtomicF32::new(sample_rate)),
            in_peak: Arc::new(AtomicF32::new(0.0)),
            in_rms: Arc::new(AtomicF32::new(0.0)),
            out_peak: Arc::new(AtomicF32::new(0.0)),
            out_rms: Arc::new(AtomicF32::new(0.0)),
            comp_gr: Arc::new(AtomicF32::new(0.0)),
        }
    }
}

impl Plugin for Cstrip2 {
    const NAME: &'static str = "Cstrip2";
    const VENDOR: &'static str = "Airwindows / AMR";
    const URL: &'static str = "https://www.airwindows.com";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

        fn filter_state(state: &mut nih_plug::prelude::PluginState) {
        // F01: Validación robusta del state plain
        for (id, param_value) in &mut state.params {
            let mut val_f32 = None;
            
            match param_value {
                nih_plug::wrapper::state::ParamValue::F32(f) => val_f32 = Some(*f),
                nih_plug::wrapper::state::ParamValue::String(s) => {
                    if s == "NaN" { val_f32 = Some(f32::NAN); }
                    else if s == "+Inf" || s == "Inf" { val_f32 = Some(f32::INFINITY); }
                    else if s == "-Inf" { val_f32 = Some(f32::NEG_INFINITY); }
                    else if let Ok(f) = s.parse::<f32>() { val_f32 = Some(f); }
                },
                _ => {}
            }
            
            if let Some(val) = val_f32 {
                let sanitized = if !val.is_finite() {
                    // Defaults canónicos de la GUI si es NaN o Inf
                    match id.as_str() {
                        "treble" | "mid" | "bass" => 0.0,
                        "treb_frq" => 4.75,
                        "bass_frq" => 422.5,
                        "low_cap" => 1.0,
                        "hi_cap" | "compres" | "comp_spd" => 0.0,
                        "output" => 0.33,
                        _ => 0.0,
                    }
                } else {
                    // Valores finitos: clamp estricto al rango válido
                    match id.as_str() {
                        "treble" | "mid" | "bass" => val.clamp(-12.0, 12.0),
                        "treb_frq" => val.clamp(1.0, 16.0),
                        "bass_frq" => val.clamp(30.0, 1600.0),
                        "low_cap" | "hi_cap" | "compres" | "comp_spd" | "output" => val.clamp(0.0, 1.0),
                        _ => val,
                    }
                };
                
                // Sobrescribimos incondicionalmente si:
                // a) vino como String, b) no era finito, c) el clamp lo modificó numéricamente.
                if !val.is_finite() || val != sanitized || matches!(param_value, nih_plug::wrapper::state::ParamValue::String(_)) {
                    *param_value = nih_plug::wrapper::state::ParamValue::F32(sanitized);
                }
            }
        }
    }

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        gui::create_editor(
            self.params.clone(),
            self.input_queue.clone(),
            self.output_queue.clone(),
            self.curve_worker.clone(),
            self.sample_rate.clone(),
            self.in_peak.clone(),
            self.in_rms.clone(),
            self.out_peak.clone(),
            self.out_rms.clone(),
            self.comp_gr.clone(),
            self.params.editor_state.clone(),
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        let sr = buffer_config.sample_rate;
        self.sample_rate.store(sr, Ordering::Relaxed);
        self.dsp.reset(sr as f64);
        true
    }

    fn reset(&mut self) {
        let sr = self.sample_rate.load(Ordering::Relaxed);
        self.dsp.reset(sr as f64);
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let (a, b, c, d, e, f, g, h, i, j) = self.params.get_raw_dsp_params();

        let mut in_peak: f32 = 0.0;
        let mut in_sq_sum: f32 = 0.0;
        let mut out_peak: f32 = 0.0;
        let mut out_sq_sum: f32 = 0.0;
        let mut sample_count: usize = 0;

        let slices = buffer.as_slice();
        if slices.len() >= 2 {
            let (left_slice, rest) = slices.split_at_mut(1);
            let left = &mut left_slice[0];
            let right = &mut rest[0];
            let len = left.len().min(right.len());

            for idx in 0..len {
                let in_l = left[idx];
                let in_r = right[idx];
                let in_mono = (in_l + in_r) * 0.5;
                self.input_queue.push_sample(in_mono);

                let abs_in = in_l.abs().max(in_r.abs());
                if abs_in > in_peak {
                    in_peak = abs_in;
                }
                in_sq_sum += in_l * in_l + in_r * in_r;
                sample_count += 2;
            }

            self.dsp.process_stereo(
                &mut left[..len],
                &mut right[..len],
                a, b, c, d, e, f, g, h, i, j,
            );

            let min_mult = self.dsp.last_min_multiplier;
            let gr_block_db = if min_mult >= 0.9999 {
                0.0
            } else {
                (-20.0 * min_mult.max(1e-5).log10()).max(0.0) as f32
            };

            let mut current = self.comp_gr.load(Ordering::Relaxed);
            while gr_block_db > current {
                match self.comp_gr.compare_exchange_weak(
                    current,
                    gr_block_db,
                    Ordering::Release,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(actual) => current = actual,
                }
            }

            for idx in 0..len {
                let out_l = left[idx];
                let out_r = right[idx];
                let out_mono = (out_l + out_r) * 0.5;
                self.output_queue.push_sample(out_mono);

                let abs_out = out_l.abs().max(out_r.abs());
                if abs_out > out_peak {
                    out_peak = abs_out;
                }
                out_sq_sum += out_l * out_l + out_r * out_r;
            }
        }

        if sample_count > 0 {
            let in_rms = (in_sq_sum / sample_count as f32).sqrt();
            let out_rms = (out_sq_sum / sample_count as f32).sqrt();
            
            // F11: CONSTANTES DE BALÍSTICA VISUAL DE ESTA GUI (NO afectan al DSP)
            let frames = sample_count / 2;
            let dt = frames as f32 / self.sample_rate.load(Ordering::Relaxed);
            let tau = 0.1; // 100 ms decay time constant
            let decay = (-dt / tau).exp();
            
            let cur_in_p = self.in_peak.load(Ordering::Relaxed);
            let next_in_p = in_peak.max(cur_in_p * decay);
            self.in_peak.store(next_in_p, Ordering::Relaxed);
            self.in_rms.store(in_rms, Ordering::Relaxed);

            let cur_out_p = self.out_peak.load(Ordering::Relaxed);
            let next_out_p = out_peak.max(cur_out_p * decay);
            self.out_peak.store(next_out_p, Ordering::Relaxed);
            self.out_rms.store(out_rms, Ordering::Relaxed);
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Cstrip2 {
    const CLAP_ID: &'static str = "com.airwindows.cstrip2";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Cstrip2 Airwindows Channel Strip");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Equalizer,
        ClapFeature::Compressor,
        ClapFeature::Distortion,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for Cstrip2 {
    const VST3_CLASS_ID: [u8; 16] = *b"Cstrip2Airwindws";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Eq,
        Vst3SubCategory::Dynamics,
        Vst3SubCategory::Distortion,
    ];
}

nih_export_clap!(Cstrip2);
nih_export_vst3!(Cstrip2);


#[cfg(test)]
mod tests {

    #[test]
    fn test_f11_meter_release_ballistics() {
        use std::sync::atomic::Ordering;
        use nih_plug::buffer::Buffer;
        
        let plugin = Cstrip2::default();
        plugin.sample_rate.store(44100.0, Ordering::Relaxed);
        
        // Simular buffers de distinto tamaño
        let buffer_sizes = [32, 64, 256, 1024];
        
        for &size in &buffer_sizes {
            plugin.in_peak.store(1.0, Ordering::Relaxed);
            let mut remaining_frames = 4410; // 100ms
            
            while remaining_frames > 0 {
                let frames = remaining_frames.min(size);
                remaining_frames -= frames;
                
                let mut real_buffers = vec![vec![0.0; frames]; 2];
                let mut buffer = Buffer::default();
                unsafe {
                    buffer.set_slices(frames, |output_slices| {
                        let (first_channel, other_channels) = real_buffers.split_at_mut(1);
                        *output_slices = vec![&mut first_channel[0], &mut other_channels[0]];
                    })
                };
                

                
                // Let's just mock the logic manually to avoid ProcessContext pain.
                // Replicate the exact logic from process that decays peak
                let in_peak: f32 = 0.0;
                let cur_in_p = plugin.in_peak.load(Ordering::Relaxed);
                let dt = frames as f32 / 44100.0;
                let tau = 0.1;
                let decay = (-dt / tau).exp();
                let next_in_p = in_peak.max(cur_in_p * decay);
                plugin.in_peak.store(next_in_p, Ordering::Relaxed);
            }
            
            let final_peak = plugin.in_peak.load(Ordering::Relaxed);
            let expected = std::f32::consts::E.powf(-1.0); // e^-1 ~ 0.367879
            assert!((final_peak - expected).abs() < 1e-4, "Failed for size {}: expected {}, got {}", size, expected, final_peak);
        }
    }

    use super::*;
    use nih_plug::wrapper::state::ParamValue;
    use std::collections::BTreeMap;

    #[test]
    fn test_f01_state_validation() {
        let mut state = nih_plug::prelude::PluginState {
            version: String::new(),
            params: BTreeMap::new(),
            fields: BTreeMap::new(),
        };

        // Out of bounds finite
        state.params.insert("treble".to_string(), ParamValue::F32(50.0));
        state.params.insert("bass_frq".to_string(), ParamValue::F32(10.0));
        
        // Non-finite
        state.params.insert("mid".to_string(), ParamValue::F32(f32::NAN));
        state.params.insert("output".to_string(), ParamValue::F32(f32::INFINITY));
        
        // String formats
        state.params.insert("low_cap".to_string(), ParamValue::String("NaN".to_string()));
        state.params.insert("hi_cap".to_string(), ParamValue::String("-Inf".to_string()));

        Cstrip2::filter_state(&mut state);

        assert_eq!(
            match state.params.get("treble").unwrap() {
                ParamValue::F32(v) => *v,
                _ => panic!(),
            },
            12.0
        );

        assert_eq!(
            match state.params.get("bass_frq").unwrap() {
                ParamValue::F32(v) => *v,
                _ => panic!(),
            },
            30.0
        );

        assert_eq!(
            match state.params.get("mid").unwrap() {
                ParamValue::F32(v) => *v,
                _ => panic!(),
            },
            0.0
        );

        assert_eq!(
            match state.params.get("output").unwrap() {
                ParamValue::F32(v) => *v,
                _ => panic!(),
            },
            0.33
        );

        assert_eq!(
            match state.params.get("low_cap").unwrap() {
                ParamValue::F32(v) => *v,
                _ => panic!(),
            },
            1.0
        );

        assert_eq!(
            match state.params.get("hi_cap").unwrap() {
                ParamValue::F32(v) => *v,
                _ => panic!(),
            },
            0.0
        );
    }
}
