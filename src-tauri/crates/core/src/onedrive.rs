//! OneDrive-Konten: Zusammenführen der Quellen (Registry, Umgebungsvariablen)
//! zu einer Liste ohne Doppelte. Die Quellen selbst liest die App-Schicht.

use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::path::{validate_path, ValidPath};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AccountKind {
    Personal,
    Business,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Origin {
    /// Unterschlüssel unter `HKCU\Software\Microsoft\OneDrive\Accounts`, z. B. `Business1`.
    Registry(String),
    /// Umgebungsvariable, z. B. `OneDriveCommercial`.
    Env(String),
    /// Ordner im Benutzerprofil, dessen Name mit „OneDrive“ beginnt.
    Profile,
    /// Vom Benutzer in den Einstellungen hinzugefügt.
    Manual,
}

/// Ein ungeprüfter Fund aus einer Quelle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raw {
    pub origin: Origin,
    pub folder: String,
    pub email: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub kind: AccountKind,
    /// Ordnername, z. B. „OneDrive - Contoso“.
    pub label: String,
    pub email: Option<String>,
    pub path: ValidPath,
}

fn kind_of(origin: &Origin, label: &str) -> AccountKind {
    let personal = match origin {
        Origin::Registry(key) => key.to_lowercase().starts_with("personal"),
        // „OneDrive“ allein ist der private Ordner, alles mit Zusatz („OneDrive - Firma“) geschäftlich.
        Origin::Profile | Origin::Manual => label.eq_ignore_ascii_case("onedrive"),
        Origin::Env(var) => match var.as_str() {
            "OneDriveConsumer" => true,
            "OneDriveCommercial" => false,
            // „OneDrive“ allein: der private Ordner heißt schlicht „OneDrive“.
            _ => label.eq_ignore_ascii_case("onedrive"),
        },
    };
    if personal {
        AccountKind::Personal
    } else {
        AccountKind::Business
    }
}

/// Was aus einem Fund geworden ist (für die Diagnose in den Einstellungen).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    /// Als Konto übernommen.
    Accepted,
    /// Derselbe Ordner wurde schon von einer anderen Quelle gefunden.
    Duplicate,
    /// Kein gültiger absoluter Pfad.
    Invalid,
    /// Der Ordner existiert nicht (oder ist kein Ordner).
    Missing,
    /// Ein Laufwerksstamm ist kein OneDrive-Ordner.
    Root,
}

/// Ein Fund samt Ergebnis, mit lesbarer Quelle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub source: String,
    pub folder: String,
    pub outcome: Outcome,
}

fn source_name(origin: &Origin) -> String {
    match origin {
        Origin::Registry(key) => format!("Registry {key}"),
        Origin::Env(var) => format!("%{var}%"),
        Origin::Profile => "Benutzerprofil".to_owned(),
        Origin::Manual => "Manuell".to_owned(),
    }
}

/// Wie [`merge`], liefert aber zusätzlich, was mit jedem Fund geschah.
pub fn merge_with_report(
    raw: Vec<Raw>,
    is_dir: &dyn Fn(&ValidPath) -> bool,
) -> (Vec<Account>, Vec<Finding>) {
    // Registry zuerst (kennt die E-Mail), dann Umgebung, Profil, manuell.
    let rank = |origin: &Origin| match origin {
        Origin::Registry(_) => 0,
        Origin::Env(_) => 1,
        Origin::Profile => 2,
        Origin::Manual => 3,
    };
    let mut sorted = raw;
    sorted.sort_by_key(|r| rank(&r.origin));

    let mut accounts: Vec<Account> = Vec::new();
    let mut findings: Vec<Finding> = Vec::new();
    for item in sorted {
        let mut report = |outcome| {
            findings.push(Finding {
                source: source_name(&item.origin),
                folder: item.folder.clone(),
                outcome,
            });
        };
        let Ok(path) = validate_path(&item.folder) else {
            report(Outcome::Invalid);
            continue;
        };
        if path.is_root() {
            report(Outcome::Root);
            continue;
        }
        if !is_dir(&path) {
            report(Outcome::Missing);
            continue;
        }
        if accounts.iter().any(|a| a.path.same_as(&path)) {
            report(Outcome::Duplicate);
            continue;
        }
        let label = path.file_name().unwrap_or("OneDrive").to_owned();
        accounts.push(Account {
            kind: kind_of(&item.origin, &label),
            label,
            email: item.email.clone().filter(|e| !e.trim().is_empty()),
            path,
        });
        report(Outcome::Accepted);
    }
    accounts.sort_by(|a, b| {
        (a.kind == AccountKind::Business, a.label.to_lowercase())
            .cmp(&(b.kind == AccountKind::Business, b.label.to_lowercase()))
    });
    (accounts, findings)
}

