use flate2::read::GzDecoder;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tar::Archive;
use wait_timeout::ChildExt;

pub const EDITOR_VERSION: &str = "6000.3.21f1";
pub const EDITOR_CHANGESET: &str = "c02631ffc030";
pub const CREATOR_SDK_VERSION: &str = "4.0.14";
pub const URP_VERSION: &str = "17.3.0";
pub const INPUT_SYSTEM_VERSION: &str = "1.20.0";
pub const SDK_DECLARED_URP_VERSION: &str = "17.4.0";
const REGISTRY_URL: &str = "https://greenfield-registry.sdq.st";
const REQUIRED_BUILTIN_MODULES: &[&str] = &[
    "com.unity.modules.accessibility",
    "com.unity.modules.adaptiveperformance",
    "com.unity.modules.ai",
    "com.unity.modules.androidjni",
    "com.unity.modules.animation",
    "com.unity.modules.assetbundle",
    "com.unity.modules.audio",
    "com.unity.modules.cloth",
    "com.unity.modules.director",
    "com.unity.modules.imageconversion",
    "com.unity.modules.imgui",
    "com.unity.modules.jsonserialize",
    "com.unity.modules.particlesystem",
    "com.unity.modules.physics",
    "com.unity.modules.physics2d",
    "com.unity.modules.screencapture",
    "com.unity.modules.terrain",
    "com.unity.modules.terrainphysics",
    "com.unity.modules.tilemap",
    "com.unity.modules.ui",
    "com.unity.modules.uielements",
    "com.unity.modules.umbra",
    "com.unity.modules.unityanalytics",
    "com.unity.modules.unitywebrequest",
    "com.unity.modules.unitywebrequestassetbundle",
    "com.unity.modules.unitywebrequestaudio",
    "com.unity.modules.unitywebrequesttexture",
    "com.unity.modules.unitywebrequestwww",
    "com.unity.modules.vectorgraphics",
    "com.unity.modules.vehicles",
    "com.unity.modules.video",
    "com.unity.modules.vr",
    "com.unity.modules.wind",
    "com.unity.modules.xr",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    pub editor_version: String,
    pub editor_changeset: String,
    pub creator_sdk_version: String,
    pub urp_version: String,
    pub input_system_version: String,
    pub sdk_declared_urp_version: String,
    pub registry_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorInstallation {
    pub version: String,
    pub root: String,
    pub executable: String,
    pub exact_recipe: bool,
    pub android_player: bool,
    pub android_sdk: bool,
    pub android_ndk: bool,
    pub open_jdk: bool,
    pub windows_standalone: bool,
    pub urp_template: Option<String>,
    pub ready: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentReport {
    pub platform: String,
    pub hub_installed: bool,
    pub hub_path: Option<String>,
    pub unity_cli_installed: bool,
    pub unity_cli_path: Option<String>,
    pub recipe: Recipe,
    pub editors: Vec<EditorInstallation>,
    pub ready: bool,
    pub blockers: Vec<String>,
    pub suggested_project_parent: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequest {
    pub project_name: String,
    pub parent_directory: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreationResult {
    pub success: bool,
    pub project_path: String,
    pub log_path: String,
    pub receipt_path: String,
    pub message: String,
}

pub fn recipe() -> Recipe {
    Recipe {
        editor_version: EDITOR_VERSION.into(),
        editor_changeset: EDITOR_CHANGESET.into(),
        creator_sdk_version: CREATOR_SDK_VERSION.into(),
        urp_version: URP_VERSION.into(),
        input_system_version: INPUT_SYSTEM_VERSION.into(),
        sdk_declared_urp_version: SDK_DECLARED_URP_VERSION.into(),
        registry_url: REGISTRY_URL.into(),
    }
}

fn platform_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

fn hub_candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if cfg!(target_os = "windows") {
        if let Some(program_files) = env::var_os("ProgramFiles") {
            paths.push(PathBuf::from(program_files).join("Unity Hub/Unity Hub.exe"));
        }
    } else if cfg!(target_os = "macos") {
        paths.push(PathBuf::from("/Applications/Unity Hub.app"));
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join("Applications/Unity Hub.app"));
        }
    } else {
        paths.extend([
            PathBuf::from("/usr/bin/unityhub"),
            PathBuf::from("/usr/bin/unityhub-bin"),
            PathBuf::from("/opt/unityhub/unityhub"),
        ]);
    }
    paths
}

fn editor_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(custom) = env::var_os("CREATOR_SETUP_EDITOR_ROOT") {
        roots.push(PathBuf::from(custom));
    }
    if cfg!(target_os = "windows") {
        if let Some(program_files) = env::var_os("ProgramFiles") {
            roots.push(PathBuf::from(program_files).join("Unity/Hub/Editor"));
        }
    } else if cfg!(target_os = "macos") {
        roots.push(PathBuf::from("/Applications/Unity/Hub/Editor"));
    } else if let Some(home) = dirs::home_dir() {
        roots.push(home.join("Unity/Hub/Editor"));
        roots.push(home.join(".local/share/unityhub/editors"));
    }
    roots.sort();
    roots.dedup();
    roots
}

