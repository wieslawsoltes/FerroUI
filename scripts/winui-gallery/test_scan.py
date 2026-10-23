"""Regression fixtures for the source accountant's failure-prone boundaries."""

import importlib.util
from pathlib import Path
import tempfile
import unittest
import sys

SPEC = importlib.util.spec_from_file_location("winui_gallery_scan", Path(__file__).with_name("scan.py"))
scan = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = scan
SPEC.loader.exec_module(scan)


class ApiParserTests(unittest.TestCase):
    def test_midl_versions_attributes_properties_events_and_overloads(self):
        source = '''namespace MU_XC_NAMESPACE {
            [MUX_PUBLIC] runtimeclass Example : Microsoft.UI.Xaml.Controls.Control {
                Example();
                [MUX_DEFAULT_VALUE("{not a body}")] Double Value;
                event Windows.Foundation.TypedEventHandler<Example, Object> Changed;
                [MUX_PUBLIC_V3] {
                    String Text { get; set; };
                    void Update(Int32 count);
                    void Update(String text);
                }
            }
        }'''
        result = scan.parse_api(source, "example.idl", "midl3")
        self.assertEqual(len(result), 1)
        self.assertEqual(result[0]["id"], "Microsoft.UI.Xaml.Controls.Example")
        self.assertEqual(result[0]["base"], "Microsoft.UI.Xaml.Controls.Control")
        members = result[0]["members"]
        self.assertEqual([m["name"] for m in members], ["Example", "Value", "Changed", "Text", "Update", "Update"])
        self.assertEqual(members[0]["kind"], "constructor")
        self.assertEqual(members[2]["kind"], "event")
        self.assertNotEqual(members[4]["signature"], members[5]["signature"])

    def test_csharp_only_public_members_and_private_contracts(self):
        source = '''namespace Microsoft.UI.Xaml.Controls {
            // public class NotAType { public int Fake; }
            [Platform(typeof(PrivateApiContract), 1)]
            public class PrivateThing {
                public int Present { get; internal set; }
                internal void Secret() { }
                private void AlsoSecret() { }
                public void Update(int count) { var misleading = "public class Wrong {}"; }
            }
        }'''
        result = scan.parse_api(source, "example.cs", "csharp-model")
        self.assertEqual([r["name"] for r in result], ["PrivateThing"])
        self.assertEqual(result[0]["audience"], "private")
        self.assertEqual([m["name"] for m in result[0]["members"]], ["Present", "Update"])
        self.assertIn("internal set", result[0]["members"][0]["signature"])

    def test_enum_values_and_namespace_identity(self):
        source = '''namespace Microsoft.UI.Xaml {
            public enum Mode { First = 3, [Some] Second = 7, Third }
        }
        namespace Microsoft.UI.Xaml.Controls { public class Mode {} }
        '''
        result = scan.parse_api(source, "example.cs", "csharp-model")
        self.assertEqual([r["id"] for r in result], ["Microsoft.UI.Xaml.Mode", "Microsoft.UI.Xaml.Controls.Mode"])
        self.assertEqual([m["name"] for m in result[0]["members"]], ["First", "Second", "Third"])

    def test_private_midl_block_remains_private(self):
        source = '''namespace MU_XC_NAMESPACE { runtimeclass Sample {
            Boolean Public;
            [MUX_PRIVATE] { Boolean Secret; }
        } }'''
        result = scan.parse_api(source, "sample.idl", "midl3")[0]["members"]
        self.assertEqual([r["audience"] for r in result], ["public-source", "private"])

    def test_delegates_and_projected_overload_names_are_preserved(self):
        source = '''namespace Microsoft.UI.Xaml.Controls {
            public delegate void ChangedHandler(Object sender, EventArgs args);
            public class Frame {
                [DXamlOverloadName("Navigate")]
                public bool NavigateWithOptions(Type source, Object value) { }
            }
        }'''
        result = scan.parse_api(source, "sample.cs", "csharp-model")
        self.assertEqual(result[0]["kind"], "delegate")
        self.assertEqual(result[0]["members"][0]["name"], "Invoke")
        self.assertEqual(result[1]["members"][0]["name"], "NavigateWithOptions")
        self.assertEqual(result[1]["members"][0]["projected_name"], "Navigate")


