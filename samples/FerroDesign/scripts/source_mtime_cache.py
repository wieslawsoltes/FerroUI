#!/usr/bin/env python3
"""Optional, content-verified source timestamp reuse for a co-restored Cargo target.

This Linux-only helper never reads or writes Cargo's fingerprints. See
SOURCE_MTIME_CACHE.md for the required restore/build/snapshot/cache-save ordering.
"""

from __future__ import annotations

import argparse
from contextlib import contextmanager
import hashlib
import json
import os
from pathlib import PurePosixPath
import re
import stat
import subprocess
import sys
import time
import uuid


SCHEMA_VERSION = 1
MANIFEST_NAME = ".ferrodesign-source-mtimes-v1.json"
MAX_FILES = 100_000
MAX_MANIFEST_BYTES = 64 * 1024 * 1024
MAX_INDEX_BYTES = 32 * 1024 * 1024
MAX_FILE_BYTES = 128 * 1024 * 1024
MAX_HASH_BYTES = 1024 * 1024 * 1024
MAX_TIMESTAMP_NS = (1 << 63) - 1
HASH_CHUNK_BYTES = 1024 * 1024
REGULAR_GIT_MODES = {"100644", "100755"}
# Tracked build inputs can include fonts, XAML, images and generated Rust. Omit
# build output trees and native/wasm artifacts even if accidentally Git-tracked.
OUTPUT_COMPONENTS = {".git", "target", ".fingerprint", "incremental"}
OUTPUT_SUFFIXES = {
    ".rlib", ".rmeta", ".o", ".obj", ".a", ".so", ".dylib", ".dll",
    ".exe", ".wasm", ".pdb", ".dwp", ".dSYM",
}


class Unavailable(Exception):
    """A conservative cache miss; normal Cargo validation may continue."""


class UnsafeCache(Exception):
    """Do not save this target cache or continue after an uncertain mutation."""


class SkipFile(Exception):
    pass


def _absolute_root(value: str) -> str:
    if not isinstance(value, str) or not value.startswith("/"):
        raise UnsafeCache("roots_must_be_absolute")
    if "\x00" in value or os.path.normpath(value) != value or value == "/":
        raise UnsafeCache("roots_must_be_normalized")
    try:
        value.encode("utf-8", errors="strict")
    except UnicodeError as error:
        raise UnsafeCache("roots_must_be_utf8") from error
    return value


def _path_parts(value: str) -> tuple[str, ...]:
    if not isinstance(value, str) or not value or "\x00" in value or "\\" in value:
        raise Unavailable("invalid_source_path")
    parts = tuple(value.split("/"))
    if any(part in {"", ".", "..", ".git"} for part in parts):
        raise Unavailable("invalid_source_path")
    # Reject unpaired JSON surrogate escapes as well as non-UTF-8 index names.
    try:
        value.encode("utf-8", errors="strict")
    except UnicodeError as error:
        raise Unavailable("invalid_source_path") from error
    return parts


def _eligible_path(path: str, repo_root: str, target_root: str) -> bool:
    parts = _path_parts(path)
    if any(part in OUTPUT_COMPONENTS for part in parts):
        return False
    if PurePosixPath(path).suffix in OUTPUT_SUFFIXES:
        return False
    return not (repo_root + "/" + path).startswith(target_root + "/")


def _require_platform() -> None:
    required = ("O_NOFOLLOW", "O_DIRECTORY", "O_NOATIME", "O_CLOEXEC")
    if not sys.platform.startswith("linux") or any(not hasattr(os, key) for key in required):
        raise UnsafeCache("linux_no_follow_no_atime_required")
    if os.open not in os.supports_dir_fd or os.utime not in os.supports_fd:
        raise UnsafeCache("file_descriptor_operations_required")


@contextmanager
def _directory(root: str):
    """Open every absolute component without resolving a symlink."""
    flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC
    descriptor = os.open("/", flags)
    try:
        for component in root.split("/")[1:]:
            next_descriptor = os.open(component, flags, dir_fd=descriptor)
            os.close(descriptor)
            descriptor = next_descriptor
        yield descriptor
    finally:
        os.close(descriptor)


@contextmanager
def _source_parent(root_descriptor: int, path: str):
    parts = _path_parts(path)
    flags = os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC
    descriptor = os.dup(root_descriptor)
    try:
        for component in parts[:-1]:
            next_descriptor = os.open(component, flags, dir_fd=descriptor)
            os.close(descriptor)
            descriptor = next_descriptor
        yield descriptor, parts[-1]
    finally:
        os.close(descriptor)


