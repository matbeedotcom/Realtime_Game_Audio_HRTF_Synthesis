//! GUI module for HRTF Synthesizer
//!
//! Provides an egui-based interface for entering anthropometric measurements
//! and generating personalized HeSuVi WAV files.

mod synthesis;

use std::path::PathBuf;

use eframe::egui;

use synthesis::SynthState;

const EAR_PARAM_LABELS: [(&str, &str); 6] = [
    ("d1", "Cavum concha height (cm)"),
    ("d2", "Cymba concha height (cm)"),
    ("d3", "Cavum concha width (cm)"),
    ("d5", "Pinna height (cm)"),
    ("d7", "Pinna width (cm)"),
    ("d8", "Ear canal to rear of pinna (cm)"),
];

/// Main application state
pub struct HrtfApp {
    // Head measurements
    head_width: String,
    head_depth: String,

    // Ear parameters (as strings for text input)
    ear_left: [String; 6],
    ear_right: [String; 6],
    mirror_ears: bool,

    // Output settings
    sample_rate: u32,
    output_path: Option<PathBuf>,

    // Synthesis
    synth_state: SynthState,
    model_path: PathBuf,
    model_error: Option<String>,

    // Validation errors
    validation_errors: Vec<String>,
}

impl HrtfApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self::default();

        // Resolve model path
        app.model_path = find_model_path().unwrap_or_default();
        if !app.model_path.exists() {
            app.model_error = Some("Model file not found. Use Browse to locate hrtf_model.bin.".into());
        }

        app
    }
}

impl Default for HrtfApp {
    fn default() -> Self {
        Self {
            head_width: "14.5".into(),
            head_depth: "19.5".into(),
            // Average ear parameters from CIPIC database (cm):
            // d1=cavum concha height, d2=cymba concha height, d3=cavum concha width,
            // d5=pinna height, d7=pinna width, d8=ear canal to rear of pinna
            ear_left: [
                "0.9".into(),  // d1
                "0.5".into(),  // d2
                "0.8".into(),  // d3
                "3.0".into(),  // d5
                "1.5".into(),  // d7
                "1.6".into(),  // d8
            ],
            ear_right: [
                "0.9".into(),  // d1
                "0.5".into(),  // d2
                "0.8".into(),  // d3
                "3.0".into(),  // d5
                "1.5".into(),  // d7
                "1.6".into(),  // d8
            ],
            mirror_ears: true,
            sample_rate: 48000,
            output_path: None,
            synth_state: SynthState::Idle,
            model_path: PathBuf::new(),
            model_error: None,
            validation_errors: Vec::new(),
        }
    }
}

impl eframe::App for HrtfApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check synthesis progress
        self.synth_state.poll();

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("HRTF Synthesizer");
                ui.label("Generate personalized spatial audio filters for gaming headphones.");
                ui.add_space(8.0);

                // Model path
                self.show_model_section(ui);
                ui.add_space(4.0);

                // Head measurements
                self.show_head_section(ui);
                ui.add_space(4.0);

                // Ear measurements
                self.show_ear_section(ui);
                ui.add_space(4.0);

                // Output settings
                self.show_output_section(ui);
                ui.add_space(8.0);

                // Synthesize button + progress
                self.show_synth_section(ui);
                ui.add_space(8.0);

                // Setup guide
                self.show_setup_guide(ui);
            });
        });

        // Keep repainting during synthesis
        if self.synth_state.is_running() {
            ctx.request_repaint();
        }
    }
}

