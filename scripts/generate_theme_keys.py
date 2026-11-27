#!/usr/bin/env python3
"""Generates the typed resource keys of the themes (`keys.rs` of each theme crate).

For every keyed resource at the level of a theme's dictionaries, the script writes a constant named
after the key and carrying the type of its value, so that a wrong key or value type does not
compile:

    keys::ButtonBackground   ThemeResourceKey<Rc<dyn IBrush>>
    keys::ControlCornerRadius ThemeResourceKey<CornerRadius>

The value type comes from the element that defines the key. A `StaticResource` alias has the type
of what it names. A key whose definitions have different types, or whose element has no type here,
is a `ThemeResourceKey<BoxedValue>`: the key is checked, its value is not.

Left out: keys that are not Rust identifiers, the keys of theme variants (`Dark`, `Light`), keys in
the resources of a style, template or control, type keys (`{x:Type Button}`), and a keyed
`ResourceInclude` (`CompactStyles`), which the constructor of a theme takes out of its dictionary:
it is a document, not a resource. The keys of `DensityStyles/Compact.xaml` are kept, and resolve
when the theme has the compact density.

How to run (from the repository root; Python 3, standard library only):

    python3 scripts/generate_theme_keys.py            # writes the files
    python3 scripts/generate_theme_keys.py --check    # exit 1 if stale
    python3 scripts/generate_theme_keys.py --report   # the key-only keys of each theme, and why

The test `typed_keys_tests` of each theme crate resolves every generated key against the loaded
theme, in each variant, and checks the declared type.
"""

import argparse
import os
import re
import sys
import xml.etree.ElementTree as ElementTree

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
THEMES = ["Fluent", "MacOS", "Material", "Simple", "WinUI"]
XAML_KEY = "{http://schemas.microsoft.com/winfx/2006/xaml}Key"

BRUSH = "::std::rc::Rc<dyn ::ferroui_base::media::IBrush>"
UNTYPED = "::ferroui_base::BoxedValue"
VALUE_TYPES = {
    "Color": "::ferroui_base::media::Color",
    "Thickness": "::ferroui_base::Thickness",
    "CornerRadius": "::ferroui_base::CornerRadius",
    "Double": "f64",
    "String": "::std::string::String",
    "Boolean": "bool",
    "FontWeight": "::ferroui_base::media::FontWeight",
    "BoxShadows": "::ferroui_base::media::BoxShadows",
    "ControlTheme": "::ferroui_base::Ref<::ferroui_base::styling::ControlTheme>",
}
RUST_KEYWORDS = {
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn",
    "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self",
    "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while", "abstract", "become",
    "box", "do", "final", "macro", "override", "priv", "try", "typeof", "unsized", "virtual", "yield",
}


# Keys a document defines that the loaded theme does not have. `MaterialTheme::new` replaces the
# theme dictionaries of `Resources/MaterialColors.xaml` with `MaterialColorScheme::resources`, which
# makes the state brushes of the color roles and not those of the status colors, and nothing
# references these. A constant of such a key would compile and find nothing.
NOT_IN_THE_LOADED_THEME = {
    "Material": re.compile(r"Material(On)?(Success|Warning)(Container)?(Hover|Focus|Pressed|Dragged)Brush"),
}


def has_constant(theme, key):
    """Whether the key of the theme gets a constant."""
    if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", key) or key in RUST_KEYWORDS:
        return False
    left_out = NOT_IN_THE_LOADED_THEME.get(theme)
    return not (left_out and left_out.fullmatch(key))


def local_name(tag):
    return tag.rsplit("}", 1)[-1]


def is_container(element, parent_is_root):
    """Whether the children of `element` are entries of a dictionary of the theme."""
    name = local_name(element.tag)
    if name == "ResourceDictionary" or name.endswith(".ThemeDictionaries") or name.endswith(".MergedDictionaries"):
        return True
    return name.endswith(".Resources") and parent_is_root


def read_entries(element, is_root, entries):
    """The keyed entries below `element`: key -> the set of the names of their elements, or of
    `=Other` for an alias of the key `Other`."""
    theme_variants = local_name(element.tag).endswith(".ThemeDictionaries")
    for child in element:
        key = child.get(XAML_KEY)
        if key is not None and not theme_variants and not key.startswith("{"):
            name = local_name(child.tag)
            if name == "StaticResource" and child.get("ResourceKey"):
                entries.setdefault(key, set()).add("=" + child.get("ResourceKey"))
            elif name != "ResourceInclude":
                entries.setdefault(key, set()).add(name)
        if is_container(child, is_root):
            read_entries(child, False, entries)


