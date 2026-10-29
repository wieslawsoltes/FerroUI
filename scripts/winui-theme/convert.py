#!/usr/bin/env python3
"""Writes the resource documents of the WinUI theme (`src/FerroUI.Themes.WinUI/Resources`) from
the theme of microsoft-ui-xaml (docs/porting/winui-theme.md, "Resources").

The resources of the theme are values: colours, brushes, aliases (`<StaticResource>`), sizes,
thicknesses, corner radii, strings, font families and weights. They are converted mechanically,
one port document per source document, so that a key is defined where WinUI defines it:

    dxaml/xcp/dxaml/themes/generic.xaml                  -> Resources/Generic.xaml
    controls/dev/CommonStyles/Button_themeresources.xaml -> Resources/Button_themeresources.xaml
    controls/dev/TreeView/TreeView.xaml                  -> Resources/TreeView.xaml

Only the effective items are written: an item that a later document of the merge replaces
(`winui_sources.merge`) is left out, so that every key is defined once in the port and its value
is the one a WinUI 3 application gets. Waived items (`waivers.json`) are left out. Styles,
templates and the other objects that belong to the control themes are not resources here: the
control documents (`Controls/*.xaml`) port them by hand. An item whose type or markup has no rule
below is reported and left out; it stays pending in the tracking (`status.py`).

The rules:

* the theme dictionaries keep their keys (`Default`, which is the dark theme of WinUI, and
  `Light`); `Dark` is merged into `Default` as the build of WinUI does; `HighContrast` is keyed
  by the variant of the theme class, `{x:Static winui:WinUITheme.HighContrast}`;
* `{ThemeResource X}` becomes `{DynamicResource X}`; `{StaticResource X}` is kept;
* `AcrylicBrush` becomes a `SolidColorBrush` of its `FallbackColor` (what WinUI draws when
  acrylic is not available);
* `LinearGradientBrush`: `MappingMode="Absolute"` keeps the points (absolute points are the
  default of FerroUI's `RelativePoint`), relative points become percentages, and a
  `RelativeTransform` becomes `Transform` with a `TransformOrigin` of its centre;
* `Visibility` becomes `x:Boolean` (`Visible` is `True`), as `Visibility` properties become
  `IsVisible`;
* `CornerRadiusFilterConverter` becomes FerroUI's `CornerRadiusFilterConverter` (the filters
  `Top`, `Bottom`, `Left`, `Right`) or `CornerRadiusToDoubleConverter` (`TopLeftValue`,
  `BottomRightValue` without a scale);
* `CornerRadiusToThicknessConverter` becomes the converter of the same name of the theme crate
  (`FerroUI.Themes.WinUI.Converters`), with its `ConversionKind` and `Multiplier`;
* the font family `XamlAutoFontFamily` becomes `Segoe UI Variable Text, Segoe UI, $Default`.

The compact density layer has one style besides its resources, the implicit `TextBlock` style
of `Compact.xaml`: a style of the density layer whose setters are plain values becomes the default
control theme of its type (`<ControlTheme x:Key="{x:Type T}" TargetType="T">` with the same
setters), the counterpart of an implicit style (docs/porting/winui-theme.md, section 4.2).

usage: convert.py [--reference <microsoft-ui-xaml>] [--check] [--report]

`--check` writes nothing and fails when a document differs from the conversion. `--report`
lists the items that are left out because no rule converts them.
"""

import argparse
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import winui_sources as w  # noqa: E402

REPOSITORY = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CRATE = os.path.join(REPOSITORY, w.PORT_CRATE)

# The objects that belong to the control themes (ported by hand in `Controls/`), not to the
# resources.
CONTROL_KINDS = {"Style", "DataTemplate", "ControlTemplate", "ItemsPanelTemplate"}

LITERAL_KINDS = {
    "Color",
    "x:Double",
    "x:String",
    "x:Boolean",
    "x:Int32",
    "Thickness",
    "CornerRadius",
    "GridLength",
    "FontWeight",
    "FontFamily",
    "HorizontalAlignment",
    "VerticalAlignment",
    "BackgroundSizing",
}

