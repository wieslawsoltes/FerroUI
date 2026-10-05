# Cloud environment probe results

Measured on 2026-10-05 in a Claude Code cloud session, checkout of `main` at
`48507636f39523f8fd7dc74f7690ab1f1f9aff9f`. No source code was changed.

| Area | Item | Result |
|---|---|---|
| Environment | OS | Ubuntu 24.04.4 LTS, kernel 6.18.44 |
| Environment | Architecture / cores / RAM | x86_64, 4 cores, 15 GiB |
| Environment | Free disk | 30 GB available (per-session allowance) |
| Environment | sudo | Yes (running as root, `sudo -n` succeeds) |
| Environment | git / gh | 2.43.0 / 2.89.0 (gh CLI installed; GitHub access goes through the MCP integration) |
| Environment | rustc / cargo / rustup | 1.97.0 / 1.97.0 / 1.28.2 (already installed, stable) |
| Environment | clang / python3 | Ubuntu clang 18.1.3 / 3.11.15 |
| Environment | node / npm | v22.22.0 / 10.9.4 |
| Environment | Chromium | Chromium 141.0.7390.37 (Playwright build at `/opt/pw-browsers/chromium-1194`); not on PATH; no Google Chrome |
| Repository | Path / branch / HEAD | `/home/user/FerroUI`, `main`, `48507636f395` |
| Build a | `cargo check -p ferroui-base` | Pass, 50 s (cold) |
| Build b | `cargo check` controls, markup, markup-xaml, loader, xamlx | Pass, 36 s |
| Build c | `cargo test -p ferroui-base --lib` | Pass, 183 s: 3754 passed, 0 failed, 0 ignored |
| Build d | `cargo test -p ferroui-markup-xaml-tests` | Pass, 212 s: 542 passed, 0 failed, 11 ignored (plus doc-test target: 1 ignored) |
| Build e | `cargo check --workspace --all-targets` | Fail (exit 101), 101 s. Only `ferroui-themes-fluent` fails (lib and lib test): `error[E0583]: file not found for module accents` at `src/FerroUI.Themes.Fluent/lib.rs:19`. The directory is tracked as `Accents/` (capital A); this resolves only on a case-insensitive filesystem such as default macOS. Crates depending on it (`control-catalog`, `control-catalog-desktop`, `ferroui-themes-simple` tests) are blocked. All other workspace crates, including `ferroui-native` and `ferroui-skia`, check cleanly. |
| Build e | Skia | `skia-bindings` 0.153.3 downloaded prebuilt binaries from github.com/rust-skia/skia-binaries (`x86_64-unknown-linux-gnu-jpegd-jpege-pdf`, i.e. without Graphite/Metal). No clone from chromium.googlesource.com occurred. |
| Network | crates.io | Reachable (API 200, index.crates.io 200, static.crates.io 200) |
| Network | github.com | Reachable (repo page 200, api.github.com 200) |
| Network | npm registry | Reachable (registry.npmjs.org 200) |
| Network | `git ls-remote` emsdk | Succeeds (refs listed) |
| Network | chromium.googlesource.com | 503 through the proxy (a Skia source build would not work) |