def _identity(info: os.stat_result) -> tuple[int, ...]:
    # Reading does not modify atime because source opens require O_NOATIME.
    return (
        info.st_dev, info.st_ino, info.st_mode, info.st_nlink, info.st_size,
        info.st_mtime_ns, info.st_ctime_ns,
    )


def _still_named(root_descriptor: int, path: str, expected: os.stat_result) -> bool:
    with _source_parent(root_descriptor, path) as (parent, name):
        actual = os.stat(name, dir_fd=parent, follow_symlinks=False)
    return _identity(actual) == _identity(expected)


class HashBudget:
    def __init__(self) -> None:
        self.used = 0

    def permits(self, size: int) -> bool:
        return 0 <= size <= MAX_FILE_BYTES and self.used + size <= MAX_HASH_BYTES


def _hash_file(descriptor: int, size: int, budget: HashBudget) -> str:
    digest = hashlib.sha256()
    read_count = 0
    while True:
        block = os.read(descriptor, min(HASH_CHUNK_BYTES, size - read_count + 1))
        if not block:
            break
        read_count += len(block)
        budget.used += len(block)
        if read_count > size or budget.used > MAX_HASH_BYTES:
            raise SkipFile("file_changed_during_hash")
        digest.update(block)
    if read_count != size:
        raise SkipFile("file_changed_during_hash")
    return digest.hexdigest()


@contextmanager
def _verified_source(root_descriptor: int, path: str, budget: HashBudget):
    # NONBLOCK prevents an unexpected FIFO from hanging before fstat rejects it.
    flags = os.O_RDONLY | os.O_NOFOLLOW | os.O_NOATIME | os.O_NONBLOCK | os.O_CLOEXEC
    with _source_parent(root_descriptor, path) as (parent, name):
        descriptor = os.open(name, flags, dir_fd=parent)
        try:
            before = os.fstat(descriptor)
            if not stat.S_ISREG(before.st_mode) or before.st_nlink != 1:
                raise SkipFile("not_single_link_regular_file")
            if not 0 <= before.st_mtime_ns <= MAX_TIMESTAMP_NS:
                raise SkipFile("unsupported_timestamp")
            if not budget.permits(before.st_size):
                raise SkipFile("hash_limit")
            digest = _hash_file(descriptor, before.st_size, budget)
            after = os.fstat(descriptor)
            if _identity(before) != _identity(after) or not _still_named(root_descriptor, path, after):
                raise SkipFile("file_changed_during_hash")
            yield descriptor, before, digest
        finally:
            os.close(descriptor)


def _parse_index(data: bytes) -> dict[str, str]:
    if len(data) > MAX_INDEX_BYTES or (data and not data.endswith(b"\x00")):
        raise Unavailable("invalid_or_oversized_git_index")
    records = data[:-1].split(b"\x00") if data else []
    if len(records) > MAX_FILES:
        raise Unavailable("git_file_limit")
    paths: dict[str, str] = {}
    seen = set()
    try:
        for record in records:
            metadata, raw_path = record.split(b"\t", 1)
            mode, object_id, stage = metadata.decode("ascii").split(" ")
            path = raw_path.decode("utf-8", errors="strict")
            _path_parts(path)
            if stage != "0" or path in seen:
                raise Unavailable("unmerged_or_duplicate_git_index")
            if mode not in REGULAR_GIT_MODES | {"120000", "160000"}:
                raise Unavailable("unsupported_git_index_mode")
            if not re.fullmatch(r"(?:[0-9a-f]{40}|[0-9a-f]{64})", object_id):
                raise Unavailable("invalid_git_object_id")
            seen.add(path)
            if mode in REGULAR_GIT_MODES:
                paths[path] = mode
    except (UnicodeError, ValueError) as error:
        raise Unavailable("invalid_git_index") from error
    return paths


def _read_tracked_files(repo_root: str) -> dict[str, str]:
    try:
        root = subprocess.run(
            ["git", "-C", repo_root, "rev-parse", "--show-toplevel"],
            check=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=30,
        ).stdout.decode("utf-8", errors="strict").rstrip("\n")
        if root != repo_root:
            raise Unavailable("repository_root_mismatch")
        result = subprocess.run(
            ["git", "-C", repo_root, "ls-files", "--stage", "-z", "--full-name"],
            check=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, timeout=30,
        )
        return _parse_index(result.stdout)
    except (OSError, subprocess.SubprocessError, UnicodeError) as error:
        raise Unavailable("git_inventory_unavailable") from error


