//! Daten für die Schnellvorschau. Es wird nur gelesen, was der Benutzer
//! ausdrücklich ansieht, und nie der Inhalt von Cloud-Platzhaltern.

use std::fs::File;
use std::io::Read;
use std::os::windows::fs::MetadataExt;

use explorer_core::attrs::{is_cloud_placeholder, FILE_ATTRIBUTE_DIRECTORY};
use explorer_core::path::{validate_path, ValidPath};
use explorer_core::preview::{self, Kind};
use explorer_core::CoreError;
use serde::Serialize;
use tauri::ipc::Response;

use super::blocking;
use crate::error::AppError;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewInfo {
    /// `image`, `pdf`, `text` oder `other`.
    kind: &'static str,
    mime: Option<&'static str>,
    size: u64,
    is_dir: bool,
    /// Nur in der Cloud: eine Vorschau würde den Inhalt herunterladen.
    cloud_only: bool,
    /// Über dem Größenlimit der jeweiligen Vorschauart.
    too_large: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextContent {
    text: String,
    truncated: bool,
}

struct Stat {
    attributes: u32,
    size: u64,
}

fn stat(path: &ValidPath) -> Result<Stat, CoreError> {
    // Nur Metadaten, folgt keinen Links und lädt nichts herunter.
    let meta = std::fs::symlink_metadata(path.to_extended())?;
    Ok(Stat {
        attributes: meta.file_attributes(),
        size: meta.len(),
    })
}

/// Liest höchstens `limit` Bytes. Cloud-Platzhalter und Ordner werden vorher abgelehnt.
fn read_limited(path: &ValidPath, limit: u64) -> Result<(Vec<u8>, u64), CoreError> {
    let info = stat(path)?;
    if info.attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
        return Err(CoreError::NotAFile);
    }
    if is_cloud_placeholder(info.attributes) {
        return Err(CoreError::CloudOnly);
    }
    let mut buffer = Vec::new();
    File::open(path.to_extended())?
        .take(limit)
        .read_to_end(&mut buffer)?;
    Ok((buffer, info.size))
}

#[tauri::command]
pub async fn preview_info(path: String) -> Result<PreviewInfo, AppError> {
    let path = validate_path(&path)?;
    blocking(move || {
        let info = stat(&path)?;
        let is_dir = info.attributes & FILE_ATTRIBUTE_DIRECTORY != 0;
        let kind = if is_dir {
            Kind::Other
        } else {
            preview::kind_for(path.file_name().unwrap_or_default())
        };
        let (name, mime) = match kind {
            Kind::Image { mime } => ("image", Some(mime)),
            Kind::Pdf => ("pdf", Some("application/pdf")),
            Kind::Text => ("text", None),
            Kind::Other => ("other", None),
        };
        Ok(PreviewInfo {
            kind: name,
            mime,
            size: info.size,
            is_dir,
            cloud_only: is_cloud_placeholder(info.attributes),
            too_large: kind.max_bytes().is_some_and(|max| info.size > max),
        })
    })
    .await
}

/// Rohe Bytes für Bild- und PDF-Vorschau (ohne JSON-Umweg).
#[tauri::command]
pub async fn preview_bytes(path: String) -> Result<Response, AppError> {
    let path = validate_path(&path)?;
    blocking(move || {
        let kind = preview::kind_for(path.file_name().unwrap_or_default());
        let Some(limit) = kind.max_bytes().filter(|_| !matches!(kind, Kind::Text)) else {
            return Err(CoreError::Unsupported.into());
        };
        let (bytes, size) = read_limited(&path, limit)?;
        if size > limit {
            return Err(CoreError::TooLarge.into());
        }
        Ok(Response::new(bytes))
    })
    .await
}

#[tauri::command]
pub async fn preview_text(path: String) -> Result<TextContent, AppError> {
    let path = validate_path(&path)?;
    blocking(move || {
        if !matches!(
            preview::kind_for(path.file_name().unwrap_or_default()),
            Kind::Text
        ) {
            return Err(CoreError::Unsupported.into());
        }
        let (bytes, size) = read_limited(&path, preview::MAX_TEXT_BYTES)?;
        let truncated = size > preview::MAX_TEXT_BYTES;
        let decoded = preview::decode_text(&bytes, truncated).ok_or(CoreError::Unsupported)?;
        Ok(TextContent {
            text: decoded.text,
            truncated: decoded.truncated,
        })
    })
    .await
}
