#!/usr/bin/env python3
"""Check the resource keys of a theme derived from the WinUI theme, per theme variant.

    python3 scripts/themes/resource_keys.py macos             # the macOS 26 theme
    python3 scripts/themes/resource_keys.py material          # the Android Material theme
    python3 scripts/themes/resource_keys.py material --verbose  # also list the documents

Every key the documents reachable from the theme's root document reference with
`{StaticResource}`, `{DynamicResource}`, `{ThemeResource}` or a
`<StaticResource ResourceKey="..."/>` element must resolve for the Light, Dark
and HighContrast variants, from

* the top-level entries of a reachable dictionary,
* the theme dictionary of the variant (HighContrast inherits Dark; every
  variant falls back to the `Default` dictionary, as the resource lookup does),
* the keys the theme's Rust code supplies (string literals of its sources: live
  preference resources, the accent provider, color schemes set from code).

Keys that the WinUI theme references without defining either (framework and
application resources such as the colour picker's) are reported separately as
inherited and do not fail the check.

A key the theme defines for `Default` (the dark values of the WinUI scheme) but
not for `Light` resolves to the dark value in Light. The compiled theme flattens
the theme dictionaries of all merged documents into one per variant, so the
comparison is over the whole theme; such asymmetries fail the check unless the
WinUI theme has the same asymmetry.

Standard library only.
"""

from __future__ import annotations

import argparse
import posixpath
import re
import sys
from pathlib import Path
from urllib.parse import urlsplit
from xml.etree import ElementTree as ET

XAML_KEY = "{http://schemas.microsoft.com/winfx/2006/xaml}Key"
REFERENCE = re.compile(r"\{(?:DynamicResource|StaticResource|ThemeResource)\s+(?:ResourceKey\s*=\s*)?([^{}\s,]+)\s*\}")
STRING_LITERAL = re.compile(r'"([A-Za-z][A-Za-z0-9_.]*)"')
VARIANTS = ("Light", "Dark", "HighContrast")
# The variant chain of a lookup (ThemeVariant inheritance, then the Default dictionary).
CHAINS = {
    "Light": ("Light", "Default"),
    "Dark": ("Dark", "Default"),
    "HighContrast": ("HighContrast", "Dark", "Default"),
}
TEMPLATE_SCOPES = {"ControlTemplate", "DataTemplate", "FocusAdornerTemplate", "ItemsPanelTemplate", "TreeDataTemplate"}


def local_name(tag: str) -> str:
    return tag.rsplit("}", 1)[-1]


def theme_dictionary_name(node: ET.Element) -> str:
    key = node.get(XAML_KEY, "")
    if key.startswith("{x:Static") and key.rstrip("}").endswith(".HighContrast"):
        return "HighContrast"
    return key


class Theme:
    def __init__(self, directory: Path, entry: str, assembly: str, code_keys: set[str]):
        self.directory = directory
        self.assembly = assembly
        self.code_keys = code_keys
        self.documents: dict[str, ET.Element] = {}
        self.top: set[str] = set()
        self.themed: dict[str, set[str]] = {name: set() for name in ("Light", "Dark", "Default", "HighContrast")}
        self.references: dict[str, set[str]] = {}
        self.parse_errors: list[str] = []
        self.missing_documents: list[str] = []
        self.load(entry)

    def load(self, entry: str) -> None:
        pending = [entry]
        while pending:
            document = pending.pop()
            if document in self.documents:
                continue
            path = self.directory / document
            if not path.is_file():
                self.missing_documents.append(document)
                continue
            try:
                root = ET.parse(path).getroot()
            except ET.ParseError as error:
                self.parse_errors.append(f"{document}: {error}")
                continue
            self.documents[document] = root
            for node in root.iter():
                kind = local_name(node.tag)
                if kind not in {"MergeResourceInclude", "ResourceInclude", "StyleInclude"}:
                    continue
                source = node.get("Source", "")
                uri = urlsplit(source)
                if uri.scheme:
                    if uri.scheme != "ferres" or uri.netloc != self.assembly:
                        continue
                    target = uri.path.lstrip("/")
                elif source.startswith("/"):
                    target = source.lstrip("/")
                else:
                    target = posixpath.normpath(posixpath.join(posixpath.dirname(document), source))
                pending.append(target)
            self.collect(document, root)

    def collect(self, document: str, root: ET.Element) -> None:
        def visit(node: ET.Element, themed: str | None, in_template: bool) -> None:
            kind = local_name(node.tag)
            if kind == "ResourceDictionary.ThemeDictionaries":
                for dictionary in node:
                    name = theme_dictionary_name(dictionary)
                    keys = {child.get(XAML_KEY) for child in dictionary if child.get(XAML_KEY)}
                    self.themed.setdefault(name, set()).update(keys)
                    for child in dictionary:
                        visit(child, name, in_template)
                return
            key = node.get(XAML_KEY)
            # Keys of styles, control themes and resources outside templates and theme dictionaries.
            if key and themed is None and not in_template and not key.startswith("{x:Type"):
                self.top.add(key)
            for attribute in node.attrib.values():
                for match in REFERENCE.finditer(attribute):
                    self.references.setdefault(match.group(1), set()).add(document)
            if kind in {"StaticResource", "DynamicResource", "StaticResourceExtension", "DynamicResourceExtension"}:
                reference = node.get("ResourceKey")
                if reference:
                    self.references.setdefault(reference, set()).add(document)
            text = (node.text or "").strip()
            for match in REFERENCE.finditer(text):
                self.references.setdefault(match.group(1), set()).add(document)
            nested = in_template or kind in TEMPLATE_SCOPES
            for child in node:
                visit(child, themed, nested)

        visit(root, None, False)

    def asymmetries(self) -> dict[str, set[str]]:
        """Keys a variant does not resolve from its own chain of theme dictionaries although
        another variant does (the compiled theme flattens the theme dictionaries of all merged
        documents into one dictionary per variant, so the comparison is over the whole theme).
        A key only in `Default` counts as missing for Light: Light would get the dark value."""
        effective = {
            "Light": self.themed.get("Light", set()),
            "Dark": self.themed.get("Dark", set()) | self.themed.get("Default", set()),
            "HighContrast": set().union(*(self.themed.get(name, set()) for name in CHAINS["HighContrast"])),
        }
        union = set().union(*effective.values())
        return {name: union - keys for name, keys in effective.items() if union - keys}

    def resolves(self, key: str, variant: str) -> bool:
        if key in self.top or key in self.code_keys:
            return True
        return any(key in self.themed.get(name, set()) for name in CHAINS[variant])

    def unresolved(self) -> dict[str, list[str]]:
        result: dict[str, list[str]] = {}
        for key in self.references:
            missing = [variant for variant in VARIANTS if not self.resolves(key, variant)]
            if missing:
                result[key] = missing
        return result


