//! Ausführung blockierender Netzwerkzugriffe mit Zeitlimit.
//!
//! Ein Windows-Aufruf auf einen toten Server lässt sich nicht abbrechen; er
//! kehrt erst nach Sekunden bis Minuten zurück. Deshalb läuft er in einem
//! eigenen Thread, der Aufrufer wartet höchstens `limit` und bekommt sonst
//! sofort [`CoreError::Timeout`]. Damit sich beim erneuten Versuch nicht
//! Dutzende hängende Threads für denselben Server ansammeln, wird ein
//! zeitüberschrittener Schlüssel eine Weile als „verdächtig“ geführt und
//! weitere Anfragen scheitern sofort.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use crate::CoreError;

#[derive(Clone, Default)]
pub struct Guard {
    suspects: Arc<Mutex<HashMap<String, Instant>>>,
}

impl Guard {
    pub fn new() -> Self {
        Self::default()
    }

    fn key(key: &str) -> String {
        key.to_lowercase()
    }

    /// Hängt für `key` noch ein zeitüberschrittener Aufruf?
    pub fn is_suspect(&self, key: &str, cool_down: Duration) -> bool {
        let suspects = self.suspects.lock().unwrap_or_else(PoisonError::into_inner);
        suspects
            .get(&Self::key(key))
            .is_some_and(|since| since.elapsed() < cool_down)
    }

    /// Führt `work` aus. `work` bekommt eine Abbruchmarke und soll sie
    /// zwischen Schritten prüfen; nach einem Timeout wird sie gesetzt.
    pub fn run<T, F>(
        &self,
        key: &str,
        limit: Duration,
        cool_down: Duration,
        work: F,
    ) -> Result<T, CoreError>
    where
        T: Send + 'static,
        F: FnOnce(&AtomicBool) -> Result<T, CoreError> + Send + 'static,
    {
        if self.is_suspect(key, cool_down) {
            return Err(CoreError::Timeout);
        }

        let key = Self::key(key);
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::sync_channel(1);

        let suspects = Arc::clone(&self.suspects);
        let thread_key = key.clone();
        let thread_cancel = Arc::clone(&cancel);
        thread::Builder::new()
            .name("network-op".into())
            .spawn(move || {
                let result = work(&thread_cancel);
                // Sobald der Aufruf zurückkehrt, ist der Server nicht mehr verdächtig.
                suspects
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .remove(&thread_key);
                // Der Empfänger ist nach einem Timeout weg; das Ergebnis verfällt.
                let _ = tx.send(result);
            })
            .map_err(CoreError::from)?;

        match rx.recv_timeout(limit) {
            Ok(result) => result,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                cancel.store(true, Ordering::Relaxed);
                self.suspects
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .insert(key, Instant::now());
                Err(CoreError::Timeout)
            }
            // Der Thread ist abgestürzt.
            Err(mpsc::RecvTimeoutError::Disconnected) => Err(CoreError::Unsupported),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHORT: Duration = Duration::from_millis(150);
    const COOL: Duration = Duration::from_secs(5);

    #[test]
    fn returns_results_and_errors_unchanged() {
        let guard = Guard::new();
        assert_eq!(guard.run("srv", SHORT, COOL, |_| Ok(42)).unwrap(), 42);
        let err = guard
            .run::<(), _>("srv", SHORT, COOL, |_| Err(CoreError::NotFound))
            .unwrap_err();
        assert!(matches!(err, CoreError::NotFound));
        assert!(!guard.is_suspect("srv", COOL));
    }

    #[test]
    fn times_out_and_cancels() {
        let guard = Guard::new();
        let (seen_tx, seen_rx) = mpsc::channel();
        let started = Instant::now();
        let err = guard
            .run::<(), _>("slow", SHORT, COOL, move |cancel| {
                thread::sleep(Duration::from_millis(400));
                let _ = seen_tx.send(cancel.load(Ordering::Relaxed));
                Ok(())
            })
            .unwrap_err();
        assert!(matches!(err, CoreError::Timeout));
        assert!(
            started.elapsed() < Duration::from_millis(350),
            "kehrte nicht rechtzeitig zurück"
        );
        assert!(
            guard.is_suspect("SLOW", COOL),
            "Schlüssel ist unabhängig von der Schreibweise"
        );
        // Die Arbeit sieht die Abbruchmarke und räumt danach den Verdacht ab.
        assert!(seen_rx.recv_timeout(Duration::from_secs(2)).unwrap());
        thread::sleep(Duration::from_millis(50));
        assert!(!guard.is_suspect("slow", COOL));
    }

    #[test]
    fn suspect_keys_fail_fast_without_new_threads() {
        let guard = Guard::new();
        let _ = guard.run::<(), _>("dead", SHORT, COOL, |_| {
            thread::sleep(Duration::from_millis(600));
            Ok(())
        });
        let started = Instant::now();
        let ran = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&ran);
        let err = guard
            .run::<(), _>("dead", Duration::from_secs(5), COOL, move |_| {
                flag.store(true, Ordering::Relaxed);
                Ok(())
            })
            .unwrap_err();
        assert!(matches!(err, CoreError::Timeout));
        assert!(
            started.elapsed() < Duration::from_millis(50),
            "muss sofort scheitern"
        );
        assert!(
            !ran.load(Ordering::Relaxed),
            "es darf kein weiterer Thread starten"
        );
        // Andere Server sind nicht betroffen.
        assert_eq!(guard.run("other", SHORT, COOL, |_| Ok(1)).unwrap(), 1);
    }

    #[test]
    fn works_again_after_cool_down() {
        let guard = Guard::new();
        let short_cool = Duration::from_millis(100);
        let _ = guard.run::<(), _>("k", Duration::from_millis(50), short_cool, |_| {
            thread::sleep(Duration::from_millis(300));
            Ok(())
        });
        thread::sleep(Duration::from_millis(120));
        // Der alte Aufruf läuft noch, aber die Abkühlzeit ist vorbei: ein neuer Versuch ist erlaubt.
        assert_eq!(guard.run("k", SHORT, short_cool, |_| Ok(7)).unwrap(), 7);
    }

    #[test]
    fn panicking_work_is_reported_not_propagated() {
        let guard = Guard::new();
        let err = guard
            .run::<(), _>("p", SHORT, COOL, |_| panic!("boom"))
            .unwrap_err();
        assert!(matches!(err, CoreError::Unsupported));
    }
}