def value_type(key, entries, seen=()):
    """The Rust type of the value of `key`, `UNTYPED` when there is no single one."""
    if key in seen or key not in entries:
        return UNTYPED
    types = set()
    for definition in entries[key]:
        if definition.startswith("="):
            types.add(value_type(definition[1:], entries, (*seen, key)))
        elif definition.endswith("Brush"):
            types.add(BRUSH)
        else:
            types.add(VALUE_TYPES.get(definition, UNTYPED))
    return types.pop() if len(types) == 1 else UNTYPED


def read_theme(theme):
    """The keyed entries of the documents of the theme."""
    directory = os.path.join(ROOT, "src", f"FerroUI.Themes.{theme}")
    entries = {}
    for root, _, files in os.walk(directory):
        for name in sorted(files):
            if name.endswith(".xaml"):
                document = ElementTree.parse(os.path.join(root, name)).getroot()
                read_entries(document, True, entries)
    return entries


def key_only_reason(key, entries, seen=()):
    """Why `key` has no single value type."""
    if key in seen:
        return "an alias that names itself"
    if key not in entries:
        return "an alias of a key outside of the theme's dictionaries"
    reasons = set()
    types = set()
    for definition in entries[key]:
        if definition.startswith("="):
            target = definition[1:]
            value = value_type(target, entries, (*seen, key))
            types.add(value)
            if value == UNTYPED:
                reasons.add(key_only_reason(target, entries, (*seen, key)))
        elif definition.endswith("Brush"):
            types.add(BRUSH)
        elif definition in VALUE_TYPES:
            types.add(VALUE_TYPES[definition])
        else:
            types.add(UNTYPED)
            reasons.add(f"element type without a mapped value type: {definition}")
    if len(types) > 1:
        return "definitions of different types"
    return sorted(reasons)[0] if reasons else "no single type"


def report(theme):
    """The key-only keys of the theme, by reason."""
    entries = read_theme(theme)
    keys = [key for key in entries if has_constant(theme, key)]
    by_reason = {}
    for key in sorted(keys):
        if value_type(key, entries) == UNTYPED:
            by_reason.setdefault(key_only_reason(key, entries), []).append(key)
    print(f"{theme}: {sum(map(len, by_reason.values()))} key-only of {len(keys)}")
    for reason in sorted(by_reason):
        print(f"  {reason} ({len(by_reason[reason])}): {', '.join(by_reason[reason])}")


def generate(theme):
    """The text of `keys.rs` of the theme."""
    entries = read_theme(theme)
    groups = {}
    for key in sorted(entries):
        if has_constant(theme, key):
            groups.setdefault(value_type(key, entries), []).append(key)
    lines = [
        "// @generated by scripts/generate_theme_keys.py - do not edit",
        f"//! The typed resource keys of the {theme} theme: a constant per key of its dictionaries, with",
        "//! the type of its value. See `scripts/generate_theme_keys.py`.",
        "",
        "#![allow(non_upper_case_globals, missing_docs)]",
        "",
        "ferroui_base::theme_keys! {",
    ]
    for value in sorted(groups):
        lines.append(f"    {value} =>")
        for index in range(0, len(groups[value]), 4):
            lines.append("        " + ", ".join(groups[value][index:index + 4]) + ",")
        lines.append("    ;")
    lines += ["}", ""]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--report", action="store_true")
    options = parser.parse_args()
    if options.report:
        for theme in THEMES:
            report(theme)
        return
    stale = False
    for theme in THEMES:
        path = os.path.join(ROOT, "src", f"FerroUI.Themes.{theme}", "keys.rs")
        text = generate(theme)
        current = None
        if os.path.exists(path):
            with open(path, encoding="utf-8") as f:
                current = f.read()
        if current == text:
            continue
        if options.check:
            print(f"stale: {os.path.relpath(path, ROOT)}")
            stale = True
            continue
        with open(path, "w", encoding="utf-8", newline="\n") as f:
            f.write(text)
        print(f"wrote {os.path.relpath(path, ROOT)}")
    if stale:
        sys.exit(1)


if __name__ == "__main__":
    main()
