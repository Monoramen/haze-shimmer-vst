use nih_plug::prelude::Editor;
use nih_plug_egui::egui::{self, Color32, RichText, Stroke, StrokeKind, Vec2};
use nih_plug_egui::{EguiState, create_egui_editor};
use std::sync::Arc;

mod knob;
use knob::ParamKnob;

use crate::dsp::envelope::{self, EnvelopeShape};
use crate::params::ShimmerParams;

pub fn default_state() -> Arc<EguiState> {
    EguiState::from_size(610, 520)
}

pub fn create(params: Arc<ShimmerParams>) -> Option<Box<dyn Editor>> {
    let state = params.editor_state.clone();
    create_egui_editor(
        state,
        (),
        |ctx, _| {
            let mut style = (*ctx.style()).clone();
            style.visuals.panel_fill = Color32::from_rgb(20, 20, 24);
            ctx.set_style(style);
        },
        move |ctx, setter, _| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add_space(2.0);
                ui.label(
                    RichText::new("GRANULAR SHIMMER ")
                        .size(16.0)
                        .color(Color32::from_rgb(232, 232, 240))
                        .strong(),
                );
                ui.add_space(6.0);

                ui.columns(4, |cols| {
                    section_with_toggle(
                        &mut cols[0],
                        "GRAIN",
                        &params.shimmer_enabled,
                        setter,
                        |ui| {
                            knob_row(ui, |ui| {
                                ui.add(ParamKnob::new("Size", &params.grain_size_ms, setter));
                                ui.add(ParamKnob::new(
                                    "Pitch",
                                    &params.pitch_shift_semitones,
                                    setter,
                                ));
                            });
                            knob_row(ui, |ui| {
                                ui.add(ParamKnob::new("Density", &params.grain_density, setter));
                                ui.add(ParamKnob::new("Pos Jit", &params.position_jitter, setter));
                            });
                            ui.add_space(4.0);
                            pitch_presets(ui, &params.pitch_shift_semitones, setter);
                        },
                    );

                    section(&mut cols[1], "TAIL", |ui| {
                        knob_row(ui, |ui| {
                            ui.add(ParamKnob::new("Time", &params.tail_time_ms, setter));
                            ui.add(ParamKnob::new("Feedback", &params.tail_feedback, setter));
                        });
                        knob_row(ui, |ui| {
                            ui.add(ParamKnob::new("Diffusion", &params.tail_diffusion, setter));
                            ui.add(ParamKnob::new("Mod", &params.tail_modulation_ms, setter));
                        });
                        ui.add_space(22.0);
                    });

                    section(&mut cols[2], "TONE", |ui| {
                        knob_row(ui, |ui| {
                            ui.add(ParamKnob::new("HPF", &params.tail_hpf, setter));
                            ui.add(ParamKnob::new("LPF", &params.tail_lpf, setter));
                        });
                        knob_row(ui, |ui| {
                            ui.add(ParamKnob::new("Drift", &params.pitch_jitter, setter));
                            ui.add(ParamKnob::new("Spread", &params.tone_spread, setter));
                        });
                        ui.add_space(22.0);
                    });

                    section(&mut cols[3], "OUTPUT", |ui| {
                        knob_row(ui, |ui| {
                            ui.add(ParamKnob::new("Dry/Wet", &params.dry_wet, setter));
                            ui.add(ParamKnob::new("Gain", &params.output_gain, setter));
                        });
                        knob_row(ui, |ui| {
                            ui.add(ParamKnob::new("Width", &params.output_width, setter));
                            ui.add(ParamKnob::new("Tail Mix", &params.tail_mix, setter));
                        });
                        ui.add_space(4.0);
                        //draw_peak_meter(ui, &peak_meter);
                        ui.add_space(18.0);
                    });
                });

                ui.add_space(6.0);

                // Bottom row: GRAIN DELAY (left) + ENV (right).
                ui.columns(2, |cols| {
                    section_with_sync(
                        &mut cols[0],
                        "GRAIN DELAY",
                        &params.grain_delay_sync,
                        setter,
                        |ui| {
                            let sync_on = params.grain_delay_sync.value();
                            knob_row(ui, |ui| {
                                if sync_on {
                                    ui.add(ParamKnob::new(
                                        "Division",
                                        &params.grain_delay_division,
                                        setter,
                                    ));
                                } else {
                                    ui.add(ParamKnob::new(
                                        "Time",
                                        &params.grain_delay_time_ms,
                                        setter,
                                    ));
                                }
                                ui.add(ParamKnob::new(
                                    "Feedback",
                                    &params.grain_delay_feedback,
                                    setter,
                                ));
                                ui.add(ParamKnob::new("Regen", &params.regen, setter));
                                ui.add(ParamKnob::new("Mix", &params.gd_mix, setter));
                            });
                            ui.add_space(16.0);
                            knob_row(ui, |ui| {
                                ui.add(ParamKnob::new("Detune", &params.gd_detune, setter));
                                ui.add(ParamKnob::new("Duck", &params.gd_duck, setter));
                                ui.add(ParamKnob::new("HPF", &params.gd_hpf, setter));
                                ui.add(ParamKnob::new("LPF", &params.gd_lpf, setter));
                            });
                            ui.add_space(4.0);
                            delay_mode_buttons(ui, &params.grain_delay_mode, setter);
                        },
                    );

                    // Right column: ENV CURVE on top, ENV SHAPE below (no separate container).
                    let shape = EnvelopeShape {
                        attack: params.env_attack.value(),
                        hold: params.env_hold.value(),
                        attack_curve: params.env_attack_curve.value(),
                        release_curve: params.env_release_curve.value(),
                    };
                    section(&mut cols[1], "ENV CURVE", |ui| {
                        draw_envelope_curve(ui, shape, 90.0);
                        ui.add_space(4.0);
                        knob_row(ui, |ui| {
                            ui.add(ParamKnob::new("Atk", &params.env_attack, setter));
                            ui.add(ParamKnob::new("Atk Crv", &params.env_attack_curve, setter));
                            ui.add(ParamKnob::new("Hold", &params.env_hold, setter));
                            ui.add(ParamKnob::new("Rel Crv", &params.env_release_curve, setter));
                        });
                        ui.add_space(4.0);
                        envelope_presets(ui, &params, setter);
                    });
                });
            });
        },
    )
}

