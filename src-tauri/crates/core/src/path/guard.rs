use std::path::PathBuf;

/// Maximale Pfadlänge in UTF-16-Einheiten (Windows erlaubt 32.767 inkl. Präfix).
pub const MAX_PATH_UNITS: usize = 32_000;
/// Maximale Länge einer einzelnen Komponente in UTF-16-Einheiten.
pub const MAX_NAME_UNITS: usize = 255;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PathError {
    #[error("path is empty")]
    Empty,
    #[error("path is not absolute")]
    NotAbsolute,
    #[error("path contains a parent component (..)")]
    ParentTraversal,
    #[error("device namespace paths are not allowed")]
    DeviceNamespace,
    #[error("alternate data streams are not allowed")]
    AlternateDataStream,
    #[error("reserved device name")]
    ReservedName,
    #[error("invalid character in path")]
    InvalidCharacter,
    #[error("component ends with a dot or space")]
    TrailingDotOrSpace,
    #[error("path or name is too long")]
    TooLong,
    #[error("UNC path is missing the share name")]
    MissingShare,
}

/// Ein geprüfter, normalisierter, absoluter Windows-Pfad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidPath {
    /// `C:\a\b` bzw. `\\server\share\a`; Laufwerkswurzel mit, sonst ohne
    /// abschließenden Backslash.
    normalized: String,
    /// Länge der Wurzel (`C:\` bzw. `\\server\share`) in Bytes.
    root_len: usize,
}

impl ValidPath {
    pub fn as_str(&self) -> &str {
        &self.normalized
    }

    pub fn into_string(self) -> String {
        self.normalized
    }

    pub fn is_unc(&self) -> bool {
        self.normalized.starts_with(r"\\")
    }

    pub fn is_root(&self) -> bool {
        self.normalized.len() == self.root_len
    }

