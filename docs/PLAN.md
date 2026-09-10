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
  Unity CLI helper. Verify registration and allow independent retry.
- Do not edit Hub's private database or silently link Unity Cloud.

Still required before promoting repair beyond preview: broader existing-project
fixtures, recovery UX, third-party package callback coverage, and native
macOS/Linux acceptance. Backups are selective; failure recovery is manual.

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
