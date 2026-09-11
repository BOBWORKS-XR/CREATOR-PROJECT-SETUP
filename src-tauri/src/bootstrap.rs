//! Explicitly approved prerequisite installs using the pinned official Unity CLI.
use crate::logic::{self, EditorInstallation, EDITOR_CHANGESET, EDITOR_VERSION};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{mpsc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

pub(crate) static OPERATION: Mutex<()> = Mutex::new(());
const MIB: u64 = 1024 * 1024;
const GIB: u64 = 1024 * MIB;
const MAX_OUTPUT: usize = 4 * MIB as usize;
const MAX_LOG: usize = 16 * MIB as usize;
const MAX_LINE: usize = 64 * 1024;
// Module ID from the pinned 6000.3.21f1 Windows release manifest, not a CLI alias.
const OPENJDK_MODULE: &str = "android-open-jdk-17.0.18+8";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub stage: String,
    pub detail: String,
    pub percent: Option<f64>,
}

fn progress(stage: &str, detail: impl Into<String>) -> Progress {
    Progress {
        stage: stage.into(),
        detail: detail.into(),
        percent: None,
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
enum Action {
    InstallEditor,
    AddModules,
    None,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    action: Action,
    pub install_hub: bool,
    pub editor_root: PathBuf,
    pub cache_root: PathBuf,
    pub modules: Vec<String>,
    pub download_bytes: u64,
    pub reserve_bytes: u64,
    pub log_directory: PathBuf,
}

impl Plan {
    pub fn confirmation(&self) -> String {
        if self.action == Action::None && self.install_hub {
            return "Install Unity Hub so you can activate Unity?\n\nYour existing Editor and build tools will be reused. Unity's official signed Hub installer will be downloaded.\n\nBy selecting Accept and install, you agree to the linked Unity terms. Windows may ask for administrator approval. You will complete sign-in and licence activation in Unity Hub. Existing projects and Editor versions will not be removed.".into();
        }
        format!(
            "Install the missing Unity requirements, then create your project?\n\nUnity {EDITOR_VERSION}; Android SDK, NDK and OpenJDK; Windows support.{}\nEditor: {}\nDownload cache: {}\nEditor/modules download: {:.1} GB.{}\nConservative free-space reserve: {:.1} GB (not an exact installed size).\n\nBy selecting Accept and install, you agree to the Unity Software Terms and the Android SDK/NDK and OpenJDK licences linked in Setup. Unity/Windows may still ask for sign-in, activation or administrator approval.\n\nExisting projects and other Editor versions will not be removed. Installers must finish before Setup can close.",
            if self.install_hub { " Unity Hub will also be installed." } else { "" },
            self.editor_root.display(), self.cache_root.display(),
            self.download_bytes as f64 / 1_000_000_000.0,
            if self.install_hub { " Hub is an additional download." } else { "" },
            self.reserve_bytes as f64 / 1_000_000_000.0,
        )
    }
}

fn roots_file() -> Option<PathBuf> {
    Some(dirs::data_local_dir()?.join("CreatorProjectSetup/editor-roots.json"))
}

pub(crate) fn known_editor_roots() -> Vec<PathBuf> {
    roots_file()
        .and_then(|path| File::open(path).ok())
        .and_then(|file| serde_json::from_reader::<_, Vec<PathBuf>>(file.take(16384)).ok())
        .unwrap_or_default()
        .into_iter()
        .filter(|path| path.is_absolute())
        .take(16)
        .collect()
}

fn remember_root(root: &Path) -> Result<(), String> {
    let path = roots_file().ok_or("Local application data is unavailable.")?;
    let mut roots = known_editor_roots();
    let parent = root
        .parent()
        .ok_or("Invalid Editor directory.")?
        .to_path_buf();
    roots.retain(|p| *p != parent);
    roots.insert(0, parent);
    roots.truncate(16);
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(
        path,
        serde_json::to_vec_pretty(&roots).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("Cannot remember the verified Editor location: {e}"))
}

fn module_selection(editor: Option<&EditorInstallation>) -> Result<(Action, Vec<String>), String> {
    let Some(editor) = editor else {
        return Ok((Action::InstallEditor, vec!["android".into()]));
    };
    if editor.ready {
        return Ok((Action::None, vec![]));
    }
    if !editor.windows_standalone || editor.urp_template.is_none() {
        return Err("The existing Editor is incomplete: its built-in Windows support or URP template is missing. Setup will not overwrite this installation. Use Unity Hub to repair/reinstall this Editor, then check again.".into());
    }
    let mut modules = Vec::new();
    if !editor.android_player {
        modules.push("android".into());
    } else {
        if !editor.android_sdk || !editor.android_ndk {
            modules.push("android-sdk-ndk-tools".into());
        }
        if !editor.open_jdk {
            modules.push(OPENJDK_MODULE.into());
        }
    }
    Ok((Action::AddModules, modules))
}

fn arguments(action: &Action, modules: &[String], preview: bool) -> Vec<String> {
    let mut args: Vec<String> = match action {
        Action::InstallEditor => vec!["install", EDITOR_VERSION, "--changeset", EDITOR_CHANGESET],
        Action::AddModules => vec!["install-modules", "--editor-version", EDITOR_VERSION],
        Action::None => vec![],
    }
    .into_iter()
    .map(String::from)
    .collect();
    if !modules.is_empty() {
        args.push("--module".into());
        args.extend_from_slice(modules);
        args.push("--cm".into());
    }
    if *action == Action::AddModules {
        // Include the same selected repair set in dry-run and installation.
        args.push("--reinstall".into());
    }
    if preview {
        args.push("--dry-run".into());
    } else if *action != Action::None {
        args.push("--accept-eula".into());
    }
    args
}

fn absolute_path(value: &Value, field: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(
        value[field]
            .as_str()
            .ok_or("Unity CLI did not report installation paths.")?,
    );
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(
            "Unity CLI reported an invalid installation path. Nothing was installed.".into(),
        );
    }
    Ok(path)
}

fn preview_size(value: &Value) -> Result<u64, String> {
    value["totalDownloadSize"]
        .as_u64()
        .filter(|size| *size > 0 && *size < 100 * GIB)
        .ok_or("Unity did not provide a valid download size. Nothing was installed.".into())
}

fn reserve(download: u64) -> Result<u64, String> {
    // Download cache, an installation/extraction allowance, and project-import headroom.
    download
        .checked_mul(4)
        .and_then(|n| n.checked_add(8 * GIB))
        .ok_or("The reported download is too large.".into())
}

#[cfg(windows)]
fn free_space(path: &Path) -> Result<u64, String> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let mut existing = path;
    while !existing.exists() {
        existing = existing.parent().ok_or("Cannot find the install volume.")?;
    }
    let wide: Vec<u16> = existing.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut available = 0;
    if unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut available,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(format!(
            "Cannot check free space at {}. Nothing was installed.",
            existing.display()
        ));
    }
    Ok(available)
}

