#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bootstrap;
mod creator_hub;
mod hosted;
mod hub;
mod hub_restart;
mod lifecycle;
mod logic;
mod repair;

use logic::{CreateRequest, CreationResult, EnvironmentReport, Recipe};
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

#[tauri::command]
fn get_recipe() -> Result<Recipe, String> {
    let _operation = lifecycle::LIFECYCLE.command()?;
    Ok(logic::recipe())
}

#[tauri::command]
fn probe_environment() -> Result<EnvironmentReport, String> {
    let _operation = lifecycle::LIFECYCLE.command()?;
    Ok(logic::probe_environment())
}

#[tauri::command]
fn pick_parent_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    let _operation = lifecycle::LIFECYCLE.command()?;
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
    let operation = lifecycle::LIFECYCLE.command()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        let _exclusive = bootstrap::OPERATION
            .try_lock()
            .map_err(|_| "Another project setup operation is running. Wait for it to finish.")?;
        bootstrap::ensure(
            &request,
            |plan| {
                app.dialog()
                    .message(plan.confirmation())
                    .title("Install Unity requirements?")
                    .kind(MessageDialogKind::Warning)
                    .buttons(MessageDialogButtons::OkCancelCustom(
                        "Accept and install".into(),
                        "Cancel".into(),
                    ))
                    .blocking_show()
            },
            |progress| hosted::emit(&app, "requirements-progress", progress),
        )?;
        if !bootstrap::licence_ready(|progress| hosted::emit(&app, "requirements-progress", progress))? {
            let open = app.dialog().message("Unity is installed, but no active Unity licence was reported.\n\nSign in and activate your licence in Unity Hub, then return to Setup and choose Create and validate project again. Your installed requirements will be reused. No project files have been created.\n\nOpen Unity Hub now?")
                .title("Unity activation required")
                .buttons(MessageDialogButtons::OkCancelCustom("Open Unity Hub".into(), "Not now".into()))
                .blocking_show();
            if open { logic::launch_hub()?; }
            return Err("Unity licence activation is required. Finish activation in Unity Hub, then choose Create and validate project again. Installed requirements were kept; no project was created.".into());
        }
        logic::create_project(request, |progress| {
            hosted::emit(&app, "setup-progress", progress);
        })
    })
    .await
    .map_err(|error| format!("Setup worker failed: {error}"))?
}

#[tauri::command]
fn open_project(path: String) -> Result<(), String> {
    let _operation = lifecycle::LIFECYCLE.command()?;
    logic::open_project(&path)
}

#[tauri::command]
fn launch_hub() -> Result<(), String> {
    let _operation = lifecycle::LIFECYCLE.command()?;
    logic::launch_hub()
}

#[tauri::command]
async fn restart_hub(app: tauri::AppHandle) -> Result<bool, String> {
    let operation = lifecycle::LIFECYCLE.command()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        let approved = app.dialog()
            .message("Fully close and reopen Unity Hub to reload its Projects list?\n\nWait for Hub downloads and installations to finish first. Unity Editors and project files will not be closed or changed. If Hub refuses to close, the restart stops without force-closing it.")
            .title("Restart Unity Hub?")
            .kind(MessageDialogKind::Warning)
            .buttons(MessageDialogButtons::OkCancelCustom("Restart Hub".into(), "Cancel".into()))
            .blocking_show();
        if !approved { return Ok(false); }
        hub::restart_hub()?;
        Ok(true)
    }).await.map_err(|e| format!("Hub restart worker failed: {e}"))?
}

#[tauri::command]
async fn register_project(path: String) -> Result<hub::HubRegistration, String> {
    let operation = lifecycle::LIFECYCLE.command()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        hub::register_project(std::path::Path::new(&path), |_| {})
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn inspect_project(path: String) -> Result<repair::Inspection, String> {
    let operation = lifecycle::LIFECYCLE.command()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        repair::inspect(std::path::Path::new(&path))
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn run_existing_project(
    app: tauri::AppHandle,
    request: repair::ExistingRequest,
) -> Result<repair::ExistingResult, String> {
    let operation = lifecycle::LIFECYCLE.command()?;
    tauri::async_runtime::spawn_blocking(move || {
        let _operation = operation;
        let _exclusive = bootstrap::OPERATION
            .try_lock()
            .map_err(|_| "Another project setup operation is running. Wait for it to finish.")?;
        repair::run(request, |progress| {
            hosted::emit(&app, "existing-progress", progress);
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn open_official_url(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let _operation = lifecycle::LIFECYCLE.command()?;
    let allowed = [
        "https://unity.com/download",
        "https://unity.com/legal/editor-terms-of-service/software",
        "https://developer.android.com/studio/terms",
        "https://openjdk.org/legal/gplv2+ce.html",
        "https://docs.unity.com/en-us/unity-cli/use-unity-cli",
        "https://greenfield-registry.sdq.st/-/web/detail/com.sidequest.creator-sdk",
        "https://github.com/BOBWORKS-XR/CREATOR-PROJECT-SETUP",
        "https://github.com/BOBWORKS-XR/CREATOR-WORKS-UNITY-MCP/releases",
        "https://github.com/BOBWORKS-XR/CREATOR-PROJECT-SETUP/blob/master/docs/CREATOR-HUB-PLAN.md",
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
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|error| format!("Cannot open link: {error}"))?;
    Ok(())
}

fn main() {
    // Metadata queries must never initialize the GUI or touch project settings.
    let startup = creator_hub::startup_mode(std::env::args_os().skip(1));
    match startup {
        creator_hub::StartupMode::Info => {
            if let Err(error) = creator_hub::write_info(std::io::stdout().lock()) {
                eprintln!("Creator Hub metadata failed: {error}");
                std::process::exit(1);
            }
            return;
        }
        creator_hub::StartupMode::Invalid => {
            eprintln!("Use --creator-hub-info alone; no other Hub arguments are supported.");
            std::process::exit(2);
        }
        creator_hub::StartupMode::Standalone | creator_hub::StartupMode::Hosted => {}
    }
    let mut context = tauri::generate_context!();
    let files = if startup == creator_hub::StartupMode::Hosted {
        context.config_mut().app.windows.clear();
        Some(hosted::assets(&context).expect("Cannot decode hosted Setup interface"))
    } else {
        None
    };
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(move |app| {
            if let Some(files) = files {
                app.manage(hosted::Output::default());
                let handle = app.handle().clone();
                std::thread::spawn(move || hosted::run(handle, files));
            }
            #[cfg(windows)]
            if let Some(window) = app.get_webview_window("main") {
                lifecycle::LIFECYCLE.attach(window.hwnd()?.0 as usize);
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                if !lifecycle::LIFECYCLE.request_close() {
                    api.prevent_close();
                    let _ = window.emit("creator-lifecycle-close-blocked", ());
                }
            }
            tauri::WindowEvent::Destroyed => lifecycle::LIFECYCLE.detach(),
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            get_recipe,
            probe_environment,
            pick_parent_folder,
            create_project,
            open_project,
            launch_hub,
            restart_hub,
            register_project,
            inspect_project,
            run_existing_project,
            open_official_url
        ])
        .build(context)
        .expect("error while running Creator Project Setup")
        .run(|_, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                if !lifecycle::LIFECYCLE.request_close() {
                    api.prevent_exit();
                }
            }
        });
}
