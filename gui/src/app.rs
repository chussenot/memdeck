use std::path::Path;

use eframe::egui;

use crate::audio_engine::{GuiAudioEngine, RenderState};
use crate::ffi::AudioRenderStats;

#[derive(Clone, Copy)]
struct DemoEntry {
    name: &'static str,
    path: Option<&'static str>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FocusArea {
    DemoList,
    Actions,
}

pub struct MemDeckGuiApp {
    audio_engine: GuiAudioEngine,
    demos: Vec<DemoEntry>,
    selected_demo: usize,
    selected_action: usize,
    focus: FocusArea,
    status_line: String,
    last_render: Option<RenderState>,
}

impl Default for MemDeckGuiApp {
    fn default() -> Self {
        let demos = vec![
            DemoEntry {
                name: "Built-in Menu Song",
                path: None,
            },
            DemoEntry {
                name: "dark_moroder",
                path: Some("../data/music/dark_moroder.abc"),
            },
            DemoEntry {
                name: "perturbator_loop",
                path: Some("../data/music/perturbator_loop.abc"),
            },
            DemoEntry {
                name: "carpenter_drive",
                path: Some("../data/music/carpenter_drive.abc"),
            },
            DemoEntry {
                name: "advanced_dsl_demo",
                path: Some("../data/music/advanced_dsl_demo.abc"),
            },
            DemoEntry {
                name: "multi_fx_demo",
                path: Some("../data/music/multi_fx_demo.abc"),
            },
            DemoEntry {
                name: "neon_nightdrive",
                path: Some("../data/music/neon_nightdrive.abc"),
            },
            DemoEntry {
                name: "metro_chase",
                path: Some("../data/music/metro_chase.abc"),
            },
            DemoEntry {
                name: "black_sunrise",
                path: Some("../data/music/black_sunrise.abc"),
            },
            DemoEntry {
                name: "machine_romance",
                path: Some("../data/music/machine_romance.abc"),
            },
            DemoEntry {
                name: "hypersleep_dream",
                path: Some("../data/music/hypersleep_dream.abc"),
            },
        ];

        Self {
            audio_engine: GuiAudioEngine::new(),
            demos,
            selected_demo: 0,
            selected_action: 0,
            focus: FocusArea::DemoList,
            status_line: "Ready. Select a demo and render.".to_string(),
            last_render: None,
        }
    }
}

impl MemDeckGuiApp {
    pub fn configure_visuals(ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(16, 16, 16);
        visuals.window_fill = egui::Color32::from_rgb(8, 8, 8);
        visuals.panel_fill = egui::Color32::from_rgb(10, 10, 10);
        visuals.override_text_color = Some(egui::Color32::from_gray(210));
        visuals.widgets.inactive.bg_fill = egui::Color32::from_gray(18);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_gray(28);
        visuals.widgets.active.bg_fill = egui::Color32::from_gray(42);
        visuals.selection.bg_fill = egui::Color32::from_gray(64);
        visuals.faint_bg_color = egui::Color32::from_gray(22);
        visuals.extreme_bg_color = egui::Color32::from_gray(2);
        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(10.0, 4.0);
        style.visuals.window_stroke = egui::Stroke::new(1.0, egui::Color32::from_gray(72));
        style.visuals.widgets.noninteractive.bg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_gray(54));
        ctx.set_style(style);
    }

    fn selected_demo_entry(&self) -> DemoEntry {
        self.demos[self.selected_demo]
    }

    fn render_selected_demo(&mut self) {
        let selected = self.selected_demo_entry();
        let render_result = match selected.path {
            Some(path) => self.audio_engine.render_abc_file(Path::new(path)),
            None => self.audio_engine.render_builtin_menu(),
        };

        match render_result {
            Ok(render) => {
                let sample_count = render.samples.len();
                self.status_line = format!(
                    "Rendered {} ({} samples @ 22050Hz)",
                    selected.name, sample_count
                );
                self.last_render = Some(render);
            }
            Err(err) => {
                self.status_line = format!("Render failed for {}: {}", selected.name, err);
            }
        }
    }

    fn play_placeholder(&mut self) {
        let selected = self.selected_demo_entry();
        self.status_line = format!(
            "Play placeholder: '{}' ready (audio output binding not implemented in foundation).",
            selected.name
        );
    }

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        ctx.input(|input| {
            if input.key_pressed(egui::Key::Tab) {
                self.focus = match self.focus {
                    FocusArea::DemoList => FocusArea::Actions,
                    FocusArea::Actions => FocusArea::DemoList,
                };
            }

            if input.key_pressed(egui::Key::ArrowUp) {
                match self.focus {
                    FocusArea::DemoList => {
                        if self.selected_demo > 0 {
                            self.selected_demo -= 1;
                        }
                    }
                    FocusArea::Actions => {
                        if self.selected_action > 0 {
                            self.selected_action -= 1;
                        }
                    }
                }
            }

            if input.key_pressed(egui::Key::ArrowDown) {
                match self.focus {
                    FocusArea::DemoList => {
                        if self.selected_demo + 1 < self.demos.len() {
                            self.selected_demo += 1;
                        }
                    }
                    FocusArea::Actions => {
                        if self.selected_action < 1 {
                            self.selected_action += 1;
                        }
                    }
                }
            }

            if input.key_pressed(egui::Key::Enter) {
                match self.focus {
                    FocusArea::DemoList => self.render_selected_demo(),
                    FocusArea::Actions => {
                        if self.selected_action == 0 {
                            self.render_selected_demo();
                        } else {
                            self.play_placeholder();
                        }
                    }
                }
            }

            if input.key_pressed(egui::Key::Space) {
                self.play_placeholder();
            }

            if input.key_pressed(egui::Key::Escape) {
                if self.status_line != "Ready. Select a demo and render." {
                    self.status_line = "Ready. Select a demo and render.".to_string();
                } else {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
        });
    }

    fn draw_stats_panel(ui: &mut egui::Ui, stats: Option<AudioRenderStats>) {
        ui.heading("Render Stats");
        ui.separator();

        if let Some(stats) = stats {
            egui::Grid::new("render_stats_grid")
                .num_columns(2)
                .spacing(egui::vec2(16.0, 4.0))
                .show(ui, |ui| {
                    ui.label("Samples");
                    ui.label(stats.sample_count.to_string());
                    ui.end_row();

                    ui.label("Duration");
                    ui.label(format!("{:.2} ms", stats.duration_ms));
                    ui.end_row();

                    ui.label("Range");
                    ui.label(format!("{}..{}", stats.min_sample, stats.max_sample));
                    ui.end_row();

                    ui.label("Peak");
                    ui.label(stats.peak.to_string());
                    ui.end_row();

                    ui.label("Clipping");
                    ui.label(stats.clipping_count.to_string());
                    ui.end_row();

                    ui.label("Checksum");
                    ui.label(format!("0x{:016x}", stats.checksum));
                    ui.end_row();

                    ui.label("Render time");
                    ui.label(format!("{:.2} ms", stats.render_time_ms));
                    ui.end_row();
                });
        } else {
            ui.label("No render stats yet.");
        }
    }

    fn draw_waveform_placeholder(ui: &mut egui::Ui, samples: Option<&[u8]>) {
        ui.heading("Waveform Preview (placeholder)");
        ui.separator();

        let desired_size = egui::vec2(ui.available_width(), 150.0);
        let (response, painter) = ui.allocate_painter(desired_size, egui::Sense::hover());
        let rect = response.rect;

        painter.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(80)),
        );

        let center_y = rect.center().y;
        painter.line_segment(
            [
                egui::pos2(rect.left() + 4.0, center_y),
                egui::pos2(rect.right() - 4.0, center_y),
            ],
            egui::Stroke::new(1.0, egui::Color32::from_gray(45)),
        );

        if let Some(data) = samples {
            if !data.is_empty() {
                let columns = 96usize;
                let stride = (data.len() / columns).max(1);
                let mut x = rect.left() + 4.0;
                let dx = ((rect.width() - 8.0) / columns as f32).max(1.0);

                for chunk in data.chunks(stride).take(columns) {
                    let mut peak = 0.0_f32;
                    for &sample in chunk {
                        let centered = (sample as f32 - 128.0).abs() / 128.0;
                        if centered > peak {
                            peak = centered;
                        }
                    }
                    let h = peak * (rect.height() * 0.45);
                    painter.line_segment(
                        [egui::pos2(x, center_y - h), egui::pos2(x, center_y + h)],
                        egui::Stroke::new(1.0, egui::Color32::from_gray(180)),
                    );
                    x += dx;
                }
            }
        } else {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No waveform yet",
                egui::FontId::monospace(13.0),
                egui::Color32::from_gray(110),
            );
        }
    }
}