/// Prüft die Funde (gültiger Pfad, Ordner existiert) und entfernt Doppelte.
/// Funde aus der Registry stehen vor den übrigen, weil nur sie die
/// E-Mail-Adresse kennen. Privat zuerst, dann Geschäft nach Name.
pub fn merge(raw: Vec<Raw>, is_dir: &dyn Fn(&ValidPath) -> bool) -> Vec<Account> {
    merge_with_report(raw, is_dir).0
}

/// Sucht im Benutzerprofil nach Ordnern, die wie ein OneDrive-Ordner heißen
/// („OneDrive“, „OneDrive - Firma“). Hilft, wenn Registry und Umgebungsvariablen
/// nichts liefern. Es werden nur die Namen der direkten Unterordner gelesen.
pub fn scan_profile(profile: &Path) -> Vec<Raw> {
    let Ok(entries) = fs::read_dir(profile) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| {
            // Links (auch Junctions) nicht verfolgen.
            entry
                .file_type()
                .is_ok_and(|t| t.is_dir() && !t.is_symlink())
        })
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.to_lowercase().starts_with("onedrive").then(|| Raw {
                origin: Origin::Profile,
                folder: entry.path().to_string_lossy().into_owned(),
                email: None,
            })
        })
        .take(16)
        .collect()
}

