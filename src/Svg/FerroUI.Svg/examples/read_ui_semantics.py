#!/usr/bin/env python3
"""Validate FerroUI's version-1 UI metadata and produce a portable JSON sidecar.

Usage: python3 read_ui_semantics.py input.svg [--output ui.json]

Only Python's standard library is required. This reads the inert metadata vocabulary,
not SVG painting commands: it never loads images, source URIs, or external resources.
Layer and node arrays retain export order; object keys are sorted in JSON. Node `id`
is its SVG identifier, `key` is its original UI key, and relationships contain target
SVG identifiers. Attribute names become snake_case; geometry arrays keep their stated
order. Layout width/height `auto` and maximum `unbounded` become JSON null. Custom
properties retain both their declared `type` and typed `value`. Missing optional
fields stay absent, so excluded content is not reconstructed from painted text.

This deliberately accepts only the published export vocabulary. Unknown versions,
fields, ambiguous IDs, invalid values and unresolved links are errors. DTDs are
forbidden; input size, XML nesting and element counts are bounded before indexing.
It is not a general-purpose SVG validator or a claim of per-path UI ownership.
"""

from __future__ import annotations

import argparse
import json
import math
from pathlib import Path
import re
import sys
import xml.etree.ElementTree as ET


SVG_NS = "http://www.w3.org/2000/svg"
UI_NS = "https://wieslawsoltes.github.io/FerroUI/svg/semantics/1"
UI_PREFIX = "https://wieslawsoltes.github.io/FerroUI/svg/semantics/"
MATRIX_ORDER = "m11 m12 m13 m21 m22 m23 m31 m32 m33".split()
MAX_INPUT_BYTES = 128 * 1024 * 1024
MAX_XML_ELEMENTS = 2_000_000
MAX_XML_DEPTH = 256
MAX_NODES = 100_000
MAX_KEY_BYTES = 4096
MAX_TEXT_BYTES = 16 * 1024 * 1024
MAX_VALUES = 500_000
INTEGER = re.compile(r"-?[0-9]+\Z")
NUMBER = re.compile(r"[+-]?(?:[0-9]+(?:\.[0-9]*)?|\.[0-9]+)(?:[eE][+-]?[0-9]+)?\Z")
KEY = re.compile(r"[A-Za-z0-9_.-]+\Z")
LAYER_KEY = re.compile(r"[A-Za-z0-9_-]+\Z")


class SemanticError(ValueError):
    """The input cannot be interpreted unambiguously as a supported UI snapshot."""


class _BoundedTreeBuilder(ET.TreeBuilder):
    def __init__(self) -> None:
        super().__init__()
        self.depth = 0
        self.elements = 0

    def start(self, tag: str, attrs: dict[str, str]) -> ET.Element:
        self.depth += 1
        self.elements += 1
        if self.depth > MAX_XML_DEPTH or self.elements > MAX_XML_ELEMENTS:
            raise SemanticError("XML nesting or element budget exceeded")
        return super().start(tag, attrs)

    def end(self, tag: str) -> ET.Element:
        result = super().end(tag)
        self.depth -= 1
        return result

    def doctype(self, _name: str, _public_id: str, _system_id: str) -> None:
        raise SemanticError("DTDs and entity declarations are not supported")


def _attrs(element: ET.Element, required: tuple[str, ...] = (),
           optional: tuple[str, ...] = ()) -> dict[str, str]:
    actual = element.attrib.keys()
    if set(required) - actual or actual - set(required) - set(optional):
        raise SemanticError(f"invalid attributes on {element.tag}")
    return element.attrib


