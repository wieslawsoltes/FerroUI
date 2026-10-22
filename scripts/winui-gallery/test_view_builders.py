"""Keep native control builders and existing page/navigation contracts distinct."""

import importlib.util
from pathlib import Path
import sys
import unittest


SCRIPT = Path(__file__).resolve().parents[1] / "generate_view_builders.py"
sys.path.insert(0, str(SCRIPT.parent))
SPEC = importlib.util.spec_from_file_location("gallery_view_builders", SCRIPT)
BUILDERS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(BUILDERS)


class NativeControlBuilderTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.model = BUILDERS.Model()
        for directory, crate in BUILDERS.CRATES:
            BUILDERS.read_paths(cls.model, directory, crate)
        for directory, crate in BUILDERS.CRATES:
            BUILDERS.read_classes(cls.model, directory, crate)
        cls.output = BUILDERS.generate(cls.model)

    def assert_output_contains(self, declaration):
        self.assertTrue(declaration in self.output, f"Missing generated declaration: {declaration}")

    def test_existing_page_and_frame_page_keep_distinct_type_paths(self):
        self.assertEqual(BUILDERS.class_path(self.model, "Page"), "ferroui_controls::Page")
        self.assertEqual(BUILDERS.class_path(self.model, "FramePage"), "ferroui_controls::FramePage")
        self.assertNotIn("Page", self.model.ambiguous)
        # The existing Page is abstract; ContentPage keeps that original base.
        self.assertEqual(self.model.bases["ContentPage"], "Page")
        self.assert_output_contains("pub fn content_page() -> El<ferroui_controls::ContentPage>")
        self.assert_output_contains("pub fn frame_page() -> El<ferroui_controls::FramePage>")
        self.assert_output_contains("pub use ferroui_controls::FramePage;")

    def test_navigation_cache_property_belongs_only_to_frame_page(self):
        native = {entry[1] for entry in self.model.properties.get("Page", [])}
        frame_page = {entry[1] for entry in self.model.properties.get("FramePage", [])}
        self.assertNotIn("navigation_cache_mode", native)
        self.assertIn("navigation_cache_mode", frame_page)
        self.assert_output_contains("pub trait PageProps")
        self.assert_output_contains("pub trait FramePageProps")

    def test_control_properties_resolve_to_native_types(self):
        self.assertEqual(
            BUILDERS.resolve(self.model, "NumberBoxSpinButtonPlacementMode"),
            "ferroui_controls::NumberBoxSpinButtonPlacementMode",
        )
        self.assert_output_contains("pub fn numeric_up_down() -> El<ferroui_controls::NumericUpDown>")
        self.assert_output_contains("pub fn number_box() -> El<ferroui_controls::NumberBox>")

    def test_implemented_controls_have_normal_native_builders(self):
        for name in [
            "AutoSuggestBox", "FontIcon", "Frame", "FramePage", "InfoBadge", "InfoBar",
            "InfoBarPanel", "NavigationView", "NavigationViewItem", "NavigationViewItemHeader",
            "NavigationViewItemSeparator", "NumberBox", "ProgressRing", "RadioButtons",
            "RatingControl", "SymbolIcon",
        ]:
            self.assert_output_contains(
                f"pub fn {BUILDERS.snake(name)}() -> El<ferroui_controls::{name}>")
        self.assertNotIn("ferroui_controls::winui::", self.output)

    def test_same_named_navigation_arguments_resolve_by_declared_or_imported_path(self):
        model = BUILDERS.Model()
        model.add_path("NavigationEventArgs", "ferroui_controls::NavigationEventArgs")
        model.add_path("FrameNavigationEventArgs", "ferroui_controls::navigation::NavigationEventArgs")
        for relative_path, imports, argument, expected in [
            ("navigation.rs", "", "NavigationEventArgs", "ferroui_controls::navigation::NavigationEventArgs"),
            ("frame.rs", "use crate::navigation::{NavigationEventArgs};", "NavigationEventArgs", "ferroui_controls::navigation::NavigationEventArgs"),
            ("frame.rs", "use crate::{navigation::{NavigationEventArgs as Args}};", "Args", "ferroui_controls::navigation::NavigationEventArgs"),
            ("frame.rs", "", "crate::navigation::NavigationEventArgs", "ferroui_controls::navigation::NavigationEventArgs"),
            ("page/navigation_page.rs", "use crate::NavigationEventArgs;", "NavigationEventArgs", "ferroui_controls::NavigationEventArgs"),
        ]:
            with self.subTest(relative_path=relative_path, imports=imports):
                source = imports + f"""
ferro_properties! {{ impl EventHost {{
    pub fn args_property() -> StyledProperty<{argument}> {{ todo!() }}
}} }}
"""
                scoped = BUILDERS.qualify_scoped_names(source, "ferroui_controls", relative_path)
                model.properties.clear()
                BUILDERS.read_members(model, scoped, "ferroui_controls", relative_path)
                value_type = model.properties["EventHost"][0][2]
                self.assertEqual(BUILDERS.resolve(model, value_type), expected)


if __name__ == "__main__":
    unittest.main()