#[cfg(not(windows))]
fn free_space(_: &Path) -> Result<u64, String> {
    Err("Automatic Unity installation is currently Windows-only.".into())
}

fn check_space(plan: &Plan, project_parent: &Path) -> Result<(), String> {
    // Checking the combined allowance on each involved volume is conservative even
    // when paths share a volume; it never undercounts their simultaneous usage.
    for path in [
        &plan.editor_root,
        &plan.cache_root,
        &project_parent.to_path_buf(),
    ] {
        let available = free_space(path)?;
        if available < plan.reserve_bytes {
            return Err(format!("Not enough free space at {}: {:.1} GB available; {:.1} GB reserved for downloads, extraction and project import. Free space or choose another installation/project location. Nothing was installed.", path.display(), available as f64 / 1e9, plan.reserve_bytes as f64 / 1e9));
        }
    }
    Ok(())
}

#[cfg(windows)]
fn require_editor_closed(executable: &Path) -> Result<(), String> {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, GetLastError, ERROR_NO_MORE_FILES, INVALID_HANDLE_VALUE},
        System::{
            Diagnostics::ToolHelp::{
                CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
                TH32CS_SNAPPROCESS,
            },
            Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
    };
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return Err("Cannot check running Editors. Nothing was installed.".into());
        }
        let result = (|| {
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            let mut more = Process32FirstW(snapshot, &mut entry);
            while more != 0 {
                let length = entry
                    .szExeFile
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(entry.szExeFile.len());
                if String::from_utf16_lossy(&entry.szExeFile[..length])
                    .eq_ignore_ascii_case("Unity.exe")
                {
                    let process =
                        OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, entry.th32ProcessID);
                    if process.is_null() {
                        return Err("A Unity Editor is running but its location cannot be verified. Close Unity normally before installing modules.".into());
                    }
                    let mut name = vec![0u16; 32768];
                    let mut count = name.len() as u32;
                    let ok = QueryFullProcessImageNameW(process, 0, name.as_mut_ptr(), &mut count);
                    CloseHandle(process);
                    if ok == 0 {
                        return Err("Cannot verify a running Unity Editor. Close Unity normally before installing modules.".into());
                    }
                    let running = PathBuf::from(String::from_utf16_lossy(&name[..count as usize]));
                    if running
                        .to_string_lossy()
                        .eq_ignore_ascii_case(&executable.to_string_lossy())
                        || fs::canonicalize(&running)
                            .ok()
                            .zip(fs::canonicalize(executable).ok())
                            .is_some_and(|(a, b)| a == b)
                    {
                        return Err(format!("Unity {EDITOR_VERSION} is running. Save your work and close its Editor windows before adding modules. Nothing was force-closed."));
                    }
                }
                more = Process32NextW(snapshot, &mut entry);
            }
            if GetLastError() != ERROR_NO_MORE_FILES {
                return Err(
                    "Could not finish checking running Editors. Nothing was installed.".into(),
                );
            }
            Ok(())
        })();
        CloseHandle(snapshot);
        result
    }
}

