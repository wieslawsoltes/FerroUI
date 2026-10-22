# Optional native Cargo source timestamp reuse

`source_mtime_cache.py` preserves source timestamps only when the current
Git-tracked file has exactly the bytes and size recorded with a restored Cargo
target cache. It addresses a specific native CI observation: the
[999ab562 fingerprint log](../../../docs/ferrodesign-evidence/2026-10-22-999ab562-native/README.md)
reports unchanged framework sources as newer than restored dependency information
after checkout. The complete preceding source comparison contains documentation
and workflow changes, and the workflow change only enables fingerprint logging.
This helper does not establish a measured reduction in FerroUI build time or any
change in editor runtime performance.

The helper is optional and supports the existing Linux native CI environment.
It uses only Python's standard library and read-only Git inventory commands.
It does not invoke Cargo, edit Cargo fingerprints, change compiled artifacts,
modify source bytes, infer Git commit dates, or override Cargo's toolchain,
environment, configuration, feature, profile or dependency checks.

## Required ordering

Use the **same absolute Git repository root** and **same absolute Cargo target
directory** in both commands. The repository argument must be the repository
root, not `samples/FerroDesign`: framework inputs are outside that sample.

1. Finish checkout and restore the Cargo target cache.
2. Run `restore`, before any Cargo process or other source writer starts.
3. Run the normal Cargo commands, preserving their original failure status.
4. After **all** Cargo processes and source writers have finished, run `snapshot`
   even when a build failed.
5. Save that same target directory with the resulting manifest, subject to the
   error rules below. Do not separately cache or copy the manifest.

```sh
python3 samples/FerroDesign/scripts/source_mtime_cache.py restore \
  --repo "$GITHUB_WORKSPACE" \
  --target-dir "$CARGO_TARGET_DIR"

# The workflow's ordinary native Cargo commands run here.

python3 samples/FerroDesign/scripts/source_mtime_cache.py snapshot \
  --repo "$GITHUB_WORKSPACE" \
  --target-dir "$CARGO_TARGET_DIR"
```

Both variables must already contain normalized absolute paths. Neither command
creates the target directory. A missing target is a cache miss. The helper must
not overlap a build, checkout, generator, editor or another helper invocation.
Its stable-file checks detect many concurrent changes, but they are not a lock
against an arbitrary writer changing paths or files between system calls.

### Manifest lifetime and command failures

The fixed metadata path is:

```text
<Cargo target directory>/.ferrodesign-source-mtimes-v1.json
```

Restore reads and **consumes** this metadata name before checking or changing
source timestamps. Snapshot invalidates any previous manifest before scanning
and publishes a complete replacement using an exclusive temporary file, file
sync and atomic rename inside the verified target directory. A failed snapshot
therefore cannot leave an old manifest paired with newly compiled artifacts.

Each command prints one JSON diagnostic. Integration must inspect its exit
status and must not use an unconditional `|| true`:

| Result | Exit | Integration behavior |
| --- | --- | --- |
| `restored` or `snapshotted` | 0 | Continue the established sequence. Counts may include conservative file skips. |
| `skipped` | 0 | Continue ordinary Cargo checks. Missing, malformed or incompatible metadata gives no source timestamp reuse. A failed snapshot has removed the old manifest. |
| `error` | 1 | Stop this optimization path and do not save the target cache. On restore, do not start Cargo after an uncertain timestamp mutation. Preserve the diagnostic as a CI failure. |

Errors include an unsafe target path, inability to invalidate old metadata,
unsupported descriptor operations, and inability to verify or repair a timestamp
write. A refused source-file write is a conservative skip only when its original
timestamps can still be verified. Ordinary build failures must remain failures;
successful snapshotting does not change their result.

## What is recorded and restored

Schema version 1 binds the manifest to the exact repository and target paths.
It records the actual snapshot start/end times and, for each eligible source,
its repository-relative path, lowercase SHA-256, byte size, actual nanosecond
mtime, permission mode and Git regular-file mode. The snapshot does not replace
mtime with the build finish time or a repository date.

Git inventory uses `git -C <repository> ls-files --stage -z --full-name`. Only
stage-zero regular entries are candidates. Tracked Rust, XAML, assets, manifests,
configuration and tracked generated sources all use the same checks. The explicit
target directory, paths containing known Cargo output directories and native/wasm
compiled-artifact suffixes are excluded. Untracked generated inputs receive no
timestamp restoration and remain subject to Cargo's normal freshness checks.

