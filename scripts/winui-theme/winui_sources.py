"""The WinUI 3 theme in microsoft-ui-xaml, as the port reads it (docs/porting/winui-theme.md).

Shared by `status.py` (the tracking of the port) and `convert.py` (the converter of resources).
Only the standard library.

The theme of a WinUI 3 application is one dictionary that the build of microsoft-ui-xaml merges
from three parts, in this order, a later definition of a key replacing an earlier one
(`controls/tools/CustomTasks/MergedDictionary.cs`, `controls/tools/GenerateMergedXaml.ps1`,
`controls/dev/dll/Microsoft.UI.Xaml.Common.targets`):

1. `dxaml/xcp/dxaml/themes/generic.xaml`: the system resources and the styles of the controls
   of the framework;
2. the pages of the controls library of type `DefaultStyle` (its `generic.xaml`);
3. the pages of type `ThemeResources` (its `themeresources.xaml`, which `XamlControlsResources`
   loads), ordered by their `Priority` (1 to 8, then the ones without).

Within a type the pages keep the order of the `.vcxitems` files that `controls/ProjectImports.targets`
imports. A key of a theme dictionary also removes the key from the dictionary itself (the
"ancestor" of the theme dictionary in `MergedDictionary.RemoveAncestorNodesWithKey`), and the
theme dictionary `Dark` is merged into `Default`.

The variant chosen here is the default one: the pages of type `StylePerf2026` and
`ThemeResourcesPerf2026` (`*_perf2026.xaml`, `generic_perf2026.xaml`) are what
`XamlControlsResources` loads only when `AreDefaultStyleOptimizationsEnabled()` is true, and are
not read. The compact density (`compact_themeresources.xaml`) is the theme resources with the
pages of `controls/dev/dll/DensityStyles` merged last; it is read as a separate layer.
"""

import os
import re
import xml.etree.ElementTree as ET
from dataclasses import dataclass, field

# The checkout of microsoft-ui-xaml the port is made from (branch winui3/main).
REFERENCE_COMMIT = "7e4b0c2096be402f51cc8f7d203ebf67cff183d0"
DEFAULT_REFERENCE = "/Users/wieslawsoltes/GitHub/microsoft-ui-xaml"

XAML_NS = "http://schemas.microsoft.com/winfx/2006/xaml"
PRESENTATION_NS = "http://schemas.microsoft.com/winfx/2006/xaml/presentation"
FERRO_NS = "https://github.com/ferroui"

GENERIC = "dxaml/xcp/dxaml/themes/generic.xaml"
DENSITY = ["controls/dev/dll/DensityStyles/Compact.xaml", "controls/dev/dll/DensityStyles/CompactDatePickerTimePickerFlyout.xaml"]

# The variants of a theme dictionary as the merge of the build names them.
VARIANT_ALIASES = {"Dark": "Default"}


def reference_root(argument=None):
    root = argument or os.environ.get("WINUI_XAML_ROOT") or DEFAULT_REFERENCE
    if not os.path.isfile(os.path.join(root, GENERIC)):
        raise SystemExit(f"error: {root} is not a checkout of microsoft-ui-xaml (no {GENERIC}); pass --reference or set WINUI_XAML_ROOT")
    return root


def x(name):
    return f"{{{XAML_NS}}}{name}"


def local_name(tag):
    return tag.rsplit("}", 1)[-1] if isinstance(tag, str) else ""


def namespace(tag):
    return tag[1:].split("}", 1)[0] if isinstance(tag, str) and tag.startswith("{") else ""


def type_name(tag):
    """`x:Double` for the XAML language namespace, the local name otherwise (with the CLR namespace
    of a `using:` namespace dropped, as WinUI resolves them all to its own types)."""
    if namespace(tag) == XAML_NS:
        return "x:" + local_name(tag)
    return local_name(tag)


def strip_prefix(value):
    """`controls:NavigationView` -> `NavigationView`."""
    return value.split(":", 1)[-1] if value else value


def parse(path):
    """Parses a XAML document with its comments."""
    parser = ET.XMLParser(target=ET.TreeBuilder(insert_comments=True))
    with open(path, "rb") as file:
        data = file.read()
    if data.startswith(b"\xef\xbb\xbf"):
        data = data[3:]
    parser.feed(data)
    return parser.close()


# --- the source documents -------------------------------------------------------------------------


def _pages(root):
    """The shared pages of the controls library: (type, priority, path), in import order."""
    imports = os.path.join(root, "controls/ProjectImports.targets")
    text = open(imports, encoding="utf-8-sig").read()
    items = re.findall(r'Import Project="\$\(MSBuildThisFileDirectory\)([^"]+\.vcxitems)"', text)
    pages = []
    for item in items:
        item_path = os.path.join(root, "controls", item.replace("\\", "/"))
        if not os.path.isfile(item_path):
            continue
        directory = os.path.dirname(item_path)
        body = open(item_path, encoding="utf-8-sig", errors="replace").read()
        for match in re.finditer(r'<Page\s+(?:Condition="([^"]*)"\s+)?Include="([^"]+)"\s*>(.*?)</Page>', body, re.S):
            condition, include, metadata = match.groups()
            if condition and "!= 'true'" not in condition:
                # The `MUXFinalRelease == 'true'` variants name generated copies of the same pages.
                continue
            if "$(MSBuildThisFileDirectory)" not in include:
                continue
            page = os.path.join(directory, include.replace("$(MSBuildThisFileDirectory)", "").replace("\\", "/"))
            kind = re.search(r"<Type>(\w+)</Type>", metadata)
            priority = re.search(r"<Priority>(\w+)</Priority>", metadata)
            pages.append((kind.group(1) if kind else "Undefined", priority.group(1) if priority else "Undefined", os.path.relpath(page, root)))
    return pages


