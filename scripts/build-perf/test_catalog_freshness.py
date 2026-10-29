"""Acceptance checks for freshness and semantic-equivalence evidence, not timings."""
import importlib.util
import json
import os
from pathlib import Path
import tempfile
import unittest


spec = importlib.util.spec_from_file_location("catalog_freshness", Path(__file__).with_name("catalog-freshness.py"))
catalog = importlib.util.module_from_spec(spec)
spec.loader.exec_module(catalog)
ROOT = next((parent for parent in Path(__file__).resolve().parents
             if (parent / "scripts/build-perf/compare.py").is_file()), None)
if os.environ.get("FERROUI_TEST_SOURCE"):
    ROOT = Path(os.environ["FERROUI_TEST_SOURCE"])
if ROOT is None:
    raise RuntimeError("Set FERROUI_TEST_SOURCE to a FerroUI checkout when testing the isolated prototype")
helpers, _ = catalog.load_helpers(ROOT)
PACKAGE = "path+file://$SOURCE/samples/ControlCatalog#control-catalog@0.1.0"


def record(*, fresh=False, dependency_fresh=True):
    return {"units": [
        {"package": PACKAGE, "name": "control_catalog", "kind": ["lib"],
         "features": ["default", "placeholder-branding"], "profile": {"opt_level": "0"}, "fresh": fresh},
        {"package": "registry+source#dependency@1.0.0", "name": "dependency", "kind": ["lib"],
         "features": [], "profile": {"opt_level": "0"}, "fresh": dependency_fresh}],
        "dirty_reasons": ["Dirty control-catalog v0.1.0 ($SOURCE/samples/ControlCatalog): the file `$TARGET/out/placeholder/App.xaml` has changed"]}


class FreshnessAcceptanceTests(unittest.TestCase):
    def test_settled_baseline_and_any_dirty_candidate_are_rejected(self):
        valid = catalog.freshness_checks(record(), PACKAGE, "baseline", "repeat", True)
        self.assertTrue(valid["self_output_dirty_reason"])
        catalog.freshness_checks(record(fresh=True), PACKAGE, "candidate", "repeat")
        for item, label in [(record(fresh=True), "baseline"), (record(), "candidate"),
                            (record(fresh=True, dependency_fresh=False), "candidate")]:
            with self.assertRaises(RuntimeError):
                catalog.freshness_checks(item, PACKAGE, label, "repeat", True)
        clean_control = catalog.freshness_checks(record(fresh=True), PACKAGE, "baseline", "repeat")
        self.assertTrue(clean_control["all_artifacts_fresh"])
        mixed = catalog.freshness_checks(record(dependency_fresh=False), PACKAGE, "baseline", "repeat", True)
        self.assertEqual(len(mixed["non_catalog_dirty_artifacts"]), 1)

    def test_wrong_dirty_reason_runtime_shortcut_and_cached_population_are_rejected(self):
        item = record()
        item["dirty_reasons"] = ["Dirty control-catalog v0.1.0: the file lib.rs has changed"]
        with self.assertRaises(RuntimeError):
            catalog.freshness_checks(item, PACKAGE, "baseline", "repeat", True)
        item = record()
        item["units"][0]["features"].append("runtime-markup")
        with self.assertRaises(RuntimeError):
            catalog.freshness_checks(item, PACKAGE, "baseline", "repeat")
        with self.assertRaises(RuntimeError):
            catalog.freshness_checks(record(fresh=True), PACKAGE, "candidate", "population")

    def test_unit_equivalence_ignores_only_freshness(self):
        self.assertEqual(catalog.definitions(record()), catalog.definitions(record(fresh=True)))
        changed = record()
        changed["units"][0]["profile"]["opt_level"] = "2"
        self.assertNotEqual(catalog.definitions(record()), catalog.definitions(changed))

    def test_original_watch_recognition_and_optional_config_scope(self):
        old = '''substitute_placeholder_geometries(&root, &out_dir, &mut assets);
for (asset_path, path) in &assets {
    let path = path.canonicalize().unwrap_or_else(|e| panic!("cannot resolve: {e}"));
    println!("cargo::rerun-if-changed={}", path.display());
}'''
        self.assertTrue(catalog.original_self_input_form(old))
        self.assertFalse(catalog.original_self_input_form(old.replace('println!("cargo::rerun-if-changed={}", path.display());',
                                                                     'if !transformed.contains_key(asset_path) { println!("cargo::rerun-if-changed={}", path.display()); }')))
        files = {"Cargo.toml": {"sha256": "manifest"}, ".cargo/config.toml": {"sha256": "automatic"}}
        self.assertEqual(catalog.source_configuration(files), catalog.source_configuration({**files, ".cargo/build-tools.toml": {"sha256": "unused opt-in"}}))
        changed = record()
        changed["units"][0]["features"].remove("placeholder-branding")
        self.assertNotEqual(catalog.definitions(record()), catalog.definitions(changed))


