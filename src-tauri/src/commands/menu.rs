use explorer_core::exec::is_risky_to_open;
use explorer_core::path::{validate_path, ValidPath};
use explorer_core::{attrs, CoreError};
use serde::{Deserialize, Serialize};
use std::os::windows::fs::MetadataExt;
use tauri::WebviewWindow;

use super::blocking;
use crate::error::AppError;
use crate::shell::ctxmenu::{self, Command, Extra, MenuItem, Outcome, Reply, Target, EXTRA_IDS};
use crate::shell::launch;

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MenuOutcome {
    Dismissed,
    Done,
    Verb { verb: String },
    NewTab,
    Extra { id: u32 },
}

impl From<Outcome> for MenuOutcome {
    fn from(outcome: Outcome) -> Self {
        match outcome {
            Outcome::Dismissed => Self::Dismissed,
            Outcome::Done => Self::Done,
            Outcome::Verb(verb) => Self::Verb { verb },
            Outcome::NewTab => Self::NewTab,
            Outcome::Extra(id) => Self::Extra { id },
        }
    }
}

/// Zusätzlicher Eintrag, den das Frontend ins Menü einfügen lässt.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuExtra {
    id: u32,
    label: String,
    #[serde(default)]
    skip_if: Vec<String>,
}

const MAX_EXTRAS: usize = 8;
const MAX_LABEL: usize = 100;

/// Nur Einträge im vorgesehenen ID-Bereich mit sinnvoller Beschriftung.
fn validated(extras: Vec<MenuExtra>) -> Vec<Extra> {
    extras
        .into_iter()
        .filter(|e| {
            EXTRA_IDS.contains(&e.id) && !e.label.trim().is_empty() && e.label.len() <= MAX_LABEL
        })
        .take(MAX_EXTRAS)
        .map(|e| Extra {
            id: e.id,
            label: e.label,
            skip_if: e.skip_if.into_iter().take(8).collect(),
        })
        .collect()
}

fn owner(window: &WebviewWindow) -> isize {
    window.hwnd().map_or(0, |hwnd| hwnd.0 as isize)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuOpened {
    session: u32,
    items: Vec<MenuItem>,
}

fn target_of(paths: &[String], folder: Option<String>) -> Result<Target, AppError> {
    if paths.is_empty() {
        let folder = folder.ok_or(CoreError::Unsupported)?;
        return Ok(Target::Background(validate_path(&folder)?));
    }
    let valid: Vec<ValidPath> = paths
        .iter()
        .take(explorer_core::ops::MAX_ITEMS)
        .map(|p| validate_path(p))
        .collect::<Result<_, _>>()?;
    // Die Shell erwartet Elemente aus einem Ordner.
    let first_parent = valid[0].parent();
    if valid.iter().any(|p| match (&first_parent, p.parent()) {
        (Some(a), Some(b)) => !a.same_as(&b),
        (None, None) => false,
        _ => true,
    }) {
        return Err(CoreError::Unsupported.into());
    }
    Ok(Target::Items(valid))
}

/// Baut das Kontextmenü auf und liefert dessen Einträge; angezeigt wird es
/// von der Oberfläche. Mit `paths` für Elemente (alle im selben Ordner),
/// sonst für die leere Fläche von `folder`.
#[tauri::command]
pub async fn menu_open(
    window: WebviewWindow,
    paths: Vec<String>,
    folder: Option<String>,
    new_tab_label: Option<String>,
    extras: Option<Vec<MenuExtra>>,
) -> Result<MenuOpened, AppError> {
    let owner = owner(&window);
    let target = target_of(&paths, folder)?;
    let extras = validated(extras.unwrap_or_default());
    blocking(move || {
        match ctxmenu::send(Command::Open {
            target,
            owner,
            new_tab_label,
            extras,
        })? {
            Reply::Menu { session, items } => Ok(MenuOpened { session, items }),
            _ => Err(CoreError::Unsupported.into()),
        }
    })
    .await
}

/// Einträge eines Untermenüs; `path` sind die Positionen ab der obersten Ebene.
#[tauri::command]
pub async fn menu_submenu(session: u32, path: Vec<u32>) -> Result<Vec<MenuItem>, AppError> {
    blocking(
        move || match ctxmenu::send(Command::Submenu { session, path })? {
            Reply::Items(items) => Ok(items),
            _ => Err(CoreError::Unsupported.into()),
        },
    )
    .await
}

/// Führt einen Eintrag des Menüs aus und schließt die Sitzung.
#[tauri::command]
pub async fn menu_invoke(session: u32, id: u32) -> Result<MenuOutcome, AppError> {
    blocking(
        move || match ctxmenu::send(Command::Invoke { session, id })? {
            Reply::Outcome(outcome) => Ok(outcome.into()),
            _ => Err(CoreError::Unsupported.into()),
        },
    )
    .await
}

/// „Weitere Optionen“: zeigt das native Windows-Menü derselben Sitzung.
#[tauri::command]
pub async fn menu_native(session: u32) -> Result<MenuOutcome, AppError> {
    blocking(move || match ctxmenu::send(Command::Native { session })? {
        Reply::Outcome(outcome) => Ok(outcome.into()),
        _ => Err(CoreError::Unsupported.into()),
    })
    .await
}

/// Schließt das Menü ohne Auswahl.
#[tauri::command]
pub async fn menu_close(session: u32) -> Result<(), AppError> {
    blocking(move || {
        ctxmenu::send(Command::Close { session })?;
        Ok(())
    })
    .await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenResult {
    /// Die Datei kann Code ausführen; die Oberfläche muss nachfragen und
    /// danach mit `confirmed` erneut aufrufen.
    needs_confirm: bool,
}

/// Öffnet eine Datei mit dem verknüpften Programm. Nur auf ausdrückliche
/// Handlung; ausführbare Typen erst nach Bestätigung.
#[tauri::command]
pub async fn open_item(
    window: WebviewWindow,
    path: String,
    confirmed: bool,
) -> Result<OpenResult, AppError> {
    let path = validate_path(&path)?;
    let owner = owner(&window);
    blocking(move || {
        let meta = std::fs::symlink_metadata(path.to_extended()).map_err(CoreError::from)?;
        if meta.file_attributes() & attrs::FILE_ATTRIBUTE_DIRECTORY != 0 {
            return Err(CoreError::NotAFile.into());
        }
        if is_risky_to_open(path.file_name().unwrap_or_default()) && !confirmed {
            return Ok(OpenResult {
                needs_confirm: true,
            });
        }
        launch::open(owner, &path)?;
        Ok(OpenResult {
            needs_confirm: false,
        })
    })
    .await
}