def _children(element: ET.Element, allowed: tuple[str, ...] = (),
              repeated: tuple[str, ...] = ()) -> dict[str, list[ET.Element]]:
    if (element.text or "").strip():
        raise SemanticError(f"unexpected text in {element.tag}")
    result: dict[str, list[ET.Element]] = {}
    for child in element:
        name = child.tag.removeprefix(f"{{{UI_NS}}}")
        if child.tag != f"{{{UI_NS}}}{name}" or name not in allowed:
            raise SemanticError(f"unsupported semantic element {child.tag}")
        if name in result and name not in repeated:
            raise SemanticError(f"duplicate semantic element {name}")
        if (child.tail or "").strip():
            raise SemanticError(f"unexpected text after {child.tag}")
        result.setdefault(name, []).append(child)
    return result


def _text(element: ET.Element, *, attributes: bool = False) -> str:
    if len(element) or (element.attrib and not attributes):
        raise SemanticError(f"expected plain text in {element.tag}")
    return element.text or ""


def _integer(value: str, minimum: int = -(2**31), maximum: int = 2**31 - 1) -> int:
    if not INTEGER.fullmatch(value) or len(value) > 20:
        raise SemanticError(f"invalid integer {value!r}")
    result = int(value)
    if not minimum <= result <= maximum:
        raise SemanticError(f"integer outside supported range: {value!r}")
    return result


def _number(value: str, *, nonnegative: bool = False) -> float:
    if not NUMBER.fullmatch(value):
        raise SemanticError(f"invalid number {value!r}")
    result = float(value)
    if not math.isfinite(result) or (nonnegative and result < 0):
        raise SemanticError(f"number outside supported range: {value!r}")
    return result


def _boolean(value: str) -> bool:
    if value not in ("true", "false"):
        raise SemanticError(f"invalid boolean {value!r}")
    return value == "true"


def _numbers(value: str, length: int, *, nonnegative: bool = False) -> list[float]:
    values = value.split()
    if len(values) != length:
        raise SemanticError(f"expected {length} numeric components")
    return [_number(item, nonnegative=nonnegative) for item in values]


def _reference(element: ET.Element) -> str:
    href = _attrs(element, ("href",))["href"]
    _children(element)
    if not href.startswith("#") or len(href) == 1 or any(c.isspace() for c in href):
        raise SemanticError("semantic links must be nonempty local fragment references")
    return href[1:]


def _geometry(element: ET.Element) -> dict:
    attrs = _attrs(element, ("layout-space", "layout-bounds", "local-bounds",
                             "matrix-order", "local-to-layer"))
    _children(element)
    if attrs["layout-space"] != "parent-layout" or attrs["matrix-order"].split() != MATRIX_ORDER:
        raise SemanticError("unsupported semantic geometry coordinate convention")
    bounds = _numbers(attrs["layout-bounds"], 4)
    local = _numbers(attrs["local-bounds"], 4)
    if (min(bounds[2:]) < 0 or local != [0, 0, *bounds[2:]]
            or not all(math.isfinite(a + b) for a, b in zip(bounds[:2], bounds[2:]))):
        raise SemanticError("invalid or inconsistent semantic layout bounds")
    return {"layout_space": "parent-layout", "layout_bounds": bounds,
            "local_bounds": local, "matrix_order": MATRIX_ORDER.copy(),
            "local_to_layer": _numbers(attrs["local-to-layer"], 9)}


def _state(element: ET.Element, *, input_state: bool = False) -> dict:
    booleans = (("enabled", "effectively-enabled", "focusable", "focused", "hit-test-visible",
                 "tab-stop") if input_state else
                ("visible", "effectively-visible", "render-suppressed", "clip-to-bounds",
                 "has-clip", "has-opacity-mask", "has-effect"))
    fields = (*booleans, "tab-index") if input_state else (*booleans, "opacity", "z-index")
    attrs = _attrs(element, optional=fields) if input_state else _attrs(element, fields)
    _children(element)
    result = {}
    for name, value in attrs.items():
        if name in booleans:
            result[name.replace("-", "_")] = _boolean(value)
        elif name == "opacity":
            opacity = _number(value, nonnegative=True)
            if opacity > 1:
                raise SemanticError("opacity must be between zero and one")
            result["opacity"] = opacity
        else:
            result[name.replace("-", "_")] = _integer(value)
    return result


