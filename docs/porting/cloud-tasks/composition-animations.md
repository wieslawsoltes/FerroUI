# Task: composition animations and the remaining compositor pieces

Branch: `composition-animations`. Pull request title: `Compositor: composition animations, composition brushes, custom visuals`.

Row 12 of `CRITICAL-PATH.md` lists what the compositor port still lacks. Port it exactly, file by file, from `src/Avalonia.Base/Rendering/Composition` (client and `Server/` sides, `Animations/`, `Expressions/`, `Transport/`) and the generated composition objects (`scripts/generate_composition.py` and `scripts/composition-schema.xml` produce the port's generated files; extend the schema and the generator where upstream's schema has the objects and the port's does not, and regenerate):

1. Composition animations: key-frame animations for every value type upstream has, easing, animation groups, implicit animations and implicit animation collections, expression animations with the expression parser and evaluator, animation instances on the server, and the property-set object.
2. Composition brushes and the remaining visual kinds listed in row 12 (composition brushes, custom visuals and their handlers, drawing surfaces where they do not need a platform graphics context; leave a seam where they do).
3. The upstream tests for all of the above, and the compositor hit-test suite of the controls tests that row 12 lists as outstanding.
4. Close the seams that waited for this work: grep for composition seams in `src/FerroUI.Controls/pull_to_refresh/` (five) and anywhere else a seam comment names a composition animation API, and port the code they stand for.

One commit per group above. Update row 12 of `CRITICAL-PATH.md`. The render-thread contract decision named in row 12 is not part of this task: keep the single-threaded arrangement the port has and do not introduce threads.

Suites that must be green: base, controls, both themes, control-catalog, and the workspace check.
