# Repair Preview and Recovery

Inspection is read-only. Repair and validation both launch Unity and can trigger
package imports, compilation, and package-owned Editor callbacks. The application
does not guarantee that third-party callbacks are read-only.

## Before Approving

- Close the selected project in Unity.
- Keep a full copy or version-control checkpoint for important content.
- Read the proposed changes. Untested versions and pipeline conversions are not
  silently accepted by this preview.

## Retained Evidence

Each approved operation creates a unique folder in
`.creator-project-setup/backups/` containing the reviewed plan, original package
manifests, project settings, generated Visual Scripting data where present, and
Unity logs/result. This is a selective settings backup, not an entire project copy.
An unsuccessful operation does not automatically roll back.

If Unity reports errors, keep the backup and logs. With Unity closed, compare the
backup with the current files before restoring anything; do not overwrite later
manual changes. Original files can be recovered from the matching relative paths
inside the operation's backup. Files newly introduced during repair have no
original counterpart, so blindly copying a backup over the project is not a full
rollback. Use the reviewed plan and version-control diff to review those additions.

If the application or computer stopped mid-operation, first confirm no Unity
process is still working on the selected project. The operation lock and temporary
validator/request may remain. Do not remove them while a process is active. Retain
the evidence and review the interrupted operation before cleaning up those files
and inspecting again. This preview deliberately refuses to guess whether it is
safe to resume an interrupted repair.

No automated project deletion, Library reset, material conversion, or scene
reconstruction is part of recovery.