fn executable_for(root: &Path) -> PathBuf {
    if cfg!(target_os = "windows") {
        root.join("Editor/Unity.exe")
    } else if cfg!(target_os = "macos") {
        root.join("Unity.app/Contents/MacOS/Unity")
    } else {
        root.join("Editor/Unity")
    }
}

fn editor_data(root: &Path) -> PathBuf {
    if cfg!(target_os = "macos") {
        root.join("Unity.app/Contents")
    } else {
        root.join("Editor/Data")
    }
}

fn find_case_insensitive_child(parent: &Path, expected: &str) -> Option<PathBuf> {
    fs::read_dir(parent)
        .ok()?
        .filter_map(Result::ok)
        .find_map(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case(expected)
                .then(|| entry.path())
        })
}

fn find_urp_template(data: &Path) -> Option<PathBuf> {
    let directories = [
        data.join("Resources/PackageManager/ProjectTemplates"),
        data.join("PackageManager/ProjectTemplates"),
    ];
    for directory in directories {
        let Ok(entries) = fs::read_dir(directory) else {
            continue;
        };
        let mut matches = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                let name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
                name.ends_with(".tgz")
                    && (name.contains("template.3d-cross-platform")
                        || name.contains("template.universal"))
            })
            .collect::<Vec<_>>();
        matches.sort();
        if let Some(path) = matches.pop() {
            return Some(path);
        }
    }
    None
}

fn find_on_path(command: &str) -> Option<PathBuf> {
    let names = if cfg!(target_os = "windows") {
        vec![format!("{command}.exe"), command.to_owned()]
    } else {
        vec![command.to_owned()]
    };
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .flat_map(|directory| names.iter().map(move |name| directory.join(name)))
        .find(|candidate| candidate.is_file())
}

fn find_unity_cli() -> Option<PathBuf> {
    if let Some(path) = find_on_path("unity") {
        return Some(path);
    }
    let home = dirs::home_dir()?;
    let candidates = if cfg!(target_os = "windows") {
        vec![home.join("AppData/Local/Microsoft/WindowsApps/unity.exe")]
    } else {
        vec![
            home.join(".local/bin/unity"),
            home.join(".unity/bin/unity"),
            PathBuf::from("/opt/homebrew/bin/unity"),
            PathBuf::from("/usr/local/bin/unity"),
        ]
    };
    candidates.into_iter().find(|path| path.is_file())
}

