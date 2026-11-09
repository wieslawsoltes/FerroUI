# Building & releasing

## Repository layout

```
├── src/
│   ├── ScreenKeyboard.Core/        engine (net8.0, net10.0) + bundled FlorisBoard data (Resources/florisboard)
│   └── ScreenKeyboard.Uno/         Uno Platform controls (Uno.Sdk)
├── tests/ScreenKeyboard.Core.Tests xUnit tests for the engine, layouts, NLP, gestures
├── samples/ScreenKeyboard.Sample/  demo app (desktop, WebAssembly, Android, iOS, Windows)
├── build/                          package icon, scripts (icon generator)
├── docs/                           documentation
└── .github/workflows/              CI, release and GitHub Pages
```

## Prerequisites

- .NET 10 SDK (see `global.json`); the Uno.Sdk version is pinned in `global.json`.
- For mobile targets: `dotnet workload install android ios` (and Xcode on macOS).
- For WebAssembly publishing: `dotnet workload install wasm-tools`.

## Build and test

```bash
dotnet test tests/ScreenKeyboard.Core.Tests
dotnet build src/ScreenKeyboard.Uno
```

Building every target framework requires the corresponding workloads. Restrict the targets for quick local builds
with the `SkTargetFrameworks` property (supported by the library and the sample):

```bash
dotnet build src/ScreenKeyboard.Uno -p:SkTargetFrameworks=net10.0-desktop
dotnet run --project samples/ScreenKeyboard.Sample -f net10.0-desktop -p:SkTargetFrameworks=net10.0-desktop
dotnet run --project samples/ScreenKeyboard.Sample -f net10.0-browserwasm -p:SkTargetFrameworks=net10.0-browserwasm
```

## Packaging

```bash
dotnet pack src/ScreenKeyboard.Core -c Release -o artifacts -p:Version=1.0.0
dotnet pack src/ScreenKeyboard.Uno -c Release -o artifacts -p:Version=1.0.0
```

Packages include XML documentation, the README, the package icon, `NOTICE`, the FlorisBoard license and symbol
packages (`.snupkg`) with Source Link. `ContinuousIntegrationBuild` is enabled on CI for deterministic builds.

## Continuous integration

| Workflow | Trigger | What it does |
|---|---|---|
| `ci.yml` | push/PR to `main` | Core build + tests with coverage (Linux); Uno library for all targets on Linux, Windows and macOS; desktop sample on all three OSes; WebAssembly sample publish; preview NuGet packages as artifacts. |
| `release.yml` | tag `v*.*.*` or manual | Tests and packs with the tag version, publishes to nuget.org with **Trusted Publishing** (OIDC, no API key secret) and creates a GitHub release with the packages. A manual run with `publish` unchecked is a dry run (draft release, nothing pushed). |
| `pages.yml` | push to `main` | Publishes the WebAssembly sample to GitHub Pages. |

## Publishing to nuget.org (Trusted Publishing)

Releases use [NuGet Trusted Publishing](https://learn.microsoft.com/nuget/nuget-org/trusted-publishing): the
`publish-nuget` job requests a GitHub OIDC token (`permissions: id-token: write`), the
[`NuGet/login`](https://github.com/NuGet/login) action exchanges it for a nuget.org API key that is valid for one
hour, and the packages are pushed with that key. No long-lived API key is stored in the repository.

One-time setup:

1. **nuget.org** → your user name → **Trusted Publishing** → **Add policy**:
   - Repository Owner: `wieslawsoltes`
   - Repository: `ScreenKeyboardControl`
   - Workflow File: `release.yml` (file name only)
   - Environment: `nuget`
   - Owner: your user (or the organization that should own the packages); allow publishing new packages so the
     first release can create the `ScreenKeyboard.Core` and `ScreenKeyboard.Uno` IDs.
2. **GitHub** → repository **Settings** → **Environments** → **New environment** `nuget` (optionally add required
   reviewers and restrict deployments to `v*` tags).
3. Add the secret **`NUGET_USER`** (repository or `nuget` environment secret) containing your nuget.org **profile
   name** — not your e-mail address.

To release: update `CHANGELOG.md`, then

```bash
git tag v1.0.0
git push origin v1.0.0
```

The workflow can also be started manually (Actions → Release → Run workflow) with an explicit version.

## Generated files

| File | Generator |
|---|---|
| `src/ScreenKeyboard.Uno/Icons/IconData.g.cs` | `build/scripts/generate_icons.py <svg-folder> <output>` (Material Symbols, rounded, 24px) |
| Settings tables in `docs/configuration.md` | `python3 build/scripts/generate_settings_docs.py` |

## Updating bundled data

The files under `src/ScreenKeyboard.Core/Resources/florisboard` come from FlorisBoard's `app/src/main/assets/ime`
directory (`keyboard/*` copied verbatim, emoji `*.txt` and `dict/data.json` gzip compressed). Icons are regenerated
with:

```bash
python3 build/scripts/generate_icons.py <folder-with-material-symbols-svgs> src/ScreenKeyboard.Uno/Icons/IconData.g.cs
```
