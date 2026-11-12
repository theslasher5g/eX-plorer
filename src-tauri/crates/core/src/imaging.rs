//! Umwandlung von Windows-Bitmaps (BGRA) in PNG für das Frontend.

use thiserror::Error;

/// Größte erlaubte Kantenlänge, Schutz vor absurden Eingaben.
pub const MAX_DIMENSION: u32 = 4096;

#[derive(Debug, Error)]
pub enum ImageError {
    #[error("invalid image dimensions or data length")]
    Invalid,
    #[error("png encoding failed")]
    Encode,
}

/// Bereitet BGRA-Pixel für die Ausgabe auf:
/// * Sind alle Alphawerte 0 (manche Thumbnails liefern das), gilt das Bild als deckend.
/// * Windows liefert Bitmaps mit Alphakanal vormultipliziert; das wird zurückgerechnet.
pub fn finalize_bgra(bgra: &mut [u8], premultiplied: bool) {
    let opaque_missing = bgra.chunks_exact(4).all(|px| px[3] == 0);
    if opaque_missing {
        for px in bgra.chunks_exact_mut(4) {
            px[3] = 255;
        }
        return;
    }
    if !premultiplied {
        return;
    }
    for px in bgra.chunks_exact_mut(4) {
        let a = u32::from(px[3]);
        if a == 0 || a == 255 {
            continue;
        }
        for channel in &mut px[..3] {
            *channel = ((u32::from(*channel) * 255 + a / 2) / a).min(255) as u8;
        }
    }
}

/// Kodiert BGRA-Pixel (Zeile für Zeile, oben beginnend) als PNG.
pub fn encode_png(width: u32, height: u32, bgra: &[u8]) -> Result<Vec<u8>, ImageError> {
    if width == 0 || height == 0 || width > MAX_DIMENSION || height > MAX_DIMENSION {
        return Err(ImageError::Invalid);
    }
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or(ImageError::Invalid)?;
    if bgra.len() != expected {
        return Err(ImageError::Invalid);
    }

    let mut rgba = Vec::with_capacity(expected);
    for px in bgra.chunks_exact(4) {
        rgba.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
    }

    let mut out = Vec::with_capacity(expected / 4 + 256);
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);
        let mut writer = encoder.write_header().map_err(|_| ImageError::Encode)?;
        writer
            .write_image_data(&rgba)
            .map_err(|_| ImageError::Encode)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(png_bytes: &[u8]) -> (u32, u32, Vec<u8>) {
        let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut buf).unwrap();
        buf.truncate(info.buffer_size());
        (info.width, info.height, buf)
    }

    #[test]
    fn roundtrip_swaps_channels() {
        // Ein blaues, ein rotes Pixel (BGRA).
        let bgra = [255, 0, 0, 255, 0, 0, 255, 255];
        let png = encode_png(2, 1, &bgra).unwrap();
        assert_eq!(&png[..4], &[0x89, b'P', b'N', b'G']);
        let (w, h, rgba) = decode(&png);
        assert_eq!((w, h), (2, 1));
        assert_eq!(rgba, vec![0, 0, 255, 255, 255, 0, 0, 255]);
    }

    #[test]
    fn all_zero_alpha_becomes_opaque() {
        let mut px = [10, 20, 30, 0, 40, 50, 60, 0];
        finalize_bgra(&mut px, true);
        assert_eq!(px, [10, 20, 30, 255, 40, 50, 60, 255]);
    }

    #[test]
    fn unpremultiplies_partial_alpha() {
        // 50 % Alpha, vormultiplizierter Wert 100 -> ca. 200.
        let mut px = [100, 0, 0, 128, 7, 7, 7, 255, 9, 9, 9, 0];
        finalize_bgra(&mut px, true);
        assert_eq!(px[0], 199);
        assert_eq!(&px[4..8], &[7, 7, 7, 255]);
        assert_eq!(&px[8..12], &[9, 9, 9, 0]);
    }

    #[test]
    fn leaves_straight_alpha_alone() {
        let mut px = [100, 0, 0, 128];
        finalize_bgra(&mut px, false);
        assert_eq!(px, [100, 0, 0, 128]);
    }

    #[test]
    fn rejects_bad_input() {
        assert!(encode_png(0, 1, &[]).is_err());
        assert!(encode_png(2, 2, &[0; 4]).is_err());
        assert!(encode_png(MAX_DIMENSION + 1, 1, &[]).is_err());
    }
}