def _layout(element: ET.Element) -> dict:
    attrs = _attrs(element, ("desired-size", "margin", "width", "height", "max-width",
                             "max-height", "min-width", "min-height", "horizontal-alignment",
                             "vertical-alignment", "measure-valid", "arrange-valid"))
    children = _children(element, ("grid",))
    result = {"desired_size": _numbers(attrs["desired-size"], 2, nonnegative=True),
              "margin": _numbers(attrs["margin"], 4)}
    for name in ("width", "height", "max-width", "max-height", "min-width", "min-height"):
        missing = "auto" if name in ("width", "height") else "unbounded" if name.startswith("max-") else None
        result[name.replace("-", "_")] = None if attrs[name] == missing else _number(attrs[name], nonnegative=True)
    for name in ("horizontal-alignment", "vertical-alignment"):
        result[name.replace("-", "_")] = attrs[name]
    for name in ("measure-valid", "arrange-valid"):
        result[name.replace("-", "_")] = _boolean(attrs[name])
    if "grid" in children:
        grid = children["grid"][0]
        grid_attrs = _attrs(grid, ("row", "column", "row-span", "column-span"))
        _children(grid)
        result["grid"] = {name.replace("-", "_"): _integer(value, 1 if name.endswith("-span") else 0)
                          for name, value in grid_attrs.items()}
    return result


def _accessibility(element: ET.Element) -> dict:
    attrs = _attrs(element, optional=("required", "heading-level", "position-in-set", "size-of-set"))
    children = _children(element, ("automation-id", "name", "help-text", "role", "landmark-type", "live-setting"))
    result = {name.replace("-", "_"): _boolean(value) if name == "required" else _integer(value)
              for name, value in attrs.items()}
    result.update((name.replace("-", "_"), _text(values[0])) for name, values in children.items())
    return result


def _properties(element: ET.Element) -> dict:
    _attrs(element)
    children = _children(element, ("property",), ("property",))
    result = {}
    for item in children.get("property", []):
        attrs = _attrs(item, ("name", "type"))
        name, kind = attrs["name"], attrs["type"]
        if not name or name in result:
            raise SemanticError("empty or duplicate semantic property name")
        if kind == "string-list":
            values = _children(item, ("item",), ("item",))
            value = [_text(child) for child in values.get("item", [])]
        elif kind in ("string", "number", "boolean"):
            value = _text(item, attributes=True)
            if kind == "number":
                value = _number(value)
            elif kind == "boolean":
                value = _boolean(value)
        else:
            raise SemanticError(f"unsupported semantic property type {kind!r}")
        result[name] = {"type": kind, "value": value}
    return result


def _node(element: ET.Element, layer_key: str) -> dict:
    attrs = _attrs(element, ("id", "key", "type"))
    key = attrs["key"]
    if not KEY.fullmatch(key) or len(key) > MAX_KEY_BYTES or not attrs["type"]:
        raise SemanticError("invalid source UI key or empty type name")
    if attrs["id"] != f"ferro-internal-ui-{len(layer_key)}-{layer_key}-{key}":
        raise SemanticError("semantic node identifier does not match its layer and source key")
    fields = _children(element, ("parent", "logical-parent", "templated-parent", "name", "classes",
                                 "geometry", "state", "input", "layout", "accessibility", "text",
                                 "source", "properties"))
    if not {"geometry", "state"} <= fields.keys():
        raise SemanticError("semantic nodes require geometry and state")
    result = dict(attrs)
    for name, items in fields.items():
        child = items[0]
        output_name = name.replace("-", "_")
        if name in ("parent", "logical-parent", "templated-parent"):
            value = _reference(child)
        elif name in ("name", "text"):
            value = _text(child)
        elif name == "classes":
            _attrs(child)
            classes = _children(child, ("class",), ("class",))
            value = [_text(item) for item in classes.get("class", [])]
        elif name == "source":
            source = _attrs(child, ("uri",), ("line", "column"))
            _children(child)
            value = {field: text if field == "uri" else _integer(text, 1, 2**32 - 1)
                     for field, text in source.items()}
        elif name == "input":
            value = _state(child, input_state=True)
        else:
            value = {"geometry": _geometry, "state": _state, "layout": _layout,
                     "accessibility": _accessibility, "properties": _properties}[name](child)
        result[output_name] = value
    return result


