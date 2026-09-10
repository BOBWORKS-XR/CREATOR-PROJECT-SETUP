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
