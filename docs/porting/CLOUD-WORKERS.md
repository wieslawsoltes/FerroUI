# Brief for cloud workers

This page is the standing brief for a worker that picks up a porting task in a fresh Linux checkout and delivers it as a pull request. The task itself comes with the request that started the worker; everything here applies to every task.

## What the project is

FerroUI is a Rust port of the Avalonia UI framework. The port is exact: the same architecture, contracts, members and behaviour, file by file, with the upstream unit tests ported alongside. It is not a reinterpretation. Where upstream has a quirk, port the quirk unless a design document in `docs/porting/` records a decision to do otherwise.

Read before writing code:

- `docs/porting/PORTING-GUIDE.md`: the class model macros, the naming and mapping rules, and the conventions every ported file follows.
- `docs/porting/CRITICAL-PATH.md`: subsystem status and known gaps.
- `docs/porting/CONTINUATION.md`: the state at the last hand-over, the tasks that come next and the decisions waiting for the owner.
- The design document for your area, when there is one: `xaml.md` (markup pipeline and the ahead-of-time compiler), `browser-platform.md` (browser backend).

## Setup

- Reference sources, read-only: `git clone https://github.com/AvaloniaUI/Avalonia /home/user/upstream`. `docs/porting/TRACKING.md` names the commit the port tracks; check it out when a file differs from what the port was written against.
- The machine is Linux. The desktop platform backend and the Skia Graphite/Metal path are macOS only and are verified by the macOS job in CI, not here.
- Never change the Skia feature set of a target. If a build starts cloning from `chromium.googlesource.com`, it has fallen back to building Skia from source: stop it and report.
- Browser work: `scripts/build-browser.sh` and the files under `scripts/browser/` install and use the pinned toolchain (Emscripten, the `wasm-bindgen` tool, headless Chromium). Chromium is at `/opt/pw-browsers/chromium-*/chrome-linux/chrome` in the cloud image.

## Verification that runs on Linux

Run what your change can affect, and always the first two:

```sh
cargo check --workspace --all-targets
grep -rIli -E "avalonia|axaml|avares|\bavn" src external tests samples native Cargo.toml --exclude=NOTICE.md   # must print nothing
cargo test -p ferroui-base --lib
cargo test -p ferroui-controls --lib
cargo test -p ferroui-markup -p ferroui-markup-xaml -p ferroui-markup-xaml-loader -p xamlx
cargo test -p ferroui-markup-xaml-tests
cargo test -p ferroui-themes-fluent -p ferroui-themes-simple
cargo test -p control-catalog -p mini-mvvm
python3 scripts/generate_markup_types.py --upstream /home/user/upstream --check
```

The macOS job of the CI workflow is the authoritative full run. After opening the pull request, wait for its checks and fix the failures your change caused.

## Rules

- The upstream project name, and `Avn`, `avares` and `axaml`, must not appear in code, comments, strings or file names under `src/`, `samples/`, `tests/`, `external/` or `native/`. CI enforces this. The name is allowed in `docs/`, `scripts/` and `NOTICE.md`. Rename as the porting guide says.
- No `todo!`, no `unimplemented!`, no stubs and no placeholder behaviour. When a part depends on something the port does not have yet, port everything that does not depend on it and leave a precisely worded seam comment that names the missing upstream API. List every seam in the pull request.
- No new `unsafe` outside the platform, rendering and interop boundaries. List any you add, with its safety argument.
- Keep the licence header of an upstream file that has one, and list the file in the `NOTICE.md` of its crate.
- New classes are registered in the crate's `register_types.rs`. A class whose upstream namespace differs from the default of its module goes under the matching `// FerroUI.…` section comment. Regenerate the markup metadata with `scripts/generate_markup_types.py` and commit the result.
- Tests keep upstream's names in snake case and upstream's assertions. A test that is not from upstream says so in its header comment.
- Before delivering, re-read every new or changed file line by line against its upstream counterpart and fix deviations. A deliberate deviation is listed in the pull request with its reason, recorded in `docs/porting/DEVIATIONS.md` (or in the area's own page that it names), and marked with a comment at its site in the code. A change that removes a divergence (for example an allocation or a copy that upstream does not make) moves or adds its row under "Corrected divergences".
- Documentation, comments and commit messages are plain professional prose. No emojis. No attribution or co-author lines in commits.

## Delivery

- Work on the branch named in your task. Never push to `main`.
- Commits are small and each one builds.
- Every commit is authored and committed as the owner. Set it in the repository, not globally (the global configuration of a cloud container is reset): `git config user.name "Wiesław Šoltés"` and `git config user.email "wieslawsoltes@users.noreply.github.com"` before the first commit. Check with `git log --format='%an <%ae> | %cn <%ce>' origin/main..` before pushing.
- Branches stay linear: rebase onto `main`, never merge `main` into a branch. Pull requests are merged with rebase merges, which GitHub refuses for a branch with merge commits.
- Open one pull request against `main`. Its description states:
  - what was ported: files, and tests added per item;
  - the exact result line of every test suite you ran, and the result of the workspace check, the naming check and the generator check;
  - deliberate deviations, remaining seams and new `unsafe`;
  - what you could not verify on Linux.
- Report measured facts. Say plainly when something was not run.