def _validate_relations_and_budget(nodes: list[dict]) -> None:
    ids = {node["id"]: index for index, node in enumerate(nodes)}
    if len(ids) != len(nodes) or len({node["key"] for node in nodes}) != len(nodes):
        raise SemanticError("duplicate semantic node ID or source key")
    text_bytes = values = 0

    def observe(value: str) -> None:
        nonlocal text_bytes, values
        text_bytes += len(value.encode("utf-8"))
        values += 1
        if text_bytes > MAX_TEXT_BYTES or values > MAX_VALUES:
            raise SemanticError("semantic text or value budget exceeded")

    for index, node in enumerate(nodes):
        for relation in ("parent", "logical_parent", "templated_parent"):
            if relation in node:
                target = ids.get(node[relation])
                if target is None or (relation == "parent" and target >= index):
                    raise SemanticError("unresolved relationship or visual parent not preceding its child")
                observe(nodes[target]["key"])
        for field in ("key", "type", "name", "text"):
            if field in node:
                observe(node[field])
        for item in node.get("classes", []):
            observe(item)
        if "layout" in node:
            observe(node["layout"]["horizontal_alignment"])
            observe(node["layout"]["vertical_alignment"])
        for value in node.get("accessibility", {}).values():
            if isinstance(value, str):
                observe(value)
        if "source" in node:
            observe(node["source"]["uri"])
        for name, item in node.get("properties", {}).items():
            observe(name)
            if item["type"] == "string":
                observe(item["value"])
            elif item["type"] == "string-list":
                for value in item["value"]:
                    observe(value)
    for relation in ("logical_parent", "templated_parent"):
        marks = bytearray(len(nodes))
        for start in range(len(nodes)):
            index = start
            path = []
            while index is not None and marks[index] != 2:
                if marks[index] == 1:
                    raise SemanticError(f"cyclic {relation} ancestry")
                marks[index] = 1
                path.append(index)
                index = ids.get(nodes[index].get(relation))
            for index in path:
                marks[index] = 2


def _summary(group: ET.Element, nodes: list[dict]) -> None:
    actual = {name: value for name, value in group.attrib.items() if name.startswith("data-ui-")}
    if not actual:
        return
    expected = {"data-ui-schema": UI_NS, "data-ui-node-count": str(len(nodes))}
    roots = [node for node in nodes if "parent" not in node]
    if len(roots) == 1:
        root = roots[0]
        expected.update({"data-ui-root": root["id"], "data-ui-type": root["type"]})
        if "name" in root:
            expected["data-ui-name"] = root["name"]
        for field in ("automation_id", "role"):
            if field in root.get("accessibility", {}):
                expected["data-ui-" + field.replace("_", "-")] = root["accessibility"][field]
    if not {"data-ui-schema", "data-ui-node-count"} <= actual.keys() or any(
            expected.get(name) != value for name, value in actual.items()):
        raise SemanticError("layer summary attributes disagree with semantic metadata")