fn inspect_editor(root: PathBuf) -> Option<EditorInstallation> {
    let version = root.file_name()?.to_string_lossy().to_string();
    let executable = executable_for(&root);
    if !executable.is_file() {
        return None;
    }
    let playback = editor_data(&root).join("PlaybackEngines");
    let android = find_case_insensitive_child(&playback, "AndroidPlayer");
    let windows = find_case_insensitive_child(&playback, "WindowsStandaloneSupport");
    let android_sdk = android
        .as_ref()
        .is_some_and(|path| path.join("SDK").is_dir());
    let android_ndk = android
        .as_ref()
        .is_some_and(|path| path.join("NDK").is_dir());
    let open_jdk = android
        .as_ref()
        .is_some_and(|path| path.join("OpenJDK").is_dir());
    let template = find_urp_template(&editor_data(&root));
    let exact_recipe = version == EDITOR_VERSION;
    let ready = exact_recipe
        && android.is_some()
        && android_sdk
        && android_ndk
        && open_jdk
        && windows.is_some()
        && template.is_some();
    Some(EditorInstallation {
        version,
        root: root.to_string_lossy().to_string(),
        executable: executable.to_string_lossy().to_string(),
        exact_recipe,
        android_player: android.is_some(),
        android_sdk,
        android_ndk,
        open_jdk,
        windows_standalone: windows.is_some(),
        urp_template: template.map(|path| path.to_string_lossy().to_string()),
        ready,
    })
}

pub fn probe_environment() -> EnvironmentReport {
    let hub = hub_candidates().into_iter().find(|path| path.exists());
    let unity_cli = find_unity_cli();
    let mut editors = Vec::new();
    for parent in editor_roots() {
        if let Ok(entries) = fs::read_dir(parent) {
            editors.extend(entries.filter_map(Result::ok).filter_map(|entry| {
                entry
                    .file_type()
                    .ok()?
                    .is_dir()
                    .then(|| entry.path())
                    .and_then(inspect_editor)
            }));
        }
    }
    editors.sort_by(|a, b| b.version.cmp(&a.version));
    editors.dedup_by(|a, b| a.root == b.root);
    let exact = editors.iter().find(|editor| editor.exact_recipe);
    let mut blockers = Vec::new();
    if hub.is_none() && unity_cli.is_none() {
        blockers
            .push("Unity Hub or Unity CLI is required to manage the Editor and license.".into());
    }
    match exact {
        None => blockers.push(format!("Unity Editor {EDITOR_VERSION} is not installed.")),
        Some(editor) => {
            if !editor.android_player {
                blockers.push("Android Build Support is missing.".into());
            }
            if !editor.android_sdk || !editor.android_ndk || !editor.open_jdk {
                blockers.push("Android SDK, NDK, or OpenJDK child modules are missing.".into());
            }
            if !editor.windows_standalone {
                blockers.push("Windows standalone build support is missing.".into());
            }
            if editor.urp_template.is_none() {
                blockers.push("The Editor's official 3D URP template is missing.".into());
            }
        }
    }
    EnvironmentReport {
        platform: platform_name().into(),
        hub_installed: hub.is_some(),
        hub_path: hub.map(|path| path.to_string_lossy().to_string()),
        unity_cli_installed: unity_cli.is_some(),
        unity_cli_path: unity_cli.map(|path| path.to_string_lossy().to_string()),
        recipe: recipe(),
        ready: blockers.is_empty() && exact.is_some_and(|editor| editor.ready),
        editors,
        blockers,
        suggested_project_parent: dirs::document_dir()
            .or_else(dirs::home_dir)
            .map(|path| path.to_string_lossy().to_string()),
    }
}

fn validate_project_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 64 {
        return Err("Project name must contain 1 to 64 characters.".into());
    }
    if !name
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, ' ' | '-' | '_'))
    {
        return Err("Use only letters, numbers, spaces, hyphens, and underscores.".into());
    }
    if name.ends_with('.') || name.ends_with(' ') {
        return Err("Project name cannot end with a space or period.".into());
    }
    Ok(())
}