#[cfg(not(windows))]
fn require_editor_closed(_: &Path) -> Result<(), String> {
    Err("Automatic installation is Windows-only.".into())
}

fn compact(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_control())
        .take(180)
        .collect()
}

fn frame_progress(frame: &Value) -> Option<Progress> {
    if frame["type"] != "progress" {
        return None;
    }
    let phase = frame["phase"].as_str()?;
    if !["download", "install"].contains(&phase) {
        return None;
    }
    let name = compact(frame["name"].as_str().unwrap_or("Unity requirements"));
    let percent = if phase == "download" {
        frame["pct"]
            .as_f64()
            .filter(|n| n.is_finite() && (0.0..=100.0).contains(n))
    } else {
        None
    };
    Some(Progress {
        stage: if phase == "download" {
            "Downloading requirements"
        } else {
            "Installing requirements"
        }
        .into(),
        detail: name,
        percent,
    })
}

struct Capture {
    bytes: Vec<u8>,
    overflow: bool,
    failed_result: bool,
}

fn capture(
    reader: impl Read,
    mut log: File,
    frames: Option<mpsc::SyncSender<Progress>>,
    record: bool,
) -> Result<Capture, String> {
    let mut reader = BufReader::new(reader);
    let mut bytes = Vec::new();
    let mut written = 0;
    let mut overflow = false;
    let mut failed_result = false;
    loop {
        let mut line = Vec::new();
        let count = reader
            .by_ref()
            .take((MAX_LINE + 1) as u64)
            .read_until(b'\n', &mut line)
            .map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        let remaining = if record {
            MAX_LOG.saturating_sub(written)
        } else {
            0
        };
        let keep = remaining.min(line.len());
        if keep > 0 {
            log.write_all(&line[..keep]).map_err(|e| e.to_string())?;
            written += keep;
        }
        if bytes.len() + line.len() <= MAX_OUTPUT {
            bytes.extend_from_slice(&line);
        } else {
            overflow = true;
        }
        if line.len() <= MAX_LINE {
            if let Ok(frame) = serde_json::from_slice::<Value>(&line) {
                failed_result |= frame["type"] == "result" && frame["success"] == false;
                if let (Some(sender), Some(event)) = (&frames, frame_progress(&frame)) {
                    let _ = sender.try_send(event);
                }
            }
        }
    }
    if written == MAX_LOG {
        log.write_all(b"\n[Setup log limit reached; remaining output was drained.]\n")
            .map_err(|e| e.to_string())?;
    }
    Ok(Capture {
        bytes,
        overflow,
        failed_result,
    })
}

fn run_cli(
    helper: &Path,
    args: &[String],
    label: &str,
    logs: &Path,
    preview: bool,
    emit: &impl Fn(Progress),
) -> Result<Value, String> {
    let stdout =
        File::create(logs.join(format!("{label}.stdout.log"))).map_err(|e| e.to_string())?;
    let stderr =
        File::create(logs.join(format!("{label}.stderr.log"))).map_err(|e| e.to_string())?;
    let mut command = Command::new(helper);
    command
        .args(args)
        .args([
            "--format",
            if preview { "json" } else { "ndjson" },
            "--non-interactive",
            "--no-banner",
            "--no-pager",
            "--no-color",
        ])
        .env("UNITY_NO_UPDATE_CHECK", "1")
        .env("UNITY_NO_CONSENT_PROMPT", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command
        .spawn()
        .map_err(|e| format!("Cannot start the verified Unity helper: {e}"))?;
    let out = child.stdout.take().ok_or("Missing Unity output pipe.")?;
    let err = child.stderr.take().ok_or("Missing Unity error pipe.")?;
    let (sender, receiver) = mpsc::sync_channel(32);
    let result = std::thread::scope(|scope| {
        // Path reports may contain proxy settings; licence reports contain account
        // identifiers. Parse their needed fields in memory, never persist the raw data.
        let record = !["paths", "licence"].contains(&label);
        let stdout_reader = scope.spawn(move || capture(out, stdout, Some(sender), record));
        let stderr_reader = scope.spawn(move || capture(err, stderr, None, record));
        let start = Instant::now();
        let mut timed_out = false;
        let status = loop {
            if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                break status;
            }
            if preview && start.elapsed() > Duration::from_secs(120) {
                // Only read-only queries may be terminated. Never interrupt an installer.
                child.kill().map_err(|e| e.to_string())?;
                timed_out = true;
                break child.wait().map_err(|e| e.to_string())?;
            }
            if let Ok(event) = receiver.recv_timeout(Duration::from_millis(200)) {
                emit(event);
            }
        };
        let out = stdout_reader
            .join()
            .map_err(|_| "Unity output reader stopped.")??;
        let err = stderr_reader
            .join()
            .map_err(|_| "Unity error reader stopped.")??;
        for event in receiver.try_iter() {
            emit(event);
        }
        if timed_out {
            return Err("Unity's read-only requirement check timed out. Check your connection and try again.".into());
        }
        if !status.success() || out.failed_result || err.failed_result {
            let guidance = match status.code() {
                Some(3) => "Sign in to Unity and check licence activation, then try again.",
                Some(4) => "Unity needs a preference or approval. Check Unity Hub and the local logs.",
                Some(7) => "Unity's download service could not be reached. Check your connection before retrying.",
                _ => "Check for a declined administrator/licence prompt, a failed download or insufficient disk space. Existing installations were preserved; recheck before retrying.",
            };
            return Err(format!(
                "Unity requirement step '{label}' failed (exit {}). {guidance} Logs: {}",
                status
                    .code()
                    .map(|n| n.to_string())
                    .unwrap_or("unknown".into()),
                logs.display()
            ));
        }
        if !preview {
            return Ok(Value::Null);
        }
        if out.overflow {
            return Err("Unity's requirement report was too large. Nothing was installed.".into());
        }
        let value: Value = serde_json::from_slice(&out.bytes)
            .map_err(|_| "Unity returned an invalid requirement report. Nothing was installed.")?;
        if value["success"] != true {
            return Err("Unity could not verify its requirements. Nothing was installed.".into());
        }
        Ok(value["data"].clone())
    });
    result
}

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|s| (*s).into()).collect()
}