fn delay_mode_buttons(
    ui: &mut egui::Ui,
    param: &nih_plug::prelude::IntParam,
    setter: &nih_plug::context::gui::ParamSetter,
) {
    const MODES: [(&str, i32); 3] = [("Stereo", 0), ("Ping Pong", 1), ("Mid/Side", 2)];
    let current = param.value();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (label, mode) in MODES {
            let selected = current == mode;
            let (fill, text_color) = if selected {
                (
                    Color32::from_rgb(74, 158, 255),
                    Color32::from_rgb(20, 20, 24),
                )
            } else {
                (
                    Color32::from_rgb(26, 26, 31),
                    Color32::from_rgb(220, 220, 232),
                )
            };
            let btn = egui::Button::new(RichText::new(label).size(10.0).color(text_color))
                .fill(fill)
                .stroke(Stroke::new(1.0, Color32::from_rgb(68, 68, 79)))
                .min_size(Vec2::new(0.0, 18.0));
            if ui.add(btn).clicked() && !selected {
                setter.begin_set_parameter(param);
                setter.set_parameter(param, mode);
                setter.end_set_parameter(param);
            }
        }
    });
}

fn section_with_toggle<R>(
    ui: &mut egui::Ui,
    title: &str,
    param: &nih_plug::prelude::BoolParam,
    setter: &nih_plug::context::gui::ParamSetter,
    content: impl FnOnce(&mut egui::Ui) -> R,
) {
    egui::Frame::group(ui.style())
        .fill(Color32::from_rgb(33, 33, 41))
        .stroke(Stroke::new(1.0, Color32::from_rgb(51, 51, 60)))
        .inner_margin(egui::Margin::same(8))
        .corner_radius(6)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let enabled = param.value();
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(title)
                            .color(Color32::from_rgb(232, 232, 240))
                            .strong()
                            .size(14.0),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (fill, text_color) = if enabled {
                        (
                            Color32::from_rgb(74, 158, 255),
                            Color32::from_rgb(20, 20, 24),
                        )
                    } else {
                        (
                            Color32::from_rgb(26, 26, 31),
                            Color32::from_rgb(140, 140, 160),
                        )
                    };
                    let btn = egui::Button::new(
                        RichText::new(if enabled { "ON" } else { "OFF" })
                            .size(10.0)
                            .color(text_color),
                    )
                    .fill(fill)
                    .stroke(Stroke::new(1.0, Color32::from_rgb(68, 68, 79)))
                    .min_size(Vec2::new(28.0, 16.0));
                    if ui.add(btn).clicked() {
                        setter.begin_set_parameter(param);
                        setter.set_parameter(param, !enabled);
                        setter.end_set_parameter(param);
                    }
                });
            });
            ui.add_space(4.0);
            content(ui);
        });
}