Before any source timestamp writes, restore validates the **entire** manifest:
version, context, exact field types, unique JSON keys and paths, safe relative
paths, digest syntax, timestamp ranges and resource limits. A malformed final
entry prevents reuse of otherwise valid earlier entries.

Each candidate then requires all of the following:

- It is still Git-tracked with the same regular-file mode.
- Every directory component and the file itself opens without following a
  symlink. The file is regular and has exactly one hard link.
- Size, SHA-256 and permission mode match the co-restored entry.
- Device, inode, size, mode, link count, mtime and ctime remain stable across
  hashing; the named path still identifies that file immediately before writing.
- The current source timestamp is not in the future. Such timestamps are left
  untouched, rather than backdated.

Source reads require Linux `O_NOATIME`, so rejected files do not get incidental
atime updates. Lack of permission for this flag skips that source; there is no
ordinary-open fallback. A descriptor-based `utime` changes only the verified
file's mtime and preserves its existing atime. The result must round-trip the
requested nanoseconds exactly; otherwise the original timestamps are restored,
or an explicit fatal error is returned if repair cannot be verified. As with any
mtime write, the filesystem may update ctime.

Changed bytes, changed size, changed mode, newly tracked paths, paths removed from
the index, missing files, symlinks, hard links and unstable files receive no
timestamp reuse. Same bytes with an already equal mtime require no timestamp
write. An actual future timestamp recorded in the cached snapshot is not clamped
to the present: restoring that timestamp preserves Cargo's conservative stale
ordering.

### Failed and partial builds

Recording the real post-build source mtime is essential. For example, if source
mtime is 200 and a failed build leaves old dependency information at 100,
snapshot records 200. The next checkout may give the same source bytes mtime
300; restoring 200 still leaves the source newer than dependency information.
The helper does not make that failed unit fresh. A source changed after a build
likewise retains its actual later timestamp.

This guarantee depends on keeping the manifest with the target it describes and
on the ordinary source timestamps accurately reflecting writes. Independently
copying manifests, concurrent builds/source writers, or other tools deliberately
backdating modified source files are unsupported. The manifest is an integrity
check against current source bytes, not an authenticity mechanism for an
untrusted build cache.

### Limits and diagnostics

The current limits are 100,000 indexed entries, 32 MiB of parsed Git index output,
64 MiB of manifest data, 128 MiB per source file and 1 GiB of total source hashing
per command. Git commands have 30-second deadlines. Larger or unavailable
sources are conservative misses; invalid manifest-wide limits prevent reuse.
Snapshot rechecks recorded file identities before publishing, so a source
changed after its individual hash also prevents publication of stale metadata.

Diagnostics include operation, status, manifest path, tracked/recorded/restored
counts, already-equal timestamps, bytes hashed and categorized skips. Restore
counts describe cached candidates; newly tracked paths absent from the manifest
are left alone and do not appear as individually skipped entries.

## Validation scope

Run the filesystem-only unit suite with:

```sh
python3 -B -m unittest discover -s samples/FerroDesign/scripts \
  -p test_source_mtime_cache.py -v
```

The tests use actual temporary files and descriptor operations, with a simulated
Git inventory. They cover exact nanosecond/atime behavior, unchanged compiled
artifacts, successful and failed-build timestamp ordering, changed content,
root-scoped inventory parsing, symlink and hardlink rejection, malformed metadata,
late source changes, atomic publication failure, consumed metadata, resource
limits, future timestamps and fatal uncertain-write handling. They launch no
Git process, Cargo build, browser, server or network request.

The separate [offline Cargo probe](test_source_mtime_cargo.py) uses a temporary
two-package repository and a tracked build-script asset. With pinned Cargo
1.99.0, its local execution restored all seven unchanged sources and reported
both crates Fresh. A changed dependency was left untouched and rebuilt both
crates, changing executable output from `42|blue` to `43|blue`. A changed
`rerun-if-changed` asset rebuilt its consumer while the dependency stayed Fresh,
producing `43|gold`. It uses private Cargo home/target directories and no network
or FerroUI compilation.

```sh
python3 samples/FerroDesign/scripts/test_source_mtime_cargo.py
```

Both native CI jobs run these safeguards before project compilation. Snapshot
and target-cache save require a successful restore/probe and a job that was not
cancelled; completed failed builds can still preserve useful partial outputs.
A failed restore or correctness probe blocks the independent application gate.
The browser job retains its existing cache behavior for this initial rollout.
The local probe establishes this isolated fixture's correctness. Actual FerroUI
cache-hit rates and reduced CI duration still require subsequent completed jobs;
no editor runtime speedup follows from this build optimization.
No application performance or Figma parity conclusion follows from this helper.
