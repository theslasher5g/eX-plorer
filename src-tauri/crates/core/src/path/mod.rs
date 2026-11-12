//! Validierung und Normalisierung von Pfaden, die vom Frontend kommen.
//!
//! Die Prüfung ist rein lexikalisch und folgt immer den Windows-Regeln, auch
//! wenn die Tests unter Linux laufen. Ein [`ValidPath`] garantiert:
//!
//! * absolut (`X:\…` oder `\\server\freigabe\…`), keine laufwerksrelativen Pfade
//! * keine `..`-Komponenten (Path Traversal), `.` und doppelte Trenner entfernt
//! * keine Geräte-Namensräume (`\\.\`, `\\?\`, `\??\`)
//! * kein `:` hinter dem Laufwerk (Alternate Data Streams)
//! * keine reservierten Gerätenamen (CON, NUL, COM1 …), auch nicht mit Endung
//! * keine Komponenten, die auf Punkt oder Leerzeichen enden (Win32 würde sie
//!   stillschweigend kürzen und damit eine andere Datei treffen)
//! * begrenzte Länge
//!
//! Für Dateisystemzugriffe wird [`ValidPath::to_extended`] verwendet
//! (`\\?\`-Form). Damit schaltet Windows seine eigene Normalisierung ab, und
//! es wird genau der Pfad angesprochen, der hier geprüft wurde.
//!
//! Reparse-Points (Junctions, Symlinks) werden bewusst nicht aufgelöst: In
//! einen verlinkten Ordner zu navigieren ist erlaubt. Rekursive Operationen
//! müssen Links selbst erkennen (siehe `listing::Entry::is_link`).

mod guard;

pub use guard::{
    validate_name, validate_path, PathError, ValidPath, MAX_NAME_UNITS, MAX_PATH_UNITS,
};
