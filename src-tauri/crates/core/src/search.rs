//! Namenssuche in einem Ordner und allen Unterordnern.
//!
//! * Nur Metadaten: Es wird nichts geöffnet oder gelesen, auch OneDrive-
//!   Platzhalter werden dadurch nicht heruntergeladen. Ordner, die nur in der
//!   Cloud liegen, werden nicht betreten (das Auflisten würde sie abrufen).
//! * Symlinks und Junctions werden gemeldet, aber nie betreten.
//! * Breitensuche mit festen Obergrenzen (Treffer, gelesene Einträge, Tiefe)
//!   und jederzeit abbrechbar.
//! * Treffer tragen den Pfad relativ zur Wurzel im Feld `name`
//!   (`unter\ordner\datei.txt`, Trenner immer `\`).

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;

use crate::attrs::is_cloud_placeholder;
use crate::listing::{self, Entry};
use crate::CoreError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Höchstzahl gelieferter Treffer.
    pub max_results: usize,
    /// Höchstzahl gelesener Verzeichniseinträge insgesamt.
    pub max_visited: usize,
    /// Wie viele Ebenen unterhalb der Wurzel durchsucht werden.
    pub max_depth: u32,
}

impl Limits {
    pub const DEFAULT: Self = Self {
        max_results: 5_000,
        max_visited: 400_000,
        max_depth: 32,
    };

