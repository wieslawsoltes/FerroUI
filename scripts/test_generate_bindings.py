#!/usr/bin/env python3
"""Tests of the generators of the bindings: the API model (`generate_api_model.py`), its Osprey
backend (`generate_osprey_bindings.py`) and the selection of an application they share with the
generator of its library (`ferroui_selection.py`).

    python3 scripts/test_generate_bindings.py

Standard library only; it reads the sources and the generated files of the repository and
writes nothing.
"""

import glob
import importlib.util
import json
import os
import re
import sys
import unittest

SCRIPTS = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, SCRIPTS)


def load(name):
    spec = importlib.util.spec_from_file_location(name, os.path.join(SCRIPTS, f"{name}.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


MODEL = load("generate_api_model")
OSPREY = load("generate_osprey_bindings")


class ModelTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.document, cls.tables = MODEL.build()

    def test_names_are_words(self):
        self.assertEqual(MODEL.words("FontSize"), ["font", "size"])
        self.assertEqual(MODEL.words("IsHitTestVisible"), ["is", "hit", "test", "visible"])
        self.assertEqual(MODEL.words("Grid.Row"), ["grid", "row"])

    def test_value_kinds(self):
        model = MODEL.meta.Model()
        model.enums["ClickMode"] = {"members": [], "crate": "", "doc": ""}
        self.assertEqual(MODEL.value_of(model, "f64")["kind"], "number")
        self.assertEqual(MODEL.value_of(model, "Option<String>")["kind"], "string")
        self.assertEqual(MODEL.value_of(model, "ClickMode")["kind"], "enum")
        self.assertEqual(MODEL.value_of(model, "Option<Ref<Control>>")["kind"], "object")
        self.assertEqual(MODEL.value_of(model, "Option<BoxedValue>")["kind"], "content")
        self.assertEqual(MODEL.value_of(model, "Thickness")["kind"], "parsed")
        self.assertEqual(MODEL.value_of(model, "Option<Rc<dyn ICommand>>")["kind"], "unsupported")

    def test_ids_are_the_order_of_the_tables(self):
        types, properties, events, fingerprint = self.tables
        self.assertEqual([c["name"] for c in self.document["classes"]], types)
        self.assertEqual([p["name"] for p in self.document["properties"]], properties)
        self.assertEqual([e["name"] for e in self.document["events"]], events)
        for index, entry in enumerate(self.document["properties"]):
            self.assertEqual(entry["id"], index + 1)
        self.assertEqual(self.document["fingerprint"], fingerprint)

    def test_the_model_knows_the_controls_of_the_samples(self):
        classes = {c["name"]: c for c in self.document["classes"]}
        self.assertTrue(classes["Button"]["creatable"])
        self.assertEqual(classes["StackPanel"]["children"], "children")
        self.assertEqual(classes["Button"]["children"], "content")
        self.assertEqual(classes["Border"]["children"], "child")
        properties = {p["name"]: p for p in self.document["properties"]}
        self.assertIn("Grid.Row", properties)
        self.assertEqual(properties["Grid.Row"]["declarations"][0]["kind"], "attached")
        events = {e["name"]: e for e in self.document["events"]}
        self.assertEqual(events["TextChanged"]["value_property"], "Text")
        self.assertTrue(any(d["raisable"] for d in events["Click"]["declarations"]))
        enums = {e["name"]: e for e in self.document["enums"]}
        self.assertEqual([m["name"] for m in enums["Orientation"]["members"]], ["Horizontal", "Vertical"])

    def test_the_abi_comes_from_the_header(self):
        functions = {f["name"]: f for f in self.document["abi"]["functions"]}
        self.assertEqual(functions["ferro_set_f64"]["params"][2], {"name": "value", "type": "f64"})
        self.assertEqual(functions["ferro_frame_commit"]["returns"], "status")
        self.assertIn("ferro_api_fingerprint", functions)
        statuses = {s["name"]: s["value"] for s in self.document["abi"]["statuses"]}
        self.assertEqual(statuses["FERRO_WRONG_THREAD"], -7)

    def test_the_frame_layout_matches_the_library(self):
        source = MODEL.meta.read(os.path.join(MODEL.ROOT, "src", "FerroUI.CApi", "frame.rs"))
        for record in self.document["frame"]["records"]:
            constant = re.search(r"pub\(crate\) const OP_" + record["name"].upper() + r": u32 = (\d+);", source)
            self.assertIsNotNone(constant, record["name"])
            self.assertEqual(int(constant.group(1)), record["op"], record["name"])
            self.assertIn(f"pub extern \"C\" fn {record['writer']}(", source.replace("unsafe extern", "extern"))

    def test_the_generated_files_are_up_to_date(self):
        text = json.dumps(self.document, indent=1, ensure_ascii=False) + "\n"
        self.assertEqual(MODEL.meta.read(MODEL.MODEL), text,
                         "bindings/model/ferroui-api.json is stale: python3 scripts/generate_api_model.py")
        self.assertEqual(MODEL.meta.read(MODEL.ID_TABLE), MODEL.id_table(*self.tables),
                         "src/FerroUI.CApi/api_ids.rs is stale: python3 scripts/generate_api_model.py")


class OspreyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.files, cls.counts = OSPREY.outputs()
        cls.text = dict(cls.files)

    def test_names_follow_osprey(self):
        self.assertEqual(OSPREY.camel(["font", "size"]), "fontSize")
        self.assertEqual(OSPREY.pascal(["vertical", "alignment"]), "VerticalAlignment")
        self.assertEqual(OSPREY.safe("match", "Of"), "matchOf")

    def test_setters_chain_and_call_constants(self):
        objects = self.text["objects.osp"]
        self.assertRegex(objects, r"// fontSize\(object: int, value: float\) -> int: sets FontSize")
        self.assertRegex(objects, r"fn fontSize\(object, value\) = \{\n"
                                  r"    let _ = ferro_set_f64\(object: object, property: \d+, value: value\)\n    object\n\}")
        self.assertIn("fn button() = ferro_create(typeId: ", objects)
        self.assertNotIn("ferro_property_id", objects, "no name is looked up at run time")
        self.assertRegex(objects, r"fn onClick\(object, tag\) = ")

    def test_generated_functions_carry_no_redundant_annotations(self):
        # The lint of Osprey on redundant annotations is slow on a large file (docs/osprey.md,
        # section 5): generated functions name their types in the comment only.
        for name, text in self.files:
            for line in text.split("\n"):
                if line.startswith("fn "):
                    self.assertNotRegex(line.split("=")[0], r":|->", f"{name}: {line}")

    def test_views_have_elements_attributes_and_messages(self):
        views = self.text["views.osp"]
        self.assertRegex(views, r"// stackPanel\(attrs: List<Attr<M>>, children: List<Element<M>>\) -> Element<M>")
        self.assertIn("fn stackPanel(attrs, children) = Element {", views)
        self.assertIn("fn textBlock(attrs) = Element {", views)
        self.assertRegex(views, r"fn onClick\(message\) = AttrOn ")
        self.assertRegex(views, r"fn onTextChanged\(message\) = AttrOnValue ")

    def test_enums_are_unions_named_apart(self):
        abi = self.text["abi.osp"]
        self.assertIn("type Orientation = OrientationHorizontal | OrientationVertical", abi)
        self.assertIn(f"fn apiFingerprint() = {OSPREY_MODEL()['fingerprint']}", abi)

    def test_the_generated_files_are_up_to_date(self):
        model, names = OSPREY.load_model()
        for directory, manifest_path in OSPREY.targets():
            manifest = None
            if manifest_path:
                with open(manifest_path, encoding="utf-8") as f:
                    manifest = json.load(f)
            files, _ = OSPREY.outputs(manifest, model, names)
            for name, text in files:
                path = os.path.join(directory, name)
                with open(path, encoding="utf-8") as f:
                    self.assertEqual(f.read(), text, f"{path} is stale: python3 scripts/generate_osprey_bindings.py")


class SelectionTests(unittest.TestCase):
    """The bindings of an application are its selection: small, so that it builds in seconds."""

    # A selection of a sample stays below this many bytes of generated Osprey; the full bindings
    # are about 1.1 MB. Raise it only with a reason (docs/osprey.md, section 5).
    LIMIT = 200_000

    @classmethod
    def setUpClass(cls):
        cls.model, cls.names = OSPREY.load_model()

    def selection(self, classes, layers=("objects", "views")):
        files, _ = OSPREY.outputs({"classes": classes, "layers": list(layers)}, self.model, self.names)
        return dict(files)

    def test_a_selection_has_its_classes_their_bases_and_nothing_else(self):
        files = self.selection(["Button"], ["objects"])
        self.assertEqual(sorted(files), ["abi.osp", "objects.osp"])
        objects = files["objects.osp"]
        self.assertIn("fn button() = ", objects)
        self.assertNotIn("fn stackPanel() = ", objects)
        # Content of ContentControl and Width of Layoutable, bases of Button.
        self.assertIn("fn content(object, value) = ", objects)
        self.assertIn("fn width(object, value) = ", objects)
        self.assertNotIn("fn spacing(object, value) = ", objects)
        self.assertIn("type ClickMode = ", files["abi.osp"])
        self.assertNotIn("type Orientation = ", files["abi.osp"])

    def test_names_are_the_names_of_the_full_bindings(self):
        full = dict(OSPREY.outputs(None, self.model, self.names)[0])
        selected = self.selection(["Window", "StackPanel", "TextBox", "Button"])
        for name in ("objects.osp", "views.osp"):
            full_functions = set(re.findall(r"^fn (\w+)", full[name], re.M))
            for function in re.findall(r"^fn (\w+)", selected[name], re.M):
                self.assertIn(function, full_functions, f"{name}: {function}")

    def test_unknown_classes_and_layers_are_errors(self):
        with self.assertRaises(SystemExit):
            self.selection(["NoSuchControl"])
        with self.assertRaises(SystemExit):
            self.selection(["Button"], ["widgets"])

    def test_every_sample_has_a_small_selection(self):
        paths = glob.glob(OSPREY.SAMPLE_MANIFESTS)
        self.assertGreaterEqual(len(paths), 6)
        for path in paths:
            directory = os.path.join(os.path.dirname(path), "ferroui")
            size = sum(os.path.getsize(os.path.join(directory, name)) for name in os.listdir(directory))
            self.assertLess(size, self.LIMIT, f"{directory}: {size} bytes of generated Osprey")
            with open(os.path.join(os.path.dirname(path), "osprey.toml"), encoding="utf-8") as f:
                roots = f.read()
            self.assertIn('"ferroui"', roots, f"{path}: the project builds its selection")
            self.assertNotIn("bindings/osprey/full", roots, f"{path}: not the full bindings")

    def test_a_selection_registers_its_classes_and_has_its_fingerprint(self):
        import ferroui_selection
        manifest = {"classes": ["Window", "StackPanel", "TextBlock", "Button"], "layers": ["objects"]}
        abi = self.selection(manifest["classes"], manifest["layers"])["abi.osp"]
        for entry in ("ferro_type_window", "ferro_type_stack_panel", "ferro_type_text_block", "ferro_type_button"):
            self.assertIn(f"extern fn {entry}() -> int", abi)
            self.assertIn(f"= {entry}()", abi)
        self.assertNotIn("ferro_type_text_box", abi)
        fingerprint = ferroui_selection.AppSelection(self.model, manifest).fingerprint
        self.assertIn(f"fn selectionFingerprint() = {fingerprint}\n", abi)
        # The order of the classes and the layers do not matter; the classes and the calls do.
        same = ferroui_selection.AppSelection(self.model, {"classes": list(reversed(manifest["classes"]))})
        self.assertEqual(same.fingerprint, fingerprint)
        other = ferroui_selection.AppSelection(self.model, {**manifest, "abi": ["frames"]})
        self.assertNotEqual(other.fingerprint, fingerprint)
        full = dict(OSPREY.outputs(None, self.model, self.names)[0])["abi.osp"]
        self.assertIn("fn selectionFingerprint() = 0\n", full)
        self.assertIn("fn registerTypes() = 0\n", full)

    def test_the_exports_of_a_selection(self):
        import ferroui_selection
        selection = ferroui_selection.AppSelection(self.model, {"classes": ["Button"], "abi": ["signals"]})
        self.assertIn("ferro_type_button", selection.exports)
        self.assertIn("ferro_create", selection.exports)
        self.assertIn("ferro_selection_fingerprint", selection.exports)
        self.assertIn("ferro_signal_new", selection.exports)
        self.assertNotIn("ferro_frame_open", selection.exports)
        self.assertNotIn("ferro_class_register", selection.exports)
        self.assertNotIn("ferro_type_window", selection.exports)
        with self.assertRaises(SystemExit):
            ferroui_selection.AppSelection(self.model, {"classes": ["Button"], "abi": ["widgets"]})


def OSPREY_MODEL():
    with open(OSPREY.MODEL, encoding="utf-8") as f:
        return json.load(f)


if __name__ == "__main__":
    unittest.main()