class XamlParserTests(unittest.TestCase):
    def test_expanded_names_attached_properties_and_nested_extensions(self):
        source = '''<Page xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
          xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"
          xmlns:local="using:WinUIGallery.Controls">
          <Button x:Name="Clicker" Grid.Row="1" Content="{Binding Title, Converter={local:Converter}}" />
          <local:Example><local:Example.Example><TextBlock Text="{x:Bind ViewModel.Label}" /></local:Example.Example></local:Example>
        </Page>'''
        types = {n: ["Microsoft.UI.Xaml.Controls." + n] for n in ("Page", "Button", "Grid", "TextBlock")}
        result = scan.parse_xaml(source, "sample.xaml", types)
        self.assertEqual(result["diagnostics"], [])
        self.assertEqual(result["members"]["Microsoft.UI.Xaml.Controls.Grid.Row"], 1)
        self.assertEqual(result["members"]["WinUIGallery.Controls.Example.Example"], 1)
        self.assertEqual(result["features"]["x:Name"], 1)
        self.assertEqual(result["features"]["markup:x:Bind"], 1)
        self.assertIn("WinUIGallery.Controls.Converter", result["types"])

    def test_namespace_prefix_shadowing(self):
        source = '''<Page xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" xmlns:c="using:Outer">
          <TextBlock Text="{c:Formatter}" />
          <Grid xmlns:c="using:Inner"><TextBlock Text="{c:Formatter}" /></Grid>
          <TextBlock Text="{c:Formatter}" />
        </Page>'''
        result = scan.parse_xaml(source, "sample.xaml", {})
        self.assertEqual(result["types"]["Outer.Formatter"], 2)
        self.assertEqual(result["types"]["Inner.Formatter"], 1)

    def test_custom_element_attached_owner_uses_default_namespace(self):
        source = '''<Page xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"
          xmlns:custom="using:Gallery" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
          mc:Ignorable="custom"><custom:Example Grid.Row="1" /></Page>'''
        result = scan.parse_xaml(source, "sample.xaml", {"Grid": ["Microsoft.UI.Xaml.Controls.Grid"]})
        self.assertIn("Microsoft.UI.Xaml.Controls.Grid.Row", result["members"])
        self.assertNotIn("Gallery.Grid.Row", result["members"])
        self.assertEqual(result["features"]["mc:Ignorable"], 1)
        self.assertFalse(any(x.endswith(".Ignorable") for x in result["members"]))

    def test_escaped_and_literal_braces_are_not_markup_extensions(self):
        source = '<Page Text="{}{day.integer}" Header="Example {ordinary}" Other="{Binding Name}" />'
        result = scan.parse_xaml(source, "sample.xaml", {})
        self.assertEqual(result["features"], {"markup:Binding": 1})

    def test_bad_xml_is_an_explicit_diagnostic(self):
        result = scan.parse_xaml("<Page><Button></Page>", "bad.xaml", {})
        self.assertEqual(result["diagnostics"][0]["path"], "bad.xaml")
        self.assertEqual(result["diagnostics"][0]["kind"], "xml-parse")


