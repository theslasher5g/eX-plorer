//! Regeln für die Schnellvorschau: Was wird wie angezeigt, und wie wird Text
//! sicher dekodiert. Reine Logik ohne Dateizugriff.

use crate::exec::extension;

pub const MAX_IMAGE_BYTES: u64 = 40 * 1024 * 1024;
pub const MAX_PDF_BYTES: u64 = 64 * 1024 * 1024;
/// Es werden höchstens so viele Bytes Text gelesen und angezeigt.
pub const MAX_TEXT_BYTES: u64 = 256 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Image { mime: &'static str },
    Pdf,
    Text,
    Other,
}

impl Kind {
    /// Größenlimit für das komplette Laden; `None` heißt: nicht ladbar.
    pub fn max_bytes(self) -> Option<u64> {
        match self {
            Self::Image { .. } => Some(MAX_IMAGE_BYTES),
            Self::Pdf => Some(MAX_PDF_BYTES),
            // Text wird abgeschnitten statt abgelehnt.
            Self::Text => Some(u64::MAX),
            Self::Other => None,
        }
    }
}

const TEXT_EXTENSIONS: &[&str] = &[
    "txt",
    "md",
    "markdown",
    "log",
    "json",
    "jsonc",
    "csv",
    "tsv",
    "xml",
    "html",
    "htm",
    "css",
    "scss",
    "js",
    "mjs",
    "cjs",
    "ts",
    "tsx",
    "jsx",
    "svelte",
    "vue",
    "rs",
    "toml",
    "yaml",
    "yml",
    "ini",
    "cfg",
    "conf",
    "py",
    "java",
    "kt",
    "c",
    "h",
    "cpp",
    "hpp",
    "cs",
    "go",
    "rb",
    "php",
    "sh",
    "bat",
    "cmd",
    "ps1",
    "sql",
    "tex",
    "srt",
    "vtt",
    "gitignore",
    "gitattributes",
    "editorconfig",
    "env",
    "properties",
    "lock",
    "svg",
];

const TEXT_NAMES: &[&str] = &[
    "readme",
    "license",
    "licence",
    "makefile",
    "dockerfile",
    "changelog",
    "notice",
];

pub fn kind_for(name: &str) -> Kind {
    let lower = name.to_lowercase();
    match extension(&lower).as_deref() {
        Some("png") => Kind::Image { mime: "image/png" },
        Some("jpg" | "jpeg" | "jfif") => Kind::Image { mime: "image/jpeg" },
        Some("gif") => Kind::Image { mime: "image/gif" },
        Some("webp") => Kind::Image { mime: "image/webp" },
        Some("bmp") => Kind::Image { mime: "image/bmp" },
        Some("ico") => Kind::Image {
            mime: "image/x-icon",
        },
        Some("pdf") => Kind::Pdf,
        // SVG wird als Text gezeigt, nicht als Bild: so läuft garantiert kein Skript.
        Some(ext) if TEXT_EXTENSIONS.contains(&ext) => Kind::Text,
        None if TEXT_NAMES.contains(&lower.as_str()) => Kind::Text,
        _ => Kind::Other,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextPreview {
    pub text: String,
    pub truncated: bool,
}

/// Windows-1252 für 0x80..=0x9F; der Rest entspricht Latin-1.
const CP1252: [char; 32] = [
    '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8D}', 'Ž', '\u{8F}',
    '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9D}', 'ž', 'Ÿ',
];

fn decode_cp1252(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| match b {
            0x80..=0x9F => CP1252[usize::from(b - 0x80)],
            _ => char::from(b),
        })
        .collect()
}

fn decode_utf16(bytes: &[u8], little_endian: bool) -> String {
    let units = bytes.chunks_exact(2).map(|pair| {
        if little_endian {
            u16::from_le_bytes([pair[0], pair[1]])
        } else {
            u16::from_be_bytes([pair[0], pair[1]])
        }
    });
    char::decode_utf16(units)
        .map(|r| r.unwrap_or('\u{FFFD}'))
        .collect()
}

/// Dekodiert Textbytes. `None` bei Binärdaten. `truncated` meldet, dass
/// `bytes` nur der Anfang einer größeren Datei ist (ein am Ende
/// abgeschnittenes Zeichen wird dann nicht als Fehler gewertet).
pub fn decode_text(bytes: &[u8], truncated: bool) -> Option<TextPreview> {
    let text = if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        decode_utf8_or_legacy(rest, truncated)?
    } else if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        decode_utf16(rest, true)
    } else if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        decode_utf16(rest, false)
    } else {
        decode_utf8_or_legacy(bytes, truncated)?
    };
    Some(TextPreview { text, truncated })
}

