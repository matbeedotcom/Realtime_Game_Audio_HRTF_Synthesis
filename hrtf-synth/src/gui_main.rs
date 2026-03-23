//! HRTF Synthesizer GUI
//!
//! Graphical interface for generating personalized HRTFs from anthropometric measurements.

#![windows_subsystem = "windows"]

use hrtf_synth::gui::HrtfApp;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([620.0, 780.0])
            .with_min_inner_size([500.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "HRTF Synthesizer",
        options,
        Box::new(|cc| Ok(Box::new(HrtfApp::new(cc)))),
    )
}
