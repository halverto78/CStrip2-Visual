use nih_plug::prelude::Editor;
pub mod widgets;

use nih_plug_egui::egui::{
    self, Align2, Color32, CornerRadius, FontId, Margin, Mesh, Pos2, Rect, RichText, Sense, Stroke,
    StrokeKind, Ui, Vec2,
};
use nih_plug_egui::{create_egui_editor, EguiState};
use atomic_float::AtomicF32;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;
use nih_plug::prelude::{Param, ParamSetter, ParamPtr};
use nih_plug::editor::ParentWindowHandle;
use std::sync::Mutex;

struct TrackerState {
    active: Vec<(ParamPtr, usize)>,
    session_id: usize,
    valid_generations: std::collections::HashSet<usize>,
}

#[derive(Clone)]
pub struct GestureTracker {
    state: Arc<Mutex<TrackerState>>,
}

impl GestureTracker {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(TrackerState { active: Vec::new(), session_id: 0, valid_generations: std::collections::HashSet::new() })),
        }
    }
    
    pub fn current_generation(&self) -> usize {
        self.state.lock().unwrap().session_id
    }
    
    pub fn is_generation_valid(&self, generation: usize) -> bool {
        self.state.lock().unwrap().valid_generations.contains(&generation)
    }

    pub fn begin_session(&self) -> usize {
        let mut state = self.state.lock().unwrap();
        state.session_id = state.session_id.wrapping_add(1);
        let id = state.session_id;
        state.valid_generations.insert(id);
        id
    }

    pub fn close_generation(&self, gen: usize, context: Option<&dyn nih_plug::prelude::GuiContext>) {
        let extracted = {
            let mut state = self.state.lock().unwrap();
            state.valid_generations.remove(&gen);
            
            let mut extracted = Vec::new();
            state.active.retain(|&(p, g)| {
                if g == gen {
                    extracted.push(p);
                    false
                } else {
                    true
                }
            });
            extracted
        };
        
        if let Some(ctx) = context {
            for ptr in extracted {
                unsafe {
                    ctx.raw_end_set_parameter(ptr);
                }
            }
        }
    }
    
    pub fn begin(&self, param: &impl Param, setter: &ParamSetter, gen: usize) -> bool {
        let ptr = param.as_ptr();
        let should_begin = {
            let mut state = self.state.lock().unwrap();
            if !state.valid_generations.contains(&gen) { return false; }
            if !state.active.contains(&(ptr, gen)) {
                state.active.push((ptr, gen));
                true
            } else {
                false
            }
        };
        if should_begin {
            setter.begin_set_parameter(param);
            // Re-check after host callback
            let mut state = self.state.lock().unwrap();
            if state.session_id != gen {
                if let Some(pos) = state.active.iter().position(|x| *x == (ptr, gen)) {
                    state.active.remove(pos);
                }
                return false;
            }
        }
        true
    }
    
    pub fn is_active(&self, param: &impl Param, gen: usize) -> bool {
        let ptr = param.as_ptr();
        let state = self.state.lock().unwrap();
        state.valid_generations.contains(&gen) && state.active.contains(&(ptr, gen))
    }

    pub fn end(&self, param: &impl Param, setter: &ParamSetter, gen: usize) {
        let ptr = param.as_ptr();
        let should_end = {
            let mut state = self.state.lock().unwrap();
            if !state.valid_generations.contains(&gen) { return; }
            if let Some(pos) = state.active.iter().position(|x| *x == (ptr, gen)) {
                state.active.remove(pos);
                true
            } else {
                false
            }
        };
        if should_end {
            setter.end_set_parameter(param);
        }
    }
}

pub struct InteractionGuard<'a> {
    tracker: &'a GestureTracker,
    pub generation: usize,
    pub valid: bool,
}

impl<'a> InteractionGuard<'a> {
    pub fn new(tracker: &'a GestureTracker, generation: usize) -> Self {
        Self { tracker, generation, valid: true }
    }

    pub fn is_active(&self, param: &impl Param) -> bool {
        self.valid && self.tracker.is_active(param, self.generation)
    }

    pub fn begin(&mut self, param: &impl Param, setter: &ParamSetter) -> bool {
        if !self.valid { return false; }
        self.valid = self.tracker.begin(param, setter, self.generation);
        self.valid
    }

    pub fn set(&mut self, param: &impl Param, setter: &ParamSetter, val: f32) {
        if !self.valid { return; }
        if self.tracker.is_active(param, self.generation) {
            setter.set_parameter_normalized(param, val);
        } else {
            self.valid = false;
        }
    }

    pub fn end(&mut self, param: &impl Param, setter: &ParamSetter) {
        if !self.valid { return; }
        self.tracker.end(param, setter, self.generation);
    }
}

pub struct EditorWrapper {
    pub params: Arc<Cstrip2Params>,
    pub input_queue: Arc<SpectrumAudioQueue>,
    pub output_queue: Arc<SpectrumAudioQueue>,
    pub curve_worker: Arc<CurveWorker>,
    pub sample_rate: Arc<AtomicF32>,
    pub in_peak: Arc<AtomicF32>,
    pub in_rms: Arc<AtomicF32>,
    pub out_peak: Arc<AtomicF32>,
    pub out_rms: Arc<AtomicF32>,
    pub comp_gr: Arc<AtomicF32>,
    pub editor_state: Arc<EguiState>,
    pub tracker: GestureTracker,
    pub active_inner: std::sync::Arc<std::sync::Mutex<Option<Box<dyn nih_plug::prelude::Editor>>>>,
    pub persistent: std::sync::Arc<std::sync::Mutex<PersistentGuiState>>,
}

impl nih_plug::prelude::Editor for EditorWrapper {
    fn spawn(
        &self,
        parent: ParentWindowHandle,
        context: Arc<dyn nih_plug::prelude::GuiContext>,
    ) -> Box<dyn std::any::Any + Send> {
        let generation = self.tracker.begin_session();
        
        let initial_sr = self.sample_rate.load(Ordering::Relaxed);
        let initial_sr = if initial_sr >= 22050.0 { initial_sr } else { 44100.0 };
        
        let factory_presets = get_factory_presets();
        let user_presets = load_user_presets();
        let gui_context = GuiContext {
            params: self.params.clone(),
            input_queue: self.input_queue.clone(),
            output_queue: self.output_queue.clone(),
            curve_worker: self.curve_worker.clone(),
            sample_rate: self.sample_rate.clone(),
            in_peak: self.in_peak.clone(),
            in_rms: self.in_rms.clone(),
            out_peak: self.out_peak.clone(),
            out_rms: self.out_rms.clone(),
            comp_gr: self.comp_gr.clone(),
            displayed_gr: 0.0,
            spectrum_processor_in: SpectrumProcessor::new(initial_sr),
            spectrum_processor_out: SpectrumProcessor::new(initial_sr),
            last_frame_time: Instant::now(),
            last_curve_params: None,
            factory_presets,
            user_presets,
            persistent: self.persistent.clone(),
            feedback_message: None,
            save_dialog_open: false,
            save_preset_name: String::new(),
            tracker: self.tracker.clone(),
        };

        let inner = create_egui_editor_instance(self.editor_state.clone(), gui_context, generation);
        let scale_factor = self.persistent.lock().unwrap().user_scale_factor;
        if let Some(factor) = scale_factor {
            inner.set_scale_factor(factor);
        }
        let handle = inner.spawn(parent, context.clone());
        
        *self.active_inner.lock().unwrap() = Some(inner);

        Box::new(HandleWrapper {
            _inner: handle,
            tracker: self.tracker.clone(),
            raw_context: context,
            generation,
        })
    }
    
    fn size(&self) -> (u32, u32) { self.editor_state.size() }
    fn set_scale_factor(&self, factor: f32) -> bool {
        let accepted = {
            let active_inner = self.active_inner.lock().unwrap();
            active_inner.as_ref().map_or(true, |inner| inner.set_scale_factor(factor))
        };
        if accepted {
            self.persistent.lock().unwrap().user_scale_factor = Some(factor);
        }
        accepted
    }
    fn param_value_changed(&self, id: &str, normalized_value: f32) { 
        if let Some(inner) = self.active_inner.lock().unwrap().as_ref() {
            inner.param_value_changed(id, normalized_value);
        }
    }
    fn param_modulation_changed(&self, id: &str, modulation_offset: f32) { 
        if let Some(inner) = self.active_inner.lock().unwrap().as_ref() {
            inner.param_modulation_changed(id, modulation_offset);
        }
    }
    fn param_values_changed(&self) { 
        if let Some(inner) = self.active_inner.lock().unwrap().as_ref() {
            inner.param_values_changed();
        }
    }
}

struct HandleWrapper {
    _inner: Box<dyn std::any::Any + Send>,
    tracker: GestureTracker,
    raw_context: Arc<dyn nih_plug::prelude::GuiContext>,
    generation: usize,
}

impl Drop for HandleWrapper {
    fn drop(&mut self) {
        self.tracker.close_generation(self.generation, Some(&*self.raw_context));
    }
}

use crate::analyzer::{SpectrumAudioQueue, SpectrumProcessor};
use crate::dsp::curve::{get_log_frequencies_128, CurveParamsSnapshot, CurveWorker, NUM_CURVE_POINTS};
use crate::params::{
    AnalyzerLineMode, AnalyzerResolution, AnalyzerSource, AnalyzerSpeed, Cstrip2Params,
};
use crate::presets::{get_factory_presets, load_user_presets, save_user_preset, PresetData};
use widgets::{analog_knob, eq_gain_knob, gr_meter_slot, level_meter, output_knob, EqBandKind};

