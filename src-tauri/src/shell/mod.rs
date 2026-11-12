//! Windows-Shell-Anbindung. `unsafe` bleibt auf diese Untermodule beschränkt;
//! jeder Block begründet seine Annahmen in einem `SAFETY`-Kommentar.

pub mod clipboard;
pub mod com;
pub mod dragdrop;
pub mod fileop;
pub mod keys;
pub mod places;