    /// Erweiterte Form (`\\?\C:\…`, `\\?\UNC\server\share\…`) für Win32-Aufrufe.
    pub fn to_extended(&self) -> PathBuf {
        let mut s = String::with_capacity(self.normalized.len() + 8);
        if let Some(rest) = self.normalized.strip_prefix(r"\\") {
            s.push_str(r"\\?\UNC\");
            s.push_str(rest);
        } else {
            s.push_str(r"\\?\");
            s.push_str(&self.normalized);
        }
        PathBuf::from(s)
    }

    /// Letzte Komponente; `None` für eine Wurzel.
    pub fn file_name(&self) -> Option<&str> {
        if self.is_root() {
            return None;
        }
        self.normalized.rsplit('\\').next()
    }

    /// Übergeordneter Ordner; `None` für eine Wurzel.
    pub fn parent(&self) -> Option<ValidPath> {
        if self.is_root() {
            return None;
        }
        let cut = self.normalized.rfind('\\')?;
        let end = cut.max(self.root_len);
        Some(ValidPath {
            normalized: self.normalized[..end].to_owned(),
            root_len: self.root_len,
        })
    }

    /// Die Wurzel: `C:\` bzw. `\\server\freigabe`.
    pub fn root(&self) -> &str {
        &self.normalized[..self.root_len]
    }

    /// Ist `self` gleich `ancestor` oder liegt darunter? Ohne Beachtung der
    /// Groß-/Kleinschreibung, wie unter Windows üblich.
    pub fn is_inside_or_same(&self, ancestor: &ValidPath) -> bool {
        let me = self.normalized.to_lowercase();
        let other = ancestor.normalized.to_lowercase();
        if me == other {
            return true;
        }
        let prefix = if other.ends_with('\\') {
            other
        } else {
            format!("{other}\\")
        };
        me.starts_with(&prefix)
    }

    pub fn same_as(&self, other: &ValidPath) -> bool {
        self.normalized.to_lowercase() == other.normalized.to_lowercase()
    }

    /// Liegen beide auf demselben Laufwerk bzw. derselben Freigabe?
    pub fn same_volume(&self, other: &ValidPath) -> bool {
        self.normalized[..self.root_len].to_lowercase()
            == other.normalized[..other.root_len].to_lowercase()
    }

    /// Hängt einen einzelnen, geprüften Namen an.
    pub fn join(&self, name: &str) -> Result<ValidPath, PathError> {
        validate_name(name)?;
        let mut normalized = self.normalized.clone();
        if !normalized.ends_with('\\') {
            normalized.push('\\');
        }
        normalized.push_str(name);
        check_total_len(&normalized)?;
        Ok(ValidPath {
            normalized,
            root_len: self.root_len,
        })
    }
}

/// Prüft einen einzelnen Datei- oder Ordnernamen (z. B. für Umbenennen oder
/// „Neuer Ordner“).
pub fn validate_name(name: &str) -> Result<(), PathError> {
    if name.is_empty() {
        return Err(PathError::Empty);
    }
    if name.contains(['\\', '/']) {
        return Err(PathError::InvalidCharacter);
    }
    if name == "." || name == ".." {
        return Err(PathError::ParentTraversal);
    }
    check_component(name, true)
}

/// Prüft und normalisiert einen vom Frontend gelieferten Pfad.
pub fn validate_path(input: &str) -> Result<ValidPath, PathError> {
    if input.is_empty() {
        return Err(PathError::Empty);
    }
    check_total_len(input)?;
    if input.contains('\0') {
        return Err(PathError::InvalidCharacter);
    }
    let unified = input.replace('/', "\\");

    if unified.starts_with(r"\\?\") || unified.starts_with(r"\\.\") || unified.starts_with(r"\??\")
    {
        return Err(PathError::DeviceNamespace);
    }

    let (mut normalized, rest) = if let Some(unc) = unified.strip_prefix(r"\\") {
        let mut parts = unc.splitn(3, '\\');
        let server = parts.next().unwrap_or_default();
        let share = parts.next().unwrap_or_default();
        let rest = parts.next().unwrap_or_default();
        if server.is_empty() {
            return Err(PathError::NotAbsolute);
        }
        if server == "." || server == "?" {
            return Err(PathError::DeviceNamespace);
        }
        if share.is_empty() {
            return Err(PathError::MissingShare);
        }
        if share == "." || share == ".." {
            return Err(PathError::ParentTraversal);
        }
        check_component(server, false)?;
        check_component(share, false)?;
        (format!(r"\\{server}\{share}"), rest.to_owned())
    } else {
        let mut chars = unified.chars();
        let letter = chars.next().filter(char::is_ascii_alphabetic);
        let colon = chars.next();
        let (Some(letter), Some(':')) = (letter, colon) else {
            return Err(PathError::NotAbsolute);
        };
        let rest = chars.as_str();
        // `C:foo` ist laufwerksrelativ und hängt vom Arbeitsverzeichnis ab.
        if !rest.is_empty() && !rest.starts_with('\\') {
            return Err(PathError::NotAbsolute);
        }
        (
            format!("{}:\\", letter.to_ascii_uppercase()),
            rest.to_owned(),
        )
    };
    let root_len = normalized.len();

    for component in rest.split('\\') {
        match component {
            "" | "." => continue,
            ".." => return Err(PathError::ParentTraversal),
            _ => check_component(component, true)?,
        }
        if !normalized.ends_with('\\') {
            normalized.push('\\');
        }
        normalized.push_str(component);
    }
    check_total_len(&normalized)?;

    Ok(ValidPath {
        normalized,
        root_len,
    })
}

fn check_total_len(path: &str) -> Result<(), PathError> {
    // Bytes >= UTF-16-Einheiten ist nicht garantiert, daher exakt zählen,
    // aber erst nach einer billigen Vorprüfung.
    if path.len() > MAX_PATH_UNITS && path.encode_utf16().count() > MAX_PATH_UNITS {
        return Err(PathError::TooLong);
    }
    if path.len() > MAX_PATH_UNITS * 3 {
        return Err(PathError::TooLong);
    }
    Ok(())
}

fn check_component(component: &str, check_reserved: bool) -> Result<(), PathError> {
    for c in component.chars() {
        match c {
            ':' => return Err(PathError::AlternateDataStream),
            '<' | '>' | '"' | '|' | '?' | '*' | '\\' | '/' => {
                return Err(PathError::InvalidCharacter)
            }
            c if (c as u32) < 0x20 => return Err(PathError::InvalidCharacter),
            _ => {}
        }
    }
    if component.ends_with(['.', ' ']) {
        return Err(PathError::TrailingDotOrSpace);
    }
    if component.encode_utf16().count() > MAX_NAME_UNITS {
        return Err(PathError::TooLong);
    }
    if check_reserved && is_reserved_name(component) {
        return Err(PathError::ReservedName);
    }
    Ok(())
}

/// Reservierte DOS-Gerätenamen. Windows behandelt auch `NUL.txt` oder `con .log`
/// als Gerät, deshalb zählt nur der Teil vor dem ersten Punkt.
fn is_reserved_name(component: &str) -> bool {
    let base = component
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches(' ')
        .to_uppercase();
    if matches!(
        base.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) {
        return true;
    }
    let mut chars = base.chars();
    let prefix: String = chars.by_ref().take(3).collect();
    if prefix != "COM" && prefix != "LPT" {
        return false;
    }
    matches!(
        (chars.next(), chars.next()),
        (Some('0'..='9' | '¹' | '²' | '³'), None)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(input: &str) -> String {
        validate_path(input).unwrap().into_string()
    }

    fn err(input: &str) -> PathError {
        validate_path(input).unwrap_err()
    }

    #[test]
    fn normalizes_drive_paths() {
        assert_eq!(ok(r"C:\"), r"C:\");
        assert_eq!(ok("c:"), r"C:\");
        assert_eq!(ok(r"c:\Users\Demo"), r"C:\Users\Demo");
        assert_eq!(ok(r"C:\Users\Demo\"), r"C:\Users\Demo");
        assert_eq!(ok("C:/Users//Demo/./Docs"), r"C:\Users\Demo\Docs");
        assert_eq!(ok(r"D:\Über uns\日本語.txt"), r"D:\Über uns\日本語.txt");
    }

    #[test]
    fn normalizes_unc_paths() {
        assert_eq!(ok(r"\\server\share"), r"\\server\share");
        assert_eq!(ok(r"\\server\share\"), r"\\server\share");
        assert_eq!(ok("//server/share/a/b"), r"\\server\share\a\b");
        assert_eq!(ok(r"\\192.168.1.10\c$\x"), r"\\192.168.1.10\c$\x");
    }

    #[test]
    fn rejects_relative_and_empty() {
        assert_eq!(err(""), PathError::Empty);
        assert_eq!(err("foo"), PathError::NotAbsolute);
        assert_eq!(err(r"foo\bar"), PathError::NotAbsolute);
        assert_eq!(err(r"\foo"), PathError::NotAbsolute);
        assert_eq!(err("C:foo"), PathError::NotAbsolute);
        assert_eq!(err(r"1:\foo"), PathError::NotAbsolute);
        assert_eq!(err(r"\\server"), PathError::MissingShare);
        assert_eq!(err(r"\\server\"), PathError::MissingShare);
        assert_eq!(err(r"\\\share"), PathError::NotAbsolute);
    }

    #[test]
    fn rejects_traversal() {
        assert_eq!(err(r"C:\Users\..\Windows"), PathError::ParentTraversal);
        assert_eq!(err(r"C:\.."), PathError::ParentTraversal);
        assert_eq!(err("C:/a/../../b"), PathError::ParentTraversal);
        assert_eq!(err(r"\\server\share\..\other"), PathError::ParentTraversal);
        assert_eq!(err(r"\\server\..\x"), PathError::ParentTraversal);
    }

    #[test]
    fn rejects_device_namespaces() {
        assert_eq!(err(r"\\?\C:\Windows"), PathError::DeviceNamespace);
        assert_eq!(err(r"\\.\PhysicalDrive0"), PathError::DeviceNamespace);
        assert_eq!(err(r"\??\C:\Windows"), PathError::DeviceNamespace);
        assert_eq!(err("//./pipe/x"), PathError::DeviceNamespace);
        assert_eq!(err("//?/C:/x"), PathError::DeviceNamespace);
        assert_eq!(err(r"\\?\UNC\server\share"), PathError::DeviceNamespace);
    }

    #[test]
    fn rejects_alternate_data_streams() {
        assert_eq!(err(r"C:\file.txt:secret"), PathError::AlternateDataStream);
        assert_eq!(err(r"C:\file.txt::$DATA"), PathError::AlternateDataStream);
        assert_eq!(err(r"C:\dir:stream\file"), PathError::AlternateDataStream);
        assert_eq!(err(r"C:\C:\x"), PathError::AlternateDataStream);
    }

    #[test]
    fn rejects_reserved_names() {
        for name in [
            "CON",
            "con",
            "PRN",
            "AUX",
            "NUL",
            "nul.txt",
            "NUL.tar.gz",
            "COM1",
            "com9.log",
            "LPT1",
            "LPT0",
            "COM¹",
            "CONIN$",
            "CONOUT$",
        ] {
            assert_eq!(
                err(&format!(r"C:\dir\{name}")),
                PathError::ReservedName,
                "{name}"
            );
            assert_eq!(
                err(&format!(r"C:\{name}\x.txt")),
                PathError::ReservedName,
                "{name}"
            );
        }
        // Ähnliche, aber erlaubte Namen
        for name in [
            "CONSOLE",
            "COM",
            "COM10",
            "LPT",
            "nulls.txt",
            "a.NUL",
            "icon",
        ] {
            assert!(validate_path(&format!(r"C:\{name}")).is_ok(), "{name}");
        }
    }

    #[test]
    fn rejects_invalid_characters() {
        for bad in ["a<b", "a>b", "a\"b", "a|b", "a?b", "a*b", "a\u{1}b", "a\tb"] {
            assert_eq!(
                err(&format!(r"C:\{bad}")),
                PathError::InvalidCharacter,
                "{bad:?}"
            );
        }
        assert_eq!(err("C:\\a\0b"), PathError::InvalidCharacter);
    }

    #[test]
    fn rejects_trailing_dot_or_space() {
        assert_eq!(err(r"C:\dir.\x"), PathError::TrailingDotOrSpace);
        assert_eq!(err(r"C:\dir \x"), PathError::TrailingDotOrSpace);
        assert_eq!(err(r"C:\file.txt."), PathError::TrailingDotOrSpace);
        assert_eq!(err(r"C:\..."), PathError::TrailingDotOrSpace);
        // Führende Punkte/Leerzeichen sind gültig.
        assert_eq!(ok(r"C:\.git\ x"), r"C:\.git\ x");
    }

    #[test]
    fn rejects_too_long() {
        let long_name = "a".repeat(MAX_NAME_UNITS + 1);
        assert_eq!(err(&format!(r"C:\{long_name}")), PathError::TooLong);
        let ok_name = "a".repeat(MAX_NAME_UNITS);
        assert!(validate_path(&format!(r"C:\{ok_name}")).is_ok());

        let mut deep = String::from("C:");
        while deep.len() <= MAX_PATH_UNITS {
            deep.push_str(r"\abcdefghij");
        }
        assert_eq!(err(&deep), PathError::TooLong);
    }

    #[test]
    fn long_paths_beyond_max_path_are_fine() {
        let mut deep = String::from("C:");
        for _ in 0..100 {
            deep.push_str(r"\abcdefghij");
        }
        assert!(deep.len() > 260);
        assert_eq!(ok(&deep), deep);
    }

    #[test]
    fn extended_form() {
        let p = validate_path(r"C:\Users\Demo").unwrap();
        assert_eq!(p.to_extended().to_str().unwrap(), r"\\?\C:\Users\Demo");
        let p = validate_path(r"C:\").unwrap();
        assert_eq!(p.to_extended().to_str().unwrap(), r"\\?\C:\");
        let p = validate_path(r"\\srv\share\a").unwrap();
        assert_eq!(p.to_extended().to_str().unwrap(), r"\\?\UNC\srv\share\a");
        assert!(p.is_unc());
    }

    #[test]
    fn parent_and_file_name() {
        let p = validate_path(r"C:\a\b").unwrap();
        assert_eq!(p.file_name(), Some("b"));
        let parent = p.parent().unwrap();
        assert_eq!(parent.as_str(), r"C:\a");
        let root = parent.parent().unwrap();
        assert_eq!(root.as_str(), r"C:\");
        assert!(root.is_root());
        assert_eq!(root.parent(), None);
        assert_eq!(root.file_name(), None);

        let p = validate_path(r"\\srv\share\a").unwrap();
        let root = p.parent().unwrap();
        assert_eq!(root.as_str(), r"\\srv\share");
        assert!(root.is_root());
        assert_eq!(root.parent(), None);
    }

    #[test]
    fn inside_and_volume_checks() {
        let a = validate_path(r"C:\Users\Demo").unwrap();
        let b = validate_path(r"c:\users\demo\Docs\x").unwrap();
        let c = validate_path(r"C:\Users\Demonstration").unwrap();
        let root = validate_path(r"C:\").unwrap();
        assert!(b.is_inside_or_same(&a));
        assert!(a.is_inside_or_same(&a));
        assert!(!a.is_inside_or_same(&b));
        assert!(!c.is_inside_or_same(&a), "Präfix allein genügt nicht");
        assert!(a.is_inside_or_same(&root));
        assert!(a.same_volume(&c));
        let d = validate_path(r"D:\x").unwrap();
        assert!(!a.same_volume(&d));
        let u1 = validate_path(r"\\srv\one\a").unwrap();
        let u2 = validate_path(r"\\SRV\ONE\b").unwrap();
        let u3 = validate_path(r"\\srv\two\a").unwrap();
        assert!(u1.same_volume(&u2));
        assert!(!u1.same_volume(&u3));
        assert!(u1.is_inside_or_same(&validate_path(r"\\srv\one").unwrap()));
    }

    #[test]
    fn join_validates_name() {
        let root = validate_path(r"C:\").unwrap();
        assert_eq!(root.join("a").unwrap().as_str(), r"C:\a");
        let dir = validate_path(r"\\srv\share").unwrap();
        assert_eq!(
            dir.join("a b.txt").unwrap().as_str(),
            r"\\srv\share\a b.txt"
        );

        assert_eq!(root.join(""), Err(PathError::Empty));
        assert_eq!(root.join(".."), Err(PathError::ParentTraversal));
        assert_eq!(root.join("."), Err(PathError::ParentTraversal));
        assert_eq!(root.join(r"a\b"), Err(PathError::InvalidCharacter));
        assert_eq!(root.join("a/b"), Err(PathError::InvalidCharacter));
        assert_eq!(root.join("a:b"), Err(PathError::AlternateDataStream));
        assert_eq!(root.join("NUL"), Err(PathError::ReservedName));
        assert_eq!(root.join("x."), Err(PathError::TrailingDotOrSpace));
    }
}
