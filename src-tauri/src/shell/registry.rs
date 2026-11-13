//! Lesender Zugriff auf `HKEY_CURRENT_USER`. Es wird nie geschrieben.

use windows::core::{HSTRING, PCWSTR, PWSTR};
use windows::Win32::System::Registry::{
    RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW, HKEY, HKEY_CURRENT_USER, KEY_READ,
    RRF_RT_REG_SZ,
};

/// Längster gelesener Wert in UTF-16-Einheiten; längere werden verworfen.
const MAX_VALUE_UNITS: usize = 2048;
/// Registry-Schlüsselnamen sind höchstens 255 Zeichen lang.
const MAX_KEY_UNITS: usize = 256;
/// Obergrenze für aufgezählte Unterschlüssel, Schutz vor absurden Strukturen.
const MAX_SUBKEYS: u32 = 256;

struct Key(HKEY);

impl Drop for Key {
    fn drop(&mut self) {
        // SAFETY: Das Handle stammt aus RegOpenKeyExW und wird genau einmal geschlossen.
        unsafe {
            let _ = RegCloseKey(self.0);
        }
    }
}

fn open(path: &str) -> Option<Key> {
    let wide = HSTRING::from(path);
    let mut key = HKEY::default();
    // SAFETY: `wide` ist nullterminiert und überlebt den Aufruf; `key` ist ein gültiger Ausgabezeiger.
    unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(wide.as_ptr()),
            None,
            KEY_READ,
            &raw mut key,
        )
    }
    .ok()
    .ok()?;
    Some(Key(key))
}

/// Namen der direkten Unterschlüssel von `path`.
pub fn subkeys(path: &str) -> Vec<String> {
    let Some(key) = open(path) else {
        return Vec::new();
    };
    let mut names = Vec::new();
    for index in 0..MAX_SUBKEYS {
        let mut buffer = [0u16; MAX_KEY_UNITS];
        let mut len = buffer.len() as u32;
        // SAFETY: `buffer` fasst `len` Einheiten; der Schlüssel ist geöffnet.
        let status = unsafe {
            RegEnumKeyExW(
                key.0,
                index,
                Some(PWSTR(buffer.as_mut_ptr())),
                &raw mut len,
                None,
                None,
                None,
                None,
            )
        };
        if status.ok().is_err() {
            break;
        }
        names.push(String::from_utf16_lossy(&buffer[..len as usize]));
    }
    names
}

/// Zeichenkettenwert `name` von `path`; `None`, wenn er fehlt oder zu lang ist.
pub fn string_value(path: &str, name: &str) -> Option<String> {
    let subkey = HSTRING::from(path);
    let value = HSTRING::from(name);
    let mut buffer = vec![0u16; MAX_VALUE_UNITS];
    let mut bytes = (buffer.len() * 2) as u32;
    // SAFETY: Alle Zeichenketten sind nullterminiert; `buffer` fasst `bytes` Bytes.
    unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            PCWSTR(subkey.as_ptr()),
            PCWSTR(value.as_ptr()),
            RRF_RT_REG_SZ,
            None,
            Some(buffer.as_mut_ptr().cast()),
            Some(&raw mut bytes),
        )
    }
    .ok()
    .ok()?;
    // Die Größe enthält das abschließende Nullzeichen.
    let units = (bytes as usize / 2).saturating_sub(1).min(buffer.len());
    let text = String::from_utf16_lossy(&buffer[..units]);
    (!text.is_empty()).then_some(text)
}