fn extract_project_template(template: &Path, target: &Path) -> Result<(), String> {
    let file =
        File::open(template).map_err(|error| format!("Cannot open URP template: {error}"))?;
    let mut archive = Archive::new(GzDecoder::new(file));
    let entries = archive
        .entries()
        .map_err(|error| format!("Cannot read URP template: {error}"))?;
    for item in entries {
        let mut entry = item.map_err(|error| format!("Cannot read template entry: {error}"))?;
        let path = entry
            .path()
            .map_err(|error| format!("Invalid template path: {error}"))?
            .into_owned();
        let relative = match path.strip_prefix("package/ProjectData~") {
            Ok(relative) if !relative.as_os_str().is_empty() => relative,
            _ => continue,
        };
        if relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            return Err("The Unity template contains an unsafe path.".into());
        }
        let allowed = matches!(
            relative.components().next(),
            Some(Component::Normal(name)) if name == "Assets" || name == "Packages" || name == "ProjectSettings"
        );
        if !allowed {
            continue;
        }
        let entry_type = entry.header().entry_type();
        if !entry_type.is_file() && !entry_type.is_dir() {
            return Err("The Unity template contains an unsupported link or special entry.".into());
        }
        let destination = target.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "Cannot create template directory {}: {error}",
                    parent.display()
                )
            })?;
        }
        entry
            .unpack(&destination)
            .map_err(|error| format!("Cannot extract {}: {error}", relative.display()))?;
    }
    Ok(())
}

fn update_manifest(project: &Path) -> Result<(), String> {
    let path = project.join("Packages/manifest.json");
    let text =
        fs::read_to_string(&path).map_err(|error| format!("Cannot read manifest: {error}"))?;
    let mut manifest: Value = serde_json::from_str(&text)
        .map_err(|error| format!("Invalid template manifest: {error}"))?;
    let dependencies = manifest
        .get_mut("dependencies")
        .and_then(Value::as_object_mut)
        .ok_or("Template manifest has no dependencies object.")?;
    // The stock URP template omits built-in modules referenced by Basis SDK.
    for module in REQUIRED_BUILTIN_MODULES {
        dependencies.insert((*module).into(), json!("1.0.0"));
    }
    // The stock template's Input System 1.12.0 does not compile against the
    // approved Unity 6000.3 line, so use the exact version from the passing smoke test.
    dependencies.insert("com.unity.inputsystem".into(), json!(INPUT_SYSTEM_VERSION));
    dependencies.insert(
        "com.sidequest.creator-sdk".into(),
        json!(CREATOR_SDK_VERSION),
    );
    dependencies.insert(
        "com.unity.render-pipelines.universal".into(),
        json!(URP_VERSION),
    );
    manifest["scopedRegistries"] = json!([{
        "name": "Greenfield-registry.sdq.st",
        "url": REGISTRY_URL,
        "scopes": [
            "com.sidequest.creator-sdk",
            "com.basis.bundlemanagement",
            "com.basis.common",
            "com.basis.sdk",
            "com.sidequest.ora",
            "com.sidequest.thirdparty.bouncycastle"
        ]
    }]);
    let serialized = serde_json::to_string_pretty(&manifest)
        .map_err(|error| format!("Cannot serialize manifest: {error}"))?;
    fs::write(&path, format!("{serialized}\n"))
        .map_err(|error| format!("Cannot write manifest: {error}"))?;
    let stale_lock = project.join("Packages/packages-lock.json");
    if stale_lock.exists() {
        fs::remove_file(stale_lock)
            .map_err(|error| format!("Cannot remove template lock: {error}"))?;
    }
    Ok(())
}

