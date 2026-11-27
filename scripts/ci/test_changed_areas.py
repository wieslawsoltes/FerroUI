"""Tests of changed_areas.py: which jobs of the CI workflow a pull request runs (docs/ci.md)."""

import os
import shutil
import subprocess
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import changed_areas  # noqa: E402

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def package(name, directory, dependencies=(), features=None):
    return {
        "name": name,
        "manifest_path": f"/repo/{directory}/Cargo.toml",
        "targets": [{"src_path": f"/repo/{directory}/lib.rs"}],
        "dependencies": [dict({"name": d[0], "path": "/repo/x", "kind": None, "target": None, "optional": False,
                               "features": [], "uses_default_features": True, "rename": None}, **d[1])
                         for d in dependencies],
        "features": features or {},
    }


class CfgTests(unittest.TestCase):
    def test_platforms(self):
        evaluate = changed_areas.evaluate_cfg
        self.assertTrue(evaluate("cfg(windows)", changed_areas.WINDOWS))
        self.assertFalse(evaluate("cfg(windows)", changed_areas.MACOS))
        self.assertTrue(evaluate('cfg(target_os = "macos")', changed_areas.MACOS))
        self.assertFalse(evaluate('cfg(target_os = "macos")', changed_areas.LINUX))
        self.assertTrue(evaluate("cfg(unix)", changed_areas.EMSCRIPTEN))
        self.assertTrue(evaluate('cfg(target_family = "wasm")', changed_areas.EMSCRIPTEN))
        self.assertFalse(evaluate('cfg(all(unix, not(target_os = "emscripten")))', changed_areas.EMSCRIPTEN))
        self.assertTrue(evaluate('cfg(any(target_os = "macos", windows, target_os = "linux"))', changed_areas.LINUX))
        self.assertTrue(evaluate("x86_64-pc-windows-msvc", changed_areas.WINDOWS))

    def test_an_unknown_cfg_keeps_the_dependency(self):
        self.assertIsNone(changed_areas.evaluate_cfg("cfg(not(ferroui_docsrs))", changed_areas.LINUX))
        self.assertIsNone(changed_areas.evaluate_cfg("cfg(all(target_vendor = \"apple\", custom))", changed_areas.MACOS))


class CompiledTests(unittest.TestCase):
    def workspace(self):
        return changed_areas.Workspace({"workspace_root": "/repo", "packages": [
            package("app", "samples/App", [
                ("base", {}),
                ("vello", {"optional": True}),
                ("win", {"target": "cfg(windows)"}),
                ("testkit", {"kind": "dev"}),
            ], features={"default": [], "vello": ["dep:vello"]}),
            package("host", "src/Host", [("app", {"features": ["vello"]})]),
            package("base", "src/Base"),
            package("vello", "src/Vello", [("base", {})]),
            package("win", "src/Win", [("base", {})]),
            package("testkit", "src/TestKit"),
        ]})

    def test_optional_platform_and_dev_dependencies(self):
        workspace = self.workspace()
        compiled, tested = workspace.compiled({"platform": changed_areas.MACOS, "roots": [changed_areas.root("app")]})
        self.assertEqual(compiled, {"app", "base", "testkit"})
        self.assertEqual(tested, {"app"})
        compiled, _ = workspace.compiled({"platform": changed_areas.WINDOWS,
                                          "roots": [changed_areas.root("app", dev=False)]})
        self.assertEqual(compiled, {"app", "base", "win"})

    def test_features_requested_by_a_dependent_turn_on_optional_dependencies(self):
        compiled, _ = self.workspace().compiled({"platform": changed_areas.LINUX,
                                                 "roots": [changed_areas.root("host", dev=False)]})
        self.assertEqual(compiled, {"host", "app", "base", "vello"})

    def test_a_dev_dependency_of_a_dependency_is_not_compiled(self):
        compiled, _ = self.workspace().compiled({"platform": changed_areas.LINUX,
                                                 "roots": [changed_areas.root("host")]})
        self.assertNotIn("testkit", compiled)


