#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod engine;
mod game;
mod install;
mod packs;
mod progress;
mod roadmap;
mod syntax;
mod updates;

use std::{path::PathBuf, sync::Mutex};
use tauri::{Emitter, Manager, image::Image};

static BACKEND_LOCK: Mutex<()> = Mutex::new(());

fn data_root(root: &std::path::Path) -> Result<PathBuf, String> {
    #[cfg(feature = "store")]
    {
        let _ = root;
        let local_data = std::env::var_os("LOCALAPPDATA")
            .ok_or("Could not locate the local application data folder.")?;
        let path = PathBuf::from(local_data).join("DeadVox");
        std::fs::create_dir_all(&path).map_err(|error| error.to_string())?;
        return Ok(path);
    }
    #[cfg(not(feature = "store"))]
    Ok(root.join("user-data"))
}

fn native_action(
    action: &str,
    game_path: Option<&str>,
    reporter: &progress::Reporter,
) -> Result<engine::Snapshot, String> {
    let _guard = BACKEND_LOCK
        .try_lock()
        .map_err(|_| "Please wait for the current operation to finish.".to_string())?;
    let _operation = progress::Operation::acquire()?;
    let root = engine::executable_root()?;
    let data = data_root(&root)?;
    #[cfg(feature = "store")]
    if action == "update_client" {
        return Err("The Microsoft Store updates DeadVox automatically.".into());
    }
    engine::action_with_progress(action, &root, &data, game_path, reporter)
}

#[tauri::command]
async fn launcher_action(
    app: tauri::AppHandle,
    action: String,
    game_path: Option<String>,
) -> Result<engine::Snapshot, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let reporter = move |phase: &str, percent: Option<f64>| {
            let _ = app.emit(
                "operation-progress",
                progress::Progress {
                    phase: phase.into(),
                    percent,
                },
            );
        };
        native_action(&action, game_path.as_deref(), &reporter)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn roadmap_audio(character: String, dubbed: bool) -> Result<Vec<u8>, String> {
    tauri::async_runtime::spawn_blocking(move || roadmap::sample(&character, dubbed))
        .await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn set_theme_icon(app: tauri::AppHandle, theme: String) -> Result<(), String> {
    let bytes = match theme.as_str() {
        "light" => include_bytes!("../icons/icon-light.png") as &[u8],
        "dark" => include_bytes!("../icons/icon-dark.png") as &[u8],
        _ => return Err("Unknown theme icon.".into()),
    };
    let icon = Image::from_bytes(bytes).map_err(|error| error.to_string())?;
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window is unavailable.".to_string())?;
    window.set_icon(icon).map_err(|error| error.to_string())
}

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    if let Some(action) = arguments
        .iter()
        .find_map(|arg| arg.strip_prefix("--check="))
    {
        let result = native_action(
            action,
            arguments
                .iter()
                .find_map(|arg| arg.strip_prefix("--game-path=")),
            &progress::quiet,
        );
        let root = engine::executable_root().unwrap();
        let data = data_root(&root).unwrap_or_else(|_| root.join("user-data"));
        let path = arguments
            .iter()
            .find_map(|arg| arg.strip_prefix("--report="))
            .map(PathBuf::from)
            .unwrap_or_else(|| data.join("native-check.json"));
        let (payload, exit_code) = match result {
            Ok(value) => (serde_json::to_value(value).unwrap(), 0),
            Err(error) => (serde_json::json!({ "error": error }), 1),
        };
        if install::write_atomic(&path, &serde_json::to_vec_pretty(&payload).unwrap()).is_err() {
            std::process::exit(2);
        }
        std::process::exit(exit_code);
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if progress::busy() {
                    api.prevent_close();
                    let _ = window.emit(
                        "close-blocked",
                        "Please wait for the current operation to finish.",
                    );
                }
            }
        })
        .invoke_handler(tauri::generate_handler![launcher_action, set_theme_icon, roadmap_audio])
        .build(tauri::generate_context!())
        .expect("Unable to start DeadVox")
        .run(|app, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                if progress::busy() {
                    api.prevent_exit();
                    let _ = app.emit(
                        "close-blocked",
                        "Please wait for the current operation to finish.",
                    );
                }
            }
        });
}