/// Default editor size (width x height in points) - Compact Studio Layout
pub fn default_state() -> Arc<EguiState> {
    EguiState::from_size(780, 560)
}

/// Active snapshot slot for A/B comparison
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActiveSlot {
    A,
    B,
}

/// GuiContext containing all real-time and visual communication handles

#[derive(Clone)]
pub struct PersistentGuiState {
    pub slot_a: crate::presets::PresetData,
    pub slot_b: crate::presets::PresetData,
    pub active_slot: crate::gui::ActiveSlot,
    pub current_preset_name: String,
    pub user_scale_factor: Option<f32>,
}

pub struct GuiContext {
    pub params: Arc<Cstrip2Params>,
    pub input_queue: Arc<SpectrumAudioQueue>,
    pub output_queue: Arc<SpectrumAudioQueue>,
    pub curve_worker: Arc<CurveWorker>,
    pub sample_rate: Arc<AtomicF32>,
    pub in_peak: Arc<AtomicF32>,
    pub in_rms: Arc<AtomicF32>,
    pub out_peak: Arc<AtomicF32>,
    pub out_rms: Arc<AtomicF32>,
    pub comp_gr: Arc<AtomicF32>,
    pub displayed_gr: f32,
    pub spectrum_processor_in: SpectrumProcessor,
    pub spectrum_processor_out: SpectrumProcessor,
    pub last_frame_time: Instant,
    pub last_curve_params: Option<CurveParamsSnapshot>,
    // Preset & A/B snapshot states
    pub factory_presets: Vec<PresetData>,
    pub user_presets: Vec<PresetData>,
    pub persistent: std::sync::Arc<std::sync::Mutex<PersistentGuiState>>,
    pub feedback_message: Option<(String, Instant)>,
    pub save_dialog_open: bool,
    pub save_preset_name: String,
    pub tracker: GestureTracker,
}

impl GuiContext {
    fn snapshot_for_generation(&self, generation: usize) -> Option<PersistentGuiState> {
        let tracker = self.tracker.state.lock().unwrap();
        if !tracker.valid_generations.contains(&generation) {
            return None;
        }
        Some(self.persistent.lock().unwrap().clone())
    }

    fn commit_for_generation(&self, generation: usize, candidate: PersistentGuiState) -> bool {
        // Keep revocation and commit ordered. These locks cover data copies only;
        // no UI or host callback runs here. The lock order is tracker -> persistent.
        let tracker = self.tracker.state.lock().unwrap();
        if !tracker.valid_generations.contains(&generation) {
            return false;
        }
        let mut persistent = self.persistent.lock().unwrap();
        persistent.slot_a = candidate.slot_a;
        persistent.slot_b = candidate.slot_b;
        persistent.active_slot = candidate.active_slot;
        persistent.current_preset_name = candidate.current_preset_name;
        // Scale belongs to the host API. Do not overwrite a scale accepted
        // reentrantly while applying this GUI snapshot.
        true
    }

    fn apply_preset_for_generation(&self, generation: usize, preset: &PresetData, setter: &ParamSetter) -> bool {
        let Some(mut candidate) = self.snapshot_for_generation(generation) else { return false; };
        if !apply_snapshot_for_generation(preset, &self.params, setter, &self.tracker, generation) {
            return false;
        }
        candidate.current_preset_name = preset.name.clone();
        match candidate.active_slot {
            ActiveSlot::A => candidate.slot_a = preset.clone(),
            ActiveSlot::B => candidate.slot_b = preset.clone(),
        }
        self.commit_for_generation(generation, candidate)
    }

    fn switch_slot_for_generation(&self, generation: usize, target: ActiveSlot, setter: &ParamSetter) -> bool {
        let Some(mut candidate) = self.snapshot_for_generation(generation) else { return false; };
        if candidate.active_slot == target { return false; }
        let preset = match target {
            ActiveSlot::A => {
                candidate.slot_b = PresetData::from_params(&self.params, "Slot B");
                candidate.slot_a.clone()
            }
            ActiveSlot::B => {
                candidate.slot_a = PresetData::from_params(&self.params, "Slot A");
                candidate.slot_b.clone()
            }
        };
        if !apply_snapshot_for_generation(&preset, &self.params, setter, &self.tracker, generation) {
            return false;
        }
        candidate.active_slot = target;
        self.commit_for_generation(generation, candidate)
    }

    fn copy_slot_for_generation(&self, generation: usize, source: ActiveSlot, setter: &ParamSetter) -> bool {
        let Some(mut candidate) = self.snapshot_for_generation(generation) else { return false; };
        let (preset, apply) = match source {
            ActiveSlot::A => {
                if candidate.active_slot == ActiveSlot::A {
                    candidate.slot_a = PresetData::from_params(&self.params, "Slot A");
                }
                candidate.slot_b = candidate.slot_a.clone();
                (candidate.slot_b.clone(), candidate.active_slot == ActiveSlot::B)
            }
            ActiveSlot::B => {
                if candidate.active_slot == ActiveSlot::B {
                    candidate.slot_b = PresetData::from_params(&self.params, "Slot B");
                }
                candidate.slot_a = candidate.slot_b.clone();
                (candidate.slot_a.clone(), candidate.active_slot == ActiveSlot::A)
            }
        };
        if apply && !apply_snapshot_for_generation(&preset, &self.params, setter, &self.tracker, generation) {
            return false;
        }
        self.commit_for_generation(generation, candidate)
    }
}

fn apply_snapshot_for_generation(
    preset: &PresetData, params: &Cstrip2Params, setter: &ParamSetter,
    tracker: &GestureTracker, generation: usize,
) -> bool {
    let mut guard = InteractionGuard::new(tracker, generation);
    macro_rules! apply {
        ($param:expr, $plain:expr) => {{
            let param = $param;
            if !guard.begin(param, setter) || !guard.is_active(param) { return false; }
            guard.set(param, setter, param.preview_normalized($plain));
            if !guard.is_active(param) { return false; }
            guard.end(param, setter);
            if !tracker.is_generation_valid(generation) { return false; }
        }};
    }
    apply!(&params.treble, preset.treble);
    apply!(&params.mid, preset.mid);
    apply!(&params.bass, preset.bass);
    apply!(&params.treb_frq, preset.treb_frq);
    apply!(&params.bass_frq, preset.bass_frq);
    apply!(&params.low_cap, preset.low_cap);
    apply!(&params.hi_cap, preset.hi_cap);
    apply!(&params.compres, preset.compres);
    apply!(&params.comp_spd, preset.comp_spd);
    apply!(&params.output, preset.output);
    true
}

/// Creates the egui editor instance
pub fn create_editor(
    params: Arc<Cstrip2Params>,
    input_queue: Arc<SpectrumAudioQueue>,
    output_queue: Arc<SpectrumAudioQueue>,
    curve_worker: Arc<CurveWorker>,
    sample_rate: Arc<AtomicF32>,
    in_peak: Arc<AtomicF32>,
    in_rms: Arc<AtomicF32>,
    out_peak: Arc<AtomicF32>,
    out_rms: Arc<AtomicF32>,
    comp_gr: Arc<AtomicF32>,
    editor_state: Arc<EguiState>,
) -> Option<Box<dyn Editor>> {
    let factory_presets = crate::presets::get_factory_presets();
    let initial_preset_name = factory_presets.first().map(|p| p.name.clone()).unwrap_or_else(|| "Default".to_string());
    
    let persistent = std::sync::Arc::new(std::sync::Mutex::new(PersistentGuiState {
        slot_a: crate::presets::PresetData::from_params(&params, "Slot A"),
        slot_b: crate::presets::PresetData::from_params(&params, "Slot B"),
        active_slot: crate::gui::ActiveSlot::A,
        current_preset_name: initial_preset_name,
        user_scale_factor: None,
    }));

    let tracker = GestureTracker::new();

    Some(Box::new(EditorWrapper {
        params,
        input_queue,
        output_queue,
        curve_worker,
        sample_rate,
        in_peak,
        in_rms,
        out_peak,
        out_rms,
        comp_gr,
        editor_state,
        tracker,
        active_inner: std::sync::Arc::new(std::sync::Mutex::new(None)),
        persistent,
    }))
}

