//! Baum-Scan mit festen Obergrenzen (wie `tree`, aber sicher bei riesigen
//! Strukturen).
//!
//! * Breitensuche: Bei erreichtem Gesamtlimit sind die oberen Ebenen
//!   vollständig, statt dass ein einzelner tiefer Ast alles verbraucht.
//! * Pro Ordner werden nur die ersten `max_per_dir` Einträge (Ordner zuerst,
//!   dann nach Name) behalten; der Rest wird gezählt, nicht gespeichert. Der
//!   Speicherbedarf hängt damit nicht von der Ordnergröße ab.
//! * Symlinks und Junctions werden gemeldet, aber nie betreten.
//! * Der Scan ist jederzeit abbrechbar.

use std::cmp::Ordering;
use std::collections::{BinaryHeap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

use serde::Serialize;

use crate::listing::{self, Entry};
use crate::CoreError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Wie viele Ebenen unterhalb der Wurzel gelesen werden.
    pub max_depth: u32,
    /// Höchstzahl gelieferter Einträge insgesamt.
    pub max_total: usize,
    /// Höchstzahl gelieferter Einträge pro Ordner.
    pub max_per_dir: usize,
}

impl Limits {
    pub const DEFAULT: Self = Self {
        max_depth: 4,
        max_total: 2000,
        max_per_dir: 200,
    };
    pub const MAX_DEPTH: u32 = 16;
    pub const MAX_TOTAL: usize = 50_000;
    pub const MAX_PER_DIR: usize = 5_000;

