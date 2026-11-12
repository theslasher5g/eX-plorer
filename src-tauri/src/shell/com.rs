//! COM-Initialisierung für Worker-Threads.

use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

/// Initialisiert COM als Single-Threaded-Apartment (von `IFileOperation`
/// verlangt) und räumt beim Drop wieder auf.
pub struct ComGuard {
    initialized: bool,
}

impl ComGuard {
    pub fn sta() -> Self {
        // SAFETY: Aufruf ohne Zeiger; wird im Drop mit CoUninitialize gepaart,
        // aber nur, wenn die Initialisierung erfolgreich war.
        let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok();
        Self { initialized }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.initialized {
            // SAFETY: Paart den erfolgreichen CoInitializeEx-Aufruf dieses Threads.
            unsafe { CoUninitialize() };
        }
    }
}
