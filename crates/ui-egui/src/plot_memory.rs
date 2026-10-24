//! Plot settings remembered between plots (TK fork). The Plot dialog used to start from the
//! defaults every time for model space; now, as in AutoCAD, the settings applied (Apply) or
//! plotted with (OK) come back the next time: per drawing and layout, and for a drawing plotted
//! for the first time, the last model-space settings of any drawing (paper, device, scale,
//! plot style — not its window, which belongs to that drawing). They are kept with the
//! preferences in the host's storage, so they survive restarts.

use std::collections::BTreeMap;
use std::sync::Mutex;

use cadcraft_doc::PageSetup;
use serde::{Deserialize, Serialize};

/// The host storage key for [`prefs_json`].
pub const PREFS_KEY: &str = "tk.plot";

/// Remembered drawings at most (the oldest are dropped).
const MAX_ENTRIES: usize = 200;

/// The dialog settings worth keeping.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Saved {
    pub page: PageSetup,
    /// Plot window corners in drawing units: [x1, y1, x2, y2].
    pub window: [f64; 4],
    pub center: bool,
    pub offset_mm: [f64; 2],
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Memory {
    /// "<drawing>|<layout>" → settings, oldest first in `order`.
    entries: BTreeMap<String, Saved>,
    order: Vec<String>,
    /// The last model-space settings of any drawing.
    last_model: Option<Saved>,
}

static MEMORY: Mutex<Option<Memory>> = Mutex::new(None);

fn key(drawing: &str, target: &str) -> String {
    format!("{drawing}|{target}")
}

fn with<R>(f: impl FnOnce(&mut Memory) -> R) -> Option<R> {
    let mut g = MEMORY.lock().ok()?;
    Some(f(g.get_or_insert_with(Memory::default)))
}

/// The settings to start from for `target` of `drawing` (its path, or its title if unsaved).
/// `extents` is the drawing's model extents, the window when only another drawing's settings
/// are known.
pub fn recall(drawing: &str, target: &str, extents: [f64; 4]) -> Option<Saved> {
    with(|m| {
        if let Some(s) = m.entries.get(&key(drawing, target)) {
            return Some(s.clone());
        }
        if target == "Model" {
            return m.last_model.clone().map(|s| Saved { window: extents, ..s });
        }
        None
    })
    .flatten()
}

/// Remember the settings used for `target` of `drawing`.
pub fn remember(drawing: &str, target: &str, saved: Saved) {
    with(|m| {
        let k = key(drawing, target);
        m.order.retain(|x| *x != k);
        m.order.push(k.clone());
        m.entries.insert(k, saved.clone());
        while m.order.len() > MAX_ENTRIES {
            let old = m.order.remove(0);
            m.entries.remove(&old);
        }
        if target == "Model" {
            m.last_model = Some(saved);
        }
    });
}

/// Everything remembered, as JSON for the host's storage.
pub fn prefs_json() -> String {
    with(|m| serde_json::to_string(m).unwrap_or_default()).unwrap_or_default()
}

/// Restore what [`prefs_json`] saved; anything malformed is ignored.
pub fn load_prefs(json: &str) {
    if json.len() > 4 * 1024 * 1024 {
        return;
    }
    if let Ok(m) = serde_json::from_str::<Memory>(json) {
        let m = Memory { order: m.order.into_iter().filter(|k| m.entries.contains_key(k)).collect(), ..m };
        if let Ok(mut g) = MEMORY.lock() {
            *g = Some(m);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remembers_per_drawing_and_shares_model_paper() {
        let a = Saved {
            page: PageSetup { paper: "ISO A3 (420.00 x 297.00 MM)".into(), plot_area: "window".into(), scale: 0.01, ..PageSetup::default() },
            window: [0.0, 0.0, 100.0, 50.0],
            center: true,
            offset_mm: [0.0, 0.0],
        };
        remember("C:\\a.dxf", "Model", a.clone());
        assert_eq!(recall("C:\\a.dxf", "Model", [0.0; 4]), Some(a.clone()));
        // Another drawing takes the paper and scale but its own extents as the window.
        let b = recall("C:\\b.dxf", "Model", [1.0, 2.0, 3.0, 4.0]).unwrap_or_default();
        assert_eq!((b.page.paper.as_str(), b.window), (a.page.paper.as_str(), [1.0, 2.0, 3.0, 4.0]));
        // Layouts only come back for their own drawing.
        assert_eq!(recall("C:\\b.dxf", "Layout1", [0.0; 4]), None);
        let json = prefs_json();
        load_prefs("not json");
        assert_eq!(prefs_json(), json, "malformed input is ignored");
    }
}