FONT_FAMILIES = {"XamlAutoFontFamily": "Segoe UI Variable Text, Segoe UI, $Default"}

VARIANT_ORDER = ["Default", "Light", "HighContrast"]

# Documents of microsoft-ui-xaml outside the merged theme whose resources FerroUI controls are built
# from (docs/porting/winui-theme.md, section 5): the tabular resources of WinUI's TableView and of
# the resize gripper of its column headers, which an application gets from the tabular library
# (TabularControlsResources), not from XamlControlsResources. They are converted whole (their keys
# are not keys of the merged theme), written after the documents of the merge and not tracked.
# The source documents whose resources belong to another crate of controls, not to the theme: the
# crate keeps its WinUI styles itself, and the theme does not depend on it. Their resources are
# written to that crate (source -> (crate, document)) and the theme does not include them; several
# sources may share one document.
EXTERNAL_DOCUMENTS = {
    "controls/dev/InkToolbar/InkToolbar.xaml": ("src/FerroUI.Controls.Inking", "Themes/WinUI/InkToolbarResources.xaml"),
    "controls/dev/InkToolbar/InkToolbar_themeresources.xaml": ("src/FerroUI.Controls.Inking", "Themes/WinUI/InkToolbarResources.xaml"),
}
EXTERNAL_NOTE = (
    "The resources of a crate of controls outside the WinUI theme, which an application includes "
    "with the styles of that crate; the theme does not include them."
)

OUTSIDE_THE_MERGE = [
    "controls/dev/CommonStyles/TabularSurfaces_themeresources.xaml",
    "controls/dev/TableView/TableView_themeresources.xaml",
    "controls/dev/ResizeGripper/ResizeGripper_themeresources.xaml",
]
OUTSIDE_THE_MERGE_NOTE = (
    "Not part of the merged theme of WinUI (the tabular library); converted for TableView and its parts."
)


class Unconvertible(Exception):
    pass


def escape(text, attribute=False):
    out = []
    for char in text:
        if char == "&":
            out.append("&amp;")
        elif char == "<":
            out.append("&lt;")
        elif char == ">":
            out.append("&gt;")
        elif attribute and char == '"':
            out.append("&quot;")
        elif ord(char) > 0x7E or (ord(char) < 0x20 and char not in "\t"):
            out.append(f"&#x{ord(char):X};")
        else:
            out.append(char)
    return "".join(out)


def convert_value(value):
    """An attribute value of WinUI markup as FerroUI markup."""
    stripped = value.strip()
    if not stripped.startswith("{") or stripped.startswith("{}"):
        return value
    match = re.fullmatch(r"\{ThemeResource\s+([^\s}]+)\s*\}", stripped)
    if match:
        return "{DynamicResource " + match.group(1) + "}"
    match = re.fullmatch(r"\{StaticResource\s+([^\s}]+)\s*\}", stripped)
    if match:
        return "{StaticResource " + match.group(1) + "}"
    if stripped == "{x:Null}":
        return stripped
    raise Unconvertible(f"markup extension {stripped}")


def attributes(element, allowed, renames=None):
    """The attributes of an element other than its key, converted; an attribute that is not in
    `allowed` makes the element unconvertible."""
    result = []
    for name, value in element.attrib.items():
        if name in (w.x("Key"), w.x("Name")):
            continue
        local = w.local_name(name) if name.startswith("{") else name
        if local not in allowed:
            raise Unconvertible(f"attribute {local}")
        result.append(((renames or {}).get(local, local), convert_value(value)))
    return result


def format_attributes(pairs):
    return "".join(f' {name}="{escape(value, True)}"' for name, value in pairs)


def key_attribute(key):
    return f' x:Key="{escape(key, True)}"'


def relative_point(text):
    parts = [p.strip() for p in text.replace(" ", ",").split(",") if p.strip()]
    if len(parts) != 2:
        raise Unconvertible(f"point {text}")

    def percent(value):
        number = float(value) * 100
        return (f"{number:.4f}".rstrip("0").rstrip(".") or "0") + "%"

    return f"{percent(parts[0])},{percent(parts[1])}"


