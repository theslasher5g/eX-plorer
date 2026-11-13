//! Windows-Shell-Anbindung. `unsafe` bleibt auf diese Untermodule beschränkt;
//! jeder Block begründet seine Annahmen in einem `SAFETY`-Kommentar.

pub mod clipboard;
pub mod com;
pub mod ctxmenu;
pub mod dragdrop;
pub mod fileop;
pub mod images;
pub mod items;
pub mod keys;
pub mod launch;
pub mod network;
pub mod onedrive;
pub mod places;
pub mod registry;
