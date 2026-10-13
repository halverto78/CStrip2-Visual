use super::*;
use nih_plug::prelude::{GuiContext as HostContext, PluginApi, PluginState};
use std::collections::{BTreeMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicUsize};

fn fixture() -> GuiContext {
    let params = Arc::new(Cstrip2Params::default());
    GuiContext {
        persistent: Arc::new(Mutex::new(PersistentGuiState {
            slot_a: PresetData::from_params(&params, "Slot A"),
            slot_b: PresetData::from_params(&params, "Slot B"),
            active_slot: ActiveSlot::A,
            current_preset_name: "Default".into(),
            user_scale_factor: None,
        })),
        params,
        input_queue: Arc::new(SpectrumAudioQueue::new()),
        output_queue: Arc::new(SpectrumAudioQueue::new()),
        curve_worker: Arc::new(CurveWorker::new()),
        sample_rate: Arc::new(AtomicF32::new(44100.0)),
        in_peak: Arc::new(AtomicF32::new(0.0)),
        in_rms: Arc::new(AtomicF32::new(0.0)),
        out_peak: Arc::new(AtomicF32::new(0.0)),
        out_rms: Arc::new(AtomicF32::new(0.0)),
        comp_gr: Arc::new(AtomicF32::new(0.0)),
        displayed_gr: 0.0,
        spectrum_processor_in: SpectrumProcessor::new(44100.0),
        spectrum_processor_out: SpectrumProcessor::new(44100.0),
        last_frame_time: Instant::now(),
        last_curve_params: None,
        factory_presets: get_factory_presets(),
        user_presets: vec![],
        feedback_message: None,
        save_dialog_open: false,
        save_preset_name: String::new(),
        tracker: GestureTracker::new(),
    }
}