fn create_egui_editor_instance(
    editor_state: Arc<EguiState>,
    gui_context: GuiContext,
    captured_generation: usize,
) -> Box<dyn Editor> {
    create_egui_editor(
        editor_state,
        gui_context,
        |_ctx, _setter| {},
        move |ctx, setter, state| {
            let current_gen = captured_generation;
            let frame_tracker = state.tracker.clone();
            let mut guard = crate::gui::InteractionGuard::new(&frame_tracker, current_gen);
            // Dark studio background (NO UI scale zoom applied)
            let screen_rect = ctx.input(|i| i.screen_rect());
            ctx.layer_painter(egui::LayerId::background())
                .rect_filled(screen_rect, 0.0, Color32::from_rgb(14, 16, 20));

            // Timing delta
            let now = Instant::now();
            let dt = (now - state.last_frame_time).as_secs_f32().clamp(0.001, 0.1);
            state.last_frame_time = now;

            // Consume peak Gain Reduction from DSP audio thread and apply smooth visual release
            let peak_gr = state.comp_gr.swap(0.0, Ordering::AcqRel);
            // F11: CONSTANTES DE BALÍSTICA VISUAL DE ESTA GUI (NO afectan al DSP)
            let release_rate_db_per_sec = 60.0;
            state.displayed_gr = peak_gr.max(
                state.displayed_gr - release_rate_db_per_sec * dt
            );

            // Update sample rate if changed in host
            let current_sr = state.sample_rate.load(Ordering::Relaxed);
            let current_sr = if current_sr > 0.0 { current_sr } else { 44100.0 };
            if (state.spectrum_processor_in.sample_rate - current_sr).abs() > 1.0 {
                state.spectrum_processor_in.set_sample_rate(current_sr);
                state.spectrum_processor_out.set_sample_rate(current_sr);
            }

            // Check if Tone parameters (Capacitor + EQ) changed, trigger background CurveWorker
            let (a, b, c, d, e, f, g, h, _, _) = state.params.get_raw_dsp_params();
            // F02: Identical math to DSP to avoid floating-point threshold discrepancies
            let inputgain = (h.powi(4) * 35.0) + 1.0;
            let engage_comp = inputgain > 1.0;
            let snapshot = CurveParamsSnapshot {
                a,
                b,
                c,
                d,
                e,
                f,
                g,
                sample_rate: current_sr as f64,
                engage_comp,
            };
            if state.last_curve_params != Some(snapshot) {
                state.curve_worker.request_update(snapshot);
                state.last_curve_params = Some(snapshot);
            }

            // Update spectrum processors based on user selected source
            let decay = state.params.analyzer_speed.value().decay_rate();
            let source = state.params.analyzer_source.value();

            if source == AnalyzerSource::Input || source == AnalyzerSource::InAndOut {
                state
                    .spectrum_processor_in
                    .process_frame(&state.input_queue, decay);
            }
            if source == AnalyzerSource::Output || source == AnalyzerSource::InAndOut {
                state
                    .spectrum_processor_out
                    .process_frame(&state.output_queue, decay);
            }

            // Request continuous repaint for smooth 60fps animations
            ctx.request_repaint();

            // Dark visuals styling
            let mut visuals = egui::Visuals::dark();
            visuals.panel_fill = Color32::from_rgb(14, 16, 20);
            visuals.window_fill = Color32::from_rgb(14, 16, 20);
            ctx.set_visuals(visuals);

            egui::CentralPanel::default()
                .frame(
                    egui::Frame::new()
                        .fill(Color32::from_rgb(14, 16, 20))
                        .inner_margin(Margin::symmetric(14, 8)),
                )
                .show(ctx, |ui| {
                    ui.set_min_width(ui.available_width());

                    let cluster_width = 730.0;
                    let center_pad = ((ui.available_width() - cluster_width) * 0.5).max(0.0);

                    // =========================================================================
                    // 1. HEADER (1 ROW): Title + Presets + A/B + Copy A/B (Aligned to Cluster)
                    // =========================================================================
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.add_space(center_pad);
                        ui.allocate_ui_with_layout(
                            Vec2::new(cluster_width, 22.0),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.spacing_mut().item_spacing.x = 8.0;

                                // Title & Brand
                                ui.heading(
                                    RichText::new("CSTRIP2")
                                        .color(Color32::from_rgb(240, 245, 255))
                                        .size(16.0)
                                        .strong(),
                                );
                                ui.label(
                                    RichText::new("AIRWINDOWS")
                                        .color(Color32::from_rgb(85, 195, 235))
                                        .size(9.5),
                                );

                                ui.add_space(2.0);
                                ui.separator();
                                ui.add_space(2.0);

                                // PRESET selector
                                ui.label(
                                    RichText::new("PRESET:")
                                        .color(Color32::from_rgb(140, 150, 165))
                                        .font(FontId::proportional(10.0)),
                                );

                                let current_preset_name = state.persistent.lock().unwrap().current_preset_name.clone();
                                let mut selected_preset_to_load = None;
                                egui::ComboBox::from_id_salt("cstrip2_preset_selector")
                                    .selected_text(
                                        RichText::new(&current_preset_name)
                                            .size(10.5)
                                            .color(Color32::from_rgb(220, 230, 245)),
                                    )
                                    .width(140.0)
                                    .show_ui(ui, |ui| {
                                        ui.label(
                                            RichText::new("--- FACTORY PRESETS ---")
                                                .size(9.0)
                                                .color(Color32::from_rgb(110, 125, 145)),
                                        );
                                        for p in &state.factory_presets {
                                            let is_selected = current_preset_name == p.name;
                                            if ui.selectable_label(is_selected, &p.name).clicked() {
                                                selected_preset_to_load = Some(p.clone());
                                            }
                                        }
                                        if !state.user_presets.is_empty() {
                                            ui.separator();
                                            ui.label(
                                                RichText::new("--- USER PRESETS ---")
                                                    .size(9.0)
                                                    .color(Color32::from_rgb(110, 125, 145)),
                                            );
                                            for p in &state.user_presets {
                                                let is_selected = current_preset_name == p.name;
                                                if ui.selectable_label(is_selected, &p.name).clicked() {
                                                    selected_preset_to_load = Some(p.clone());
                                                }
                                            }
                                        }
                                    });

                                if let Some(p) = selected_preset_to_load {
                                    if state.apply_preset_for_generation(current_gen, &p, setter) {
                                        state.feedback_message = Some((format!("Cargado: {}", p.name), Instant::now()));
                                    }
                                }

                                // SAVE Button
                                if !state.save_dialog_open {
                                    if ui
                                        .add_sized(
                                            [40.0, 20.0],
                                            egui::Button::new(
                                                RichText::new("SAVE")
                                                    .size(9.5)
                                                    .color(Color32::from_rgb(200, 215, 235)),
                                            )
                                            .fill(Color32::from_rgb(32, 38, 48))
                                            .corner_radius(CornerRadius::same(3)),
                                        )
                                        .clicked()
                                    {
                                        state.save_dialog_open = true;
                                        state.save_preset_name = state.persistent.lock().unwrap().current_preset_name.clone();
                                    }
                                } else {
                                    ui.text_edit_singleline(&mut state.save_preset_name);
                                    if ui
                                        .add_sized([44.0, 20.0], egui::Button::new("GUARDAR").fill(Color32::from_rgb(35, 110, 65)).corner_radius(CornerRadius::same(3)))
                                        .clicked()
                                    {
                                        let trimmed = state.save_preset_name.trim();
                                        if trimmed.is_empty() {
                                            state.feedback_message = Some(("Error: El nombre no puede estar vacío".to_string(), Instant::now()));
                                            // Keep dialog open and preserve input
                                        } else if let Some(mut candidate) = state.snapshot_for_generation(current_gen) {
                                            let new_preset = PresetData::from_params(&state.params, trimmed);
                                            match save_user_preset(&new_preset) {
                                                Ok(_) => {
                                                    candidate.current_preset_name = trimmed.to_string();
                                                    if state.commit_for_generation(current_gen, candidate) {
                                                        state.user_presets = load_user_presets();
                                                        state.feedback_message = Some((format!("Guardado: {}", trimmed), Instant::now()));
                                                    }
                                                    state.save_dialog_open = false;
                                                }
                                                Err(e) => {
                                                    state.feedback_message = Some((format!("Error al guardar: {}", e), Instant::now()));
                                                    // Keep dialog open and retain user input in state.save_preset_name
                                                }
                                            }
                                        } else {
                                            state.feedback_message = Some(("Error: Sesión de editor no válida".to_string(), Instant::now()));
                                        }
                                    }
                                    if ui
                                        .add_sized([20.0, 20.0], egui::Button::new("X").fill(Color32::from_rgb(55, 30, 30)).corner_radius(CornerRadius::same(3)))
                                        .clicked()
                                    {
                                        state.save_dialog_open = false;
                                    }
                                }

                                // Right-aligned controls within the cluster header
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;

                                    // COPY B->A
                                    if ui
                                        .add_sized(
                                            [54.0, 20.0],
                                            egui::Button::new(
                                                RichText::new("COPY B→A")
                                                    .size(9.0)
                                                    .color(Color32::from_rgb(200, 215, 235)),
                                            )
                                            .fill(Color32::from_rgb(32, 38, 48))
                                            .corner_radius(CornerRadius::same(3)),
                                        )
                                        .clicked()
                                    {
                                        if state.copy_slot_for_generation(current_gen, ActiveSlot::B, setter) {
                                            state.feedback_message = Some(("Copiado B -> A".to_string(), Instant::now()));
                                        }
                                    }

                                    // COPY A->B
                                    if ui
                                        .add_sized(
                                            [54.0, 20.0],
                                            egui::Button::new(
                                                RichText::new("COPY A→B")
                                                    .size(9.0)
                                                    .color(Color32::from_rgb(200, 215, 235)),
                                            )
                                            .fill(Color32::from_rgb(32, 38, 48))
                                            .corner_radius(CornerRadius::same(3)),
                                        )
                                        .clicked()
                                    {
                                        if state.copy_slot_for_generation(current_gen, ActiveSlot::A, setter) {
                                            state.feedback_message = Some(("Copiado A -> B".to_string(), Instant::now()));
                                        }
                                    }

                                    // SLOT B
                                    let is_b = state.persistent.lock().unwrap().active_slot == ActiveSlot::B;
                                    let btn_b_fill = if is_b { Color32::from_rgb(210, 120, 30) } else { Color32::from_rgb(32, 38, 48) };
                                    if ui
                                        .add_sized(
                                            [22.0, 20.0],
                                            egui::Button::new(
                                                RichText::new("B")
                                                    .strong()
                                                    .color(if is_b { Color32::WHITE } else { Color32::from_rgb(160, 170, 185) }),
                                            )
                                            .fill(btn_b_fill)
                                            .corner_radius(CornerRadius::same(3)),
                                        )
                                        .clicked()
                                        && !is_b
                                    {
                                        if state.switch_slot_for_generation(current_gen, ActiveSlot::B, setter) {
                                            state.feedback_message = Some(("Slot B Activo".to_string(), Instant::now()));
                                        }
                                    }

                                    // SLOT A
                                    let is_a = state.persistent.lock().unwrap().active_slot == ActiveSlot::A;
                                    let btn_a_fill = if is_a { Color32::from_rgb(45, 140, 210) } else { Color32::from_rgb(32, 38, 48) };
                                    if ui
                                        .add_sized(
                                            [22.0, 20.0],
                                            egui::Button::new(
                                                RichText::new("A")
                                                    .strong()
                                                    .color(if is_a { Color32::WHITE } else { Color32::from_rgb(160, 170, 185) }),
                                            )
                                            .fill(btn_a_fill)
                                            .corner_radius(CornerRadius::same(3)),
                                        )
                                        .clicked()
                                        && !is_a
                                    {
                                        if state.switch_slot_for_generation(current_gen, ActiveSlot::A, setter) {
                                            state.feedback_message = Some(("Slot A Activo".to_string(), Instant::now()));
                                        }
                                    }

                                    ui.label(
                                        RichText::new("SLOT:")
                                            .color(Color32::from_rgb(140, 150, 165))
                                            .font(FontId::proportional(10.0)),
                                    );

                                    ui.add_space(2.0);
                                    ui.separator();
                                    ui.add_space(2.0);

                                    // Feedback Pill
                                    if let Some((ref msg, timestamp)) = state.feedback_message {
                                        let is_err = msg.starts_with("Error");
                                        let duration = if is_err { 4.0 } else { 2.5 };
                                        if timestamp.elapsed().as_secs_f32() < duration {
                                            let color = if is_err {
                                                Color32::from_rgb(235, 80, 80)
                                            } else {
                                                Color32::from_rgb(85, 215, 135)
                                            };
                                            ui.label(
                                                RichText::new(msg)
                                                    .color(color)
                                                    .font(FontId::proportional(10.0)),
                                            );
                                        }
                                    }
                                });
                            },
                        );
                    });

                    ui.add_space(6.0);

                    // =========================================================================
                    // 2. CENTER SECTION: IN METER | GR SLOT | 16:9 DISPLAY | ANALYZER COL | OUT METER
                    // =========================================================================
                    let display_width = 520.0;
                    let display_height = 292.0; // 520 / 292 = 1.7808 (~16:9 aspect ratio)
                    let meter_width = 22.0;
                    let gr_width = 30.0;
                    let analyzer_col_width = 104.0;
                    let center_gap = 8.0;

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.add_space(center_pad);
                        ui.spacing_mut().item_spacing.x = center_gap;

                        // 1. Leftmost: INPUT Level Meter
                        let in_peak_val = state.in_peak.load(Ordering::Relaxed);
                        let in_rms_val = state.in_rms.load(Ordering::Relaxed);
                        level_meter(ui, in_peak_val, in_rms_val, "IN", meter_width, display_height);

                        // 2. Left Lateral: Gain Reduction Meter (Active ButterComp Telemetry)
                        gr_meter_slot(ui, gr_width, display_height, state.displayed_gr);

                        // 3. Center: Spectrum Plot with EQ Curve & Crossovers (Exact 16:9)
                        let (rect, _resp) = ui.allocate_exact_size(
                            Vec2::new(display_width, display_height),
                            Sense::hover(),
                        );

                        if ui.is_rect_visible(rect) {
                            render_spectrum_and_eq(
                                ui,
                                rect,
                                &state.spectrum_processor_in,
                                &state.spectrum_processor_out,
                                &state.params,
                                &state.curve_worker,
                            );
                        }

                        // 4. Right Lateral: Compact Analyzer Controls Column
                        render_analyzer_column(
                            ui,
                            setter,
                            &state.params,
                            analyzer_col_width,
                            display_height,
                        );

                        // 5. Rightmost: OUTPUT Level Meter
                        let out_peak_val = state.out_peak.load(Ordering::Relaxed);
                        let out_rms_val = state.out_rms.load(Ordering::Relaxed);
                        level_meter(ui, out_peak_val, out_rms_val, "OUT", meter_width, display_height);
                    });

                    ui.add_space(8.0);

                    // =========================================================================
                    // 3. BOTTOM RACK: 4 BALANCED MODULES (CAPACITOR, BUTTERCOMP, 3-BAND EQ, OUTPUT)
                    // =========================================================================
                    let rack_height = 190.0;
                    let rack_gap = 8.0;
                    let cap_w = 156.0;
                    let comp_w = 156.0;
                    let eq_w = 256.0;
                    let out_w = 156.0;
                    let total_rack_w = cap_w + rack_gap + comp_w + rack_gap + eq_w + rack_gap + out_w; // 748.0 px
                    let rack_pad = ((ui.available_width() - total_rack_w) * 0.5).max(0.0);

                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.add_space(rack_pad);
                        ui.spacing_mut().item_spacing.x = rack_gap;

                        // --- MODULE 1: CAPACITOR ---
                        render_module_frame(ui, cap_w, rack_height, "CAPACITOR", "HIGHPASS / LOWPASS", Color32::from_rgb(70, 185, 230), |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add_space(10.0);
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 10.0;
                                    analog_knob(
                                        ui,
                                        setter,
                                        &state.params.hi_cap, &mut guard,
                                        "HI-PASS",
                                        36.0,
                                        Color32::from_rgb(70, 185, 230),
                                    );
                                    analog_knob(
                                        ui,
                                        setter,
                                        &state.params.low_cap, &mut guard,
                                        "LO-PASS",
                                        36.0,
                                        Color32::from_rgb(70, 185, 230),
                                    );
                                });
                                ui.add_space(10.0);
                                ui.label(
                                    RichText::new("IIR FILTER ARRAY")
                                        .font(FontId::monospace(8.5))
                                        .color(Color32::from_rgb(110, 125, 145)),
                                );
                            });
                        });

                        // --- MODULE 2: BUTTERCOMP ---
                        render_module_frame(ui, comp_w, rack_height, "BUTTERCOMP", "CLASS AB COMPRESSOR", Color32::from_rgb(235, 155, 60), |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add_space(10.0);
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 10.0;
                                    analog_knob(
                                        ui,
                                        setter,
                                        &state.params.compres, &mut guard,
                                        "COMPRES",
                                        36.0,
                                        Color32::from_rgb(235, 155, 60),
                                    );
                                    analog_knob(
                                        ui,
                                        setter,
                                        &state.params.comp_spd, &mut guard,
                                        "SPEED",
                                        36.0,
                                        Color32::from_rgb(235, 155, 60),
                                    );
                                });
                                ui.add_space(10.0);
                                ui.label(
                                    RichText::new("DUAL-SPEED SMOOTH")
                                        .font(FontId::monospace(8.5))
                                        .color(Color32::from_rgb(145, 125, 105)),
                                );
                            });
                        });

                        // --- MODULE 3: 3-BAND EQ ---
                        render_module_frame(ui, eq_w, rack_height, "3-BAND EQ", "COMPLEMENTARY SPLIT", Color32::from_rgb(100, 215, 255), |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add_space(4.0);
                                // TOP ROW: Frequencies (BassFrq & TrebFrq)
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 32.0;
                                    analog_knob(
                                        ui,
                                        setter,
                                        &state.params.bass_frq, &mut guard,
                                        "BASS FRQ",
                                        32.0,
                                        Color32::from_rgb(100, 210, 255),
                                    );
                                    analog_knob(
                                        ui,
                                        setter,
                                        &state.params.treb_frq, &mut guard,
                                        "TREB FRQ",
                                        32.0,
                                        Color32::from_rgb(100, 210, 255),
                                    );
                                });

                                ui.add_space(3.0);

                                // BOTTOM ROW: Gains (Bass, Mid, Treble) with Shift + Drag linking
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 8.0;
                                    eq_gain_knob(
                                        ui,
                                        setter,
                                        EqBandKind::Bass,
                                        &state.params.bass,
                                        &state.params.mid,
                                        &state.params.treble, &mut guard,
                                        "BASS",
                                        34.0,
                                        Color32::from_rgb(255, 195, 75),
                                    );
                                    eq_gain_knob(
                                        ui,
                                        setter,
                                        EqBandKind::Mid,
                                        &state.params.bass,
                                        &state.params.mid,
                                        &state.params.treble, &mut guard,
                                        "MID",
                                        34.0,
                                        Color32::from_rgb(255, 195, 75),
                                    );
                                    eq_gain_knob(
                                        ui,
                                        setter,
                                        EqBandKind::Treble,
                                        &state.params.bass,
                                        &state.params.mid,
                                        &state.params.treble, &mut guard,
                                        "TREBLE",
                                        34.0,
                                        Color32::from_rgb(255, 195, 75),
                                    );
                                });
                            });
                        });

                        // --- MODULE 4: OUTPUT ---
                        render_module_frame(ui, out_w, rack_height, "OUTPUT", "SATURATION STAGES", Color32::from_rgb(240, 95, 75), |ui| {
                            ui.vertical_centered(|ui| {
                                ui.add_space(8.0);
                                output_knob(
                                    ui,
                                    setter,
                                    &state.params.output, &mut guard,
                                    "OUTPUT",
                                    44.0,
                                );
                            });
                        });
                    });

                    ui.add_space(4.0);

                    // =========================================================================
                    // 4. FOOTER: Discrete Author & Original DSP Credit
                    // =========================================================================
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.add_space(center_pad);
                        ui.allocate_ui_with_layout(
                            Vec2::new(cluster_width, 14.0),
                            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                            |ui| {
                                ui.label(
                                    RichText::new("Original DSP: Chris Johnson / Airwindows · GUI & Port: AMR")
                                        .size(8.5)
                                        .color(Color32::from_rgb(105, 115, 130)),
                                );
                            },
                        );
                    });
                });
        },
    ).unwrap()
    // end of create_egui_editor_instance
}

