//! GUI module for HRTF Synthesizer
//!
//! Provides an egui-based interface for entering anthropometric measurements
//! and generating personalized HeSuVi WAV files.

mod apo_install;
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

    // APO export status
    apo_export_message: Option<(bool, String)>, // (is_success, message)

    // APO endpoint picker: (index, endpoint_guid, friendly_name)
    apo_endpoints: Vec<(u32, String, String)>,
    apo_selected_endpoint: Option<usize>,

    // APO bypass control (shared memory toggle)
    bypass_control: Option<apo_install::BypassControl>,
    hrtf_enabled: bool,

    // Background APO operation (so UAC + elevated calls don't freeze the GUI)
    apo_pending: Option<std::sync::Arc<std::sync::Mutex<Option<Result<String, String>>>>>,
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
            apo_export_message: None,
            apo_endpoints: Vec::new(),
            apo_selected_endpoint: None,
            bypass_control: apo_install::BypassControl::open(),
            hrtf_enabled: true,
            apo_pending: None,
        }
    }
}

impl eframe::App for HrtfApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check synthesis progress
        self.synth_state.poll();

        // Check background APO operation
        let mut apo_done = false;
        if let Some(ref pending) = self.apo_pending {
            if let Ok(mut guard) = pending.try_lock() {
                if let Some(result) = guard.take() {
                    match result {
                        Ok(msg) => {
                            self.apo_export_message = Some((true, msg));
                            self.apo_endpoints =
                                apo_install::list_audio_endpoints().unwrap_or_default();
                        }
                        Err(msg) => {
                            self.apo_export_message = Some((false, msg));
                        }
                    }
                    apo_done = true;
                }
            }
        }
        if apo_done {
            self.apo_pending = None;
        }

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

                // APO export
                self.show_apo_export(ui);
                ui.add_space(8.0);

                // Setup guide
                self.show_setup_guide(ui);
            });
        });

        // Keep repainting during synthesis or pending APO op
        if self.synth_state.is_running() || self.apo_pending.is_some() {
            ctx.request_repaint();
        }
    }
}