fn editor_for(gui: &GuiContext) -> EditorWrapper {
    EditorWrapper {
        params: gui.params.clone(),
        input_queue: gui.input_queue.clone(),
        output_queue: gui.output_queue.clone(),
        curve_worker: gui.curve_worker.clone(),
        sample_rate: gui.sample_rate.clone(),
        in_peak: gui.in_peak.clone(),
        in_rms: gui.in_rms.clone(),
        out_peak: gui.out_peak.clone(),
        out_rms: gui.out_rms.clone(),
        comp_gr: gui.comp_gr.clone(),
        editor_state: default_state(),
        tracker: gui.tracker.clone(),
        persistent: gui.persistent.clone(),
        active_inner: Arc::new(Mutex::new(None)),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase { Begin, Set, End }

#[derive(Clone, Debug, PartialEq)]
enum Event { Begin(String), Set(String, f32), End(String), Revoked }

struct Probe {
    persistent: Arc<Mutex<PersistentGuiState>>,
    tracker: GestureTracker,
    generation: usize,
    close_at: Option<Phase>,
    closed: AtomicBool,
    lock_checks: AtomicUsize,
    events: Mutex<Vec<Event>>,
}

impl Probe {
    fn new(gui: &GuiContext, generation: usize, close_at: Option<Phase>) -> Self {
        Self {
            persistent: gui.persistent.clone(), tracker: gui.tracker.clone(), generation,
            close_at, closed: AtomicBool::new(false), lock_checks: AtomicUsize::new(0),
            events: Mutex::new(vec![]),
        }
    }

    fn callback(&self, phase: Phase, event: Event) {
        assert!(self.persistent.try_lock().is_ok(), "persistent mutex held during {phase:?}");
        self.lock_checks.fetch_add(1, Ordering::Relaxed);
        self.events.lock().unwrap().push(event);
        if self.close_at == Some(phase) && !self.closed.swap(true, Ordering::Relaxed) {
            self.tracker.close_generation(self.generation, Some(self));
            assert!(!self.tracker.is_generation_valid(self.generation));
            self.events.lock().unwrap().push(Event::Revoked);
        }
    }

    fn assert_balanced(&self) -> (usize, usize, usize) {
        let mut active = HashSet::new();
        let (mut begins, mut sets, mut ends) = (0, 0, 0);
        for event in self.events.lock().unwrap().iter() {
            match event {
                Event::Begin(name) => { assert!(active.insert(name.clone()), "duplicate Begin"); begins += 1; }
                Event::Set(name, _) => { assert!(active.contains(name), "Set outside gesture"); sets += 1; }
                Event::End(name) => { assert!(active.remove(name), "End without Begin"); ends += 1; }
                Event::Revoked => {}
            }
        }
        assert!(active.is_empty(), "gestures left open: {active:?}");
        assert_eq!(begins, ends);
        (begins, sets, ends)
    }
}

impl HostContext for Probe {
    fn plugin_api(&self) -> PluginApi { PluginApi::Clap }
    fn request_resize(&self) -> bool { true }
    unsafe fn raw_begin_set_parameter(&self, ptr: ParamPtr) {
        self.callback(Phase::Begin, Event::Begin(ptr.name().into()));
    }
    unsafe fn raw_set_parameter_normalized(&self, ptr: ParamPtr, value: f32) {
        self.callback(Phase::Set, Event::Set(ptr.name().into(), value));
    }
    unsafe fn raw_end_set_parameter(&self, ptr: ParamPtr) {
        self.callback(Phase::End, Event::End(ptr.name().into()));
    }
    fn get_state(&self) -> PluginState {
        PluginState { version: String::new(), params: BTreeMap::new(), fields: BTreeMap::new() }
    }
    fn set_state(&self, _: PluginState) {}
}

struct ScaleProbe {
    persistent: Arc<Mutex<PersistentGuiState>>,
    accept: bool,
    requests: Arc<Mutex<Vec<f32>>>,
}

impl Editor for ScaleProbe {
    fn spawn(&self, _: ParentWindowHandle, _: Arc<dyn HostContext>) -> Box<dyn std::any::Any + Send> {
        panic!("unit test does not create native windows")
    }
    fn size(&self) -> (u32, u32) { (780, 560) }
    fn set_scale_factor(&self, factor: f32) -> bool {
        assert!(self.persistent.try_lock().is_ok(), "persistent mutex held during editor callback");
        self.requests.lock().unwrap().push(factor);
        self.accept
    }
    fn param_value_changed(&self, _: &str, _: f32) {}
    fn param_modulation_changed(&self, _: &str, _: f32) {}
    fn param_values_changed(&self) {}
}

#[test]
fn stale_generation_cannot_overwrite_new_window_or_emit_actions() {
    let gui = fixture();
    let old = gui.tracker.begin_session();
    let mut stale = gui.snapshot_for_generation(old).unwrap();
    stale.current_preset_name = "obsolete".into();
    stale.slot_b.output = 0.1;
    gui.tracker.close_generation(old, None);
    let current = gui.tracker.begin_session();
    let mut live = gui.snapshot_for_generation(current).unwrap();
    live.active_slot = ActiveSlot::B;
    live.current_preset_name = "Vocal Air & Body".into();
    live.slot_a.output = 0.4;
    live.slot_b.output = 0.7;
    assert!(gui.commit_for_generation(current, live.clone()));
    assert!(!gui.commit_for_generation(old, stale));
    let probe = Probe::new(&gui, old, None);
    let setter = ParamSetter::new(&probe);
    assert!(!gui.apply_preset_for_generation(old, &PresetData::default(), &setter));
    assert!(!gui.switch_slot_for_generation(old, ActiveSlot::A, &setter));
    assert!(!gui.copy_slot_for_generation(old, ActiveSlot::B, &setter));
    assert!(probe.events.lock().unwrap().is_empty());
    assert!(gui.snapshot_for_generation(old).is_none());
    let stored = gui.snapshot_for_generation(current).unwrap();
    assert_eq!(stored.slot_a, live.slot_a);
    assert_eq!(stored.slot_b, live.slot_b);
    assert_eq!(stored.active_slot, ActiveSlot::B);
    assert_eq!(stored.current_preset_name, "Vocal Air & Body");
}

#[test]
fn window_commit_does_not_roll_back_later_host_scale() {
    let gui = fixture();
    let generation = gui.tracker.begin_session();
    let mut candidate = gui.snapshot_for_generation(generation).unwrap();
    assert_eq!(candidate.user_scale_factor, None);
    let editor = editor_for(&gui);
    assert!(editor.set_scale_factor(1.75));
    candidate.current_preset_name = "GUI edit".into();
    candidate.active_slot = ActiveSlot::B;
    candidate.slot_a.bass = 3.0;
    candidate.slot_b.treble = -3.0;
    assert!(gui.commit_for_generation(generation, candidate.clone()));
    let stored = gui.persistent.lock().unwrap();
    assert_eq!(stored.user_scale_factor, Some(1.75));
    assert_eq!(stored.current_preset_name, "GUI edit");
    assert_eq!(stored.slot_a, candidate.slot_a);
    assert_eq!(stored.slot_b, candidate.slot_b);
    assert_eq!(stored.active_slot, ActiveSlot::B);
}

#[test]
fn rejected_editor_scale_does_not_change_persistent_request() {
    let gui = fixture();
    let editor = editor_for(&gui);
    assert!(editor.set_scale_factor(1.5));
    assert!(editor.set_scale_factor(2.0));
    let requests = Arc::new(Mutex::new(vec![]));
    *editor.active_inner.lock().unwrap() = Some(Box::new(ScaleProbe {
        persistent: gui.persistent.clone(), accept: false, requests: requests.clone(),
    }));
    assert!(!editor.set_scale_factor(1.75));
    assert_eq!(gui.persistent.lock().unwrap().user_scale_factor, Some(2.0));
    *editor.active_inner.lock().unwrap() = Some(Box::new(ScaleProbe {
        persistent: gui.persistent.clone(), accept: true, requests: requests.clone(),
    }));
    assert!(editor.set_scale_factor(1.25));
    assert_eq!(gui.persistent.lock().unwrap().user_scale_factor, Some(1.25));
    assert_eq!(*requests.lock().unwrap(), vec![1.75, 1.25]);
}

#[test]
fn preset_host_callbacks_run_without_persistent_lock() {
    let gui = fixture();
    let generation = gui.tracker.begin_session();
    let probe = Probe::new(&gui, generation, None);
    let setter = ParamSetter::new(&probe);
    let preset = PresetData { name: "lock probe".into(), bass: 3.0, output: 0.72, ..PresetData::default() };
    assert!(gui.apply_preset_for_generation(generation, &preset, &setter));
    assert_eq!(probe.assert_balanced(), (10, 10, 10));
    assert_eq!(probe.lock_checks.load(Ordering::Relaxed), 30);
    assert!(gui.tracker.state.lock().unwrap().active.is_empty());
    let events = probe.events.lock().unwrap();
    let expected_output = gui.params.output.preview_normalized(preset.output);
    assert!(events.iter().any(|event| matches!(event, Event::Set(name, value) if name == "Output" && (*value - expected_output).abs() < 1e-6)));
}

#[test]
fn preset_reentrant_close_stops_after_begin_set_or_end() {
    for phase in [Phase::Begin, Phase::Set, Phase::End] {
        let gui = fixture();
        let generation = gui.tracker.begin_session();
        let before = gui.persistent.lock().unwrap().clone();
        let probe = Probe::new(&gui, generation, Some(phase));
        let setter = ParamSetter::new(&probe);
        assert!(!gui.apply_preset_for_generation(generation, &PresetData::default(), &setter));
        assert_eq!(probe.assert_balanced(), (1, usize::from(phase != Phase::Begin), 1));
        let events = probe.events.lock().unwrap();
        assert_eq!(events.last(), Some(&Event::Revoked), "callback after revocation at {phase:?}: {events:?}");
        assert!(gui.tracker.state.lock().unwrap().active.is_empty());
        assert!(!gui.tracker.is_generation_valid(generation));
        let stored = gui.persistent.lock().unwrap();
        assert_eq!(stored.slot_a, before.slot_a);
        assert_eq!(stored.slot_b, before.slot_b);
        assert_eq!(stored.active_slot, before.active_slot);
        assert_eq!(stored.current_preset_name, before.current_preset_name);
    }
}