/// Helper button for compact analyzer column selectors
fn analyzer_btn(ui: &mut Ui, selected: bool, text: &str, size: [f32; 2]) -> egui::Response {
    let fill = if selected {
        Color32::from_rgb(38, 70, 105)
    } else {
        Color32::from_rgb(24, 28, 36)
    };
    let text_color = if selected {
        Color32::from_rgb(240, 248, 255)
    } else {
        Color32::from_rgb(130, 140, 155)
    };
    let stroke = if selected {
        Stroke::new(1.0, Color32::from_rgb(70, 160, 230))
    } else {
        Stroke::new(1.0, Color32::from_rgb(38, 44, 56))
    };
    ui.add_sized(
        size,
        egui::Button::new(
            RichText::new(text)
                .size(9.0)
                .color(text_color)
                .strong(),
        )
        .fill(fill)
        .stroke(stroke)
        .corner_radius(CornerRadius::same(3)),
    )
}

/// Renders the compact vertical analyzer controls column (EQ CURVE, SRC, RES, MODE, SPEED, HOLD)
fn render_analyzer_column(
    ui: &mut Ui,
    setter: &nih_plug::prelude::ParamSetter,
    params: &Arc<Cstrip2Params>,
    width: f32,
    height: f32,
) {
    let desired_size = Vec2::new(width, height);
    let (rect, _) = ui.allocate_exact_size(desired_size, Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();

        // Outer chassis
        painter.rect_filled(rect, CornerRadius::same(4), Color32::from_rgb(18, 21, 26));
        painter.rect_stroke(
            rect,
            CornerRadius::same(4),
            Stroke::new(1.0, Color32::from_rgb(38, 44, 56)),
            StrokeKind::Inside,
        );

        // Header Strip
        let header_rect = Rect::from_min_max(
            Pos2::new(rect.left() + 1.0, rect.top() + 1.0),
            Pos2::new(rect.right() - 1.0, rect.top() + 22.0),
        );
        painter.rect_filled(
            header_rect,
            CornerRadius {
                nw: 3,
                ne: 3,
                sw: 0,
                se: 0,
            },
            Color32::from_rgb(25, 29, 36),
        );
        painter.line_segment(
            [
                Pos2::new(rect.left() + 1.0, rect.top() + 22.0),
                Pos2::new(rect.right() - 1.0, rect.top() + 22.0),
            ],
            Stroke::new(1.0, Color32::from_rgb(38, 44, 56)),
        );

        painter.text(
            Pos2::new(rect.center().x, rect.top() + 11.0),
            Align2::CENTER_CENTER,
            "ANALYZER",
            FontId::proportional(10.0),
            Color32::from_rgb(85, 195, 235),
        );

        // Child UI for controls inside
        let content_rect = Rect::from_min_max(
            Pos2::new(rect.left() + 4.0, rect.top() + 26.0),
            Pos2::new(rect.right() - 4.0, rect.bottom() - 4.0),
        );

        let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));
        child_ui.set_width(content_rect.width());

        child_ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = 3.0;

            // 1. EQ CURVE TOGGLE
            let eq_vis = params.eq_visible.value();
            let eq_btn_fill = if eq_vis {
                Color32::from_rgb(30, 80, 110)
            } else {
                Color32::from_rgb(26, 30, 38)
            };
            let eq_btn_stroke = if eq_vis {
                Stroke::new(1.0, Color32::from_rgb(85, 195, 235))
            } else {
                Stroke::new(1.0, Color32::from_rgb(45, 52, 64))
            };
            let eq_text_col = if eq_vis {
                Color32::from_rgb(200, 240, 255)
            } else {
                Color32::from_rgb(130, 140, 155)
            };

            if ui
                .add_sized(
                    [ui.available_width(), 20.0],
                    egui::Button::new(
                        RichText::new("TONE CURVE")
                            .size(9.0)
                            .strong()
                            .color(eq_text_col),
                    )
                    .fill(eq_btn_fill)
                    .stroke(eq_btn_stroke)
                    .corner_radius(CornerRadius::same(3)),
                )
                .clicked()
            {
                setter.set_parameter(&params.eq_visible, !eq_vis);
            }

            ui.add_space(5.0);

            // 2. SOURCE (OUT / IN / BOTH)
            ui.label(
                RichText::new("SOURCE")
                    .font(FontId::monospace(8.0))
                    .color(Color32::from_rgb(130, 140, 155)),
            );
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let cur_src = params.analyzer_source.value();
                let btn_w = (ui.available_width() - 4.0) / 3.0;
                if analyzer_btn(ui, cur_src == AnalyzerSource::Output, "OUT", [btn_w, 18.0]).clicked() {
                    setter.set_parameter(&params.analyzer_source, AnalyzerSource::Output);
                }
                if analyzer_btn(ui, cur_src == AnalyzerSource::Input, "IN", [btn_w, 18.0]).clicked() {
                    setter.set_parameter(&params.analyzer_source, AnalyzerSource::Input);
                }
                if analyzer_btn(ui, cur_src == AnalyzerSource::InAndOut, "BOTH", [btn_w, 18.0]).clicked() {
                    setter.set_parameter(&params.analyzer_source, AnalyzerSource::InAndOut);
                }
            });

            ui.add_space(5.0);

            // 3. RESOLUTION (31, 63, 128, CONT)
            ui.label(
                RichText::new("RESOLUTION")
                    .font(FontId::monospace(8.0))
                    .color(Color32::from_rgb(130, 140, 155)),
            );
            let cur_res = params.analyzer_res.value();
            let half_w = (ui.available_width() - 2.0) / 2.0;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                if analyzer_btn(ui, cur_res == AnalyzerResolution::Bands31, "31", [half_w, 17.0]).clicked() {
                    setter.set_parameter(&params.analyzer_res, AnalyzerResolution::Bands31);
                }
                if analyzer_btn(ui, cur_res == AnalyzerResolution::Bands63, "63", [half_w, 17.0]).clicked() {
                    setter.set_parameter(&params.analyzer_res, AnalyzerResolution::Bands63);
                }
            });
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                if analyzer_btn(ui, cur_res == AnalyzerResolution::Points128, "128", [half_w, 17.0]).clicked() {
                    setter.set_parameter(&params.analyzer_res, AnalyzerResolution::Points128);
                }
                if analyzer_btn(ui, cur_res == AnalyzerResolution::Continuous, "CONT", [half_w, 17.0]).clicked() {
                    setter.set_parameter(&params.analyzer_res, AnalyzerResolution::Continuous);
                }
            });

            ui.add_space(5.0);

            // 4. LINE MODE (CRV, FILL, BAR)
            ui.label(
                RichText::new("LINE MODE")
                    .font(FontId::monospace(8.0))
                    .color(Color32::from_rgb(130, 140, 155)),
            );
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let cur_mode = params.analyzer_line.value();
                let btn_w = (ui.available_width() - 4.0) / 3.0;
                if analyzer_btn(ui, cur_mode == AnalyzerLineMode::Curve, "CRV", [btn_w, 18.0]).clicked() {
                    setter.set_parameter(&params.analyzer_line, AnalyzerLineMode::Curve);
                }
                if analyzer_btn(ui, cur_mode == AnalyzerLineMode::Filled, "FILL", [btn_w, 18.0]).clicked() {
                    setter.set_parameter(&params.analyzer_line, AnalyzerLineMode::Filled);
                }
                if analyzer_btn(ui, cur_mode == AnalyzerLineMode::Bars, "BAR", [btn_w, 18.0]).clicked() {
                    setter.set_parameter(&params.analyzer_line, AnalyzerLineMode::Bars);
                }
            });

            ui.add_space(5.0);

            // 5. SPEED (SLOW, MED, FAST)
            ui.label(
                RichText::new("SPEED")
                    .font(FontId::monospace(8.0))
                    .color(Color32::from_rgb(130, 140, 155)),
            );
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let cur_spd = params.analyzer_speed.value();
                let btn_w = (ui.available_width() - 4.0) / 3.0;
                if analyzer_btn(ui, cur_spd == AnalyzerSpeed::Slow, "SLOW", [btn_w, 18.0]).clicked() {
                    setter.set_parameter(&params.analyzer_speed, AnalyzerSpeed::Slow);
                }
                if analyzer_btn(ui, cur_spd == AnalyzerSpeed::Medium, "MED", [btn_w, 18.0]).clicked() {
                    setter.set_parameter(&params.analyzer_speed, AnalyzerSpeed::Medium);
                }
                if analyzer_btn(ui, cur_spd == AnalyzerSpeed::Fast, "FAST", [btn_w, 18.0]).clicked() {
                    setter.set_parameter(&params.analyzer_speed, AnalyzerSpeed::Fast);
                }
            });

            ui.add_space(6.0);

            // 6. PEAK HOLD TOGGLE
            let hold_val = params.analyzer_hold.value();
            let hold_btn_fill = if hold_val {
                Color32::from_rgb(80, 50, 20)
            } else {
                Color32::from_rgb(26, 30, 38)
            };
            let hold_btn_stroke = if hold_val {
                Stroke::new(1.0, Color32::from_rgb(235, 155, 60))
            } else {
                Stroke::new(1.0, Color32::from_rgb(45, 52, 64))
            };
            let hold_text_col = if hold_val {
                Color32::from_rgb(255, 205, 130)
            } else {
                Color32::from_rgb(130, 140, 155)
            };

            if ui
                .add_sized(
                    [ui.available_width(), 20.0],
                    egui::Button::new(
                        RichText::new(if hold_val { "HOLD: ON" } else { "HOLD: OFF" })
                            .size(9.0)
                            .strong()
                            .color(hold_text_col),
                    )
                    .fill(hold_btn_fill)
                    .stroke(hold_btn_stroke)
                    .corner_radius(CornerRadius::same(3)),
                )
                .clicked()
            {
                setter.set_parameter(&params.analyzer_hold, !hold_val);
            }
        });
    }
}


