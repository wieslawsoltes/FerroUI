# Compiler cache (sccache)

[sccache](https://github.com/mozilla/sccache) wraps rustc and stores the output of each
compilation under a hash of its inputs. A later compilation with the same inputs is copied from
the cache instead of compiled. FerroUI builds with stock Cargo and rustc; sccache is optional and
changes nothing in the produced code. This page is the practical setup; the measurements behind
it are in [Build performance](build-performance.md#reuse-identical-rust-compilation-through-sccache).

## When it helps

- Rebuilding after `cargo clean`, or in a new or emptied target directory.
- Switching between branches that share most of their dependencies.
- CI jobs that restore the cache directory between runs.

It does not help the edit-compile loop of one crate: there, rustc's incremental compilation
reuses more (see "Incremental compilation" below). It does not speed up the first build either,
which fills the cache.

## Install

| Platform | Command |
|---|---|
| macOS | `brew install sccache` |
| Windows | `winget install Mozilla.sccache` or `scoop install sccache` |
| Linux | the package of the distribution, or a release binary from the [releases page](https://github.com/mozilla/sccache/releases) |
| Any | `cargo install sccache --locked` (compiles it, which takes a few minutes) |

Check with `sccache --version`. The measurements of the repository use sccache 0.19.0.

## Enable it

Choose one of these.

**For one command**, with the configuration in the repository:

```sh
cargo --config .cargo/compiler-cache.toml build --locked -p control-catalog-desktop
```

`.cargo/compiler-cache.toml` sets sccache as the compiler wrapper and turns incremental
compilation off for the `dev` and `release` profiles. Cargo reads it only when it is named with
`--config`.

**For a shell session**, with environment variables:

```sh
export RUSTC_WRAPPER=sccache
export CARGO_INCREMENTAL=0
```

On Windows PowerShell: `$env:RUSTC_WRAPPER = "sccache"; $env:CARGO_INCREMENTAL = "0"`.

**For every Cargo project of your account**, in `~/.cargo/config.toml` (on Windows
`%USERPROFILE%\.cargo\config.toml`):

```toml
[build]
rustc-wrapper = "sccache"
```

That applies to all your Rust projects; leave incremental compilation as it is there and turn it
off per build where the cache matters (`CARGO_INCREMENTAL=0`).

## Where the cache lives, and its size

The cache is a local directory: by default `~/Library/Caches/Mozilla.sccache` on macOS,
`~/.cache/sccache` on Linux and `%LOCALAPPDATA%\Mozilla\sccache\cache` on Windows, limited to
10 GB. Builds of several FerroUI applications, profiles or branches can outgrow that, so set a
larger limit, and move the cache to a disk with room if needed:

```sh
export SCCACHE_DIR=/path/with/space/sccache
export SCCACHE_CACHE_SIZE=30G
```

sccache runs a small background server that reads these variables when it starts. After changing
them, restart it with `sccache --stop-server`; the next build starts it again.

## Check that it works

```sh
sccache --zero-stats
cargo build --locked -p control-catalog-desktop   # with the wrapper enabled
sccache --show-stats
```

`Cache hits` and `Cache misses` count the compilations of the last builds; `Non-cacheable calls`
counts the compilations sccache passed through to rustc unchanged.

## What is and is not cached

- **Cached:** the compilation of library crates, including the dependencies from crates.io.
- **Not cached:** linking an executable, test binaries, proc-macro crates and other dynamic
  libraries, and incremental compilations. Build scripts still run: the generated markup of the
  themes and the catalog is produced again, and the crates that include it are cached by their
  content.
- **Paths are part of the key.** A crate compiled from another directory has a different key, so
  two worktrees of the repository share the cached crates.io dependencies, but not the workspace
  crates, unless they are built from the same path.

## Incremental compilation

sccache cannot cache an incremental compilation, and Cargo compiles the crates of the workspace
incrementally in the `dev` profile by default; with the wrapper and incremental compilation on,
the workspace crates are passed through to rustc and only the dependencies come from the cache.
Turn incremental compilation off (`CARGO_INCREMENTAL=0`, or the repository configuration above)
when the cache should cover the workspace crates too: rebuilds from scratch and CI. Keep it on,
without sccache, for a long edit-compile session on a few crates.

## Profiles

The cache works for every profile, and each profile has its own entries. To try an application
with optimised code, `--profile fast-release` builds without link-time optimisation, so the
cached crates are linked directly instead of being optimised again as one program; `release`
and `dist` still run their link-time optimisation on every build of an executable (see
[Binary size](binary-size.md#6-the-settings-applications-should-use)).

## CI

The [build-performance workflow](../.github/workflows/build-performance.yml) installs sccache
with `Mozilla-Actions/sccache-action` and measures cache population and replay. The regular CI
workflow caches the target directory with `Swatinem/rust-cache` instead and does not use
sccache.

## Troubleshooting

- **No hits after a rebuild:** check that incremental compilation is off and that the path of
  the checkout is the same as before.
- **`sccache: error: Timed out waiting for server startup`:** run `sccache --stop-server` and
  build again; a server started with another `SCCACHE_DIR` may still be running.
- **Turning it off for one command:** `RUSTC_WRAPPER= cargo build ...` (an empty value), or omit
  the `--config` file.
