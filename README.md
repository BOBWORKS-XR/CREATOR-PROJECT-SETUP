# Creator Project Setup

Creator Project Setup is a portable desktop wizard for creating a known-good
SideQuest Creator SDK project without asking a new Unity user to edit package
manifests, drag `.unitypackage` files, install Git, or use a terminal.

The first release is Windows-first, with shared macOS and Linux detection and
packaging support. Windows is the first physically tested platform; macOS and
Linux packages remain preview quality until their full Unity workflows have
been exercised on real machines.

## Current Recipe

- Unity Editor `6000.3.21f1`
- SideQuest Creator SDK `4.0.14`
- Universal Render Pipeline `17.3.0` as resolved by the approved Editor
- Input System `1.20.0`
- Android Build Support with SDK, NDK, and OpenJDK, always required
- Windows standalone build support, always required

The recipe is deliberately pinned, including transitive packages that have
already caused compatibility failures, and updated only after a real compile and
validation run. Creator SDK `4.0.14` declares URP `17.4.0`, but the approved
Unity 6000.3 Editor resolves its built-in URP `17.3.0`; the application records
both the declared and observed values instead of hiding that discrepancy.

## First Release Scope

- Detect Unity Hub, Unity CLI, compatible Editors, and required build modules.
- Create a new project from the matching Editor's official 3D URP template.
- Add the official Greenfield scoped registry and pinned Creator SDK package.
- Resolve packages in batch mode with Android active and produce a local
  setup/validation receipt that confirms both Android and Windows support.
- Open Unity Hub for missing installations and open a completed project.
- Never overwrite an existing project directory.

Unity account sign-in, license activation, administrator prompts, and module
license agreements remain explicit Unity/operating-system handoffs.

## Development

```powershell
npm install
npm run check
npm run dev
```

Build a native package with `npm run build`. Tagged builds are packaged for
Windows, macOS, and Linux by GitHub Actions.

## Roadmap

See [docs/PLAN.md](docs/PLAN.md). Existing-project repair and optional Creator
Works MCP setup follow the proven new-project workflow; neither is hidden inside
the initial creation path.

## License

MIT. Unity and SideQuest products, trademarks, packages, and installers remain
the property of their respective owners. The application downloads or invokes
official sources and does not redistribute the Unity Editor.
