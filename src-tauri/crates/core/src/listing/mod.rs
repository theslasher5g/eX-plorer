//! Auflisten eines einzelnen Ordners – ausschließlich Metadaten.
//!
//! Es wird nie eine Datei geöffnet oder gelesen. Unter Windows kommt alles aus
//! `FindFirstFileExW`; damit werden OneDrive-Platzhalter nicht heruntergeladen.

use std::path::Path;

use serde::Serialize;

use crate::CoreError;

#[cfg(not(windows))]
mod portable;
#[cfg(windows)]
mod win;

/// Ein Verzeichniseintrag, wie er ans Frontend geht.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub name: String,
    pub is_dir: bool,
    /// Größe in Bytes (0 für Ordner).
    pub size: u64,
    /// Änderungszeit in Millisekunden seit der Unix-Epoche.
    pub modified_ms: Option<i64>,
    pub created_ms: Option<i64>,
    /// Rohe Windows-Dateiattribute (0 auf anderen Plattformen).
    pub attributes: u32,
    pub hidden: bool,
    pub system: bool,
    /// Symlink oder Junction. Rekursive Operationen dürfen dem nicht folgen.
    pub is_link: bool,
}

/// Listet `dir` auf und liefert die Einträge in Stapeln von höchstens
/// `batch_size` an `sink`. Gibt `sink` `false` zurück, wird abgebrochen.
/// Rückgabewert ist die Anzahl gelieferter Einträge.
pub fn list_dir(
    dir: &Path,
    batch_size: usize,
    mut sink: impl FnMut(Vec<Entry>) -> bool,
) -> Result<usize, CoreError> {
    let batch_size = batch_size.max(1);
    let mut batch = Vec::with_capacity(batch_size.min(4096));
    let mut total = 0usize;
    let mut stopped = false;

    #[cfg(windows)]
    let iterate = win::for_each_entry;
    #[cfg(not(windows))]
    let iterate = portable::for_each_entry;

    iterate(dir, &mut |entry| {
        batch.push(entry);
        total += 1;
        if batch.len() >= batch_size {
            let full = std::mem::replace(&mut batch, Vec::with_capacity(batch_size.min(4096)));
            if !sink(full) {
                stopped = true;
                return false;
            }
        }
        true
    })?;

    if !stopped && !batch.is_empty() {
        sink(batch);
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn collect(dir: &Path, batch: usize) -> (Vec<Entry>, Vec<usize>) {
        let mut all = Vec::new();
        let mut sizes = Vec::new();
        let total = list_dir(dir, batch, |b| {
            sizes.push(b.len());
            all.extend(b);
            true
        })
        .unwrap();
        assert_eq!(total, all.len());
        all.sort_by(|a, b| a.name.cmp(&b.name));
        (all, sizes)
    }

    #[test]
    fn lists_files_and_dirs_with_metadata() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("sub")).unwrap();
        fs::write(tmp.path().join("a.txt"), b"hello").unwrap();
        fs::write(tmp.path().join("b.bin"), vec![0u8; 1234]).unwrap();

        let (entries, _) = collect(tmp.path(), 100);
        let names: Vec<_> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["a.txt", "b.bin", "sub"]);
        assert!(!entries[0].is_dir);
        assert_eq!(entries[0].size, 5);
        assert_eq!(entries[1].size, 1234);
        assert!(entries[2].is_dir);
        assert_eq!(entries[2].size, 0);
        assert!(entries[0].modified_ms.unwrap() > 1_600_000_000_000);
        assert!(entries.iter().all(|e| !e.is_link));
    }

    #[test]
    fn empty_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let (entries, sizes) = collect(tmp.path(), 10);
        assert!(entries.is_empty());
        assert!(sizes.is_empty());
    }

    #[test]
    fn batches_are_bounded() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..25 {
            fs::write(tmp.path().join(format!("f{i:02}")), b"").unwrap();
        }
        let (entries, sizes) = collect(tmp.path(), 10);
        assert_eq!(entries.len(), 25);
        assert_eq!(sizes, [10, 10, 5]);
    }

    #[test]
    fn sink_can_stop_early() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..25 {
            fs::write(tmp.path().join(format!("f{i:02}")), b"").unwrap();
        }
        let mut calls = 0;
        list_dir(tmp.path(), 10, |_| {
            calls += 1;
            false
        })
        .unwrap();
        assert_eq!(calls, 1);
    }

    #[test]
    fn missing_directory_is_not_found() {
        let tmp = tempfile::tempdir().unwrap();
        let result = list_dir(&tmp.path().join("nope"), 10, |_| true);
        assert!(matches!(result, Err(CoreError::NotFound)));
    }

    #[test]
    fn file_is_not_a_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("a.txt");
        fs::write(&file, b"x").unwrap();
        let result = list_dir(&file, 10, |_| true);
        assert!(matches!(result, Err(CoreError::NotADirectory)));
    }

    #[test]
    fn large_directory() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..20_000 {
            fs::File::create(tmp.path().join(format!("file-{i:05}.dat"))).unwrap();
        }
        let (entries, sizes) = collect(tmp.path(), 2000);
        assert_eq!(entries.len(), 20_000);
        assert!(sizes.iter().all(|&s| s <= 2000));
    }

    /// Ein Link, der auf seinen eigenen Elternordner zeigt (Endlosschleife),
    /// wird als Link gemeldet und nicht verfolgt.
    #[test]
    fn link_loop_is_reported_not_followed() {
        let tmp = tempfile::tempdir().unwrap();
        let link = tmp.path().join("loop");
        #[cfg(unix)]
        std::os::unix::fs::symlink(tmp.path(), &link).unwrap();
        #[cfg(windows)]
        {
            // Junctions brauchen im Gegensatz zu Symlinks keine Sonderrechte.
            let status = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(&link)
                .arg(tmp.path())
                .output()
                .unwrap();
            assert!(status.status.success());
        }
        let (entries, _) = collect(tmp.path(), 10);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "loop");
        assert!(entries[0].is_link);
    }
}