def code_keys(paths: list[Path]) -> set[str]:
    keys: set[str] = set()
    for path in paths:
        if path.is_file():
            keys.update(STRING_LITERAL.findall(path.read_text(encoding="utf-8")))
    return keys


# Theme name: (crate directory, root document, Rust sources that supply resource keys).
THEMES = {
    "macos": ("FerroUI.Themes.MacOS", "MacOSTheme.xaml", ["settings_resources.rs", "Accents/system_accent_colors.rs"]),
    "material": ("FerroUI.Themes.Material", "MaterialTheme.xaml", ["material_theme.rs", "color_scheme.rs", "Accents/*.rs"]),
}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("theme", nargs="?", default="macos", choices=sorted(THEMES), help="the theme to check")
    parser.add_argument("--verbose", action="store_true", help="list the documents of each finding")
    args = parser.parse_args()
    repository = Path(__file__).resolve().parents[2]
    crate, entry, sources = THEMES[args.theme]
    macos_dir = repository / "src" / crate
    winui_dir = repository / "src/FerroUI.Themes.WinUI"
    code = sorted({path for pattern in sources for path in macos_dir.glob(pattern) if path.name != "compiled_xaml.rs"})
    macos = Theme(macos_dir, entry, crate, code_keys(code))
    winui = Theme(winui_dir, "WinUITheme.xaml", "FerroUI.Themes.WinUI",
                  code_keys([winui_dir / "Accents/system_accent_colors.rs"]))

    failures = 0
    for theme in (macos, winui):
        for error in theme.parse_errors:
            print(f"ERROR {theme.assembly}: invalid XML {error}")
        for document in theme.missing_documents:
            print(f"ERROR {theme.assembly}: missing merged document {document}")
    failures += len(macos.parse_errors) + len(macos.missing_documents)

    macos_unresolved = macos.unresolved()
    winui_unresolved = winui.unresolved()
    winui_referenced = set(winui.references)
    inherited = {key: v for key, v in macos_unresolved.items() if key in winui_unresolved}
    dangling = {key: v for key, v in macos_unresolved.items() if key not in winui_unresolved}

    print(f"{crate}: {len(macos.documents)} documents, {len(macos.references)} referenced keys, "
          f"{len(macos.top)} top-level keys, theme dictionaries "
          + ", ".join(f"{name} {len(keys)}" for name, keys in sorted(macos.themed.items()))
          + f", {len(macos.code_keys)} code literals")
    for key in sorted(dangling):
        origin = f"new in {args.theme}" if key not in winui_referenced else f"defined by WinUI, not by {args.theme}"
        print(f"ERROR dangling key {key} ({origin}) for {', '.join(dangling[key])}"
              + (f": {', '.join(sorted(macos.references[key]))}" if args.verbose else ""))
    failures += len(dangling)
    if inherited:
        print(f"note: {len(inherited)} keys are unresolved in the WinUI theme too (framework or application "
              f"resources): {', '.join(sorted(inherited))}")

    winui_asymmetries = winui.asymmetries()
    for variant, keys in sorted(macos.asymmetries().items()):
        baseline = winui_asymmetries.get(variant, set())
        new = sorted(keys - baseline)
        if new:
            print(f"ERROR theme dictionary {variant} lacks {len(new)} keys its siblings define: {', '.join(new)}")
            failures += 1
        if keys & baseline:
            print(f"note: theme dictionary {variant} lacks {len(keys & baseline)} keys, as in the WinUI theme")

    print(f"{crate} resource keys: " + ("FAIL" if failures else "PASS"))
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