def source_documents(root):
    """The documents of the theme in the order of the merge: [(path, layer)] where layer is
    `generic`, `default-style` or `theme-resources`."""
    pages = _pages(root)
    order = [str(n) for n in range(1, 9)] + ["Undefined"]

    def ordered(kind):
        result = []
        for priority in order:
            result += [path for (k, p, path) in pages if k == kind and p == priority]
        return result

    documents = [(GENERIC, "generic")]
    documents += [(path, "default-style") for path in ordered("DefaultStyle")]
    documents += [(path, "theme-resources") for path in ordered("ThemeResources")]
    seen = set()
    unique = []
    for path, layer in documents:
        if path not in seen:
            seen.add(path)
            unique.append((path, layer))
    return unique


# --- the items of a document ----------------------------------------------------------------------


@dataclass
class Item:
    document: str
    variant: str  # "" for the dictionary itself, else the theme dictionary (Default, Light, HighContrast)
    key: str  # x:Key, or "{TargetType}" for an implicit style
    kind: str  # the type of the element (`Color`, `x:Double`, `Style`, ...)
    element: ET.Element = field(repr=False, compare=False)
    target_type: str = ""
    line: int = 0

    @property
    def is_style(self):
        return self.kind == "Style"

    @property
    def ident(self):
        return f"{self.variant}/{self.key}" if self.variant else self.key


def element_key(element):
    key = element.get(x("Key")) or element.get(x("Name"))
    if key:
        return key
    if type_name(element.tag) == "Style":
        target = element.get("TargetType")
        if target:
            return "{" + strip_prefix(target) + "}"
    return None


def document_items(root, path):
    """The keyed entries of a resource dictionary document: those of the dictionary and those of
    its theme dictionaries."""
    tree = parse(os.path.join(root, path))
    items = []
    for child in tree:
        if not isinstance(child.tag, str):
            continue
        name = local_name(child.tag)
        if name == "ResourceDictionary.ThemeDictionaries":
            for dictionary in child:
                if not isinstance(dictionary.tag, str):
                    continue
                variant = dictionary.get(x("Key"), "Default")
                variant = VARIANT_ALIASES.get(variant, variant)
                for entry in dictionary:
                    if not isinstance(entry.tag, str):
                        continue
                    key = element_key(entry)
                    if key:
                        items.append(Item(path, variant, key, type_name(entry.tag), entry, strip_prefix(entry.get("TargetType", ""))))
        elif name == "ResourceDictionary.MergedDictionaries":
            continue
        else:
            key = element_key(child)
            if key:
                items.append(Item(path, "", key, type_name(child.tag), child, strip_prefix(child.get("TargetType", ""))))
    return items


@dataclass
class Theme:
    """The merged theme: the effective items and the ones a later definition replaced."""

    documents: list
    effective: dict  # (variant, key) -> Item
    superseded: list  # (Item, Item that replaced it)
    compact: dict  # key -> Item of the density layer
    items_by_document: dict


def is_inert_style(item):
    """An implicit style without setters and without a base: it changes nothing. WinUI applies an
    implicit style of the application resources (`XamlControlsResources`) on top of the default
    style of the control (`generic.xaml`), so such a style (`<Style TargetType="ProgressBar" />`
    of `Deprecated_themeresources.xaml`) leaves the default style in effect; it is not an item of
    the theme."""
    if item.kind != "Style" or not item.key.startswith("{"):
        return False
    return item.element.get("BasedOn") is None and not any(isinstance(child.tag, str) for child in item.element)


def merge(root):
    documents = source_documents(root)
    effective = {}
    superseded = []
    items_by_document = {}
    for path, _layer in documents:
        items = document_items(root, path)
        items_by_document[path] = items
        for item in items:
            if is_inert_style(item):
                continue
            previous = effective.get((item.variant, item.key))
            if previous is not None:
                superseded.append((previous, item))
            effective[(item.variant, item.key)] = item
            if item.variant:
                ancestor = effective.pop(("", item.key), None)
                if ancestor is not None:
                    superseded.append((ancestor, item))
    # A key of the dictionary itself shadows the same key of its theme dictionaries: a lookup reads
    # the entries of a dictionary before its theme dictionaries.
    for (variant, key), item in list(effective.items()):
        if variant and ("", key) in effective:
            superseded.append((item, effective[("", key)]))
            del effective[(variant, key)]
    compact = {}
    for path in DENSITY:
        items = document_items(root, path)
        items_by_document[path] = items
        for item in items:
            compact[item.key] = item
    return Theme(documents, effective, superseded, compact, items_by_document)


