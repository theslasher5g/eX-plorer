//! Planung von Dateioperationen. Hier wird entschieden, *ob und wie* etwas
//! passiert; die eigentliche Ausführung übernimmt die Windows-Shell
//! (`IFileOperation`) in der App-Schicht. So bleibt die Logik testbar und die
//! Schutzregeln liegen an einer Stelle.

use std::io;
use std::path::Path;

use crate::path::{validate_name, ValidPath};
use crate::CoreError;

/// Obergrenze für Elemente pro Operation, als Schutz vor missbräuchlichen
/// oder fehlerhaften Aufrufen.
pub const MAX_ITEMS: usize = 20_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Copy,
    Move,
}

/// Gewünschter Modus aus dem Frontend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Wie im Explorer: gleiches Laufwerk verschiebt, anderes kopiert;
    /// Strg erzwingt Kopieren, Umschalt Verschieben.
    Auto,
    Copy,
    Move,
}

pub fn resolve_kind(mode: Mode, ctrl: bool, shift: bool, same_volume: bool) -> Kind {
    match mode {
        Mode::Copy => Kind::Copy,
        Mode::Move => Kind::Move,
        Mode::Auto if ctrl => Kind::Copy,
        Mode::Auto if shift => Kind::Move,
        Mode::Auto if same_volume => Kind::Move,
        Mode::Auto => Kind::Copy,
    }
}

/// Eine Gruppe von Quellen, die mit denselben Optionen verarbeitet wird.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch {
    pub sources: Vec<ValidPath>,
    /// Bei Namenskollision automatisch „ – Kopie“ anhängen statt zu fragen.
    /// Nur sinnvoll beim Kopieren in den eigenen Ordner (Duplizieren).
    pub rename_on_collision: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferPlan {
    pub kind: Kind,
    pub dest: ValidPath,
    pub batches: Vec<Batch>,
}

impl TransferPlan {
    pub fn count(&self) -> usize {
        self.batches.iter().map(|b| b.sources.len()).sum()
    }
}

/// Entfernt doppelte Pfade und solche, die bereits in einem ebenfalls
/// ausgewählten Ordner enthalten sind (der Ordner nimmt sie mit).
fn collapse(paths: Vec<ValidPath>) -> Vec<ValidPath> {
    let mut kept: Vec<ValidPath> = Vec::with_capacity(paths.len());
    for path in paths {
        if kept.iter().any(|k| path.is_inside_or_same(k)) {
            continue;
        }
        kept.retain(|k| !k.is_inside_or_same(&path));
        kept.push(path);
    }
    kept
}

fn check_count(len: usize) -> Result<(), CoreError> {
    if len > MAX_ITEMS {
        return Err(CoreError::TooMany);
    }
    Ok(())
}

pub fn plan_transfer(
    sources: Vec<ValidPath>,
    dest: ValidPath,
    kind: Kind,
) -> Result<TransferPlan, CoreError> {
    check_count(sources.len())?;
    let sources = collapse(sources);
    if sources.iter().any(ValidPath::is_root) {
        return Err(CoreError::RootNotAllowed);
    }
    // Ziel darf nicht der Quellordner selbst oder darin liegen.
    if sources.iter().any(|s| dest.is_inside_or_same(s)) {
        return Err(CoreError::IntoItself);
    }

    let (same_parent, others): (Vec<_>, Vec<_>) = sources
        .into_iter()
        .partition(|s| s.parent().is_some_and(|p| p.same_as(&dest)));

    let mut batches = Vec::new();
    if !others.is_empty() {
        batches.push(Batch {
            sources: others,
            rename_on_collision: false,
        });
    }
    // Verschieben in den eigenen Ordner ist ein No-op; Kopieren dupliziert.
    if kind == Kind::Copy && !same_parent.is_empty() {
        batches.push(Batch {
            sources: same_parent,
            rename_on_collision: true,
        });
    }
    Ok(TransferPlan {
        kind,
        dest,
        batches,
    })
}