def _unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise Unavailable("duplicate_manifest_key")
        result[key] = value
    return result


def _integer(value, minimum: int, maximum: int) -> bool:
    return type(value) is int and minimum <= value <= maximum


def _decode_manifest(data: bytes, repo_root: str, target_root: str) -> dict:
    if len(data) > MAX_MANIFEST_BYTES:
        raise Unavailable("manifest_size_limit")
    try:
        value = json.loads(
            data.decode("utf-8", errors="strict"), object_pairs_hook=_unique_object,
            parse_constant=lambda _: (_ for _ in ()).throw(Unavailable("nonfinite_manifest_number")),
        )
    except (ValueError, UnicodeError, RecursionError) as error:
        raise Unavailable("invalid_manifest_json") from error
    expected = {
        "schema_version", "hash_algorithm", "repo_root", "target_root",
        "snapshot_started_ns", "snapshot_finished_ns", "entries",
    }
    if type(value) is not dict or set(value) != expected:
        raise Unavailable("invalid_manifest_schema")
    if type(value["schema_version"]) is not int or value["schema_version"] != SCHEMA_VERSION:
        raise Unavailable("unsupported_manifest_version")
    if value["hash_algorithm"] != "sha256" or value["repo_root"] != repo_root or value["target_root"] != target_root:
        raise Unavailable("manifest_context_mismatch")
    if not _integer(value["snapshot_started_ns"], 0, MAX_TIMESTAMP_NS) or not _integer(
        value["snapshot_finished_ns"], value["snapshot_started_ns"], MAX_TIMESTAMP_NS
    ):
        raise Unavailable("invalid_snapshot_time")
    entries = value["entries"]
    if type(entries) is not list or len(entries) > MAX_FILES:
        raise Unavailable("manifest_file_limit")
    paths = set()
    total_size = 0
    for entry in entries:
        if type(entry) is not dict or set(entry) != {"path", "sha256", "size", "mtime_ns", "mode", "git_mode"}:
            raise Unavailable("invalid_manifest_entry")
        path = entry["path"]
        if not _eligible_path(path, repo_root, target_root) or path in paths:
            raise Unavailable("duplicate_or_excluded_manifest_path")
        if not isinstance(entry["sha256"], str) or not re.fullmatch(r"[0-9a-f]{64}", entry["sha256"]):
            raise Unavailable("invalid_manifest_digest")
        if not _integer(entry["size"], 0, MAX_FILE_BYTES) or not _integer(entry["mtime_ns"], 0, MAX_TIMESTAMP_NS):
            raise Unavailable("invalid_manifest_file_stat")
        if not _integer(entry["mode"], 0, 0o7777) or not isinstance(entry["git_mode"], str) or entry["git_mode"] not in REGULAR_GIT_MODES:
            raise Unavailable("invalid_manifest_file_mode")
        paths.add(path)
        total_size += entry["size"]
    if total_size > MAX_HASH_BYTES:
        raise Unavailable("manifest_hash_limit")
    return value


def _discard_manifest(target_descriptor: int) -> None:
    try:
        # Unlink removes this metadata name itself, never a symlink's target.
        os.unlink(MANIFEST_NAME, dir_fd=target_descriptor)
    except FileNotFoundError:
        pass
    except OSError as error:
        raise UnsafeCache("cannot_invalidate_old_manifest") from error


def _consume_manifest(target_descriptor: int) -> bytes | None:
    descriptor = None
    try:
        try:
            descriptor = os.open(
                MANIFEST_NAME, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK | os.O_CLOEXEC,
                dir_fd=target_descriptor,
            )
        except FileNotFoundError:
            return None
        except OSError as error:
            raise Unavailable("manifest_unreadable") from error
        info = os.fstat(descriptor)
        if not stat.S_ISREG(info.st_mode) or info.st_nlink != 1 or info.st_size > MAX_MANIFEST_BYTES:
            raise Unavailable("manifest_not_bounded_regular_file")
        with os.fdopen(descriptor, "rb", closefd=False) as stream:
            data = stream.read(MAX_MANIFEST_BYTES + 1)
        if _identity(info) != _identity(os.fstat(descriptor)):
            raise Unavailable("manifest_changed_during_read")
        return data
    finally:
        if descriptor is not None:
            os.close(descriptor)
        # Consume even malformed metadata. No old manifest may be saved beside
        # artifacts modified by the subsequent build if its new snapshot fails.
        _discard_manifest(target_descriptor)