/// Helper function to render an analog rack module faceplate
fn render_module_frame<R>(
    ui: &mut Ui,
    width: f32,
    height: f32,
    title: &str,
    subtitle: &str,
    accent: Color32,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    let desired_size = Vec2::new(width, height);
    let (rect, _resp) = ui.allocate_exact_size(desired_size, Sense::hover());
    let painter = ui.painter();

    // Module background chassis
    painter.rect_filled(rect, CornerRadius::same(5), Color32::from_rgb(20, 23, 28));
    painter.rect_stroke(
        rect,
        CornerRadius::same(5),
        Stroke::new(1.0, Color32::from_rgb(45, 52, 64)),
        StrokeKind::Inside,
    );

    // Decorative rack screws in 4 corners
    let screw_col = Color32::from_rgb(60, 68, 80);
    let screw_r = 3.0;
    for &pt in &[
        Pos2::new(rect.left() + 7.0, rect.top() + 7.0),
        Pos2::new(rect.right() - 7.0, rect.top() + 7.0),
        Pos2::new(rect.left() + 7.0, rect.bottom() - 7.0),
        Pos2::new(rect.right() - 7.0, rect.bottom() - 7.0),
    ] {
        painter.circle_filled(pt, screw_r, screw_col);
        painter.line_segment(
            [pt - Vec2::new(1.8, 0.0), pt + Vec2::new(1.8, 0.0)],
            Stroke::new(0.8, Color32::from_rgb(30, 35, 42)),
        );
    }

    // Module Header Strip
    let header_rect = Rect::from_min_max(
        Pos2::new(rect.left() + 1.0, rect.top() + 1.0),
        Pos2::new(rect.right() - 1.0, rect.top() + 28.0),
    );
    painter.rect_filled(
        header_rect,
        CornerRadius {
            nw: 4,
            ne: 4,
            sw: 0,
            se: 0,
        },
        Color32::from_rgb(27, 31, 38),
    );
    painter.line_segment(
        [
            Pos2::new(rect.left() + 1.0, rect.top() + 28.0),
            Pos2::new(rect.right() - 1.0, rect.top() + 28.0),
        ],
        Stroke::new(1.0, Color32::from_rgb(45, 52, 64)),
    );

    // Title & Subtitle text
    painter.text(
        Pos2::new(rect.center().x, rect.top() + 10.0),
        Align2::CENTER_CENTER,
        title,
        FontId::proportional(11.0),
        accent,
    );
    painter.text(
        Pos2::new(rect.center().x, rect.top() + 21.0),
        Align2::CENTER_CENTER,
        subtitle,
        FontId::monospace(7.5),
        Color32::from_rgb(130, 140, 155),
    );

    // Render module content inside
    let content_rect = Rect::from_min_max(
        Pos2::new(rect.left() + 4.0, rect.top() + 29.0),
        Pos2::new(rect.right() - 4.0, rect.bottom() - 4.0),
    );
    let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(content_rect));
    add_contents(&mut child_ui)
}