fn section_with_sync<R>(
    ui: &mut egui::Ui,
    title: &str,
    param: &nih_plug::prelude::BoolParam,
    setter: &nih_plug::context::gui::ParamSetter,
    content: impl FnOnce(&mut egui::Ui) -> R,
) {
    egui::Frame::group(ui.style())
        .fill(Color32::from_rgb(33, 33, 41))
        .stroke(Stroke::new(1.0, Color32::from_rgb(51, 51, 60)))
        .inner_margin(egui::Margin::same(8))
        .corner_radius(6)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let enabled = param.value();
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(title)
                            .color(Color32::from_rgb(232, 232, 240))
                            .strong()
                            .size(14.0),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (fill, text_color) = if enabled {
                        (
                            Color32::from_rgb(100, 200, 120),
                            Color32::from_rgb(20, 20, 24),
                        )
                    } else {
                        (
                            Color32::from_rgb(26, 26, 31),
                            Color32::from_rgb(140, 140, 160),
                        )
                    };
                    let btn = egui::Button::new(
                        RichText::new(if enabled { "SYNC" } else { "SYNC" })
                            .size(10.0)
                            .color(text_color),
                    )
                    .fill(fill)
                    .stroke(Stroke::new(1.0, Color32::from_rgb(68, 68, 79)))
                    .min_size(Vec2::new(34.0, 16.0));
                    if ui.add(btn).clicked() {
                        setter.begin_set_parameter(param);
                        setter.set_parameter(param, !enabled);
                        setter.end_set_parameter(param);
                    }
                });
            });
            ui.add_space(4.0);
            content(ui);
        });
}

fn section<R>(ui: &mut egui::Ui, title: &str, content: impl FnOnce(&mut egui::Ui) -> R) {
    egui::Frame::group(ui.style())
        .fill(Color32::from_rgb(33, 33, 41))
        .stroke(Stroke::new(1.0, Color32::from_rgb(51, 51, 60)))
        .inner_margin(egui::Margin::same(8))
        .corner_radius(6)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(title)
                        .color(Color32::from_rgb(232, 232, 240))
                        .strong()
                        .size(14.0),
                );
            });
            ui.add_space(4.0);
            content(ui);
        });
}

fn knob_row<R>(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui) -> R) {
    ui.vertical_centered(|ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            content(ui);
        });
    });
}

fn pitch_presets(
    ui: &mut egui::Ui,
    param: &nih_plug::prelude::FloatParam,
    setter: &nih_plug::context::gui::ParamSetter,
) {
    const PRESETS: [(&str, f32); 5] = [
        ("-12", -12.0),
        ("-6", -6.0),
        ("0", 0.0),
        ("+6", 6.0),
        ("+12", 12.0),
    ];
    let current = param.value();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (label, st) in PRESETS {
            let selected = (current - st).abs() < 0.05;
            let text = RichText::new(label).size(10.0).color(if selected {
                Color32::from_rgb(20, 20, 24)
            } else {
                Color32::from_rgb(220, 220, 232)
            });
            let btn = egui::Button::new(text)
                .fill(if selected {
                    Color32::from_rgb(74, 158, 255)
                } else {
                    Color32::from_rgb(26, 26, 31)
                })
                .stroke(Stroke::new(1.0, Color32::from_rgb(68, 68, 79)))
                .min_size(Vec2::new(0.0, 18.0));
            if ui.add(btn).clicked() && !selected {
                setter.begin_set_parameter(param);
                setter.set_parameter(param, st);
                setter.end_set_parameter(param);
            }
        }
    });
}

fn envelope_presets(
    ui: &mut egui::Ui,
    params: &Arc<ShimmerParams>,
    setter: &nih_plug::context::gui::ParamSetter,
) {
    // (название, attack, hold, attack_curve, release_curve)
    const PRESETS: [(&str, f32, f32, f32, f32); 4] = [
        ("Hann", 0.5, 0.0, 0.0, 0.0),
        ("Tukey", 0.15, 0.7, 0.0, 0.0),
        ("Smooth", 0.4, 0.1, -0.5, 0.5),
        ("Perc", 0.01, 0.0, 0.0, 0.7),
    ];

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for (label, a, h, ac, rc) in PRESETS {
            // Проверяем, совпадает ли текущее состояние с пресетом
            let selected = (params.env_attack.value() - a).abs() < 0.01
                && (params.env_hold.value() - h).abs() < 0.01
                && (params.env_attack_curve.value() - ac).abs() < 0.01
                && (params.env_release_curve.value() - rc).abs() < 0.01;

            let text = RichText::new(label).size(10.0).color(if selected {
                Color32::from_rgb(20, 20, 24)
            } else {
                Color32::from_rgb(220, 220, 232)
            });

            let btn = egui::Button::new(text)
                .fill(if selected {
                    Color32::from_rgb(74, 158, 255)
                } else {
                    Color32::from_rgb(26, 26, 31)
                })
                .stroke(Stroke::new(1.0, Color32::from_rgb(68, 68, 79)))
                .min_size(Vec2::new(44.0, 18.0));

            if ui.add(btn).clicked() && !selected {
                // Пакетное обновление параметров
                let pairs = [
                    (&params.env_attack, a),
                    (&params.env_hold, h),
                    (&params.env_attack_curve, ac),
                    (&params.env_release_curve, rc),
                ];

                for (p, val) in pairs {
                    setter.begin_set_parameter(p);
                    setter.set_parameter(p, val);
                    setter.end_set_parameter(p);
                }
            }
        }
    });
}