fn decode_utf8_or_legacy(bytes: &[u8], truncated: bool) -> Option<String> {
    // NUL-Bytes in den ersten KiB sind ein sicheres Zeichen für Binärdaten.
    if bytes.iter().take(8192).any(|&b| b == 0) {
        return None;
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => Some(s.to_owned()),
        Err(err) => {
            // Am Ende abgeschnittenes Mehrbyte-Zeichen: ignorieren.
            if truncated && err.error_len().is_none() {
                return Some(String::from_utf8_lossy(&bytes[..err.valid_up_to()]).into_owned());
            }
            Some(decode_cp1252(bytes))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_by_extension() {
        assert_eq!(kind_for("a.PNG"), Kind::Image { mime: "image/png" });
        assert_eq!(kind_for("x.jpeg"), Kind::Image { mime: "image/jpeg" });
        assert_eq!(kind_for("Bericht.PDF"), Kind::Pdf);
        assert_eq!(kind_for("main.rs"), Kind::Text);
        assert_eq!(kind_for("README"), Kind::Text);
        assert_eq!(kind_for(".gitignore"), Kind::Text);
        assert_eq!(kind_for("a.docx"), Kind::Other);
        assert_eq!(kind_for("noext"), Kind::Other);
        // SVG nie als Bild ausliefern.
        assert_eq!(kind_for("logo.svg"), Kind::Text);
    }

    #[test]
    fn limits_per_kind() {
        assert_eq!(kind_for("a.png").max_bytes(), Some(MAX_IMAGE_BYTES));
        assert_eq!(kind_for("a.pdf").max_bytes(), Some(MAX_PDF_BYTES));
        assert_eq!(kind_for("a.docx").max_bytes(), None);
    }

    #[test]
    fn decodes_utf8_and_bom() {
        let p = decode_text("Grüße".as_bytes(), false).unwrap();
        assert_eq!(p.text, "Grüße");
        let mut with_bom = vec![0xEF, 0xBB, 0xBF];
        with_bom.extend_from_slice("ok".as_bytes());
        assert_eq!(decode_text(&with_bom, false).unwrap().text, "ok");
    }

    #[test]
    fn decodes_utf16_with_bom() {
        let le: Vec<u8> = [0xFF, 0xFE]
            .into_iter()
            .chain("Hä".encode_utf16().flat_map(u16::to_le_bytes))
            .collect();
        assert_eq!(decode_text(&le, false).unwrap().text, "Hä");
        let be: Vec<u8> = [0xFE, 0xFF]
            .into_iter()
            .chain("Hä".encode_utf16().flat_map(u16::to_be_bytes))
            .collect();
        assert_eq!(decode_text(&be, false).unwrap().text, "Hä");
    }

    #[test]
    fn falls_back_to_windows_1252() {
        // „Müller“ in Latin-1/CP1252, dazu das Euro-Zeichen (0x80).
        let bytes = [b'M', 0xFC, b'l', b'l', b'e', b'r', b' ', 0x80];
        assert_eq!(decode_text(&bytes, false).unwrap().text, "Müller €");
    }

    #[test]
    fn truncated_multibyte_tail_is_dropped() {
        let full = "aä".as_bytes(); // 'ä' hat 2 Bytes
        let cut = &full[..full.len() - 1];
        let p = decode_text(cut, true).unwrap();
        assert_eq!(p.text, "a");
        assert!(p.truncated);
        // Ohne Abschneiden ist dieselbe Folge „kaputtes UTF-8“ -> CP1252-Fallback.
        assert!(decode_text(cut, false).unwrap().text.starts_with('a'));
    }

    #[test]
    fn rejects_binary() {
        assert!(decode_text(&[0x50, 0x4B, 0x03, 0x04, 0, 0, 0], false).is_none());
    }
}
