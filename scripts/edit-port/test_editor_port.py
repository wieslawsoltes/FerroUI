#!/usr/bin/env python3
"""Small source and mapping fixtures; no network or full upstream checkout."""

import copy
import importlib.util
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("editor_port", Path(__file__).with_name("editor_port.py"))
port = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(port)


class EditorPortTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.repo = Path(self.temp.name)
        destination = self.repo / "src/Editor/document.rs"
        destination.parent.mkdir(parents=True)
        destination.write_text('''
            pub struct Document;
            impl Document { pub fn replace(&self) {} }
            // pub struct Imaginary;
            const EXAMPLE: &str = "pub struct NotADeclaration;";
            #[cfg(test)] mod tests { struct OnlyInTests; }
        ''')
        self.inventory = {"schema_version": 1, "repositories": []}
        self.mappings = {"schema_version": 1, "files": []}
        for name, pin in port.PINS.items():
            source = {
                "path": "src/Editor/Document.cs", "category": "library", "kind": "csharp",
                "git_blob": "0" * 40, "sha256": "1" * 64, "bytes": 16, "lines": 1,
                "types": [{"name": "Document", "qualified_name": "Editor.Document", "namespace": "Editor", "kind": "class", "visibility": "public", "line": 1}],
            }
            self.inventory["repositories"].append({"id": name, **pin, "root": "src", "files": [source]})
            self.mappings["files"].append({
                "source": name + ":" + source["path"], "status": "mapped", "notes": "Exact type traceability only.",
                "targets": [{"path": "src/Editor/document.rs", "symbols": ["Document", "Document::replace"],
                             "aliases": [{"upstream": "Editor.Document", "rust": "Document"}]}],
                "unmapped_types": [], "limitations": ["No member compatibility claim."],
            })

    def validate(self):
        return port.validate_mappings(self.repo, port.validate_inventory(self.inventory), self.mappings)

    def write_inputs(self):
        for relative, value in [(port.INVENTORY, self.inventory), (port.MAPPINGS, self.mappings)]:
            path = self.repo / relative
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(port.json_text(value))

    def test_csharp_declarations_ignore_comments_literals_and_generic_constraints(self):
        source = '''
            // public class NotInComment {}
            namespace Editor {
                #region generic interface
                public partial class Outer<T> where T : class {
                    const string s = @"public class NotInVerbatim { ""quoted"" }";
                    const string t = "struct NotInString {}";
                    public record struct Item(int Offset);
                    protected class Inner {}
                }
                public delegate void Changed(int value);
            }
        '''
        types = port.csharp_types(source)
        self.assertEqual([item["qualified_name"] for item in types], ["Editor.Outer", "Editor.Outer.Item", "Editor.Outer.Inner", "Editor.Changed"])
        self.assertEqual(types[1]["kind"], "record struct")
        self.assertEqual(types[2]["visibility"], "protected")

    def test_csharp_file_namespace_and_verbatim_quote_escapes(self):
        source = 'namespace Editor; public class Source { string s = @""""; } public enum Kind { A }'
        self.assertEqual([item["qualified_name"] for item in port.csharp_types(source)], ["Editor.Source", "Editor.Kind"])

    def test_missing_source_mapping_is_rejected(self):
        self.mappings["files"].pop()
        with self.assertRaisesRegex(port.InventoryError, "Sources without an explicit"):
            self.validate()

    def test_stale_upstream_pin_and_invalid_hash_are_rejected(self):
        original = copy.deepcopy(self.inventory)
        self.inventory["repositories"][0]["commit"] = "f" * 40
        with self.assertRaisesRegex(port.InventoryError, "pinned source"):
            self.validate()
        self.inventory = original
        self.inventory["repositories"][0]["files"][0]["sha256"] = "not a hash"
        with self.assertRaisesRegex(port.InventoryError, "Invalid SHA-256"):
            self.validate()

    def test_exact_symbols_not_substrings_comments_or_test_types(self):
        for symbol in ["Doc", "Document::remove", "Imaginary", "NotADeclaration", "OnlyInTests"]:
            with self.subTest(symbol=symbol):
                self.mappings["files"][0]["targets"][0]["symbols"].append(symbol)
                with self.assertRaisesRegex(port.InventoryError, "Missing exact Rust symbol"):
                    self.validate()
                self.mappings["files"][0]["targets"][0]["symbols"].pop()

    def test_missing_and_escaping_destinations_are_rejected(self):
        target = self.mappings["files"][0]["targets"][0]
        target["path"] = "src/missing.rs"
        with self.assertRaisesRegex(port.InventoryError, "Missing Rust destination"):
            self.validate()
        target["path"] = "../outside.rs"
        with self.assertRaisesRegex(port.InventoryError, "Unsafe repository-relative"):
            self.validate()

    def test_each_type_requires_mapping_or_explicit_unmapped_decision(self):
        source = self.inventory["repositories"][0]["files"][0]
        source["types"].append({"name": "Tracker", "qualified_name": "Editor.Tracker", "namespace": "Editor", "kind": "interface", "visibility": "public", "line": 2})
        with self.assertRaisesRegex(port.InventoryError, "Every declared type"):
            self.validate()
        mapping = self.mappings["files"][0]
        mapping["unmapped_types"] = ["Editor.Tracker"]
        with self.assertRaisesRegex(port.InventoryError, "Mapped status requires"):
            self.validate()
        mapping["status"] = "adapted"
        self.assertEqual(self.validate()[0]["unmapped_types"], ["Editor.Tracker"])

    def test_alias_requires_declared_original_and_exact_destination(self):
        alias = self.mappings["files"][0]["targets"][0]["aliases"][0]
        alias["upstream"] = "WrongNamespace.Document"
        with self.assertRaisesRegex(port.InventoryError, "undeclared upstream type"):
            self.validate()
        alias["upstream"] = "Editor.Document"
        alias["rust"] = "Document::replace"
        with self.assertRaisesRegex(port.InventoryError, "requires adapted"):
            self.validate()
        self.mappings["files"][0]["status"] = "adapted"
        self.validate()

    def test_unmapped_file_is_preserved_in_report(self):
        mapping = self.mappings["files"][0]
        mapping.update(status="unmapped", targets=[], unmapped_types=["Editor.Document"], notes="Document API is absent.")
        rows = self.validate()
        status, markdown = port.render(self.inventory, self.mappings, rows)
        self.assertEqual(status["counts"], {"mapped": 1, "adapted": 0, "unmapped": 1})
        self.assertIn("Document API is absent.", markdown)
        self.assertIn("not a completeness or behavioral-parity score", markdown)

    def test_generation_is_deterministic_and_check_is_offline(self):
        self.write_inputs()
        with patch.object(port, "git", side_effect=AssertionError("Offline check must not read upstream Git")):
            self.assertTrue(port.update_outputs(self.repo))
            self.assertFalse(port.update_outputs(self.repo, check=True))
            outputs = port.generate(self.repo)
            self.assertEqual(outputs, port.generate(self.repo))
            (self.repo / port.REPORT).write_text("stale")
            with self.assertRaisesRegex(port.InventoryError, "Stale generated editor reports"):
                port.update_outputs(self.repo, check=True)

    def test_capture_uses_committed_git_objects_and_checks_pin(self):
        upstream = self.repo / "upstream"
        upstream.mkdir()
        def git(*args):
            return subprocess.run(["git", "-C", str(upstream), *args], check=True, capture_output=True).stdout.decode().strip()
        git("init", "-q")
        source = upstream / "src/Editor/Document.cs"
        source.parent.mkdir(parents=True)
        source.write_text("namespace Editor { public class Document {} }\n")
        body = source.read_bytes()
        (source.parent / "View.xaml").write_text("<Control/>\n")
        (source.parent / "View.axaml").write_text("<Control/>\n")
        (source.parent / "Ignored.txt").write_text("Sample input\n")
        git("add", ".")
        git("-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "commit", "-qm", "Fixture")
        revision = git("rev-parse", "HEAD")
        source.write_text("uncommitted, deliberately invalid input")
        pin = {**port.PINS["avaloniaedit"], "commit": revision}
        with patch.dict(port.PINS, {"avaloniaedit": pin}):
            inventory = port.capture_repository("avaloniaedit", upstream)
        self.assertEqual(len(inventory["files"]), 3)
        self.assertEqual([file["kind"] for file in inventory["files"]], ["csharp", "xaml", "xaml"])
        captured = inventory["files"][0]
        self.assertEqual(captured["sha256"], port.sha256(body))
        self.assertEqual(captured["types"][0]["qualified_name"], "Editor.Document")
        with self.assertRaisesRegex(port.InventoryError, "expected checkout HEAD"):
            port.capture_repository("avaloniaedit", upstream)


if __name__ == "__main__":
    unittest.main()
