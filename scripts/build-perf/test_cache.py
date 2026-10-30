"""Acceptance checks for cache evidence, isolation and invalidation orchestration.

These tests do not invoke Rust, sccache or a real build. The CI experiment runs a
separate real Rust fixture and the selected FerroUI correctness jobs.
"""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("cache_benchmark", Path(__file__).with_name("cache.py"))
cache = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cache)


def record(label, seconds, features=None):
    return {
        "label": label, "seconds": seconds, "exit_code": 0,
        "command": ["cargo", "build", "--lib", "--profile", "release"],
        "units": [{"name": "controls", "features": features or [], "fresh": False,
                   "profile": {"opt_level": "3", "debug_assertions": False}}],
        "generated": {"controls/model.xamlmeta": ["content-hash"]},
        "cache_counters": {"rust_cache_hits": 4, "cache_errors": 0},
    }


def stats(hits=1):
    return {"stats": {"compile_requests": 8,
                      "cache_hits": {"counts": {"Rust": hits, "C/C++": 2}, "adv_counts": {"rust": 200}},
                      "cache_misses": {"counts": {"Rust": 1}},
                      "cache_errors": {"counts": {}}}}


class CacheEvidenceTests(unittest.TestCase):
    def test_replay_median_excludes_population_and_preserves_samples(self):
        records = [record("population", 70)]
        records.extend(record("uncached", n) for n in (10, 12, 300))
        records.extend(record("replay", n) for n in (1, 2, 200))
        summary = cache.summarize(records)
        self.assertEqual(summary["local_populated_cache_speedup"], 6)
        self.assertEqual(summary["population_seconds"], [70])
        self.assertEqual(summary["uncached_seconds"], [10, 12, 300])
        self.assertTrue(summary["same_units_features_profiles"])

    def test_weakened_features_profile_or_generated_output_are_not_equivalent(self):
        records = [record("uncached", 100, ["full"]), record("population", 110, ["full"]),
                   record("replay", 1, ["full"])]
        self.assertTrue(cache.summarize(records)["same_generated_rust_and_metadata"])
        changed = copy.deepcopy(records)
        changed[-1]["units"][0]["features"] = []
        self.assertFalse(cache.summarize(changed)["same_units_features_profiles"])
        changed = copy.deepcopy(records)
        changed[-1]["units"][0]["profile"]["opt_level"] = "0"
        self.assertFalse(cache.summarize(changed)["same_units_features_profiles"])
        changed = copy.deepcopy(records)
        changed[-1]["generated"]["controls/model.xamlmeta"] = ["stale"]
        self.assertFalse(cache.summarize(changed)["same_generated_rust_and_metadata"])

    def test_cargo_no_op_and_zero_cache_hits_do_not_count_as_cache_replay(self):
        records = [record("uncached", 100), record("population", 110), record("replay", 1)]
        records[-1]["units"][0]["fresh"] = True
        records[-1]["cache_counters"]["rust_cache_hits"] = 0
        summary = cache.summarize(records)
        self.assertFalse(summary["all_cargo_units_rebuilt"])
        self.assertFalse(summary["every_replay_has_rust_cache_hits"])
        records[-1]["cache_counters"]["cache_write_errors"] = 1
        self.assertFalse(cache.summarize(records)["no_cache_errors"])

    def test_failed_missing_and_invalid_results_are_rejected(self):
        records = [record("uncached", 100), record("population", 110), record("replay", 1)]
        for invalid in (0, -1, float("nan"), float("inf")):
            with self.subTest(invalid=invalid):
                changed = copy.deepcopy(records)
                changed[-1]["seconds"] = invalid
                with self.assertRaisesRegex(RuntimeError, "invalid timing"):
                    cache.summarize(changed)
        records[-1]["exit_code"] = 1
        with self.assertRaises(RuntimeError):
            cache.summarize(records)
        with self.assertRaisesRegex(RuntimeError, "Missing"):
            cache.summarize(records[:2])

    def test_cache_statistics_do_not_double_count_advanced_breakdowns(self):
        counts = cache.cache_counters(stats(3))
        self.assertEqual(counts["cache_hits"], 5)
        self.assertEqual(counts["rust_cache_hits"], 3)
        self.assertEqual(counts["rust_cache_misses"], 1)
        with self.assertRaisesRegex(RuntimeError, "stats object"):
            cache.cache_counters({"unexpected": {}})

    def test_generated_output_paths_normalized_without_hiding_content_changes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "source"
            results = []
            for index in (1, 2):
                target = root / f"target-{index}"
                out = target / f"arbitrary-hash-{index}"
                out.mkdir(parents=True)
                (out / "generated.rs").write_text(f'// {source}/file.rs {out}/dependency.rs\npub const V:u8=1;')
                (out / "model.xamlmeta").write_text("metadata")
                (out / "timing.txt").write_text(str(index))
                messages = [{"reason": "build-script-executed", "package_id": "model#1", "out_dir": str(out)}]
                results.append(cache.build_evidence(messages, source, target))
            self.assertEqual(results[0], results[1])
            self.assertEqual(len(results[0]["generated"]), 2)
            (out / "model.xamlmeta").write_text("changed metadata")
            self.assertNotEqual(results[0], cache.build_evidence(messages, source, target))

    def test_generated_output_outside_owned_target_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            message = {"reason": "build-script-executed", "package_id": "p", "out_dir": str(root / "outside")}
            with self.assertRaisesRegex(RuntimeError, "escaped"):
                cache.build_evidence([message], root / "source", root / "target")

    def test_comparison_command_preserves_profiles_and_feature_selection(self):
        options = SimpleNamespace(cargo="cargo", package=["ferroui-controls", "ferroui-base"],
                                  features=["ferroui-base/testing"], profile="dist", jobs=4)
        command = cache.cargo_command(options)
        self.assertIn("dist", command)
        self.assertIn("ferroui-base/testing", command)
        self.assertNotIn("--no-default-features", command)
        self.assertEqual(command.count("-p"), 2)
        tests = cache.cargo_command(options, "test")
        self.assertNotIn("--no-run", tests)
        self.assertEqual(command[2:], tests[2:])

    def test_explicit_configs_apply_identically_to_build_test_and_fetch(self):
        options = SimpleNamespace(cargo="cargo", cargo_config=[Path("/workspace/build-tools.toml"), Path("/workspace/extra.toml")],
                                  package=["ferroui-controls"], features=[], profile="dev", jobs=4)
        prefix = ["cargo", "--config", "/workspace/build-tools.toml", "--config", "/workspace/extra.toml"]
        self.assertEqual(cache.cargo_prefix(options), prefix)
        build = cache.cargo_command(options)
        tests = cache.cargo_command(options, "test")
        self.assertEqual(build[:len(prefix)], prefix)
        self.assertEqual(tests[:len(prefix)], prefix)
        self.assertEqual(build[len(prefix)], "build")
        self.assertEqual(tests[len(prefix)], "test")
        self.assertEqual(build[len(prefix) + 1:], tests[len(prefix) + 1:])
        self.assertEqual((cache.cargo_prefix(options) + ["fetch", "--locked"])[:len(prefix)], prefix)

    def test_config_digest_detects_changes_even_outside_source_tree(self):
        with tempfile.TemporaryDirectory() as temporary:
            config = Path(temporary) / "build-tools.toml"
            config.write_text('[profile.dev.package.ferroui-build-scan]\nopt-level=3\n')
            before = cache.config_identity([config])
            self.assertEqual(before, cache.config_identity([config]))
            config.write_text('[profile.dev.package.ferroui-build-scan]\nopt-level=0\n')
            self.assertNotEqual(before, cache.config_identity([config]))


