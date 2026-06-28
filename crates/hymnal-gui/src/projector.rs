//! Projector window lifecycle and display targeting.

use crate::ProjectorWindow;
use log::{info, warn};
use slint::ComponentHandle;

/// A connected display the user can project onto.
#[derive(Debug, Clone)]
pub struct Display {
    pub index: i32,
    pub label: String,
    pub x: i32,
    pub y: i32,
    pub is_primary: bool,
}

/// Enumerate connected displays. Returns at least one (index 0) even on error,
/// so the picker is never empty.
pub fn list_displays() -> Vec<Display> {
    match display_info::DisplayInfo::all() {
        Ok(list) if !list.is_empty() => list
            .into_iter()
            .enumerate()
            .map(|(i, d)| Display {
                index: i as i32,
                label: format!("{} ({}x{})", d.name, d.width, d.height),
                x: d.x,
                y: d.y,
                is_primary: d.is_primary,
            })
            .collect(),
        other => {
            if let Err(e) = other {
                warn!("display enumeration failed: {e}; assuming single display");
            }
            vec![Display {
                index: 0,
                label: "Primary".into(),
                x: 0,
                y: 0,
                is_primary: true,
            }]
        }
    }
}

/// Pick a sensible default output: first non-primary display if present,
/// else primary.
pub fn default_display_index(displays: &[Display]) -> i32 {
    displays
        .iter()
        .find(|d| !d.is_primary)
        .or_else(|| displays.first())
        .map(|d| d.index)
        .unwrap_or(0)
}

/// Create + show a ProjectorWindow positioned on the chosen display, fullscreen.
pub fn open_projector(displays: &[Display], target: i32) -> Option<ProjectorWindow> {
    let win = ProjectorWindow::new().ok()?;
    if let Some(d) = displays.iter().find(|d| d.index == target) {
        info!("opening projector on display {} at ({},{})", d.label, d.x, d.y);
        win.window()
            .set_position(slint::PhysicalPosition::new(d.x, d.y));
    }
    win.show().ok()?;
    win.window().set_fullscreen(true);
    Some(win)
}
