# Kache: executable-cache candidate, not yet measured

This is a source review and proposed experiment, not a FerroUI performance result
or recommendation to enable Kache in production. No Kache installation, daemon,
`init`, or benchmark was run for this review. The reviewed version is
[v1.2.2](https://github.com/kunobi-ninja/kache/releases/tag/v1.2.2), pinned to
`96052b4842239bb216f6c4812054203c21550788`, released October 15, 2026.

## What it could add

The [existing sccache experiment](build-performance.md#reuse-identical-rust-compilation-through-sccache)
uses stock Cargo and an audited native core dependency graph. sccache's
[Rust limitations](https://github.com/mozilla/sccache#rust) exclude compilation
that invokes the system linker, including executable and proc-macro crate types.
Kache therefore has an additional opportunity: restore eligible linked test
executables and binaries after Cargo's target directory has been emptied.
This is reuse of unchanged compilation, not faster compilation of changed code.

[Kache's compiler policy][configuration] enables eligible Rust executable caching
on Linux/macOS by default. Windows executable caching is opt-in; its implemented
native MSVC path has additional restrictions. [Target handling][rustc] also
supports selected rustc-bundled WebAssembly links, but passes Emscripten and
externally linked/custom WebAssembly targets through. A Linux result would not
establish a gain for FerroUI's browser or Windows builds.

## Correctness boundaries found in the pinned source

| Area | Tracked behavior and remaining boundary | Implication for FerroUI |
| --- | --- | --- |
| Rust compilation | [Cache-key implementation][keys] hashes compiler identity, flags/features, source/include dep-info, extern contents and reported compile-time environment. | Preserve the same toolchain, feature set, profiles and source tree in every comparison. A hit must not substitute a different build configuration. |
| Procedural macros | [`apply_key_env_vars`][keys] explicitly describes an incorrect-hit risk when a macro reads `std::env::var` without a declaration. `cache.key_env_vars` / `KACHE_KEY_ENV_VARS` adds exact values to the key. The [regression fixture][macro-test] tests this case. | Audit macro file/environment reads. Ordinary `env!` tracking does not make arbitrary procedural macros hermetic. Declare additional inputs or bypass affected units. |
| Build-script declarations used by rustc | [`build_script_inputs.rs`][script-inputs] adds a workspace/path package's own `rerun-if-*` declarations to its compiler key. Registry, Git and vendored packages are excluded from this mechanism. | A dependency's declarations do not automatically describe every input a consuming macro reads. Disabling execution replay alone does not solve hidden macro inputs. |
| Native libraries | [`fold_native_link_inputs`][keys] hashes selected archives and linker inputs, but explicitly excludes a library named only in dependency metadata and found only in the linker's default search directories. | Do not treat the entire Skia/desktop/native dependency graph as audited by a scanner-only experiment. Freeze the system image and audit those inputs before expanding scope. |
| Linker/tool identity | [`get_linker_identity`][keys] uses the selected linker's first `--version` output line. [Native-link handling][native-links] additionally models CRT/libc, platform SDK inputs and restrictive MSVC cases. | This is not a complete hash of an arbitrary linker wrapper and every external input it consults. Keep the actual linker, SDK and image fixed; avoid custom wrappers in the first experiment. |

### Keep build-script execution caching disabled

Kache's [build-script execution cache][scripts] can restore generated files and
replay Cargo directives, keyed by the script binary, selected Cargo environment,
and declared file/environment dependencies. Its documented limits matter here:
undeclared `pkg-config`, system-library and tool inputs can leave stale results
even after `cargo clean`. In the default non-hermetic mode, a value derived from
`OUT_DIR` or another machine path, such as a hash or length, can replay incorrectly
after relocation when the original path does not appear literally in the output.
Unrecorded external side effects are not recreated.

The opt-in hermetic mode uses a shared, read-only output directory and has Unix,
non-root and input/output restrictions; it does not discover all undeclared
inputs. It is a separate experiment. Use `KACHE_BUILD_SCRIPT_CACHE=0` initially:
the [launcher implementation][script-code] then executes the original build
script. Caching the compiled build-script executable is a different operation
from skipping execution of that script.

## Proposed isolated experiment

Start with native Linux and the real scanner test executable:

```sh
cargo test --locked --offline -p ferroui-build-scan --lib --no-run --message-format=json
```

Use an owned, frozen checkout under an otherwise empty temporary parent; pin
Rust/Cargo 1.99.1 and the system image. Compare bare Cargo, the existing sccache
0.19.0 policy, and Kache 1.2.2. Give each tool its own cache. Keep dev/test debug
settings identical, use `CARGO_INCREMENTAL=0` in every arm, and clear only the
experiment's target/build directories between samples. Preserve the same absolute
checkout and output paths. Fetch dependencies and install tools outside timers.
Measure population cost separately, then alternate at least three empty-target,
populated-cache samples. The existing [cache harness](../scripts/build-perf/cache.py)
is a useful basis, but it does not currently implement this Kache experiment.

The official pinned installation command, with an isolated destination, is:

```sh
CARGO_HOME="$kache_trial_root/cargo-home" \
  RUSTC_WRAPPER= RUSTC_WORKSPACE_WRAPPER= \
  cargo +1.99.1 install kache --locked --version 1.2.2 \
  --root "$kache_trial_root/tool" --target-dir "$kache_trial_root/install-target"
```

Here `kache_trial_root` must be an absolute, experiment-owned directory. Do not run
`kache init`, install Cargo shims, or edit user Cargo/shell configuration. Put this
configuration in `$kache_trial_root/kache.toml`:

```toml
[cache]
local_only = true
cache_executables = true
scheduler = false
input_predictions = false
deferred_discovery = false
adaptive_incremental = false
preserve_incremental = false
modified_input_guard = true
out_dir_alias = false
seed_new_targets = false
auto_gc = false
index_auto_compact = false
auto_clean_orphaned_targets = false
auto_share_target_files = false
auto_clean_idle_targets_days = 0
auto_clean_unused_units_days = 0
auto_recover_min_free_bytes = 0
auto_recover_targets = false
```

Run all Kache commands in the same sanitized subprocess environment, removing
inherited `KACHE_*`, compiler-wrapper and target-directory overrides first. Set
the following absolute paths and explicit policies there; preserve the pinned
toolchain/system-tool `PATH` and the same required build environment in all arms:

```sh
export CARGO_HOME="$kache_trial_root/cargo-home"
export CARGO_TARGET_DIR="$kache_trial_root/target"
export CARGO_BUILD_BUILD_DIR="$kache_trial_root/build"
export CARGO_INCREMENTAL=0
export KACHE_CONFIG="$kache_trial_root/kache.toml"
export KACHE_HOST_CONFIG=
export KACHE_CACHE_DIR="$kache_trial_root/cache"
export KACHE_RUNTIME_DIR="$kache_trial_root/runtime"
export KACHE_SOCKET_PATH="$kache_trial_root/runtime/daemon.sock"
export KACHE_BUILD_SCRIPT_CACHE=0
export KACHE_RUSTC_PATH_NORMALIZE=0
export KACHE_FALLBACK=off
export KACHE_VERIFY_RESTORES=always
export RUSTC_WORKSPACE_WRAPPER=
export RUSTC_WRAPPER="$kache_trial_root/tool/bin/kache"
```

These settings deliberately exclude target seeding, input predictions, path
rewriting and adaptive incremental behavior from the first measurement. They
also disable remote storage and automatic target/cache maintenance. Kache can
start a daemon through its wrapper, so a separate runtime/socket is required;
stop only this instance afterward using the same environment and
`"$kache_trial_root/tool/bin/kache" daemon stop`. Configuration precedence and
maintenance controls are documented in the [pinned configuration reference][configuration].

### Acceptance gates

Capture Cargo JSON and
`"$kache_trial_root/tool/bin/kache" stats --full --last-build --root "$PWD" --json`
after each sample; retain `kache explain --json` output for unexpected misses.
The [command reference][commands] defines these reports. Require evidence that
the linked test executable itself hit the cache, not merely its libraries or
Cargo's freshness check. Execute every restored test binary outside the compile
timer, compare artifact inventories, and retain all individual timings.

Run a separate untimed validation with `KACHE_VERIFY=1`, plus source, include,
environment, build-script-input and intentional-compile-error mutations. Extend
the existing [mutation probe](../scripts/build-perf/cache.py) rather than treating
one successful replay as adequate coverage. **Kache verification does not fail
the build on an artifact mismatch:** it leaves the restored files in place and
records `verify_compare` as `path-debug` or `content`. The harness must explicitly
reject content differences and investigate path differences. Restore hashing
(`KACHE_VERIFY_RESTORES`) only checks stored bytes; it is not this recompilation
check. See the [verification contract][configuration].

Expand to native linking only after archive/linker/environment mutation checks
pass for that graph. Measure cross-worktree normalization, concurrent same-key
builds and copy-on-write storage as separate experiments; none establishes a
single-build speedup by itself. No numeric Kache benefit is claimed here, and no
Cargo fork is needed to perform this experiment.

[configuration]: https://github.com/kunobi-ninja/kache/blob/96052b4842239bb216f6c4812054203c21550788/docs/getting-started/configuration.mdx
[keys]: https://github.com/kunobi-ninja/kache/blob/96052b4842239bb216f6c4812054203c21550788/src/cache_key.rs
[rustc]: https://github.com/kunobi-ninja/kache/blob/96052b4842239bb216f6c4812054203c21550788/src/compiler/rustc.rs
[native-links]: https://github.com/kunobi-ninja/kache/blob/96052b4842239bb216f6c4812054203c21550788/src/native_link_key.rs
[script-inputs]: https://github.com/kunobi-ninja/kache/blob/96052b4842239bb216f6c4812054203c21550788/src/build_script_inputs.rs
[scripts]: https://github.com/kunobi-ninja/kache/blob/96052b4842239bb216f6c4812054203c21550788/docs/how-it-works/build-scripts.mdx
[script-code]: https://github.com/kunobi-ninja/kache/blob/96052b4842239bb216f6c4812054203c21550788/src/build_script.rs
[macro-test]: https://github.com/kunobi-ninja/kache/blob/96052b4842239bb216f6c4812054203c21550788/tests/proc_macro_env_key_test.rs
[commands]: https://github.com/kunobi-ninja/kache/blob/96052b4842239bb216f6c4812054203c21550788/docs/commands/reference.mdx
