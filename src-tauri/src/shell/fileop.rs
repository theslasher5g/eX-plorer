//! Dateioperationen über `IFileOperation`.
//!
//! Die Shell übernimmt Papierkorb, Konfliktdialoge, Fortschrittsanzeige und
//! den korrekten Umgang mit OneDrive-Platzhaltern. Alle Pfade sind vorher
//! vom Kern geprüft (`explorer_core::ops`). Die Funktionen müssen auf einem
//! Worker-Thread laufen (blockierend, öffnen ggf. Dialoge).

use explorer_core::ops::{Kind, TransferPlan};
use explorer_core::path::ValidPath;
use explorer_core::{winerr, CoreError};
use windows::core::{Error as WinError, HSTRING, PCWSTR};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
use windows::Win32::UI::Shell::{
    FileOperation, IFileOperation, FILEOPERATION_FLAGS, FOFX_RECYCLEONDELETE, FOF_ALLOWUNDO,
    FOF_RENAMEONCOLLISION,
};

use super::com::ComGuard;
use super::items::item;

/// Ergebnis einer Operation. `aborted`: Der Benutzer hat im Windows-Dialog abgebrochen.
#[derive(Debug, Clone, Copy)]
pub struct Outcome {
    pub aborted: bool,
}

fn map(err: &WinError) -> Result<Outcome, CoreError> {
    if winerr::is_cancelled(err.code().0) {
        Ok(Outcome { aborted: true })
    } else {
        Err(winerr::from_hresult(err.code().0))
    }
}

fn operation(owner: isize, flags: FILEOPERATION_FLAGS) -> Result<IFileOperation, WinError> {
    // SAFETY: Standard-Instanziierung einer COM-Klasse; COM ist durch den
    // ComGuard des aufrufenden Threads initialisiert.
    let op: IFileOperation = unsafe { CoCreateInstance(&FileOperation, None, CLSCTX_ALL) }?;
    // SAFETY: `op` ist ein gültiges COM-Objekt; der HWND stammt vom Hauptfenster
    // (oder ist 0, dann gibt es keinen Besitzer).
    unsafe {
        op.SetOperationFlags(flags)?;
        if owner != 0 {
            op.SetOwnerWindow(HWND(owner as *mut _))?;
        }
    }
    Ok(op)
}

fn perform(op: &IFileOperation) -> Result<Outcome, WinError> {
    // SAFETY: gültiges COM-Objekt; blockiert, bis die Operation fertig ist.
    unsafe {
        op.PerformOperations()?;
        let aborted = op.GetAnyOperationsAborted()?.as_bool();
        Ok(Outcome { aborted })
    }
}

fn run_transfer(owner: isize, plan: &TransferPlan) -> Result<Outcome, WinError> {
    let dest = item(&plan.dest)?;
    let mut aborted = false;
    for batch in &plan.batches {
        let flags = if batch.rename_on_collision {
            FOF_RENAMEONCOLLISION
        } else {
            FILEOPERATION_FLAGS(0)
        };
        let op = operation(owner, FOF_ALLOWUNDO | flags)?;
        for source in &batch.sources {
            let src = item(source)?;
            // SAFETY: gültige Shell-Items; ohne neuen Namen und ohne Fortschrittssenke.
            unsafe {
                match plan.kind {
                    Kind::Copy => op.CopyItem(&src, &dest, PCWSTR::null(), None)?,
                    Kind::Move => op.MoveItem(&src, &dest, PCWSTR::null(), None)?,
                }
            }
        }
        aborted |= perform(&op)?.aborted;
        if aborted {
            break;
        }
    }
    Ok(Outcome { aborted })
}

pub fn transfer(owner: isize, plan: &TransferPlan) -> Result<Outcome, CoreError> {
    let _com = ComGuard::sta();
    run_transfer(owner, plan).or_else(|e| map(&e))
}

fn run_delete(owner: isize, paths: &[ValidPath], permanent: bool) -> Result<Outcome, WinError> {
    // Mit Papierkorb: Wo die Shell keinen Papierkorb hat (Netzlaufwerk, USB),
    // fragt sie selbst nach, bevor sie endgültig löscht. Deshalb bewusst kein
    // FOF_NOCONFIRMATION.
    let flags = if permanent {
        FILEOPERATION_FLAGS(0)
    } else {
        FOF_ALLOWUNDO | FOFX_RECYCLEONDELETE
    };
    let op = operation(owner, flags)?;
    for path in paths {
        let it = item(path)?;
        // SAFETY: gültiges Shell-Item, keine Fortschrittssenke.
        unsafe { op.DeleteItem(&it, None)? };
    }
    perform(&op)
}

pub fn delete(owner: isize, paths: &[ValidPath], permanent: bool) -> Result<Outcome, CoreError> {
    let _com = ComGuard::sta();
    run_delete(owner, paths, permanent).or_else(|e| map(&e))
}

fn run_rename(owner: isize, path: &ValidPath, new_name: &str) -> Result<Outcome, WinError> {
    let op = operation(owner, FOF_ALLOWUNDO)?;
    let it = item(path)?;
    let name = HSTRING::from(new_name);
    // SAFETY: gültiges Shell-Item; `name` ist nullterminiert und überlebt den Aufruf.
    unsafe { op.RenameItem(&it, PCWSTR(name.as_ptr()), None)? };
    perform(&op)
}

pub fn rename(owner: isize, path: &ValidPath, new_name: &str) -> Result<Outcome, CoreError> {
    let _com = ComGuard::sta();
    run_rename(owner, path, new_name).or_else(|e| map(&e))
}
