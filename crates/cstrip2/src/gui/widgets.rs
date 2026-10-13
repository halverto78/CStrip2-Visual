use nih_plug::prelude::{FloatParam, Param, ParamSetter};
use nih_plug_egui::egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2,
};

/// Analog style rotary knob for standard unipolar parameters
pub fn analog_knob(
    ui: &mut Ui,
    setter: &ParamSetter,
    param: &impl Param,
    guard: &mut crate::gui::InteractionGuard,
    label: &str,
    diameter: f32,
    accent_color: Color32,
) -> Response {
    let desired_size = Vec2::new(diameter + 14.0, diameter + 32.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

    // F05: Pre-interaction cancellation gate
    let mut cancelled_this_frame = false;
    ui.input(|i| {
        for event in &i.events {
            if matches!(event, nih_plug_egui::egui::Event::PointerGone | nih_plug_egui::egui::Event::WindowFocused(false)) {
                cancelled_this_frame = true;
            }
        }
    });

    if cancelled_this_frame {
        if guard.is_active(param) {
            guard.end(param, setter);
        }
        return response; // Abort further events this frame
    }

    let val = param.unmodulated_normalized_value();

    if response.drag_started() {
        guard.begin(param, setter);
    }
    
    if response.drag_stopped() {
        guard.end(param, setter);
    }

    if response.dragged() {
        let delta = -response.drag_delta().y / 150.0;
        let new_val = (val + delta).clamp(0.0, 1.0);
        guard.set(param, setter, new_val);
    }



    if response.double_clicked() {
        if guard.begin(param, setter) {
            guard.set(param, setter, param.default_normalized_value());
            guard.end(param, setter);
        }
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let center = Pos2::new(rect.center().x, rect.top() + diameter * 0.5 + 2.0);
        let radius = diameter * 0.5;

        // Outer brushed metal ring
        painter.circle_filled(center, radius + 2.0, Color32::from_rgb(32, 35, 42));
        painter.circle_stroke(
            center,
            radius + 2.0,
            Stroke::new(1.0, Color32::from_rgb(55, 60, 70)),
        );

        // Knob body
        painter.circle_filled(center, radius, Color32::from_rgb(20, 22, 26));

        // Arc track (from 135 deg to 405 deg)
        let start_angle = 135.0f32.to_radians();
        let end_angle = 405.0f32.to_radians();
        let current_angle = start_angle + val * (end_angle - start_angle);

        // Background track arc
        let track_radius = radius - 3.0;
        let n_points = 24;
        for i in 0..n_points {
            let t1 = i as f32 / n_points as f32;
            let t2 = (i + 1) as f32 / n_points as f32;
            let a1 = start_angle + t1 * (end_angle - start_angle);
            let a2 = start_angle + t2 * (end_angle - start_angle);
            let p1 = center + Vec2::new(a1.cos(), a1.sin()) * track_radius;
            let p2 = center + Vec2::new(a2.cos(), a2.sin()) * track_radius;
            painter.line_segment([p1, p2], Stroke::new(2.0, Color32::from_rgb(40, 44, 52)));
        }

        // Active track arc with accent color
        let active_points = (val * n_points as f32).ceil() as usize;
        for i in 0..active_points {
            let t1 = i as f32 / n_points as f32;
            let t2 = ((i + 1) as f32 / n_points as f32).min(val);
            let a1 = start_angle + t1 * (end_angle - start_angle);
            let a2 = start_angle + t2 * (end_angle - start_angle);
            let p1 = center + Vec2::new(a1.cos(), a1.sin()) * track_radius;
            let p2 = center + Vec2::new(a2.cos(), a2.sin()) * track_radius;
            painter.line_segment([p1, p2], Stroke::new(2.5, accent_color));
        }

        // Pointer notch
        let ptr_inner = center + Vec2::new(current_angle.cos(), current_angle.sin()) * (radius * 0.3);
        let ptr_outer = center + Vec2::new(current_angle.cos(), current_angle.sin()) * (radius - 2.0);
        painter.line_segment([ptr_inner, ptr_outer], Stroke::new(2.0, Color32::WHITE));

        // Label above
        let label_pos = Pos2::new(center.x, rect.bottom() - 20.0);
        painter.text(
            label_pos,
            nih_plug_egui::egui::Align2::CENTER_CENTER,
            label,
            FontId::proportional(11.0),
            Color32::from_rgb(180, 185, 195),
        );

        // Value text below
        let val_str = param.to_string();
        let val_pos = Pos2::new(center.x, rect.bottom() - 7.0);
        let val_color = if response.hovered() || response.dragged() {
            accent_color
        } else {
            Color32::from_rgb(220, 225, 235)
        };
        painter.text(
            val_pos,
            nih_plug_egui::egui::Align2::CENTER_CENTER,
            val_str,
            FontId::monospace(10.0),
            val_color,
        );
    }

    response
}

/// Bipolar rotary knob for EQ gains with 0 dB at 12 o'clock center
pub fn bipolar_knob(
    ui: &mut Ui,
    setter: &ParamSetter,
    param: &impl Param,
    guard: &mut crate::gui::InteractionGuard,
    label: &str,
    diameter: f32,
    accent_color: Color32,
) -> Response {
    let desired_size = Vec2::new(diameter + 14.0, diameter + 32.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

    // F05: Pre-interaction cancellation gate
    let mut cancelled_this_frame = false;
    ui.input(|i| {
        for event in &i.events {
            if matches!(event, nih_plug_egui::egui::Event::PointerGone | nih_plug_egui::egui::Event::WindowFocused(false)) {
                cancelled_this_frame = true;
            }
        }
    });

    if cancelled_this_frame {
        if guard.is_active(param) {
            guard.end(param, setter);
        }
        return response; // Abort further events this frame
    }

    let val = param.unmodulated_normalized_value();

    if response.drag_started() {
        guard.begin(param, setter);
    }
    
    if response.drag_stopped() {
        guard.end(param, setter);
    }

    if response.dragged() {
        let delta = -response.drag_delta().y / 150.0;
        let new_val = (val + delta).clamp(0.0, 1.0);
        guard.set(param, setter, new_val);
    }



    if response.double_clicked() {
        if guard.begin(param, setter) {
            guard.set(param, setter, 0.5); // Center default 0 dB
            guard.end(param, setter);
        }
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let center = Pos2::new(rect.center().x, rect.top() + diameter * 0.5 + 2.0);
        let radius = diameter * 0.5;

        painter.circle_filled(center, radius + 2.0, Color32::from_rgb(32, 35, 42));
        painter.circle_stroke(
            center,
            radius + 2.0,
            Stroke::new(1.0, Color32::from_rgb(55, 60, 70)),
        );
        painter.circle_filled(center, radius, Color32::from_rgb(20, 22, 26));

        let start_angle = 135.0f32.to_radians();
        let end_angle = 405.0f32.to_radians();
        let center_angle = 270.0f32.to_radians(); // 12 o'clock
        let current_angle = start_angle + val * (end_angle - start_angle);

        let track_radius = radius - 3.0;

        // Background track arc
        let n_points = 24;
        for i in 0..n_points {
            let t1 = i as f32 / n_points as f32;
            let t2 = (i + 1) as f32 / n_points as f32;
            let a1 = start_angle + t1 * (end_angle - start_angle);
            let a2 = start_angle + t2 * (end_angle - start_angle);
            let p1 = center + Vec2::new(a1.cos(), a1.sin()) * track_radius;
            let p2 = center + Vec2::new(a2.cos(), a2.sin()) * track_radius;
            painter.line_segment([p1, p2], Stroke::new(2.0, Color32::from_rgb(40, 44, 52)));
        }

        // Center 0 dB notch tick
        let p_c1 = center + Vec2::new(0.0, -1.0) * (radius - 5.0);
        let p_c2 = center + Vec2::new(0.0, -1.0) * (radius + 2.0);
        painter.line_segment([p_c1, p_c2], Stroke::new(1.5, Color32::from_rgb(90, 95, 110)));

        // Bipolar active track from center_angle (270 deg) to current_angle
        let (arc_start, arc_end) = if current_angle < center_angle {
            (current_angle, center_angle)
        } else {
            (center_angle, current_angle)
        };

        let steps = 12;
        for i in 0..steps {
            let t1 = i as f32 / steps as f32;
            let t2 = (i + 1) as f32 / steps as f32;
            let a1 = arc_start + t1 * (arc_end - arc_start);
            let a2 = arc_start + t2 * (arc_end - arc_start);
            let p1 = center + Vec2::new(a1.cos(), a1.sin()) * track_radius;
            let p2 = center + Vec2::new(a2.cos(), a2.sin()) * track_radius;
            painter.line_segment([p1, p2], Stroke::new(2.5, accent_color));
        }

        // Pointer notch
        let ptr_inner = center + Vec2::new(current_angle.cos(), current_angle.sin()) * (radius * 0.3);
        let ptr_outer = center + Vec2::new(current_angle.cos(), current_angle.sin()) * (radius - 2.0);
        painter.line_segment([ptr_inner, ptr_outer], Stroke::new(2.0, Color32::WHITE));

        // Label
        let label_pos = Pos2::new(center.x, rect.bottom() - 20.0);
        painter.text(
            label_pos,
            nih_plug_egui::egui::Align2::CENTER_CENTER,
            label,
            FontId::proportional(11.0),
            Color32::from_rgb(180, 185, 195),
        );

        // Value text
        let val_str = param.to_string();
        let val_pos = Pos2::new(center.x, rect.bottom() - 7.0);
        let val_color = if response.hovered() || response.dragged() {
            accent_color
        } else {
            Color32::from_rgb(220, 225, 235)
        };
        painter.text(
            val_pos,
            nih_plug_egui::egui::Align2::CENTER_CENTER,
            val_str,
            FontId::monospace(10.0),
            val_color,
        );
    }

    response
}

/// Output knob with visual zone indicators for Clean [0..1/3], Spiral [1/3..2/3], Density [2/3..1.0]
pub fn output_knob(
    ui: &mut Ui,
    setter: &ParamSetter,
    param: &impl Param,
    guard: &mut crate::gui::InteractionGuard,
    label: &str,
    diameter: f32,
) -> Response {
    let desired_size = Vec2::new(diameter + 18.0, diameter + 42.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

    // F05: Pre-interaction cancellation gate
    let mut cancelled_this_frame = false;
    ui.input(|i| {
        for event in &i.events {
            if matches!(event, nih_plug_egui::egui::Event::PointerGone | nih_plug_egui::egui::Event::WindowFocused(false)) {
                cancelled_this_frame = true;
            }
        }
    });

    if cancelled_this_frame {
        if guard.is_active(param) {
            guard.end(param, setter);
        }
        return response; // Abort further events this frame
    }

    let val = param.unmodulated_normalized_value();

    if response.drag_started() {
        guard.begin(param, setter);
    }
    
    if response.drag_stopped() {
        guard.end(param, setter);
    }

    if response.dragged() {
        let delta = -response.drag_delta().y / 150.0;
        let new_val = (val + delta).clamp(0.0, 1.0);
        guard.set(param, setter, new_val);
    }



    if response.double_clicked() {
        if guard.begin(param, setter) {
            guard.set(param, setter, 0.33); // Airwindows exact default!
            guard.end(param, setter);
        }
    }

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let center = Pos2::new(rect.center().x, rect.top() + diameter * 0.5 + 2.0);
        let radius = diameter * 0.5;

        painter.circle_filled(center, radius + 2.0, Color32::from_rgb(32, 35, 42));
        painter.circle_stroke(
            center,
            radius + 2.0,
            Stroke::new(1.0, Color32::from_rgb(55, 60, 70)),
        );
        painter.circle_filled(center, radius, Color32::from_rgb(20, 22, 26));

        let start_angle = 135.0f32.to_radians();
        let end_angle = 405.0f32.to_radians();
        let current_angle = start_angle + val * (end_angle - start_angle);

        let track_radius = radius - 3.0;

        // Zone 1: Clean (0..1/3) -> Green/Cyan
        let a_t1 = start_angle + (1.0 / 3.0) * (end_angle - start_angle);
        // Zone 2: Spiral (1/3..2/3) -> Amber/Orange
        let a_t2 = start_angle + (2.0 / 3.0) * (end_angle - start_angle);

        // Draw colored zone backgrounds
        let draw_zone_arc = |a_start: f32, a_end: f32, color: Color32, is_active: bool| {
            let steps = 10;
            let stroke = if is_active {
                Stroke::new(3.0, color)
            } else {
                Stroke::new(1.5, color.gamma_multiply(0.3))
            };
            for i in 0..steps {
                let t1 = i as f32 / steps as f32;
                let t2 = (i + 1) as f32 / steps as f32;
                let a1 = a_start + t1 * (a_end - a_start);
                let a2 = a_start + t2 * (a_end - a_start);
                let p1 = center + Vec2::new(a1.cos(), a1.sin()) * track_radius;
                let p2 = center + Vec2::new(a2.cos(), a2.sin()) * track_radius;
                painter.line_segment([p1, p2], stroke);
            }
        };

        // Zone boundaries
        let col_clean = Color32::from_rgb(70, 190, 140);
        let col_spiral = Color32::from_rgb(230, 160, 50);
        let col_density = Color32::from_rgb(230, 70, 70);

        // Draw full zone arcs dim
        draw_zone_arc(start_angle, a_t1, col_clean, val <= 1.0 / 3.0);
        draw_zone_arc(a_t1, a_t2, col_spiral, val > 1.0 / 3.0 && val <= 2.0 / 3.0);
        draw_zone_arc(a_t2, end_angle, col_density, val > 2.0 / 3.0);

        // Boundary tick marks at 1/3 and 2/3
        for &a in &[a_t1, a_t2] {
            let p1 = center + Vec2::new(a.cos(), a.sin()) * (track_radius - 4.0);
            let p2 = center + Vec2::new(a.cos(), a.sin()) * (track_radius + 4.0);
            painter.line_segment([p1, p2], Stroke::new(1.5, Color32::from_rgb(180, 190, 210)));
        }

        // Pointer notch
        let ptr_inner = center + Vec2::new(current_angle.cos(), current_angle.sin()) * (radius * 0.3);
        let ptr_outer = center + Vec2::new(current_angle.cos(), current_angle.sin()) * (radius - 2.0);
        painter.line_segment([ptr_inner, ptr_outer], Stroke::new(2.0, Color32::WHITE));

        // Zone indicator badge text
        let zone_name = if val <= 1.0 / 3.0 {
            "CLEAN"
        } else if val <= 2.0 / 3.0 {
            "SPIRAL"
        } else {
            "DENSITY"
        };
        let zone_color = if val <= 1.0 / 3.0 {
            col_clean
        } else if val <= 2.0 / 3.0 {
            col_spiral
        } else {
            col_density
        };

        let label_pos = Pos2::new(center.x, rect.bottom() - 30.0);
        painter.text(
            label_pos,
            nih_plug_egui::egui::Align2::CENTER_CENTER,
            label,
            FontId::proportional(11.0),
            Color32::from_rgb(180, 185, 195),
        );

        let badge_pos = Pos2::new(center.x, rect.bottom() - 18.0);
        painter.text(
            badge_pos,
            nih_plug_egui::egui::Align2::CENTER_CENTER,
            zone_name,
            FontId::proportional(9.0),
            zone_color,
        );

        let val_str = param.to_string();
        let val_pos = Pos2::new(center.x, rect.bottom() - 6.0);
        painter.text(
            val_pos,
            nih_plug_egui::egui::Align2::CENTER_CENTER,
            val_str,
            FontId::monospace(10.0),
            Color32::from_rgb(220, 225, 235),
        );
    }

    response
}

/// Stereo Level Meter Bar (dBFS scale with peak and RMS)
pub fn level_meter(
    ui: &mut Ui,
    peak: f32,
    rms: f32,
    label: &str,
    width: f32,
    height: f32,
) {
    let desired_size = Vec2::new(width, height);
    let (rect, _) = ui.allocate_exact_size(desired_size, Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();

        // Meter background
        painter.rect_filled(rect, CornerRadius::same(3), Color32::from_rgb(15, 17, 20));
        painter.rect_stroke(
            rect,
            CornerRadius::same(3),
            Stroke::new(1.0, Color32::from_rgb(40, 44, 52)),
            StrokeKind::Inside,
        );

        // Convert linear peak/rms to dBFS
        let peak_db = 20.0 * (peak + 1e-6).log10();
        let rms_db = 20.0 * (rms + 1e-6).log10();

        // Scale: -60 dBFS to +6 dBFS (total 66 dB range)
        let db_to_norm = |db: f32| ((db + 60.0) / 66.0).clamp(0.0, 1.0);
        let peak_norm = db_to_norm(peak_db);
        let rms_norm = db_to_norm(rms_db);

        let bar_width = (width - 6.0) * 0.5;
        let bar_bottom = rect.bottom() - 18.0;
        let bar_top = rect.top() + 16.0;
        let bar_height = bar_bottom - bar_top;

        // RMS Bar (Left half)
        let rms_h = bar_height * rms_norm;
        let rms_rect = Rect::from_min_max(
            Pos2::new(rect.left() + 2.0, bar_bottom - rms_h),
            Pos2::new(rect.left() + 2.0 + bar_width, bar_bottom),
        );
        let rms_color = if rms_db > 0.0 {
            Color32::from_rgb(235, 70, 70)
        } else if rms_db > -6.0 {
            Color32::from_rgb(235, 170, 40)
        } else {
            Color32::from_rgb(60, 170, 110)
        };
        painter.rect_filled(rms_rect, CornerRadius::same(1), rms_color);

        // Peak Bar (Right half)
        let peak_h = bar_height * peak_norm;
        let peak_rect = Rect::from_min_max(
            Pos2::new(rect.left() + 4.0 + bar_width, bar_bottom - peak_h),
            Pos2::new(rect.right() - 2.0, bar_bottom),
        );
        let peak_color = if peak_db > 0.0 {
            Color32::from_rgb(255, 60, 60)
        } else if peak_db > -6.0 {
            Color32::from_rgb(255, 190, 50)
        } else {
            Color32::from_rgb(80, 200, 130)
        };
        painter.rect_filled(peak_rect, CornerRadius::same(1), peak_color);

        // Clip indicator LED at top
        let clip_rect = Rect::from_min_max(
            Pos2::new(rect.left() + 3.0, rect.top() + 3.0),
            Pos2::new(rect.right() - 3.0, rect.top() + 12.0),
        );
        let clip_color = if peak_db >= 0.0 {
            Color32::from_rgb(255, 30, 30) // Red clip!
        } else {
            Color32::from_rgb(45, 20, 20) // Dim off
        };
        painter.rect_filled(clip_rect, CornerRadius::same(2), clip_color);

        // Label at bottom
        painter.text(
            Pos2::new(rect.center().x, rect.bottom() - 8.0),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(10.0),
            Color32::from_rgb(170, 175, 185),
        );
    }
}

/// Gain Reduction meter displaying real ButterComp linear attenuation in dB
pub fn gr_meter_slot(ui: &mut Ui, width: f32, height: f32, gr_db: f32) {
    let desired_size = Vec2::new(width, height);
    let (rect, _) = ui.allocate_exact_size(desired_size, Sense::hover());

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();

        // Meter background chassis
        painter.rect_filled(rect, CornerRadius::same(3), Color32::from_rgb(15, 17, 20));
        painter.rect_stroke(
            rect,
            CornerRadius::same(3),
            Stroke::new(1.0, Color32::from_rgb(40, 44, 52)),
            StrokeKind::Inside,
        );

        // Header label
        painter.text(
            Pos2::new(rect.center().x, rect.top() + 8.0),
            Align2::CENTER_CENTER,
            "GR",
            FontId::proportional(10.0),
            Color32::from_rgb(235, 155, 60), // Amber ButterComp accent
        );

        // Divider
        painter.line_segment(
            [
                Pos2::new(rect.left() + 2.0, rect.top() + 16.0),
                Pos2::new(rect.right() - 2.0, rect.top() + 16.0),
            ],
            Stroke::new(1.0, Color32::from_rgb(32, 38, 48)),
        );

        // Scale geometry: 0, -3, -6, -10, -20 dB downwards
        let track_left = rect.left() + 3.0;
        let track_width = 8.0;
        let track_top = rect.top() + 20.0;
        let track_bottom = rect.bottom() - 18.0;
        let track_height = track_bottom - track_top;

        let track_rect = Rect::from_min_max(
            Pos2::new(track_left, track_top),
            Pos2::new(track_left + track_width, track_bottom),
        );

        // Dark recessed LED track
        painter.rect_filled(track_rect, CornerRadius::same(1), Color32::from_rgb(10, 12, 15));
        painter.rect_stroke(
            track_rect,
            CornerRadius::same(1),
            Stroke::new(1.0, Color32::from_rgb(25, 30, 38)),
            StrokeKind::Inside,
        );

        // Active amber LED ladder segments
        let num_leds = 24;
        for i in 0..num_leds {
            let y0 = track_top + (i as f32 / num_leds as f32) * track_height;
            let y1 = track_top + ((i + 1) as f32 / num_leds as f32) * track_height - 1.0;
            let led_rect = Rect::from_min_max(
                Pos2::new(track_left + 1.0, y0 + 1.0),
                Pos2::new(track_left + track_width - 1.0, y1),
            );

            let led_thresh = (i as f32 + 0.5) / (num_leds as f32) * 20.0;
            if gr_db >= led_thresh {
                let led_color = if led_thresh < 6.0 {
                    Color32::from_rgb(255, 185, 55)
                } else if led_thresh < 12.0 {
                    Color32::from_rgb(255, 140, 30)
                } else {
                    Color32::from_rgb(255, 80, 20)
                };
                painter.rect_filled(led_rect, CornerRadius::same(1), led_color);
            } else {
                painter.rect_filled(led_rect, CornerRadius::same(1), Color32::from_rgb(24, 20, 16));
            }
        }

        // Scale marks (0 dB at top, 20 dB reduction at bottom)
        let marks = [
            (0.00f32, "0"),
            (0.15f32, "3"),
            (0.30f32, "6"),
            (0.50f32, "10"),
            (0.75f32, "15"),
            (1.00f32, "20"),
        ];

        let tick_x = track_left + track_width + 1.5;
        let text_x = rect.right() - 2.5;

        for &(norm, label) in &marks {
            let y = track_top + norm * track_height;
            painter.line_segment(
                [Pos2::new(tick_x, y), Pos2::new(tick_x + 2.5, y)],
                Stroke::new(1.0, Color32::from_rgb(60, 70, 85)),
            );
            painter.text(
                Pos2::new(text_x, y),
                Align2::RIGHT_CENTER,
                label,
                FontId::monospace(7.5),
                Color32::from_rgb(120, 130, 145),
            );
        }

        // Label at bottom
        painter.text(
            Pos2::new(rect.center().x, rect.bottom() - 8.0),
            Align2::CENTER_CENTER,
            "dB",
            FontId::monospace(8.0),
            Color32::from_rgb(140, 150, 165),
        );
    }
}

/// Identifies which EQ gain band is being manipulated
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EqBandKind {
    Bass,
    Mid,
    Treble,
}

#[derive(Clone, Copy, Debug, Default)]
struct EqGainDragState {
    start_bass: f32,
    start_mid: f32,
    start_treble: f32,
    total_drag_y: f32,
    is_linked: bool,
}

/// Specialized rotary knob for Bass, Mid, and Treble EQ gains.
/// Normal drag adjusts the single band.
/// Shift + Drag moves all three EQ gains simultaneously by the exact same delta relative to snapshot.
pub fn eq_gain_knob(
    ui: &mut Ui,
    setter: &ParamSetter,
    kind: EqBandKind,
    bass_param: &FloatParam,
    mid_param: &FloatParam,
    treble_param: &FloatParam,
    guard: &mut crate::gui::InteractionGuard,
    label: &str,
    diameter: f32,
    accent_color: Color32,
) -> Response {
    let desired_size = Vec2::new(diameter + 14.0, diameter + 32.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, Sense::click_and_drag());

    let state_id = ui.id().with("cstrip2_eq_gain_drag");

    // F05: Pre-interaction cancellation gate for EQ
    let mut cancelled_this_frame = false;
    ui.input(|i| {
        for event in &i.events {
            if matches!(event, nih_plug_egui::egui::Event::PointerGone | nih_plug_egui::egui::Event::WindowFocused(false)) {
                cancelled_this_frame = true;
            }
        }
    });

    if cancelled_this_frame {
        if guard.is_active(bass_param) { guard.end(bass_param, setter); }
        if guard.is_active(mid_param) { guard.end(mid_param, setter); }
        if guard.is_active(treble_param) { guard.end(treble_param, setter); }
        ui.data_mut(|d| d.remove_temp::<EqGainDragState>(state_id));
        return response;
    }

    let this_param = match kind {
        EqBandKind::Bass => bass_param,
        EqBandKind::Mid => mid_param,
        EqBandKind::Treble => treble_param,
    };

    if response.drag_started() {
        let is_linked = ui.input(|i| i.modifiers.shift);
        let start_bass = bass_param.unmodulated_normalized_value();
        let start_mid = mid_param.unmodulated_normalized_value();
        let start_treble = treble_param.unmodulated_normalized_value();

        if is_linked {
            guard.begin(bass_param, setter);
            guard.begin(mid_param, setter);
            guard.begin(treble_param, setter);
        } else {
            guard.begin(this_param, setter);
        }

        ui.data_mut(|d| {
            d.insert_temp(
                state_id,
                EqGainDragState {
                    start_bass,
                    start_mid,
                    start_treble,
                    total_drag_y: 0.0,
                    is_linked,
                },
            );
        });
    }

    if response.drag_stopped() {
        if let Some(state) = ui.data_mut(|d| d.get_temp::<EqGainDragState>(state_id)) {
            if state.is_linked {
                guard.end(bass_param, setter);
                guard.end(mid_param, setter);
                guard.end(treble_param, setter);
            } else {
                guard.end(this_param, setter);
            }
            ui.data_mut(|d| d.remove_temp::<EqGainDragState>(state_id));
        }
    }

    if response.dragged() {
        if let Some(mut state) = ui.data_mut(|d| d.get_temp::<EqGainDragState>(state_id)) {
            state.total_drag_y += response.drag_delta().y;
            let total_delta = -state.total_drag_y / 150.0;

            if state.is_linked {
                let new_bass = (state.start_bass + total_delta).clamp(0.0, 1.0);
                let new_mid = (state.start_mid + total_delta).clamp(0.0, 1.0);
                let new_treble = (state.start_treble + total_delta).clamp(0.0, 1.0);
                guard.set(bass_param, setter, new_bass);
                guard.set(mid_param, setter, new_mid);
                guard.set(treble_param, setter, new_treble);
            } else {
                let start_val = match kind {
                    EqBandKind::Bass => state.start_bass,
                    EqBandKind::Mid => state.start_mid,
                    EqBandKind::Treble => state.start_treble,
                };
                let new_val = (start_val + total_delta).clamp(0.0, 1.0);
                guard.set(this_param, setter, new_val);
            }

            ui.data_mut(|d| d.insert_temp(state_id, state));
        }
    }

    if response.double_clicked() {
        if ui.input(|i| i.modifiers.shift) {
            if guard.begin(bass_param, setter) {
                guard.set(bass_param, setter, 0.5);
                guard.end(bass_param, setter);
            }
            if guard.begin(mid_param, setter) {
                guard.set(mid_param, setter, 0.5);
                guard.end(mid_param, setter);
            }
            if guard.begin(treble_param, setter) {
                guard.set(treble_param, setter, 0.5);
                guard.end(treble_param, setter);
            }
        } else {
            if guard.begin(this_param, setter) {
                guard.set(this_param, setter, 0.5);
                guard.end(this_param, setter);
            }
        }
    }

    response = response.on_hover_text(
        "Arrastrar: Ajustar ganancia\nShift + Arrastrar: Vincular Bass, Mid y Treble\nDoble clic: Reset 0 dB (Shift resetea los 3)",
    );

    let val = this_param.unmodulated_normalized_value();

    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let center = Pos2::new(rect.center().x, rect.top() + diameter * 0.5 + 2.0);
        let radius = diameter * 0.5;

        painter.circle_filled(center, radius + 2.0, Color32::from_rgb(32, 35, 42));
        painter.circle_stroke(
            center,
            radius + 2.0,
            Stroke::new(1.0, Color32::from_rgb(55, 60, 70)),
        );
        painter.circle_filled(center, radius, Color32::from_rgb(20, 22, 26));

        let start_angle = 135.0f32.to_radians();
        let end_angle = 405.0f32.to_radians();
        let center_angle = 270.0f32.to_radians(); // 12 o'clock
        let current_angle = start_angle + val * (end_angle - start_angle);

        let track_radius = radius - 3.0;

        // Background track arc
        let n_points = 24;
        for i in 0..n_points {
            let t1 = i as f32 / n_points as f32;
            let t2 = (i + 1) as f32 / n_points as f32;
            let a1 = start_angle + t1 * (end_angle - start_angle);
            let a2 = start_angle + t2 * (end_angle - start_angle);
            let p1 = center + Vec2::new(a1.cos(), a1.sin()) * track_radius;
            let p2 = center + Vec2::new(a2.cos(), a2.sin()) * track_radius;
            painter.line_segment([p1, p2], Stroke::new(2.0, Color32::from_rgb(40, 44, 52)));
        }

        // Center 0 dB notch tick
        let p_c1 = center + Vec2::new(0.0, -1.0) * (radius - 5.0);
        let p_c2 = center + Vec2::new(0.0, -1.0) * (radius + 2.0);
        painter.line_segment([p_c1, p_c2], Stroke::new(1.5, Color32::from_rgb(90, 95, 110)));

        // Bipolar active track from center_angle (270 deg) to current_angle
        let (arc_start, arc_end) = if current_angle < center_angle {
            (current_angle, center_angle)
        } else {
            (center_angle, current_angle)
        };

        let steps = 12;
        for i in 0..steps {
            let t1 = i as f32 / steps as f32;
            let t2 = (i + 1) as f32 / steps as f32;
            let a1 = arc_start + t1 * (arc_end - arc_start);
            let a2 = arc_start + t2 * (arc_end - arc_start);
            let p1 = center + Vec2::new(a1.cos(), a1.sin()) * track_radius;
            let p2 = center + Vec2::new(a2.cos(), a2.sin()) * track_radius;
            painter.line_segment([p1, p2], Stroke::new(2.5, accent_color));
        }

        // Pointer notch
        let ptr_inner = center + Vec2::new(current_angle.cos(), current_angle.sin()) * (radius * 0.3);
        let ptr_outer = center + Vec2::new(current_angle.cos(), current_angle.sin()) * (radius - 2.0);
        painter.line_segment([ptr_inner, ptr_outer], Stroke::new(2.0, Color32::WHITE));

        // Label
        let label_pos = Pos2::new(center.x, rect.bottom() - 20.0);
        painter.text(
            label_pos,
            nih_plug_egui::egui::Align2::CENTER_CENTER,
            label,
            FontId::proportional(11.0),
            Color32::from_rgb(180, 185, 195),
        );

        // Value text
        let val_str = this_param.to_string();
        let val_pos = Pos2::new(center.x, rect.bottom() - 7.0);
        let val_color = if response.hovered() || response.dragged() {
            accent_color
        } else {
            Color32::from_rgb(220, 225, 235)
        };
        painter.text(
            val_pos,
            nih_plug_egui::egui::Align2::CENTER_CENTER,
            val_str,
            FontId::monospace(10.0),
            val_color,
        );
    }

    response
}

