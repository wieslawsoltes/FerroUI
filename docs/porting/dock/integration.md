# Integrating the Dock port

The Dock port was developed as a standalone workspace against the published 0.3 packages of
FerroUI, with seven of those packages vendored and changed. It is imported here as it was, in the
layout of this repository:

| Standalone | Here |
|---|---|
| `src/FerroDock.*` | `src/Dock/FerroDock.*` |
| `tests/FerroDock.*`, `tests/data`, `tests/support` | `tests/Dock/...` |
| `samples/FerroDockReactiveUISample` | `samples/Dock/FerroDockReactiveUISample` |
| `scripts/*` | `scripts/dock/*` |
| `docs/*`, `README.md` | `docs/porting/dock/*` |
| `staging/` (notices) | `docs/porting/dock/staging/` |
| `vendor/*.patch`, provenance and notes | `docs/porting/dock/core-patches/` |
| `Cargo.toml` (the standalone workspace) | `docs/porting/dock/standalone-workspace.toml` |
| `NOTICE.md` | `src/Dock/NOTICE.md` |

Relative paths in the sources follow the new layout. The reference programs and the scripts that
read the original C# sources expect a checkout of the original repository beside this one
(`../Dock`). The vendored package copies are not imported.

## Workspace membership

The Dock crates are members of the workspace (step 2). They take the version of the workspace and
the ferroui crates of the workspace, keep their own lints in their manifests (unsafe code
forbidden, missing documentation denied), and are not published (`publish = false`): releases are
on hold, and publishing the Dock crates with FerroUI is a separate decision. Until it is made,
`scripts/release/publish.sh`, `crates.py check` and the crates table of the README leave them out.

## Remaining steps

1. Make the changes of `core-patches/` in the crates of this repository (ferroui-base,
   ferroui-controls, ferroui-native, ferroui-markup-xaml, ferroui-markup-xaml-loader,
   ferroui-build, ferroui-build-scan), against main rather than 0.3.0, each with its tests.
2. Move the dependencies of `standalone-workspace.toml` into the workspace manifest (path
   dependencies on the crates here, at the version of the workspace), make the Dock crates
   members and remove the `exclude` entries.
3. Bring the Dock crates to the current API of main and run their tests on macOS, then the sample.
4. Add the Dock CI workflow (path-filtered, like the other areas).
