# Task: the ahead-of-time XAML compiler

Branch: `xaml-compiler` (it exists; continue on it and keep it rebased on `main`). One pull request per stage, each against `main`, titled `XAML compiler: stage E<n> - <what>`; open the next stage's branch from the merged result or stack it and say so.

Read, in this order: `docs/porting/xaml.md` (the whole pipeline; section 9 is the compiler design and 9.12 its rulings), `docs/porting/xaml-compiler/HANDOVER.md` (the state of the draft on this branch and everything the previous worker knew), `docs/porting/xaml-compiler/DRAFT-AUTHOR-REPORT.md` (what the draft's author could not verify).

Why this matters: every application today parses, transforms and interprets its XAML at start-up. Measured on macOS, release build: a window with no theme opens in 190 ms; with the Simple theme 417 ms; with Fluent 543 ms; the sample catalog 876 ms; stripped binaries 22.9, 48.7 and 85.8 MB, and the browser module of the catalog is 84.8 MB. The compiler removes the run-time parse and lets applications stop linking the parser, the transformers, the interpreter and the by-name metadata.

Rules specific to this task, in addition to the standing brief:

- The emitter produces real Rust that rustc type-checks: direct constructor calls, direct typed property sets and adds, typed constants, paths for `x:Static` and `x:Type`. A by-name call through the run-time metadata is the fallback only for a member that has no static path, as ruling 5 says; it is never the general form.
- Nothing is approximated. A node the emitter does not cover makes the document "not eligible", with the reason reported. Coverage is always a measured count.
- The proof is differential: every eligible document is built both ways, by the run-time loader and by the compiled code, and the two object graphs are compared. Generated sources that are only syntax-checked do not count.
- The run-time loader stays fully working and its suites stay green at every stage.
- Emission metadata is carried by the registries and produced mechanically (macros, the generator). Hand-written lists of Rust paths are a stopgap that stage E2 removes.

Stages. Each ends with measured numbers in its pull request: documents eligible and matching out of the corpus, and from E3 on the effect on start-up time and binary size of the `themed_window` example (build it for the host; time-to-first-frame cannot be measured on Linux, so report the time spent loading the theme, measured inside a test, and the size of the stripped binary and of the browser module of the `themed_view` example).

- E1. Make the draft build. Regenerate the checked-in generated sources with the real emitter (the file on the branch is a prediction written by hand). Differential harness green. Scope: object construction, registered styled and attached properties, constants, names, init calls.
- E2. Emission metadata done properly: mechanically derived public Rust paths for every registered class, enum, value type and contract; accessor text for plain properties, content properties and collection adds, which means rewriting how the declaration macros capture accessors (the handover explains why the draft stopped there) and covering the list projection the loader's type system synthesises; direct properties.
- E3. Markup extensions, bindings (compiled binding paths as typed accessors, as upstream's compiled bindings are), static and dynamic resources, the parent stack and the service-provider context.
- E4. Templates and deferred content, styles and selectors, control themes, setters, includes and merged dictionaries: the node set the two themes need. Target: both theme crates pass their suites when the themes are loaded from compiled output, and the `themed_window` example no longer links the run-time loader.
- E5. Build integration as section 9 designs it (build-script helper and the include macro), code-behind and `x:Class` documents, event handlers; the sample catalog on compiled XAML; the transform step running at build time without linking the framework, if E2's tables make that possible.

Suites that must be green at every stage: base, controls, the three markup crates, xamlx, the XAML tests, both themes, control-catalog, the workspace check, the generator check.
