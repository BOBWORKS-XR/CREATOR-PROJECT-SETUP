# Creator Project Setup 0.3.0-alpha.2

Windows diagnostics hotfix prerelease. Stable 0.2.2 and the earlier prerelease
remain unchanged. This release keeps the same Unity/Creator SDK recipe.

## What Changed

- A recognized final Unity package-download failure now shows the failed package,
  host, connection error, exit code and connection-check guidance, instead of only
  "Unity setup failed". Earlier recovered warnings are not treated as the cause.
- Creation receipts now record the Setup version, timestamp, platform and paths
  of existing Unity logs, so optional user reports can be tied to the right build.
- Failed creation explicitly explains that the retained folder cannot be resumed
  or overwritten by Create. A fresh attempt needs a different project name.
- [Reporting instructions](https://github.com/BOBWORKS-XR/CREATOR-PROJECT-SETUP/blob/hub-compatibility/docs/REPORTING.md)
  explain which files to keep and what to redact before sharing.

Logs remain local. No telemetry, automatic upload, automatic retry, security-policy
changes or project deletion was added. This improves diagnosis; it does not claim
to fix a user's network or Unity's package server.

## Install

Use the Windows installer or the portable ZIP from this release. Keep `licenses/`
with the portable EXE. Close Setup normally before upgrading. Both forms work
standalone; Creator Hub is optional. Windows binaries are not Authenticode signed.

Hub's hosted view requires exact-build acceptance. Do not assume an older Hub can
host this newly built version. Standalone use remains available while the matching
Hub compatibility update is prepared. No MCP update is required for this hotfix.

## Validation

Final clean-build and installed-upgrade evidence will accompany the accepted
release assets. Unit/browser tests alone do not establish a successful Unity
download or project completion on the reporting user's machine. Native macOS and
Linux workflows, full project recovery and headset behavior are not validated by
this Windows diagnostics release.