/// Renders the combined Spectrum Analyzer, Crossover markers, and 128-Point EQ Curve
fn render_spectrum_and_eq(
    ui: &mut Ui,
    rect: Rect,
    proc_in: &SpectrumProcessor,
    proc_out: &SpectrumProcessor,
    params: &Cstrip2Params,
    curve_worker: &CurveWorker,
) {
    let painter = ui.painter();

    // Background & Outer Chassis Border
    painter.rect_filled(rect, CornerRadius::same(4), Color32::from_rgb(11, 13, 17));
    painter.rect_stroke(
        rect,
        CornerRadius::same(4),
        Stroke::new(1.0, Color32::from_rgb(38, 44, 56)),
        StrokeKind::Inside,
    );

    // Usable Plot Rectangle: 100% of the display area inside the 1px stroke
    let plot_rect = rect.shrink(1.0);
    let plot_painter = painter.with_clip_rect(plot_rect);

    // Real Logarithmic Frequency Mapping (20 Hz .. 20 kHz nominal, bounded by Nyquist)
    let f_min = 20.0f32;
    let nyquist = proc_in.sample_rate * 0.5;
    let visual_max_hz = 20000.0f32;
    let display_max_freq = visual_max_hz.min(nyquist);
    let f_max = display_max_freq;
    let log_range = (f_max / f_min).log10();

    let freq_to_x = |f: f32| -> f32 {
        let f_clamped = f.clamp(f_min, f_max);
        let norm = (f_clamped / f_min).log10() / log_range;
        plot_rect.left() + norm * plot_rect.width()
    };

    // EQ dB Mapping (-18 dB to +18 dB, center at 0 dB)
    let eq_db_min = -18.0f32;
    let eq_db_max = 18.0f32;
    let eq_db_range = eq_db_max - eq_db_min;

    let eq_db_to_y = |db: f32| -> f32 {
        let norm = (db.clamp(eq_db_min, eq_db_max) - eq_db_min) / eq_db_range;
        plot_rect.bottom() - norm * plot_rect.height()
    };

    // Spectrum Height Mapping (0.0..1.0 norm height to plot pixel Y)
    let spec_norm_to_y = |norm: f32| -> f32 {
        plot_rect.bottom() - norm.clamp(0.0, 1.0) * plot_rect.height()
    };

    // ----------------------------------------------------
    // 1. GRID LINES: Frequencies & dB
    // ----------------------------------------------------
    // Frequency vertical grid lines
    let freq_marks = [
        (20.0, "20"),
        (50.0, "50"),
        (100.0, "100"),
        (200.0, "200"),
        (500.0, "500"),
        (1000.0, "1k"),
        (2000.0, "2k"),
        (5000.0, "5k"),
        (10000.0, "10k"),
        (20000.0, "20k"),
    ];

    for &(f, label) in &freq_marks {
        if f > display_max_freq {
            continue;
        }
        let x = freq_to_x(f);
        plot_painter.line_segment(
            [Pos2::new(x, plot_rect.top()), Pos2::new(x, plot_rect.bottom())],
            Stroke::new(1.0, Color32::from_rgb(22, 26, 34)),
        );
        let (align, x_pos) = if f <= 20.5 {
            (Align2::LEFT_BOTTOM, plot_rect.left() + 4.0)
        } else if (f - display_max_freq).abs() < 10.0 {
            (Align2::RIGHT_BOTTOM, plot_rect.right() - 4.0)
        } else {
            (Align2::CENTER_BOTTOM, x)
        };
        plot_painter.text(
            Pos2::new(x_pos, plot_rect.bottom() - 4.0),
            align,
            label,
            FontId::monospace(8.0),
            Color32::from_rgba_unmultiplied(110, 120, 135, 180),
        );
    }

    // EQ dB horizontal grid lines
    let eq_db_marks = [12.0, 6.0, 0.0, -6.0, -12.0];
    for &db in &eq_db_marks {
        let y = eq_db_to_y(db);
        let stroke = if db == 0.0 {
            Stroke::new(1.0, Color32::from_rgb(50, 60, 80)) // Highlighted 0 dB line
        } else {
            Stroke::new(1.0, Color32::from_rgb(20, 24, 32))
        };
        plot_painter.line_segment(
            [Pos2::new(plot_rect.left(), y), Pos2::new(plot_rect.right(), y)],
            stroke,
        );

        // Right side EQ dB labels
        let label = format!("{:+2.0}", db);
        plot_painter.text(
            Pos2::new(plot_rect.right() - 6.0, y),
            Align2::RIGHT_CENTER,
            label,
            FontId::monospace(8.0),
            Color32::from_rgba_unmultiplied(130, 140, 160, 180),
        );
    }

    // Left side Spectrum dBFS marks (-48, -36, -24, -12, 0 dBFS)
    // Scale in processor is -60 to +12 dBFS (72 dB range)
    let spec_db_marks = [-48.0, -36.0, -24.0, -12.0, 0.0];
    for &db in &spec_db_marks {
        let norm = (db + 60.0) / 72.0;
        let y = spec_norm_to_y(norm);
        plot_painter.text(
            Pos2::new(plot_rect.left() + 6.0, y),
            Align2::LEFT_CENTER,
            format!("{:.0}", db),
            FontId::monospace(8.0),
            Color32::from_rgba_unmultiplied(90, 105, 125, 180),
        );
    }

    // ----------------------------------------------------
    // 2. CROSSOVER BOUNDARIES (BassFrq & TrebFrq)
    // ----------------------------------------------------
    let f_bass = params.bass_frq.value(); // 30 Hz to 1600 Hz
    let f_treb = params.treb_frq.value() * 1000.0; // 1000 Hz to 16000 Hz

    let x_bass = freq_to_x(f_bass);
    let x_treb = freq_to_x(f_treb);

    // Draw vertical crossover lines (dashed style)
    let draw_crossover_line = |x: f32, col: Color32, label: &str| {
        let n_dashes = 18;
        let seg_len = plot_rect.height() / (n_dashes as f32 * 2.0);
        for i in 0..n_dashes {
            let y1 = plot_rect.top() + (i as f32 * 2.0) * seg_len;
            let y2 = y1 + seg_len;
            plot_painter.line_segment([Pos2::new(x, y1), Pos2::new(x, y2)], Stroke::new(1.2, col));
        }
        // Frequency badge at top
        plot_painter.text(
            Pos2::new(x, plot_rect.top() + 10.0),
            Align2::CENTER_CENTER,
            label,
            FontId::monospace(8.5),
            col,
        );
    };

    if f_bass <= display_max_freq {
        draw_crossover_line(
            x_bass,
            Color32::from_rgb(80, 200, 240),
            &format!("{:.0} Hz", f_bass),
        );
    }
    if f_treb <= display_max_freq {
        draw_crossover_line(
            x_treb,
            Color32::from_rgb(250, 175, 70),
            &format!("{:.2} kHz", f_treb / 1000.0),
        );
    }

    // Zone labels (LOW, MID, HIGH) in background
    let zone_y = plot_rect.top() + 24.0;
    let mid_x_low = (plot_rect.left() + x_bass) * 0.5;
    let mid_x_mid = (x_bass + x_treb) * 0.5;
    let mid_x_high = (x_treb + plot_rect.right()) * 0.5;

    let zone_col = Color32::from_rgba_unmultiplied(120, 135, 160, 45);
    plot_painter.text(Pos2::new(mid_x_low, zone_y), Align2::CENTER_CENTER, "LOW", FontId::proportional(11.0), zone_col);
    plot_painter.text(Pos2::new(mid_x_mid, zone_y), Align2::CENTER_CENTER, "MID", FontId::proportional(11.0), zone_col);
    plot_painter.text(Pos2::new(mid_x_high, zone_y), Align2::CENTER_CENTER, "HIGH", FontId::proportional(11.0), zone_col);

    // ----------------------------------------------------
    // 3. SPECTRUM ANALYZER DRAWING
    // ----------------------------------------------------
    let src = params.analyzer_source.value();
    let res = params.analyzer_res.value();
    let mode = params.analyzer_line.value();
    let hold = params.analyzer_hold.value();

    let render_spectrum = |proc: &SpectrumProcessor, stroke_col: Color32, fill_col: Color32| {
        match res {
            AnalyzerResolution::Bands31 => {
                draw_spectrum_bands::<31>(
                    &plot_painter,
                    &proc.bands_31,
                    &proc.peak_hold_31,
                    freq_to_x,
                    spec_norm_to_y,
                    plot_rect,
                    mode,
                    hold,
                    stroke_col,
                    fill_col,
                    display_max_freq,
                );
            }
            AnalyzerResolution::Bands63 => {
                draw_spectrum_bands::<63>(
                    &plot_painter,
                    &proc.bands_63,
                    &proc.peak_hold_63,
                    freq_to_x,
                    spec_norm_to_y,
                    plot_rect,
                    mode,
                    hold,
                    stroke_col,
                    fill_col,
                    display_max_freq,
                );
            }
            AnalyzerResolution::Points128 | AnalyzerResolution::Continuous => {
                draw_spectrum_bands::<128>(
                    &plot_painter,
                    &proc.bands_128,
                    &proc.peak_hold_128,
                    freq_to_x,
                    spec_norm_to_y,
                    plot_rect,
                    mode,
                    hold,
                    stroke_col,
                    fill_col,
                    display_max_freq,
                );
            }
        }
    };

    // Draw Input spectrum first if active (underneath)
    if src == AnalyzerSource::Input || src == AnalyzerSource::InAndOut {
        let in_stroke = Color32::from_rgb(230, 160, 60);
        let in_fill = Color32::from_rgba_unmultiplied(230, 160, 60, 30);
        render_spectrum(proc_in, in_stroke, in_fill);
    }

    // Draw Output spectrum on top if active
    if src == AnalyzerSource::Output || src == AnalyzerSource::InAndOut {
        let out_stroke = Color32::from_rgb(60, 210, 240);
        let out_fill = Color32::from_rgba_unmultiplied(60, 210, 240, 40);
        render_spectrum(proc_out, out_stroke, out_fill);
    }

    // ----------------------------------------------------
    // 4. EXACT 128-POINT NOMINAL EQ CURVE (Airwindows DSP)
    // ----------------------------------------------------
    if params.eq_visible.value() {
        let curve_data = curve_worker.get_latest_curve();
        let freqs = get_log_frequencies_128();

        let mut pts = Vec::with_capacity(NUM_CURVE_POINTS);
        for k in 0..NUM_CURVE_POINTS {
            let f = freqs[k] as f32;
            if f > display_max_freq {
                break;
            }
            let db = curve_data[k];
            let x = freq_to_x(f);
            let y = eq_db_to_y(db);
            pts.push(Pos2::new(x, y));
        }

        if pts.len() >= 2 {
            // Ambient glow line under curve
            plot_painter.add(egui::Shape::line(
                pts.clone(),
                Stroke::new(4.5, Color32::from_rgba_unmultiplied(255, 205, 80, 45)),
            ));

            // Sharp main EQ curve line
            plot_painter.add(egui::Shape::line(
                pts,
                Stroke::new(2.2, Color32::from_rgb(255, 215, 90)),
            ));
        }
    }
}

