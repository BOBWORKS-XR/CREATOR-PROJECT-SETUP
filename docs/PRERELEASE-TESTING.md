# Coordinated Windows Prerelease Testing

Use the prerelease assets from the three repositories, not old stable installers
or a random local development build. Each release must identify its tested source,
installer hashes, checksums and remaining limits. The suite is not yet a stable release.

## Test Safely

- Use a disposable Windows account or a backed-up test machine for installer testing.
- Close Setup before updating it. Finish MCP work and disconnect clients using its
  private runtime before an MCP update; do not close Unity with unsaved work.
- Use a new empty folder for a throwaway Unity project. Do not use Forest, Blamb or
  valuable existing content to test a new repair or SDK migration path first.
- Installing updates requires approval, even when prereleases and verified downloads
  are enabled by default. Saved opt-outs must remain off.
- Download checksums and signed Hub catalog metadata do not make an unsigned
  Windows executable Authenticode-signed. Expect the documented publisher warning.

## Main Test Route

1. Install and open the Creator Hub prerelease. Check that it opens without errors.
2. Check whether Hub detects existing Creator apps, and compare displayed versions
   with their app/release versions. Do not approve an unexpected replacement path.
3. Use the supported install/update actions for MCP and Project Setup. If Hub says
   an action is unavailable, capture the reason; do not bypass verification or invent
   registry entries. Hosting/adoption and self-update have separate acceptance gates.
4. Open Project Setup. Confirm required Android and Windows modules are detected.
   Create a disposable Creator SDK project and wait for validation to complete.
5. Open that project in Unity and check for compile errors or a Visual Scripting
   initialization prompt. Inspect its `.creator-project-setup` logs and receipt.
6. Connect Creator Works MCP to this project through the supported setup flow.
   Ask for a small scene read, then create and move one clearly named test object.
7. Save, close and reopen the apps/project. Check the saved test object, app detection,
   project list, and explicit update choices. Record failures rather than assuming
   a launcher success message proves the full task worked.

## Additional Routes

### Stable and Prerelease Channels

These are separate routes, not permission to install any older build:

| Route | Expected behavior | Evidence required |
| --- | --- | --- |
| Stable to newer prerelease | Offered only with prereleases enabled; approval still required | Released installer upgrade, preservation checks and app startup |
| Prerelease to newer stable | Offered even with prereleases disabled | Channel/version selection and installed upgrade checks |
| Prerelease to an older stable | No automatic downgrade | A separately reviewed, explicit rollback with compatible settings/backups |

For example, `0.2.2 -> 0.3.0-alpha.1 -> 0.3.0` is forward progression.
Disabling prereleases must not silently turn `0.3.0-alpha.1` into `0.2.2`.
The Windows fixture covers the first route only. Do not imply a real alpha-to-stable
installer test has happened before that stable candidate exists. This preview does
not advertise a general-purpose rollback feature.

### Installations and Recovery

- Start with MCP already installed, then add Hub; verify settings and client
  connections survive any supported update/adoption action.
- Try updating Setup while it is open. Expect a clear refusal, not a force-close
  or a partially replaced installation. Close it normally, then retry.
- Test cancellation and offline/update-download failure without losing the current
  app. Never select Ignore on an installer file-write error; capture the error.
- Run the standalone Setup EXE without Hub and verify its core workflow still works.

## Report a Result

Include Windows version, all three app versions, original install locations,
clean-install or upgrade route, exact steps, expected versus actual result, and
the relevant error/log. Remove credentials and private project data before sharing.
For project setup include the validation receipt and the smallest useful Unity log
excerpt. A screenshot of the final message alone is not proof of successful setup.

## What Automated Tests Do Not Establish

Installed tests use synthetic preservation markers, not real user preferences or
valuable Unity projects. Browser mocks are not native Hub hosting tests. No claim
is made here of completed Windows/Android player builds, published spaces, working
multiplayer, acceptable headset performance, or native macOS/Linux validation.
