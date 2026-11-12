//! Icons und Thumbnails über `IShellItemImageFactory`.
//!
//! Wichtig für OneDrive und Netzlaufwerke: Für Cloud-Platzhalter und Dateien
//! auf Netzwerkpfaden wird **nichts** erzeugt, denn das würde den Inhalt
//! herunterladen bzw. das Netz belasten. Das Frontend zeigt dort seine
//! gezeichneten Typ-Symbole.

use std::mem::size_of;
use std::os::windows::fs::MetadataExt;

use explorer_core::attrs::is_cloud_placeholder;
use explorer_core::imaging;
use explorer_core::path::ValidPath;
use explorer_core::CoreError;
use windows::core::Interface;
use windows::Win32::Foundation::SIZE;
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER,
    BI_RGB, DIB_RGB_COLORS, HBITMAP, HGDIOBJ,
};
use windows::Win32::UI::Shell::{IShellItemImageFactory, SIIGBF_BIGGERSIZEOK};

use super::items::{item, to_core};
use super::places;

pub const MIN_SIZE: u32 = 16;
pub const MAX_SIZE: u32 = 512;

/// Räumt das Bitmap beim Drop weg.
struct Bitmap(HBITMAP);

impl Drop for Bitmap {
    fn drop(&mut self) {
        // SAFETY: Das Handle gehört uns (von GetImage geliefert) und wird einmal gelöscht.
        unsafe {
            let _ = DeleteObject(HGDIOBJ(self.0 .0));
        }
    }
}

fn to_bgra(bitmap: &Bitmap) -> Result<(u32, u32, Vec<u8>), CoreError> {
    let mut info = BITMAP::default();
    // SAFETY: gültiges Bitmap-Handle; `info` ist ein ausreichend großer Ausgabepuffer.
    let read = unsafe {
        GetObjectW(
            HGDIOBJ(bitmap.0 .0),
            size_of::<BITMAP>() as i32,
            Some((&raw mut info).cast()),
        )
    };
    if read == 0 || info.bmWidth <= 0 || info.bmHeight == 0 {
        return Err(CoreError::Unsupported);
    }
    let width = info.bmWidth as u32;
    let height = info.bmHeight.unsigned_abs();
    if width > imaging::MAX_DIMENSION || height > imaging::MAX_DIMENSION {
        return Err(CoreError::TooLarge);
    }

    let mut header = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            // Negativ: Zeilen von oben nach unten.
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut pixels = vec![0u8; width as usize * height as usize * 4];

    // SAFETY: Der Gerätekontext wird nach der Abfrage freigegeben; `pixels` hat genau
    // die Größe für `height` Zeilen zu je `width` BGRA-Pixeln.
    let lines = unsafe {
        let hdc = GetDC(None);
        let lines = GetDIBits(
            hdc,
            bitmap.0,
            0,
            height,
            Some(pixels.as_mut_ptr().cast()),
            &raw mut header,
            DIB_RGB_COLORS,
        );
        ReleaseDC(None, hdc);
        lines
    };
    if lines == 0 {
        return Err(CoreError::Unsupported);
    }
    Ok((width, height, pixels))
}

/// Liefert Icon oder Thumbnail der Datei als PNG.
pub fn render_png(path: &ValidPath, size: u32) -> Result<Vec<u8>, CoreError> {
    // Nur Metadaten: öffnet die Datei nicht und lädt nichts herunter.
    let meta = std::fs::symlink_metadata(path.to_extended())?;
    if is_cloud_placeholder(meta.file_attributes()) || places::is_remote(path) {
        return Err(CoreError::Unsupported);
    }

    let size = size.clamp(MIN_SIZE, MAX_SIZE);
    let factory: IShellItemImageFactory =
        item(path).and_then(|i| i.cast()).map_err(|e| to_core(&e))?;
    // SAFETY: gültige Factory; Größe und Flags sind Werte, keine Zeiger.
    let hbitmap = unsafe {
        factory.GetImage(
            SIZE {
                cx: size as i32,
                cy: size as i32,
            },
            SIIGBF_BIGGERSIZEOK,
        )
    }
    .map_err(|e| to_core(&e))?;
    let bitmap = Bitmap(hbitmap);

    let (width, height, mut pixels) = to_bgra(&bitmap)?;
    // Windows liefert Bitmaps mit Alphakanal vormultipliziert.
    imaging::finalize_bgra(&mut pixels, true);
    imaging::encode_png(width, height, &pixels).map_err(|_| CoreError::Unsupported)
}