    /// Erzwingt sinnvolle Grenzen, egal was der Aufrufer schickt.
    pub fn clamped(self) -> Self {
        Self {
            max_depth: self.max_depth.clamp(1, Self::MAX_DEPTH),
            max_total: self.max_total.clamp(1, Self::MAX_TOTAL),
            max_per_dir: self.max_per_dir.clamp(1, Self::MAX_PER_DIR),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Request {
    pub limits: Limits,
    /// Überspringt im Wurzelordner die ersten Einträge („weitere laden“).
    pub offset: usize,
    pub show_hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TreeEntry {
    pub name: String,
    pub is_dir: bool,
    /// Symlink oder Junction: wird nie automatisch betreten.
    pub is_link: bool,
    pub size: u64,
    /// Ordner wurde wegen des Tiefenlimits nicht gelesen.
    pub depth_limited: bool,
}

/// Inhalt eines Ordners. `rel` ist der Pfad relativ zur Scan-Wurzel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirChunk {
    pub rel: Vec<String>,
    pub entries: Vec<TreeEntry>,
    /// Anzahl weiterer, nicht gelieferter Einträge dieses Ordners.
    pub more: usize,
    /// Ordner konnte nicht gelesen werden (`access_denied`, `not_found`, `io`).
    pub error: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    /// Gelieferte Einträge.
    pub count: usize,
    pub cancelled: bool,
    /// Das Gesamtlimit hat den Scan beendet; es gibt ungelesene Ordner.
    pub truncated: bool,
}

struct Keyed {
    key: (bool, String, String),
    entry: Entry,
}

impl Keyed {
    fn new(entry: Entry) -> Self {
        Self {
            key: (!entry.is_dir, entry.name.to_lowercase(), entry.name.clone()),
            entry,
        }
    }
}

impl PartialEq for Keyed {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}
impl Eq for Keyed {}
impl PartialOrd for Keyed {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Keyed {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}

/// Liest einen Ordner und behält nur die Einträge `offset..offset + take` der
/// sortierten Reihenfolge. Liefert zusätzlich die Zahl der übrigen Einträge.
/// `None` bei Abbruch.
fn read_dir_window(
    dir: &Path,
    offset: usize,
    take: usize,
    show_hidden: bool,
    cancel: &AtomicBool,
) -> Result<Option<(Vec<Entry>, usize)>, CoreError> {
    let keep = offset.saturating_add(take);
    let mut heap: BinaryHeap<Keyed> = BinaryHeap::new();
    let mut total = 0usize;

    listing::list_dir(dir, 1024, |batch| {
        for entry in batch {
            if !show_hidden && (entry.hidden || entry.system) {
                continue;
            }
            total += 1;
            heap.push(Keyed::new(entry));
            if heap.len() > keep {
                heap.pop();
            }
        }
        !cancel.load(AtomicOrdering::Relaxed)
    })?;
    if cancel.load(AtomicOrdering::Relaxed) {
        return Ok(None);
    }

    let entries: Vec<Entry> = heap
        .into_sorted_vec()
        .into_iter()
        .skip(offset)
        .map(|k| k.entry)
        .collect();
    let more = total.saturating_sub(offset).saturating_sub(entries.len());
    Ok(Some((entries, more)))
}

fn error_code(err: &CoreError) -> &'static str {
    match err {
        CoreError::AccessDenied => "access_denied",
        CoreError::NotFound | CoreError::NotADirectory => "not_found",
        _ => "io",
    }
}

/// Scannt `root` und liefert pro gelesenem Ordner einen [`DirChunk`].
/// Fehler im Wurzelordner brechen ab; Fehler in Unterordnern werden im
/// jeweiligen Chunk gemeldet.
pub fn scan(
    root: &Path,
    request: &Request,
    cancel: &AtomicBool,
    sink: &mut dyn FnMut(DirChunk),
) -> Result<Summary, CoreError> {
    let limits = request.limits.clamped();
    let mut summary = Summary {
        count: 0,
        cancelled: false,
        truncated: false,
    };
    let mut budget = limits.max_total;
    // (Ordner, relativer Pfad, Ebene seiner Kinder, Offset)
    let mut queue: VecDeque<(PathBuf, Vec<String>, u32, usize)> = VecDeque::new();
    queue.push_back((root.to_path_buf(), Vec::new(), 1, request.offset));

    while let Some((dir, rel, depth, offset)) = queue.pop_front() {
        if cancel.load(AtomicOrdering::Relaxed) {
            summary.cancelled = true;
            break;
        }
        if budget == 0 {
            summary.truncated = true;
            break;
        }
        let take = limits.max_per_dir.min(budget);
        let (entries, more) = match read_dir_window(&dir, offset, take, request.show_hidden, cancel)
        {
            Ok(Some(window)) => window,
            Ok(None) => {
                summary.cancelled = true;
                break;
            }
            Err(err) if rel.is_empty() => return Err(err),
            Err(err) => {
                sink(DirChunk {
                    rel,
                    entries: Vec::new(),
                    more: 0,
                    error: Some(error_code(&err)),
                });
                continue;
            }
        };
        if more > 0 && take < limits.max_per_dir {
            summary.truncated = true;
        }
        budget -= entries.len();
        summary.count += entries.len();

        let mut out = Vec::with_capacity(entries.len());
        for entry in entries {
            let mut depth_limited = false;
            if entry.is_dir && !entry.is_link {
                if depth < limits.max_depth {
                    let mut child_rel = rel.clone();
                    child_rel.push(entry.name.clone());
                    queue.push_back((dir.join(&entry.name), child_rel, depth + 1, 0));
                } else {
                    depth_limited = true;
                }
            }
            out.push(TreeEntry {
                name: entry.name,
                is_dir: entry.is_dir,
                is_link: entry.is_link,
                size: entry.size,
                depth_limited,
            });
        }
        sink(DirChunk {
            rel,
            entries: out,
            more,
            error: None,
        });
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::fs;

    fn req(depth: u32, total: usize, per_dir: usize) -> Request {
        Request {
            limits: Limits {
                max_depth: depth,
                max_total: total,
                max_per_dir: per_dir,
            },
            offset: 0,
            show_hidden: false,
        }
    }

    fn run(root: &Path, request: &Request) -> (HashMap<String, DirChunk>, Summary) {
        let cancel = AtomicBool::new(false);
        let mut chunks = HashMap::new();
        let summary = scan(root, request, &cancel, &mut |c| {
            let key = c.rel.join("/");
            assert!(chunks.insert(key, c).is_none(), "Ordner doppelt geliefert");
        })
        .unwrap();
        (chunks, summary)
    }

    fn names(chunk: &DirChunk) -> Vec<&str> {
        chunk.entries.iter().map(|e| e.name.as_str()).collect()
    }

    /// a/{b/{c/{d/{e/}}}, x.txt}, y.txt, Z.txt
    fn sample() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("a/b/c/d/e")).unwrap();
        fs::write(tmp.path().join("a/x.txt"), b"12345").unwrap();
        fs::write(tmp.path().join("y.txt"), b"1").unwrap();
        fs::write(tmp.path().join("Z.txt"), b"1").unwrap();
        fs::write(tmp.path().join("a/b/c/d/e/deep.txt"), b"1").unwrap();
        tmp
    }

    #[test]
    fn sorts_dirs_first_then_case_insensitive() {
        let tmp = sample();
        let (chunks, summary) = run(tmp.path(), &req(1, 100, 100));
        assert_eq!(names(&chunks[""]), ["a", "y.txt", "Z.txt"]);
        assert_eq!(summary.count, 3);
        assert!(!summary.truncated);
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn depth_limit_marks_unread_dirs() {
        let tmp = sample();
        let (chunks, _) = run(tmp.path(), &req(2, 100, 100));
        assert!(chunks.contains_key("a"));
        assert!(
            !chunks.contains_key("a/b"),
            "Ebene 3 darf nicht gelesen werden"
        );
        let a = &chunks[""].entries[0];
        assert!(!a.depth_limited);
        let b = chunks["a"].entries.iter().find(|e| e.name == "b").unwrap();
        assert!(b.depth_limited);
        assert_eq!(
            chunks["a"]
                .entries
                .iter()
                .find(|e| e.name == "x.txt")
                .unwrap()
                .size,
            5
        );
    }

    #[test]
    fn default_depth_is_four() {
        let tmp = sample();
        let (chunks, _) = run(
            tmp.path(),
            &Request {
                limits: Limits::DEFAULT,
                offset: 0,
                show_hidden: false,
            },
        );
        assert!(chunks.contains_key("a/b/c"));
        assert!(!chunks.contains_key("a/b/c/d"));
        assert!(chunks["a/b/c"].entries[0].depth_limited);
    }

    #[test]
    fn per_dir_limit_counts_the_rest() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..1434 {
            fs::File::create(tmp.path().join(format!("f{i:04}"))).unwrap();
        }
        fs::create_dir(tmp.path().join("zz-dir")).unwrap();
        let (chunks, summary) = run(tmp.path(), &req(4, 2000, 200));
        let root = &chunks[""];
        assert_eq!(root.entries.len(), 200);
        assert_eq!(root.more, 1235);
        assert_eq!(root.entries[0].name, "zz-dir", "Ordner zuerst");
        assert_eq!(root.entries[1].name, "f0000");
        assert_eq!(root.entries[199].name, "f0198");
        assert_eq!(summary.count, 200);
        assert!(
            !summary.truncated,
            "nur das Ordnerlimit, nicht das Gesamtlimit"
        );
    }

    #[test]
    fn offset_loads_the_next_window() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..450 {
            fs::File::create(tmp.path().join(format!("f{i:04}"))).unwrap();
        }
        let mut request = req(1, 2000, 200);
        request.offset = 200;
        let (chunks, _) = run(tmp.path(), &request);
        assert_eq!(chunks[""].entries.len(), 200);
        assert_eq!(chunks[""].entries[0].name, "f0200");
        assert_eq!(chunks[""].more, 50);
        request.offset = 400;
        let (chunks, _) = run(tmp.path(), &request);
        assert_eq!(chunks[""].entries.len(), 50);
        assert_eq!(chunks[""].more, 0);
        request.offset = 9999;
        let (chunks, _) = run(tmp.path(), &request);
        assert!(chunks[""].entries.is_empty());
        assert_eq!(chunks[""].more, 0);
    }

    #[test]
    fn total_limit_stops_and_reports_truncation() {
        let tmp = tempfile::tempdir().unwrap();
        for d in 0..10 {
            let dir = tmp.path().join(format!("d{d}"));
            fs::create_dir(&dir).unwrap();
            for i in 0..50 {
                fs::File::create(dir.join(format!("f{i:02}"))).unwrap();
            }
        }
        let (chunks, summary) = run(tmp.path(), &req(4, 120, 200));
        assert_eq!(summary.count, 120);
        assert!(summary.truncated);
        let delivered: usize = chunks.values().map(|c| c.entries.len()).sum();
        assert_eq!(delivered, 120);
        // Breitensuche: die oberste Ebene ist vollständig.
        assert_eq!(chunks[""].entries.len(), 10);
        // d0 und d1 vollständig, d2 angeschnitten, der Rest ungelesen.
        assert_eq!(chunks["d0"].entries.len(), 50);
        assert_eq!(chunks["d2"].entries.len(), 10);
        assert_eq!(chunks["d2"].more, 40);
        assert!(!chunks.contains_key("d3"));
    }

    #[test]
    fn hidden_entries_are_filtered_unless_requested() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("visible"), b"").unwrap();
        fs::write(tmp.path().join(".hidden"), b"").unwrap();
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("attrib")
                .arg("+h")
                .arg(tmp.path().join(".hidden"))
                .status();
        }
        let (chunks, _) = run(tmp.path(), &req(1, 10, 10));
        assert_eq!(names(&chunks[""]), ["visible"]);
        let mut request = req(1, 10, 10);
        request.show_hidden = true;
        let (chunks, _) = run(tmp.path(), &request);
        assert_eq!(chunks[""].entries.len(), 2);
    }

    /// Ein Link auf den eigenen Elternordner würde bei naivem Folgen endlos laufen.
    #[test]
    fn link_loops_are_not_followed() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("real")).unwrap();
        fs::write(tmp.path().join("real/file"), b"").unwrap();
        let link = tmp.path().join("real/loop");
        #[cfg(unix)]
        std::os::unix::fs::symlink(tmp.path(), &link).unwrap();
        #[cfg(windows)]
        {
            let out = std::process::Command::new("cmd")
                .args(["/C", "mklink", "/J"])
                .arg(&link)
                .arg(tmp.path())
                .output()
                .unwrap();
            assert!(out.status.success());
        }
        let (chunks, summary) = run(tmp.path(), &req(16, 50_000, 5_000));
        assert_eq!(summary.count, 3, "real, real/file, real/loop");
        assert!(!summary.truncated);
        let looped = chunks["real"]
            .entries
            .iter()
            .find(|e| e.name == "loop")
            .unwrap();
        assert!(looped.is_link && looped.is_dir);
        assert!(!looped.depth_limited);
        assert!(!chunks.contains_key("real/loop"));
    }

    #[test]
    fn cancel_stops_the_scan() {
        let tmp = sample();
        let cancel = AtomicBool::new(false);
        let mut seen = 0;
        let summary = scan(tmp.path(), &req(16, 1000, 100), &cancel, &mut |_| {
            seen += 1;
            cancel.store(true, AtomicOrdering::Relaxed);
        })
        .unwrap();
        assert!(summary.cancelled);
        assert_eq!(seen, 1);

        let cancel = AtomicBool::new(true);
        let summary = scan(tmp.path(), &req(4, 100, 100), &cancel, &mut |_| {
            panic!("kein Chunk")
        })
        .unwrap();
        assert!(summary.cancelled);
        assert_eq!(summary.count, 0);
    }

    #[test]
    fn root_errors_fail_but_child_errors_are_reported() {
        let tmp = tempfile::tempdir().unwrap();
        let cancel = AtomicBool::new(false);
        let result = scan(
            &tmp.path().join("nope"),
            &req(1, 1, 1),
            &cancel,
            &mut |_| {},
        );
        assert!(matches!(result, Err(CoreError::NotFound)));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let locked = tmp.path().join("locked");
            fs::create_dir(&locked).unwrap();
            fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
            // Als root greift die Sperre nicht; dann gibt es nichts zu prüfen.
            if fs::read_dir(&locked).is_err() {
                let (chunks, summary) = run(tmp.path(), &req(4, 100, 100));
                assert_eq!(chunks["locked"].error, Some("access_denied"));
                assert_eq!(summary.count, 1);
            }
            fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    #[test]
    fn limits_are_clamped() {
        let wild = Limits {
            max_depth: 0,
            max_total: usize::MAX,
            max_per_dir: 0,
        }
        .clamped();
        assert_eq!(
            wild,
            Limits {
                max_depth: 1,
                max_total: Limits::MAX_TOTAL,
                max_per_dir: 1
            }
        );
        assert_eq!(Limits::DEFAULT.clamped(), Limits::DEFAULT);
        assert_eq!(
            Limits::DEFAULT,
            Limits {
                max_depth: 4,
                max_total: 2000,
                max_per_dir: 200
            }
        );
    }

    /// 100.000 Dateien in einem Ordner: Ergebnis bleibt klein und korrekt gezählt.
    #[test]
    fn huge_directory_stays_bounded() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..100_000 {
            fs::File::create(tmp.path().join(format!("f{i:06}"))).unwrap();
        }
        let (chunks, summary) = run(
            tmp.path(),
            &Request {
                limits: Limits::DEFAULT,
                offset: 0,
                show_hidden: false,
            },
        );
        assert_eq!(chunks[""].entries.len(), 200);
        assert_eq!(chunks[""].more, 99_800);
        assert_eq!(summary.count, 200);
    }
}
