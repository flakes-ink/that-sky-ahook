//! On-screen log console: a bounded ring buffer fed by `ui::log!`.

use std::collections::VecDeque;
use std::sync::Mutex;

const MAX_LINES: usize = 300;

static LINES: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

/// Append a line to the console (ring buffer, oldest lines dropped).
pub fn push(line: String) {
    let mut lines = LINES.lock().unwrap_or_else(|p| p.into_inner());
    if lines.len() >= MAX_LINES {
        lines.pop_front();
    }
    lines.push_back(line);
}

/// Snapshot of the current lines.
fn snapshot() -> Vec<String> {
    LINES
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .iter()
        .cloned()
        .collect()
}

fn clear() {
    LINES.lock().unwrap_or_else(|p| p.into_inner()).clear();
}

/// Render the Console tab.
pub fn show(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if ui.button("Clear").clicked() {
            clear();
        }
    });

    let lines = snapshot();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(true)
        .show(ui, |ui| {
            if lines.is_empty() {
                ui.label(egui::RichText::new("(empty)").weak());
            }
            for line in &lines {
                ui.monospace(line);
            }
        });
}
