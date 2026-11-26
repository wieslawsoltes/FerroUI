"""Tests of deprecated_uses.py: python3 -m unittest discover -s scripts/idioms -p 'test_*.py'."""

import json
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import deprecated_uses  # noqa: E402


def message(file_name, line, column, text):
    return json.dumps({
        "reason": "compiler-message",
        "message": {
            "code": {"code": "deprecated"},
            "message": text,
            "spans": [{"file_name": file_name, "line_start": line, "column_start": column, "is_primary": True}],
        },
    })


class ReadTests(unittest.TestCase):
    def test_a_json_warning_gives_its_place_and_item(self):
        lines = [message("samples/A/lib.rs", 3, 9, "use of deprecated type alias `ferroui_base::animation::TimeSpan`")]
        self.assertEqual(
            deprecated_uses.uses_of_messages(lines),
            [("samples/A/lib.rs", 3, 9, "type alias", "ferroui_base::animation::TimeSpan")],
        )

    def test_the_rendered_text_of_a_log_of_another_checkout_is_read(self):
        lines = [
            "2026-11-23T19:40:33.39Z warning: use of deprecated method `ferroui_base::Thing::old`: use `new`",
            "2026-11-23T19:40:33.39Z    --> /home/runner/work/FerroUI/FerroUI/samples/B/main.rs:12:5",
        ]
        self.assertEqual(
            deprecated_uses.uses_of_rendered(lines), [("samples/B/main.rs", 12, 5, "method", "ferroui_base::Thing::old")]
        )

    def test_only_ferroui_items_in_samples_are_kept(self):
        self.assertTrue(deprecated_uses.is_kept(("samples/A/lib.rs", 1, 1, "function", "ferroui_base::old")))
        self.assertFalse(deprecated_uses.is_kept(("samples/A/lib.rs", 1, 1, "function", "skia_safe::old")))
        self.assertFalse(deprecated_uses.is_kept(("src/A/lib.rs", 1, 1, "function", "ferroui_base::old")))

    def test_a_deprecated_alias_is_read_from_its_declaration(self):
        source = '#[deprecated(since = "0.5.0", note = "use `SignedDuration`")]\npub type TimeSpan = SignedDuration;\n'
        self.assertEqual(deprecated_uses.ALIAS.findall(source), [("TimeSpan", "SignedDuration")])


class ApplyTests(unittest.TestCase):
    def test_an_alias_is_replaced_at_its_place_and_other_uses_are_left(self):
        with tempfile.TemporaryDirectory() as root:
            os.makedirs(os.path.join(root, "samples", "A"))
            path = os.path.join(root, "samples", "A", "lib.rs")
            with open(path, "w", encoding="utf-8") as handle:
                handle.write("use x::TimeSpan;\nfn f(t: TimeSpan) { old(); }\n")
            saved, deprecated_uses.ROOT = deprecated_uses.ROOT, root
            try:
                left = deprecated_uses.apply(
                    [
                        ("samples/A/lib.rs", 1, 8, "type alias", "x::TimeSpan"),
                        ("samples/A/lib.rs", 2, 9, "type alias", "x::TimeSpan"),
                        ("samples/A/lib.rs", 2, 21, "function", "x::old"),
                    ],
                    {"TimeSpan": "SignedDuration"},
                )
            finally:
                deprecated_uses.ROOT = saved
            with open(path, encoding="utf-8") as handle:
                self.assertEqual(handle.read(), "use x::SignedDuration;\nfn f(t: SignedDuration) { old(); }\n")
            self.assertEqual(left, [("samples/A/lib.rs", 2, 21, "function", "x::old")])


if __name__ == "__main__":
    unittest.main()