def convert_linear_gradient(item, indent):
    element = item.element
    mapping = element.get("MappingMode", "RelativeToBoundingBox")
    pairs = []
    for name, value in element.attrib.items():
        if name in (w.x("Key"),):
            continue
        if name == "MappingMode":
            continue
        if name in ("StartPoint", "EndPoint"):
            pairs.append((name, value if mapping == "Absolute" else relative_point(value)))
        elif name in ("Opacity", "SpreadMethod"):
            pairs.append((name, convert_value(value)))
        else:
            raise Unconvertible(f"attribute {name} of LinearGradientBrush")
    if mapping == "RelativeToBoundingBox":
        pairs += [(n, relative_point(d)) for n, d in (("StartPoint", "0,0"), ("EndPoint", "1,1")) if element.get(n) is None]
    lines = [f"{indent}<LinearGradientBrush{key_attribute(item.key)}{format_attributes(pairs)}>"]
    stops = []
    transform = None
    for child in element:
        if not isinstance(child.tag, str):
            continue
        name = w.local_name(child.tag)
        if name == "LinearGradientBrush.GradientStops":
            stops += [s for s in child if isinstance(s.tag, str)]
        elif name == "GradientStop":
            stops.append(child)
        elif name == "LinearGradientBrush.RelativeTransform":
            transforms = [t for t in child if isinstance(t.tag, str)]
            if len(transforms) != 1 or w.local_name(transforms[0].tag) != "ScaleTransform":
                raise Unconvertible("RelativeTransform other than one ScaleTransform")
            transform = transforms[0]
        else:
            raise Unconvertible(f"child {name} of LinearGradientBrush")
    if transform is not None:
        center = (float(transform.get("CenterX", "0")), float(transform.get("CenterY", "0")))
        origin = relative_point(f"{center[0]},{center[1]}")
        lines[0] = lines[0][:-1] + f' TransformOrigin="{origin}">'
        scale = [(n, transform.get(n)) for n in ("ScaleX", "ScaleY") if transform.get(n) is not None]
        lines.append(f"{indent}  <LinearGradientBrush.Transform>")
        lines.append(f"{indent}    <ScaleTransform{format_attributes(scale)} />")
        lines.append(f"{indent}  </LinearGradientBrush.Transform>")
    lines.append(f"{indent}  <LinearGradientBrush.GradientStops>")
    for stop in stops:
        if w.local_name(stop.tag) != "GradientStop":
            raise Unconvertible("gradient stop of another type")
        lines.append(f"{indent}    <GradientStop{format_attributes(attributes(stop, {'Offset', 'Color'}))} />")
    lines.append(f"{indent}  </LinearGradientBrush.GradientStops>")
    lines.append(f"{indent}</LinearGradientBrush>")
    return lines, set()


CORNER_FILTERS = {
    "Top": "TopLeft, TopRight",
    "Bottom": "BottomLeft, BottomRight",
    "Left": "TopLeft, BottomLeft",
    "Right": "TopRight, BottomRight",
}
CORNER_VALUES = {"TopLeftValue": "TopLeft", "BottomRightValue": "BottomRight"}