def parse_ui_semantics(data: bytes) -> dict:
    """Parse SVG bytes, validate the supported vocabulary, and return a JSON-ready manifest."""
    if len(data) > MAX_INPUT_BYTES:
        raise SemanticError("SVG exceeds the input byte budget")
    try:
        root = ET.fromstring(data, parser=ET.XMLParser(target=_BoundedTreeBuilder()))
    except ET.ParseError as error:
        raise SemanticError(f"invalid SVG XML: {error}") from error
    if root.tag != f"{{{SVG_NS}}}svg":
        raise SemanticError("the document root must be an SVG element")
    ids: dict[str, ET.Element] = {}
    parents = {}
    trees = []
    semantic_elements = set()
    annotated_groups = set()
    for element in root.iter():
        for child in element:
            parents[child] = element
        if "id" in element.attrib:
            identifier = element.attrib["id"]
            if not identifier or any(c.isspace() for c in identifier) or identifier in ids:
                raise SemanticError("empty, invalid, or duplicate SVG identifier")
            ids[identifier] = element
        if element.tag.startswith("{" + UI_PREFIX):
            if not element.tag.startswith("{" + UI_NS + "}"):
                raise SemanticError("unsupported UI semantic namespace version")
            semantic_elements.add(element)
            if element.tag == f"{{{UI_NS}}}tree":
                trees.append(element)
        if any(name.startswith("data-ui-") for name in element.attrib):
            annotated_groups.add(element)

    layers, groups, parsed_elements = [], set(), set()
    for tree in trees:
        attrs = _attrs(tree, ("schema-version", "hierarchy", "coordinate-space", "association", "node-count"))
        if [attrs[name] for name in ("schema-version", "hierarchy", "coordinate-space", "association")] != [
                "1", "visual", "layer-local", "layer"]:
            raise SemanticError("unsupported UI schema version, hierarchy, coordinates, or association")
        count = _integer(attrs["node-count"], 0, MAX_NODES)
        fields = _children(tree, ("rendering", "node"), ("node",))
        if "rendering" not in fields or len(fields.get("node", [])) != count:
            raise SemanticError("missing rendering association or incorrect semantic node count")
        layer_id = _reference(fields["rendering"][0])
        group = ids.get(layer_id)
        metadata = parents.get(tree)
        if (group is None or group.tag != f"{{{SVG_NS}}}g" or metadata is None
                or metadata.tag != f"{{{SVG_NS}}}metadata" or parents.get(metadata) is not group
                or group in groups):
            raise SemanticError("semantic tree must belong to its uniquely referenced SVG layer")
        layer_key = layer_id.removeprefix("ferro-")
        if (layer_id != "ferro-" + layer_key or not LAYER_KEY.fullmatch(layer_key)
                or layer_key.startswith("internal-")):
            raise SemanticError("invalid exported semantic layer ID")
        nodes = [_node(element, layer_key) for element in fields.get("node", [])]
        _validate_relations_and_budget(nodes)
        _summary(group, nodes)
        groups.add(group)
        parsed_elements.update(tree.iter())
        layers.append({"id": layer_id, "rendering": "#" + layer_id, "hierarchy": "visual",
                       "coordinate_space": "layer-local", "association": "layer",
                       "node_count": count, "nodes": nodes})
    if semantic_elements != parsed_elements or annotated_groups - groups:
        raise SemanticError("orphaned semantic elements or summary without a semantic tree")
    return {"schema": UI_NS, "schema_version": 1, "layers": layers}


def read_ui_semantics(path: Path) -> dict:
    """Read one local file. External URLs and embedded resource links are never opened."""
    with path.open("rb") as stream:
        return parse_ui_semantics(stream.read(MAX_INPUT_BYTES + 1))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("svg", type=Path)
    parser.add_argument("--output", type=Path, help="write JSON here instead of stdout")
    args = parser.parse_args(argv)
    try:
        manifest = read_ui_semantics(args.svg)
        result = json.dumps(manifest, ensure_ascii=False, allow_nan=False, sort_keys=True, indent=2) + "\n"
        if args.output is None:
            sys.stdout.write(result)
        else:
            args.output.write_text(result, encoding="utf-8", newline="\n")
    except (OSError, SemanticError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