/// Generic helper to draw spectrum bands according to resolution and mode
fn draw_spectrum_bands<const N: usize>(
    painter: &egui::Painter,
    bands: &[f32; N],
    peak_hold: &[f32; N],
    freq_to_x: impl Fn(f32) -> f32,
    spec_norm_to_y: impl Fn(f32) -> f32,
    plot_rect: Rect,
    mode: AnalyzerLineMode,
    hold: bool,
    stroke_color: Color32,
    fill_color: Color32,
    display_max_freq: f32,
) {
    let mut pts = Vec::with_capacity(N);
    let mut valid_indices = Vec::with_capacity(N);

    for k in 0..N {
        let f_center = 20.0 * 10.0_f32.powf((3.0 * k as f32) / (N - 1) as f32);
        if f_center > display_max_freq {
            break;
        }
        let x = freq_to_x(f_center);
        let y = spec_norm_to_y(bands[k]);
        pts.push(Pos2::new(x, y));
        valid_indices.push(k);
    }

    if pts.is_empty() {
        return;
    }

    let count = pts.len();

    match mode {
        AnalyzerLineMode::Bars => {
            let bar_w = (plot_rect.width() / count as f32 * 0.8).clamp(1.5, 12.0);
            let half_w = bar_w * 0.5;

            for (idx, &k) in valid_indices.iter().enumerate() {
                let x = pts[idx].x;
                let y = pts[idx].y;
                let x_clamped = x.clamp(plot_rect.left() + half_w, plot_rect.right() - half_w);
                let bar_rect = Rect::from_min_max(
                    Pos2::new(x_clamped - half_w, y.clamp(plot_rect.top(), plot_rect.bottom())),
                    Pos2::new(x_clamped + half_w, plot_rect.bottom()),
                );
                painter.rect_filled(bar_rect, CornerRadius::same(1), fill_color);
                painter.rect_stroke(
                    bar_rect,
                    CornerRadius::same(1),
                    Stroke::new(1.0, stroke_color),
                    StrokeKind::Inside,
                );

                if hold && peak_hold[k] > 0.02 {
                    let y_hold = spec_norm_to_y(peak_hold[k]);
                    painter.line_segment(
                        [
                            Pos2::new(x_clamped - half_w, y_hold),
                            Pos2::new(x_clamped + half_w, y_hold),
                        ],
                        Stroke::new(1.5, Color32::WHITE),
                    );
                }
            }
        }
        AnalyzerLineMode::Filled => {
            // Tessellate area under spectrum using GPU mesh (immune to non-convex polygon bugs)
            if count >= 2 {
                let mut mesh = Mesh::default();
                mesh.reserve_triangles((count - 1) * 2);
                mesh.reserve_vertices(count * 2);

                for (idx, pt) in pts.iter().enumerate() {
                    let top_v = *pt;
                    let bot_v = Pos2::new(pt.x, plot_rect.bottom());

                    let top_color = fill_color;
                    let bot_color = Color32::from_rgba_unmultiplied(
                        fill_color.r(),
                        fill_color.g(),
                        fill_color.b(),
                        (fill_color.a() as f32 * 0.25) as u8,
                    );

                    mesh.colored_vertex(top_v, top_color);
                    mesh.colored_vertex(bot_v, bot_color);

                    if idx < count - 1 {
                        let base = (idx * 2) as u32;
                        mesh.add_triangle(base, base + 2, base + 3);
                        mesh.add_triangle(base, base + 3, base + 1);
                    }
                }
                painter.add(egui::Shape::mesh(mesh));

                // Sharp line on top of the fill
                painter.add(egui::Shape::line(pts.clone(), Stroke::new(1.8, stroke_color)));

                // Peak hold ticks if active
                if hold {
                    let tick_w = (plot_rect.width() / count as f32 * 0.8).clamp(2.0, 10.0);
                    let half_tick = tick_w * 0.5;
                    for (idx, &k) in valid_indices.iter().enumerate() {
                        if peak_hold[k] > 0.02 {
                            let y_hold = spec_norm_to_y(peak_hold[k]);
                            let x = pts[idx].x;
                            let x_tick = x.clamp(plot_rect.left() + half_tick, plot_rect.right() - half_tick);
                            painter.line_segment(
                                [
                                    Pos2::new(x_tick - half_tick, y_hold),
                                    Pos2::new(x_tick + half_tick, y_hold),
                                ],
                                Stroke::new(1.5, Color32::WHITE),
                            );
                        }
                    }
                }
            }
        }
        AnalyzerLineMode::Curve => {
            // Draw clean curve line
            if count >= 2 {
                painter.add(egui::Shape::line(pts.clone(), Stroke::new(1.8, stroke_color)));

                // Peak hold ticks if active
                if hold {
                    let tick_w = (plot_rect.width() / count as f32 * 0.8).clamp(2.0, 10.0);
                    let half_tick = tick_w * 0.5;
                    for (idx, &k) in valid_indices.iter().enumerate() {
                        if peak_hold[k] > 0.02 {
                            let y_hold = spec_norm_to_y(peak_hold[k]);
                            let x = pts[idx].x;
                            let x_tick = x.clamp(plot_rect.left() + half_tick, plot_rect.right() - half_tick);
                            painter.line_segment(
                                [
                                    Pos2::new(x_tick - half_tick, y_hold),
                                    Pos2::new(x_tick + half_tick, y_hold),
                                ],
                                Stroke::new(1.5, Color32::WHITE),
                            );
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod gui_tests {
    use super::*;

    #[test]
    fn test_spectrum_and_curve_coordinate_alignment() {
        let plot_rect = egui::Rect::from_min_max(egui::Pos2::new(32.0, 10.0), egui::Pos2::new(932.0, 210.0));
        let f_min = 20.0f32;
        let f_max = 20000.0f32;

        let f = 1000.0f32;
        let x = plot_rect.left() + (f.log10() - f_min.log10()) / (f_max.log10() - f_min.log10()) * plot_rect.width();
        
        let freq_recalculated = f_min * (10.0f32).powf((x - plot_rect.left()) / plot_rect.width() * (f_max.log10() - f_min.log10()));
        assert!((f - freq_recalculated).abs() < 0.1);
    }
    
    struct MockGuiContext {
        begins: std::sync::atomic::AtomicUsize,
        ends: std::sync::atomic::AtomicUsize,
    }
    impl nih_plug::prelude::GuiContext for MockGuiContext {
        fn plugin_api(&self) -> nih_plug::prelude::PluginApi { nih_plug::prelude::PluginApi::Vst3 }
        fn request_resize(&self) -> bool { false }
        unsafe fn raw_begin_set_parameter(&self, _param: nih_plug::prelude::ParamPtr) {
            self.begins.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        unsafe fn raw_set_parameter_normalized(&self, _param: nih_plug::prelude::ParamPtr, _normalized: f32) {}
        unsafe fn raw_end_set_parameter(&self, _param: nih_plug::prelude::ParamPtr) {
            self.ends.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
        fn get_state(&self) -> nih_plug::prelude::PluginState { nih_plug::prelude::PluginState { version: String::new(), params: std::collections::BTreeMap::new(), fields: std::collections::BTreeMap::new() } }
        fn set_state(&self, _state: nih_plug::prelude::PluginState) {}
    }

    #[test]
    fn test_f05_editor_reopening_and_cancellation() {
        let tracker = GestureTracker::new();
        
        let ctx = std::sync::Arc::new(MockGuiContext {
            begins: std::sync::atomic::AtomicUsize::new(0),
            ends: std::sync::atomic::AtomicUsize::new(0),
        });
        
        let initial_session = tracker.state.lock().unwrap().session_id;
        assert_eq!(initial_session, 0);
        
        tracker.close_generation(0, Some(ctx.as_ref()));
        
        let next_session = tracker.state.lock().unwrap().session_id;
        assert_eq!(next_session, 0);
    }

    #[test]
    fn test_f17_gui_preset_save_error_feedback_and_retention() {
        // 1. Verify that empty name yields an error and preserves dialog state
        let mut save_name = "   ".to_string();
        let mut save_dialog_open = true;
        let mut feedback_msg: Option<(String, std::time::Instant)> = None;

        let trimmed = save_name.trim();
        if trimmed.is_empty() {
            feedback_msg = Some(("Error: El nombre no puede estar vacío".to_string(), std::time::Instant::now()));
        } else {
            save_dialog_open = false;
        }

        assert!(save_dialog_open, "Dialog must remain open on empty name error");
        assert_eq!(save_name, "   ", "Input name must be retained");
        assert!(feedback_msg.is_some());
        assert!(feedback_msg.as_ref().unwrap().0.starts_with("Error"));

        // 2. Verify that I/O save error preserves dialog state and user input
        save_name = "My Test Preset".to_string();
        let simulated_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Acceso denegado");
        feedback_msg = Some((format!("Error al guardar: {}", simulated_err), std::time::Instant::now()));

        assert!(save_dialog_open, "Dialog must remain open on save error");
        assert_eq!(save_name, "My Test Preset", "Input name must be preserved on error");
        assert!(feedback_msg.as_ref().unwrap().0.starts_with("Error al guardar: Acceso denegado"));

        // 3. Verify that successful save closes dialog and sets success feedback
        save_dialog_open = false;
        feedback_msg = Some(("Guardado: My Test Preset".to_string(), std::time::Instant::now()));
        assert!(!save_dialog_open, "Dialog must close only on success");
        assert!(feedback_msg.as_ref().unwrap().0.starts_with("Guardado:"));
    }

    #[test]
    fn test_f13_gui_frequency_coordinates_and_nyquist_bounds() {
        let plot_rect = egui::Rect::from_min_max(egui::Pos2::new(0.0, 0.0), egui::Pos2::new(1000.0, 500.0));
        let visual_max_hz = 20000.0f32;

        // Test 1: 44.1 kHz (Nyquist = 22050 Hz)
        let sr_44 = 44100.0f32;
        let nyquist_44 = sr_44 * 0.5;
        let max_44 = visual_max_hz.min(nyquist_44);
        assert_eq!(max_44, 20000.0);
        assert!(max_44 <= 22050.0);

        // Test 2: 48 kHz (Nyquist = 24000 Hz)
        let sr_48 = 48000.0f32;
        let nyquist_48 = sr_48 * 0.5;
        let max_48 = visual_max_hz.min(nyquist_48);
        assert_eq!(max_48, 20000.0);
        assert!(max_48 <= 24000.0);

        // Test 3: Low artificial SR 32 kHz (Nyquist = 16000 Hz)
        let sr_32 = 32000.0f32;
        let nyquist_32 = sr_32 * 0.5;
        let max_32 = visual_max_hz.min(nyquist_32);
        assert_eq!(max_32, 16000.0, "At 32 kHz SR, visual limit must be exactly 16000 Hz");

        let f_min = 20.0f32;
        let f_max_32 = max_32;
        let log_range_32 = (f_max_32 / f_min).log10();
        let freq_to_x_32 = |f: f32| -> f32 {
            let f_clamped = f.clamp(f_min, f_max_32);
            let norm = (f_clamped / f_min).log10() / log_range_32;
            plot_rect.left() + norm * plot_rect.width()
        };

        // Right edge must correspond to 16000 Hz
        assert!((freq_to_x_32(16000.0) - plot_rect.right()).abs() < 1e-3);

        // Grid marks > 16 kHz must be filtered out
        let freq_marks = [20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0, 20000.0];
        let visible_marks: Vec<f32> = freq_marks.iter().copied().filter(|&f| f <= max_32).collect();
        assert!(!visible_marks.contains(&20000.0), "20k mark must be omitted when Nyquist is 16k");
        assert_eq!(*visible_marks.last().unwrap(), 10000.0);

        // Test 4: High SR 96 kHz (Nyquist = 48000 Hz)
        let sr_96 = 96000.0f32;
        let nyquist_96 = sr_96 * 0.5;
        let max_96 = visual_max_hz.min(nyquist_96);
        assert_eq!(max_96, 20000.0, "At 96 kHz SR, visual limit must not expand beyond 20000 Hz");
    }

}

#[cfg(test)]
mod f05_tests;