def convert_item(item, indent):
    """The lines of one item and the XML namespaces they need."""
    element = item.element
    kind = item.kind
    if any(isinstance(child.tag, str) for child in element) and kind not in ("LinearGradientBrush",):
        raise Unconvertible(f"{kind} with child elements")
    key = key_attribute(item.key)
    text = (element.text or "")
    if kind in LITERAL_KINDS:
        attributes(element, set())
        value = text if kind == "x:String" else text.strip()
        if kind == "FontFamily":
            value = FONT_FAMILIES.get(value, value)
        return [f"{indent}<{kind}{key}>{escape(value)}</{kind}>"], set()
    if kind == "Visibility":
        attributes(element, set())
        value = {"Visible": "True", "Collapsed": "False"}.get(text.strip())
        if value is None:
            raise Unconvertible(f"visibility {text.strip()}")
        return [f"{indent}<x:Boolean{key}>{value}</x:Boolean>"], set()
    if kind == "StaticResource":
        pairs = attributes(element, {"ResourceKey"})
        return [f"{indent}<StaticResource{key}{format_attributes(pairs)} />"], set()
    if kind == "SolidColorBrush":
        pairs = attributes(element, {"Color", "Opacity"})
        if text.strip():
            return [f"{indent}<SolidColorBrush{key}{format_attributes(pairs)}>{escape(text.strip())}</SolidColorBrush>"], set()
        return [f"{indent}<SolidColorBrush{key}{format_attributes(pairs)} />"], set()
    if kind == "AcrylicBrush":
        attributes(element, {"TintColor", "TintOpacity", "TintLuminosityOpacity", "FallbackColor", "BackgroundSource", "AlwaysUseFallback"})
        fallback = element.get("FallbackColor")
        if fallback is not None:
            pairs = [("Color", convert_value(fallback))]
        else:
            pairs = [("Color", convert_value(element.get("TintColor", "Transparent")))]
            if element.get("TintOpacity") is not None:
                pairs.append(("Opacity", element.get("TintOpacity")))
        return [f"{indent}<SolidColorBrush{key}{format_attributes(pairs)} />"], set()
    if kind == "LinearGradientBrush":
        return convert_linear_gradient(item, indent)
    if kind == "CornerRadiusFilterConverter":
        attributes(element, {"Filter", "Scale"})
        filter_ = element.get("Filter")
        scale = element.get("Scale", "1")
        if filter_ in CORNER_FILTERS and scale == "1":
            return [f'{indent}<converters:CornerRadiusFilterConverter{key} Filter="{CORNER_FILTERS[filter_]}" />'], {"converters"}
        if filter_ in CORNER_VALUES and scale == "1":
            return [f'{indent}<converters:CornerRadiusToDoubleConverter{key} Corner="{CORNER_VALUES[filter_]}" />'], {"converters"}
        raise Unconvertible(f"CornerRadiusFilterConverter Filter={filter_} Scale={scale}")
    if kind == "CornerRadiusToThicknessConverter":
        pairs = attributes(element, {"ConversionKind", "Multiplier"})
        if any(value.startswith("{") for _, value in pairs):
            raise Unconvertible("CornerRadiusToThicknessConverter with a markup extension")
        return [f"{indent}<winuiconverters:CornerRadiusToThicknessConverter{key}{format_attributes(pairs)} />"], {"winuiconverters"}
    raise Unconvertible(f"type {kind}")


def convert_density_style(item, indent):
    """An implicit style of the density layer as the default control theme of its type: only a
    style without a key, a base or a template, whose setters set plain values, is converted."""
    element = item.element
    target = element.get("TargetType")
    if not target or element.get(w.x("Key")) is not None or element.get("BasedOn") is not None:
        raise Unconvertible("density style that is not a plain implicit style")
    attributes(element, {"TargetType"})
    setters = []
    for child in element:
        if not isinstance(child.tag, str):
            continue
        if w.local_name(child.tag) != "Setter":
            raise Unconvertible(f"child {w.local_name(child.tag)} of a density style")
        pairs = attributes(child, {"Property", "Value"})
        if any(name == "Value" and value.startswith("{") for name, value in pairs) or len(pairs) != 2:
            raise Unconvertible("density style setter that is not a plain value")
        setters.append(f"{indent}  <Setter{format_attributes(pairs)} />")
    if not setters:
        raise Unconvertible("density style without setters")
    lines = [f'{indent}<ControlTheme x:Key="{{x:Type {target}}}" TargetType="{escape(target, True)}">']
    lines += setters
    lines.append(f"{indent}</ControlTheme>")
    return lines, set()


NAMESPACES = {
    "winui": 'xmlns:winui="using:FerroUI.Themes.WinUI"',
    "converters": 'xmlns:converters="using:FerroUI.Controls.Converters"',
    "winuiconverters": 'xmlns:winuiconverters="using:FerroUI.Themes.WinUI.Converters"',
}


def port_path(source):
    if source == w.GENERIC:
        return "Resources/Generic.xaml"
    return "Resources/" + os.path.basename(source)


