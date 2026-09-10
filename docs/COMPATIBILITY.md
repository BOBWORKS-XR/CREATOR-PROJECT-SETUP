# Compatibility Evidence

## Approved Windows Recipe

| Item | Requested or declared | Observed in Unity |
|---|---:|---:|
| Unity Editor | 6000.3.21f1 | 6000.3.21f1 |
| Creator SDK | 4.0.14 | 4.0.14 |
| URP | SDK declares 17.4.0 | 17.3.0 built-in |
| Input System | SDK declares 1.4.4 minimum | 1.20.0 |
| Android support | Required | Available |
| Windows support | Required | Available |

Disposable Windows project `CreatorSetupSmoke-008` compiled and passed the
strict baseline validator on 2026-09-10. The test used the exact project-creation
function behind the application button.

Unity 6000.3.10f1 was also tested and rejected for this recipe: Creator SDK
4.0.14 uses UI Toolkit members that are not available to compilation on that
Editor patch, despite the package metadata declaring 6000.3.10f1. This is why
the application pins a tested recipe instead of following the registry's
`latest` tag or minimum Editor field automatically.

This proves package resolution and Editor compilation on the tested Windows
machine. It does not prove Android/Windows player builds, hosted loading,
headset behavior, macOS/Linux execution, or multiplayer behavior.

## 0.2.0 Windows Preview Checks (2026-09-10)

- `CreatorSetupSmoke-011`: fresh creation through the application's backend,
  separate Unity reopen/validation, then automatic Hub registration. The CLI
  listed the exact project path after adding it, before any interactive opening.
- `CreatorSetupRepair-003`: disposable copy with a missing required module and
  missing node database, plus a custom `System.Text.StringBuilder` type selection.
  Approved repair restored the module and generated 31,904 nodes, including 172
  Creator nodes. All pre-repair type/assembly selections passed the preservation
  check after reopening. The sample scene and user-content sentinel were unchanged;
  the backed-up manifest matched the original bytes.
- A subsequent validation-only operation also passed on that repaired project.
  Both validation logs contained zero C# compilation errors.
- Hub retry passed for an already-registered project. Unity CLI treats a duplicate
  `projects add` as an error, so Setup checks the registry first and verifies the
  final state instead of reporting a false failure.
- 17 Rust unit tests and 15 Playwright tests passed, including stale-review
  rejection, open-project blocking, approval gating, backup integrity, exact-path
  Hub matching, failed Hub retry, and both 980px and 720px UI layouts.
- Rust Clippy passed with warnings treated as errors.
- The real helper download path was tested with the cache absent. The downloaded
  Windows binary matched the pinned size and SHA-256, and Hub verification passed.

These checks cover the tested Windows/Unity recipe, not arbitrary existing
projects. Hub registry readback is distinct from a running older Hub window
refreshing its UI. Native macOS/Linux workflows and full player builds remain
unverified. No user-maintained project was repaired during these tests.
