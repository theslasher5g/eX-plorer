//! Windows-Dateiattribute, soweit sie die Logik betreffen.

pub const FILE_ATTRIBUTE_READONLY: u32 = 0x0000_0001;
pub const FILE_ATTRIBUTE_HIDDEN: u32 = 0x0000_0002;
pub const FILE_ATTRIBUTE_SYSTEM: u32 = 0x0000_0004;
pub const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0000_0010;
pub const FILE_ATTRIBUTE_ARCHIVE: u32 = 0x0000_0020;
pub const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
pub const FILE_ATTRIBUTE_OFFLINE: u32 = 0x0000_1000;
pub const FILE_ATTRIBUTE_NOT_CONTENT_INDEXED: u32 = 0x0000_2000;
pub const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x0004_0000;
pub const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;
/// „Immer auf diesem Gerät behalten“.
pub const FILE_ATTRIBUTE_PINNED: u32 = 0x0008_0000;
/// „Speicherplatz freigeben“.
pub const FILE_ATTRIBUTE_UNPINNED: u32 = 0x0010_0000;

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

/// Sync-Zustand einer Datei im OneDrive-Ordner (wie im Explorer-Symbol).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncState {
    /// Nur in der Cloud, Inhalt nicht lokal vorhanden.
    Cloud,
    /// Lokal verfügbar, kann bei Platzmangel wieder freigegeben werden.
    Local,
    /// Immer auf diesem Gerät behalten.
    Pinned,
    /// Wird gerade hoch- oder heruntergeladen.
    Syncing,
}

/// Cloud-Platzhalter tragen ein Reparse-Tag `0x9000_?01A` (IO_REPARSE_TAG_CLOUD
/// und seine Varianten 1 bis F).
pub fn is_cloud_reparse_tag(tag: u32) -> bool {
    tag & 0xFFFF_0FFF == 0x9000_001A
}

/// Bestimmt den Sync-Zustand allein aus den Verzeichnisdaten (Attribute,
/// Reparse-Tag, „in sync“-Flag der Cloud-Files-API). Es wird nichts gelesen
/// oder heruntergeladen. `None`: kein Cloud-Platzhalter.
pub fn sync_state(attributes: u32, reparse_tag: u32, in_sync: Option<bool>) -> Option<SyncState> {
    if attributes & FILE_ATTRIBUTE_REPARSE_POINT == 0 || !is_cloud_reparse_tag(reparse_tag) {
        return None;
    }
    let pinned = attributes & FILE_ATTRIBUTE_PINNED != 0;
    Some(if is_cloud_placeholder(attributes) {
        // „Immer behalten“ gesetzt, aber noch nicht heruntergeladen.
        if pinned {
            SyncState::Syncing
        } else {
            SyncState::Cloud
        }
    } else if in_sync == Some(false) {
        SyncState::Syncing
    } else if pinned {
        SyncState::Pinned
    } else {
        SyncState::Local
    })
}

/// Neue Attribute für „Immer behalten“ (`pinned`) bzw. „Speicherplatz freigeben“.
/// Nur veränderbare Attribute werden übernommen; beide Zustände schließen
/// sich aus.
pub fn pin_attributes(current: u32, pinned: bool) -> u32 {
    const KEEP: u32 = FILE_ATTRIBUTE_READONLY
        | FILE_ATTRIBUTE_HIDDEN
        | FILE_ATTRIBUTE_SYSTEM
        | FILE_ATTRIBUTE_ARCHIVE
        | FILE_ATTRIBUTE_NOT_CONTENT_INDEXED;
    (current & KEEP)
        | if pinned {
            FILE_ATTRIBUTE_PINNED
        } else {
            FILE_ATTRIBUTE_UNPINNED
        }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLACEHOLDER: u32 = FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_ARCHIVE;
    const TAG: u32 = 0x9000_001A;

    #[test]
    fn recognizes_cloud_tags() {
        assert!(is_cloud_reparse_tag(0x9000_001A));
        assert!(is_cloud_reparse_tag(0x9000_901A));
        assert!(is_cloud_reparse_tag(0x9000_F01A));
        assert!(!is_cloud_reparse_tag(0xA000_0003), "Junction");
        assert!(!is_cloud_reparse_tag(0xA000_000C), "Symlink");
        assert!(!is_cloud_reparse_tag(0));
    }

    #[test]
    fn maps_sync_states() {
        let cloud = PLACEHOLDER | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS;
        assert_eq!(sync_state(cloud, TAG, Some(true)), Some(SyncState::Cloud));
        assert_eq!(
            sync_state(cloud | FILE_ATTRIBUTE_UNPINNED, TAG, None),
            Some(SyncState::Cloud)
        );
        assert_eq!(
            sync_state(PLACEHOLDER, TAG, Some(true)),
            Some(SyncState::Local)
        );
        assert_eq!(sync_state(PLACEHOLDER, TAG, None), Some(SyncState::Local));
        assert_eq!(
            sync_state(PLACEHOLDER | FILE_ATTRIBUTE_PINNED, TAG, Some(true)),
            Some(SyncState::Pinned)
        );
    }

    #[test]
    fn pending_work_means_syncing() {
        // Lokal geändert, noch nicht hochgeladen.
        assert_eq!(
            sync_state(PLACEHOLDER, TAG, Some(false)),
            Some(SyncState::Syncing)
        );
        assert_eq!(
            sync_state(PLACEHOLDER | FILE_ATTRIBUTE_PINNED, TAG, Some(false)),
            Some(SyncState::Syncing)
        );
        // „Immer behalten“ gesetzt, Inhalt kommt noch.
        let downloading =
            PLACEHOLDER | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS | FILE_ATTRIBUTE_PINNED;
        assert_eq!(
            sync_state(downloading, TAG, Some(true)),
            Some(SyncState::Syncing)
        );
    }

    #[test]
    fn non_cloud_entries_have_no_state() {
        assert_eq!(sync_state(0x20, 0, None), None);
        assert_eq!(
            sync_state(FILE_ATTRIBUTE_REPARSE_POINT, 0xA000_0003, None),
            None,
            "Junction"
        );
        // Tag ohne Reparse-Attribut ist Rauschen aus anderen Feldern.
        assert_eq!(sync_state(0x20, TAG, None), None);
    }

    #[test]
    fn pin_attributes_are_exclusive_and_keep_user_flags() {
        let current = FILE_ATTRIBUTE_ARCHIVE
            | FILE_ATTRIBUTE_HIDDEN
            | FILE_ATTRIBUTE_UNPINNED
            | FILE_ATTRIBUTE_REPARSE_POINT;
        let pinned = pin_attributes(current, true);
        assert_eq!(
            pinned,
            FILE_ATTRIBUTE_ARCHIVE | FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_PINNED
        );
        let freed = pin_attributes(pinned | FILE_ATTRIBUTE_PINNED, false);
        assert_eq!(
            freed,
            FILE_ATTRIBUTE_ARCHIVE | FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_UNPINNED
        );
        assert_eq!(
            pin_attributes(0x80, true),
            FILE_ATTRIBUTE_PINNED,
            "NORMAL verträgt sich mit nichts"
        );
    }

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