fn update_project_name(project: &Path, project_name: &str) -> Result<(), String> {
    let path = project.join("ProjectSettings/ProjectSettings.asset");
    let text = fs::read_to_string(&path)
        .map_err(|error| format!("Cannot read project settings: {error}"))?;
    let mut replaced = false;
    let output = text
        .lines()
        .map(|line| {
            if line.trim_start().starts_with("productName:") {
                replaced = true;
                format!("  productName: {project_name}")
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    if !replaced {
        return Err("Unity template does not contain a productName setting.".into());
    }
    fs::write(path, format!("{output}\n"))
        .map_err(|error| format!("Cannot write project settings: {error}"))
}

fn validation_script() -> String {
    format!(
        r#"using System;
using System.IO;
using UnityEditor;
using UnityEngine;
using UnityEngine.Rendering;

namespace CreatorWorks
{{
    public static class ProjectSetupValidator
    {{
        [Serializable]
        private sealed class Result
        {{
            public bool success;
            public string unityVersion;
            public string creatorSdkVersion;
            public string urpVersion;
            public string inputSystemVersion;
            public bool urpConfigured;
            public bool androidSupported;
            public bool windowsSupported;
            public string checkedAtUtc;
        }}

        public static void Validate()
        {{
            var package = UnityEditor.PackageManager.PackageInfo.FindForPackageName("com.sidequest.creator-sdk");
            var urpPackage = UnityEditor.PackageManager.PackageInfo.FindForPackageName("com.unity.render-pipelines.universal");
            var inputPackage = UnityEditor.PackageManager.PackageInfo.FindForPackageName("com.unity.inputsystem");
            var result = new Result
            {{
                unityVersion = Application.unityVersion,
                creatorSdkVersion = package == null ? "missing" : package.version,
                urpVersion = urpPackage == null ? "missing" : urpPackage.version,
                inputSystemVersion = inputPackage == null ? "missing" : inputPackage.version,
                urpConfigured = GraphicsSettings.defaultRenderPipeline != null,
                androidSupported = BuildPipeline.IsBuildTargetSupported(BuildTargetGroup.Android, BuildTarget.Android),
                windowsSupported = BuildPipeline.IsBuildTargetSupported(BuildTargetGroup.Standalone, BuildTarget.StandaloneWindows64),
                checkedAtUtc = DateTime.UtcNow.ToString("O")
            }};
            result.success = result.unityVersion == "{EDITOR_VERSION}"
                && result.creatorSdkVersion == "{CREATOR_SDK_VERSION}"
                && result.urpVersion == "{URP_VERSION}"
                && result.inputSystemVersion == "{INPUT_SYSTEM_VERSION}"
                && result.urpConfigured
                && result.androidSupported
                && result.windowsSupported;
            var root = Directory.GetParent(Application.dataPath).FullName;
            var folder = Path.Combine(root, ".creator-project-setup");
            Directory.CreateDirectory(folder);
            File.WriteAllText(Path.Combine(folder, "unity-validation.json"), JsonUtility.ToJson(result, true));
            AssetDatabase.SaveAssets();
            EditorApplication.Exit(result.success ? 0 : 2);
        }}
    }}
}}
"#
    )
}

fn write_receipt(project: &Path, value: &Value) -> Result<PathBuf, String> {
    let folder = project.join(".creator-project-setup");
    fs::create_dir_all(&folder)
        .map_err(|error| format!("Cannot create receipt folder: {error}"))?;
    let path = folder.join("setup-receipt.json");
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| format!("Cannot serialize setup receipt: {error}"))?;
    fs::write(&path, format!("{text}\n"))
        .map_err(|error| format!("Cannot write setup receipt: {error}"))?;
    Ok(path)
}

pub fn create_project(request: CreateRequest) -> Result<CreationResult, String> {
    let project_name = request.project_name.trim();
    validate_project_name(project_name)?;
    let parent = PathBuf::from(request.parent_directory.trim());
    if !parent.is_dir() {
        return Err("Choose an existing parent folder for the project.".into());
    }
    let target = parent.join(project_name);
    if target.exists() {
        return Err(format!(
            "{} already exists. Creator Project Setup never overwrites a project folder.",
            target.display()
        ));
    }
    let environment = probe_environment();
    if !environment.ready {
        return Err(format!(
            "This computer is not ready: {}",
            environment.blockers.join(" ")
        ));
    }
    let editor = environment
        .editors
        .iter()
        .find(|editor| editor.ready)
        .ok_or("Compatible Unity Editor could not be selected.")?;
    let template = PathBuf::from(
        editor
            .urp_template
            .as_ref()
            .ok_or("The official URP template is unavailable.")?,
    );

    fs::create_dir(&target).map_err(|error| format!("Cannot create project folder: {error}"))?;
    let setup = (|| -> Result<CreationResult, String> {
        extract_project_template(&template, &target)?;
        fs::write(
            target.join("ProjectSettings/ProjectVersion.txt"),
            format!(
                "m_EditorVersion: {EDITOR_VERSION}\nm_EditorVersionWithRevision: {EDITOR_VERSION} ({EDITOR_CHANGESET})\n"
            ),
        )
        .map_err(|error| format!("Cannot write ProjectVersion.txt: {error}"))?;
        update_manifest(&target)?;
        update_project_name(&target, project_name)?;

        let validator_dir = target.join("Assets/Editor");
        fs::create_dir_all(&validator_dir)
            .map_err(|error| format!("Cannot create temporary validator folder: {error}"))?;
        let validator = validator_dir.join("CreatorProjectSetupValidator.cs");
        fs::write(&validator, validation_script())
            .map_err(|error| format!("Cannot write temporary validator: {error}"))?;

        let status_folder = target.join(".creator-project-setup");
        fs::create_dir_all(&status_folder)
            .map_err(|error| format!("Cannot create status folder: {error}"))?;
        let log_path = status_folder.join("unity-setup.log");
        let _receipt_path = write_receipt(
            &target,
            &json!({
                "success": false,
                "stage": "launching_unity",
                "projectPath": target,
                "recipe": recipe(),
                "activeBuildTarget": "android",
                "requiredBuildTargets": ["android", "windows"],
                "note": "Unity setup has started. This is not a completion receipt."
            }),
        )?;

        let mut child = Command::new(&editor.executable)
            .args(["-batchmode", "-quit", "-nographics", "-projectPath"])
            .arg(&target)
            .args([
                "-buildTarget",
                "Android",
                "-executeMethod",
                "CreatorWorks.ProjectSetupValidator.Validate",
                "-logFile",
            ])
            .arg(&log_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("Cannot launch Unity: {error}"))?;

        let exit = child
            .wait_timeout(Duration::from_secs(30 * 60))
            .map_err(|error| format!("Cannot wait for Unity: {error}"))?;
        let status = match exit {
            Some(status) => status,
            None => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "Unity setup exceeded 30 minutes. The project and log were preserved at {}.",
                    target.display()
                ));
            }
        };
        let validation_path = status_folder.join("unity-validation.json");
        if !status.success() || !validation_path.is_file() {
            return Err(format!(
                "Unity did not validate the project. Review {}. The partial project was preserved.",
                log_path.display()
            ));
        }
        let mut validation_text = String::new();
        File::open(&validation_path)
            .and_then(|mut file| file.read_to_string(&mut validation_text))
            .map_err(|error| format!("Cannot read Unity validation: {error}"))?;
        let validation: Value = serde_json::from_str(&validation_text)
            .map_err(|error| format!("Invalid Unity validation result: {error}"))?;
        if validation.get("success") != Some(&Value::Bool(true)) {
            return Err(format!(
                "Unity completed but the compatibility checks failed. Review {}.",
                validation_path.display()
            ));
        }

        let _ = fs::remove_file(&validator);
        let _ = fs::remove_file(validator.with_extension("cs.meta"));
        let receipt_path = write_receipt(
            &target,
            &json!({
                "success": true,
                "stage": "validated",
                "projectPath": target,
                "recipe": recipe(),
                "activeBuildTarget": "android",
                "requiredBuildTargets": ["android", "windows"],
                "validation": validation,
                "logPath": log_path
            }),
        )?;
        Ok(CreationResult {
            success: true,
            project_path: target.to_string_lossy().to_string(),
            log_path: log_path.to_string_lossy().to_string(),
            receipt_path: receipt_path.to_string_lossy().to_string(),
            message: "Creator SDK project compiled and passed the baseline checks.".into(),
        })
    })();

    if let Err(error) = &setup {
        let _ = write_receipt(
            &target,
            &json!({
                "success": false,
                "stage": "failed",
                "projectPath": target,
                "recipe": recipe(),
                "error": error,
                "note": "The project was preserved for diagnosis and was not deleted automatically."
            }),
        );
    }
    setup
}