class CandidateTests(unittest.TestCase):
    def test_inheritance_and_ambiguous_base_do_not_guess(self):
        rust = {
            "FerroUI.Controls.Button": {"name": "Button", "namespace": "FerroUI.Controls", "base": "ContentControl", "members": {"click"}},
            "FerroUI.Controls.ContentControl": {"name": "ContentControl", "namespace": "FerroUI.Controls", "base": "Control", "members": {"content"}},
            "FerroUI.Controls.Control": {"name": "Control", "namespace": "FerroUI.Controls", "base": None, "members": {"width"}},
        }
        self.assertEqual(scan.inherited_member(rust, "FerroUI.Controls.Button", "width"), "FerroUI.Controls.Control")
        rust["Other.Control"] = {"name": "Control", "namespace": "Other", "base": None, "members": {"missing"}}
        self.assertIsNone(scan.inherited_member(rust, "FerroUI.Controls.Button", "missing"))

    def test_inheritance_cycles_terminate(self):
        rust = {"N.A": {"name": "A", "namespace": "N", "base": "B", "members": set()},
                "N.B": {"name": "B", "namespace": "N", "base": "A", "members": set()}}
        self.assertIsNone(scan.inherited_member(rust, "N.A", "missing"))

    def test_namespaces_choose_longest_module_prefix(self):
        mappings = [("ferroui_controls", "FerroUI.Controls"),
                    ("ferroui_controls::automation", "FerroUI.Automation"),
                    ("ferroui_controls::automation::peers", "FerroUI.Automation.Peers")]
        result = scan.rust_namespace("ferroui_controls", Path("automation/peers/button_peer.rs"), mappings)
        self.assertEqual(result, "FerroUI.Automation.Peers")
        self.assertIsNone(scan.rust_namespace("other", Path("controls.rs"), mappings))

    def test_candidate_is_not_behavioral_parity(self):
        api = {"types": [{"id": "Microsoft.UI.Xaml.Controls.Button", "namespace": "Microsoft.UI.Xaml.Controls", "name": "Button",
                          "audience": "public-source", "members": [{"name": "Content", "kind": "property", "signature": "Object Content", "audience": "public-source"}]}]}
        rust = {"FerroUI.Controls.Button": {"name": "Button", "namespace": "FerroUI.Controls", "base": None,
                                           "members": {"content"}, "sources": [], "registered": True}}
        result = scan.compare_api(api, rust, {"types": {}})["types"][0]
        self.assertEqual(result["members"][0]["status"], "source-candidate")
        self.assertEqual(result["members"][0]["behavior"], "unverified")
        self.assertEqual(result["behavior"], "unverified")

    def test_constructor_overloads_can_map_to_distinct_native_entry_points(self):
        api = {"types": [{"id": "Microsoft.UI.Xaml.Controls.SymbolIcon", "namespace": "Microsoft.UI.Xaml.Controls", "name": "SymbolIcon",
                          "audience": "public-source", "members": [
                              {"name": "SymbolIcon", "kind": "constructor", "signature": "public SymbolIcon()", "audience": "public-source"},
                              {"name": "SymbolIcon", "kind": "constructor", "signature": "public SymbolIcon(Symbol symbol)", "audience": "public-source"},
                          ]}]}
        rust = {"FerroUI.Controls.SymbolIcon": {"name": "SymbolIcon", "namespace": "FerroUI.Controls", "base": None,
                                                     "members": {"new", "with_symbol"}, "sources": [], "registered": True}}
        mappings = {"types": {"Microsoft.UI.Xaml.Controls.SymbolIcon": {
            "target": "FerroUI.Controls.SymbolIcon",
            "overloads": {"public SymbolIcon(Symbol symbol)": "with_symbol"},
        }}}
        result = scan.compare_api(api, rust, mappings)["types"][0]
        self.assertEqual([member["rust_member"] for member in result["members"]], ["new", "with_symbol"])
        self.assertTrue(all(member["behavior"] == "unverified" for member in result["members"]))

    def test_upstream_inherited_property_resolves_declaring_owner(self):
        api = {"N.Button": {"namespace": "N", "name": "Button", "base": "N.Control", "members": []},
               "N.Control": {"namespace": "N", "name": "Control", "base": None,
                             "members": [{"name": "Width", "kind": "property"}]}}
        result = scan.upstream_member(api, "N.Button", "Width")
        self.assertEqual(result["declaring_type"], "N.Control")
        self.assertIsNone(scan.upstream_member(api, "N.Button", "DoesNotExist"))


