//! Windows-Dateiattribute, soweit sie die Logik betreffen.

pub const FILE_ATTRIBUTE_HIDDEN: u32 = 0x0000_0002;
pub const FILE_ATTRIBUTE_SYSTEM: u32 = 0x0000_0004;
pub const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0000_0010;
pub const FILE_ATTRIBUTE_OFFLINE: u32 = 0x0000_1000;
pub const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x0004_0000;
pub const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;

/// Ist der Inhalt nicht lokal vorhanden (OneDrive „nur in der Cloud“)?
/// Das Öffnen oder Lesen einer solchen Datei würde sie herunterladen, deshalb
/// werden für sie keine Vorschauen oder Thumbnails erzeugt.
pub fn is_cloud_placeholder(attributes: u32) -> bool {
    attributes
        & (FILE_ATTRIBUTE_OFFLINE
            | FILE_ATTRIBUTE_RECALL_ON_OPEN
            | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
        != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_placeholders() {
        assert!(is_cloud_placeholder(FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS));
        assert!(is_cloud_placeholder(
            FILE_ATTRIBUTE_RECALL_ON_OPEN | FILE_ATTRIBUTE_HIDDEN
        ));
        assert!(is_cloud_placeholder(FILE_ATTRIBUTE_OFFLINE));
        assert!(!is_cloud_placeholder(0x20));
        assert!(!is_cloud_placeholder(
            FILE_ATTRIBUTE_DIRECTORY | FILE_ATTRIBUTE_SYSTEM
        ));
    }
}