def header(sources, note=None):
    lines = ["<!--"]
    lines.append("  Ported from microsoft-ui-xaml (https://github.com/microsoft/microsoft-ui-xaml),")
    lines.append(f"  commit {w.REFERENCE_COMMIT}:")
    for source in sources:
        lines.append(f"    {source}")
    lines.append("  Copyright (c) Microsoft Corporation. Licensed under the MIT License (see NOTICE.md of the crate).")
    lines.append("  Generated by scripts/winui-theme/convert.py: do not edit; change the converter or")
    lines.append("  scripts/winui-theme/waivers.json and run it again (docs/porting/winui-theme.md).")
    if note:
        lines.append(f"  {note}")
    lines.append("-->")
    return lines


def render(sources, items, note=None):
    """A resource dictionary document of `items` (all of one source document, or of the density
    layer)."""
    namespaces = set()
    themed = {}
    plain = []
    for item, lines, needed in items:
        namespaces |= needed
        if item.variant:
            themed.setdefault(item.variant, []).append(lines)
        else:
            plain.append(lines)
    if themed and "HighContrast" in themed:
        namespaces.add("winui")
    body = []
    if themed:
        body.append("  <ResourceDictionary.ThemeDictionaries>")
        for variant in VARIANT_ORDER + sorted(set(themed) - set(VARIANT_ORDER)):
            if variant not in themed:
                continue
            key = "{x:Static winui:WinUITheme.HighContrast}" if variant == "HighContrast" else variant
            body.append(f'    <ResourceDictionary x:Key="{key}">')
            for lines in themed[variant]:
                body += lines
            body.append("    </ResourceDictionary>")
        body.append("  </ResourceDictionary.ThemeDictionaries>")
    for lines in plain:
        body += lines
    root = ['<ResourceDictionary xmlns="https://github.com/ferroui"', '                    xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"']
    for name in sorted(namespaces):
        root.append(f"                    {NAMESPACES[name]}")
    root.append('                    x:ClassModifier="internal">')
    return "\n".join(header(sources, note) + root + body + ["</ResourceDictionary>", ""])


def convert(root):
    theme = w.merge(root)
    waivers = w.load_waivers()
    effective_ids = {id(item) for item in theme.effective.values()}
    documents = {}
    external = {}
    left_out = []
    for source, _layer in theme.documents:
        converted = []
        for item in theme.items_by_document[source]:
            if id(item) not in effective_ids or item.kind in CONTROL_KINDS or w.waiver_for(waivers, item):
                continue
            indent = "      " if item.variant else "  "
            try:
                lines, needed = convert_item(item, indent)
            except Unconvertible as error:
                left_out.append((item, str(error)))
                continue
            converted.append((item, lines, needed))
        if source in EXTERNAL_DOCUMENTS:
            sources, items = external.setdefault(EXTERNAL_DOCUMENTS[source], ([], []))
            sources.append(source)
            items.extend(converted)
            continue
        if converted:
            path = port_path(source)
            if path in documents:
                raise SystemExit(f"error: two source documents map to {path}")
            documents[path] = render([source], converted)

    # The documents outside the merged theme that FerroUI controls are built from (every item of
    # them; they define no key of the merged theme).
    for source in OUTSIDE_THE_MERGE:
        converted = []
        for item in w.document_items(root, source):
            if item.kind in CONTROL_KINDS or w.waiver_for(waivers, item):
                continue
            indent = "      " if item.variant else "  "
            try:
                lines, needed = convert_item(item, indent)
            except Unconvertible as error:
                left_out.append((item, str(error)))
                continue
            converted.append((item, lines, needed))
        path = port_path(source)
        if path in documents:
            raise SystemExit(f"error: two source documents map to {path}")
        documents[path] = render([source], converted, OUTSIDE_THE_MERGE_NOTE)

    # The density layer: the resources of the compact dictionary of WinUI, and its implicit
    # styles as default control themes.
    converted = []
    for key, item in theme.compact.items():
        if w.waiver_for(waivers, item) or (item.kind in CONTROL_KINDS and item.kind != "Style"):
            continue
        try:
            if item.kind == "Style":
                lines, needed = convert_density_style(item, "  ")
            else:
                lines, needed = convert_item(item, "  ")
        except Unconvertible as error:
            left_out.append((item, str(error)))
            continue
        converted.append((item, lines, needed))
    documents["DensityStyles/Compact.xaml"] = render(
        w.DENSITY,
        converted,
        "The resources of WinUI's compact density (compact_themeresources.xaml: the theme resources with these documents merged last), with its implicit TextBlock style as the default control theme of TextBlock; WinUITheme looks them up first when its DensityStyle is Compact.",
    )

    # The includes of the theme document: the system resources, the converted documents in the
    # order of the merge of WinUI, the theme shadows and the outlines of the glyphs.
    order = ["Resources/SystemResources.xaml"]
    order += [port_path(source) for source, _ in theme.documents if port_path(source) in documents]
    order += [port_path(source) for source in OUTSIDE_THE_MERGE]
    order += ["Resources/ThemeShadows.xaml", "Resources/Glyphs.xaml"]
    theme_path = os.path.join(CRATE, THEME_DOCUMENT)
    text = open(theme_path, encoding="utf-8").read()
    begin, end = text.find(INCLUDES_BEGIN), text.find(INCLUDES_END)
    if begin < 0 or end < begin:
        raise SystemExit(f"error: {THEME_DOCUMENT} has no {INCLUDES_BEGIN} ... {INCLUDES_END} markers")
    line_start = text.rfind("\n", 0, begin) + 1
    indent = text[line_start:begin]
    includes = "".join(f'{indent}<MergeResourceInclude Source="/{path}" />\n' for path in order)
    documents[THEME_DOCUMENT] = text[: begin + len(INCLUDES_BEGIN)] + "\n" + includes + indent + text[end:]
    external_documents = {
        (crate, path): render(sources, items, EXTERNAL_NOTE) for (crate, path), (sources, items) in external.items()
    }
    return documents, external_documents, left_out