impl eframe::App for MemDeckGuiApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_keyboard(ctx);

        egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("MemDeck GUI Foundation");
                ui.label("| retro synth render front-end (egui/eframe)");
            });
            ui.separator();
            ui.label("Keys: Up/Down navigate • Enter render/activate • Space play placeholder • Tab focus • Esc reset/quit");
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("Status:");
                ui.monospace(&self.status_line);
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.columns(2, |columns| {
                columns[0].group(|ui| {
                    let focus_marker = if self.focus == FocusArea::DemoList {
                        "[FOCUS]"
                    } else {
                        ""
                    };
                    ui.heading(format!("Demo List {focus_marker}"));
                    ui.separator();

                    for (idx, demo) in self.demos.iter().enumerate() {
                        let selected = self.selected_demo == idx;
                        if ui.selectable_label(selected, demo.name).clicked() {
                            self.selected_demo = idx;
                            self.focus = FocusArea::DemoList;
                        }
                    }
                });

                columns[0].add_space(8.0);

                columns[0].group(|ui| {
                    let focus_marker = if self.focus == FocusArea::Actions {
                        "[FOCUS]"
                    } else {
                        ""
                    };
                    ui.heading(format!("Actions {focus_marker}"));
                    ui.separator();

                    let render_selected = self.selected_action == 0;
                    let play_selected = self.selected_action == 1;

                    let render_label = if render_selected { "> Render" } else { "Render" };
                    let play_label = if play_selected { "> Play (placeholder)" } else { "Play (placeholder)" };

                    if ui.button(render_label).clicked() {
                        self.focus = FocusArea::Actions;
                        self.selected_action = 0;
                        self.render_selected_demo();
                    }

                    if ui.button(play_label).clicked() {
                        self.focus = FocusArea::Actions;
                        self.selected_action = 1;
                        self.play_placeholder();
                    }
                });

                columns[1].group(|ui| {
                    let stats = self
                        .last_render
                        .as_ref()
                        .and_then(|render| render.stats)
                        .or_else(|| self.audio_engine.get_render_stats());
                    Self::draw_stats_panel(ui, stats);
                });

                columns[1].add_space(8.0);

                columns[1].group(|ui| {
                    let samples = self.last_render.as_ref().map(|render| render.samples.as_slice());
                    Self::draw_waveform_placeholder(ui, samples);
                });
            });
        });
    }
}
