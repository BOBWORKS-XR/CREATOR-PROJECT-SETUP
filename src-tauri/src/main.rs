#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod logic;

use logic::{CreateRequest, CreationResult, EnvironmentReport, Recipe};
use tauri::Emitter;
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
fn get_recipe() -> Recipe {
    logic::recipe()
}

#[tauri::command]
fn probe_environment() -> EnvironmentReport {
    logic::probe_environment()
}

#[tauri::command]
fn pick_parent_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    Ok(app
        .dialog()
        .file()
        .blocking_pick_folder()
        .map(|path| path.to_string()))
}

#[tauri::command]
async fn create_project(
    app: tauri::AppHandle,
    request: CreateRequest,
) -> Result<CreationResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        logic::create_project(request, |progress| {
            let _ = app.emit("setup-progress", progress);
        })
    })
    .await
    .map_err(|error| format!("Setup worker failed: {error}"))?
}

#[tauri::command]
fn open_project(path: String) -> Result<(), String> {
    logic::open_project(&path)
}

#[tauri::command]
fn launch_hub() -> Result<(), String> {
    logic::launch_hub()
}

#[tauri::command]
fn open_official_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let allowed = [
        "https://unity.com/download",
        "https://docs.unity.com/en-us/unity-cli/use-unity-cli",
        "https://greenfield-registry.sdq.st/-/web/detail/com.sidequest.creator-sdk",
    ];
    if !allowed.contains(&url.as_str()) {
        return Err("Only pinned official setup links can be opened.".into());
    }
    let mut command = if cfg!(target_os = "windows") {
        let mut command = std::process::Command::new("rundll32.exe");
        command.args(["url.dll,FileProtocolHandler", &url]);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = std::process::Command::new("open");
        command.arg(&url);
        command
    } else {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(&url);
        command
    };
    let _ = app;
    command
        .spawn()
        .map_err(|error| format!("Cannot open link: {error}"))?;
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_recipe,
            probe_environment,
            pick_parent_folder,
            create_project,
            open_project,
            launch_hub,
            open_official_url
        ])
        .run(tauri::generate_context!())
        .expect("error while running Creator Project Setup");
}
