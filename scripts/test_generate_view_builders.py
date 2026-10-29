#!/usr/bin/env python3
"""Tests of the scoped names of `generate_view_builders.py` (`SCOPED_PATHS`).

    python3 scripts/test_generate_view_builders.py

Standard library only; it reads the sources of the repository, writes nothing.
"""

import importlib.util
import os
import sys
import unittest

SCRIPTS = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, SCRIPTS)
SPEC = importlib.util.spec_from_file_location("generate_view_builders", os.path.join(SCRIPTS, "generate_view_builders.py"))
BUILDERS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUILDERS)

UPSTREAM = "ferroui_controls::NavigationEventArgs"
SCOPED = "ferroui_controls::navigation::NavigationEventArgs"


def property_source(imports, argument):
    return imports + f"""
ferro_properties! {{ impl EventHost {{
    pub fn args_property() -> StyledProperty<{argument}> {{ todo!() }}
}} }}
"""


class ScopedNameTests(unittest.TestCase):
    def setUp(self):
        self.model = BUILDERS.Model()
        self.model.add_path("NavigationEventArgs", UPSTREAM)
        self.model.add_path(BUILDERS.SCOPED_PATHS[SCOPED], SCOPED)

    def value_type(self, relative_path, imports, argument):
        scoped = BUILDERS.qualify_scoped_names(property_source(imports, argument), "ferroui_controls", relative_path)
        self.model.properties.clear()
        BUILDERS.read_members(self.model, scoped, "ferroui_controls", relative_path)
        return BUILDERS.resolve(self.model, self.model.properties["EventHost"][0][2])

    def test_a_name_resolves_by_the_module_that_declares_or_imports_it(self):
        for relative_path, imports, argument, expected in [
            ("navigation.rs", "", "NavigationEventArgs", SCOPED),
            ("navigation/mod.rs", "", "NavigationEventArgs", SCOPED),
            ("frame.rs", "use crate::navigation::{NavigationEventArgs};", "NavigationEventArgs", SCOPED),
            ("frame.rs", "use crate::{navigation::{NavigationEventArgs as Args}};", "Args", SCOPED),
            ("frame.rs", "use crate::navigation::*;", "NavigationEventArgs", SCOPED),
            ("frame.rs", "", "crate::navigation::NavigationEventArgs", SCOPED),
            ("navigation/journal.rs", "use super::NavigationEventArgs;", "NavigationEventArgs", SCOPED),
            ("page/navigation_page.rs", "use crate::NavigationEventArgs;", "NavigationEventArgs", UPSTREAM),
            ("page/navigation_page.rs", "", "NavigationEventArgs", UPSTREAM),
        ]:
            with self.subTest(relative_path=relative_path, imports=imports):
                self.assertEqual(expected, self.value_type(relative_path, imports, argument))

    def test_a_source_without_the_scoped_path_is_unchanged(self):
        source = property_source("use crate::NavigationEventArgs;", "NavigationEventArgs")
        self.assertEqual(source, BUILDERS.qualify_scoped_names(source, "ferroui_controls", "page/navigation_page.rs"))


class RepositoryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.model = BUILDERS.Model()
        for directory, crate in BUILDERS.CRATES:
            BUILDERS.read_paths(cls.model, directory, crate)
        for directory, crate in BUILDERS.CRATES:
            BUILDERS.read_classes(cls.model, directory, crate)
        cls.output = BUILDERS.generate(cls.model)

    def test_the_upstream_pages_keep_their_paths(self):
        self.assertEqual("ferroui_controls::Page", BUILDERS.class_path(self.model, "Page"))
        self.assertNotIn("Page", self.model.ambiguous)
        self.assertEqual("Page", self.model.bases["ContentPage"])
        self.assertIn("pub fn content_page() -> El<ferroui_controls::ContentPage>", self.output)


if __name__ == "__main__":
    unittest.main()
