//! Welche Dateien beim Öffnen eine Rückfrage brauchen.

/// Dateitypen, die beim Öffnen Code ausführen können. Das Öffnen erfolgt nur
/// auf ausdrückliche Handlung des Benutzers (Doppelklick, Eingabetaste) und
/// bei diesen Typen erst nach Bestätigung.
const RISKY: &[&str] = &[
    "exe",
    "com",
    "scr",
    "pif",
    "bat",
    "cmd",
    "ps1",
    "psm1",
    "psd1",
    "vbs",
    "vbe",
    "js",
    "jse",
    "wsf",
    "wsh",
    "hta",
    "msi",
    "msp",
    "mst",
    "reg",
    "cpl",
    "msc",
    "jar",
    "application",
    "gadget",
    "appx",
    "appxbundle",
    "msix",
    "msixbundle",
    "appinstaller",
    "lnk",
    "url",
    "scf",
    "inf",
    "ps1xml",
    "dll",
    "sys",
    "diagcab",
    "settingcontent-ms",
];

/// Liefert die Endung in Kleinbuchstaben (ohne Punkt). Für `.gitignore` ist
/// das `gitignore`; ohne Punkt gibt es keine Endung.
pub fn extension(name: &str) -> Option<String> {
    let dot = name.rfind('.')?;
    let ext = &name[dot + 1..];
    (!ext.is_empty()).then(|| ext.to_lowercase())
}

pub fn is_risky_to_open(name: &str) -> bool {
    // Windows ignoriert hinten angehängte Leerzeichen und Punkte nicht (der
    // Pfadwächter lehnt sie ab), aber doppelte Endungen wie `a.pdf.exe` zählen
    // nur über die letzte.
    extension(name).is_some_and(|ext| RISKY.contains(&ext.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_executables_case_insensitive() {
        for name in [
            "setup.exe",
            "SETUP.EXE",
            "a.pdf.exe",
            "run.Bat",
            "x.ps1",
            "inst.msi",
            "f.lnk",
            "s.js",
        ] {
            assert!(is_risky_to_open(name), "{name}");
        }
    }

    #[test]
    fn allows_documents() {
        for name in [
            "a.pdf",
            "b.docx",
            "c.txt",
            "README",
            "x.exe.txt",
            "archiv.zip",
            ".gitignore",
            "a.",
        ] {
            assert!(!is_risky_to_open(name), "{name}");
        }
    }

    #[test]
    fn extension_parsing() {
        assert_eq!(extension("a.TXT").as_deref(), Some("txt"));
        assert_eq!(extension(".gitignore").as_deref(), Some("gitignore"));
        assert_eq!(extension("README"), None);
        assert_eq!(extension("a."), None);
    }
}