def _publish_manifest(target_descriptor: int, manifest: dict) -> None:
    data = (json.dumps(manifest, indent=2, sort_keys=True, ensure_ascii=True) + "\n").encode("utf-8")
    if len(data) > MAX_MANIFEST_BYTES:
        raise Unavailable("manifest_size_limit")
    temporary = MANIFEST_NAME + "." + uuid.uuid4().hex + ".tmp"
    descriptor = None
    try:
        descriptor = os.open(
            temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC,
            0o600, dir_fd=target_descriptor,
        )
        with os.fdopen(descriptor, "wb", closefd=False) as stream:
            stream.write(data)
            stream.flush()
            os.fsync(descriptor)
        os.close(descriptor)
        descriptor = None
        os.replace(temporary, MANIFEST_NAME, src_dir_fd=target_descriptor, dst_dir_fd=target_descriptor)
        os.fsync(target_descriptor)
    except OSError as error:
        _discard_manifest(target_descriptor)
        raise Unavailable("manifest_publication_failed") from error
    finally:
        if descriptor is not None:
            os.close(descriptor)
        try:
            os.unlink(temporary, dir_fd=target_descriptor)
        except FileNotFoundError:
            pass


def _skip(report: dict, reason: str) -> None:
    report["skipped"] += 1
    report["skip_reasons"][reason] = report["skip_reasons"].get(reason, 0) + 1


def _snapshot(root_descriptor: int, target_descriptor: int, repo_root: str, target_root: str, report: dict) -> None:
    _discard_manifest(target_descriptor)
    started = time.time_ns()
    tracked = _read_tracked_files(repo_root)
    report["tracked_files"] = len(tracked)
    entries = []
    stable_stats = {}
    budget = HashBudget()
    for path, git_mode in sorted(tracked.items()):
        if not _eligible_path(path, repo_root, target_root):
            _skip(report, "excluded_build_output")
            continue
        try:
            with _verified_source(root_descriptor, path, budget) as (_, info, digest):
                entries.append({
                    "path": path, "sha256": digest, "size": info.st_size,
                    "mtime_ns": info.st_mtime_ns, "mode": stat.S_IMODE(info.st_mode), "git_mode": git_mode,
                })
                stable_stats[path] = info
        except SkipFile as error:
            _skip(report, str(error))
        except OSError:
            _skip(report, "source_unavailable_or_unsafe")
    report["bytes_hashed"] = budget.used
    # Catch sources changed after their individual hash while other inputs were
    # scanned. Concurrent builds/source writers are unsupported even with this
    # check: run snapshot only after every Cargo process has finished.
    for entry in entries:
        try:
            if not _still_named(root_descriptor, entry["path"], stable_stats[entry["path"]]):
                raise Unavailable("source_changed_before_publication")
        except OSError as error:
            raise Unavailable("source_changed_before_publication") from error
    _publish_manifest(target_descriptor, {
        "schema_version": SCHEMA_VERSION, "hash_algorithm": "sha256",
        "repo_root": repo_root, "target_root": target_root,
        "snapshot_started_ns": started, "snapshot_finished_ns": time.time_ns(), "entries": entries,
    })
    report.update(status="snapshotted", recorded=len(entries))


def _apply_timestamp(descriptor: int, info: os.stat_result, mtime_ns: int) -> None:
    try:
        os.utime(descriptor, ns=(info.st_atime_ns, mtime_ns))
    except OSError as error:
        try:
            rejected = os.fstat(descriptor)
        except OSError as verification_error:
            raise UnsafeCache("cannot_verify_rejected_timestamp_write") from verification_error
        if rejected.st_mtime_ns != info.st_mtime_ns or rejected.st_atime_ns != info.st_atime_ns:
            raise UnsafeCache("timestamp_changed_despite_write_error") from error
        raise SkipFile("timestamp_write_rejected") from error
    try:
        after = os.fstat(descriptor)
    except OSError as error:
        raise UnsafeCache("cannot_verify_written_timestamp") from error
    before_object = (info.st_dev, info.st_ino, info.st_mode, info.st_nlink, info.st_size)
    after_object = (after.st_dev, after.st_ino, after.st_mode, after.st_nlink, after.st_size)
    if before_object != after_object:
        raise UnsafeCache("source_changed_during_timestamp_write")
    if after.st_mtime_ns == mtime_ns and after.st_atime_ns == info.st_atime_ns:
        return
    # Decline filesystems that cannot round-trip nanosecond timestamps. Repair
    # the original value before allowing normal Cargo validation to continue.
    try:
        os.utime(descriptor, ns=(info.st_atime_ns, info.st_mtime_ns))
        restored = os.fstat(descriptor)
    except OSError as error:
        raise UnsafeCache("timestamp_rollback_failed") from error
    if restored.st_mtime_ns != info.st_mtime_ns or restored.st_atime_ns != info.st_atime_ns:
        raise UnsafeCache("timestamp_rollback_failed")
    raise SkipFile("timestamp_precision_unsupported")