fn draw_envelope_curve(ui: &mut egui::Ui, shape: EnvelopeShape, height: f32) {
    let desired = Vec2::new(ui.available_width(), height);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    let painter = ui.painter().with_clip_rect(rect);

    // 1. Фон и рамка
    painter.rect_filled(rect, 4.0, Color32::from_rgb(12, 12, 15));
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0, Color32::from_rgb(50, 50, 60)),
        StrokeKind::Inside,
    );

    let inner_rect = rect.shrink(8.0);

    // 2. Отрисовка фоновой сетки (Grid)
    let grid_stroke = Stroke::new(1.0, Color32::from_rgb(25, 25, 30));
    for i in 1..4 {
        let x = inner_rect.left() + inner_rect.width() * (i as f32 / 4.0);
        painter.line_segment(
            [
                egui::pos2(x, inner_rect.top()),
                egui::pos2(x, inner_rect.bottom()),
            ],
            grid_stroke,
        );
        let y = inner_rect.top() + inner_rect.height() * (i as f32 / 4.0);
        painter.line_segment(
            [
                egui::pos2(inner_rect.left(), y),
                egui::pos2(inner_rect.right(), y),
            ],
            grid_stroke,
        );
    }

    // Рассчитываем точки кривой
    const STEPS: usize = 120;
    let mut pts: Vec<egui::Pos2> = Vec::with_capacity(STEPS + 1);
    for i in 0..=STEPS {
        let phase = i as f32 / STEPS as f32;
        let val = envelope::evaluate(shape, phase);
        let x = inner_rect.left() + phase * inner_rect.width();
        let y = inner_rect.bottom() - val * inner_rect.height();
        pts.push(egui::pos2(x, y));
    }

    // 3. Заливка под кривой (полупрозрачная)
    let mut mesh_pts = pts.clone();
    mesh_pts.push(egui::pos2(inner_rect.right(), inner_rect.bottom()));
    mesh_pts.push(egui::pos2(inner_rect.left(), inner_rect.bottom()));
    painter.add(egui::Shape::convex_polygon(
        mesh_pts,
        Color32::from_rgba_premultiplied(110, 70, 200, 30), // Тускло-фиолетовый
        Stroke::NONE,
    ));

    // 4. Основная линия со свечением
    // Сначала рисуем толстую размытую линию (свечение)
    painter.add(egui::Shape::line(
        pts.clone(),
        Stroke::new(3.0, Color32::from_rgba_premultiplied(110, 70, 200, 100)),
    ));
    // Затем тонкую яркую линию
    painter.add(egui::Shape::line(
        pts,
        Stroke::new(2.0, Color32::from_rgb(180, 150, 255)),
    ));

    // 5. Отмечаем ключевые точки (Узлы)
    // Нам нужно знать фазы: Attack End, Hold End
    let mut a = shape.attack.clamp(0.0, 1.0);
    let mut h = shape.hold.clamp(0.0, 1.0);
    if a + h > 1.0 {
        let sum = a + h;
        a /= sum;
        h /= sum;
    }

    let key_phases = [0.0, a, a + h, 1.0];
    for phase in key_phases {
        let val = envelope::evaluate(shape, phase);
        let x = inner_rect.left() + phase * inner_rect.width();
        let y = inner_rect.bottom() - val * inner_rect.height();

        // Рисуем точку с обводкой
        painter.circle_filled(egui::pos2(x, y), 3.5, Color32::from_rgb(200, 180, 255));
        painter.circle_stroke(
            egui::pos2(x, y),
            4.0,
            Stroke::new(1.0, Color32::from_rgb(20, 20, 25)),
        );
    }
}