/// Liegt `path` in einem der OneDrive-Ordner?
pub fn contains(accounts: &[Account], path: &ValidPath) -> bool {
    accounts.iter().any(|a| path.is_inside_or_same(&a.path))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reg(key: &str, folder: &str, email: Option<&str>) -> Raw {
        Raw {
            origin: Origin::Registry(key.into()),
            folder: folder.into(),
            email: email.map(Into::into),
        }
    }

    fn env(var: &str, folder: &str) -> Raw {
        Raw {
            origin: Origin::Env(var.into()),
            folder: folder.into(),
            email: None,
        }
    }

    fn all_exist(_: &ValidPath) -> bool {
        true
    }

    #[test]
    fn merges_registry_and_env_without_duplicates() {
        let accounts = merge(
            vec![
                env("OneDrive", r"C:\Users\Anna\OneDrive"),
                env("OneDriveCommercial", r"c:\users\anna\ONEDRIVE - Contoso\"),
                reg(
                    "Business1",
                    r"C:\Users\Anna\OneDrive - Contoso",
                    Some("anna@contoso.com"),
                ),
                reg("Personal", r"C:\Users\Anna\OneDrive", Some("anna@mail.de")),
            ],
            &all_exist,
        );
        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].kind, AccountKind::Personal);
        assert_eq!(accounts[0].email.as_deref(), Some("anna@mail.de"));
        assert_eq!(accounts[0].label, "OneDrive");
        assert_eq!(accounts[1].kind, AccountKind::Business);
        assert_eq!(
            accounts[1].email.as_deref(),
            Some("anna@contoso.com"),
            "Registry gewinnt"
        );
        assert_eq!(
            accounts[1].path.as_str(),
            r"C:\Users\Anna\OneDrive - Contoso"
        );
    }

    #[test]
    fn supports_several_business_accounts_sorted() {
        let accounts = merge(
            vec![
                reg("Business2", r"D:\OneDrive - Zeta", None),
                reg("Business1", r"D:\OneDrive - Alpha", None),
                reg("Personal", r"D:\OneDrive", None),
            ],
            &all_exist,
        );
        let labels: Vec<_> = accounts.iter().map(|a| a.label.as_str()).collect();
        assert_eq!(labels, ["OneDrive", "OneDrive - Alpha", "OneDrive - Zeta"]);
    }

    #[test]
    fn skips_invalid_missing_and_root_folders() {
        let accounts = merge(
            vec![
                reg("Personal", r"relativ\OneDrive", None),
                reg("Business1", r"C:\..\Windows", None),
                reg("Business2", r"\\.\C:\OneDrive", None),
                reg("Business3", r"C:\", None),
                reg("Business4", r"C:\Gone\OneDrive - X", None),
                reg("Business5", r"C:\Real\OneDrive - Y", None),
            ],
            &|p| !p.as_str().contains("Gone"),
        );
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].label, "OneDrive - Y");
    }

    #[test]
    fn guesses_kind_from_env_variable() {
        let accounts = merge(
            vec![
                env("OneDriveConsumer", r"C:\Users\A\Privat"),
                env("OneDriveCommercial", r"C:\Users\A\OneDrive"),
            ],
            &all_exist,
        );
        let privat = accounts.iter().find(|a| a.label == "Privat").unwrap();
        assert_eq!(privat.kind, AccountKind::Personal);
        let commercial = accounts.iter().find(|a| a.label == "OneDrive").unwrap();
        assert_eq!(
            commercial.kind,
            AccountKind::Business,
            "Variable schlägt den Namen"
        );
        let plain = merge(vec![env("OneDrive", r"C:\U\OneDrive - Firma")], &all_exist);
        assert_eq!(plain[0].kind, AccountKind::Business);
    }

    #[test]
    fn blank_email_is_dropped_and_contains_works() {
        let accounts = merge(
            vec![reg("Personal", r"C:\U\OneDrive", Some("  "))],
            &all_exist,
        );
        assert_eq!(accounts[0].email, None);
        let inside = validate_path(r"c:\u\onedrive\Dokumente\a.txt").unwrap();
        let outside = validate_path(r"C:\U\OneDrive2\a.txt").unwrap();
        assert!(contains(&accounts, &inside));
        assert!(!contains(&accounts, &outside), "Namenspräfix zählt nicht");
        assert!(contains(
            &accounts,
            &validate_path(r"C:\U\OneDrive").unwrap()
        ));
    }

    #[test]
    fn report_explains_every_finding() {
        let (accounts, findings) = merge_with_report(
            vec![
                reg("Business1", r"C:\U\OneDrive - Firma", Some("a@firma.de")),
                env("OneDriveCommercial", r"c:\u\onedrive - firma"),
                reg("Business2", r"C:\Gone\OneDrive - X", None),
                reg("Business3", r"relativ", None),
                reg("Business4", r"C:\", None),
            ],
            &|p| !p.as_str().contains("Gone"),
        );
        assert_eq!(accounts.len(), 1);
        let mut by_source: Vec<(String, Outcome)> = findings
            .iter()
            .map(|f| (f.source.clone(), f.outcome))
            .collect();
        by_source.sort_by(|a, b| a.0.cmp(&b.0));
        assert!(by_source.contains(&("Registry Business1".into(), Outcome::Accepted)));
        assert!(by_source.contains(&("Registry Business2".into(), Outcome::Missing)));
        assert!(by_source.contains(&("Registry Business3".into(), Outcome::Invalid)));
        assert!(by_source.contains(&("Registry Business4".into(), Outcome::Root)));
        assert!(by_source.contains(&("%OneDriveCommercial%".into(), Outcome::Duplicate)));
    }

    #[test]
    fn profile_and_manual_names_decide_the_kind() {
        let raw = |origin, folder: &str| Raw {
            origin,
            folder: folder.into(),
            email: None,
        };
        let accounts = merge(
            vec![
                raw(Origin::Profile, r"C:\U\OneDrive"),
                raw(Origin::Profile, r"C:\U\OneDrive - Muster GmbH"),
                raw(Origin::Manual, r"D:\Sync\OneDrive - Zweitfirma"),
            ],
            &all_exist,
        );
        let kinds: Vec<_> = accounts.iter().map(|a| a.kind).collect();
        assert_eq!(
            kinds,
            [
                AccountKind::Personal,
                AccountKind::Business,
                AccountKind::Business
            ]
        );
    }

    #[test]
    fn scans_the_profile_for_onedrive_folders() {
        let tmp = tempfile::tempdir().unwrap();
        for name in [
            "OneDrive",
            "OneDrive - Muster GmbH",
            "Documents",
            "onedrive-alt",
        ] {
            fs::create_dir(tmp.path().join(name)).unwrap();
        }
        fs::write(tmp.path().join("OneDrive.txt"), b"x").unwrap();
        let mut found: Vec<String> = scan_profile(tmp.path())
            .into_iter()
            .map(|r| {
                Path::new(&r.folder)
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        found.sort();
        assert_eq!(
            found,
            ["OneDrive", "OneDrive - Muster GmbH", "onedrive-alt"]
        );
        assert!(scan_profile(&tmp.path().join("nope")).is_empty());
    }
}
