//! Kleiner Arbeits-Pool für Icon- und Thumbnail-Anfragen.
//!
//! Die neueste Anfrage kommt zuerst dran (die Zeilen, die der Benutzer gerade
//! sieht). Ist die Warteschlange voll, wird die älteste verworfen, statt
//! unbegrenzt zu wachsen.

use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread;

use crate::shell::com::ComGuard;

pub struct Task {
    pub run: Box<dyn FnOnce() + Send>,
    /// Wird aufgerufen, wenn die Aufgabe verworfen wird.
    pub reject: Box<dyn FnOnce() + Send>,
}

struct Shared {
    queue: Mutex<Vec<Task>>,
    wake: Condvar,
    max_queue: usize,
}

pub struct Pool {
    shared: Arc<Shared>,
}

impl Pool {
    pub fn new(threads: usize, max_queue: usize) -> Self {
        let shared = Arc::new(Shared {
            queue: Mutex::new(Vec::new()),
            wake: Condvar::new(),
            max_queue,
        });
        for _ in 0..threads {
            let shared = Arc::clone(&shared);
            // Fehler beim Thread-Start: der Pool arbeitet mit weniger Threads weiter.
            let _ = thread::Builder::new()
                .name("shell-images".into())
                .spawn(move || {
                    let _com = ComGuard::sta();
                    loop {
                        let task = {
                            let mut queue =
                                shared.queue.lock().unwrap_or_else(PoisonError::into_inner);
                            loop {
                                if let Some(task) = queue.pop() {
                                    break task;
                                }
                                queue = shared
                                    .wake
                                    .wait(queue)
                                    .unwrap_or_else(PoisonError::into_inner);
                            }
                        };
                        (task.run)();
                    }
                });
        }
        Self { shared }
    }

    pub fn submit(&self, task: Task) {
        let dropped = {
            let mut queue = self
                .shared
                .queue
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let dropped = (queue.len() >= self.shared.max_queue).then(|| queue.remove(0));
            queue.push(task);
            dropped
        };
        self.shared.wake.notify_one();
        if let Some(old) = dropped {
            (old.reject)();
        }
    }
}
