//! Beobachtet geöffnete Ordner und meldet Änderungen gebündelt.
//!
//! Es wird nie rekursiv beobachtet: Pro Ordner reicht ein Watcher auf genau
//! dieser Ebene, das hält Last und Handle-Verbrauch klein. Ereignisse werden
//! kurz gesammelt, damit eine Operation mit hunderten Dateien nur eine
//! Aktualisierung auslöst.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};

/// Wie lange nach dem ersten Ereignis weitere gesammelt werden.
const DEBOUNCE: Duration = Duration::from_millis(250);

fn key(path: &Path) -> String {
    path.to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_lowercase()
}

type Watched = Arc<Mutex<HashMap<String, PathBuf>>>;

pub struct DirWatcher {
    watcher: Option<RecommendedWatcher>,
    watched: Watched,
}

impl DirWatcher {
    /// `on_change` bekommt den Ordner so, wie er in `set_dirs` übergeben wurde.
    /// Der Aufruf kommt aus einem Hintergrund-Thread.
    pub fn new(on_change: impl Fn(&Path) + Send + 'static) -> Self {
        let watched: Watched = Arc::default();
        let (tx, rx) = mpsc::channel::<String>();

        let lookup = Arc::clone(&watched);
        thread::spawn(move || {
            let mut pending: HashSet<String> = HashSet::new();
            loop {
                let timeout = if pending.is_empty() {
                    Duration::from_secs(3600)
                } else {
                    DEBOUNCE
                };
                match rx.recv_timeout(timeout) {
                    Ok(dir) => {
                        pending.insert(dir);
                    }
                    Err(RecvTimeoutError::Timeout) => {
                        let dirs: Vec<PathBuf> = {
                            let map = lookup.lock().unwrap_or_else(PoisonError::into_inner);
                            pending
                                .drain()
                                .filter_map(|k| map.get(&k).cloned())
                                .collect()
                        };
                        for dir in dirs {
                            on_change(&dir);
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
        });

        let map = Arc::clone(&watched);
        let watcher = notify::recommended_watcher(move |event: notify::Result<Event>| {
            let Ok(event) = event else { return };
            if event.kind.is_access() {
                return;
            }
            let map = map.lock().unwrap_or_else(PoisonError::into_inner);
            send_matches(&event, &map, &tx);
        })
        .ok();

        Self { watcher, watched }
    }

    /// Ersetzt die Menge der beobachteten Ordner. Nicht (mehr) vorhandene
    /// Ordner werden stillschweigend übersprungen.
    pub fn set_dirs(&mut self, dirs: &[PathBuf]) {
        let Some(watcher) = self.watcher.as_mut() else {
            return;
        };
        let wanted: HashMap<String, PathBuf> = dirs.iter().map(|d| (key(d), d.clone())).collect();

        let mut current = self.watched.lock().unwrap_or_else(PoisonError::into_inner);
        let stale: Vec<String> = current
            .keys()
            .filter(|k| !wanted.contains_key(*k))
            .cloned()
            .collect();
        for k in stale {
            if let Some(path) = current.remove(&k) {
                let _ = watcher.unwatch(&path);
            }
        }
        for (k, path) in wanted {
            if current.contains_key(&k) {
                continue;
            }
            if watcher.watch(&path, RecursiveMode::NonRecursive).is_ok() {
                current.insert(k, path);
            }
        }
    }
}

fn send_matches(event: &Event, map: &HashMap<String, PathBuf>, tx: &Sender<String>) {
    for path in &event.paths {
        // Ereignis im Ordner (Datei geändert) oder am Ordner selbst.
        let candidates = [path.parent().map(key), Some(key(path))];
        for k in candidates.into_iter().flatten() {
            if map.contains_key(&k) {
                let _ = tx.send(k);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::mpsc::channel;

    fn wait(rx: &mpsc::Receiver<PathBuf>) -> Option<PathBuf> {
        rx.recv_timeout(Duration::from_secs(5)).ok()
    }

    #[test]
    fn reports_changes_in_watched_dir_once() {
        let tmp = tempfile::tempdir().unwrap();
        let (tx, rx) = channel();
        let tx = Mutex::new(tx);
        let mut w = DirWatcher::new(move |p| {
            let _ = tx.lock().unwrap().send(p.to_path_buf());
        });
        w.set_dirs(&[tmp.path().to_path_buf()]);
        thread::sleep(Duration::from_millis(200));

        for i in 0..20 {
            fs::write(tmp.path().join(format!("f{i}")), b"x").unwrap();
        }
        assert_eq!(wait(&rx).as_deref(), Some(tmp.path()));
        // Gebündelt: keine zwanzig einzelnen Meldungen.
        thread::sleep(Duration::from_millis(600));
        let mut extra = 0;
        while rx.try_recv().is_ok() {
            extra += 1;
        }
        assert!(extra <= 2, "zu viele Meldungen: {extra}");
    }

    #[test]
    fn ignores_unwatched_dirs_and_stops_after_removal() {
        let watched = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let (tx, rx) = channel();
        let tx = Mutex::new(tx);
        let mut w = DirWatcher::new(move |p| {
            let _ = tx.lock().unwrap().send(p.to_path_buf());
        });
        w.set_dirs(&[watched.path().to_path_buf()]);
        thread::sleep(Duration::from_millis(200));

        fs::write(other.path().join("x"), b"x").unwrap();
        assert!(rx.recv_timeout(Duration::from_millis(900)).is_err());

        w.set_dirs(&[]);
        fs::write(watched.path().join("y"), b"y").unwrap();
        assert!(rx.recv_timeout(Duration::from_millis(900)).is_err());
    }

    #[test]
    fn missing_dir_is_skipped() {
        let tmp = tempfile::tempdir().unwrap();
        let mut w = DirWatcher::new(|_| {});
        w.set_dirs(&[tmp.path().join("gibt-es-nicht")]);
    }
}
