"""Small filesystem/process tests for the CI gate; no browser, Cargo or network."""

import copy
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

import browser_ci as ci


REPO = Path(__file__).resolve().parents[3]


class PartitionTests(unittest.TestCase):
    def setUp(self):
        self.config = ci.read_json(REPO / ci.CONFIG)
        self.source = (REPO / ci.SAMPLE / "tests/browser.mjs").read_text()

    def test_exact_registry_and_eight_contiguous_mode_groups(self):
        names = ci.validate_config(self.config, self.source)
        self.assertEqual(len(names), 38)
        self.assertEqual(len(ci.matrix(self.config)["include"]), 8)
        self.assertEqual([len(group["scenarios"]) for group in self.config["groups"]], [8, 8, 10, 12])
        self.assertEqual(names[0], "vector-network")
        self.assertEqual(names[-1], "bounded-history")

    def test_missing_duplicate_unknown_or_reordered_names_fail_closed(self):
        for replacement in ([], ["unknown"] * 8, self.config["groups"][1]["scenarios"],
                            list(reversed(self.config["groups"][0]["scenarios"]))):
            with self.subTest(replacement=replacement):
                config = copy.deepcopy(self.config)
                config["groups"][0]["scenarios"] = replacement
                with self.assertRaises(ci.GateError):
                    ci.validate_config(config, self.source)

    def test_dynamic_unparsed_or_duplicate_registration_cannot_disappear(self):
        for source in (self.source + '\nawait scenario(name, callback);',
                       self.source.replace('await scenario("corners",', 'await scenario("native-authoring",'),
                       self.source.replace('await scenario("corners",', 'await scenario(dynamicName,')):
            with self.subTest(source=source[-60:]):
                with self.assertRaises(ci.GateError):
                    ci.validate_config(self.config, source)

    def test_new_literal_registration_requires_explicit_partition_review(self):
        with self.assertRaises(ci.GateError):
            ci.validate_config(self.config, self.source + '\nawait scenario("new-feature", callback);')

    def test_modes_and_group_identifiers_are_not_implicitly_normalized(self):
        for modes in (["Software2D"], ["WebGL2", "Software2D"], ["Software2D", "Software2D"]):
            config = copy.deepcopy(self.config)
            config["modes"] = modes
            with self.assertRaises(ci.GateError):
                ci.validate_config(config, self.source)
        for name in ("../outside", "geometry-paint", ""):
            config = copy.deepcopy(self.config)
            config["groups"][1]["name"] = name
            with self.assertRaises(ci.GateError):
                ci.validate_config(config, self.source)


