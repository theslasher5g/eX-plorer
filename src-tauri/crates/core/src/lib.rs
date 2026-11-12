//! Plattformneutraler Kern von eX-plorer.
//!
//! Dieses Crate enthält alles, was sich ohne Tauri und ohne laufendes Windows
//! testen lässt: die Pfadvalidierung (`path`) und das Verzeichnislisting
//! (`listing`). Windows-API-Aufrufe sind in `listing::win` gekapselt.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod error;
pub mod listing;
pub mod ops;
pub mod path;
pub mod watch;
pub mod winerr;

pub use error::CoreError;