impl HrtfApp {
    fn show_model_section(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Model File")
            .default_open(self.model_error.is_some())
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let path_text = if self.model_path.as_os_str().is_empty() {
                        "No model selected".to_string()
                    } else {
                        self.model_path.display().to_string()
                    };
                    ui.label(&path_text);

                    if ui.button("Browse...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Model", &["bin"])
                            .set_title("Select HRTF model file")
                            .pick_file()
                        {
                            self.model_path = path;
                            self.model_error = None;
                        }
                    }
                });

                if let Some(err) = &self.model_error {
                    ui.colored_label(egui::Color32::from_rgb(255, 100, 100), err);
                }
            });
    }

    fn show_head_section(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Head Measurements")
            .default_open(true)
            .show(ui, |ui| {
                egui::Grid::new("head_grid")
                    .num_columns(2)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Head width (cm):")
                            .on_hover_text("Distance between left and right ear canal entrances");
                        ui.add(egui::TextEdit::singleline(&mut self.head_width).desired_width(80.0));
                        ui.end_row();

                        ui.label("Head depth (cm):")
                            .on_hover_text("Distance from front of head to back");
                        ui.add(egui::TextEdit::singleline(&mut self.head_depth).desired_width(80.0));
                        ui.end_row();
                    });
            });
    }

    fn show_ear_section(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Left Ear Parameters")
            .default_open(true)
            .show(ui, |ui| {
                Self::show_ear_grid(ui, "ear_left_grid", &mut self.ear_left, false);
            });

        ui.add_space(2.0);
        ui.checkbox(&mut self.mirror_ears, "Mirror left ear to right ear");

        if self.mirror_ears {
            self.ear_right = self.ear_left.clone();
        }

        egui::CollapsingHeader::new("Right Ear Parameters")
            .default_open(!self.mirror_ears)
            .show(ui, |ui| {
                if self.mirror_ears {
                    ui.label("(Mirrored from left ear)");
                } else {
                    Self::show_ear_grid(ui, "ear_right_grid", &mut self.ear_right, false);
                }
            });
    }

    fn show_ear_grid(ui: &mut egui::Ui, id: &str, params: &mut [String; 6], disabled: bool) {
        egui::Grid::new(id)
            .num_columns(2)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                for (i, (label, tooltip)) in EAR_PARAM_LABELS.iter().enumerate() {
                    ui.label(format!("{label}:")).on_hover_text(*tooltip);
                    ui.add_enabled(
                        !disabled,
                        egui::TextEdit::singleline(&mut params[i]).desired_width(80.0),
                    );
                    ui.end_row();
                }
            });
    }

    fn show_output_section(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("Output Settings")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Sample rate:");
                    ui.radio_value(&mut self.sample_rate, 44100, "44100 Hz");
                    ui.radio_value(&mut self.sample_rate, 48000, "48000 Hz");
                    ui.radio_value(&mut self.sample_rate, 96000, "96000 Hz");
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label("Output file:");
                    let path_text = self
                        .output_path
                        .as_ref()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "Not selected".into());
                    ui.label(&path_text);

                    if ui.button("Browse...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("WAV", &["wav"])
                            .set_title("Save HeSuVi WAV file")
                            .set_file_name("personalized_hrtf.wav")
                            .save_file()
                        {
                            self.output_path = Some(path);
                        }
                    }
                });
            });
    }

    fn show_synth_section(&mut self, ui: &mut egui::Ui) {
        // Validation errors
        for err in &self.validation_errors {
            ui.colored_label(egui::Color32::from_rgb(255, 100, 100), err);
        }

        match &self.synth_state {
            SynthState::Idle => {
                let can_run = self.model_error.is_none()
                    && self.model_path.exists()
                    && self.output_path.is_some();

                if ui
                    .add_enabled(can_run, egui::Button::new("Synthesize").min_size(egui::vec2(200.0, 36.0)))
                    .clicked()
                {
                    self.start_synthesis();
                }
            }
            SynthState::Running { progress, total, .. } => {
                let current = progress.load(std::sync::atomic::Ordering::Relaxed);
                let frac = current as f32 / *total as f32;
                ui.add(
                    egui::ProgressBar::new(frac)
                        .text(format!("Synthesizing... {current}/{total} directions"))
                        .animate(true),
                );
            }
            SynthState::Complete { message } => {
                ui.colored_label(egui::Color32::from_rgb(100, 255, 100), message);
                if ui.button("Synthesize Again").clicked() {
                    self.synth_state = SynthState::Idle;
                }
            }
            SynthState::Error { message } => {
                ui.colored_label(egui::Color32::from_rgb(255, 100, 100), format!("Error: {message}"));
                if ui.button("Try Again").clicked() {
                    self.synth_state = SynthState::Idle;
                }
            }
        }
    }

    fn show_setup_guide(&self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("HeSuVi Setup Guide")
            .default_open(false)
            .show(ui, |ui| {
                ui.label("To use your personalized HRTF with games:");
                ui.add_space(4.0);
                ui.label("1. Install EqualizerAPO (equalizer-apo.sourceforge.net)");
                ui.label("2. Install HeSuVi (sourceforge.net/projects/hesuvi/)");
                ui.label("3. Copy the generated .wav file to:");
                ui.label("   C:\\Program Files\\EqualizerAPO\\config\\HeSuVi\\hrir\\");
                ui.label("4. Open HeSuVi and select your file from the HRIR dropdown");
                ui.label("5. Enable virtualization — all game audio will now use your personalized HRTF");
                ui.add_space(4.0);
                ui.label("Tip: Set your game audio output to 7.1 surround for best results.");
            });
    }

    fn start_synthesis(&mut self) {
        self.validation_errors.clear();

        // Parse and validate inputs
        let head_width = match self.head_width.trim().parse::<f32>() {
            Ok(v) if (10.0..=25.0).contains(&v) => v,
            Ok(_) => {
                self.validation_errors.push("Head width should be 10-25 cm.".into());
                return;
            }
            Err(_) => {
                self.validation_errors.push("Invalid head width value.".into());
                return;
            }
        };

        let head_depth = match self.head_depth.trim().parse::<f32>() {
            Ok(v) if (15.0..=30.0).contains(&v) => v,
            Ok(_) => {
                self.validation_errors.push("Head depth should be 15-30 cm.".into());
                return;
            }
            Err(_) => {
                self.validation_errors.push("Invalid head depth value.".into());
                return;
            }
        };

        let mut ear_left = [0.0f32; 6];
        let mut ear_right = [0.0f32; 6];

        for (i, (label, _)) in EAR_PARAM_LABELS.iter().enumerate() {
            match self.ear_left[i].trim().parse::<f32>() {
                Ok(v) if (0.1..=8.0).contains(&v) => ear_left[i] = v,
                Ok(_) => {
                    self.validation_errors
                        .push(format!("Left ear {label} out of range (0.1-8.0 cm)."));
                    return;
                }
                Err(_) => {
                    self.validation_errors
                        .push(format!("Invalid left ear {label} value."));
                    return;
                }
            }

            let right_src = if self.mirror_ears {
                &self.ear_left[i]
            } else {
                &self.ear_right[i]
            };
            match right_src.trim().parse::<f32>() {
                Ok(v) if (0.1..=8.0).contains(&v) => ear_right[i] = v,
                Ok(_) => {
                    self.validation_errors
                        .push(format!("Right ear {label} out of range (0.1-8.0 cm)."));
                    return;
                }
                Err(_) => {
                    self.validation_errors
                        .push(format!("Invalid right ear {label} value."));
                    return;
                }
            }
        }

        let output_path = self.output_path.clone().unwrap();

        self.synth_state = synthesis::start(
            self.model_path.clone(),
            self.sample_rate,
            head_width,
            head_depth,
            ear_left,
            ear_right,
            output_path,
        );
    }
}

/// Search for the model file in common locations
fn find_model_path() -> Option<PathBuf> {
    let candidates = [
        // Relative to executable
        std::env::current_exe().ok().and_then(|p| {
            p.parent()
                .map(|dir| dir.join("../models/hrtf_model.bin"))
        }),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|dir| dir.join("hrtf_model.bin"))),
        // Relative to CWD
        Some(PathBuf::from("models/hrtf_model.bin")),
        Some(PathBuf::from("../models/hrtf_model.bin")),
    ];

    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}
