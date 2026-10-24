# port-status

Generates the port tracking documents. The port tracks more than one upstream reference; each is listed in
`scripts/port-status/references.toml` with its own name, checkout, tracked commit (the `upstreamCommit` of its
extraction), project list and data files:

| Reference | Checkout | Project list | Data | Pages |
|---|---|---|---|---|
| `avalonia` (default) | `../Avalonia`, or `$UPSTREAM` | `scripts/api-extract/projects.json` | `docs/porting/data/` | `docs/porting/TRACKING.md`, `REMAINING.md`, `TESTS.md`, `docs/porting/tracking/` |
| `reactive` (Rx.NET) | `/Volumes/1TB-macOS/reactive-ref/reactive`, or `$RX_UPSTREAM` | `scripts/api-extract/projects-reactive.json` | `docs/porting/data/reactive/` | `docs/porting/tracking/reactive/` |

The framework reference (`documents = "framework"`) writes the documents below. Another reference
(`documents = "library"`) writes a page per project and an index, `<tracking>/TRACKING.md` (totals, projects, tests,
waivers by reason), to its tracking directory, and its `port-status.json` to its data directory; section "Library
references" below has its rules.

The documents of the framework reference:

- `docs/porting/TRACKING.md` - master document (the two headline numbers, totals, per-project table, project structure)
- `docs/porting/REMAINING.md` - what is left to port, in one place: every upstream project with its scope and what is
  present, missing and waived, the upstream test projects, the upstream samples, and the missing files, types and
  members by name
- `docs/porting/tracking/<Project>.md` - per project: contracts, files by directory, missing types and members
- `docs/porting/data/port-status.json` - the same totals, machine readable

```sh
scripts/port-status/run.sh            # every reference; the framework's checkout at ../Avalonia, or UPSTREAM=/path/to/Avalonia
scripts/port-status/run.sh --reference avalonia   # one reference (`reactive` for Rx.NET)
scripts/port-status/run.sh --force    # re-run the upstream extractor
scripts/port-status/run.sh --check    # exit 1 when the generated documents of any reference are out of date; runs no extraction
TRACKED_COMMIT=<sha> scripts/port-status/run.sh   # move the tracking of the default reference (or of --reference) to another commit
python3 scripts/port-status/port_status.py --explain Avalonia.Base Point.cs   # how each member was matched
python3 scripts/port-status/port_status.py --reference reactive --explain System.Reactive Notification.cs
```

Requirements: Python 3.11+ (standard library only); the .NET SDK only when the upstream JSON has to be regenerated.

The tracking documents count members. The upstream unit tests that the port does not have are counted by a
script of its own, which reads the upstream test projects at the tracked commit and the test functions of the port:

```sh
python3 scripts/port-status/test_gaps.py ../Avalonia --all                           # every project: docs/porting/data/test-gaps-*.txt and the table
python3 scripts/port-status/test_gaps.py ../Avalonia tests/Avalonia.Base.UnitTests   # the report of one project
python3 scripts/port-status/test_gaps.py --reference reactive --all                  # Rx.NET: docs/porting/data/reactive/test-gaps-system-reactive.txt
```

Its rules are at its top; `docs/porting/data/test-aliases.toml` holds the tests the port has under another name
and the ones that are not ported for a stated reason, with a section per upstream test project.

`tests_audit.py` measures every upstream test project at once, the ones that do not apply included (each with
its reason), and writes `docs/porting/TESTS.md` (per project: tests, found, waived, missing, and the missing tests by
name) and `docs/porting/data/tests-status.json`. It reads the upstream tests from an inventory,
`docs/porting/data/upstream-tests.json`, which it re-reads from the checkout (`git archive` of the tracked commit
and of the XamlX and D-Bus submodules) only when it names another commit or with `--extract`; `run.sh` runs it as
step 3 and `run.sh --check` runs its `--check`, which needs neither the checkout nor git. Its match is stricter than
that of `test_gaps.py` (one Rust function per upstream test, over all projects; a name of fewer than three words
only in the Rust file named after the upstream file); the rules and the calibration by hand are at the top of
`TESTS.md`.

```sh
python3 scripts/port-status/tests_audit.py                                    # TESTS.md and tests-status.json
python3 scripts/port-status/tests_audit.py --check                            # exit 1 when out of date
UPSTREAM=../Avalonia python3 scripts/port-status/tests_audit.py --extract     # re-read the upstream test projects
python3 scripts/port-status/tests_audit.py --explain Avalonia.Base.UnitTests 'Styling/*'   # what matched each test
```

## Library references

A reference with `documents = "library"` (Rx.NET) is extracted and scanned by the same code, with the options of its
entry in `references.toml`:

