import importlib.util
import json
from pathlib import Path
import unittest

SPEC = importlib.util.spec_from_file_location("winui_event_targets", Path(__file__).with_name("event_targets.py"))
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class EventTargets(unittest.TestCase):
    def test_inherited_winui_events_resolve_exact_owners(self):
        table = MODULE.rows(json.loads(MODULE.MODEL.read_bytes()))
        self.assertTrue(MODULE.is_event(table, MODULE.PRESENTATION, "Button", "Click"))
        self.assertTrue(MODULE.is_event(table, "using:Microsoft.UI.Xaml.Controls", "Button", "Loaded"))
        self.assertTrue(MODULE.is_event(table, MODULE.PRESENTATION, "ListView", "SelectionChanged"))
        self.assertFalse(MODULE.is_event(table, MODULE.PRESENTATION, "Button", "Content"))

    def test_custom_and_unknown_members_are_not_rejected_by_name(self):
        table = MODULE.rows(json.loads(MODULE.MODEL.read_bytes()))
        self.assertFalse(MODULE.is_event(table, "using:MyApp", "Button", "Click"))
        self.assertFalse(MODULE.is_event(table, MODULE.PRESENTATION, "Unregistered", "Click"))
        self.assertFalse(MODULE.is_event(table, MODULE.PRESENTATION, "Button", "UnknownEvent"))

    def test_cycles_and_ambiguous_default_names_fail_closed(self):
        table = [("Microsoft.UI.Xaml.Controls.A", "Microsoft.UI.Xaml.Controls.B", ()),
                 ("Microsoft.UI.Xaml.Controls.B", "Microsoft.UI.Xaml.Controls.A", ()),
                 ("Microsoft.UI.Xaml.Controls.Primitives.A", "", ("Click",))]
        self.assertFalse(MODULE.is_event(table, MODULE.PRESENTATION, "A", "Click"))
        self.assertFalse(MODULE.is_event(table, "using:Microsoft.UI.Xaml.Controls", "B", "Click"))

    def test_generated_output_is_deterministic(self):
        raw = MODULE.MODEL.read_bytes()
        self.assertEqual(MODULE.render(raw), MODULE.render(raw))
        self.assertIn("upstream-api.json SHA256:", MODULE.render(raw))


if __name__ == "__main__":
    unittest.main()
