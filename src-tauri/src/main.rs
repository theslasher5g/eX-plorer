// Im Release-Build kein Konsolenfenster öffnen.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(windows))]
compile_error!("eX-plorer unterstützt ausschließlich Windows 10/11.");

mod commands;
mod error;
mod protocol;
mod shell;
mod state;
mod workers;

use std::sync::Mutex;

use explorer_core::watch::DirWatcher;
use tauri::{Emitter, Manager};

fn main() {
    let result = tauri::Builder::default()
        .register_asynchronous_uri_scheme_protocol(protocol::SCHEME, |_ctx, request, responder| {
            protocol::handle(&request, responder);
        })
        .setup(|app| {
            let material = app
                .get_webview_window("main")
                .map(|window| state::apply_material(&window))
                .unwrap_or(state::Material::Solid);
            let handle = app.handle().clone();
            let watcher = DirWatcher::new(move |dir| {
                let _ = handle.emit("dir-changed", dir.to_string_lossy().into_owned());
            });
            app.manage(state::AppState {
                material,
                watcher: Mutex::new(watcher),
                scans: Mutex::default(),
                net: explorer_core::timeout::Guard::new(),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::app_info,
            commands::app::sidebar,
            commands::fs::list_dir,
            commands::ops::new_folder,
            commands::ops::rename_item,
            commands::ops::transfer,
            commands::ops::delete_items,
            commands::ops::clipboard_set,
            commands::ops::clipboard_get,
            commands::ops::paste,
            commands::ops::start_drag,
            commands::watch::watch_dirs,
            commands::preview::preview_info,
            commands::preview::preview_bytes,
            commands::preview::preview_text,
            commands::menu::context_menu,
            commands::menu::open_item,
            commands::onedrive::onedrive_set_pinned,
            commands::network::network_drives,
            commands::network::network_probe,
            commands::network::network_connect,
            commands::network::network_parse,
            commands::network::network_shares,
            commands::tree::tree_scan,
            commands::tree::tree_cancel,
            commands::settings::settings_get,
            commands::settings::settings_set,
            commands::ops::clipboard_set_text,
        ])
        .run(tauri::generate_context!());

    if let Err(err) = result {
        eprintln!("eX-plorer konnte nicht gestartet werden: {err}");
        std::process::exit(1);
    }
}