fn active_licence(data: &Value) -> Result<bool, String> {
    data["active"].as_bool().ok_or("Unity did not return a valid licence status. Check activation in Unity Hub before creating the project.".into())
}

pub fn licence_ready(emit: impl Fn(Progress)) -> Result<bool, String> {
    let helper =
        crate::hub::prepare_helper(&|detail| emit(progress("Checking Unity licence", detail)))?;
    emit(progress(
        "Checking Unity licence",
        "Checking activation before creating any project files.",
    ));
    let logs = dirs::data_local_dir()
        .ok_or("Local application data is unavailable.")?
        .join("CreatorProjectSetup/logs")
        .join(format!("licence-{}", crate::hub::unique_id()));
    fs::create_dir_all(&logs).map_err(|e| e.to_string())?;
    active_licence(&run_cli(
        &helper,
        &strings(&["license", "status"]),
        "licence",
        &logs,
        true,
        &emit,
    )?)
}

fn registered_editor(data: &Value) -> Result<Option<EditorInstallation>, String> {
    let editors = data
        .as_array()
        .ok_or("Unity did not return its Editor inventory.")?;
    let matches: Vec<_> = editors
        .iter()
        .filter(|e| e["version"] == EDITOR_VERSION)
        .collect();
    if matches.len() > 1 {
        return Err("Multiple matching Editor installations were found. Resolve the ambiguity in Unity Hub before installing modules.".into());
    }
    let Some(entry) = matches.first() else {
        return Ok(None);
    };
    let executable = absolute_path(entry, "location")?;
    let root = executable
        .parent()
        .and_then(Path::parent)
        .ok_or("Unexpected Unity Editor path.")?;
    let editor = logic::inspect_editor(root.to_path_buf()).ok_or("The registered Editor is missing or incomplete. Review its installation in Unity Hub before retrying.")?;
    if !editor.exact_recipe || Path::new(&editor.executable) != executable {
        return Err("Unity's registered Editor location does not match the pinned recipe.".into());
    }
    Ok(Some(editor))
}

fn check_local_inventory(
    local: &[EditorInstallation],
    registered: Option<&EditorInstallation>,
) -> Result<(), String> {
    for editor in local.iter().filter(|e| e.exact_recipe) {
        if !registered
            .is_some_and(|entry| entry.executable.eq_ignore_ascii_case(&editor.executable))
        {
            return Err("An existing matching Editor was found outside Unity CLI's verified inventory. Add that installation in Unity Hub and check again. Setup will not install a duplicate or modify an unregistered Editor.".into());
        }
    }
    Ok(())
}

fn check_plan_current(plan: &Plan, paths: &Value, editors: &Value) -> Result<(), String> {
    let existing = registered_editor(editors)?;
    let root = existing
        .as_ref()
        .map(|e| PathBuf::from(&e.root))
        .unwrap_or(absolute_path(paths, "editorInstallPath")?.join(EDITOR_VERSION));
    let (action, modules) = module_selection(existing.as_ref())?;
    if root != plan.editor_root
        || absolute_path(paths, "downloadCachePath")? != plan.cache_root
        || action != plan.action
        || modules != plan.modules
        || (action == Action::InstallEditor && root.exists())
    {
        return Err("Unity's installation location or requirements changed while approval was open. Nothing further was installed. Check again to review a fresh plan.".into());
    }
    if let Some(editor) = existing.filter(|_| action == Action::AddModules) {
        require_editor_closed(Path::new(&editor.executable))?;
    }
    Ok(())
}