class EvidenceTests(unittest.TestCase):
    def page(self):
        return {"pages": [{"id": "Button", "title": "Button", "category": {"title": "Basic"},
                           "examples": [{"source": "Button.txt"}], "required_types": []}]}

    def test_missing_entries_remain_pending(self):
        result = scan.page_map(Path("."), self.page(), {"types": []}, {"pages": {}})
        self.assertEqual(result["pages"][0]["status"], "pending")

    def test_unknown_page_ids_fail(self):
        with self.assertRaisesRegex(ValueError, "Unknown page IDs"):
            scan.page_map(Path("."), self.page(), {"types": []}, {"pages": {"Typo": {}}})

    def test_test_names_are_not_execution_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "sample.rs").write_text("fn button_click() {}")
            manifest = {"pages": {"Button": {"status": "verified", "implementation": ["sample.rs"],
                        "tests": [{"path": "sample.rs", "name": "button_click"}],
                        "examples": [{"source": "Button.txt", "status": "verified"}]}}}
            with self.assertRaisesRegex(ValueError, "execution record"):
                scan.page_map(root, self.page(), {"types": []}, manifest)

    def test_evidence_cannot_escape_repository(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "repo"
            root.mkdir()
            (root.parent / "outside.rs").write_text("fn fake_test() {}")
            with self.assertRaisesRegex(ValueError, "Invalid or missing evidence"):
                scan.evidence_path(root, "../outside.rs")

    def test_execution_requires_current_source_and_passed_test(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "sample.rs"
            source.write_text("fn exercises_contract() {}")
            record = {"schema_version": 1, "command": ["cargo", "test"], "result": "passed",
                      "passed_tests": ["exercises_contract"],
                      "source_sha256": {"sample.rs": scan.digest(source.read_bytes())}}
            (root / "execution.json").write_text(scan.canonical(record))
            tests = [{"path": "sample.rs", "name": "exercises_contract"}]
            verified = scan.verify_execution(root, {"record": "execution.json"}, ["sample.rs"], tests)
            self.assertEqual(verified["result"], "passed")
            with self.assertRaisesRegex(ValueError, "does not cover"):
                scan.verify_execution(root, {"record": "execution.json"}, ["sample.rs"], [])
            source.write_text("fn exercises_contract() { changed(); }")
            with self.assertRaisesRegex(ValueError, "stale"):
                scan.verify_execution(root, {"record": "execution.json"}, ["sample.rs"], tests)

    def test_unknown_api_contract_fails(self):
        with self.assertRaisesRegex(ValueError, "Unknown API contract"):
            scan.apply_api_evidence(Path("."), {"types": []}, {"members": {"missing": {"status": "partial"}}})

    def test_partial_api_execution_is_checked_without_promoting_behavior(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "sample.rs"
            source.write_text("fn exercises_subset() {}")
            record = {"schema_version": 1, "command": ["cargo", "test"], "result": "passed",
                      "passed_tests": ["exercises_subset"],
                      "source_sha256": {"sample.rs": scan.digest(source.read_bytes())}}
            (root / "execution.json").write_text(scan.canonical(record))
            evidence = {"status": "partial", "implementation": ["sample.rs"],
                        "tests": [{"path": "sample.rs", "name": "exercises_subset"}],
                        "execution": {"record": "execution.json"}}
            for scope, identity in (("types", "N.Widget"), ("members", "N.Widget::property:Value")):
                mapping = {"types": [{"upstream": "N.Widget", "behavior": "unverified",
                           "members": [{"id": "N.Widget::property:Value", "behavior": "unverified", "audience": "public-source"}]}]}
                manifest = {scope: {identity: evidence}}
                scan.apply_api_evidence(root, mapping, manifest)
                row = mapping["types"][0] if scope == "types" else mapping["types"][0]["members"][0]
                self.assertEqual(row["behavior"], "partial")
                self.assertEqual(row["execution"]["result"], "passed")
                source.write_text("fn exercises_subset() { changed(); }")
                with self.assertRaisesRegex(ValueError, "stale"):
                    scan.apply_api_evidence(root, mapping, manifest)
                source.write_text("fn exercises_subset() {}")


class SummaryTests(unittest.TestCase):
    @staticmethod
    def member(kind="property", behavior="unverified", audience="public-source"):
        return {"kind": kind, "behavior": behavior, "audience": audience, "status": "source-candidate"}

    def summarize(self, types, pages=None):
        api = {"limitations": [], "types": [{"id": item["upstream"], "kind": "enum" if item["upstream"].endswith("Symbol") else "class"} for item in types]}
        catalog = {"pages": [], "unlisted_control_info_ids": []}
        return scan.summarize(api, catalog, {"code": [], "xaml": []}, {"types": types}, {"pages": pages or []})

    def test_behavior_counts_exclude_private_types_and_members_and_separate_enum_values(self):
        types = [
            {"upstream": "Microsoft.UI.Xaml.Controls.Symbol", "audience": "public-source", "behavior": "verified", "status": "source-candidate",
             "members": [self.member("enum-value", "verified"), self.member("enum-value", "verified"), self.member("enum-value", "verified", "private")]},
            {"upstream": "N.Partial", "audience": "public-source", "behavior": "partial", "status": "source-candidate", "members": [self.member(behavior="partial")]},
            {"upstream": "N.Candidate", "audience": "public-source", "behavior": "unverified", "status": "source-candidate", "members": [self.member()]},
            {"upstream": "N.Private", "audience": "private", "behavior": "verified", "status": "source-candidate", "members": [self.member(behavior="verified")]},
        ]
        summary = self.summarize(types)["upstream"]
        self.assertEqual(summary["types_public_source"], 3)
        self.assertEqual(summary["types_private_source"], 1)
        self.assertEqual(summary["members_public_source"], 4)
        self.assertEqual(summary["types_behavior"], {"verified": 1, "partial": 1, "unverified": 1})
        self.assertEqual(summary["members_behavior"], {"verified": 2, "partial": 1, "unverified": 1})
        self.assertEqual(summary["non_enum_members_behavior"], {"verified": 0, "partial": 1, "unverified": 1})
        self.assertEqual(summary["verified_enum_values_by_type"], [{"type": "Microsoft.UI.Xaml.Controls.Symbol", "count": 2}])
        self.assertEqual(summary["verified_types"], [{"id": "Microsoft.UI.Xaml.Controls.Symbol", "kind": "enum", "public_members": 2}])

    def test_declaration_candidate_never_becomes_verified_in_summary(self):
        types = [{"upstream": "N.Button", "audience": "public-source", "status": "source-candidate", "members": [self.member()]}]
        summary = self.summarize(types)["upstream"]
        self.assertEqual(summary["type_candidates"], {"source-candidate": 1})
        self.assertEqual(summary["types_behavior"], {"verified": 0, "partial": 0, "unverified": 1})
        self.assertEqual(summary["members_behavior"], {"verified": 0, "partial": 0, "unverified": 1})

    def test_report_shows_page_state_totals_and_behavior_separately(self):
        pages = [{"id": str(index), "category": "Basic", "status": state, "examples_total": 1}
                 for index, state in enumerate(["pending", "partial", "partial", "ported", "verified"])]
        summary = self.summarize([], pages)
        self.assertEqual(summary["gallery"]["page_statuses"], {"pending": 1, "partial": 2, "ported": 1, "verified": 1})
        report = scan.report(summary, {"pages": pages}, {"repositories": {}})
        self.assertIn("| Type contracts | 0 | 0 | 0 |", report)
        self.assertIn("| partial | 2 |", report)
        self.assertLess(report.index("| Page state | Count |"), report.index("| Page | Category | State |"))
        self.assertIn("Members excluding enum values", report)
        self.assertIn("No complete type contract has been verified", report)


if __name__ == "__main__":
    unittest.main()
