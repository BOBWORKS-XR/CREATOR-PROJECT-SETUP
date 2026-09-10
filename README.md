# Creator Project Setup

Creator Project Setup is a portable desktop wizard for creating a known-good
SideQuest Creator SDK project without asking a new Unity user to edit package
manifests, drag `.unitypackage` files, install Git, or use a terminal.

The first release is Windows-first, with shared macOS and Linux detection and
packaging support. Windows is the first physically tested platform; macOS and
Linux packages remain preview quality until their full Unity workflows have
been exercised on real machines.

## Download 0.2.2

Development branch: `0.3.0-alpha.1` adds read-only Creator Hub identity and a
compact shared app interface. See [the UI preview notes](docs/UI-PREVIEW.md).
These changes are not included in the stable download below.

[Download the Windows portable EXE](https://github.com/BOBWORKS-XR/CREATOR-PROJECT-SETUP/releases/download/v0.2.2/Creator-Project-Setup-0.2.2-Windows.exe)
or see the [0.2.2 release](https://github.com/BOBWORKS-XR/CREATOR-PROJECT-SETUP/releases/tag/v0.2.2)
for the optional installer, portable ZIP, checksums, and known limitations.
No GitHub account is required. The Windows files are unsigned.

The tested 0.2.2 Windows files were promoted unchanged from prerelease after the
Hub restart also passed user testing. Their existing footer still says Preview;
this is not a different download. Existing-project repair remains experimental,
and native macOS/Linux Unity workflows are not yet accepted as tested.

## Current Recipe

- Unity Editor `6000.3.21f1`
- SideQuest Creator SDK `4.0.14`
- Universal Render Pipeline `17.3.0` as resolved by the approved Editor
- Input System `1.20.0`
- Android Build Support with SDK, NDK, and OpenJDK, always required
- Windows standalone build support, always required

The recipe is deliberately pinned, including transitive packages that have
already caused compatibility failures, and updated only after a real compile and
validation run. Creator SDK `4.0.14` declares URP `17.4.0`, but the approved
Unity 6000.3 Editor resolves its built-in URP `17.3.0`; the application records
both the declared and observed values instead of hiding that discrepancy.

## First Release Scope

- Detect Unity Hub, Unity CLI, compatible Editors, and required build modules.
- Create a new project from the matching Editor's official 3D URP template.
- Add the official Greenfield scoped registry and pinned Creator SDK package.
- Resolve packages in batch mode with Android active and produce a local
  setup/validation receipt that confirms both Android and Windows support.
- Initialize Visual Scripting, apply the installed SDK's supported types and
  assemblies, and generate the Creator node database before completion.
- Reopen Unity in a second batch session to check that configuration persisted.
- Show real setup stages, recent import activity, and elapsed time. Stage counts
  are not download percentages or an estimate of time remaining.
- With a compatible Hub, add completed projects automatically and read back the
  registry to verify the exact project path. Opening the Editor is separate.
- Offer a confirmed Windows Hub restart to reload a running Hub's cached list,
  without closing Unity Editors or force-terminating Hub.
- Open Unity Hub for missing installations and open a completed project.
- Keep a completed project ready to open; creating another project is a separate
  action, so a second click cannot restart creation in the same folder.
- Never overwrite an existing project directory.

Unity account sign-in, license activation, administrator prompts, and module
license agreements remain explicit Unity/operating-system handoffs.

## Validation Boundaries

The receipt checks pinned package versions, an assigned URP asset, available
Android/Windows build targets, persisted Visual Scripting configuration, and a
nonempty Creator node database. It does not claim to have built a Windows player
or Android APK, uploaded a space, or tested interaction in a headset.

Setup logs and receipts are kept in `.creator-project-setup` inside the created
project. The temporary Editor validator is removed after successful creation.

## 0.2.2 Release

### Automatic Unity Hub Registration

Automatic registration requires **Unity Hub 3.21.1 or newer**, opened at least
once so its version can be detected. Older or unknown Hub versions do not block
project creation, but Setup reports that automatic registration is unavailable.
Use Hub's **Add > Add project from disk**, or update and open Hub, then select
**Recheck Hub and add project**. The project is not recreated.

After new-project validation, Setup uses the official
[Unity CLI project commands](https://docs.unity.com/en-us/unity-cli/unity-cli-reference#manage-projects-in-the-hub-registry)
to register the project. It downloads a private, pinned Unity CLI
`1.0.0-beta.9` helper from Unity's CDN (about 21 MB on Windows), verifies its
size and SHA-256, and rechecks the cached helper before execution. No Git,
terminal, PATH change, or separate CLI installation is required.

Registration has its own progress stage and retry action. A download or Hub
failure does not discard the validated project or prevent opening it. Setup does not
edit Hub's internal database, link a Unity Cloud project, or change analytics
consent. The helper is a Unity beta component, not source code bundled under this
repository's MIT license.

**Hub is open, but the new project is missing?** Hub 3.21.1 loads its registered
projects into memory at startup. CLI registration does not refresh that running
list. The completion screen keeps refresh pending and offers **Restart Unity
Hub** on Windows. Confirm only after Hub downloads and installations finish.
The action fully exits and reopens the detected Hub; closing its window alone
can leave it running in the system tray. Unity Editors remain open. A refused
shutdown is reported without force-closing anything, and the project remains
ready to open. Cancelling makes no changes.

Restart uses Windows Restart Manager with only the verified Hub executable's
main process ID and creation time. No project files, services, or Editor process
trees are registered for shutdown. No Hub code or database is patched. macOS
and Linux currently show manual full-quit/reopen instructions instead.

**Correction to 0.2.0:** a CLI readback was incorrectly presented as proof of
registration with any desktop Hub. Hub 3.14.4 reads `projects-v1.json`, whereas
the pinned CLI writes `hub.db`. Restarting that older Hub does not bridge the two
stores. Version 0.2.1 checks compatibility first and no longer claims the project
is visible in the Hub UI. The additional live-list issue was reproduced on Hub
3.21.1 and addressed in 0.2.2 with an explicit restart action. A native Windows
test confirmed the missing project appeared after restart while the existing
Unity Editor processes remained running. Database readback and process restart
still are not automatic visual verification of every user's Projects list.

### Existing Projects

1. Select **Existing project**, choose the Unity project folder, and inspect it.
   Inspection reads files only; it does not open Unity or change the project.
2. Review findings and proposed changes. Close the project in Unity before repair
   or validation. Changed configuration invalidates the earlier review.
3. Approve the settings backup, then apply the offered repairs or run validation.

This first repair pass handles missing tested packages, missing Creator registry
scopes, required built-in modules, and incomplete Creator Visual Scripting setup.
It preserves unrelated manifest entries and merges existing Visual Scripting
type selections. Conflicting registries, untested Editor/SDK/package versions,
linked project files, and open projects are refused, not silently rewritten.

**Backup scope:** package manifests, all project settings, and generated Visual
Scripting data. It is not a complete project backup. Unity imports and installed
package callbacks can modify assets. Use version control or a separate full copy
for valuable content. A failed run retains the backup and logs for **manual
recovery**; automatic rollback is not implemented in this preview.

Backups, reviewed plans, results, and Unity logs are stored under
`.creator-project-setup/backups/` in the selected project. Repair does not rebuild
scenes, convert materials, change render pipelines, upgrade Unity, or migrate
Banter projects. A missing URP assignment requires review in Unity.
See [repair recovery notes](docs/REPAIR-RECOVERY.md) before working on important
existing content.

## Earlier 0.1.1 Hotfix

- Fixed a first-open warning caused by missing Visual Scripting project settings.
- Added a second-process validation gate instead of relying on machine-wide SDK
  preferences or an in-memory initialization.
- Fixed the completion screen offering another creation attempt in an existing
  directory and losing the Open action after that attempt failed.
- Added staged progress, elapsed time, and frontend regression tests.

New projects receive these changes automatically. Existing projects are not
silently rewritten. `scripts/Check-CreatedProject.ps1` is a Windows maintenance
helper for projects with a setup receipt: with the Editor closed it can recheck
the project, or initialize Visual Scripting with `-ConfigureVisualScripting`.
It backs up existing VS settings and records before/after hashes for protected
scene, prefab, material, package, and project-settings files.

## Development

```powershell
npm install
npm run check
npx playwright install chromium
npm run test:ui
npm run dev
```

Build a native package with `npm run build`. Tagged builds are packaged for
Windows, macOS, and Linux by GitHub Actions.

## Roadmap

See [docs/PLAN.md](docs/PLAN.md). Existing-project repair is a separate preview
workflow; optional Creator Works MCP installation follows later and is not
required for SDK correctness.
The coordinated [MCP integration notes](docs/MCP-INTEGRATION.md) now point toward
an optional Creator Hub for installing, opening, and updating independent tools.
Hub compatibility is next-version work, not a feature of the 0.2.2 download.
See the [approved Creator Hub plan](docs/CREATOR-HUB-PLAN.md) for app ownership,
the metadata contract, update discovery, installation safeguards and release gates.

## License

MIT. Unity and SideQuest products, trademarks, packages, and installers remain
the property of their respective owners. The application downloads or invokes
official sources and does not redistribute the Unity Editor.
