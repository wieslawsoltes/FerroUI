# FerroUI Designer Project

Portable Cargo solution discovery, multi-document buffers, resource URI resolution and cancellable build/run tasks. No renderer or dispatcher is linked.

`Solution::discover` uses locked offline Cargo metadata, preserving Cargo's actual workspace membership and binary targets. A directory without a manifest becomes a loose-file project. Descriptors are serializable and refreshable. Scanning skips generated/output directories and symbolic links and has explicit depth/file-count bounds.

`ProjectWorkspace` owns versioned open buffers, independent saved and authored text, active documents, safe save-as/rename, dirty-close protection and all-or-nothing multi-document updates. Invalid initial XAML remains editable. `store_document` retains the native session's history when switching tabs. Saves compare the disk contents with the previously saved snapshot, use a synchronized adjacent temporary file and atomic rename, and preserve permissions. External modifications require an explicit overwrite policy. Paths are checked against canonical solution roots, including symlink parents.

Save candidates are reserved with exclusive creation and bounded collision retries; cleanup only removes a candidate owned by that save. New-file and Save As publication use a no-replace hard link, so a destination created after the preflight check is preserved. Filesystems without hard-link support reject that publication while retaining the authored buffer and any existing destination.

`file_uri`, `file_path` and `resolve_uri` preserve UTF-8 paths and percent escapes; `ProjectWorkspace::resolve_path` maps native `ferres://assembly/path` resources into discovered project roots.

`BuildTask::cargo` creates reviewable, shell-free Check, Build or Run commands, validates binary targets and supports release configuration. Starting a task is an explicit host action and executes project build scripts with the host's privileges. Polling is nonblocking, stdout/stderr are independently captured with a bounded one-MiB queue, and cancellation terminates the owned process group on Unix or process tree on Windows. Dropping a running handle cancels it.

## Resource and task integration

Native `resm:` embedded names resolve from discovered document paths and optional `assembly=` queries. A host can use `register_resource` for custom embedded names; mappings retain the same canonical-root checks as ordinary file paths. File URI aliases share one buffer, and relative URI resolution preserves Unicode and escaped characters. `ferres` traversal cannot leave the selected assembly root even when another root belongs to the same solution.

All buffer operations accept filesystem aliases, including updates, history storage, saves, activation and close. Canonical identity is checked before validating a multi-document batch, so two URI spellings cannot apply conflicting updates to the same file. `file_uri` remains a lexical encoder; `canonical_file_uri` resolves existing files and the nearest existing parent of a new file when a host needs the identity used by project and language services.

Windows drive and extended drive paths encode as `file:///C:/…`; UNC and extended UNC paths encode as `file://server/share/…`. Decoding produces extended Windows paths for long-path IO. Root-relative, drive-relative, device-namespace and disguised-authority paths are rejected rather than assigned an implicit drive. Unix hosts continue to reject nonlocal file authorities. Canonicalization rejects dangling links, and a new document's path is checked again when saving so a newly introduced symlink cannot escape the solution roots.

```rust,ignore
let solution = Solution::discover(project_directory)?;
let mut workspace = ProjectWorkspace::new(solution);
let buffer = workspace.open(file_uri(path)?.as_str())?;
let version = buffer.version();
workspace.update_source(uri, version, edited_source)?;
workspace.save(uri)?; // rejects an external modification

let task = BuildTask::cargo(&workspace.solution.projects[0], TaskKind::Check, false)?;
let mut handle = task.start()?;
// Poll from the host; source updates and rendering do not block on the compiler.
let status = handle.poll();
let output = handle.take_output();
handle.cancel()?;
```

Task descriptions contain an executable, argument vector and working directory and do not interpret a shell command. Output decoding retains UTF-8 characters split across pipe reads. An exited status is visible only after both output readers finish, so the final compiler message is available. Unix cancellation also terminates descendants that retain output pipes after their group leader exits; Windows uses `taskkill /T` for the owned process tree. The portable tests exercise discovery with an actual minimal Cargo workspace, disk conflict detection, rename/save-as/tab history, atomic buffer batches, URI boundaries, process output and cancellation.
