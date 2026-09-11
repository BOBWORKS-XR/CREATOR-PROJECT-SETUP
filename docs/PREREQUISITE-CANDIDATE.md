# Windows Prerequisite Installation Candidate

Status: unreleased `0.3.0-alpha.3`, branch `feat/prerequisite-setup`.
Published `0.3.0-alpha.2` does not install missing Unity requirements.

## Intended Flow

1. Enter a new project name and parent folder.
2. Choose **Set up and create project** when Windows requirements are missing.
3. Review the native installation confirmation: exact Editor version, locations,
   estimated download and conservative free-space reserve. Explicitly accept
   the linked Unity, Android SDK/NDK and OpenJDK terms to proceed.
4. Setup downloads the pinned official Hub installer and checks its exact SHA-256,
   valid Windows Authenticode signature, signing certificate and product version
   before launch. The verified official Unity CLI installs Editor/modules.
   Windows may request administrator approval.
5. Setup checks actual Editor/tool files and the installed URP template. An
   installer exit code alone is not considered sufficient.
6. If Unity reports no active licence, open Unity Hub from the prompt, complete
   sign-in/activation, and return to the same form. Installed prerequisites are
   retained; no project folder is created before this check passes.
7. Continue through the existing Creator SDK project creation, compilation,
   Visual Scripting initialization and second-session validation.

This is a single setup action, not a promise to bypass account, licence or OS
approvals. macOS/Linux automatic installation is not implemented.

## Boundaries

- The Editor/SDK recipe is unchanged. Android and Windows remain mandatory.
- An existing old Hub is not silently upgraded; automatic project registration
  still requires Hub 3.21.1 or newer. New projects can be added from disk in an
  older Hub.
- A ready, licensed CLI-only installation remains usable without adding Hub.
  If activation is missing and Hub is absent, the candidate offers an approved
  Hub installation before the sign-in handoff. That native CLI-only activation
  route still needs user acceptance testing.
- Missing Android modules can be added to the verified managed Editor. A
  damaged core Editor or unregistered/conflicting installation is blocked with
  instructions instead of being overwritten or duplicated.
- Installation approval is followed by a fresh path/inventory check. Module
  installs also check that the selected Editor is closed. No app is force-closed.
- Cancellation is available before installation. Once an installer has started,
  Setup waits for it; it does not abruptly kill an installer or offer a misleading
  immediate cancel button.
- Download percentages are component-specific. Installation phases remain
  indeterminate because the pinned CLI reports a placeholder installation
  percentage rather than measured extraction progress.
- Preflight storage is a conservative reserve, not an exact disk estimate. It
  cannot guarantee that another program will not consume space during setup.
- Retrying inspects the installation again. A partial/unregistered Editor that
  cannot be verified is not silently removed. Failed project folders are still
  preserved, not recreated over existing content.
- Raw Unity account/licence and CLI environment responses are parsed only in
  memory, not written into support logs. Other logs still require review before
  sharing.

## Validation Gates

The local candidate passes 56 Rust unit tests, 2 executable metadata
integration tests, 43 UI tests and strict Clippy. These are not clean-machine
installation proof.

The healthy-machine creation smoke passed on 2026-09-11: existing requirements
were reused without installation; the licence precheck passed; a new disposable
project compiled, initialized 31,818 Visual Scripting nodes including 172 Creator
nodes, passed second-session validation for Android/Windows and URP, and was
read back from Hub's project registry. This does not prove a fresh-account
activation flow, native Hub list visibility, or actual Android/Windows builds.

Local packaging checks also passed, including original notices, extracted
installed payload identity, portable metadata and an active-app installer guard.
These are not installed-upgrade acceptance of alpha.3.

The owner approved a disposable Windows CI installation of Unity and required
Android tools without using their Unity account. Early runs stopped before
installation: the image included Hub, preview output was not pure JSON, and
the system volume had insufficient free space. The image's signed Hub is now
removed with its own uninstaller; only ordinary files in unused CodeQL/Python
tool caches are cleared. Linked entries and all existing Editor folders are
left untouched. Application storage checks were not relaxed for CI.

Run [34595273184](https://github.com/BOBWORKS-XR/CREATOR-PROJECT-SETUP/actions/runs/34595273184)
captured the exact CLI preview format for a committed regression fixture.
The current installation run is
[34597376140](https://github.com/BOBWORKS-XR/CREATOR-PROJECT-SETUP/actions/runs/34597376140).
It tests cancel-before-install, reuse, isolated OpenJDK repair, execution of
Java/javac/ADB/NDK clang, and detection of missing activation. Its final outcome
must be recorded before release. A failed preflight is not installation proof.

Separate gates remain for real module repair, declined elevation, interrupted
downloads and retry, fresh-machine sign-in/activation, and Creator project
creation after a fresh install. Do not label these verified from unit tests.

Creator Hub currently pins the older Setup executable and drops the new event.
An isolated Hub patch (`a02cf14817b23546958516996a6d09bea0518301`) adds native and
renderer `requirements-progress` allowlists and independent bounded event
throttles, with forwarding/session/transition tests. It is not published or
pinned to this executable. Do not update that pin before candidate acceptance.

## Source Contracts

- [Unity CLI reference](https://docs.unity.com/en-us/unity-cli/unity-cli-reference)
- [Unity's installation command guidance](https://github.com/Unity-Technologies/skills/blob/main/skills/unity-cli/references/editors-install.md)

The pinned CLI is experimental. Its actual `--help` and read-only previews were
checked: JSON dry-run is required; the NDJSON dry-run produced no final report.
Even `--quiet --json` previews prepend dependency notices on stdout. The adapter
permits only the exact known notice syntax before one complete JSON document;
unknown warnings/errors and trailing data remain rejected.
The genuine Hub 3.21.1 Windows download was independently verified locally.
Beta.9's `hub install` rejected it during CI with a signature error naming a
macOS Developer ID; Setup therefore uses its own Windows trust verification
and official installer launch. No signature-check bypass flag is used.
OpenJDK uses a release-specific module ID, and repair dry-runs must include
`--reinstall` to describe the same repair set as the approved install.