def _restore(root_descriptor: int, repo_root: str, target_root: str, data: bytes, report: dict) -> None:
    # Validate every entry first. A malformed final entry must not cause a
    # partially applied restore of otherwise valid earlier entries.
    manifest = _decode_manifest(data, repo_root, target_root)
    tracked = _read_tracked_files(repo_root)
    report["tracked_files"] = len(tracked)
    started = time.time_ns()
    budget = HashBudget()
    for entry in manifest["entries"]:
        path = entry["path"]
        if tracked.get(path) != entry["git_mode"]:
            _skip(report, "not_currently_tracked_with_same_mode")
            continue
        try:
            with _verified_source(root_descriptor, path, budget) as (descriptor, info, digest):
                if info.st_size != entry["size"] or digest != entry["sha256"]:
                    _skip(report, "content_mismatch")
                    continue
                if stat.S_IMODE(info.st_mode) != entry["mode"]:
                    _skip(report, "mode_mismatch")
                    continue
                if info.st_mtime_ns > started:
                    _skip(report, "current_timestamp_is_in_future")
                    continue
                if info.st_mtime_ns == entry["mtime_ns"]:
                    report["already_equal"] += 1
                    continue
                # Recheck immediately before the descriptor-based write. This
                # also detects a parent path replaced while this file was read.
                if _identity(info) != _identity(os.fstat(descriptor)) or not _still_named(root_descriptor, path, info):
                    raise SkipFile("source_changed_before_restore")
                _apply_timestamp(descriptor, info, entry["mtime_ns"])
                report["restored"] += 1
        except SkipFile as error:
            _skip(report, str(error))
        except OSError:
            _skip(report, "source_unavailable_or_unsafe")
    report.update(status="restored", bytes_hashed=budget.used)


def run(operation: str, repo_root: str, target_root: str) -> dict:
    report = {
        "operation": operation, "schema_version": SCHEMA_VERSION, "status": "skipped",
        "tracked_files": 0, "recorded": 0, "restored": 0, "already_equal": 0,
        "skipped": 0, "skip_reasons": {}, "bytes_hashed": 0,
    }
    try:
        if operation not in {"restore", "snapshot"}:
            raise UnsafeCache("unknown_operation")
        _require_platform()
        repo_root = _absolute_root(repo_root)
        target_root = _absolute_root(target_root)
        if repo_root == target_root or repo_root.startswith(target_root + "/"):
            raise UnsafeCache("target_must_not_contain_repository")
        report["manifest"] = target_root + "/" + MANIFEST_NAME
        try:
            target_context = _directory(target_root)
            with target_context as target_descriptor:
                data = None
                if operation == "restore":
                    data = _consume_manifest(target_descriptor)
                    if data is None:
                        raise Unavailable("manifest_missing")
                else:
                    # Invalidate before opening the repository or reading Git.
                    _discard_manifest(target_descriptor)
                try:
                    with _directory(repo_root) as root_descriptor:
                        if operation == "snapshot":
                            _snapshot(root_descriptor, target_descriptor, repo_root, target_root, report)
                        else:
                            _restore(root_descriptor, repo_root, target_root, data, report)
                except OSError as error:
                    raise Unavailable("repository_or_source_unavailable") from error
        except FileNotFoundError as error:
            raise Unavailable("target_missing") from error
        except OSError as error:
            raise UnsafeCache("target_unavailable_or_unsafe") from error
    except Unavailable as error:
        report.update(status="skipped", reason=str(error))
    except UnsafeCache as error:
        report.update(status="error", reason=str(error))
    return report


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("operation", choices=("restore", "snapshot"))
    parser.add_argument("--repo", required=True, help="Exact absolute Git repository root")
    parser.add_argument("--target-dir", required=True, help="Exact absolute co-cached Cargo target directory")
    arguments = parser.parse_args()
    report = run(arguments.operation, arguments.repo, arguments.target_dir)
    print(json.dumps(report, sort_keys=True))
    return 1 if report["status"] == "error" else 0


if __name__ == "__main__":
    raise SystemExit(main())
