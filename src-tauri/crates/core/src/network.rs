//! Netzwerk: Fehlerklassifizierung und Prüfung von Servereingaben.
//! Reine Logik, ohne Netzwerkzugriff.

use serde::Serialize;

use crate::path::{validate_name, PathError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Reachability {
    Online,
    /// Server oder Freigabe antwortet nicht oder existiert nicht.
    Offline,
    /// Erreichbar, aber ohne (gültige) Anmeldung.
    AuthRequired,
}

/// Ordnet einen Windows-Fehlercode einer Erreichbarkeit zu.
/// Für die Wurzel einer Freigabe bedeutet „Zugriff verweigert“ (5): Anmeldung nötig.
pub fn classify(code: u32) -> Reachability {
    match code {
        0 => Reachability::Online,
        // Anmeldung fehlgeschlagen, nicht authentifiziert, Konflikt mit bestehender
        // Sitzung, falsches Kennwort, Konto gesperrt/deaktiviert, Zugriff verweigert.
        5 | 86 | 1219 | 1244 | 1326 | 1327 | 1328 | 1329 | 1330 | 1331 | 1385 | 1909 => {
            Reachability::AuthRequired
        }
        // Netzpfad/Name nicht gefunden, Verbindung weg, Netzwerk/Host nicht erreichbar,
        // Zeitüberschreitung, nicht verbunden, sonstige Netzwerkfehler.
        2 | 3 | 52 | 53 | 55 | 59 | 64 | 65 | 67 | 121 | 1167 | 1203 | 1222 | 1225 | 1226
        | 1229 | 1231 | 1232 | 1235 | 1236 | 2250 => Reachability::Offline,
        _ => Reachability::Offline,
    }
}

/// Ein Server, optional mit Freigabe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerRef {
    host: String,
    share: Option<String>,
}

impl ServerRef {
    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn share(&self) -> Option<&str> {
        self.share.as_deref()
    }

    /// `\\server`
    pub fn server_path(&self) -> String {
        format!(r"\\{}", self.host)
    }

    /// `\\server\freigabe`, wenn eine Freigabe angegeben war.
    pub fn share_path(&self) -> Option<String> {
        self.share.as_ref().map(|s| format!(r"\\{}\{s}", self.host))
    }
}

const MAX_HOST: usize = 255;
const MAX_LABEL: usize = 63;

fn valid_host(host: &str) -> bool {
    if host.is_empty() || host.len() > MAX_HOST || host.starts_with('.') || host.ends_with('.') {
        return false;
    }
    host.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= MAX_LABEL
            && !label.starts_with('-')
            && !label.ends_with('-')
            && label
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    })
}

/// Liest `\\server`, `\\server\freigabe`, `//server/freigabe` oder einen
/// nackten Servernamen. Mehr als eine Freigabe-Ebene ist nicht erlaubt; wer
/// einen Unterordner meint, nutzt die Pfadleiste.
pub fn parse_server(input: &str) -> Result<ServerRef, PathError> {
    let unified = input.trim().replace('/', "\\");
    if unified.is_empty() {
        return Err(PathError::Empty);
    }
    let rest = unified.trim_start_matches('\\');
    if unified.starts_with(r"\\") && unified.starts_with(r"\\\") {
        return Err(PathError::NotAbsolute);
    }
    let mut parts = rest.split('\\').filter(|p| !p.is_empty());
    let host = parts.next().ok_or(PathError::Empty)?;
    if host == "." || host == "?" {
        return Err(PathError::DeviceNamespace);
    }
    if !valid_host(host) {
        return Err(PathError::InvalidCharacter);
    }
    let share = parts.next();
    if let Some(share) = share {
        if share == "." || share == ".." {
            return Err(PathError::ParentTraversal);
        }
        // Verwaltungsfreigaben wie `C$` und `IPC$` sind gültige Namen.
        validate_name(share)?;
    }
    if parts.next().is_some() {
        return Err(PathError::InvalidCharacter);
    }
    Ok(ServerRef {
        host: host.to_owned(),
        share: share.map(str::to_owned),
    })
}

/// Zeigt `path` auf eine Freigabe (`\\server\freigabe…`)? Liefert den Server.
pub fn unc_server(path: &str) -> Option<&str> {
    let rest = path.strip_prefix(r"\\")?;
    let host = rest.split('\\').next()?;
    (!host.is_empty()).then_some(host)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_codes() {
        assert_eq!(classify(0), Reachability::Online);
        for code in [5, 86, 1219, 1244, 1326, 1331] {
            assert_eq!(classify(code), Reachability::AuthRequired, "{code}");
        }
        for code in [53, 64, 67, 121, 1231, 1232, 1225, 9999] {
            assert_eq!(classify(code), Reachability::Offline, "{code}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn io_errors_map_to_network_variants() {
        use crate::CoreError;
        let err = |code| CoreError::from(std::io::Error::from_raw_os_error(code));
        assert!(matches!(err(53), CoreError::Offline));
        assert!(matches!(err(1231), CoreError::Offline));
        assert!(matches!(err(1326), CoreError::AuthRequired));
        assert!(
            matches!(err(5), CoreError::AccessDenied),
            "5 bleibt allgemein"
        );
        assert!(
            matches!(err(2), CoreError::NotFound),
            "2 bleibt „nicht gefunden“"
        );
        assert!(matches!(err(3), CoreError::NotFound));
        assert!(
            matches!(err(9999), CoreError::Io(_)),
            "unbekannte Codes bleiben E/A-Fehler"
        );
        assert!(matches!(err(32), CoreError::Busy));
    }

    #[test]
    fn parses_server_forms() {
        for input in [r"\\nas", "//nas", "nas", r"  \\nas\ ", r"\\nas\"] {
            let s = parse_server(input).unwrap();
            assert_eq!(s.host(), "nas", "{input}");
            assert_eq!(s.share(), None);
            assert_eq!(s.server_path(), r"\\nas");
            assert_eq!(s.share_path(), None);
        }
        let s = parse_server(r"\\srv-buero.firma.local\Buchhaltung").unwrap();
        assert_eq!(
            s.share_path().unwrap(),
            r"\\srv-buero.firma.local\Buchhaltung"
        );
        assert_eq!(parse_server("//10.0.0.5/c$").unwrap().share(), Some("c$"));
        assert_eq!(parse_server(r"\\NAS\IPC$").unwrap().share(), Some("IPC$"));
    }

    #[test]
    fn rejects_bad_server_input() {
        for input in [
            "",
            "   ",
            r"\\",
            r"\\\\nas",
            r"\\.\pipe\x",
            r"\\?\UNC\nas",
            r"\\nas\share\sub",
            r"\\na s",
            r"\\nas\..",
            r"\\nas\a:b",
            r"\\nas\CON",
            "-nas",
            "nas-",
            ".nas",
            "nas..x",
            "n:as",
            r"\\nas\sh*re",
            "C:\\x",
            "a/b/c/d",
        ] {
            assert!(parse_server(input).is_err(), "{input:?}");
        }
        assert!(parse_server(&"a".repeat(64)).is_err(), "Label zu lang");
        assert!(parse_server(&format!("{}.b", "a".repeat(63))).is_ok());
    }

    #[test]
    fn extracts_unc_server() {
        assert_eq!(unc_server(r"\\nas\share\x"), Some("nas"));
        assert_eq!(unc_server(r"\\nas"), Some("nas"));
        assert_eq!(unc_server(r"C:\x"), None);
        assert_eq!(unc_server(r"\\"), None);
    }
}
