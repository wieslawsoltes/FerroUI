#!/usr/bin/env python3
"""Count the property lookups by name that compiled themes still make at run time (#603).

    python3 scripts/themes/by_name_lookups.py                 # the checked-in compiled themes
    python3 scripts/themes/by_name_lookups.py FILE...         # any generated files
    python3 scripts/themes/by_name_lookups.py --by-property   # one line per owner and property

Two run-time helpers of the compiled markup find a declared (plain) property by its name:

* `rt::clr_property_info(markup, "Name", ..)`: the description of a property as the provide-value
  target property of a nested markup extension (`TemplateBinding.Converter` with a
  `{StaticResource}`, `MultiBinding.Bindings` and `Converter`);
* `rt::path_property(&builder, markup, "Name", ..)`: a plain property in a compiled binding path
  (`ProgressBar.TemplateSettings`, `CommandBar.VisiblePrimaryCommands`, the view models of the
  managed file chooser).

The script reads the generated Rust text and counts the calls of each, for each file, and with
`--by-property` for each owner (the markup type, as the call writes it) and property name.
`rt::setter_value_property()` (`Setter.Value`, one shared description) is not a lookup of a call
site and is not counted. Standard library only; it compiles nothing.
"""

import argparse
import re
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# The compiled themes that are checked in (a theme compiled only in its build script has none).
THEMES = ["WinUI", "Fluent", "Simple", "MacOS", "Material"]

HELPERS = {
    "clr_property_info": re.compile(r"rt::clr_property_info\(\s*(?P<owner>[^,]+?),\s*\"(?P<name>[^\"]*)\""),
    "path_property": re.compile(r"rt::path_property\(\s*&?builder,\s*(?P<owner>[^,]+?),\s*\"(?P<name>[^\"]*)\""),
}


def short_owner(owner: str) -> str:
    """The markup type of an owner expression, by the last segment of its Rust path
    (`<::ferroui_base::data::TemplateBinding as ..>::MARKUP` is `TemplateBinding`)."""
    match = re.search(r"<\s*(?:dyn\s+)?::?([A-Za-z0-9_:]+)\s+as\s", owner)
    path = match.group(1) if match else owner
    return path.rsplit("::", 1)[-1]


def count(text: str) -> dict:
    """The calls of each helper in `text`, by `(owner, name)`."""
    return {
        helper: Counter((short_owner(m.group("owner")), m.group("name")) for m in pattern.finditer(text))
        for helper, pattern in HELPERS.items()
    }


def default_files() -> list:
    files = []
    for theme in THEMES:
        path = ROOT / "src" / f"FerroUI.Themes.{theme}" / "compiled_xaml.rs"
        if path.is_file():
            files.append(path)
    return files


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("files", nargs="*", type=Path, help="generated files (default: the checked-in themes)")
    parser.add_argument("--by-property", action="store_true", help="one line per owner and property")
    args = parser.parse_args()
    files = args.files or default_files()
    if not files:
        print("no generated files found", file=sys.stderr)
        return 1
    totals = {helper: 0 for helper in HELPERS}
    for path in files:
        counts = count(path.read_text(encoding="utf-8"))
        label = path.relative_to(ROOT) if path.is_relative_to(ROOT) else path
        line = ", ".join(f"{helper} {sum(found.values())}" for helper, found in counts.items())
        print(f"{label}: {line}")
        for helper, found in counts.items():
            totals[helper] += sum(found.values())
            if args.by_property:
                for (owner, name), number in sorted(found.items(), key=lambda item: (-item[1], item[0])):
                    print(f"    {helper:18} {number:4}  {owner}.{name}")
    print("total: " + ", ".join(f"{helper} {number}" for helper, number in totals.items()))
    return 0


if __name__ == "__main__":
    sys.exit(main())
