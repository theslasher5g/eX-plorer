//! OneDrive-Konten: Zusammenführen der Quellen (Registry, Umgebungsvariablen)
//! zu einer Liste ohne Doppelte. Die Quellen selbst liest die App-Schicht.

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

/// Prüft die Funde (gültiger Pfad, Ordner existiert) und entfernt Doppelte.
/// Funde aus der Registry stehen vor denen aus Umgebungsvariablen, weil nur
/// sie die E-Mail-Adresse kennen. Privat zuerst, dann Geschäft nach Name.
pub fn merge(raw: Vec<Raw>, is_dir: &dyn Fn(&ValidPath) -> bool) -> Vec<Account> {
    let mut sorted = raw;
    sorted.sort_by_key(|r| matches!(r.origin, Origin::Env(_)));

    let mut accounts: Vec<Account> = Vec::new();
    for item in sorted {
        let Ok(path) = validate_path(&item.folder) else {
            continue;
        };
        if path.is_root() || !is_dir(&path) || accounts.iter().any(|a| a.path.same_as(&path)) {
            continue;
        }
        let label = path.file_name().unwrap_or("OneDrive").to_owned();
        accounts.push(Account {
            kind: kind_of(&item.origin, &label),
            label,
            email: item.email.filter(|e| !e.trim().is_empty()),
            path,
        });
    }
    accounts.sort_by(|a, b| {
        (a.kind == AccountKind::Business, a.label.to_lowercase())
            .cmp(&(b.kind == AccountKind::Business, b.label.to_lowercase()))
    });
    accounts
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
}