impl HrtfApp {
    /// Spawn a background APO operation so the GUI thread isn't blocked.
    fn spawn_apo_op<F>(&mut self, op: F)
    where
        F: FnOnce() -> Result<String, String> + Send + 'static,
    {
        let result = std::sync::Arc::new(std::sync::Mutex::new(None));
        let result_clone = result.clone();
        std::thread::spawn(move || {
            let r = op();
            *result_clone.lock().unwrap() = Some(r);
        });
        self.apo_pending = Some(result);
        self.apo_export_message = Some((true, "Working... (approve UAC if prompted)".into()));
    }

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
        egui::CollapsingHeader::new("Setup Guide")
            .default_open(false)
            .show(ui, |ui| {
                ui.label("To use personalized HRTF with games:");
                ui.add_space(4.0);
                ui.label("1. Click 'Install APO' and approve the admin prompt");
                ui.label("2. Select your headphone output device");
                ui.label("3. Enter your ear measurements and click 'Export HRTF & Apply'");
                ui.label("4. Set your game audio to 7.1 surround output");
                ui.label("5. All game audio will now be spatialized through your personalized HRTF");
                ui.add_space(4.0);
                ui.label("The APO processes audio at the driver level with ~1ms latency.");
                ui.label("Use the HRTF ON/OFF toggle to compare with and without processing.");
            });
    }

    fn show_apo_export(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new("HRTF APO (System Audio Driver)")
            .default_open(true)
            .show(ui, |ui| {
                let apo_installed = apo_install::is_apo_installed();
                let ir_exists = apo_install::ir_file_exists();
                let busy = self.apo_pending.is_some();

                // ── Step 1: Install / Update APO ──
                ui.label("Step 1: Install the APO driver");
                ui.add_space(2.0);

                if !apo_installed {
                    if ui.add_enabled(!busy, egui::Button::new("Install APO (requires admin)")).clicked() {
                        self.spawn_apo_op(|| {
                            apo_install::register_apo()
                                .map(|()| "APO installed. Select your audio device below.".into())
                        });
                    }
                } else {
                    let dll_needs_update = apo_install::dll_needs_update();
                    if dll_needs_update {
                        ui.horizontal(|ui| {
                            ui.colored_label(egui::Color32::from_rgb(255, 200, 100), "APO installed (update available)");
                            if ui.add_enabled(!busy, egui::Button::new("Update")).clicked() {
                                self.spawn_apo_op(|| {
                                    apo_install::register_apo()
                                        .map(|()| "APO updated. Export HRTF & Apply to activate.".into())
                                });
                            }
                        });
                    } else {
                        ui.colored_label(egui::Color32::from_rgb(100, 200, 100), "APO is installed.");
                    }

                    // ── HRTF on/off toggle ──
                    ui.add_space(4.0);
                    if let Some(ref ctrl) = self.bypass_control {
                        self.hrtf_enabled = !ctrl.is_bypassed();
                    }
                    let prev = self.hrtf_enabled;
                    let label = if self.hrtf_enabled { "HRTF: ON" } else { "HRTF: OFF" };
                    ui.toggle_value(&mut self.hrtf_enabled, label);
                    if self.hrtf_enabled != prev {
                        if let Some(ref ctrl) = self.bypass_control {
                            ctrl.set_bypassed(!self.hrtf_enabled);
                        }
                    }
                }

                // ── Step 2: Select endpoint + associate ──
                ui.add_space(8.0);
                ui.label("Step 2: Select audio endpoint (your headphones)");
                ui.add_space(2.0);

                if self.apo_endpoints.is_empty() {
                    if ui.button("Refresh audio devices").clicked() {
                        self.apo_endpoints = apo_install::list_audio_endpoints().unwrap_or_default();
                    }
                } else if ui.button("Refresh").clicked() {
                    self.apo_endpoints = apo_install::list_audio_endpoints().unwrap_or_default();
                    self.apo_selected_endpoint = None;
                }

                let mut clicked_endpoint: Option<(usize, String, String)> = None;
                for (i, (_idx, guid, name)) in self.apo_endpoints.iter().enumerate() {
                    let selected = self.apo_selected_endpoint == Some(i);
                    let display = format!("{name}  ({guid})");
                    if ui.add_enabled(!busy, egui::SelectableLabel::new(selected, &display)).clicked()
                        && apo_installed
                    {
                        clicked_endpoint = Some((i, guid.clone(), name.clone()));
                    }
                }
                if let Some((i, guid, name)) = clicked_endpoint {
                    self.apo_selected_endpoint = Some(i);
                    self.spawn_apo_op(move || {
                        apo_install::associate_endpoint(&guid)
                            .map(|()| format!("APO associated with: {name}"))
                    });
                }

                // ── Step 3: Export HRTF & Apply ──
                ui.add_space(8.0);
                ui.label("Step 3: Export your personalized HRTF");
                ui.add_space(2.0);

                let can_export = !busy
                    && self.model_error.is_none()
                    && self.model_path.exists()
                    && matches!(self.synth_state, SynthState::Idle | SynthState::Complete { .. });

                if ui
                    .add_enabled(
                        can_export,
                        egui::Button::new("Export HRTF & Apply").min_size(egui::vec2(200.0, 30.0)),
                    )
                    .on_hover_text("Synthesize, export IRs, and restart audio service")
                    .clicked()
                {
                    self.export_for_apo();
                }

                if ir_exists && self.apo_export_message.is_none() {
                    ui.colored_label(
                        egui::Color32::from_rgb(100, 200, 100),
                        "Personalized IRs are active.",
                    );
                }

                // ── Uninstall ──
                if apo_installed {
                    ui.add_space(8.0);
                    if ui.add_enabled(!busy, egui::Button::new("Uninstall APO")).clicked() {
                        self.apo_endpoints.clear();
                        self.apo_selected_endpoint = None;
                        self.spawn_apo_op(|| {
                            apo_install::uninstall_apo()
                                .map(|()| "APO uninstalled. Original audio restored.".into())
                        });
                    }
                }

                // ── Status messages ──
                if let Some((is_success, ref msg)) = self.apo_export_message {
                    ui.add_space(4.0);
                    let color = if is_success {
                        egui::Color32::from_rgb(100, 255, 100)
                    } else {
                        egui::Color32::from_rgb(255, 100, 100)
                    };
                    ui.colored_label(color, msg.as_str());
                }
            });
    }

    fn export_for_apo(&mut self) {
        self.apo_export_message = None;
        self.validation_errors.clear();

        // Parse inputs (same validation as start_synthesis)
        let head_width = match self.head_width.trim().parse::<f32>() {
            Ok(v) if (10.0..=25.0).contains(&v) => v,
            _ => {
                self.apo_export_message = Some((false, "Invalid head width.".into()));
                return;
            }
        };
        let head_depth = match self.head_depth.trim().parse::<f32>() {
            Ok(v) if (15.0..=30.0).contains(&v) => v,
            _ => {
                self.apo_export_message = Some((false, "Invalid head depth.".into()));
                return;
            }
        };

        let mut ear_left = [0.0f32; 6];
        let mut ear_right = [0.0f32; 6];
        for i in 0..6 {
            ear_left[i] = match self.ear_left[i].trim().parse::<f32>() {
                Ok(v) if (0.1..=8.0).contains(&v) => v,
                _ => {
                    self.apo_export_message = Some((false, format!("Invalid left ear param {}", i + 1)));
                    return;
                }
            };
            let src = if self.mirror_ears { &self.ear_left[i] } else { &self.ear_right[i] };
            ear_right[i] = match src.trim().parse::<f32>() {
                Ok(v) if (0.1..=8.0).contains(&v) => v,
                _ => {
                    self.apo_export_message = Some((false, format!("Invalid right ear param {}", i + 1)));
                    return;
                }
            };
        }

        // Synthesize and export
        let config = crate::Config {
            sample_rate: self.sample_rate,
            ..Default::default()
        };

        let synthesizer = match crate::HrtfSynthesizer::load(&self.model_path, config) {
            Ok(s) => s,
            Err(e) => {
                self.apo_export_message = Some((false, format!("Model load failed: {e}")));
                return;
            }
        };

        let anthro = crate::Anthropometry {
            head_width,
            head_depth,
            ear_params_left: ear_left,
            ear_params_right: ear_right,
        };

        let hrtf_data = synthesizer.synthesize(&anthro);
        let speaker_irs = crate::SpeakerIrSet::from_hrtf_data(&hrtf_data);

        // Write to ProgramData
        let program_data = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into());
        let ir_path = std::path::PathBuf::from(program_data).join("HrtfApo").join("hrtf_irs.bin");

        match speaker_irs.save(&ir_path) {
            Ok(()) => {
                // Restart Windows Audio Service so the APO picks up new IRs
                let restart_ok = restart_audio_service();
                let msg = if restart_ok {
                    format!("Exported and audio service restarted. HRTF is now active.")
                } else {
                    format!("Exported to {}. Could not restart audio service — restart manually:\n  net stop audiosrv && net start audiosrv", ir_path.display())
                };
                self.apo_export_message = Some((true, msg));
            }
            Err(e) => {
                self.apo_export_message = Some((false, format!("Export failed: {e}")));
            }
        }
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
/// Embedded model binary (8.8MB, compiled in)
const EMBEDDED_MODEL: &[u8] = include_bytes!(env!("HRTF_MODEL_PATH"));

