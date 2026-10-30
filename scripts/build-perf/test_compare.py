"""Acceptance tests for the benchmark's evidence checks, not timing assertions."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("compare", Path(__file__).with_name("compare.py"))
compare = importlib.util.module_from_spec(spec)
spec.loader.exec_module(compare)


class EvidenceTests(unittest.TestCase):
    def test_build_tool_override_allows_only_disclosed_optimization(self):
        unit = {"package": "registry+example#syn@2.0", "profile": {"opt_level": "0", "debug_assertions": True}, "features": ["full"]}
        changed = {**unit, "profile": {"opt_level": "2", "debug_assertions": True}}
        self.assertNotEqual(compare.comparable_units([unit], False), compare.comparable_units([changed], False))
        self.assertEqual(compare.comparable_units([unit], True), compare.comparable_units([changed], True))
        changed["profile"]["debug_assertions"] = False
        self.assertNotEqual(compare.comparable_units([unit], True), compare.comparable_units([changed], True))
        runtime = {**unit, "package": "path+file://$SOURCE/src/FerroUI.Base#ferroui-base@0.4.0"}
        runtime_changed = {**runtime, "profile": {"opt_level": "2", "debug_assertions": True}}
        self.assertNotEqual(compare.comparable_units([runtime], True), compare.comparable_units([runtime_changed], True))

    def test_build_tool_override_matches_the_checked_configuration_and_rejects_other_levels(self):
        import tomllib

        configuration = Path(__file__).resolve().parents[2] / ".cargo/build-tools.toml"
        parsed = tomllib.loads(configuration.read_text())
        packages = parsed["profile"]["dev"]["package"]
        # Cargo's JSON profile does not expose every compiler setting (for example,
        # panic strategy or codegen-unit count). Keep the opt-in file itself narrow.
        self.assertEqual(parsed, {"profile": {"dev": {"package": packages}}})
        self.assertEqual(set(packages), compare.BUILD_TOOL_PACKAGES)
        for package, settings in packages.items():
            self.assertEqual(settings, {"opt-level": int(compare.BUILD_TOOL_OPT_LEVEL)})
            unit = {"package": f"registry+example#{package}@2.0",
                    "profile": {"opt_level": "0", "debug_assertions": True}, "features": ["full"]}
            for level in ("1", "3", "s", "z"):
                changed = {**unit, "profile": {"opt_level": level, "debug_assertions": True}}
                self.assertNotEqual(compare.comparable_units([unit], True), compare.comparable_units([changed], True))

    def test_changed_features_or_generated_content_fail_equivalence(self):
        unit = {"features": ["full"], "fresh": False}
        baseline = {"label": "baseline", "seconds": 20, "units": [unit], "generated": {"metadata": ["a"]}}
        candidate = {"label": "candidate", "seconds": 2, "units": [unit], "generated": {"metadata": ["a"]}}
        result = compare.summarize([baseline, candidate])
        self.assertEqual(result["speedup"], 10)
        self.assertTrue(result["same_units_features_profiles"])
        changed_features = dict(candidate, units=[{"features": [], "fresh": False}])
        self.assertFalse(compare.summarize([baseline, changed_features])["same_units_features_profiles"])
        changed_content = dict(candidate, generated={"metadata": ["stale"]})
        self.assertFalse(compare.summarize([baseline, changed_content])["same_generated_rust_and_metadata"])

    def test_individual_samples_use_medians_and_preserve_multiplicity(self):
        records = [{"label": label, "seconds": seconds, "units": [], "generated": {"x": ["a", "a"]}}
                   for label, seconds in [("baseline", 10), ("baseline", 12), ("baseline", 300),
                                          ("candidate", 1), ("candidate", 2), ("candidate", 200)]]
        result = compare.summarize(records)
        self.assertEqual(result["speedup"], 6)
        self.assertEqual(result["generated_files_per_build"], 2)
        records[-1]["generated"] = {"x": ["a"]}
        self.assertFalse(compare.summarize(records)["same_generated_rust_and_metadata"])

    def test_only_source_and_target_paths_are_normalized(self):
        source = Path("/root/work/source")
        target = source / "target"
        self.assertEqual(compare.normalized(f"{target}/x {source}/y actual-change", source, target),
                         "$TARGET/x $SOURCE/y actual-change")

    def test_generated_files_come_from_cargo_protocol_and_content_changes_remain_visible(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            out = root / "arbitrary-out"
            out.mkdir()
            (out / "model.xamlmeta").write_text("first")
            (out / "timing.txt").write_text("99 seconds")
            messages = [{"reason": "build-script-executed", "package_id": "scan#1", "out_dir": str(out)}]
            before = compare.generated_outputs(messages, root / "source", root / "target")
            (out / "model.xamlmeta").write_text("second")
            after = compare.generated_outputs(messages, root / "source", root / "target")
            self.assertNotEqual(before, after)
            self.assertEqual(list(before), ["scan#1/model.xamlmeta"])


if __name__ == "__main__":
    unittest.main()