pub fn ensure(
    request: &logic::CreateRequest,
    approve: impl Fn(&Plan) -> bool,
    emit: impl Fn(Progress),
) -> Result<(), String> {
    ensure_inner(request, false, approve, emit)
}

pub fn ensure_activation_hub(
    request: &logic::CreateRequest,
    approve: impl Fn(&Plan) -> bool,
    emit: impl Fn(Progress),
) -> Result<(), String> {
    ensure_inner(request, true, approve, emit)
}

fn can_reuse(ready: bool, hub_installed: bool, require_hub: bool) -> bool {
    ready && (!require_hub || hub_installed)
}

fn ensure_inner(
    request: &logic::CreateRequest,
    require_hub: bool,
    approve: impl Fn(&Plan) -> bool,
    emit: impl Fn(Progress),
) -> Result<(), String> {
    let target = logic::creation_target(request)?;
    let initial = logic::probe_environment();
    if can_reuse(initial.ready, initial.hub_installed, require_hub) {
        if cfg!(windows) && free_space(target.parent().unwrap())? < 8 * GIB {
            return Err("Project creation needs an 8 GiB free-space reserve for package downloads and imports. Choose a location with more space before starting.".into());
        }
        return Ok(());
    }
    if !cfg!(windows) {
        return Err("Automatic prerequisite installation is currently Windows-only. Install the listed requirements through Unity Hub, then create your project.".into());
    }
    let logs = dirs::data_local_dir()
        .ok_or("Local application data is unavailable.")?
        .join("CreatorProjectSetup/logs")
        .join(format!("requirements-{}", crate::hub::unique_id()));
    fs::create_dir_all(&logs).map_err(|e| e.to_string())?;
    let result = (|| {
        emit(progress(
            "Checking Unity requirements",
            "Preparing the verified official Unity helper.",
        ));
        let helper = crate::hub::prepare_helper(&|detail| {
            emit(progress("Checking Unity requirements", detail))
        })?;
        let paths = run_cli(&helper, &strings(&["env"]), "paths", &logs, true, &emit)?;
        let install_parent = absolute_path(&paths, "editorInstallPath")?;
        let cache_root = absolute_path(&paths, "downloadCachePath")?;
        let editors = run_cli(
            &helper,
            &strings(&["editors", "--installed"]),
            "editors",
            &logs,
            true,
            &emit,
        )?;
        let existing = registered_editor(&editors)?;
        check_local_inventory(&initial.editors, existing.as_ref())?;
        if let Some(editor) = &existing {
            remember_root(Path::new(&editor.root))?;
        }
        let (action, modules) = module_selection(existing.as_ref())?;
        if action == Action::AddModules {
            require_editor_closed(Path::new(&existing.as_ref().unwrap().executable))?;
        }
        if action == Action::None && initial.hub_installed {
            if free_space(target.parent().unwrap())? < 8 * GIB {
                return Err("Project creation needs an 8 GiB free-space reserve. Choose a location with more space.".into());
            }
            return Ok(());
        }
        let editor_root = existing
            .as_ref()
            .map(|e| PathBuf::from(&e.root))
            .unwrap_or_else(|| install_parent.join(EDITOR_VERSION));
        if action == Action::InstallEditor && editor_root.exists() {
            return Err("The target Editor directory already exists but is not a verified registered installation. Setup will not overwrite it. Review the partial/manual installation in Unity Hub.".into());
        }
        let download_bytes = if action == Action::None {
            0
        } else {
            emit(progress(
                "Planning Unity installation",
                "Checking download sizes without installing anything.",
            ));
            preview_size(&run_cli(
                &helper,
                &arguments(&action, &modules, true),
                "preview",
                &logs,
                true,
                &emit,
            )?)?
        };
        let plan = Plan {
            action,
            install_hub: !initial.hub_installed,
            editor_root,
            cache_root,
            modules,
            download_bytes,
            reserve_bytes: reserve(download_bytes)?,
            log_directory: logs.clone(),
        };
        check_space(&plan, target.parent().unwrap())?;
        emit(progress(
            "Approval required",
            "Review Unity's installation location, download and licence terms.",
        ));
        if !approve(&plan) {
            return Err("Setup cancelled before installation. No Editor or modules were installed and no project was created.".into());
        }
        logic::creation_target(request)?;
        check_space(&plan, target.parent().unwrap())?;
        let recheck = || -> Result<(), String> {
            let paths = run_cli(&helper, &strings(&["env"]), "paths", &logs, true, &emit)?;
            let editors = run_cli(
                &helper,
                &strings(&["editors", "--installed"]),
                "editors",
                &logs,
                true,
                &emit,
            )?;
            check_plan_current(&plan, &paths, &editors)
        };
        recheck()?;
        if plan.install_hub {
            emit(progress(
                "Installing Unity Hub",
                "Downloading the official signed installer. Windows may ask for approval.",
            ));
            run_cli(
                &helper,
                &strings(&[
                    "hub",
                    "install",
                    "--hub-version",
                    crate::hub::MINIMUM_HUB_VERSION,
                    "--headless",
                ]),
                "install-hub",
                &logs,
                false,
                &emit,
            )?;
            if !logic::probe_environment().hub_installed {
                return Err("Unity Hub did not appear after installation. Check the Windows installer and local logs before retrying.".into());
            }
        }
        if plan.action != Action::None {
            // Hub installation can change its configured Editor location.
            if plan.install_hub {
                recheck()?;
            }
            if let Some(editor) = &existing {
                require_editor_closed(Path::new(&editor.executable))?;
            }
            emit(progress("Installing Unity requirements", "Unity is downloading and installing the approved components. Administrator prompts may appear."));
            run_cli(
                &helper,
                &arguments(&plan.action, &plan.modules, false),
                "install-editor",
                &logs,
                false,
                &emit,
            )?;
        }
        emit(progress(
            "Verifying Unity requirements",
            "Checking the installed Editor, Android tools, Windows support and URP template.",
        ));
        let installed = logic::inspect_editor(plan.editor_root.clone())
            .ok_or("The required Editor executable is still missing after installation.")?;
        if !installed.ready {
            return Err("Unity finished, but one or more required components are still missing. No project was created. Recheck before retrying; inspect the requirement logs.".into());
        }
        remember_root(&plan.editor_root)?;
        if !logic::probe_environment().ready {
            return Err("Requirement verification did not pass. No project was created.".into());
        }
        Ok(())
    })();
    let receipt = json!({ "schemaVersion": 1, "setupVersion": env!("CARGO_PKG_VERSION"), "recordedAtUnixMs": SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis(), "editorVersion": EDITOR_VERSION, "success": result.is_ok(), "error": result.as_ref().err(), "logDirectory": logs, "projectCreated": false });
    fs::write(
        logs.join("requirements-receipt.json"),
        serde_json::to_vec_pretty(&receipt).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("Cannot write requirement receipt: {e}"))?;
    result.map_err(|error: String| {
        format!(
            "{error} Requirement report: {}",
            logs.join("requirements-receipt.json").display()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_only_install_reuse_depends_on_activation_handoff() {
        assert!(can_reuse(true, false, false));
        assert!(!can_reuse(true, false, true));
        assert!(can_reuse(true, true, true));
        assert!(!can_reuse(false, true, false));
        let plan = Plan {
            action: Action::None,
            install_hub: true,
            editor_root: PathBuf::new(),
            cache_root: PathBuf::new(),
            modules: vec![],
            download_bytes: 0,
            reserve_bytes: 0,
            log_directory: PathBuf::new(),
        };
        assert!(plan
            .confirmation()
            .contains("existing Editor and build tools will be reused"));
        assert!(!plan.confirmation().contains("0.0 GB"));
    }

    #[test]
    #[cfg(windows)]
    fn real_child_pipes_preserve_reports_progress_and_failure_results() {
        let temp = tempfile::tempdir().unwrap();
        let helper = PathBuf::from(std::env::var_os("SystemRoot").unwrap())
            .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let fixture =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/prerequisite-cli.ps1");
        let call = |mode: &str, query: bool, emit: &dyn Fn(Progress)| {
            run_cli(
                &helper,
                &[
                    "-NoProfile".into(),
                    "-NonInteractive".into(),
                    "-File".into(),
                    fixture.to_string_lossy().into(),
                    mode.into(),
                ],
                "fixture",
                temp.path(),
                query,
                &|p| emit(p),
            )
        };
        assert_eq!(
            call("query", true, &|_| {}).unwrap()["totalDownloadSize"],
            1234
        );
        assert!(call("bad-json", true, &|_| {}).is_err());
        assert!(
            call("false-result", false, &|_| {}).is_err(),
            "A zero exit code cannot override a failed result"
        );
        assert!(call("offline", false, &|_| {})
            .unwrap_err()
            .contains("connection"));
        let events = Mutex::new(Vec::new());
        call("progress", false, &|event| {
            events.lock().unwrap().push(event)
        })
        .unwrap();
        let events = events.lock().unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].percent, Some(42.0));
        assert_eq!(events[1].percent, None);
    }

    #[test]
    #[cfg(windows)]
    fn active_editor_guard_blocks_only_the_selected_executable() {
        use std::os::windows::process::CommandExt;
        let temp = tempfile::tempdir().unwrap();
        let helper =
            PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32/ping.exe");
        let executable = temp.path().join("Unity.exe");
        fs::copy(helper, &executable).unwrap();
        let mut child = Command::new(&executable)
            .args(["-t", "127.0.0.1"])
            .creation_flags(0x08000000)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        // Capture outcomes before assertions so this exact owned fixture is always reaped.
        let selected = require_editor_closed(&executable);
        let unrelated = require_editor_closed(&temp.path().join("another/Unity.exe"));
        let _ = child.kill();
        child.wait().unwrap();
        assert!(selected.unwrap_err().contains("running"));
        assert!(unrelated.is_ok());
    }

    #[test]
    #[cfg(windows)]
    #[ignore = "installs Unity on an explicitly approved disposable Windows Actions runner only"]
    fn disposable_install_smoke() {
        for (key, value) in [
            ("GITHUB_ACTIONS", "true"),
            ("CI", "true"),
            ("RUNNER_ENVIRONMENT", "github-hosted"),
            ("RUNNER_OS", "Windows"),
            ("CREATOR_SETUP_ACCEPT_TEST_LICENSES", "true"),
        ] {
            assert_eq!(
                std::env::var(key).as_deref(),
                Ok(value),
                "Disposable installation requires explicit CI licence approval"
            );
        }
        let environment = logic::probe_environment();
        assert!(
            !environment.hub_installed,
            "This must be a clean runner, not the user's computer"
        );
        assert!(
            environment.editors.is_empty(),
            "This test requires no existing Editor"
        );
        let parent = std::env::var_os("RUNNER_TEMP").map(PathBuf::from).unwrap();
        let request = logic::CreateRequest {
            project_name: "CreatorBootstrapSmoke".into(),
            parent_directory: parent.to_string_lossy().into(),
        };
        assert!(!parent.join(&request.project_name).exists());
        let cancelled = ensure(&request, |_| false, |_| {}).unwrap_err();
        assert!(cancelled.contains("cancelled before installation"));
        assert!(!logic::probe_environment().hub_installed);
        assert!(logic::probe_environment().editors.is_empty());
        assert!(!parent.join(&request.project_name).exists());
        ensure(
            &request,
            |plan| {
                println!(
                    "Approved disposable plan: {}",
                    serde_json::to_string(plan).unwrap()
                );
                true
            },
            |event| println!("{}", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();
        assert!(logic::probe_environment().ready);
        ensure(
            &request,
            |_| panic!("A verified installation must be reused"),
            |_| {},
        )
        .unwrap();

        // Damage only a named file in the installation created above, on this
        // disposable runner, to prove repair defeats stale Installed metadata.
        let installed = logic::probe_environment()
            .editors
            .into_iter()
            .find(|e| e.exact_recipe)
            .unwrap();
        let root = fs::canonicalize(&installed.root).unwrap();
        let java = root.join("Editor/Data/PlaybackEngines/AndroidPlayer/OpenJDK/bin/java.exe");
        assert!(fs::canonicalize(&java).unwrap().starts_with(&root));
        fs::remove_file(&java).unwrap();
        assert!(!logic::probe_environment().ready);
        ensure(
            &request,
            |plan| {
                assert_eq!(plan.action, Action::AddModules);
                assert_eq!(plan.modules, vec![OPENJDK_MODULE]);
                assert!(!plan.install_hub);
                println!(
                    "Approved isolated OpenJDK repair: {}",
                    serde_json::to_string(plan).unwrap()
                );
                true
            },
            |event| println!("{}", serde_json::to_string(&event).unwrap()),
        )
        .unwrap();
        assert!(java.is_file());
        assert!(logic::probe_environment().ready);
        assert!(
            !licence_ready(|_| {}).unwrap(),
            "This disposable test must not acquire a Unity account licence"
        );
        assert!(
            !parent.join(&request.project_name).exists(),
            "Prerequisite testing must not pretend it created a Unity project"
        );
        let report = json!({"setupVersion":env!("CARGO_PKG_VERSION"), "editorVersion":EDITOR_VERSION,"prerequisitesVerified":true,"cancellationVerified":true,"existingInstallReused":true,"missingJdkRepaired":true,"inactiveLicenceDetected":true,"projectCreated":false,"unityAccountUsed":false,"licenseActivationTested":false});
        fs::write(
            parent.join("creator-prerequisite-acceptance.json"),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
    }
    fn editor() -> EditorInstallation {
        EditorInstallation {
            version: EDITOR_VERSION.into(),
            root: "test".into(),
            executable: "test".into(),
            exact_recipe: true,
            android_player: true,
            android_sdk: true,
            android_ndk: true,
            open_jdk: true,
            windows_standalone: true,
            urp_template: Some("template.tgz".into()),
            ready: true,
        }
    }
    #[test]
    fn plans_only_the_missing_android_requirements() {
        assert_eq!(
            module_selection(None).unwrap(),
            (Action::InstallEditor, vec!["android".into()])
        );
        let mut e = editor();
        assert_eq!(module_selection(Some(&e)).unwrap().0, Action::None);
        e.ready = false;
        e.open_jdk = false;
        assert_eq!(module_selection(Some(&e)).unwrap().1, vec![OPENJDK_MODULE]);
        e.android_ndk = false;
        assert_eq!(
            module_selection(Some(&e)).unwrap().1,
            vec!["android-sdk-ndk-tools", OPENJDK_MODULE]
        );
        e.android_player = false;
        assert_eq!(module_selection(Some(&e)).unwrap().1, vec!["android"]);
    }
    #[test]
    fn damaged_core_is_not_silently_reinstalled() {
        let mut e = editor();
        e.ready = false;
        e.urp_template = None;
        assert!(module_selection(Some(&e)).is_err());
        e.urp_template = Some("x".into());
        e.windows_standalone = false;
        assert!(module_selection(Some(&e)).is_err());
    }
    #[test]
    fn preview_never_accepts_licences_or_forces_installation() {
        for action in [Action::InstallEditor, Action::AddModules] {
            let args = arguments(&action, &["android".into()], true);
            assert!(args.iter().any(|a| a == "--dry-run"));
            for denied in [
                "--accept-eula",
                "--yes",
                "--force",
                "--skip-signature-check",
                "latest",
            ] {
                assert!(!args.iter().any(|a| a == denied));
            }
            assert!(args.iter().any(|a| a == EDITOR_VERSION));
            assert_eq!(
                args.iter().any(|a| a == "--reinstall"),
                action == Action::AddModules
            );
        }
    }
    #[test]
    fn rejects_missing_or_unreasonable_size_and_paths() {
        for value in [
            json!({}),
            json!({"totalDownloadSize": -1}),
            json!({"totalDownloadSize": 0}),
            json!({"totalDownloadSize": 100 * GIB}),
        ] {
            assert!(preview_size(&value).is_err());
        }
        assert_eq!(preview_size(&json!({"totalDownloadSize": 42})).unwrap(), 42);
        assert!(absolute_path(&json!({"path": "relative"}), "path").is_err());
        assert!(reserve(u64::MAX).is_err());
    }
    #[test]
    fn installer_percentage_is_never_presented_as_download_progress() {
        let event =
            frame_progress(&json!({"type":"progress","phase":"install","pct":50,"name":"Android"}))
                .unwrap();
        assert_eq!(event.percent, None);
        assert_eq!(
            frame_progress(&json!({"type":"progress","phase":"download","pct":24}))
                .unwrap()
                .percent,
            Some(24.0)
        );
        assert_eq!(
            frame_progress(&json!({"type":"progress","phase":"download","pct":101}))
                .unwrap()
                .percent,
            None
        );
        assert!(frame_progress(&json!({"type":"result","success":true})).is_none());
    }
    #[test]
    fn logs_and_capture_are_bounded_and_failed_frames_recorded() {
        let temp = tempfile::tempdir().unwrap();
        let bytes = b"{\"type\":\"result\",\"success\":false}\n";
        let result = capture(
            bytes.as_slice(),
            File::create(temp.path().join("log")).unwrap(),
            None,
            true,
        )
        .unwrap();
        assert!(result.failed_result);
        let result = capture(
            vec![b'x'; MAX_OUTPUT + 2].as_slice(),
            File::create(temp.path().join("large")).unwrap(),
            None,
            true,
        )
        .unwrap();
        assert!(result.overflow);
        assert!(result.bytes.len() <= MAX_OUTPUT);
    }

    #[test]
    fn sensitive_query_output_is_never_written_to_local_reports() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("licence.log");
        let content = b"{\"active\":true,\"organization\":\"private-org\"}\n";
        let result = capture(
            content.as_slice(),
            File::create(&path).unwrap(),
            None,
            false,
        )
        .unwrap();
        assert!(!result.bytes.is_empty());
        assert!(fs::read(path).unwrap().is_empty());
        assert!(active_licence(&json!({"active": true})).unwrap());
        assert!(!active_licence(&json!({"active": false})).unwrap());
        assert!(active_licence(&json!({"active": "true"})).is_err());
    }

    #[test]
    fn unregistered_matching_editors_block_duplicate_installation() {
        let e = editor();
        assert!(check_local_inventory(std::slice::from_ref(&e), None).is_err());
        assert!(check_local_inventory(std::slice::from_ref(&e), Some(&e)).is_ok());
        let mut other = e.clone();
        other.executable = "another-editor".into();
        assert!(check_local_inventory(&[other], Some(&e)).is_err());
        assert!(check_local_inventory(&[], None).is_ok());
    }

    #[test]
    fn changed_install_paths_or_occupied_targets_reject_approved_plan() {
        let temp = tempfile::tempdir().unwrap();
        let plan = Plan {
            action: Action::InstallEditor,
            install_hub: false,
            editor_root: temp.path().join(EDITOR_VERSION),
            cache_root: temp.path().join("cache"),
            modules: vec!["android".into()],
            download_bytes: 1,
            reserve_bytes: 1,
            log_directory: temp.path().into(),
        };
        let paths = json!({"editorInstallPath": temp.path(), "downloadCachePath": plan.cache_root});
        assert!(check_plan_current(&plan, &paths, &json!([])).is_ok());
        let mut changed = paths.clone();
        changed["downloadCachePath"] = json!(temp.path().join("changed"));
        assert!(check_plan_current(&plan, &changed, &json!([])).is_err());
        fs::create_dir(&plan.editor_root).unwrap();
        assert!(check_plan_current(&plan, &paths, &json!([])).is_err());
    }
}