class EvidenceFixture(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="ferrodesign-browser-ci-test-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.dist = self.root / "dist"
        self.dist.mkdir()
        self.config, self.names = ci.load_plan(REPO)
        self.source = {"commit": "a" * 40, "tree": "b" * 40, "repository": "example/FerroUI", "run_id": "12345",
                       "harness": {"sha256": "c" * 64, "files": [{"path": "tests/browser.mjs", "sha256": "d" * 64, "bytes": 17}]}}
        for name, data in {"index.html": b"<!doctype html>", "main.js": b"// host", "ferrodesign-app.js": b"// wasm loader",
                           "favicon.svg": b"<svg/>", "application.wasm": b"\0asm", "dependencies.txt": b"ferrodesign-app\n  ferroui-vello v0.4.0\n"}.items():
            (self.dist / name).write_bytes(data)
        self.manifest = {"schema_version": 1, "kind": "ferrodesign-browser-ci", "source": self.source,
                         "run_attempt": "1", "profile": "browser", "target": "wasm32-unknown-emscripten",
                         "files": ci.distribution_files(self.dist)}
        self.reseal()
        self.artifacts = self.root / "artifacts"
        self.artifacts.mkdir()
        self.needs = {name: {"result": "success", "outputs": {}} for name in ci.DEPENDENCIES}
        self.needs["browser-build"]["outputs"]["manifest_sha256"] = self.checksum

    def reseal(self):
        ci.write_json(self.dist / ci.MANIFEST, self.manifest)
        self.checksum = ci.file_record(self.dist / ci.MANIFEST)["sha256"]
        self.identity = {**self.source, "distribution_manifest_sha256": self.checksum, "build_run_attempt": "1"}

    def verify_dist(self, attempt="1"):
        return ci.validate_distribution(self.dist, self.manifest, self.source, self.checksum, attempt)

    def scenario_report(self, group, mode):
        selected = next(item["scenarios"] for item in self.config["groups"] if item["name"] == group)
        entries, checks = [], ["Common checks before scenarios"]
        for name in self.names:
            entry = {"name": name, "status": "not-requested", "checksBefore": len(checks)}
            if name in selected:
                entry.update(status="passed", ran=True, checks=1, durationMs=10)
                checks.append("Completed " + name)
            entries.append(entry)
        checks.append("Common checks after scenarios")
        return {"passed": True, "scope": {"full": False, "modes": [mode], "scenarios": selected}, "results": [{
            "requestedMode": mode, "requestedRenderer": "Vello", "surface": {"kind": mode}, "status": "passed",
            "initial": {"renderer": "FerroUI Vello"}, "final": {"renderer": "FerroUI Vello", "unsupportedEffects": 0},
            "pageErrors": [], "consoleErrors": [], "checks": checks, "scenarios": entries,
        }]}

    def persistence_report(self):
        return {"passed": True, "browser": "test-browser", "checks": ci.PERSISTENCE_CHECKS,
                "largeDocument": {"flushed": True, "characters": 11 * 1024 * 1024, "state": {"dirty": False}}}

    def benchmark_report(self, directory):
        directory.mkdir(parents=True, exist_ok=True)
        fixture = b'{"fixture":"synthetic test evidence"}'
        (directory / "fixture.json").write_bytes(fixture)
        files = [{"name": row["path"], "bytes": row["bytes"], "sha256": row["sha256"]}
                 for row in self.manifest["files"] if Path(row["path"]).suffix in (".js", ".html", ".wasm")]
        report = {"schemaVersion": 2, "benchmark": "native-text-pan-v2", "warmupCount": 12, "measuredCount": 36,
                  "traceDiagnosticRequested": True, "browserTransport": "owned-playwright-server-websocket", "finished": "2026-10-23T15:02:00+00:00",
                  "builds": [{"id": "current", "label": self.source["commit"], "files": files,
                              "fingerprint": ci.digest(json.dumps(files, separators=(",", ":")).encode())}],
                  "workload": {"inputSha256": ci.digest(fixture), "labels": 150}, "results": [],
                  "browserShutdown": {"status": "closed", "processExit": {"observed": True, "code": 0}, "errors": []}}
        for mode in ci.MODES:
            trace_name = f"current-{mode}-pan-trace.json"
            (directory / trace_name).write_text('{"traceEvents": []}')
            raw = ci.file_record(directory / trace_name)
            report["results"].append({
                "requestedMode": mode, "status": "passed", "build": "current", "label": self.source["commit"],
                "environment": {"surface": mode}, "metricBridge": {"kind": "compact-v1"},
                "pageErrors": [], "consoleErrors": [], "renderPanics": [],
                "samples": [{"index": index, "durationMs": 3, "dispatchMs": 1, "frameObservationMs": 2,
                             "presentationMs": 4, "settledObservationMs": 8, "frameDelta": 1, "presentedFrameDelta": 1}
                            for index in range(12, 48)],
                "traceDiagnostic": {
                    "status": "completed", "normalSamplesIncluded": False, "fatalBrowserClosure": False,
                    "build": "current", "mode": mode, "cleanupErrors": [], "pageErrors": [], "consoleErrors": [], "droppedErrors": 0,
                    "limits": {"samples": 6}, "motions": [[1, 0]] * 6,
                    "observation": {"droppedInputEvents": 0, "samples": [{"index": index, "complete": True,
                        "input": {"capture": {"trusted": True}, "bubble": {"trusted": True}}} for index in range(6)]},
                    "analysis": {"status": "complete", "samples": [{"index": index} for index in range(6)]},
                    "trace": {"file": trace_name, "bytes": raw["bytes"], "sha256": raw["sha256"], "streamTruncated": False, "dataLossOccurred": False},
                },
            })
        return report

    def shard_dir(self, group="geometry-paint", mode="Software2D"):
        return self.artifacts / f"FerroDesign-browser-evidence-{group}-{mode}-{self.source['commit']}"

    def data_dir(self):
        return self.artifacts / f"FerroDesign-browser-evidence-data-{self.source['commit']}"

    def write_proof(self, directory, kind, report, group=None, mode=None):
        directory.mkdir(parents=True, exist_ok=True)
        ci.write_json(directory / "report.json", report)
        (directory / "stdout.log").write_text("original command stdout\n")
        (directory / "stderr.log").write_text("")
        if kind == "scenario":
            names = next(item["scenarios"] for item in self.config["groups"] if item["name"] == group)
            coverage = {"pairs": [[name, mode] for name in names], "checks": len(names) + 2, "scenario_checks": len(names)}
        elif kind == "persistence":
            coverage = {"checks": 5}
        else:
            coverage = {"modes": ci.MODES, "normal_samples_per_mode": 36, "trace_samples_per_mode": 6}
        proof = {"schema_version": 1, "kind": kind, "group": group, "mode": mode,
                 "identity": self.identity, "identity_after": self.identity, "ci": {"job": ci.JOBS[kind], "run_attempt": "1"},
                 "command": ci.COMMANDS[kind], "status": "passed", "validation_errors": [], "coverage": coverage,
                 "process": {"started_utc": "2026-10-23T15:00:00+00:00", "finished_utc": "2026-10-23T15:01:00+00:00", "exit_code": 0},
                 "files": {name: ci.file_record(directory / name) for name in ("report.json", "stdout.log", "stderr.log")}}
        ci.write_json(directory / "proof.json", proof)

    def populate(self):
        for item in ci.matrix(self.config)["include"]:
            self.write_proof(self.shard_dir(**item), "scenario", self.scenario_report(**item), **item)
        self.write_proof(self.data_dir() / "persistence", "persistence", self.persistence_report())
        benchmark_dir = self.data_dir() / "benchmark"
        self.write_proof(benchmark_dir, "benchmark", self.benchmark_report(benchmark_dir))

    def gate(self, attempt="1"):
        return ci.aggregate(self.artifacts, self.config, self.manifest, self.identity, self.needs, attempt)

    def change_json(self, path, change):
        value = ci.read_json(path)
        change(value)
        ci.write_json(path, value)


class DistributionTests(EvidenceFixture):
    def test_exact_payload_manifest_and_source_are_accepted(self):
        self.assertEqual(self.verify_dist(), self.identity)

    def test_changed_payload_is_rejected(self):
        (self.dist / "application.wasm").write_bytes(b"changed")
        with self.assertRaisesRegex(ci.GateError, "digest differ"):
            self.verify_dist()

    def test_missing_and_extra_payloads_are_rejected(self):
        (self.dist / "extra.js").write_text("extra")
        with self.assertRaises(ci.GateError):
            self.verify_dist()
        (self.dist / "extra.js").unlink()
        (self.dist / "favicon.svg").unlink()
        with self.assertRaises(ci.GateError):
            self.verify_dist()

    def test_forged_manifest_does_not_match_the_build_job_digest(self):
        original = self.checksum
        self.manifest["files"][0]["sha256"] = "e" * 64
        self.reseal()
        with self.assertRaisesRegex(ci.GateError, "build job output"):
            ci.validate_distribution(self.dist, self.manifest, self.source, original, "1")

    def test_duplicate_payload_path_and_traversal_are_rejected(self):
        for name in ("../outside", "/absolute", "./main.js", "nested//main.js", "nested\\main.js", ""):
            with self.subTest(name=name), self.assertRaises(ci.GateError):
                ci.relative_file(self.dist, name)
        self.manifest["files"].append(copy.deepcopy(self.manifest["files"][0]))
        self.reseal()
        with self.assertRaisesRegex(ci.GateError, "Duplicate"):
            self.verify_dist()

    def test_symlink_payload_or_parent_is_rejected(self):
        (self.root / "outside.js").write_text("outside")
        (self.dist / "alias.js").symlink_to(self.root / "outside.js")
        with self.assertRaises(ci.GateError):
            ci.distribution_files(self.dist)
        (self.dist / "alias.js").unlink()
        (self.dist / "linked").symlink_to(self.root, target_is_directory=True)
        with self.assertRaises(ci.GateError):
            ci.relative_file(self.dist, "linked/outside.js")

    def test_source_tree_harness_and_run_cannot_be_substituted(self):
        for key in ("commit", "tree", "repository", "run_id", "harness"):
            source = copy.deepcopy(self.source)
            source[key] = "different"
            with self.subTest(key=key), self.assertRaises(ci.GateError):
                ci.validate_distribution(self.dist, self.manifest, source, self.checksum, "1")

    def test_only_nonfuture_positive_build_attempts_are_accepted(self):
        self.assertEqual(self.verify_dist("2"), self.identity)
        for attempt in ("0", "-1", "future", None, 1, "01"):
            self.manifest["run_attempt"] = attempt
            self.reseal()
            with self.subTest(attempt=attempt), self.assertRaises(ci.GateError):
                self.verify_dist("2")
        self.manifest["run_attempt"] = "3"
        self.reseal()
        with self.assertRaisesRegex(ci.GateError, "future"):
            self.verify_dist("2")

    def test_active_graph_must_retain_vello_without_skia(self):
        for graph in ("ferrodesign-app", "ferroui-vello\n skia-bindings"):
            (self.dist / "dependencies.txt").write_text(graph)
            self.manifest["files"] = ci.distribution_files(self.dist)
            self.reseal()
            with self.assertRaisesRegex(ci.GateError, "Vello/no-Skia"):
                self.verify_dist()

    def test_duplicate_json_keys_and_nonfinite_numbers_are_rejected(self):
        path = self.root / "forged.json"
        for text in ('{"passed":false,"passed":true}', '{"durationMs": NaN}', '{"durationMs": Infinity}'):
            path.write_text(text)
            with self.subTest(text=text), self.assertRaises(ci.GateError):
                ci.read_json(path)


class CoverageTests(EvidenceFixture):
    def setUp(self):
        super().setUp()
        self.populate()

    def test_exact_eight_shards_and_both_data_gates_cover_all_76_pairs(self):
        summary = self.gate()
        self.assertIs(summary["passed"], True)
        self.assertEqual(summary["scenario_mode_pairs"], 76)
        self.assertEqual(len(summary["shards"]), 8)
        self.assertEqual(summary["benchmark"]["normal_samples_per_mode"], 36)
        self.assertEqual(summary["benchmark"]["trace_samples_per_mode"], 6)

    def test_missing_extra_or_duplicate_artifact_is_rejected(self):
        path = self.shard_dir()
        path.rename(path.with_name(path.name + "-duplicate"))
        with self.assertRaises(ci.GateError):
            self.gate()
        path.with_name(path.name + "-duplicate").rename(path)
        (self.artifacts / "unexpected").mkdir()
        with self.assertRaises(ci.GateError):
            self.gate()

    def test_missing_final_report_from_a_timeout_is_rejected(self):
        (self.shard_dir() / "report.json").unlink()
        with self.assertRaises(ci.GateError):
            self.gate()

    def test_missing_or_failed_process_cannot_be_overruled_by_passed_report(self):
        path = self.shard_dir() / "proof.json"
        original = ci.read_json(path)
        for code in (None, 1, -15, True):
            proof = copy.deepcopy(original)
            proof["process"]["exit_code"] = code
            ci.write_json(path, proof)
            with self.subTest(code=code), self.assertRaisesRegex(ci.GateError, "process did not"):
                self.gate()

    def test_failed_canceled_skipped_or_missing_dependency_job_is_rejected(self):
        for job in ci.DEPENDENCIES:
            for status in ("failure", "cancelled", "skipped", None):
                self.needs[job]["result"] = status
                with self.subTest(job=job, status=status), self.assertRaises(ci.GateError):
                    self.gate()
            self.needs[job]["result"] = "success"
        del self.needs["browser-data"]
        with self.assertRaises(ci.GateError):
            self.gate()

    def test_distribution_or_harness_mismatch_is_rejected_even_with_passed_status(self):
        path = self.shard_dir() / "proof.json"
        original = ci.read_json(path)
        for field in ("commit", "tree", "repository", "run_id", "harness", "distribution_manifest_sha256"):
            proof = copy.deepcopy(original)
            proof["identity"][field] = "forged"
            ci.write_json(path, proof)
            with self.subTest(field=field), self.assertRaises(ci.GateError):
                self.gate()

    def test_successful_earlier_sibling_is_retained_only_for_identical_build_identity(self):
        self.change_json(self.shard_dir() / "proof.json", lambda proof: proof["ci"].update(run_attempt="2"))
        self.assertEqual(self.gate("2")["scenario_mode_pairs"], 76)
        self.change_json(self.shard_dir() / "proof.json", lambda proof: proof["process"].update(exit_code=1))
        with self.assertRaises(ci.GateError):
            self.gate("2")

    def test_future_invalid_or_prebuild_execution_attempt_is_rejected(self):
        path = self.shard_dir() / "proof.json"
        for value in ("3", "0", None, "01"):
            self.change_json(path, lambda proof: proof["ci"].update(run_attempt=value))
            with self.subTest(value=value), self.assertRaises(ci.GateError):
                self.gate("2")
        proof = ci.read_json(path)
        proof["ci"]["run_attempt"] = "1"
        ci.write_json(path, proof)
        identity = copy.deepcopy(self.identity)
        identity["build_run_attempt"] = "2"
        proof["identity"] = proof["identity_after"] = identity
        ci.write_json(path, proof)
        with self.assertRaisesRegex(ci.GateError, "invalid execution attempt"):
            ci.validate_proof(self.shard_dir(), "scenario", self.config, self.manifest, identity, "2", "geometry-paint", "Software2D")

    def test_report_digest_is_checked_independently_of_a_passed_summary(self):
        self.change_json(self.shard_dir() / "report.json", lambda report: report.update(passed=False))
        with self.assertRaisesRegex(ci.GateError, "digest differ"):
            self.gate()

    def test_rehashed_forged_report_still_must_have_every_actual_selected_scenario(self):
        original = self.scenario_report("geometry-paint", "Software2D")
        for value in ("not-requested", "running", "failed"):
            report = copy.deepcopy(original)
            report["results"][0]["scenarios"][0]["status"] = value
            self.write_proof(self.shard_dir(), "scenario", report, "geometry-paint", "Software2D")
            with self.subTest(status=value), self.assertRaises(ci.GateError):
                self.gate()
        report = copy.deepcopy(original)
        report["results"][0]["scenarios"][0]["ran"] = False
        self.write_proof(self.shard_dir(), "scenario", report, "geometry-paint", "Software2D")
        with self.assertRaises(ci.GateError):
            self.gate()

    def test_wrong_scope_mode_surface_registry_and_extra_execution_are_rejected(self):
        changes = [
            lambda report: report["scope"].update(full=True),
            lambda report: report["scope"].update(modes=ci.MODES),
            lambda report: report["results"][0]["surface"].update(kind="WebGL2"),
            lambda report: report["results"][0]["scenarios"].pop(),
            lambda report: report["results"][0]["scenarios"].append(copy.deepcopy(report["results"][0]["scenarios"][0])),
            lambda report: report["results"][0]["scenarios"][9].update(ran=True),
            lambda report: report["results"][0].update(checks=[]),
            lambda report: report["results"][0].update(pageErrors=["uncaught"]),
        ]
        for index, change in enumerate(changes):
            report = self.scenario_report("geometry-paint", "Software2D")
            change(report)
            self.write_proof(self.shard_dir(), "scenario", report, "geometry-paint", "Software2D")
            with self.subTest(change=index), self.assertRaises(ci.GateError):
                self.gate()

    def test_wrong_original_command_job_and_forged_coverage_summary_are_rejected(self):
        path = self.shard_dir() / "proof.json"
        original = ci.read_json(path)
        for key, value in (("command", ["node", "-e", "process.exit(0)"]), ("coverage", {"pairs": [], "checks": 10}),
                           ("ci", {"job": "unassigned", "run_attempt": "1"}), ("identity_after", {})):
            proof = copy.deepcopy(original)
            proof[key] = value
            ci.write_json(path, proof)
            with self.subTest(key=key), self.assertRaises(ci.GateError):
                self.gate()

    def test_missing_or_incomplete_persistence_check_is_rejected(self):
        report = self.persistence_report()
        report["checks"] = report["checks"][:-1]
        self.write_proof(self.data_dir() / "persistence", "persistence", report)
        with self.assertRaises(ci.GateError):
            self.gate()

    def test_benchmark_cannot_substitute_incomplete_untraced_or_failed_samples(self):
        directory = self.data_dir() / "benchmark"
        changes = [
            lambda report: report.update(traceDiagnosticRequested=False),
            lambda report: report["results"].pop(),
            lambda report: report["results"][0]["samples"].pop(),
            lambda report: report["results"][0]["samples"][0].update(index=13),
            lambda report: report["results"][0]["samples"][0].update(presentedFrameDelta=0),
            lambda report: report["results"][0]["traceDiagnostic"]["observation"]["samples"].pop(),
            lambda report: report["results"][0]["traceDiagnostic"].update(status="stopped"),
            lambda report: report["results"][0]["traceDiagnostic"].update(normalSamplesIncluded=True),
            lambda report: report["results"][0]["traceDiagnostic"]["trace"].update(streamTruncated=True),
            lambda report: report["results"][0]["traceDiagnostic"]["observation"]["samples"][0].update(complete=False),
            lambda report: report["results"][0]["traceDiagnostic"]["observation"]["samples"][0]["input"]["capture"].update(trusted=False),
            lambda report: report["results"][0].update(consoleErrors=["missing asset"]),
            lambda report: report["browserShutdown"]["processExit"].update(observed=False),
            lambda report: report["builds"][0]["files"][0].update(sha256="f" * 64),
        ]
        for index, change in enumerate(changes):
            report = self.benchmark_report(directory)
            change(report)
            self.write_proof(directory, "benchmark", report)
            with self.subTest(change=index), self.assertRaises(ci.GateError):
                self.gate()

    def test_trace_file_bytes_are_verified_not_just_the_summary(self):
        (self.data_dir() / "benchmark/current-WebGL2-pan-trace.json").write_text("changed")
        with self.assertRaisesRegex(ci.GateError, "digest differ"):
            self.gate()


class ProcessTests(EvidenceFixture):
    def test_real_child_exit_and_both_output_streams_are_preserved(self):
        output = self.root / "process"
        output.mkdir()
        command = [sys.executable, "-c", "import sys; print('stdout proof'); print('stderr proof', file=sys.stderr); sys.exit(7)"]
        with tempfile.TemporaryFile("w+b") as sink:
            with mock.patch.object(sys, "stdout", mock.Mock(buffer=sink)), mock.patch.object(sys, "stderr", mock.Mock(buffer=sink)):
                code = ci.run_child(command, self.root, os.environ.copy(), output)
        self.assertEqual(code, 7)
        self.assertEqual((output / "stdout.log").read_text(), "stdout proof\n")
        self.assertEqual((output / "stderr.log").read_text(), "stderr proof\n")

    def test_wrapper_retains_nonzero_exit_even_if_child_forged_a_passing_report(self):
        output = self.root / "failed-process"
        environment = {"GITHUB_JOB": "browser-scenarios", "GITHUB_RUN_ATTEMPT": "1"}

        def failed_child(command, cwd, env, directory):
            ci.write_json(directory / "report.json", self.scenario_report("geometry-paint", "Software2D"))
            (directory / "stdout.log").write_text("claimed pass\n")
            (directory / "stderr.log").write_text("late cleanup failure\n")
            return 7

        with mock.patch.object(ci, "source_identity", return_value=self.source), mock.patch.object(ci, "run_child", side_effect=failed_child):
            code = ci.run_validation(REPO, self.dist, output, self.checksum, "scenario", "geometry-paint", "Software2D", environment)
        proof = ci.read_json(output / "proof.json")
        self.assertEqual(code, 1)
        self.assertEqual(proof["process"]["exit_code"], 7)
        self.assertEqual(proof["status"], "failed")
        self.assertIn("report.json", proof["files"])

    def test_wrapper_controls_scope_and_does_not_inherit_baseline_or_renderer_override(self):
        output = self.root / "controlled-process"
        environment = {"GITHUB_JOB": "browser-scenarios", "GITHUB_RUN_ATTEMPT": "1", "FERRODESIGN_SCENARIOS": "corners",
                       "FERRODESIGN_MODES": "WebGL2", "FERRODESIGN_BASELINE_DIST": "/old", "FERRODESIGN_CHROMIUM": "/wrong"}

        def child(command, cwd, env, directory):
            self.assertEqual(command, ["node", "tests/browser.mjs"])
            self.assertEqual(env["FERRODESIGN_MODES"], "Software2D")
            self.assertEqual(env["FERRODESIGN_SCENARIOS"].split(","), self.config["groups"][0]["scenarios"])
            self.assertNotIn("FERRODESIGN_BASELINE_DIST", env)
            self.assertNotIn("FERRODESIGN_CHROMIUM", env)
            ci.write_json(directory / "report.json", self.scenario_report("geometry-paint", "Software2D"))
            (directory / "stdout.log").write_text("completed\n")
            (directory / "stderr.log").write_text("")
            return 0

        with mock.patch.object(ci, "source_identity", return_value=self.source), mock.patch.object(ci, "run_child", side_effect=child):
            self.assertEqual(ci.run_validation(REPO, self.dist, output, self.checksum, "scenario", "geometry-paint", "Software2D", environment), 0)
        self.assertEqual(ci.read_json(output / "proof.json")["status"], "passed")


class WorkflowContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.workflow = (REPO / ".github/workflows/ferrodesign.yml").read_text()

    def job(self, name):
        match = re.search(r"^  " + re.escape(name) + r":\n(.*?)(?=^  [a-z][a-z-]*:|\Z)", self.workflow, re.M | re.S)
        self.assertIsNotNone(match, name)
        return match.group(1)

    def test_every_action_is_an_existing_reviewed_sha_pin(self):
        pins = {
            "actions/checkout": "11d5960a326750d5838078e36cf38b85af677262",
            "actions/setup-node": "49933ea5288caeca8642d1e84afbd3f7d6820020",
            "actions/cache/restore": "0057852bfaa89a56745cba8c7296529d2fc39830",
            "actions/cache/save": "0057852bfaa89a56745cba8c7296529d2fc39830",
            "actions/upload-artifact": "ea165f8d65b6e75b540449e92b4886f43607fa02",
            "actions/download-artifact": "d3f86a106a0bac45b974a628896c90dbdf5c8093",
        }
        uses = re.findall(r"uses: ([^\s]+)", self.workflow)
        self.assertTrue(uses)
        for use in uses:
            action, revision = use.split("@")
            self.assertEqual(revision, pins[action])

    def test_one_build_fail_fast_false_bounded_matrix_and_independent_data_gates(self):
        build, shards, data = self.job("browser-build"), self.job("browser-scenarios"), self.job("browser-data")
        self.assertEqual(self.workflow.count("scripts/build-browser.sh 2>&1"), 1)
        self.assertIn("steps.partition.outputs.matrix", build)
        self.assertIn("browser_ci.py seal", build)
        self.assertLess(build.index("Upload complete browser build"), build.index("Snapshot source timestamps"))
        for job in (shards, data):
            self.assertIn("needs: browser-build", job)
            self.assertIn("timeout-minutes: 60", job)
            self.assertIn("browser_ci.py verify", job)
            self.assertNotIn("cargo ", job)
        self.assertIn("fail-fast: false", shards)
        self.assertIn("fromJSON(needs.browser-build.outputs.matrix)", shards)
        self.assertIn("--kind persistence", data)
        self.assertIn("--kind benchmark", data)
        self.assertNotIn("continue-on-error", self.workflow)

    def test_aggregate_always_runs_and_keeps_eight_artifacts_distinct(self):
        aggregate = self.job("browser-coverage")
        self.assertIn("if: always()", aggregate)
        self.assertIn("needs: [browser-build, browser-scenarios, browser-data]", aggregate)
        self.assertIn("toJSON(needs)", aggregate)
        self.assertIn("merge-multiple: false", aggregate)
        self.assertIn("browser_ci.py aggregate", aggregate)
        self.assertIn("ferrodesign-v4-${{ github.ref }}", self.workflow)
        self.assertIn("cancel-in-progress: false", self.workflow)


if __name__ == "__main__":
    unittest.main(verbosity=2)