pub fn plan_delete(paths: Vec<ValidPath>) -> Result<Vec<ValidPath>, CoreError> {
    check_count(paths.len())?;
    let paths = collapse(paths);
    if paths.iter().any(ValidPath::is_root) {
        return Err(CoreError::RootNotAllowed);
    }
    Ok(paths)
}

/// Prüft eine Umbenennung. `None` heißt: Name unverändert, nichts zu tun.
pub fn plan_rename(path: &ValidPath, new_name: &str) -> Result<Option<ValidPath>, CoreError> {
    if path.is_root() {
        return Err(CoreError::RootNotAllowed);
    }
    validate_name(new_name)?;
    if path.file_name() == Some(new_name) {
        return Ok(None);
    }
    let parent = path.parent().ok_or(CoreError::RootNotAllowed)?;
    Ok(Some(parent.join(new_name)?))
}

/// Legt unterhalb von `parent` einen neuen Ordner an: `base`, sonst
/// `base (2)`, `base (3)` …. Atomar über `create_dir`, damit parallele
/// Änderungen nichts überschreiben. Liefert den gewählten Namen.
pub fn create_unique_dir(parent: &Path, base: &str) -> Result<String, CoreError> {
    validate_name(base)?;
    for n in 1..=999u32 {
        let name = if n == 1 {
            base.to_owned()
        } else {
            format!("{base} ({n})")
        };
        match std::fs::create_dir(parent.join(&name)) {
            Ok(()) => return Ok(name),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err.into()),
        }
    }
    Err(CoreError::AlreadyExists)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::validate_path;

    fn vp(s: &str) -> ValidPath {
        validate_path(s).unwrap()
    }

    fn names(plan: &TransferPlan) -> Vec<Vec<String>> {
        plan.batches
            .iter()
            .map(|b| b.sources.iter().map(|s| s.as_str().to_owned()).collect())
            .collect()
    }

    #[test]
    fn resolves_kind_like_explorer() {
        assert_eq!(resolve_kind(Mode::Auto, false, false, true), Kind::Move);
        assert_eq!(resolve_kind(Mode::Auto, false, false, false), Kind::Copy);
        assert_eq!(resolve_kind(Mode::Auto, true, false, true), Kind::Copy);
        assert_eq!(resolve_kind(Mode::Auto, false, true, false), Kind::Move);
        assert_eq!(resolve_kind(Mode::Copy, false, true, true), Kind::Copy);
        assert_eq!(resolve_kind(Mode::Move, true, false, false), Kind::Move);
    }

    #[test]
    fn plans_simple_copy() {
        let plan = plan_transfer(
            vec![vp(r"C:\a\x.txt"), vp(r"C:\a\y")],
            vp(r"D:\dest"),
            Kind::Copy,
        )
        .unwrap();
        assert_eq!(plan.count(), 2);
        assert_eq!(plan.batches.len(), 1);
        assert!(!plan.batches[0].rename_on_collision);
    }

    #[test]
    fn move_into_own_parent_is_noop() {
        let plan = plan_transfer(vec![vp(r"C:\a\x.txt")], vp(r"c:\A"), Kind::Move).unwrap();
        assert_eq!(plan.count(), 0);
    }

    #[test]
    fn copy_into_own_parent_duplicates_with_auto_rename() {
        let plan = plan_transfer(
            vec![vp(r"C:\a\x.txt"), vp(r"C:\b\y.txt")],
            vp(r"C:\a"),
            Kind::Copy,
        )
        .unwrap();
        assert_eq!(plan.batches.len(), 2);
        assert_eq!(names(&plan), vec![vec![r"C:\b\y.txt"], vec![r"C:\a\x.txt"]]);
        assert!(!plan.batches[0].rename_on_collision);
        assert!(plan.batches[1].rename_on_collision);
    }

    #[test]
    fn rejects_folder_into_itself_or_descendant() {
        for kind in [Kind::Copy, Kind::Move] {
            let err = plan_transfer(vec![vp(r"C:\a")], vp(r"C:\a"), kind).unwrap_err();
            assert!(matches!(err, CoreError::IntoItself));
            let err = plan_transfer(vec![vp(r"C:\a")], vp(r"c:\A\sub\deep"), kind).unwrap_err();
            assert!(matches!(err, CoreError::IntoItself));
        }
        // Ähnlicher Name ist erlaubt.
        assert!(plan_transfer(vec![vp(r"C:\a")], vp(r"C:\ab"), Kind::Copy).is_ok());
    }

    #[test]
    fn rejects_roots() {
        let err = plan_transfer(vec![vp(r"C:\")], vp(r"D:\x"), Kind::Copy).unwrap_err();
        assert!(matches!(err, CoreError::RootNotAllowed));
        let err = plan_transfer(vec![vp(r"\\srv\share")], vp(r"D:\x"), Kind::Move).unwrap_err();
        assert!(matches!(err, CoreError::RootNotAllowed));
        assert!(matches!(
            plan_delete(vec![vp(r"E:\")]),
            Err(CoreError::RootNotAllowed)
        ));
    }

    #[test]
    fn collapses_duplicates_and_nested_selection() {
        let plan = plan_transfer(
            vec![vp(r"C:\a\sub\f.txt"), vp(r"C:\a"), vp(r"c:\A"), vp(r"C:\b")],
            vp(r"D:\x"),
            Kind::Copy,
        )
        .unwrap();
        assert_eq!(names(&plan), vec![vec![r"C:\a", r"C:\b"]]);
        let del = plan_delete(vec![vp(r"C:\a\f"), vp(r"C:\a")]).unwrap();
        assert_eq!(del.len(), 1);
    }

    #[test]
    fn limits_item_count() {
        let many: Vec<_> = (0..=MAX_ITEMS)
            .map(|i| vp(&format!(r"C:\a\f{i}")))
            .collect();
        assert!(matches!(plan_delete(many), Err(CoreError::TooMany)));
    }

    #[test]
    fn plans_rename() {
        let p = vp(r"C:\a\old.txt");
        assert_eq!(
            plan_rename(&p, "new.txt").unwrap().unwrap().as_str(),
            r"C:\a\new.txt"
        );
        assert_eq!(plan_rename(&p, "old.txt").unwrap(), None);
        // Nur Groß-/Kleinschreibung ändern ist eine echte Umbenennung.
        assert!(plan_rename(&p, "OLD.txt").unwrap().is_some());
        assert!(plan_rename(&p, "").is_err());
        assert!(plan_rename(&p, r"..\evil").is_err());
        assert!(plan_rename(&p, "a:b").is_err());
        assert!(plan_rename(&p, "NUL").is_err());
        assert!(plan_rename(&p, "x.").is_err());
        assert!(matches!(
            plan_rename(&vp(r"C:\"), "x"),
            Err(CoreError::RootNotAllowed)
        ));
    }

    #[test]
    fn unique_dir_counts_up() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(
            create_unique_dir(tmp.path(), "Neuer Ordner").unwrap(),
            "Neuer Ordner"
        );
        assert_eq!(
            create_unique_dir(tmp.path(), "Neuer Ordner").unwrap(),
            "Neuer Ordner (2)"
        );
        assert_eq!(
            create_unique_dir(tmp.path(), "Neuer Ordner").unwrap(),
            "Neuer Ordner (3)"
        );
        assert!(tmp.path().join("Neuer Ordner (3)").is_dir());
    }

    #[test]
    fn unique_dir_rejects_bad_base_and_missing_parent() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(create_unique_dir(tmp.path(), "a/b").is_err());
        assert!(matches!(
            create_unique_dir(&tmp.path().join("nope"), "x"),
            Err(CoreError::NotFound)
        ));
    }
}
