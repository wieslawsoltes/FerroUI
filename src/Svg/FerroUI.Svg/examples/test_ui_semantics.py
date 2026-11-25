"""Independent version-1 UI metadata consumer tests; no third-party dependencies."""

from __future__ import annotations

import contextlib
import copy
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import xml.etree.ElementTree as ET

import read_ui_semantics as reader


def ui(name: str) -> str:
    return f"{{{reader.UI_NS}}}{name}"


def svg(name: str) -> str:
    return f"{{{reader.SVG_NS}}}{name}"


def identifier(key: str, layer: str = "ui") -> str:
    return f"ferro-internal-ui-{len(layer)}-{layer}-{key}"


def make_node(key: str, *, parent: str | None = None, layer: str = "ui") -> ET.Element:
    node = ET.Element(ui("node"), {"id": identifier(key, layer), "key": key,
                                   "type": "FerroUI.Controls.Button"})
    if parent is not None:
        ET.SubElement(node, ui("parent"), {"href": "#" + identifier(parent, layer)})
    ET.SubElement(node, ui("geometry"), {
        "layout-space": "parent-layout", "layout-bounds": "10 20 80 30", "local-bounds": "0 0 80 30",
        "matrix-order": " ".join(reader.MATRIX_ORDER), "local-to-layer": "1 0 0 0 1 0 10 20 1",
    })
    ET.SubElement(node, ui("state"), {
        "visible": "true", "effectively-visible": "true", "render-suppressed": "false", "opacity": "0.75",
        "z-index": "-2", "clip-to-bounds": "true", "has-clip": "false", "has-opacity-mask": "false",
        "has-effect": "false",
    })
    return node


def make_layer(nodes: list[ET.Element], *, key: str = "ui") -> ET.Element:
    group = ET.Element(svg("g"), {"id": "ferro-" + key})
    tree = ET.SubElement(ET.SubElement(group, svg("metadata")), ui("tree"), {
        "schema-version": "1", "hierarchy": "visual", "coordinate-space": "layer-local",
        "association": "layer", "node-count": str(len(nodes)),
    })
    ET.SubElement(tree, ui("rendering"), {"href": "#ferro-" + key})
    tree.extend(nodes)
    return group


def document(*groups: ET.Element) -> ET.Element:
    root = ET.Element(svg("svg"), {"viewBox": "0 0 300 200"})
    root.extend(groups)
    return root


def parse(root: ET.Element) -> dict:
    return reader.parse_ui_semantics(ET.tostring(root, encoding="utf-8"))