    /// Schützt vor übergroßen Werten aus dem Frontend.
    pub fn clamped(self) -> Self {
        Self {
            max_results: self.max_results.clamp(1, 20_000),
            max_visited: self.max_visited.clamp(1, 2_000_000),
            max_depth: self.max_depth.clamp(1, 64),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub found: usize,
    pub visited: usize,
    /// Die Suche wurde wegen einer Obergrenze vorzeitig beendet.
    pub truncated: bool,
    /// Ordner, die nur in der Cloud liegen und nicht betreten wurden.
    pub skipped_cloud: usize,
    /// Ordner, die sich nicht lesen ließen (z. B. kein Zugriff).
    pub unreadable: usize,
}

/// Ein Suchmuster: Teilzeichenfolge, bei `*` oder `?` ein Muster auf den ganzen Namen.
#[derive(Debug, Clone)]
struct Pattern {
    needle: Vec<char>,
    glob: bool,
}

impl Pattern {
    fn new(query: &str) -> Option<Self> {
        let query = query.trim();
        if query.is_empty() {
            return None;
        }
        Some(Self {
            needle: query.to_lowercase().chars().collect(),
            glob: query.contains(['*', '?']),
        })
    }

    fn matches(&self, name: &str) -> bool {
        let name: Vec<char> = name.to_lowercase().chars().collect();
        if self.glob {
            glob(&self.needle, &name)
        } else {
            name.windows(self.needle.len())
                .any(|w| w == self.needle.as_slice())
        }
    }
}

/// `*` (beliebig viele Zeichen) und `?` (genau eins) auf den ganzen Namen.
fn glob(pattern: &[char], text: &[char]) -> bool {
    let (mut p, mut t) = (0usize, 0usize);
    let mut star: Option<(usize, usize)> = None;
    while t < text.len() {
        if p < pattern.len() && (pattern[p] == '?' || pattern[p] == text[t]) {
            p += 1;
            t += 1;
        } else if p < pattern.len() && pattern[p] == '*' {
            star = Some((p, t));
            p += 1;
        } else if let Some((sp, st)) = star {
            p = sp + 1;
            t = st + 1;
            star = Some((sp, st + 1));
        } else {
            return false;
        }
    }
    while p < pattern.len() && pattern[p] == '*' {
        p += 1;
    }
    p == pattern.len()
}

/// Treffer pro gelieferten Stapel.
const BATCH: usize = 200;

/// Durchsucht `root` nach Namen, die `query` enthalten. Treffer gehen in Stapeln
/// an `sink`. Eine leere Suche liefert nichts.
pub fn search(
    root: &Path,
    query: &str,
    limits: &Limits,
    show_hidden: bool,
    cancel: &AtomicBool,
    sink: &mut dyn FnMut(Vec<Entry>),
) -> Result<Summary, CoreError> {
    let limits = limits.clamped();
    let mut summary = Summary::default();
    let Some(pattern) = Pattern::new(query) else {
        return Ok(summary);
    };

    let mut queue: VecDeque<(PathBuf, String, u32)> = VecDeque::new();
    queue.push_back((root.to_path_buf(), String::new(), 0));
    let mut pending: Vec<Entry> = Vec::new();

    'walk: while let Some((dir, rel, depth)) = queue.pop_front() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let mut subdirs: Vec<(PathBuf, String)> = Vec::new();
        let mut stop = false;
        let result = listing::list_dir(&dir, 1024, |batch| {
            for entry in batch {
                summary.visited += 1;
                let hidden = entry.hidden || entry.system;
                if hidden && !show_hidden {
                    continue;
                }
                let relative = if rel.is_empty() {
                    entry.name.clone()
                } else {
                    format!("{rel}\\{}", entry.name)
                };
                if entry.is_dir && !entry.is_link && depth < limits.max_depth {
                    if is_cloud_placeholder(entry.attributes) {
                        summary.skipped_cloud += 1;
                    } else {
                        subdirs.push((dir.join(&entry.name), relative.clone()));
                    }
                }
                if pattern.matches(&entry.name) {
                    summary.found += 1;
                    pending.push(Entry {
                        name: relative,
                        ..entry
                    });
                    if summary.found >= limits.max_results {
                        summary.truncated = true;
                        stop = true;
                        return false;
                    }
                }
                if summary.visited >= limits.max_visited {
                    summary.truncated = true;
                    stop = true;
                    return false;
                }
            }
            !cancel.load(Ordering::Relaxed)
        });
        match result {
            Ok(_) => {}
            // Die Wurzel selbst muss sich lesen lassen; Unterordner dürfen scheitern.
            Err(err) if depth == 0 => return Err(err),
            Err(_) => summary.unreadable += 1,
        }
        if pending.len() >= BATCH || stop {
            sink(std::mem::take(&mut pending));
        }
        if stop {
            break 'walk;
        }
        for (path, relative) in subdirs {
            queue.push_back((path, relative, depth + 1));
        }
        // Treffer zeitnah liefern, auch wenn der Stapel noch klein ist.
        if !pending.is_empty() {
            sink(std::mem::take(&mut pending));
        }
    }
    if !pending.is_empty() {
        sink(pending);
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn run(root: &Path, query: &str, limits: Limits) -> (Vec<String>, Summary) {
        let mut names = Vec::new();
        let summary = search(
            root,
            query,
            &limits,
            false,
            &AtomicBool::new(false),
            &mut |batch| {
                names.extend(batch.into_iter().map(|e| e.name));
            },
        )
        .unwrap();
        names.sort();
        (names, summary)
    }

    fn tree() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("a/b/c")).unwrap();
        fs::create_dir_all(root.join("Berichte/2026")).unwrap();
        fs::write(root.join("Bericht.txt"), b"x").unwrap();
        fs::write(root.join("a/bericht-alt.txt"), b"x").unwrap();
        fs::write(root.join("a/b/c/BERICHT final.docx"), b"x").unwrap();
        fs::write(root.join("Berichte/2026/q1.pdf"), b"x").unwrap();
        fs::write(root.join("a/other.txt"), b"x").unwrap();
        tmp
    }

    #[test]
    fn finds_matches_in_subfolders_with_relative_names() {
        let tmp = tree();
        let (names, summary) = run(tmp.path(), "bericht", Limits::DEFAULT);
        assert_eq!(
            names,
            [
                "Bericht.txt",
                "Berichte",
                "a\\b\\c\\BERICHT final.docx",
                "a\\bericht-alt.txt"
            ]
        );
        assert_eq!(summary.found, 4);
        assert!(!summary.truncated);
        assert!(summary.visited >= 9);
    }