fn find_model_path() -> Option<PathBuf> {
    // First check common file locations
    let candidates = [
        std::env::current_exe().ok().and_then(|p| {
            p.parent()
                .map(|dir| dir.join("../../models/hrtf_model.bin"))
        }),
        std::env::current_exe().ok().and_then(|p| {
            p.parent()
                .map(|dir| dir.join("../models/hrtf_model.bin"))
        }),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|dir| dir.join("hrtf_model.bin"))),
        Some(PathBuf::from("models/hrtf_model.bin")),
    ];

    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            return Some(candidate);
        }
    }

    // Fall back to extracting the embedded model
    if !EMBEDDED_MODEL.is_empty() {
        let dir = std::env::temp_dir().join("hrtf_synth");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("hrtf_model.bin");
        if std::fs::write(&path, EMBEDDED_MODEL).is_ok() {
            return Some(path);
        }
    }

    None
}

/// Restart the Windows Audio Service so the APO picks up new IRs.
fn restart_audio_service() -> bool {
    use std::process::Command;
    let stop = Command::new("net").args(["stop", "audiosrv"]).output();
    let start = Command::new("net").args(["start", "audiosrv"]).output();
    match (stop, start) {
        (Ok(s), Ok(r)) => s.status.success() && r.status.success(),
        _ => false,
    }
}

