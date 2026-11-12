//! Dateioperationen. Jeder Command validiert seine Eingaben, lässt den Kern
//! planen und führt dann über die Windows-Shell aus (blockierend, daher auf
//! einem Worker-Thread).

use explorer_core::ops::{self, Kind, Mode};
use explorer_core::path::{validate_path, ValidPath};
use explorer_core::CoreError;
use serde::{Deserialize, Serialize};
use tauri::WebviewWindow;

use super::blocking;
use crate::error::AppError;
use crate::shell::{clipboard, dragdrop, fileop, keys};

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransferMode {
    Auto,
    Copy,
    Move,
}

impl From<TransferMode> for Mode {
    fn from(mode: TransferMode) -> Self {
        match mode {
            TransferMode::Auto => Self::Auto,
            TransferMode::Copy => Self::Copy,
            TransferMode::Move => Self::Move,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpResult {
    /// Der Benutzer hat im Windows-Dialog abgebrochen.
    aborted: bool,
    /// Anzahl der bearbeiteten Elemente.
    count: usize,
    /// Es wurde verschoben (nicht kopiert oder gelöscht).
    moved: bool,
    /// Namen der bearbeiteten Elemente (zum Markieren nach dem Neuladen).
    names: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameResult {
    name: String,
    path: String,
    aborted: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardState {
    paths: Vec<String>,
    cut: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PasteResult {
    #[serde(flatten)]
    result: OpResult,
    cut: bool,
}

fn owner(window: &WebviewWindow) -> isize {
    window.hwnd().map_or(0, |hwnd| hwnd.0 as isize)
}

fn validate_all(paths: &[String]) -> Result<Vec<ValidPath>, AppError> {
    if paths.len() > ops::MAX_ITEMS {
        return Err(CoreError::TooMany.into());
    }
    paths
        .iter()
        .map(|p| validate_path(p).map_err(AppError::from))
        .collect()
}

fn names_of(paths: &[ValidPath]) -> Vec<String> {
    paths
        .iter()
        .filter_map(|p| p.file_name().map(str::to_owned))
        .collect()
}

/// Gemeinsamer Ausführungsweg für Kopieren, Verschieben und Einfügen.
fn run_transfer(
    owner: isize,
    sources: Vec<ValidPath>,
    dest: ValidPath,
    kind: Kind,
) -> Result<OpResult, AppError> {
    let plan = ops::plan_transfer(sources, dest, kind)?;
    let count = plan.count();
    if count == 0 {
        return Ok(OpResult {
            aborted: false,
            count: 0,
            moved: false,
            names: Vec::new(),
        });
    }
    let names: Vec<String> = plan
        .batches
        .iter()
        .flat_map(|b| names_of(&b.sources))
        .collect();
    let outcome = fileop::transfer(owner, &plan)?;
    Ok(OpResult {
        aborted: outcome.aborted,
        count,
        moved: kind == Kind::Move,
        names,
    })
}

/// Legt einen neuen Ordner an und liefert den tatsächlich gewählten Namen.
#[tauri::command]
pub async fn new_folder(parent: String, base: String) -> Result<String, AppError> {
    let parent = validate_path(&parent)?;
    blocking(move || Ok(ops::create_unique_dir(&parent.to_extended(), &base)?)).await
}

#[tauri::command]
pub async fn rename_item(
    window: WebviewWindow,
    path: String,
    new_name: String,
) -> Result<RenameResult, AppError> {
    let path = validate_path(&path)?;
    let owner = owner(&window);
    blocking(move || {
        let Some(target) = ops::plan_rename(&path, &new_name)? else {
            return Ok(RenameResult {
                name: new_name,
                path: path.into_string(),
                aborted: false,
            });
        };
        let outcome = fileop::rename(owner, &path, &new_name)?;
        Ok(RenameResult {
            name: new_name,
            path: if outcome.aborted { path } else { target }.into_string(),
            aborted: outcome.aborted,
        })
    })
    .await
}

#[tauri::command]
pub async fn transfer(
    window: WebviewWindow,
    sources: Vec<String>,
    dest: String,
    mode: TransferMode,
) -> Result<OpResult, AppError> {
    let sources = validate_all(&sources)?;
    let dest = validate_path(&dest)?;
    let owner = owner(&window);
    // Tastenstatus sofort lesen: Er gehört zum Moment des Drops.
    let mods = keys::modifiers();
    blocking(move || {
        let same_volume = sources.iter().all(|s| s.same_volume(&dest));
        let kind = ops::resolve_kind(mode.into(), mods.ctrl, mods.shift, same_volume);
        run_transfer(owner, sources, dest, kind)
    })
    .await
}

#[tauri::command]
pub async fn delete_items(
    window: WebviewWindow,
    paths: Vec<String>,
    permanent: bool,
) -> Result<OpResult, AppError> {
    let paths = validate_all(&paths)?;
    let owner = owner(&window);
    blocking(move || {
        let paths = ops::plan_delete(paths)?;
        let names = names_of(&paths);
        let outcome = fileop::delete(owner, &paths, permanent)?;
        Ok(OpResult {
            aborted: outcome.aborted,
            count: paths.len(),
            moved: false,
            names,
        })
    })
    .await
}

#[tauri::command]
pub async fn clipboard_set(
    window: WebviewWindow,
    paths: Vec<String>,
    cut: bool,
) -> Result<(), AppError> {
    let paths: Vec<String> = validate_all(&paths)?
        .into_iter()
        .map(ValidPath::into_string)
        .collect();
    let owner = owner(&window);
    blocking(move || Ok(clipboard::set(owner, &paths, cut)?)).await
}

/// Größter Text, der in die Zwischenablage gelegt wird (Baum-Export).
const MAX_CLIPBOARD_TEXT: usize = 8 * 1024 * 1024;

#[tauri::command]
pub async fn clipboard_set_text(window: WebviewWindow, text: String) -> Result<(), AppError> {
    if text.len() > MAX_CLIPBOARD_TEXT {
        return Err(CoreError::TooLarge.into());
    }
    let owner = owner(&window);
    blocking(move || Ok(clipboard::set_text(owner, &text)?)).await
}

/// Liest die Datei-Zwischenablage. Nur Pfade, die unsere Prüfung bestehen,
/// werden weitergegeben.
fn read_clipboard(owner: isize) -> Result<Option<(Vec<ValidPath>, bool)>, AppError> {
    let Some(contents) = clipboard::get(owner)? else {
        return Ok(None);
    };
    let paths: Vec<ValidPath> = contents
        .paths
        .iter()
        .filter_map(|p| validate_path(p).ok())
        .collect();
    Ok((!paths.is_empty()).then_some((paths, contents.cut)))
}

#[tauri::command]
pub async fn clipboard_get(window: WebviewWindow) -> Result<Option<ClipboardState>, AppError> {
    let owner = owner(&window);
    blocking(move || {
        Ok(read_clipboard(owner)?.map(|(paths, cut)| ClipboardState {
            paths: paths.into_iter().map(ValidPath::into_string).collect(),
            cut,
        }))
    })
    .await
}

#[tauri::command]
pub async fn paste(window: WebviewWindow, dest: String) -> Result<PasteResult, AppError> {
    let dest = validate_path(&dest)?;
    let owner = owner(&window);
    blocking(move || {
        let (sources, cut) = read_clipboard(owner)?.ok_or(CoreError::NothingToPaste)?;
        let kind = if cut { Kind::Move } else { Kind::Copy };
        let result = run_transfer(owner, sources, dest, kind)?;
        // Nach dem Verschieben ist die Ablage verbraucht, wie im Explorer.
        if cut && !result.aborted && result.count > 0 {
            let _ = clipboard::clear(owner);
        }
        Ok(PasteResult { result, cut })
    })
    .await
}

/// Startet einen OLE-Drag. Läuft modal auf dem UI-Thread und kehrt erst nach
/// dem Loslassen zurück; Ergebnis ist der gewählte Drop-Effekt (0 = abgebrochen).
#[tauri::command]
pub async fn start_drag(window: WebviewWindow, paths: Vec<String>) -> Result<u32, AppError> {
    let paths = validate_all(&paths)?;
    if paths.is_empty() {
        return Ok(0);
    }
    let (tx, rx) = std::sync::mpsc::channel();
    window
        .run_on_main_thread(move || {
            let _ = tx.send(dragdrop::run(&paths));
        })
        .map_err(|e| AppError::Internal(e.to_string()))?;
    blocking(move || {
        rx.recv()
            .map_err(|e| AppError::Internal(e.to_string()))?
            .map_err(AppError::from)
    })
    .await
}
