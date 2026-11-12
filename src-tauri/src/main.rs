// Im Release-Build kein Konsolenfenster öffnen.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(not(windows))]
compile_error!("eX-plorer unterstützt ausschließlich Windows 10/11.");

mod commands;
mod error;
mod shell;
mod state;

use tauri::Manager;

fn main() {
    let result = tauri::Builder::default()
        .setup(|app| {
            let material = app
                .get_webview_window("main")
                .map(|window| state::apply_material(&window))
                .unwrap_or(state::Material::Solid);
            app.manage(state::AppState { material });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app::app_info,
            commands::app::sidebar,
            commands::fs::list_dir,
        ])
        .run(tauri::generate_context!());

    if let Err(err) = result {
        eprintln!("eX-plorer konnte nicht gestartet werden: {err}");
        std::process::exit(1);
    }
}