pub fn open_project(path: &str) -> Result<(), String> {
    let project = PathBuf::from(path);
    if !project.join("ProjectSettings/ProjectVersion.txt").is_file() {
        return Err("The selected path is not a validated Unity project.".into());
    }
    let editor = probe_environment()
        .editors
        .into_iter()
        .find(|editor| editor.exact_recipe)
        .ok_or("The compatible Unity Editor is not installed.")?;
    Command::new(editor.executable)
        .arg("-projectPath")
        .arg(project)
        .spawn()
        .map_err(|error| format!("Cannot open Unity project: {error}"))?;
    Ok(())
}

pub fn launch_hub() -> Result<(), String> {
    let report = probe_environment();
    let path = report.hub_path.ok_or("Unity Hub is not installed.")?;
    let mut command = if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(path);
        command
    } else {
        Command::new(path)
    };
    command
        .spawn()
        .map_err(|error| format!("Cannot open Unity Hub: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipe_is_pinned() {
        let value = recipe();
        assert_eq!(value.editor_version, "6000.3.21f1");
        assert_eq!(value.creator_sdk_version, "4.0.14");
        assert_eq!(value.urp_version, "17.3.0");
        assert_eq!(value.input_system_version, "1.20.0");
        assert_eq!(value.sdk_declared_urp_version, "17.4.0");
    }

    #[test]
    fn rejects_unsafe_project_names() {
        for name in [
            "",
            "..",
            "bad/name",
            "bad\\name",
            "bad:name",
            "trailing.",
            "comment#name",
        ] {
            assert!(validate_project_name(name).is_err(), "accepted {name:?}");
        }
        assert!(validate_project_name("My Creator Space").is_ok());
    }

    #[test]
    fn manifest_adds_pinned_sdk_and_registry() {
        let root =
            env::temp_dir().join(format!("creator-project-setup-test-{}", std::process::id()));
        let packages = root.join("Packages");
        fs::create_dir_all(&packages).unwrap();
        fs::write(
            packages.join("manifest.json"),
            r#"{"dependencies":{"com.unity.render-pipelines.universal":"17.0.1"}}"#,
        )
        .unwrap();
        fs::write(packages.join("packages-lock.json"), "{}").unwrap();
        update_manifest(&root).unwrap();
        let value: Value =
            serde_json::from_str(&fs::read_to_string(packages.join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(
            value["dependencies"]["com.sidequest.creator-sdk"],
            CREATOR_SDK_VERSION
        );
        assert_eq!(
            value["dependencies"]["com.unity.render-pipelines.universal"],
            URP_VERSION
        );
        assert_eq!(value["dependencies"]["com.unity.modules.cloth"], "1.0.0");
        assert_eq!(
            value["dependencies"]["com.unity.inputsystem"],
            INPUT_SYSTEM_VERSION
        );
        assert_eq!(value["scopedRegistries"][0]["url"], REGISTRY_URL);
        assert!(!packages.join("packages-lock.json").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    #[ignore = "launches a real installed Unity Editor and creates a disposable project"]
    fn unity_creation_smoke() {
        let parent = env::var("CREATOR_SETUP_SMOKE_PARENT")
            .expect("set CREATOR_SETUP_SMOKE_PARENT to an existing disposable test directory");
        let name =
            env::var("CREATOR_SETUP_SMOKE_NAME").unwrap_or_else(|_| "CreatorSetupSmoke".into());
        let result = create_project(CreateRequest {
            project_name: name,
            parent_directory: parent,
        })
        .expect("real Unity setup should succeed");
        assert!(result.success);
        assert!(Path::new(&result.receipt_path).is_file());
        println!("{}", serde_json::to_string(&result).unwrap());
    }
}
