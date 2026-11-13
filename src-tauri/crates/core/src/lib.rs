//! Plattformneutraler Kern von eX-plorer.
//!
//! Dieses Crate enthält alles, was sich ohne Tauri und ohne laufendes Windows
//! testen lässt: die Pfadvalidierung (`path`) und das Verzeichnislisting
//! (`listing`). Windows-API-Aufrufe sind in `listing::win` gekapselt.
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod attrs;
pub mod error;
pub mod exec;
pub mod imaging;
pub mod listing;
pub mod onedrive;
pub mod ops;
pub mod path;
pub mod preview;
pub mod settings;
pub mod tree;
pub mod watch;
pub mod winerr;

pub use error::CoreError;