class OutputAcceptanceTests(unittest.TestCase):
    def test_only_exact_representation_and_timing_files_are_exceptions(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source, target, out = root / "source", root / "target", root / "target/generated"
            (out / "placeholder").mkdir(parents=True)
            (out / "placeholder/App.xaml").write_bytes(b"<Application />\r\n")
            (out / "model.xamlmeta").write_text('{"model": 1}')
            (out / "assets.rs").write_text("first representation")
            (out / "xamlmeta-scan.txt").write_text("1 millisecond")
            (out / "unfamiliar.bin").write_bytes(b"opaque meaningful bytes")
            package = f"path+file://{source}/samples/ControlCatalog#control-catalog@0.1.0"
            messages = [{"reason": "build-script-executed", "package_id": package, "out_dir": str(out)}]
            metadata = {"workspace_members": [package]}
            first = catalog.snapshot_outputs(messages, source, target, root / "one", metadata, package, helpers)
            (out / "assets.rs").write_text("equivalent different representation")
            (out / "xamlmeta-scan.txt").write_text("987 milliseconds")
            second = catalog.snapshot_outputs(messages, source, target, root / "two", metadata, package, helpers)
            self.assertEqual(first["first_party_semantic_files"], second["first_party_semantic_files"])
            self.assertEqual(first["generated_rust_and_metadata"], second["generated_rust_and_metadata"])
            self.assertEqual(len(first["additional_first_party_outputs"]), 1)
            (out / "unfamiliar.bin").write_bytes(b"changed bytes")
            third = catalog.snapshot_outputs(messages, source, target, root / "three", metadata, package, helpers)
            self.assertNotEqual(first["first_party_semantic_files"], third["first_party_semantic_files"])
            (out / "placeholder/App.xaml").write_bytes(b"<Application />\n")
            fourth = catalog.snapshot_outputs(messages, source, target, root / "four", metadata, package, helpers)
            self.assertNotEqual(third["first_party_semantic_files"], fourth["first_party_semantic_files"])
            self.assertEqual(catalog.file_role(PACKAGE, PACKAGE, "nested/xamlmeta-scan.txt"), "additional-first-party-output-compared")
            self.assertEqual(catalog.file_role("other", PACKAGE, "assets.rs"), "generated-semantic-output")

    def test_normalization_keeps_content_and_newline_changes(self):
        locations = [(b"/owned/target/out", b"$OUT/catalog"), (b"/owned/source", b"$SOURCE")]
        self.assertEqual(catalog.normalize_locations(b"/owned/target/out/f actual-deadbeef\r\n", locations),
                         b"$OUT/catalog/f actual-deadbeef\r\n")
        self.assertNotEqual(catalog.normalize_locations(b"actual-deadbeef\r\n", locations),
                            catalog.normalize_locations(b"actual-deadbeef\n", locations))

    def test_binary_probe_compares_all_tables_without_normalizing_asset_bytes(self):
        rustc = os.environ.get("CATALOG_BENCH_TEST_RUSTC", "rustc")
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary) / 'paths with spaces and "## quoted'
            root.mkdir()
            app = root / "App.xaml"
            app.write_bytes('<App name="雪">line\\path\r\n</App>'.encode())
            common = '''pub(crate) static ASSETS: &[(&str, &[u8])] = &[("/image", b"\\0\\xff")];
pub(crate) static DOCUMENTS: &[(&str, Option<&str>)] = &[("/App.xaml", Some("雪.App"))];
pub(crate) static EXCLUDED: &[ExcludedDocument] = &[ExcludedDocument { path: "/App.xaml", page_only: true, reason: "existing reason" }];
'''
            before = root / "before.rs"
            after = root / "after.rs"
            before.write_text(common + '#[cfg(test)] pub(crate) static DOCUMENT_ASSETS: &[(&str, &[u8])] = &[("/App.xaml", include_bytes!('
                              + catalog.rust_literal(str(app)) + '))];\n', encoding="utf-8")
            after.write_text(common + '#[cfg(test)] pub(crate) static DOCUMENT_ASSETS: &[(&str, &[u8])] = &[("/App.xaml", '
                             + json.dumps(app.read_bytes().decode(), ensure_ascii=False) + '.as_bytes())];\n', encoding="utf-8")
            baseline = catalog.run_probe(before, root / "baseline", root, dict(os.environ), rustc)
            candidate = catalog.run_probe(after, root / "candidate", root, dict(os.environ), rustc)
            self.assertEqual(Path(baseline["binary_file"]).read_bytes(), Path(candidate["binary_file"]).read_bytes())
            self.assertEqual(baseline["tables"]["DOCUMENT_ASSETS"]["app_xaml_entries"], 1)
            self.assertEqual(baseline["tables"]["EXCLUDED"]["entries"], 1)
            changed = after.read_text().replace("page_only: true", "page_only: false").replace("existing reason", "changed reason")
            after.write_text(changed, encoding="utf-8")
            different = catalog.run_probe(after, root / "different", root, dict(os.environ), rustc)
            self.assertNotEqual(Path(baseline["binary_file"]).read_bytes(), Path(different["binary_file"]).read_bytes())
            app.write_bytes(b"changed asset bytes at unchanged path")
            changed_asset = catalog.run_probe(before, root / "changed-asset", root, dict(os.environ), rustc)
            self.assertNotEqual(Path(baseline["binary_file"]).read_bytes(), Path(changed_asset["binary_file"]).read_bytes())
            truncated = root / "truncated.bin"
            truncated.write_bytes(Path(baseline["binary_file"]).read_bytes()[:-1])
            with self.assertRaises(RuntimeError):
                catalog.inspect_probe(truncated)


if __name__ == "__main__":
    unittest.main()
