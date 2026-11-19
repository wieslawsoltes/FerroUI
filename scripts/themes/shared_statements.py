#!/usr/bin/env python3
"""Count the statements of compiled markup that shared helpers of `rt` can stand for (#568).

    python3 scripts/themes/shared_statements.py               # the checked-in compiled themes
    python3 scripts/themes/shared_statements.py FILE...       # any generated files
    python3 scripts/themes/shared_statements.py --json        # the same as JSON

The native emitter writes the construction of a template or a document as one statement per
member call. A few sequences are the same at every element of every template; each is counted
here twice, in the form the emitter wrote before a shared helper stood for it ("inline") and in
the form of the call of the helper ("shared"), so that the same script reads a file before and
after a change of the emitter:

* `push_parent`: an element on the parent stack of the context,
  `context.push_parent(rt::to_value(x.clone()))` against `rt::push_parent(&context, &x)`;
* `intermediate_root`: the root of deferred content,
  `context.set_intermediate_root_object(rt::to_value(x.clone()))` against
  `rt::set_intermediate_root(&context, &x)`;
* `end_init`: the end of the initialisation of a styled element,
  `rt::invoked(x.try_end_init(), ..)?` (after `context.pop_parent()` or alone) against
  `rt::pop_parent_end_init(..)` and `rt::end_init(..)`;
* `name`: the name of an element, `set_direct_value` of `StyledElement.Name` followed by
  `rt::register_name_value` against `rt::name_element(..)`;
* `compiled_binding`: a `{CompiledBinding}` provided and bound to a property (the extension
  created with its path, the target property set, `ProvideValue`, the target property cleared,
  `rt::bind`) against `rt::bind_compiled_binding(..)`;
* `static_resource`: the value of a `{StaticResource}` for a registered property (the extension
  created with the key, `rt::provide_value_invoked` with `rt::property_value(..)`, the provided
  value held as a node value) against `rt::static_resource_value(..)`;
* `dynamic_resource`: a `{DynamicResource}` bound to a registered property, one
  `rt::bind_dynamic_resource(..)` each against a row of a run (`rt::DynamicResourceRow`).

With them it counts the lines, the functions and the code statements of each file (a code
statement is a line inside a function that is not a comment, a brace or part of a table). Only
the shapes of the emitter are recognised; the counts are of the source, not of the linked
executable, whose size is measured on the file (`docs/binary-size.md`, section 2). Standard
library only; it compiles nothing.
"""

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# The compiled themes that are checked in, and the corpus of the emitter tests.
FILES = [
    "src/FerroUI.Themes.WinUI/compiled_xaml.rs",
    "src/FerroUI.Themes.Fluent/compiled_xaml.rs",
    "src/FerroUI.Themes.Simple/compiled_xaml.rs",
    "src/FerroUI.Themes.MacOS/compiled_xaml.rs",
    "tests/FerroUI.Markup.Xaml.UnitTests/emitter/generated.rs",
]

LOCAL = r"[a-z_][a-z0-9_]*"

PATTERNS = {
    "push_parent": (
        re.compile(rf"^\s*context\.push_parent\(rt::to_value\(({LOCAL})\.clone\(\)\)\);$"),
        re.compile(rf"^\s*rt::push_parent\(&context, &{LOCAL}\);$"),
    ),
    "intermediate_root": (
        re.compile(rf"^\s*context\.set_intermediate_root_object\(rt::to_value\({LOCAL}\.clone\(\)\)\);$"),
        re.compile(rf"^\s*rt::set_intermediate_root\(&context, &{LOCAL}\);$"),
    ),
    "end_init": (
        re.compile(rf"^\s*rt::invoked\({LOCAL}\.try_end_init\(\), -?\d+, -?\d+\)\?;$"),
        re.compile(rf"^\s*rt::(?:pop_parent_end_init\(&context, |end_init\()&{LOCAL}, -?\d+, -?\d+\)\?;$"),
    ),
    "name": (
        re.compile(r"^\s*rt::register_name_value\(name_scope\.as_ref\(\), "),
        re.compile(r"^\s*rt::name_element\(name_scope\.as_ref\(\), "),
    ),
    "compiled_binding": (
        re.compile(r"CompiledBindingExtension::__markup_ProvideValue_0\("),
        re.compile(r"^\s*rt::bind_compiled_binding\("),
    ),
    "static_resource": (
        re.compile(r"rt::provide_value_invoked\(&context, rt::property_value\(.*StaticResourceExtension::__markup_ProvideValue_0, "),
        re.compile(r"rt::static_resource_value\(&context, "),
    ),
    "dynamic_resource": (
        re.compile(r"^\s*rt::bind_dynamic_resource\(&context, "),
        re.compile(r"^\s*rt::DynamicResourceRow \{"),
    ),
}

# The name of an element written inline: the set of `StyledElement.Name` right before the
# registration (other registrations, of objects that are no styled element, are not counted).
NAME_SET = re.compile(r"\.set_direct_value\(::ferroui_base::StyledElement::name_property\(\), ")


def count(text: str) -> dict:
    lines = text.splitlines()
    result = {name: {"inline": 0, "shared": 0} for name in PATTERNS}
    functions = 0
    statements = 0
    depth = 0
    in_function = False
    previous = ""
    for line in lines:
        stripped = line.strip()
        if re.match(r"^(pub(\([a-z]+\))? )?fn ", line):
            functions += 1
            in_function = True
        if in_function and stripped and not stripped.startswith("//") and stripped not in ("{", "}", "};", "],", "];", ")", ");"):
            if not line.startswith(("fn ", "pub fn ", "pub(crate) fn ", "pub(super) fn ")) and depth >= 1:
                statements += 1
        depth += line.count("{") - line.count("}")
        if in_function and depth == 0 and stripped == "}":
            in_function = False
        for name, (inline, shared) in PATTERNS.items():
            if name == "name":
                if inline.search(line) and NAME_SET.search(previous):
                    result[name]["inline"] += 1
            elif inline.search(line):
                result[name]["inline"] += 1
            if shared.search(line):
                result[name]["shared"] += 1
        if not stripped.startswith("//"):
            previous = line
    result["lines"] = len(lines)
    result["functions"] = functions
    result["statements"] = statements
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("files", nargs="*", help="generated files (default: the checked-in themes and corpus)")
    parser.add_argument("--json", action="store_true", help="print JSON")
    args = parser.parse_args()
    files = [Path(f) for f in args.files] or [ROOT / f for f in FILES if (ROOT / f).is_file()]
    results = {}
    for path in files:
        results[str(path.relative_to(ROOT) if path.is_absolute() and ROOT in path.parents else path)] = count(
            path.read_text(encoding="utf-8")
        )
    if args.json:
        json.dump(results, sys.stdout, indent=2)
        print()
        return 0
    names = list(PATTERNS)
    header = ["file", "lines", "functions", "statements"] + [f"{n} (inline/shared)" for n in names]
    print(" | ".join(header))
    for path, result in results.items():
        row = [path, str(result["lines"]), str(result["functions"]), str(result["statements"])]
        row += [f"{result[n]['inline']}/{result[n]['shared']}" for n in names]
        print(" | ".join(row))
    return 0


if __name__ == "__main__":
    sys.exit(main())