class IsolationTests(unittest.TestCase):
    def test_existing_cache_backend_and_wrappers_are_not_inherited(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            control = root / "control"
            control.mkdir()
            environment = {"PATH": "unchanged", "RUSTFLAGS": "-Cdebuginfo=2", "CARGO_INCREMENTAL": "1",
                           "RUSTC_WRAPPER": "foreign-wrapper", "SCCACHE_BUCKET": "foreign-bucket",
                           "SCCACHE_SERVER_PORT": "4226", "SCCACHE_CONF": "foreign-config",
                           "SCCACHE_RECACHE": "1"}
            isolated = cache.isolated_cache_environment(environment, root, control)
            self.assertNotIn("SCCACHE_BUCKET", isolated)
            self.assertNotIn("SCCACHE_RECACHE", isolated)
            self.assertNotEqual(isolated["SCCACHE_CONF"], "foreign-config")
            plain = cache.build_environment(isolated, root / "target", "")
            wrapped = cache.build_environment(isolated, root / "target", "/isolated/sccache")
            self.assertEqual(plain["CARGO_INCREMENTAL"], "0")
            self.assertEqual(wrapped["CARGO_INCREMENTAL"], "0")
            self.assertEqual(plain["RUSTC_WRAPPER"], "")
            self.assertEqual(plain["RUSTC_WORKSPACE_WRAPPER"], "")
            self.assertEqual(plain["RUSTFLAGS"], wrapped["RUSTFLAGS"])
            self.assertEqual(plain["CARGO_TARGET_DIR"], wrapped["CARGO_TARGET_DIR"])
            self.assertEqual(environment["RUSTC_WRAPPER"], "foreign-wrapper")

    def test_cleanup_only_removes_the_owned_target(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            target = root / "target"
            target.mkdir()
            (target / "artifact").write_text("owned")
            protected = root / "developer-target"
            protected.mkdir()
            (protected / "keep").write_text("keep")
            cache.reset_target(target, root)
            self.assertEqual(list(target.iterdir()), [])
            with self.assertRaises(RuntimeError):
                cache.reset_target(protected, root)
            self.assertTrue((protected / "keep").is_file())

    def test_source_fingerprint_ignores_report_but_detects_new_input_changes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            report = root / "report"
            report.mkdir()
            (root / "input.rs").write_text("first")
            (report / "result.json").write_text("first report")
            with patch.object(cache.subprocess, "check_output", return_value=b"input.rs\0report/result.json\0"):
                before = cache.source_fingerprint(root, report)
                (report / "result.json").write_text("later report")
                self.assertEqual(before, cache.source_fingerprint(root, report))
                (root / "input.rs").write_text("changed source")
                self.assertNotEqual(before, cache.source_fingerprint(root, report))


class FailureDetectionTests(unittest.TestCase):
    def test_exit_zero_without_cargo_build_success_is_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with patch.object(cache.subprocess, "run", return_value=SimpleNamespace(returncode=0)):
                with self.assertRaisesRegex(RuntimeError, "omitted successful"):
                    cache.run_build(["cargo", "build"], root, root / "target", {}, root / "result")

    def test_negative_control_cannot_pass_using_old_successful_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with patch.object(cache.subprocess, "run", return_value=SimpleNamespace(returncode=0)):
                with self.assertRaisesRegex(RuntimeError, "Negative control"):
                    cache.run_build(["cargo", "build"], root, root / "target", {}, root / "result", expect_failure=True)

    def test_negative_control_accepts_sccache_stderr_and_cargo_json_diagnostics(self):
        for diagnostic_channel in ("stderr", "json"):
            with self.subTest(diagnostic_channel=diagnostic_channel), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)

                def failed_compile(command, **kwargs):
                    diagnostic = "error: cache_probe_intentional_failure\n"
                    if diagnostic_channel == "stderr":
                        kwargs["stderr"].write(diagnostic)
                    else:
                        kwargs["stdout"].write(json.dumps({"reason": "compiler-message", "message": {
                            "rendered": diagnostic}}) + "\n")
                    kwargs["stdout"].write(json.dumps({"reason": "build-finished", "success": False}) + "\n")
                    return SimpleNamespace(returncode=101)

                with patch.object(cache.subprocess, "run", side_effect=failed_compile):
                    result, _ = cache.run_build(["cargo", "build"], root, root / "target", {},
                                                root / "result", expect_failure=True)
                self.assertEqual(result["exit_code"], 101)

    def test_negative_control_requires_cargo_failure_and_the_expected_diagnostic(self):
        cases = [(None, "cache_probe_intentional_failure"),
                 (True, "cache_probe_intentional_failure"), (False, "unrelated compilation failure")]
        for cargo_success, diagnostic in cases:
            with self.subTest(cargo_success=cargo_success, diagnostic=diagnostic), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)

                def failed_compile(command, **kwargs):
                    kwargs["stderr"].write(diagnostic)
                    if cargo_success is not None:
                        kwargs["stdout"].write(json.dumps({"reason": "build-finished", "success": cargo_success}) + "\n")
                    return SimpleNamespace(returncode=101)

                with patch.object(cache.subprocess, "run", side_effect=failed_compile):
                    with self.assertRaisesRegex(RuntimeError, "Negative control"):
                        cache.run_build(["cargo", "build"], root, root / "target", {},
                                        root / "result", expect_failure=True)

    def test_fixture_rejects_a_runtime_value_stale_after_rust_source_change(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            reports = root / "reports"
            reports.mkdir()
            options = SimpleNamespace(cargo="cargo", sccache="sccache", output=reports,
                                      cargo_config=[root / "workspace-only-config.toml"])

            def fake_run(command, source, target, env, directory, expect_failure=False):
                self.assertNotIn("--config", command)  # Independent fixture has its own dev profile.
                directory.mkdir(parents=True)
                executable = target / "probe"
                executable.write_text("fake compiler output")
                return ({"seconds": 1, "exit_code": 0}, [{
                    "reason": "compiler-artifact", "executable": str(executable),
                    "target": {"name": "ferroui-cache-probe"}}])

            def fake_output(command, source, env=None):
                if "--show-stats" in command:
                    return json.dumps(stats())
                if Path(command[0]).name == "probe":
                    return "10:20:30:A"  # Simulate an invalid cache returning old code.
                return ""

            with patch.object(cache, "run_build", side_effect=fake_run), \
                    patch.object(cache, "output", side_effect=fake_output):
                with self.assertRaisesRegex(RuntimeError, "Stale compiler output after rust-source"):
                    cache.validate_invalidation(options, root, {})


if __name__ == "__main__":
    unittest.main()
