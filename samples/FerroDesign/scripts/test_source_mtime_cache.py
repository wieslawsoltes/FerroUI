"""Filesystem-only tests: no Git process, Cargo, browser, network or cache service."""

from contextlib import redirect_stdout
import copy
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest import mock

import source_mtime_cache as cache


OLD = 1_700_000_000_123_456_789
CHECKOUT = OLD + 90_000_000_000
ACCESS = OLD - 123_456_789


@unittest.skipUnless(sys.platform.startswith("linux"), "native CI helper requires Linux no-follow/no-atime APIs")
class SourceMtimeCacheTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="ferrodesign-source-mtime-test-")
        self.addCleanup(self.temporary.cleanup)
        self.base = Path(self.temporary.name)
        self.repo = self.base / "repo"
        self.repo.mkdir()
        # Deliberately not named target: the explicit cache path must be excluded.
        self.target = self.repo / "cargo-cache"
        self.target.mkdir()
        self.manifest = self.target / cache.MANIFEST_NAME
        self.tracked = {}

    def source(self, path="src/framework.rs", content=b"source A", mtime=OLD, tracked=True):
        file = self.repo / path
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_bytes(content)
        os.utime(file, ns=(ACCESS, mtime))
        if tracked:
            self.tracked[path] = "100644"
        return file

    def run_helper(self, operation, *, repo=None, target=None):
        with mock.patch.object(cache, "_read_tracked_files", return_value=dict(self.tracked)):
            return cache.run(operation, str(repo or self.repo), str(target or self.target))

    def snapshot(self):
        result = self.run_helper("snapshot")
        self.assertEqual(result["status"], "snapshotted", result)
        return result, self.manifest.read_bytes()

    def restore(self, data):
        self.manifest.write_bytes(data)
        return self.run_helper("restore")

    def checkout(self, file, mtime=CHECKOUT):
        os.utime(file, ns=(ACCESS, mtime))

    def test_exact_bytes_restore_actual_nanoseconds_and_preserve_atime(self):
        source = self.source("src/FerroUI.Build.Scan/input.rs")
        result, data = self.snapshot()
        self.assertEqual(result["recorded"], 1)
        self.assertEqual(source.stat().st_atime_ns, ACCESS)
        entry = json.loads(data)["entries"][0]
        self.assertEqual(entry["mtime_ns"], OLD)
        self.assertEqual(entry["sha256"], hashlib.sha256(b"source A").hexdigest())
        self.checkout(source)
        result = self.run_helper("restore")
        self.assertEqual(result["status"], "restored", result)
        self.assertEqual(result["restored"], 1)
        self.assertEqual(source.stat().st_mtime_ns, OLD)
        self.assertEqual(source.stat().st_atime_ns, ACCESS)
        self.assertEqual(source.read_bytes(), b"source A")
        self.assertFalse(self.manifest.exists(), "restore must consume the manifest before any later build")

    def test_changed_same_size_changed_size_new_and_untracked_files_stay_fresh(self):
        same_size = self.source("same.rs", b"abc")
        changed_size = self.source("size.rs", b"abc")
        untracked = self.source("untracked.rs", b"abc")
        removed = self.source("removed.rs", b"abc")
        _, data = self.snapshot()
        same_size.write_bytes(b"xyz")
        changed_size.write_bytes(b"abcd")
        new = self.source("new.rs", b"new", CHECKOUT)
        del self.tracked["untracked.rs"]
        removed.unlink()
        for file in (same_size, changed_size, untracked):
            self.checkout(file)
        result = self.restore(data)
        self.assertEqual(result["restored"], 0, result)
        self.assertEqual(result["skip_reasons"]["content_mismatch"], 2)
        for file in (same_size, changed_size, untracked, new):
            self.assertEqual(file.stat().st_mtime_ns, CHECKOUT)
            self.assertEqual(file.stat().st_atime_ns, ACCESS)

    def test_successful_and_failed_build_ordering_and_compiled_artifacts_are_unchanged(self):
        source = self.source("src/input.rs", mtime=OLD + 200)
        dependency_info = self.source("cargo-cache/debug/.fingerprint/dep-info", b"artifact", OLD + 100)
        native_object = self.source("prebuilt/file.rlib", b"compiled", OLD + 100)
        other_target = self.source("another/target/output.rs", b"generated output", OLD + 100)
        snapshot, data = self.snapshot()
        self.assertEqual(snapshot["recorded"], 1)
        self.checkout(source)
        self.restore(data)
        self.assertGreater(source.stat().st_mtime_ns, dependency_info.stat().st_mtime_ns)
        self.assertEqual(dependency_info.stat().st_mtime_ns, OLD + 100)
        self.assertEqual(native_object.stat().st_mtime_ns, OLD + 100)
        self.assertEqual(other_target.stat().st_mtime_ns, OLD + 100)
        # A later completed compile retains the opposite, fresh ordering.
        os.utime(dependency_info, ns=(ACCESS, OLD + 300))
        _, data = self.snapshot()
        self.checkout(source)
        self.restore(data)
        self.assertLess(source.stat().st_mtime_ns, dependency_info.stat().st_mtime_ns)
        self.assertEqual(dependency_info.read_bytes(), b"artifact")
        self.assertEqual(native_object.read_bytes(), b"compiled")

    def test_source_changed_after_build_keeps_its_actual_late_timestamp(self):
        source = self.source(mtime=OLD + 100)
        dependency_info = self.source("cargo-cache/debug/dep-info", b"old output", OLD + 200)
        source.write_bytes(b"new input after build")
        os.utime(source, ns=(ACCESS, OLD + 300))
        _, data = self.snapshot()
        self.checkout(source)
        self.restore(data)
        self.assertEqual(source.stat().st_mtime_ns, OLD + 300)
        self.assertGreater(source.stat().st_mtime_ns, dependency_info.stat().st_mtime_ns)

    def test_malformed_last_entry_never_partially_restores_first_entry(self):
        first = self.source("a.rs")
        self.source("z.rs")
        _, original = self.snapshot()
        valid = json.loads(original)
        mutations = [
            ("path", "../outside.rs"), ("path", "/outside.rs"), ("path", "a//b.rs"),
            ("path", "a/./b.rs"), ("path", "a\\b.rs"), ("path", "a/../b.rs"),
            ("path", ".git/config"), ("path", "target/compiled.rs"),
            ("path", "cargo-cache/anything"), ("path", "code.rlib"),
            ("path", "a.rs"), ("path", ["z.rs"]), ("path", "bad\ud800.rs"),
            ("sha256", "f" * 63), ("sha256", []), ("size", -1), ("size", True),
            ("size", cache.MAX_FILE_BYTES + 1), ("mtime_ns", -1), ("mtime_ns", 1.5),
            ("mtime_ns", cache.MAX_TIMESTAMP_NS + 1), ("mode", True), ("mode", 0o10000),
            ("git_mode", []), ("git_mode", "120000"),
        ]
        for key, value in mutations:
            with self.subTest(key=key, value=repr(value)):
                manifest = copy.deepcopy(valid)
                manifest["entries"][-1][key] = value
                self.checkout(first)
                result = self.restore(json.dumps(manifest).encode())
                self.assertEqual(result["status"], "skipped", result)
                self.assertEqual(result["restored"], 0)
                self.assertEqual(first.stat().st_mtime_ns, CHECKOUT)
                self.assertFalse(self.manifest.exists())

    def test_invalid_context_schema_and_duplicate_keys_are_conservative_misses(self):
        source = self.source()
        _, original = self.snapshot()
        valid = json.loads(original)
        invalid_documents = [b"{", b"[]", b"null", b"\xff", original[:-1] + b"garbage"]
        invalid_documents.append(original.replace(b'"schema_version": 1', b'"schema_version": 1, "schema_version": 1'))
        for key, value in (
            ("schema_version", 2), ("schema_version", True), ("hash_algorithm", "md5"),
            ("repo_root", str(self.repo / "different")), ("target_root", str(self.base / "other-cache")),
            ("entries", {}), ("snapshot_started_ns", -1), ("snapshot_finished_ns", 0),
        ):
            document = copy.deepcopy(valid)
            document[key] = value
            invalid_documents.append(json.dumps(document).encode())
        invalid_documents.append(original.replace(b'"size": 8', b'"size": NaN'))
        for data in invalid_documents:
            with self.subTest(data=data[:70]):
                self.checkout(source)
                result = self.restore(data)
                self.assertEqual(result["status"], "skipped", result)
                self.assertEqual(source.stat().st_mtime_ns, CHECKOUT)
                self.assertFalse(self.manifest.exists())

    def test_missing_manifest_does_not_inventory_or_touch_sources(self):
        source = self.source(mtime=CHECKOUT)
        with mock.patch.object(cache, "_read_tracked_files", side_effect=AssertionError("unexpected inventory")):
            result = cache.run("restore", str(self.repo), str(self.target))
        self.assertEqual(result["reason"], "manifest_missing")
        self.assertEqual(source.stat().st_mtime_ns, CHECKOUT)

    def test_leaf_symlink_does_not_touch_external_file(self):
        source = self.source()
        _, data = self.snapshot()
        outside = self.base / "outside.rs"
        outside.write_bytes(b"source A")
        self.checkout(outside)
        source.unlink()
        source.symlink_to(outside)
        result = self.restore(data)
        self.assertEqual(result["restored"], 0, result)
        self.assertEqual(outside.stat().st_mtime_ns, CHECKOUT)
        self.assertEqual(outside.stat().st_atime_ns, ACCESS)

    def test_parent_symlink_does_not_touch_external_file(self):
        source = self.source()
        _, data = self.snapshot()
        parent = source.parent
        external_parent = self.base / "moved-source-directory"
        parent.rename(external_parent)
        self.checkout(external_parent / source.name)
        parent.symlink_to(external_parent, target_is_directory=True)
        result = self.restore(data)
        self.assertEqual(result["restored"], 0, result)
        self.assertEqual((external_parent / source.name).stat().st_mtime_ns, CHECKOUT)

    def test_hard_link_is_not_read_or_timestamped(self):
        source = self.source()
        _, data = self.snapshot()
        self.checkout(source)
        outside = self.base / "outside-hardlink.rs"
        os.link(source, outside)
        result = self.restore(data)
        self.assertEqual(result["skip_reasons"]["not_single_link_regular_file"], 1)
        self.assertEqual(result["bytes_hashed"], 0)
        self.assertEqual(outside.stat().st_mtime_ns, CHECKOUT)
        self.assertEqual(outside.stat().st_atime_ns, ACCESS)

    def test_fifo_directory_and_symlink_sources_are_omitted_from_snapshot(self):
        fifo = self.repo / "fifo.rs"
        os.mkfifo(fifo)
        directory = self.repo / "directory.rs"
        directory.mkdir()
        linked = self.repo / "linked.rs"
        linked.symlink_to(fifo)
        self.tracked.update({name: "100644" for name in (fifo.name, directory.name, linked.name)})
        result, data = self.snapshot()
        self.assertEqual(result["recorded"], 0)
        self.assertEqual(result["skipped"], 3)
        self.assertEqual(json.loads(data)["entries"], [])

    def test_symlinked_target_is_an_error_and_never_followed(self):
        self.source()
        self.snapshot()
        moved_target = self.base / "real-target"
        self.target.rename(moved_target)
        self.target.symlink_to(moved_target, target_is_directory=True)
        result = self.run_helper("restore")
        self.assertEqual(result["status"], "error", result)
        self.assertTrue((moved_target / cache.MANIFEST_NAME).is_file())

    def test_symlinked_repository_component_is_never_followed(self):
        source = self.source()
        _, data = self.snapshot()
        self.checkout(source)
        alias = self.base / "alias"
        alias.symlink_to(self.repo, target_is_directory=True)
        self.manifest.write_bytes(data)
        result = self.run_helper("restore", repo=alias)
        self.assertEqual(result["status"], "skipped", result)
        self.assertEqual(source.stat().st_mtime_ns, CHECKOUT)
        self.assertFalse(self.manifest.exists())

    def test_manifest_symlink_is_consumed_without_reading_its_target(self):
        source = self.source(mtime=CHECKOUT)
        outside = self.base / "outside.json"
        outside.write_bytes(b"external data")
        os.utime(outside, ns=(ACCESS, OLD))
        self.manifest.symlink_to(outside)
        result = self.run_helper("restore")
        self.assertEqual(result["status"], "skipped", result)
        self.assertFalse(self.manifest.exists())
        self.assertEqual(source.stat().st_mtime_ns, CHECKOUT)
        self.assertEqual(outside.stat().st_atime_ns, ACCESS)
        self.assertEqual(outside.stat().st_mtime_ns, OLD)
        self.assertEqual(outside.read_bytes(), b"external data")

    def test_unremovable_old_manifest_is_fatal_for_cache_save(self):
        source = self.source()
        self.manifest.mkdir()
        result = self.run_helper("snapshot")
        self.assertEqual(result["status"], "error", result)
        self.assertEqual(result["reason"], "cannot_invalidate_old_manifest")
        self.assertEqual(source.stat().st_mtime_ns, OLD)

    def test_failed_inventory_snapshot_cannot_preserve_a_stale_old_manifest(self):
        self.source()
        self.snapshot()
        with mock.patch.object(cache, "_read_tracked_files", side_effect=cache.Unavailable("git_unavailable")):
            result = cache.run("snapshot", str(self.repo), str(self.target))
        self.assertEqual(result["status"], "skipped", result)
        self.assertFalse(self.manifest.exists())

    def test_atomic_publication_failure_leaves_no_stale_manifest_or_temporary(self):
        self.source()
        self.snapshot()
        with mock.patch.object(cache.os, "replace", side_effect=PermissionError("injected replace failure")):
            result = self.run_helper("snapshot")
        self.assertEqual(result["status"], "skipped", result)
        self.assertEqual(result["reason"], "manifest_publication_failed")
        self.assertEqual(list(self.target.iterdir()), [])

    def test_snapshot_omits_source_mutated_during_hash(self):
        source = self.source()
        real_hash = cache._hash_file

        def changing_hash(descriptor, size, budget):
            result = real_hash(descriptor, size, budget)
            source.write_bytes(b"source B")
            self.checkout(source)
            return result

        with mock.patch.object(cache, "_hash_file", side_effect=changing_hash):
            result, data = self.snapshot()
        self.assertEqual(result["recorded"], 0)
        self.assertEqual(result["skip_reasons"]["file_changed_during_hash"], 1)
        self.assertEqual(json.loads(data)["entries"], [])
        self.assertEqual(source.stat().st_mtime_ns, CHECKOUT)

    def test_snapshot_declines_if_an_earlier_file_changes_before_publication(self):
        first = self.source("a.rs")
        self.source("z.rs")
        self.snapshot()
        real_hash = cache._hash_file
        calls = 0

        def changing_hash(descriptor, size, budget):
            nonlocal calls
            result = real_hash(descriptor, size, budget)
            calls += 1
            if calls == 2:
                first.write_bytes(b"source B")
                self.checkout(first)
            return result

        with mock.patch.object(cache, "_hash_file", side_effect=changing_hash):
            result = self.run_helper("snapshot")
        self.assertEqual(result["reason"], "source_changed_before_publication", result)
        self.assertFalse(self.manifest.exists())

    def test_restore_declines_source_mutated_during_hash_even_if_bytes_match(self):
        source = self.source()
        _, data = self.snapshot()
        self.checkout(source)
        real_hash = cache._hash_file

        def changing_hash(descriptor, size, budget):
            result = real_hash(descriptor, size, budget)
            source.write_bytes(b"source A")
            self.checkout(source, CHECKOUT + 123)
            return result

        with mock.patch.object(cache, "_hash_file", side_effect=changing_hash):
            result = self.restore(data)
        self.assertEqual(result["restored"], 0, result)
        self.assertEqual(source.stat().st_mtime_ns, CHECKOUT + 123)

    def test_chmod_and_git_mode_changes_do_not_get_restored(self):
        source = self.source()
        _, data = self.snapshot()
        self.checkout(source)
        source.chmod(0o755)
        result = self.restore(data)
        self.assertEqual(result["skip_reasons"]["mode_mismatch"], 1)
        self.assertEqual(source.stat().st_mtime_ns, CHECKOUT)
        self.tracked["src/framework.rs"] = "100755"
        result = self.restore(data)
        self.assertEqual(result["skip_reasons"]["not_currently_tracked_with_same_mode"], 1)
        self.assertEqual(source.stat().st_mtime_ns, CHECKOUT)

    def test_current_future_timestamp_is_never_backdated(self):
        source = self.source()
        _, data = self.snapshot()
        future = time.time_ns() + 3_600_000_000_000
        self.checkout(source, future)
        result = self.restore(data)
        self.assertEqual(result["skip_reasons"]["current_timestamp_is_in_future"], 1)
        self.assertEqual(source.stat().st_mtime_ns, future)

    def test_cached_actual_future_timestamp_is_preserved_without_clamping(self):
        future = time.time_ns() + 3_600_000_000_000
        source = self.source(mtime=future)
        _, data = self.snapshot()
        self.checkout(source)
        result = self.restore(data)
        self.assertEqual(result["restored"], 1, result)
        self.assertEqual(source.stat().st_mtime_ns, future)

    def test_read_without_noatime_permission_is_a_miss_not_a_fallback(self):
        source = self.source()
        _, data = self.snapshot()
        self.checkout(source)
        real_open = cache.os.open

        def reject_source_open(path, flags, *args, **kwargs):
            if flags & os.O_NOATIME:
                raise PermissionError("no-atime permission denied")
            return real_open(path, flags, *args, **kwargs)

        with mock.patch.object(cache.os, "open", side_effect=reject_source_open):
            # The platform capability set contains the real function object.
            with mock.patch.object(cache, "_require_platform"):
                result = self.restore(data)
        self.assertEqual(result["restored"], 0, result)
        self.assertEqual(source.stat().st_mtime_ns, CHECKOUT)
        self.assertEqual(source.stat().st_atime_ns, ACCESS)

    def test_hash_limits_and_manifest_size_are_conservative_misses(self):
        source = self.source()
        _, data = self.snapshot()
        self.checkout(source)
        with mock.patch.object(cache, "MAX_MANIFEST_BYTES", 3):
            result = self.restore(data)
        self.assertEqual(result["status"], "skipped", result)
        self.assertEqual(source.stat().st_mtime_ns, CHECKOUT)
        with mock.patch.object(cache, "MAX_HASH_BYTES", 7):
            result = self.restore(data)
        self.assertEqual(result["reason"], "manifest_hash_limit", result)
        with mock.patch.object(cache, "MAX_FILE_BYTES", 7):
            result, data = self.snapshot()
        self.assertEqual(result["recorded"], 0)
        self.assertEqual(result["skip_reasons"]["hash_limit"], 1)

    def test_rejected_timestamp_write_is_a_miss_only_when_original_times_remain(self):
        source = self.source()
        _, data = self.snapshot()
        self.checkout(source)
        with mock.patch.object(cache.os, "utime", side_effect=PermissionError("injected rejection")):
            with mock.patch.object(cache, "_require_platform"):
                result = self.restore(data)
        self.assertEqual(result["skip_reasons"]["timestamp_write_rejected"], 1)
        self.assertEqual(source.stat().st_mtime_ns, CHECKOUT)
        real_utime = cache.os.utime

        def uncertain_write(descriptor, *, ns):
            real_utime(descriptor, ns=ns)
            raise OSError("injected uncertain failure after mutation")

        with mock.patch.object(cache.os, "utime", side_effect=uncertain_write):
            with mock.patch.object(cache, "_require_platform"):
                result = self.restore(data)
        self.assertEqual(result["status"], "error", result)
        self.assertEqual(result["reason"], "timestamp_changed_despite_write_error")
        self.assertFalse(self.manifest.exists())

    def test_timestamp_rounding_repairs_original_value_or_reports_fatal_failure(self):
        source = self.source()
        _, data = self.snapshot()
        self.checkout(source)
        real_utime = cache.os.utime
        calls = 0

        def rounding_once(descriptor, *, ns):
            nonlocal calls
            calls += 1
            return real_utime(descriptor, ns=(ns[0], ns[1] + 1) if calls == 1 else ns)

        with mock.patch.object(cache.os, "utime", side_effect=rounding_once):
            with mock.patch.object(cache, "_require_platform"):
                result = self.restore(data)
        self.assertEqual(result["skip_reasons"]["timestamp_precision_unsupported"], 1)
        self.assertEqual(source.stat().st_mtime_ns, CHECKOUT)
        self.assertEqual(source.stat().st_atime_ns, ACCESS)

        def rounding_always(descriptor, *, ns):
            return real_utime(descriptor, ns=(ns[0], ns[1] + 1))

        with mock.patch.object(cache.os, "utime", side_effect=rounding_always):
            with mock.patch.object(cache, "_require_platform"):
                result = self.restore(data)
        self.assertEqual(result["status"], "error", result)
        self.assertEqual(result["reason"], "timestamp_rollback_failed")

    def test_no_op_restore_does_not_issue_a_timestamp_write(self):
        source = self.source()
        _, data = self.snapshot()
        with mock.patch.object(cache.os, "utime", side_effect=AssertionError("unexpected timestamp write")):
            with mock.patch.object(cache, "_require_platform"):
                result = self.restore(data)
        self.assertEqual(result["already_equal"], 1)
        self.assertEqual(result["restored"], 0)
        self.assertEqual(source.stat().st_mtime_ns, OLD)

    def test_missing_target_is_a_miss_but_unsafe_root_arguments_are_errors(self):
        result = self.run_helper("restore", target=self.repo / "absent")
        self.assertEqual(result["reason"], "target_missing")
        for root in ("relative", str(self.repo) + "/../repo", "/", str(self.repo) + "/"):
            with self.subTest(root=root):
                result = cache.run("restore", root, str(self.target))
                self.assertEqual(result["status"], "error", result)
        self.assertEqual(self.run_helper("snapshot", target=self.repo)["status"], "error")

    def test_unsupported_platform_does_not_claim_reuse(self):
        with mock.patch.object(cache.sys, "platform", "darwin"):
            result = self.run_helper("restore")
        self.assertEqual(result["status"], "error", result)
        self.assertEqual(result["reason"], "linux_no_follow_no_atime_required")

    def test_index_parser_covers_repository_paths_unicode_modes_and_conflicts(self):
        digest = b"a" * 40
        records = (
            b"100644 " + digest + b" 0\tsrc/FerroUI.Build.Scan/lib.rs\x00"
            b"100755 " + digest + b" 0\tscripts/build script.sh\x00"
            b"120000 " + digest + b" 0\tsymlink\x00"
            b"160000 " + digest + b" 0\tsubmodule\x00"
            b"100644 " + digest + b" 0\t" + "src/λ.rs".encode() + b"\x00"
        )
        parsed = cache._parse_index(records)
        self.assertEqual(set(parsed), {"src/FerroUI.Build.Scan/lib.rs", "scripts/build script.sh", "src/λ.rs"})
        self.assertEqual(parsed["scripts/build script.sh"], "100755")
        for invalid in (
            records[:-1], records + records, records.replace(b" 0\t", b" 1\t", 1),
            b"100644 " + digest + b" 0\t../outside\x00", b"malformed\x00",
        ):
            with self.subTest(invalid=invalid[:70]), self.assertRaises(cache.Unavailable):
                cache._parse_index(invalid)

    def test_git_inventory_is_explicitly_root_scoped_without_commit_dates(self):
        outputs = [
            subprocess.CompletedProcess([], 0, (str(self.repo) + "\n").encode()),
            subprocess.CompletedProcess([], 0, b"100644 " + b"a" * 40 + b" 0\tsrc/outside-sample.rs\x00"),
        ]
        with mock.patch.object(cache.subprocess, "run", side_effect=outputs) as run:
            paths = cache._read_tracked_files(str(self.repo))
        self.assertEqual(paths, {"src/outside-sample.rs": "100644"})
        commands = [call.args[0] for call in run.call_args_list]
        self.assertEqual(commands[0], ["git", "-C", str(self.repo), "rev-parse", "--show-toplevel"])
        self.assertEqual(commands[1], ["git", "-C", str(self.repo), "ls-files", "--stage", "-z", "--full-name"])

    def test_cli_reports_machine_readable_miss_and_fatal_exit_status(self):
        output = io.StringIO()
        with mock.patch.object(sys, "argv", ["source_mtime_cache.py", "restore", "--repo", str(self.repo), "--target-dir", str(self.target)]):
            with redirect_stdout(output):
                code = cache.main()
        self.assertEqual(code, 0)
        self.assertEqual(json.loads(output.getvalue())["reason"], "manifest_missing")
        output = io.StringIO()
        with mock.patch.object(sys, "argv", ["source_mtime_cache.py", "restore", "--repo", "relative", "--target-dir", str(self.target)]):
            with redirect_stdout(output):
                code = cache.main()
        self.assertEqual(code, 1)
        self.assertEqual(json.loads(output.getvalue())["status"], "error")


if __name__ == "__main__":
    unittest.main()
