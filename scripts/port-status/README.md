# port-status

Generates the port tracking documents:

- `docs/porting/TRACKING.md` - master document (totals, per-project table, project structure)
- `docs/porting/tracking/<Project>.md` - per project: contracts, files by directory, missing types and members
- `docs/porting/data/port-status.json` - the same totals, machine readable

```sh
scripts/port-status/run.sh            # upstream checkout at ../Avalonia, or UPSTREAM=/path/to/Avalonia
scripts/port-status/run.sh --force    # re-run the upstream extractor
scripts/port-status/run.sh --check    # exit 1 when the generated documents are out of date
python3 scripts/port-status/port_status.py --explain Avalonia.Base Point.cs   # how each member was matched
```

Requirements: Python 3.11+ (standard library only); the .NET SDK only when the upstream JSON has to be regenerated.

The tracking documents count members. The upstream unit tests that the port does not have are counted by a
script of its own, which reads the upstream test projects at the tracked commit and the test functions of the port:

```sh
python3 scripts/port-status/test_gaps.py ../Avalonia --all                           # every project: docs/porting/data/test-gaps-*.txt and the table
python3 scripts/port-status/test_gaps.py ../Avalonia tests/Avalonia.Base.UnitTests   # the report of one project
```

Its rules are at its top; `docs/porting/data/test-aliases.toml` holds the tests the port has under another name
and the ones that are not ported for a stated reason, with a section per upstream test project.

## Pipeline

1. `scripts/api-extract` (C#, Roslyn syntax trees, no compilation) reads the upstream projects listed in
   `scripts/api-extract/projects.json` and writes `docs/porting/data/upstream-api.json`: project -> file -> type -> member,
   public, protected and internal (private members are only counted). If the JSON grows beyond 15 MB the extractor
   writes one file per project to `docs/porting/data/upstream/` and an index; the scanner reads both layouts.
   `run.sh` re-runs it when the JSON is missing, the upstream `HEAD` differs from the recorded commit, or the extractor
   or its project list changed.
2. `port_status.py` maps every upstream file to its Rust path (docs/porting/PORTING-GUIDE.md), scans the Rust files
   with `rustscan.py` (comments and literals blanked, brace structure walked, items found with regular expressions)
   and matches types and members by name. The Rust tree is only read.

## Files you edit

- `docs/porting/data/path-overrides.toml` - merged / renamed / replaced / not-applicable files, Rust-only files.
- `docs/porting/data/member-waivers.toml` - `[[waive]]` (not ported, with a reason) and `[[alias]]` (ported under a
  name the default rule cannot derive).
- `scripts/api-extract/projects.json` - project list, level of detail, target crate, phase and priority.

## Matching rules

| Upstream | Rust evidence |
|---|---|
| file `Dir/Foo.cs` | `dir/foo.rs`; `` Foo`1.cs `` -> `foo.rs`; `Foo.Part.cs` -> `foo_part.rs` or `foo.rs`; `IFoo.cs` -> `i_foo.rs`, or `foo.rs` if it declares `IFoo`. Names are compared ignoring case and underscores |
| type `Foo` | `struct` / `enum` / `union` / `trait` / `type` / `ferro_class!` named `Foo` (`Avalonia` -> `Ferro`, `Avn` -> `Frn`) in the mapped file, else anywhere in the crate; nested `Outer.Inner` -> `Inner` in the mapped file or `OuterInner`; a static class also counts when its members exist as free items of the mapped file |
| property / field `Foo` | fn `foo` or `get_foo` (+ `set_foo` when the setter is not private), struct field `foo`, const / variant `FOO` |
| `FooProperty`, `FooEvent` | `foo_property()`, `foo_event()` (covers `ferro_property!`) |
| method `DoIt` | fn `do_it`; n-th overload: n-th of `do_it`, aliases, public `do_it_*` (names that belong to another upstream member are excluded); parameterless `GetFoo()` -> `foo()`; abstract / virtual members also match a fn of any trait declared in the same file |
| `ToString`, `Equals`, `GetHashCode`, `CompareTo`, `Parse` / `TryParse`, `Clone`, `Dispose`, `GetEnumerator` | `Display`, `PartialEq`, `Hash`, `PartialOrd`, `FromStr`, `Clone`, `Drop`, `IntoIterator` (derive or impl), or the method by name |
| constructors | `new` / `construct` (one constructor), `new_*`, `from_*`, `create`, `Default`, `From` impls, counted |
| static constructor | `static_constructor` (or the older `class_init`) |
| operators | `Add` / `Sub` / `Mul` / `Div` / `Neg` / `Not` / ... impls counted per operator; `==` / `!=` -> `PartialEq`; conversions -> `From` impl naming both types, or `to_x` / `from_x` / `as_x` |
| event `Foo` | fn `foo`, `add_foo` |
| indexer | `Index` impl, or `get` / `get_item` / `item` |
| enum member | variant or flag constant |
| explicit interface member `IFoo.Bar` | fn `bar` on the type (name only) |

The members of a type are collected from every `impl` block, trait impl and `ferro_class!` virtual list of the type,
of `XImpl`, `XExt`, `XImplExt` and `XDyn`, in the mapped files and in the other files of the crate that mention the type.

## Limits

Matching is by name. It does not compare signatures, parameter types or behaviour.

- False positives: a same-named item with different semantics; a type of the same name in another module of the crate;
  an `xxx_*` function counted as an overload of `Xxx`; trait derives counted for members they only partly cover.
- False negatives: overload sets collapsed into one generic function or enum parameter; members renamed beyond the
  suffix rule (use `[[alias]]`); abstract members moved to a trait declared in another file; delegates ported as
  `Rc<dyn Fn>` without a named alias; code generated by macros other than `ferro_class!` / `ferro_property!` / `bitflags!`.
- For overloads the tool reports how many are missing, not which ones (public overloads are assumed to be ported first).
