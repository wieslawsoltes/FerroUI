#!/usr/bin/env python3
"""Tests of the generators of the bindings: the API model (`generate_api_model.py`) and its Osprey
backend (`generate_osprey_bindings.py`).

    python3 scripts/test_generate_bindings.py

Standard library only; it reads the sources and the generated files of the repository and
writes nothing.
"""

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
        self.assertRegex(objects, r"fn fontSize\(object: int, value: float\) -> int = \{\n"
                                  r"    let _ = ferro_set_f64\(object: object, property: \d+, value: value\)\n    object\n\}")
        self.assertIn("fn button() -> int = ferro_create(typeId: ", objects)
        self.assertNotIn("ferro_property_id", objects, "no name is looked up at run time")
        self.assertRegex(objects, r"fn onClick\(object: int, tag: int\) -> int")

    def test_views_have_elements_attributes_and_messages(self):
        views = self.text["views.osp"]
        self.assertRegex(views, r"fn stackPanel<M>\(attrs: List<Attr<M>>, children: List<Element<M>>\) -> Element<M>")
        self.assertRegex(views, r"fn textBlock<M>\(attrs: List<Attr<M>>\) -> Element<M>")
        self.assertRegex(views, r"fn onClick<M>\(message: M\) -> Attr<M> = AttrOn")
        self.assertRegex(views, r"fn onTextChanged<M>\(message: M\) -> Attr<M> = AttrOnValue")

    def test_enums_are_unions_named_apart(self):
        abi = self.text["abi.osp"]
        self.assertIn("type Orientation = OrientationHorizontal | OrientationVertical", abi)
        self.assertIn(f"fn apiFingerprint() -> int = {OSPREY_MODEL()['fingerprint']}", abi)

    def test_the_generated_files_are_up_to_date(self):
        for name, text in self.files:
            path = os.path.join(OSPREY.OUT, name)
            with open(path, encoding="utf-8") as f:
                self.assertEqual(f.read(), text, f"{name} is stale: python3 scripts/generate_osprey_bindings.py")


def OSPREY_MODEL():
    with open(OSPREY.MODEL, encoding="utf-8") as f:
        return json.load(f)


if __name__ == "__main__":
    unittest.main()
