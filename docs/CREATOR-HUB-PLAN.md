# Creator Hub: Approved Direction

Decision: 2026-09-10. Creator Hub is an optional, lightweight install/open/update
front door. Creator Project Setup 0.2.2 has shipped publicly; none of this changes
its immutable downloads or claims Hub compatibility for that existing build.

## Product Boundary

- Collapsible left navigation with a real menu toggle and icon tooltips.
- Shared compact chrome: a protruding cube-logo tab without hamburger lines,
  H/M/P corner badges, and a gray outer frame for Hub. Keep standalone navigation
  until a trusted hosted-context contract exists; installation presence alone
  never changes how the UI behaves.
- Creator Works MCP and Creator Project Setup show Install, Open, or Update.
- URP Converter is Coming soon, with no fake install or download action.
- Flat tool detail pages, installed/available versions, download progress, and
  visible failures. No account, subscription, always-running service, or Unity
  Editor bundled in Hub.
- Optional first-launch Unity/Creator SDK assistance belongs in Hub. Keep a
  permanent Setup action. Existing standalone users do not acquire a mandatory
  onboarding flow, another app copy, or changed settings.
- Initial Open launches the normal standalone app, not an embedded EXE window.
  Closing Hub must not close tools, Unity, or MCP server processes. A future
  hosted interface should share the tool's implementation, not fork its logic.

## Ownership

Hub owns the catalog, installed-app inventory, update comparison and user-approved
download/install orchestration. Each tool owns its GUI, project operations,
settings, lifecycle safeguards, and independently usable release. Unity Hub still
owns Unity installation, accounts, licenses and build-module handoffs.

The BANTWORKS MCP task owns MCP compatibility. This task owns Setup compatibility
and the shared contract below. The URP converter remains separate work. Do not
change its repository or promise compatibility before its owner agrees.

## Contract 1: Read-only Identity

The exact argument `--creator-hub-info` must exit successfully with one compact
JSON object and a newline on stdout. It must run before GUI, single-instance,
network, configuration migration, project scanning or Unity work. Hub captures
stdout/stderr with a timeout and a maximum 4 KiB combined response. Unknown/extra Hub
arguments fail nonzero with no GUI and no operation. No output path, command,
project route, repair permission or installer instruction is accepted.

Required fields:

```json
{
  "schemaVersion": 1,
  "appId": "creator-project-setup",
  "displayName": "Creator Project Setup",
  "version": "0.3.0-alpha.1",
  "platform": "windows",
  "architecture": "x86_64",
  "capabilities": ["launch.standalone"]
}
```

Stable app IDs: `creator-project-setup`, `creator-works-mcp`. Reserve
`creator-urp-converter` but do not publish a supported entry yet. Platform names
are `windows`, `macos`, `linux`; architectures are `x86_64`, `aarch64` for the
initial catalog. Version is the actual compiled SemVer, not a hard-coded feed
value. Ignore additional metadata fields, reject unsupported schema versions,
and never interpret returned strings as shell commands or trust decisions.

`launch.singleInstance` means a normal second invocation on that platform
reopens the running compatible GUI without resetting it or triggering work. It
does not cover pre-contract releases, elevated copies or a different login
session, and does not indicate a tool is idle or safe to replace. Hub must verify
an executable's provenance before executing even this read-only flag. Its
trusted release descriptor must explicitly advertise `identityProtocol: 1`.
Legacy releases may ignore unknown arguments: do not probe them based on a name
or version guess. Use a verified descriptor or ask the user to install a
supported version. Failed probes must not fall back to ordinary launch.

## Contract 2: Release and Update Catalog

The Hub must discover newer tool releases without requiring a new Hub build.
Use a small authenticated catalog with per-tool stable/preview channels, SemVer,
minimum Hub contract, release-note URL, and per-platform/architecture artifact
records (HTTPS URL, exact size, SHA-256, package type and relative entrypoint).
No arbitrary scripts, command templates or caller-controlled executable paths.

