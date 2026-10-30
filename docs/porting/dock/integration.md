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

## Integration steps

The integration is a stack of three pull requests: the core changes (#365), the import with the
workspace membership and the API of main (#354), and the CI workflow.

1. Done (#365): the changes of `core-patches/` are made in the crates of this repository
   (ferroui-base, ferroui-controls, ferroui-native, ferroui-markup-xaml,
   ferroui-markup-xaml-loader, ferroui-build, ferroui-build-scan), against main, with their tests.
   For 0.4, which allows additions only, they are additive: provided trait methods
   (`IResourceDictionary::merged_dictionaries`, `IResourceProvider::as_any` instead of an `Any`
   supertrait), the value registration of name scopes with its own `NameScopeValueError`, and
   the fields of the metadata structs `MarkupType` and `TypeModel`, which the API gate does not
   count. `scripts/dock/check_framework_patch.py` verified the vendored copies of the standalone
   workspace against these patches; the copies are gone, and the patches stay as the record of
   the changes.
2. Done (#354): the dependencies of `standalone-workspace.toml` are in the workspace manifest and
   the Dock crates are members (see "Workspace membership").
3. Done (#354): the Dock crates are on the API of main. On macOS their 1967 tests pass and the
   sample runs. What changed: providers are downcast through `IResourceProvider::as_any`, the
   value registrations report `NameScopeValueError`, event-add metadata returns the subscription
   (`MarkupEvent::add`), the generated serializer tests read the ReactiveUI command contract from
   the workspace, and the inventory scripts audit `src/Dock` and `samples/Dock` only.
4. Done: `.github/workflows/dock.yml`, for a pull request that changes the Dock sources, tests,
   sample, scripts or records. On Linux it runs the inventory of the port against the original
   at the recorded revision (`check_port.py`, `check_inspector_views.py`, `check_theme_links.py`
   and the tests of the scripts) and builds and tests every Dock crate.
