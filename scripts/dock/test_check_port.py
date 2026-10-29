"""Tests for the inventory checks that protect port completeness."""

import unittest
import json
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

from check_port import audit, enum_members, integer_expression, source_digest, verify_enum


class EnumInventoryTests(unittest.TestCase):
    def test_additional_metadata_target_cannot_bypass_pure_enum_audit(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "source"
            port = root / "port"
            (source / "src").mkdir(parents=True)
            (port / "src").mkdir(parents=True)
            (port / "docs/porting/dock").mkdir(parents=True)
            original = "public enum Example { First, Second }"
            (source / "src/Example.cs").write_text(original)
            (port / "src/example.rs").write_text("pub enum Example { First = 0 }")
            (port / "src/metadata.rs").write_text("// descriptor correspondence\n")
            baseline = {"source_revision":"fixture","files":[{"path":"src/Example.cs","sha256":source_digest(original.encode())}]}
            records = {"source_revision":"fixture","files":[{"source":"src/Example.cs","targets":["src/example.rs","src/metadata.rs"],"status":"ported","members":["First","Second"]}]}
            (port / "docs/porting/dock/upstream-files.json").write_text(json.dumps(baseline))
            (port / "docs/porting/dock/ports.json").write_text(json.dumps(records))
            with patch("check_port.source_paths", return_value={"src/Example.cs"}):
                errors, _ = audit(port, source)
            self.assertTrue(any("Rust numeric members differ" in error for error in errors))
            (port / "src/example.rs").write_text("pub enum Example { First = 0, Second = 1 }")
            with patch("check_port.source_paths", return_value={"src/Example.cs"}):
                errors, _ = audit(port, source)
            self.assertEqual(errors, [])

    def test_open_enum_values_are_checked_on_transparent_signed_storage(self):
        source = "public enum Example { First, Second }"
        target = "#[repr(transparent)] pub struct Example(i32); impl Example { pub const First: Self = Self(0); pub const Second: Self = Self(1); pub fn from_raw(v: i32)->Self { Self(v) } }"
        self.assertEqual(verify_enum(source, target, ["First", "Second"]), [])
        self.assertTrue(verify_enum(source, target.replace("Self(1);", "Self(2);"), ["First", "Second"]))
        self.assertTrue(verify_enum(source, target.replace("pub struct Example(i32)", "pub struct Example(i64)"), ["First", "Second"]))
        self.assertTrue(verify_enum(source, target.replace("#[repr(transparent)]", ""), ["First", "Second"]))

    def test_open_enum_test_assertions_comments_and_duplicate_constants_are_not_definitions(self):
        source = "public enum Example { First, Second }"
        target = "#[repr(transparent)] pub struct Example(i32); impl Example { pub const First: Self = Self(0); /* missing Second */ } assert_eq!(Example::Second.to_raw(),1);"
        self.assertTrue(verify_enum(source, target, ["First", "Second"]))
        duplicate = target.replace("/* missing Second */", "pub const Second: Self = Self(1); pub const Second: Self = Self(1);")
        self.assertTrue(verify_enum(source, duplicate, ["First", "Second"]))

    def test_sample_rust_units_are_audited_for_mapping_names_and_placeholders(self):
        with TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "docs/porting/dock").mkdir(parents=True)
            fixture = {"source_revision": "fixture", "files": []}
            for name in ["upstream-files.json", "ports.json"]:
                (root / "docs/porting/dock" / name).write_text(json.dumps(fixture))
            sample = root / "samples" / "Dock" / "Demo"
            sample.mkdir(parents=True)
            (sample / "unmapped.rs").write_text("struct AvaloniaControl; fn absent() { unimplemented!(); }")
            with patch("check_port.source_paths", return_value=set()):
                errors, _ = audit(root, root)
            self.assertTrue(any("source mapping" in error and "samples/Dock/Demo/unmapped.rs" in error for error in errors))
            self.assertTrue(any("upstream UI identifier" in error for error in errors))
            self.assertTrue(any("placeholder implementation" in error for error in errors))

    def test_source_digests_ignore_checkout_line_endings_but_detect_content_changes(self):
        self.assertEqual(source_digest(b"first\nsecond\n"), source_digest(b"first\r\nsecond\r\n"))
        self.assertNotEqual(source_digest(b"first\n"), source_digest(b"second\n"))

    def test_single_line_xml_comments_do_not_hide_outer_operations(self):
        source = '''public enum DockOperation {
            None,
            /// <summary>A normal operation.</summary>
            Left,
            /// <summary>An outer operation.</summary>
            OuterLeft
        }'''
        target = "pub enum DockOperation { None = 0, Left = 1 }"
        errors = verify_enum(source, target, ["None", "Left"])
        self.assertEqual(len(errors), 2)
        self.assertIn("Rust numeric members differ", errors[1])

    def test_explicit_and_implicit_values_are_preserved(self):
        source = "public enum Alignment { Unset = -1, Left = 0, Bottom, Right, Top }"
        self.assertEqual(enum_members(source), ("Alignment", {"Unset": -1, "Left": 0, "Bottom": 1, "Right": 2, "Top": 3}, False))
        target = "pub enum Alignment { Unset = -1, Left = 0, Bottom = 1, Right = 2, Top = 3 }"
        self.assertEqual(verify_enum(source, target, ["Unset", "Left", "Bottom", "Right", "Top"]), [])

    def test_flag_masks_combine_named_values_without_evaluating_code(self):
        source = "[System.Flags] public enum Mask { None = 0, Fill = 1 << 0, OuterLeft = 1 << 1, All = Fill | OuterLeft }"
        target = "pub struct Mask: i32 { const NONE = 0; const FILL = 1; const OUTER_LEFT = 2; const ALL = 3; }"
        self.assertEqual(verify_enum(source, target, ["None", "Fill", "OuterLeft", "All"]), [])
        with self.assertRaises(ValueError):
            integer_expression("__import__('os')", {})

    def test_test_assertions_cannot_mask_a_missing_declaration(self):
        source = "public enum Example { First, Second }"
        target = "pub enum Example { First = 0 } // Second = 1\nassert_eq!(Example::Second as i32, 1);"
        self.assertTrue(verify_enum(source, target, ["First", "Second"]))

    def test_extra_or_wrong_numeric_members_are_detected(self):
        source = "public enum Example { First, Second }"
        target = "pub enum Example { First = 0, Second = 2, Third = 3 }"
        self.assertTrue(verify_enum(source, target, ["First", "Second"]))


if __name__ == "__main__":
    unittest.main()
