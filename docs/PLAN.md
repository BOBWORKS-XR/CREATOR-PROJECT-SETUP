# Product Plan

## 0.1 - New Project Baseline

- Windows portable app and installer.
- Environment detection with exact missing requirements.
- Guarded creation from Unity's installed URP template.
- Pinned Creator SDK registry installation.
- Android active by default, with Android and Windows support both mandatory.
- Batch compilation and a local receipt.
- Initialize Creator Visual Scripting and validate it in a second Editor session.
- Real setup stages and elapsed time; explicit completion/Open/Create another states.
- macOS/Linux packaging and detection preview.

## 0.2 - Hub Registration and Repair Preview

- Inspect an existing project without changing it.
- Show a proposed compatibility repair.
- Reject open projects, stale reviews, links and incompatible versions.
- Back up manifests, project settings and generated Visual Scripting data before
  an approved repair or Unity validation; verify backup file hashes.
- Merge custom Visual Scripting selections and verify them on reopening.
- Re-run package, compilation, render-pipeline, and platform validation.
- Register completed new projects using a pinned, checksum-verified official
  Unity CLI helper with a compatible desktop Hub. Require Hub 3.21.1+, distinguish
  CLI records from UI visibility, and allow independent retry. Older Hubs keep the
  manual Add-from-disk route; do not present CLI-only records as desktop success.
- Do not edit Hub's private database or silently link Unity Cloud.
- Windows 0.2.2 adds an explicit confirmed Hub restart to reload the running
  list; it never force-closes Hub or targets Unity Editors.

Still required before promoting repair beyond preview: broader existing-project
fixtures, recovery UX, third-party package callback coverage, and native
macOS/Linux acceptance. Backups are selective; failure recovery is manual.

## 0.3 - Optional Creator Works MCP

- Offer Creator Works MCP after the Unity project passes validation.
- Keep MCP setup optional and independent from SDK correctness.
- Verify the selected project and bridge version after setup.
- Coordinate the reciprocal optional first-launch/manual Setup companion in
  the MCP launcher. See [MCP-INTEGRATION.md](MCP-INTEGRATION.md); the initial
  interface is an ordinary GUI launch and manual project selection, not IPC.

## Next Safety Pass

- Add project/cache volume free-space preflight and a clear disk-full failure.
  A packaged 0.2.2 test on F: failed during UPM extraction with `ENOSPC`; Setup
  preserved the failed folder/logs but only reported a generic Unity failure.
  Do not auto-delete unrelated content or retry a partial project as new.

## Later

- Tested macOS and Linux installation workflows.
- Signed/notarized packages.
- SideQuest-maintained compatibility recipe feed with integrity metadata.
- Safe SDK upgrades with preview, backup, rollback, and migration checks.
- Team presets and optional starter samples.
