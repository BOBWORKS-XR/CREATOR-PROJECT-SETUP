# Product Plan

## 0.1 - New Project Baseline

- Windows portable app and installer.
- Environment detection with exact missing requirements.
- Guarded creation from Unity's installed URP template.
- Pinned Creator SDK registry installation.
- Android active by default, with Android and Windows support both mandatory.
- Batch compilation and a local receipt.
- macOS/Linux packaging and detection preview.

## 0.2 - Repair Mode

- Inspect an existing project without changing it.
- Show a proposed compatibility repair.
- Back up manifests and project settings before an approved repair.
- Re-run package, compilation, render-pipeline, and platform validation.

## 0.3 - Optional Creator Works MCP

- Offer Creator Works MCP after the Unity project passes validation.
- Keep MCP setup optional and independent from SDK correctness.
- Verify the selected project and bridge version after setup.

## Later

- Tested macOS and Linux installation workflows.
- Signed/notarized packages.
- SideQuest-maintained compatibility recipe feed with integrity metadata.
- Safe SDK upgrades with preview, backup, rollback, and migration checks.
- Team presets and optional starter samples.