- `api = "public"`: the types and members counted are the public API (public and protected, with explicit interface
  implementations). Internal types (upstream's operator classes and their sinks, the internal helpers) count only
  through their files: a file whose types are all internal is a file to port, not a file without types.
- `overloads = "name"`: every overload of a method is present once the Rust function of its name in snake case
  exists, or that name with a suffix (`_on` for a scheduler argument, `_with`, `_with_index`, ...) or the `try_`
  prefix; constructors also match `with_*`. A waiver that names an overload (by name or by a glob on its signature)
  waives it even then.
- `method_macros = true`: the functions a `macro_rules!` of the crate writes count where the macro is invoked: in a
  trait or an `impl` (`super::select::select_methods!();` in `trait ObservableExt`), as its functions; at module level
  (`join_arity!(Pattern2, Plan2, ..);`), as the items of its first arm with the leading metavariables bound to the
  arguments.
- `doc_citations = true`: an upstream file without a Rust file at its own path, or a `[[map]]`, is present in the Rust
  files whose module documentation (the leading `//!` lines) names it in backticks (`` `Linq/Observable/Select.cs` ``,
  or a path below the directory of the Rust file, or the one upstream file of that name).
- `waived_files = true`: a missing file whose types are all waived counts as a waived file.
- `arity_names = true`: a generic type is also looked for under its name with its number of type parameters
  (`Pattern<T1, T2>` as `Pattern2`).

Its `path-overrides.toml` has one more table, `[[dir]]` (`upstream = "Linq/Observable"`, `rust = "linq"`): the Rust
directory of an upstream directory. Its `member-waivers.toml` holds the waivers of `docs/porting/reactive.md`,
section 5. For its tests (`test_gaps.py --reference reactive`), `test_projects`, `test_roots` and `test_attributes`
(MSTest's `TestMethod`) name what is read, and with `stated_reasons = true` a line `// Not ported: <Name> - <reason>`
of the Rust test file of an upstream test file (`Tests/Linq/Observable/SelectTest.cs` ->
`tests/FerroUI.Reactive.Tests/tests/linq/observable/select_test.rs`) states why that test is not ported: it counts as
deferred when the reason names a later phase, else as waived. The lines are read below the first test root, or below
the directories that an optional third element of an entry of `test_projects` lists (Xaml.Behaviors, whose test
project `Xaml.Behaviors.Interactions.UnitTests` is ported in the crates of the types it tests); the first reason
found for a test is taken. The test files of the waived API are waived in its
`test-aliases.toml`. `run.sh` does not run `test_gaps.py`; run it before `run.sh` when tests were ported, since the
index reads the totals of its report.

To add a reference: an entry in `references.toml`, a project list for `scripts/api-extract` (the format of
`projects.json`, with `rust` and `crate` set), and its data directory; `run.sh --reference <name>` extracts it.

## Pipeline

1. `scripts/api-extract` (C#, Roslyn syntax trees, no compilation) reads the upstream projects listed in
   `scripts/api-extract/projects.json` and writes `docs/porting/data/upstream-api.json`: project -> file -> type -> member,
   public, protected and internal (private members are only counted). If the JSON grows beyond 15 MB the extractor
   writes one file per project to `docs/porting/data/upstream/` and an index; the scanner reads both layouts.
   `run.sh` re-runs it when the JSON is missing, when the extractor or its project list changed, or when
   `TRACKED_COMMIT` names a commit other than the recorded one. The extractor reads the tracked commit, not the `HEAD`
   of the checkout, which is usually ahead: `run.sh` exports the tree of that commit, and of each submodule at the
   commit the tree names, with `git archive` into a temporary directory (below `PORT_STATUS_TMP`, else `TMPDIR`), runs
   the extractor on it with `--commit`, and removes it. Nothing is checked out. Files of the checkout that upstream
   ignores (build outputs) are therefore no part of the extraction. `API_EXTRACT_ARTIFACTS` names a directory for the
   build output of the extractor; `PYTHON` names the interpreter (3.11 or later).
2. `port_status.py` maps every upstream file to its Rust path (docs/porting/PORTING-GUIDE.md), scans the Rust files
   with `rustscan.py` (comments and literals blanked, brace structure walked, items found with regular expressions)
   and matches types and members by name. The Rust tree is only read. `REMAINING.md` also reads the totals at the top
   of the reports of `test_gaps.py` (`docs/porting/data/test-gaps-*.txt`) and `docs/porting/data/samples.toml`; it does
   not run `test_gaps.py`, so run that first when tests were ported.

## Files you edit

The files of the framework reference; another reference has the same files in its own data directory
(`docs/porting/data/reactive/` for Rx.NET) and its project list beside `projects.json`.

- `docs/porting/data/path-overrides.toml` - merged / renamed / replaced / not-applicable files, Rust-only files.
- `docs/porting/data/member-waivers.toml` - `[[waive]]` (not ported, with a reason) and `[[alias]]` (ported under a
  name the default rule cannot derive). Prefer an alias to a waiver whenever the member exists under another name: a
  waiver holds whether or not the item it names exists, an alias is checked at every run. An alias names a method, a
  constructor (`member = ".ctor"`), a property, a field, an event or an indexer, and the Rust items that stand for it
  (`rust = ["with_parent"]`): functions, fields or constants of the type, or of its file when the type is ported as a
  module. `docs/porting/waiver-audit.md` is the audit of the waivers of 2026-10-10 and says what a sound reason is.
- `docs/porting/data/samples.toml` - the upstream samples and the directory each is ported to.
- `scripts/api-extract/projects.json` - project list, level of detail, target crate, phase and priority.
  The scope, phase, priority and crate of a project are read from this file at every run, so moving a project into
  scope needs no new extraction; its level of detail is what the extraction holds. A project in scope that was
  extracted with `types` detail has its files and types looked for, and its members counted as totals. The list
  `expanded` gives the planning data of one project that a `dir/*` entry expands to, where it differs from its
  siblings (the scanner reads it, the extractor does not).

## Matching rules

| Upstream | Rust evidence |
|---|---|
| file `Dir/Foo.cs` | `dir/foo.rs`; `` Foo`1.cs `` -> `foo.rs`; `Foo.Part.cs` -> `foo_part.rs` or `foo.rs`; `IFoo.cs` -> `i_foo.rs`, or `foo.rs` if it declares `IFoo`. Names are compared ignoring case and underscores |
| type `Foo` | `struct` / `enum` / `union` / `trait` / `type` / `ferro_class!` named `Foo` (`Avalonia` -> `Ferro`, `Avn` -> `Frn`) in the mapped file, else anywhere in the crate; the items of a macro called with parentheses count like the items of one called with braces (`define_class!( pub struct Foo; impl Foo { .. } );`); nested `Outer.Inner` -> `Inner` in the mapped file or `OuterInner`; a static class also counts when its members exist as free items of the mapped file |
| property / field `Foo` | fn `foo` or `get_foo` (+ `set_foo` when the setter is not private), struct field `foo`, const / variant `FOO` |
| `FooProperty`, `FooEvent` | `foo_property()`, `foo_event()` (covers `ferro_property!`) |
| any member `AvaloniaFoo` | the name with the framework renamed as well (`ferro_foo`, `FERRO_FOO`): rule 2 of the porting guide |
| method `DoIt` | fn `do_it`; n-th overload: n-th of `do_it`, aliases, public `do_it_*` (names that belong to another upstream member are excluded); parameterless `GetFoo()` -> `foo()`; abstract / virtual members also match a fn of any trait declared in the same file |
| `ToString`, `Equals`, `GetHashCode`, `CompareTo`, `Parse` / `TryParse`, `Clone`, `Dispose`, `GetEnumerator` | `Display`, `PartialEq`, `Hash`, `PartialOrd`, `FromStr`, `Clone`, `Drop`, `IntoIterator` (derive or impl), or the method by name |
| constructors | `new` / `construct` (one constructor), `new_*`, `from_*`, `create`, `Default`, `From` impls, counted |
| static constructor | `static_constructor` (or the older `class_init`) |
| operators | `Add` / `Sub` / `Mul` / `Div` / `Neg` / `Not` / ... impls counted per operator; `==` / `!=` -> `PartialEq`; conversions -> `From` impl naming both types, or `to_x` / `from_x` / `as_x` |
| event `Foo` | fn `foo`, `add_foo` |
| indexer | `Index` impl, or `get` / `get_item` / `item` |
| enum member | variant or flag constant |
| explicit interface member `IFoo.Bar` | fn `bar` on the type (name only) |
| a function of a table of entry points (`DllImport`) | a field of the table written for `native_functions!` (`lib::name as "Symbol": unsafe extern "C" fn(..);`), when the table is the type |

The members of a type are collected from every `impl` block, trait impl and `ferro_class!` virtual list of the type,
of `XImpl`, `XExt`, `XImplExt` and `XDyn`, in the mapped files and in the other files of the crate that mention the type.

## Limits

Matching is by name. It does not compare signatures, parameter types or behaviour.

- False positives: a same-named item with different semantics; a type of the same name in another module of the crate;
  an `xxx_*` function counted as an overload of `Xxx`; trait derives counted for members they only partly cover.
- False negatives: overload sets collapsed into one generic function or enum parameter; members renamed beyond the
  suffix rule (use `[[alias]]`); abstract members moved to a trait declared in another file; delegates ported as
  `Rc<dyn Fn>` without a named alias; code generated by macros other than `ferro_class!` / `ferro_property!` / `bitflags!` / `native_functions!`
  (the items of any macro called with braces whose content is items, such as `pub fn name(..) -> T = Function;`, are read).
- For overloads the tool reports how many are missing, not which ones (public overloads are assumed to be ported first).