THEME_DOCUMENT = "WinUITheme.xaml"
INCLUDES_BEGIN = "<!-- resources:begin (written by scripts/winui-theme/convert.py; do not edit) -->"
INCLUDES_END = "<!-- resources:end -->"


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--reference", help="the checkout of microsoft-ui-xaml")
    parser.add_argument("--check", action="store_true", help="write nothing; fail when a document is not current")
    parser.add_argument("--report", action="store_true", help="list the items no rule converts")
    arguments = parser.parse_args()
    root = w.reference_root(arguments.reference)
    documents, external_documents, left_out = convert(root)

    stale = []
    outputs = [(os.path.join(CRATE, path), f"{w.PORT_CRATE}/{path}", text) for path, text in documents.items()]
    outputs += [
        (os.path.join(REPOSITORY, crate, path), f"{crate}/{path}", text) for (crate, path), text in external_documents.items()
    ]
    for target, path, text in sorted(outputs):
        current = open(target, encoding="utf-8").read() if os.path.exists(target) else None
        if current == text:
            continue
        if arguments.check:
            stale.append(path)
        else:
            os.makedirs(os.path.dirname(target), exist_ok=True)
            with open(target, "w", encoding="utf-8") as file:
                file.write(text)
            print(f"wrote {path}")
    resources = os.path.join(CRATE, "Resources")
    if os.path.isdir(resources):
        generated = {os.path.basename(p) for p in documents if p.startswith("Resources/")}
        for name in sorted(os.listdir(resources)):
            if name.endswith(".xaml") and name not in generated and not name.startswith("System"):
                open_text = open(os.path.join(resources, name), encoding="utf-8").read()
                if "Generated by scripts/winui-theme/convert.py" in open_text:
                    stale.append(f"Resources/{name} (no longer generated)")
    if arguments.report or not arguments.check:
        print(f"{len(documents)} documents; {len(left_out)} items left out (no conversion rule)")
    if arguments.report:
        for item, reason in sorted(left_out, key=lambda pair: (pair[0].document, pair[0].key)):
            print(f"  {item.document} {item.ident}: {reason}")
    if stale:
        print("stale: " + ", ".join(stale), file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