# --- waivers ---------------------------------------------------------------------------------------

HERE = os.path.dirname(os.path.abspath(__file__))
WAIVERS = os.path.join(HERE, "waivers.json")


def load_waivers(path=WAIVERS):
    import json

    with open(path, encoding="utf-8") as file:
        data = json.load(file)
    waivers = []
    for entry in data["waivers"]:
        if not entry.get("reason"):
            raise SystemExit(f"error: the waiver {entry} has no reason")
        key = entry["key"]
        pattern = re.compile(key[3:]) if key.startswith("re:") else None
        waivers.append((entry["document"], key, pattern, entry.get("variant"), entry["reason"]))
    return waivers


def waiver_for(waivers, item):
    """The reason an item is waived, or None."""
    for document, key, pattern, variant, reason in waivers:
        if document not in ("*", item.document):
            continue
        if variant and variant != item.variant:
            continue
        if key == "*" or key == item.key or (pattern is not None and pattern.fullmatch(item.key)):
            return reason
    return None


# --- the port --------------------------------------------------------------------------------------

PORT_CRATE = "src/FerroUI.Themes.WinUI"
HIGH_CONTRAST_KEY = "{x:Static winui:WinUITheme.HighContrast}"
CLAIM = re.compile(r"WinUI:\s*([^\n]*?)\s*$")
REFERENCE = re.compile(r"\{(?:DynamicResource|StaticResource)\s+(?:ResourceKey=)?([^\s,}]+)\s*\}")


@dataclass
class PortDocument:
    path: str  # relative to the crate
    keys: set  # (variant, key)
    claims: set  # keys claimed by `<!-- WinUI: ... -->` comments
    references: set  # resource keys the document names
    sources: list  # the source documents its header names


def port_variant(value):
    if value is None:
        return "Default"
    if "HighContrast" in value:
        return "HighContrast"
    return VARIANT_ALIASES.get(value, value)


def port_key(element):
    key = element.get(x("Key"))
    if key is None:
        return None
    match = re.match(r"\{x:Type\s+(?:\w+:)?(\w+)\s*\}", key)
    if match:
        return "{" + match.group(1) + "}"
    return key


def read_port_document(crate_root, relative):
    path = os.path.join(crate_root, relative)
    text = open(path, encoding="utf-8-sig").read()
    tree = parse(path)
    keys = set()

    def collect(dictionary, variant):
        for child in dictionary:
            if not isinstance(child.tag, str):
                continue
            name = local_name(child.tag)
            if name == "ResourceDictionary.ThemeDictionaries":
                for theme in child:
                    if isinstance(theme.tag, str):
                        collect(theme, port_variant(theme.get(x("Key"))))
            elif name in ("ResourceDictionary.MergedDictionaries", "Styles.Resources"):
                for nested in child:
                    if isinstance(nested.tag, str) and local_name(nested.tag) == "ResourceDictionary":
                        collect(nested, variant)
            else:
                key = port_key(child)
                if key:
                    keys.add((variant, key))

    collect(tree, "")
    claims = set()
    for comment in re.findall(r"<!--(.*?)-->", text, re.S):
        for line in comment.strip().splitlines():
            match = CLAIM.match(line.strip())
            if match:
                for claimed in match.group(1).split(","):
                    if claimed.strip():
                        claims.add(claimed.strip())
    references = set(REFERENCE.findall(text)) | set(re.findall(r'ResourceKey="([^"{]+)"', text))
    header = re.match(r"\s*(?:<\?xml[^>]*>\s*)?<!--(.*?)-->", text, re.S)
    sources = re.findall(r"(?:dxaml|controls)/[\w./-]+\.xaml", header.group(1)) if header else []
    return PortDocument(relative, keys, claims, references, sources)


# The WinUI styles that crates of controls outside the theme keep themselves (the theme does not
# depend on them): the directory of their documents, relative to the repository. They count as
# ported like the documents of the theme.
EXTERNAL_PORT_DIRECTORIES = ["src/FerroUI.Controls.Inking/Themes"]


def port_documents(repository_root):
    crate_root = os.path.join(repository_root, PORT_CRATE)
    documents = []
    for directory, subdirectories, files in os.walk(crate_root):
        subdirectories[:] = sorted(d for d in subdirectories if d not in ("tests", "target") and not d.startswith("."))
        for name in sorted(files):
            if name.endswith(".xaml"):
                relative = os.path.relpath(os.path.join(directory, name), crate_root)
                documents.append(read_port_document(crate_root, relative))
    for external in EXTERNAL_PORT_DIRECTORIES:
        for directory, subdirectories, files in os.walk(os.path.join(repository_root, external)):
            subdirectories.sort()
            for name in sorted(files):
                if name.endswith(".xaml"):
                    relative = os.path.relpath(os.path.join(directory, name), crate_root)
                    documents.append(read_port_document(crate_root, relative))
    return documents
