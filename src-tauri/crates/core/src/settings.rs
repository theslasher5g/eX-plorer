//! Benutzereinstellungen. Alles, was von der Platte oder vom Frontend kommt,
//! läuft durch [`Settings::sanitized`].

use serde::{Deserialize, Serialize};

use crate::tree::Limits;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Versteckte und Systemdateien anzeigen.
    pub show_hidden: bool,
    pub tree_depth: u32,
    pub tree_total: usize,
    pub tree_per_dir: usize,
    /// Kachelbreite in Pixeln.
    pub tile_size: u32,
}

pub const TILE_MIN: u32 = 100;
pub const TILE_MAX: u32 = 220;

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_hidden: false,
            tree_depth: Limits::DEFAULT.max_depth,
            tree_total: Limits::DEFAULT.max_total,
            tree_per_dir: Limits::DEFAULT.max_per_dir,
            tile_size: 132,
        }
    }
}

impl Settings {
    pub fn sanitized(mut self) -> Self {
        let limits = self.tree_limits();
        self.tree_depth = limits.max_depth;
        // Untergrenzen, damit die Ansicht nicht unbrauchbar wird.
        self.tree_total = limits.max_total.max(100);
        self.tree_per_dir = limits.max_per_dir.max(10);
        self.tile_size = self.tile_size.clamp(TILE_MIN, TILE_MAX);
        self
    }

    pub fn tree_limits(&self) -> Limits {
        Limits {
            max_depth: self.tree_depth,
            max_total: self.tree_total,
            max_per_dir: self.tree_per_dir,
        }
        .clamped()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_requirements() {
        let s = Settings::default();
        assert_eq!((s.tree_depth, s.tree_total, s.tree_per_dir), (4, 2000, 200));
        assert!(!s.show_hidden);
        assert_eq!(s.clone().sanitized(), s);
    }

    #[test]
    fn clamps_out_of_range_values() {
        let s = Settings {
            show_hidden: true,
            tree_depth: 999,
            tree_total: 0,
            tree_per_dir: 9_999_999,
            tile_size: 5,
        }
        .sanitized();
        assert_eq!(s.tree_depth, Limits::MAX_DEPTH);
        assert_eq!(s.tree_total, 100);
        assert_eq!(s.tree_per_dir, Limits::MAX_PER_DIR);
        assert_eq!(s.tile_size, TILE_MIN);
        assert!(s.show_hidden);
    }
}