class DecideTests(unittest.TestCase):
    def setUp(self):
        self.workspace = changed_areas.Workspace({"workspace_root": "/repo", "packages": [
            package(name, directory) for name, directory in (
                ("ferroui-base", "src/FerroUI.Base"), ("ferroui-native", "src/FerroUI.Native"))]})
        self.saved = changed_areas.TARGETS
        changed_areas.TARGETS = {job: {"platform": changed_areas.LINUX, "roots": []} for job in changed_areas.TARGETS}
        changed_areas.TARGETS["macos"] = {"platform": changed_areas.MACOS,
                                          "roots": [changed_areas.root("ferroui-native")]}

    def tearDown(self):
        changed_areas.TARGETS = self.saved

    def decide(self, *files, event="pull_request"):
        reasons = changed_areas.decide(list(files), event, self.workspace)
        return {job for job, reason in reasons.items() if reason}

    def test_documentation_runs_nothing(self):
        self.assertEqual(self.decide("docs/ci.md", "README.md", "samples/README.md"), set())

    def test_markdown_in_a_package_is_a_file_of_the_package(self):
        # A crate can include it (`#![doc = include_str!("../README.md")]`).
        self.assertEqual(self.decide("src/FerroUI.Native/README.md"), {"api-gate", "macos"})

    def test_a_package_runs_the_jobs_that_build_it(self):
        self.assertEqual(self.decide("src/FerroUI.Native/lib.rs"), {"api-gate", "macos"})
        self.assertEqual(self.decide("src/FerroUI.Base/lib.rs"), {"api-gate"})

    def test_scripts_of_a_job(self):
        self.assertEqual(self.decide("scripts/x11/with-window-manager.sh"), {"x11"})
        self.assertEqual(self.decide("scripts/browser/tests/x.test.mjs"), {"browser"})
        self.assertEqual(self.decide("scripts/perf/gate.py"), set())

    def test_what_every_job_builds_with_runs_everything(self):
        for path in ("Cargo.lock", "Cargo.toml", ".github/workflows/ci.yml", ".cargo/config.toml",
                     "scripts/ci/changed_areas.py", "external/XamlX/x.rs"):
            self.assertEqual(self.decide(path), set(changed_areas.JOBS), path)

    def test_another_workflow_runs_nothing_here(self):
        self.assertEqual(self.decide(".github/workflows/vello.yml"), set())

    def test_an_unknown_file_runs_everything(self):
        self.assertEqual(self.decide("src/Unknown/file.rs"), set(changed_areas.JOBS))
        self.assertEqual(self.decide("rustfmt.toml"), set(changed_areas.JOBS))

    def test_other_events_run_everything(self):
        for event in ("push", "schedule", "workflow_dispatch"):
            self.assertEqual(self.decide(event=event), set(changed_areas.JOBS))

    def test_an_incomplete_list_runs_everything(self):
        reasons = changed_areas.decide(["docs/ci.md"], "pull_request", self.workspace, complete=False)
        self.assertTrue(all(reasons.values()))

    def test_without_the_manifests_everything_runs(self):
        reasons = changed_areas.decide(["docs/ci.md"], "pull_request", None)
        self.assertTrue(all(reasons.values()))


@unittest.skipUnless(shutil.which("cargo"), "cargo is not installed")
class WorkspaceTests(unittest.TestCase):
    """The rules against the manifests of this repository."""

    @classmethod
    def setUpClass(cls):
        cls.workspace = changed_areas.load_workspace(None, ROOT)

    def decide(self, *files):
        reasons = changed_areas.decide(list(files), "pull_request", self.workspace)
        return {job for job, reason in reasons.items() if reason}

    def test_every_root_is_a_package(self):
        for job, description in changed_areas.TARGETS.items():
            for entry in description["roots"]:
                self.assertIn(entry["package"], self.workspace.packages, job)
            for name in description.get("exclude", ()):
                self.assertIn(name, self.workspace.packages, job)

    def test_every_job_builds_the_base(self):
        self.assertEqual(self.decide("src/FerroUI.Base/lib.rs"), set(changed_areas.JOBS))

    def test_vello_does_not_run_the_native_macos_job(self):
        jobs = self.decide("src/Vello/FerroUI.Vello/lib.rs")
        self.assertNotIn("macos", jobs)
        self.assertNotIn("browser", jobs)
        self.assertIn("linux", jobs)

    def test_a_platform_backend_runs_its_own_job(self):
        self.assertIn("x11", self.decide("src/FerroUI.X11/lib.rs"))
        self.assertNotIn("macos", self.decide("src/FerroUI.X11/lib.rs"))
        self.assertIn("macos", self.decide("src/FerroUI.Native/lib.rs"))
        self.assertNotIn("windows", self.decide("src/FerroUI.Native/lib.rs"))

    def test_a_file_named_by_another_package(self):
        # samples/ControlCatalog.Desktop/program.rs has #[path = "../../scripts/perf/startup_probe.rs"].
        self.assertIn("macos", self.decide("scripts/perf/startup_probe.rs"))
        # The native sources of ferroui-native, outside its directory.
        self.assertIn("macos", self.decide("native/FerroUI.Native/FerroUI.Native.mm"))

    def test_the_command_line(self):
        output = subprocess.run([sys.executable, os.path.join(ROOT, "scripts/ci/changed_areas.py"), "--files", "-"],
                                input="docs/ci.md\n", capture_output=True, text=True, cwd=ROOT, check=True).stdout
        self.assertEqual(output.split(), [f"{job}=false" for job in changed_areas.JOBS])


if __name__ == "__main__":
    unittest.main()
