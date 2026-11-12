//! Fallback ohne Windows-API. Dient nur dazu, die Kernlogik auf anderen
//! Plattformen testen zu können.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use super::Entry;
use crate::CoreError;

fn to_ms(time: std::io::Result<SystemTime>) -> Option<i64> {
    let since = time.ok()?.duration_since(UNIX_EPOCH).ok()?;
    i64::try_from(since.as_millis()).ok()
}

pub(super) fn for_each_entry(
    dir: &Path,
    visit: &mut dyn FnMut(Entry) -> bool,
) -> Result<(), CoreError> {
    for item in fs::read_dir(dir)? {
        let item = item?;
        // symlink_metadata folgt Links nicht.
        let Ok(meta) = fs::symlink_metadata(item.path()) else {
            continue;
        };
        let name = item.file_name().to_string_lossy().into_owned();
        let is_link = meta.file_type().is_symlink();
        let is_dir = meta.is_dir() || (is_link && item.path().is_dir());
        let entry = Entry {
            hidden: name.starts_with('.'),
            name,
            is_dir,
            size: if is_dir { 0 } else { meta.len() },
            modified_ms: to_ms(meta.modified()),
            created_ms: to_ms(meta.created()),
            attributes: 0,
            system: false,
            is_link,
        };
        if !visit(entry) {
            break;
        }
    }
    Ok(())
}
