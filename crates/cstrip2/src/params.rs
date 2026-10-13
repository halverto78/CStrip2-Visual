use nih_plug::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum UiScaleChoice {
    #[name = "75%"]
    Scale75,
    #[name = "100%"]
    Scale100,
    #[name = "125%"]
    Scale125,
    #[name = "150%"]
    Scale150,
}

impl UiScaleChoice {
    pub fn factor(&self) -> f32 {
        match self {
            Self::Scale75 => 0.75,
            Self::Scale100 => 1.0,
            Self::Scale125 => 1.25,
            Self::Scale150 => 1.5,
        }
    }
}

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AnalyzerResolution {
    #[name = "31 Bands"]
    Bands31,
    #[name = "63 Bands"]
    Bands63,
    #[name = "128 Points"]
    Points128,
    #[name = "Continuous"]
    Continuous,
}

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AnalyzerLineMode {
    #[name = "Curve"]
    Curve,
    #[name = "Filled"]
    Filled,
    #[name = "Bars"]
    Bars,
}

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AnalyzerSpeed {
    #[name = "Slow"]
    Slow,
    #[name = "Medium"]
    Medium,
    #[name = "Fast"]
    Fast,
    #[name = "Realtime"]
    Realtime,
}

impl AnalyzerSpeed {
    pub fn decay_rate(&self) -> f32 {
        match self {
            Self::Slow => 0.03,
            Self::Medium => 0.08,
            Self::Fast => 0.18,
            Self::Realtime => 0.95,
        }
    }
}

#[derive(Enum, PartialEq, Eq, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum AnalyzerSource {
    #[name = "Output"]
    Output,
    #[name = "Input"]
    Input,
    #[name = "In + Out"]
    InAndOut,
}

#[derive(Params)]
pub struct Cstrip2Params {
    // 10 Canonical DSP Parameters
    #[id = "treble"]
    pub treble: FloatParam,

    #[id = "mid"]
    pub mid: FloatParam,

    #[id = "bass"]
    pub bass: FloatParam,

    #[id = "treb_frq"]
    pub treb_frq: FloatParam,

    #[id = "bass_frq"]
    pub bass_frq: FloatParam,

    #[id = "low_cap"]
    pub low_cap: FloatParam,

    #[id = "hi_cap"]
    pub hi_cap: FloatParam,

    #[id = "compres"]
    pub compres: FloatParam,

    #[id = "comp_spd"]
    pub comp_spd: FloatParam,

    #[id = "output"]
    pub output: FloatParam,

    // UI & Analyzer Controls (Non-DSP, visual only)
    #[id = "ui_scale"]
    pub ui_scale: EnumParam<UiScaleChoice>,

    #[id = "analyzer_res"]
    pub analyzer_res: EnumParam<AnalyzerResolution>,

    #[id = "analyzer_line"]
    pub analyzer_line: EnumParam<AnalyzerLineMode>,

    #[id = "analyzer_speed"]
    pub analyzer_speed: EnumParam<AnalyzerSpeed>,

    #[id = "analyzer_hold"]
    pub analyzer_hold: BoolParam,

    #[id = "analyzer_source"]
    pub analyzer_source: EnumParam<AnalyzerSource>,

    #[id = "eq_visible"]
    pub eq_visible: BoolParam,

    #[persist = "cstrip2-editor-state-v1"]
    pub editor_state: Arc<nih_plug_egui::EguiState>,
}

impl Default for Cstrip2Params {
    fn default() -> Self {
        Self {
            treble: FloatParam::new(
                "Treble",
                0.0,
                FloatRange::Linear { min: -12.0, max: 12.0 },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            mid: FloatParam::new(
                "Mid",
                0.0,
                FloatRange::Linear { min: -12.0, max: 12.0 },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            bass: FloatParam::new(
                "Bass",
                0.0,
                FloatRange::Linear { min: -12.0, max: 12.0 },
            )
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            // D=0.5 -> (0.5^2 * 15) + 1 = 4.75 kHz
            treb_frq: FloatParam::new(
                "TrebFrq",
                4.75,
                FloatRange::Skewed { min: 1.0, max: 16.0, factor: 0.5 },
            )
            .with_unit(" kHz")
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            // E=0.5 -> (0.5^2 * 1570) + 30 = 422.5 Hz
            bass_frq: FloatParam::new(
                "BassFrq",
                422.5,
                FloatRange::Skewed { min: 30.0, max: 1600.0, factor: 0.5 },
            )
            .with_unit(" Hz")
            .with_value_to_string(formatters::v2s_f32_rounded(1)),

            low_cap: FloatParam::new(
                "LowCap",
                1.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            hi_cap: FloatParam::new(
                "HiCap",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            compres: FloatParam::new(
                "Compres",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            comp_spd: FloatParam::new(
                "CompSpd",
                0.0,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            output: FloatParam::new(
                "Output",
                0.33,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            ui_scale: EnumParam::new("UI Scale", UiScaleChoice::Scale100),
            analyzer_res: EnumParam::new("Resolution", AnalyzerResolution::Bands63),
            analyzer_line: EnumParam::new("Mode", AnalyzerLineMode::Curve),
            analyzer_speed: EnumParam::new("Speed", AnalyzerSpeed::Medium),
            analyzer_hold: BoolParam::new("Peak Hold", false),
            analyzer_source: EnumParam::new("Source", AnalyzerSource::Output),
            eq_visible: BoolParam::new("EQ Curve", true),
            editor_state: crate::gui::default_state(),
        }
    }
}

impl Cstrip2Params {
    /// Extracts normalized raw 0.0..1.0 values matching Airwindows A..J
    pub fn get_raw_dsp_params(&self) -> (f64, f64, f64, f64, f64, f64, f64, f64, f64, f64) {
        let a = ((self.treble.value() + 12.0) / 24.0) as f64;
        let b = ((self.mid.value() + 12.0) / 24.0) as f64;
        let c = ((self.bass.value() + 12.0) / 24.0) as f64;

        let treb_val = self.treb_frq.value() as f64;
        let d = ((treb_val - 1.0) / 15.0).max(0.0).sqrt();

        let bass_val = self.bass_frq.value() as f64;
        let e = ((bass_val - 30.0) / 1570.0).max(0.0).sqrt();

        let f = self.low_cap.value() as f64;
        let g = self.hi_cap.value() as f64;
        let h = self.compres.value() as f64;
        let i = self.comp_spd.value() as f64;
        let j = self.output.value() as f64;

        (a, b, c, d, e, f, g, h, i, j)
    }
}
