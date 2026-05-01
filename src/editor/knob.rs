use nih_plug::context::gui::ParamSetter;
use nih_plug::prelude::Param;
use nih_plug_egui::egui::{
    self, Align2, Color32, FontId, Pos2, Response, Sense, Shape, Stroke, Ui, Vec2, Widget,
};
use std::f32::consts::PI;

const ARC_START: f32 = 3.0 * PI / 4.0;
const ARC_SWEEP: f32 = 1.5 * PI;

pub struct ParamKnob<'a, P: Param> {
    param: &'a P,
    setter: &'a ParamSetter<'a>,
    label: &'a str,
    diameter: f32,
}

impl<'a, P: Param> ParamKnob<'a, P> {
    pub fn new(label: &'a str, param: &'a P, setter: &'a ParamSetter<'a>) -> Self {
        Self {
            param,
            setter,
            label,
            diameter: 44.0,
        }
    }
}

impl<'a, P: Param> Widget for ParamKnob<'a, P> {
    fn ui(self, ui: &mut Ui) -> Response {
        let desired = Vec2::new(self.diameter + 16.0, self.diameter + 32.0);
        let (rect, mut response) = ui.allocate_exact_size(desired, Sense::click_and_drag());

        let mut normalized = self.param.modulated_normalized_value();

        let steps = self.param.step_count().unwrap_or(0);

        let discrete_sensitivity = if steps > 0 { 1.0 / steps as f32 } else { 0.0 };

        if response.drag_started() {
            self.setter.begin_set_parameter(self.param);
        }
        if response.dragged() {
            let dy = response.drag_delta().y;
            let sensitivity = if steps > 0 {
                discrete_sensitivity
            } else if ui.input(|i| i.modifiers.shift) {
                0.0008
            } else {
                0.005
            };
            normalized = (normalized - dy * sensitivity).clamp(0.0, 1.0);
            self.setter.set_parameter_normalized(self.param, normalized);
            response.mark_changed();
        }
        if response.drag_stopped() {
            self.setter.end_set_parameter(self.param);
        }

        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if response.hovered() && scroll != 0.0 {
            let sensitivity = if steps > 0 {
                discrete_sensitivity
            } else if ui.input(|i| i.modifiers.shift) {
                0.0001
            } else {
                0.0005
            };
            self.setter.begin_set_parameter(self.param);
            normalized = (normalized + scroll * sensitivity).clamp(0.0, 1.0);
            self.setter.set_parameter_normalized(self.param, normalized);
            self.setter.end_set_parameter(self.param);
            response.mark_changed();
        }

        if response.double_clicked() {
            self.setter.begin_set_parameter(self.param);
            self.setter.set_parameter_normalized(
                self.param,
                self.param.default_normalized_value(),
            );
            self.setter.end_set_parameter(self.param);
        }

        let painter = ui.painter_at(rect);
        let center = Pos2::new(rect.center().x, rect.min.y + 16.0 + self.diameter * 0.5);
        let radius = self.diameter * 0.5;

        painter.text(
            Pos2::new(rect.center().x, rect.min.y + 9.0),
            Align2::CENTER_CENTER,
            self.label,
            FontId::proportional(11.0),
            Color32::from_rgb(200, 200, 216),
        );

        draw_arc(
            &painter,
            center,
            radius * 0.88,
            ARC_START,
            ARC_START + ARC_SWEEP,
            3.0,
            Color32::from_rgb(51, 51, 60),
        );

        let value_angle = ARC_START + ARC_SWEEP * normalized;
        draw_arc(
            &painter,
            center,
            radius * 0.88,
            ARC_START,
            value_angle,
            3.0,
            Color32::from_rgb(74, 158, 255),
        );

        painter.circle_filled(center, radius * 0.68, Color32::from_rgb(26, 26, 31));
        painter.circle_stroke(
            center,
            radius * 0.68,
            Stroke::new(1.0, Color32::from_rgb(68, 68, 79)),
        );

        let inner = Pos2::new(
            center.x + (radius * 0.2) * value_angle.cos(),
            center.y + (radius * 0.2) * value_angle.sin(),
        );
        let outer = Pos2::new(
            center.x + (radius * 0.62) * value_angle.cos(),
            center.y + (radius * 0.62) * value_angle.sin(),
        );
        painter.line_segment(
            [inner, outer],
            Stroke::new(2.0, Color32::from_rgb(232, 232, 240)),
        );

        let value_str = self
            .param
            .normalized_value_to_string(self.param.modulated_normalized_value(), true);
        painter.text(
            Pos2::new(rect.center().x, rect.max.y - 7.0),
            Align2::CENTER_CENTER,
            value_str,
            FontId::proportional(10.0),
            Color32::from_rgb(200, 200, 210),
        );

        response.on_hover_cursor(egui::CursorIcon::ResizeVertical)
    }
}

fn draw_arc(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    start: f32,
    end: f32,
    stroke_width: f32,
    color: Color32,
) {
    let segments = 48;
    let step = (end - start) / segments as f32;
    let points: Vec<Pos2> = (0..=segments)
        .map(|i| {
            let a = start + step * i as f32;
            Pos2::new(center.x + radius * a.cos(), center.y + radius * a.sin())
        })
        .collect();
    painter.add(Shape::line(points, Stroke::new(stroke_width, color)));
}
