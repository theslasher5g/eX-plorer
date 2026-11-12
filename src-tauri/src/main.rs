// Im Release-Build kein Konsolenfenster öffnen.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(windows))]
compile_error!("eX-plorer unterstützt ausschließlich Windows 10/11.");

mod commands;
mod error;
mod shell;
mod state;

use std::sync::Mutex;

use explorer_core::watch::DirWatcher;
use tauri::{Emitter, Manager};

fn main() {
    let result = tauri::Builder::default()
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
        ])
        .run(tauri::generate_context!());

    if let Err(err) = result {
        eprintln!("eX-plorer konnte nicht gestartet werden: {err}");
        std::process::exit(1);
    }
}