    #[test]
    fn matching_is_case_insensitive_and_supports_wildcards() {
        let tmp = tree();
        let (names, _) = run(tmp.path(), "*.PDF", Limits::DEFAULT);
        assert_eq!(names, ["Berichte\\2026\\q1.pdf"]);
        let (names, _) = run(tmp.path(), "q?.pdf", Limits::DEFAULT);
        assert_eq!(names, ["Berichte\\2026\\q1.pdf"]);
        let (names, _) = run(tmp.path(), "bericht*.txt", Limits::DEFAULT);
        assert_eq!(names, ["Bericht.txt", "a\\bericht-alt.txt"]);
        assert!(run(tmp.path(), "   ", Limits::DEFAULT).0.is_empty());
    }

    #[test]
    fn depth_limit_stops_descending() {
        let tmp = tree();
        let limits = Limits {
            max_depth: 1,
            ..Limits::DEFAULT
        };
        let (names, _) = run(tmp.path(), "bericht", limits);
        assert!(names.contains(&"a\\bericht-alt.txt".to_string()));
        assert!(!names.iter().any(|n| n.contains("BERICHT final")));
    }

    #[test]
    fn result_limit_truncates() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..50 {
            fs::write(tmp.path().join(format!("treffer{i}.txt")), b"x").unwrap();
        }
        let limits = Limits {
            max_results: 10,
            ..Limits::DEFAULT
        };
        let (names, summary) = run(tmp.path(), "treffer", limits);
        assert_eq!(names.len(), 10);
        assert!(summary.truncated);
    }

    #[test]
    fn visited_limit_truncates() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..100 {
            fs::write(tmp.path().join(format!("f{i}.txt")), b"x").unwrap();
        }
        let limits = Limits {
            max_visited: 20,
            ..Limits::DEFAULT
        };
        let (_, summary) = run(tmp.path(), "zzz", limits);
        assert!(summary.truncated);
        assert!(summary.visited <= 100);
    }

    #[test]
    fn cancel_stops_early() {
        let tmp = tree();
        let cancel = AtomicBool::new(true);
        let mut got = 0;
        let summary = search(
            tmp.path(),
            "bericht",
            &Limits::DEFAULT,
            false,
            &cancel,
            &mut |b| got += b.len(),
        )
        .unwrap();
        assert_eq!(got, 0);
        assert_eq!(summary.found, 0);
    }

    #[test]
    fn missing_root_is_an_error() {
        let tmp = tempfile::tempdir().unwrap();
        let err = search(
            &tmp.path().join("nope"),
            "x",
            &Limits::DEFAULT,
            false,
            &AtomicBool::new(false),
            &mut |_| {},
        );
        assert!(err.is_err());
    }

    #[cfg(unix)]
    #[test]
    fn links_are_reported_but_not_followed() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir(tmp.path().join("real")).unwrap();
        fs::write(tmp.path().join("real/datei.txt"), b"x").unwrap();
        std::os::unix::fs::symlink(tmp.path(), tmp.path().join("real/schleife")).unwrap();
        let (names, summary) = run(tmp.path(), "datei", Limits::DEFAULT);
        assert_eq!(names, ["real\\datei.txt"]);
        assert!(!summary.truncated);
        let (names, _) = run(tmp.path(), "schleife", Limits::DEFAULT);
        assert_eq!(names, ["real\\schleife"]);
    }

    #[test]
    fn glob_basics() {
        let g = |p: &str, t: &str| {
            glob(
                &p.chars().collect::<Vec<_>>(),
                &t.chars().collect::<Vec<_>>(),
            )
        };
        assert!(g("*", "x"));
        assert!(g("a*c", "abbbc"));
        assert!(g("a?c", "abc"));
        assert!(!g("a?c", "ac"));
        assert!(g("*.txt", "x.txt"));
        assert!(!g("*.txt", "x.txt.bak"));
        assert!(g("**a", "bba"));
    }
}