class UiSemanticsTests(unittest.TestCase):
    def test_all_fields_have_portable_typed_json_and_distinct_relations(self) -> None:
        root, child, other = make_node("root"), make_node("save", parent="root"), make_node("other", parent="root")
        ET.SubElement(child, ui("logical-parent"), {"href": "#" + identifier("other")})
        ET.SubElement(child, ui("templated-parent"), {"href": "#" + identifier("root")})
        ET.SubElement(child, ui("name")).text = 'Save & "Publish" <工作>'
        classes = ET.SubElement(child, ui("classes"))
        for value in ("primary", ":focus", ""):
            ET.SubElement(classes, ui("class")).text = value
        ET.SubElement(child, ui("input"), {"enabled": "true", "focused": "false", "tab-index": "7"})
        layout = ET.SubElement(child, ui("layout"), {
            "desired-size": "80 30", "margin": "-2 4 5 6", "width": "auto", "height": "30",
            "max-width": "unbounded", "max-height": "100", "min-width": "0", "min-height": "20",
            "horizontal-alignment": "Stretch", "vertical-alignment": "Center", "measure-valid": "true",
            "arrange-valid": "false",
        })
        ET.SubElement(layout, ui("grid"), {"row": "2", "column": "0", "row-span": "1", "column-span": "3"})
        accessibility = ET.SubElement(child, ui("accessibility"), {
            "required": "false", "heading-level": "2", "position-in-set": "1", "size-of-set": "3",
        })
        for name, value in (("automation-id", "save"), ("name", "Save changes"), ("help-text", "Write\nthe document"),
                            ("role", "Button"), ("landmark-type", "Form"), ("live-setting", "Polite")):
            ET.SubElement(accessibility, ui(name)).text = value
        ET.SubElement(child, ui("text")).text = "  Caption\nwith whitespace  "
        ET.SubElement(child, ui("source"), {"uri": "../views/Settings.fxml", "line": "42", "column": "9"})
        properties = ET.SubElement(child, ui("properties"))
        for name, kind, value in (("token", "string", "Accent & surface"), ("elevation", "number", "1.25e2"),
                                  ("selected", "boolean", "false"), ("empty", "string", "")):
            ET.SubElement(properties, ui("property"), {"name": name, "type": kind}).text = value
        tags = ET.SubElement(properties, ui("property"), {"name": "tags", "type": "string-list"})
        for value in ("design", "", "é"):
            ET.SubElement(tags, ui("item")).text = value
        group = make_layer([root, child, other])
        group.set("data-ui-schema", reader.UI_NS)
        group.set("data-ui-node-count", "3")
        group.set("data-ui-root", identifier("root"))
        group.set("data-ui-type", "FerroUI.Controls.Button")
        manifest = parse(document(group))
        self.assertEqual(reader.UI_NS, manifest["schema"])
        self.assertEqual(1, manifest["schema_version"])
        layer = manifest["layers"][0]
        self.assertEqual("#ferro-ui", layer["rendering"])
        self.assertEqual("layer-local", layer["coordinate_space"])
        self.assertEqual(3, layer["node_count"])
        node = layer["nodes"][1]
        self.assertEqual(identifier("save"), node["id"])
        self.assertEqual("save", node["key"])
        self.assertEqual(identifier("root"), node["parent"])
        self.assertEqual(identifier("other"), node["logical_parent"])
        self.assertEqual(identifier("root"), node["templated_parent"])
        self.assertEqual('Save & "Publish" <工作>', node["name"])
        self.assertEqual(["primary", ":focus", ""], node["classes"])
        self.assertEqual([10, 20, 80, 30], node["geometry"]["layout_bounds"])
        self.assertEqual([1, 0, 0, 0, 1, 0, 10, 20, 1], node["geometry"]["local_to_layer"])
        self.assertEqual(reader.MATRIX_ORDER, node["geometry"]["matrix_order"])
        self.assertEqual(False, node["state"]["has_effect"])
        self.assertEqual(-2, node["state"]["z_index"])
        self.assertEqual({"enabled": True, "focused": False, "tab_index": 7}, node["input"])
        self.assertIsNone(node["layout"]["width"])
        self.assertIsNone(node["layout"]["max_width"])
        self.assertEqual({"row": 2, "column": 0, "row_span": 1, "column_span": 3}, node["layout"]["grid"])
        self.assertEqual("Write\nthe document", node["accessibility"]["help_text"])
        self.assertIs(node["accessibility"]["required"], False)
        self.assertEqual("  Caption\nwith whitespace  ", node["text"])
        self.assertEqual({"uri": "../views/Settings.fxml", "line": 42, "column": 9}, node["source"])
        self.assertEqual({"type": "number", "value": 125.0}, node["properties"]["elevation"])
        self.assertEqual({"type": "boolean", "value": False}, node["properties"]["selected"])
        self.assertEqual({"type": "string-list", "value": ["design", "", "é"]}, node["properties"]["tags"])
        self.assertEqual("", node["properties"]["empty"]["value"])
        self.assertEqual(manifest, json.loads(json.dumps(manifest, allow_nan=False)))

    def test_optional_fields_stay_absent_and_no_painted_content_is_inferred(self) -> None:
        group = make_layer([make_node("root")])
        ET.SubElement(group, svg("text")).text = "Private painted text"
        ET.SubElement(group, svg("image"), {"href": "https://invalid.example/no-fetch.png"})
        node = parse(document(group))["layers"][0]["nodes"][0]
        self.assertFalse({"text", "name", "accessibility", "source", "input", "layout", "properties"} & node.keys())

    def test_multiple_layers_forests_empty_trees_and_plain_svg(self) -> None:
        first = make_layer([make_node("root"), make_node("independent")])
        second = make_layer([make_node("root", layer="overlay")], key="overlay")
        empty = make_layer([], key="empty")
        result = parse(document(first, second, empty))
        self.assertEqual(["ferro-ui", "ferro-overlay", "ferro-empty"], [layer["id"] for layer in result["layers"]])
        self.assertEqual([2, 1, 0], [layer["node_count"] for layer in result["layers"]])
        self.assertEqual([], parse(document())["layers"])

    def test_foreign_namespaces_and_unsupported_schema_contracts_fail_closed(self) -> None:
        original = document(make_layer([make_node("root")]))
        for attribute, value in (("schema-version", "2"), ("node-count", "0"), ("node-count", "100001"),
                                 ("hierarchy", "logical"), ("coordinate-space", "document"), ("association", "path")):
            with self.subTest(attribute=attribute, value=value):
                root = copy.deepcopy(original)
                root.find(".//" + ui("tree")).set(attribute, value)
                with self.assertRaises(reader.SemanticError):
                    parse(root)
        data = ET.tostring(original)
        with self.assertRaisesRegex(reader.SemanticError, "namespace version"):
            reader.parse_ui_semantics(data.replace(reader.UI_NS.encode(), (reader.UI_PREFIX + "2").encode()))
        for tag in (ui("future-field"), "{https://example.invalid/extension}field"):
            root = copy.deepcopy(original)
            ET.SubElement(root.find(".//" + ui("node")), tag)
            with self.assertRaises(reader.SemanticError):
                parse(root)

    def test_duplicate_ids_source_keys_fields_and_properties_are_rejected(self) -> None:
        root = document(make_layer([make_node("root")]))
        ET.SubElement(root, svg("path"), {"id": identifier("root")})
        with self.assertRaises(reader.SemanticError):
            parse(root)
        for name in ("geometry", "state", "name"):
            node = make_node("root")
            if name == "name":
                ET.SubElement(node, ui("name")).text = "first"
            node.append(copy.deepcopy(node.find(ui(name))))
            with self.assertRaises(reader.SemanticError):
                parse(document(make_layer([node])))
        node = make_node("root")
        properties = ET.SubElement(node, ui("properties"))
        for _ in range(2):
            ET.SubElement(properties, ui("property"), {"name": "duplicate", "type": "string"})
        with self.assertRaises(reader.SemanticError):
            parse(document(make_layer([node])))
        with self.assertRaises(reader.SemanticError):
            parse(document(make_layer([make_node("root"), make_node("root")])))

    def test_local_links_are_layer_scoped_ordered_and_acyclic(self) -> None:
        for relation, href in (("parent", "#absent"), ("logical-parent", "#absent"),
                               ("templated-parent", "https://invalid.example/ui"), ("parent", "#"),
                               ("logical-parent", "#" + identifier("root"))):
            with self.subTest(relation=relation, href=href):
                node = make_node("root")
                ET.SubElement(node, ui(relation), {"href": href})
                with self.assertRaises(reader.SemanticError):
                    parse(document(make_layer([node])))
        with self.assertRaises(reader.SemanticError):
            parse(document(make_layer([make_node("child", parent="root"), make_node("root")])))
        for relation in ("logical-parent", "templated-parent"):
            first, second = make_node("a"), make_node("b")
            ET.SubElement(first, ui(relation), {"href": "#" + identifier("b")})
            ET.SubElement(second, ui(relation), {"href": "#" + identifier("a")})
            with self.assertRaisesRegex(reader.SemanticError, "cyclic"):
                parse(document(make_layer([first, second])))
        first, second = make_node("root"), make_node("root", layer="second")
        ET.SubElement(first, ui("logical-parent"), {"href": "#" + identifier("root", "second")})
        with self.assertRaises(reader.SemanticError):
            parse(document(make_layer([first]), make_layer([second], key="second")))

    def test_rendering_association_and_layer_summaries_must_match(self) -> None:
        for href in ("#absent", "#" + identifier("root"), "https://invalid.example/layer"):
            group = make_layer([make_node("root")])
            group.find(".//" + ui("rendering")).set("href", href)
            with self.assertRaises(reader.SemanticError):
                parse(document(group))
        for attribute, value in (("data-ui-node-count", "2"), ("data-ui-root", "absent"),
                                 ("data-ui-type", "Wrong.Control"), ("data-ui-name", "Not captured")):
            group = make_layer([make_node("root")])
            group.attrib.update({"data-ui-schema": reader.UI_NS, "data-ui-node-count": "1", attribute: value})
            with self.assertRaises(reader.SemanticError):
                parse(document(group))
        group = ET.Element(svg("g"), {"id": "ferro-ui", "data-ui-schema": reader.UI_NS})
        with self.assertRaises(reader.SemanticError):
            parse(document(group))
        group = make_layer([])
        group.append(copy.deepcopy(group[0]))
        with self.assertRaises(reader.SemanticError):
            parse(document(group))

    def test_geometry_and_state_reject_nonfinite_inconsistent_or_wrongly_typed_values(self) -> None:
        cases = (("geometry", "local-to-layer", "1 0 0 0 NaN 0 0 0 1"),
                 ("geometry", "local-to-layer", "1 0 0 1 0 0"),
                 ("geometry", "layout-bounds", "0 0 -1 30"),
                 ("geometry", "layout-bounds", "1e308 0 1e308 30"),
                 ("geometry", "local-bounds", "1 0 80 30"),
                 ("geometry", "matrix-order", "a b c d e f"),
                 ("state", "opacity", "1.1"), ("state", "opacity", "-0.1"),
                 ("state", "opacity", "1_0"), ("state", "visible", "True"),
                 ("state", "z-index", "1.0"), ("state", "z-index", "2147483648"))
        for tag, attribute, value in cases:
            with self.subTest(tag=tag, attribute=attribute, value=value):
                node = make_node("root")
                node.find(ui(tag)).set(attribute, value)
                with self.assertRaises(reader.SemanticError):
                    parse(document(make_layer([node])))

    def test_properties_source_and_grid_values_are_strict(self) -> None:
        for kind, value in (("number", "NaN"), ("number", "1e9999"), ("boolean", "1"), ("object", "{}")):
            node = make_node("root")
            props = ET.SubElement(node, ui("properties"))
            ET.SubElement(props, ui("property"), {"name": "value", "type": kind}).text = value
            with self.assertRaises(reader.SemanticError):
                parse(document(make_layer([node])))
        for line in ("0", "-1", "4294967296", "1.5"):
            node = make_node("root")
            ET.SubElement(node, ui("source"), {"uri": "relative.fxml", "line": line})
            with self.assertRaises(reader.SemanticError):
                parse(document(make_layer([node])))
        for attribute, value in (("row", "-1"), ("column-span", "0"), ("row-span", "2147483648")):
            node = make_node("root")
            layout = ET.SubElement(node, ui("layout"), {
                "desired-size": "0 0", "margin": "0 0 0 0", "width": "auto", "height": "auto",
                "max-width": "unbounded", "max-height": "unbounded", "min-width": "0", "min-height": "0",
                "horizontal-alignment": "Stretch", "vertical-alignment": "Stretch", "measure-valid": "false",
                "arrange-valid": "false",
            })
            ET.SubElement(layout, ui("grid"), {"row": "0", "column": "0", "row-span": "1", "column-span": "1",
                                               attribute: value})
            with self.assertRaises(reader.SemanticError):
                parse(document(make_layer([node])))

    def test_dtds_entities_orphaned_elements_and_malformed_xml_fail_closed(self) -> None:
        valid = f'<svg xmlns="{reader.SVG_NS}"/>'
        payloads = (f'<!DOCTYPE svg SYSTEM "https://invalid.example/external.dtd">{valid}',
                    f'<!DOCTYPE svg [<!ENTITY name "injected">]>{valid}', "<svg>",
                    f'<svg xmlns="{reader.SVG_NS}">&unknown;</svg>')
        for payload in payloads:
            for encoding in ("utf-8", "utf-16"):
                with self.subTest(payload=payload, encoding=encoding), self.assertRaises(reader.SemanticError):
                    reader.parse_ui_semantics(payload.encode(encoding))
        root = document()
        root.append(make_node("orphan"))
        with self.assertRaises(reader.SemanticError):
            parse(root)

    def test_budgets_stop_excessive_input_and_metadata(self) -> None:
        root = document(make_layer([make_node("root")]))
        for constant, limit in (("MAX_INPUT_BYTES", 4), ("MAX_XML_ELEMENTS", 3), ("MAX_XML_DEPTH", 3),
                                ("MAX_NODES", 0), ("MAX_KEY_BYTES", 3), ("MAX_TEXT_BYTES", 10), ("MAX_VALUES", 1)):
            with self.subTest(constant=constant), patch.object(reader, constant, limit):
                with self.assertRaises(reader.SemanticError):
                    parse(root)

    def test_cli_stdout_and_output_file_are_identical_deterministic_and_portable(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "input with spaces.svg"
            output = Path(directory) / "sidecar.json"
            source.write_bytes(ET.tostring(document(make_layer([make_node("root")]))))
            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                self.assertEqual(0, reader.main([str(source)]))
            self.assertEqual(0, reader.main([str(source), "--output", str(output)]))
            self.assertEqual(stdout.getvalue(), output.read_text(encoding="utf-8"))
            self.assertNotIn(directory, stdout.getvalue())
            self.assertEqual(reader.read_ui_semantics(source), json.loads(stdout.getvalue()))
            source.write_text("<broken>", encoding="utf-8")
            before = output.read_bytes()
            error = io.StringIO()
            with contextlib.redirect_stderr(error):
                self.assertEqual(2, reader.main([str(source), "--output", str(output)]))
            self.assertIn("error:", error.getvalue())
            self.assertEqual(before, output.read_bytes())


if __name__ == "__main__":
    unittest.main()