GitHub public release metadata is a discovery source, not arbitrary executable
authority. Only approved repositories/artifacts belong in the catalog. Release
descriptors are data; an adjacent checksum alone is not publisher authentication.
Before automatic downloading/launching ships, establish catalog signing with a
pinned public key, key recovery/rotation, bounded parsing, and rollback/replay
policy. This is separate from paid Windows Authenticode or macOS notarization.

Compare Semantic Versions, not strings or release dates. Ignore drafts and
prereleases by default; preview is explicit opt-in. No downgrade of an installed
newer version. Cache verified catalog data for offline use, show stale/failed
checks honestly, and check at startup/on request rather than continuous polling.
Background downloading is opt-in; installation is a separate explicit action.

## Installation Safety

- Detect existing installations and offer reuse. Hub inventory is advisory, not
  permission to execute an arbitrary file. Recheck identities and hashes.
- Hub-managed portable installs get versioned directories and a retained prior
  version. Existing native/legacy installations use their supported updater or
  an explicit migration; do not create a competing installation silently.
- Check free space on download, extraction, target and rollback volumes.
- Verify metadata before download and size/hash before extraction or execution.
  Reject archive traversal, links, unexpected entrypoints and partial payloads.
- Do not replace running tools or force-close them. MCP includes long-lived
  server processes used by AI clients, not just its launcher window. Reliable
  process/operation detection is a release gate, not implied by the info command.
- Never change a Unity project, bridge, active MCP route, or AI client settings
  as a side effect of app discovery, Hub closure, or a tool's process exit.
- Keep logs local, bounded and redacted. Uninstalling Hub leaves independently
  installed tools and all user projects/settings intact.

## Delivery Order and Gates

1. Done: publish the tested Setup 0.2.2 Windows release publicly, unchanged.
2. In progress: next-version read-only metadata in Setup and MCP, with matching
   field names and automated/native tests. Reliable Windows re-open is a
   separate gate before either app advertises `launch.singleInstance`.
3. Next: release descriptors, authenticated catalog and deterministic update
   comparison tests; public assets must be downloadable without credentials.
4. Next: minimal Hub install/open/update workflow and fake/offline fixture tests.
   No shell-command execution from catalog data and no running-app replacement.
5. Accept: fresh install, existing install adoption, cancellation, offline,
   signature/hash failures, disk full, newer installed version, preview channels,
   running MCP server, and repeated Open with preserved GUI state.
6. Later: a shared project picker and hosted tool interfaces, only if useful.
   Player emulation is outside this plan; URP remains Coming soon.

## Interface Pass: 2026-09-10

Setup and MCP now have coordinated next-version compact interfaces. Setup's
creation fields collapse into a summary/progress view without changing the
underlying Unity creation or repair contracts. See [UI preview](UI-PREVIEW.md).

A separate local Creator Hub `0.1.0-alpha.1` interface shell now provides an app
catalog, detail views, accessible switcher and pinned public release/source
links. It does not yet install, inventory, update, or embed apps. Its Windows
EXE is an interface preview, not completion of gates 3-5 above. This distinction
must remain visible in the app and any download description.

Next-version support may ship independently after both app contracts agree.
It must not be advertised as a complete Hub or safe automatic updater. Windows
is first; macOS/Linux need native lifecycle, packaging and Unity acceptance.

## Lifecycle Review Finding

The current Windows source for the official Tauri single-instance plugin 2.4.4
allows initialization to continue when its mutex exists but its message window
does not yet exist. Its forwarding call is also synchronous without a timeout.
This is a source-level startup-race/blocked-window risk, not a reproduced failure
in Setup. The candidate plugin was removed from this first metadata-only pass;
no duplicate-launch prevention is claimed. Require simultaneous cold-start and
blocked-GUI tests before adding the capability. Use a narrow reviewed solution,
not an expanding custom IPC framework.

Source reviewed: [Tauri single-instance Windows implementation](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/single-instance/src/platform_impl/windows.rs).
