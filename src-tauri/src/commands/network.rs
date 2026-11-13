//! Netzwerklaufwerke und Freigaben. Alles läuft asynchron und hinter einem
//! Zeitlimit, damit ein toter Server nie die Oberfläche blockiert.

use std::time::Duration;

use explorer_core::network::{parse_server, Reachability};
use explorer_core::path::validate_path;
use explorer_core::CoreError;
use serde::Serialize;
use tauri::{State, WebviewWindow};

use super::blocking;
use crate::error::AppError;
use crate::shell::network;
use crate::state::AppState;

/// Wie lange auf eine Antwort gewartet wird.
const PROBE_LIMIT: Duration = Duration::from_secs(4);
const SHARES_LIMIT: Duration = Duration::from_secs(8);
/// So lange scheitern weitere Anfragen an einen hängenden Server sofort.
pub const COOL_DOWN: Duration = Duration::from_secs(15);

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetDrive {
    /// Laufwerksbuchstabe, z. B. `Z`.
    letter: String,
    /// Wurzelpfad, z. B. `Z:\`.
    path: String,
    /// `\\server\freigabe`, wenn bekannt.
    remote: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareInfo {
    name: String,
    comment: String,
    path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerRefDto {
    /// `\\server`
    server: String,
    /// `\\server\freigabe`, wenn angegeben.
    share: Option<String>,
}

/// Zugeordnete Netzlaufwerke (nur lokale Tabelle, ohne Netzwerkzugriff).
#[tauri::command]
pub async fn network_drives() -> Result<Vec<NetDrive>, AppError> {
    blocking(|| {
        Ok(network::mapped_drives()
            .into_iter()
            .map(|drive| NetDrive {
                letter: drive.letter.to_string(),
                path: format!("{}:\\", drive.letter),
                remote: drive.remote,
            })
            .collect())
    })
    .await
}

/// Prüft die Erreichbarkeit einer Wurzel (`Z:\` oder `\\server\freigabe`).
/// Zeitüberschreitung gilt als „nicht erreichbar“.
#[tauri::command]
pub async fn network_probe(
    state: State<'_, AppState>,
    path: String,
) -> Result<Reachability, AppError> {
    let valid = validate_path(&path)?;
    let guard = state.net.clone();
    let root = valid.root().to_owned();
    blocking(move || {
        let target = validate_path(&root)?.to_extended();
        match guard.run(&root, PROBE_LIMIT, COOL_DOWN, move |_| {
            Ok(network::probe(&target))
        }) {
            Ok(status) => Ok(status),
            Err(CoreError::Timeout) => Ok(Reachability::Offline),
            Err(err) => Err(err.into()),
        }
    })
    .await
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectResult {
    /// `false`: Der Benutzer hat den Windows-Dialog abgebrochen.
    connected: bool,
}

/// Stellt eine Verbindung her; bei Bedarf fragt der Windows-Dialog nach
/// Anmeldedaten. `target` ist `Z:\` (zugeordnetes Laufwerk neu verbinden) oder
/// `\\server\freigabe`.
#[tauri::command]
pub async fn network_connect(
    window: WebviewWindow,
    target: String,
) -> Result<ConnectResult, AppError> {
    let owner = window.hwnd().map_or(0, |hwnd| hwnd.0 as isize);
    // Verwaltungsfreigaben wie IPC$ sind erlaubt (dient der Anmeldung am Server).
    let valid = validate_path(&target)?;
    blocking(move || {
        let (remote, local) = if valid.is_unc() {
            (valid.root().to_owned(), None)
        } else {
            let letter = valid.root().chars().next().ok_or(CoreError::Unsupported)?;
            let remote = network::remote_name(letter).ok_or(CoreError::NotFound)?;
            (remote, Some(letter))
        };
        Ok(ConnectResult {
            connected: network::connect(owner, &remote, local)?,
        })
    })
    .await
}

/// Prüft eine Servereingabe und gibt sie normalisiert zurück.
#[tauri::command]
pub fn network_parse(input: String) -> Result<ServerRefDto, AppError> {
    let parsed = parse_server(&input)?;
    Ok(ServerRefDto {
        server: parsed.server_path(),
        share: parsed.share_path(),
    })
}

/// Datenfreigaben eines Servers (`\\server`).
#[tauri::command]
pub async fn network_shares(
    state: State<'_, AppState>,
    server: String,
) -> Result<Vec<ShareInfo>, AppError> {
    let parsed = parse_server(&server)?;
    let host = parsed.host().to_owned();
    let guard = state.net.clone();
    blocking(move || {
        let key = host.clone();
        let shares = guard.run(&key, SHARES_LIMIT, COOL_DOWN, move |_| {
            network::shares(&host).map(|list| (host, list))
        })?;
        let (host, list) = shares;
        Ok(list
            .into_iter()
            .map(|share| ShareInfo {
                path: format!(r"\\{host}\{}", share.name),
                name: share.name,
                comment: share.comment,
            })
            .collect())
    })
    .await
}
