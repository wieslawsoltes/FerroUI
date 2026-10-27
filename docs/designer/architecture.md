# FerroUI visual designer

The visual designer is a reusable Rust editing system with a FerroUI workspace. It uses a source-preserving XAML document as the authority for visual edits, source edits, undo, collaboration and automation. The implementation is tracked in this branch from its first architecture commit.

## Module boundaries

- **Document engine:** lossless XAML syntax, source spans, stable node identity, validated atomic edits and undo/redo. Invalid source remains editable while the last valid preview is retained.
- **Design tools:** extensible property metadata and editors, panel layout adapters, selection, geometry, snapping, manipulation and drag/drop. Tools produce document transactions instead of mutating the preview behind the source editor.
- **Animation and interaction authoring:** independent source-identified programs, extension codecs, deterministic keyframes, timeline editing and full native timing; source-preserving visual-state/transition and behavior/trigger/action models.
- **Collaboration:** replicated document operations, deterministic conflict resolution, causal identity, presence and transport-independent synchronization.
- **Collaboration hosting:** private durable rooms, actor-bound member roles, invitations, epoch checkpoints, offline outboxes and a background service over the existing isolated MCP transport. Network and disk work stay on workers; source acceptance stays on the UI thread.
- **Projects and language:** Cargo/loose solutions, versioned multi-document buffers, resource mapping, shell-free cancellable tasks, runtime-derived schema and shared editor/LSP semantics. Cross-document edits pin every affected version.
- **Workspace:** a WinUI-themed native shell with independently reusable asset catalog and timeline controls, TreeDataGrid hierarchy/project/property inspectors, XAML editor and artboard. View/navigation state is separate from document state; virtual cells and drag gestures pin their source context before writing. The workspace is embeddable; the sample is a host of the same components.
- **Runtime integration:** reuse FerroUI.Inspect for visual/logical inspection, FerroUI.HotReload for accepted debug preview transactions, and FerroUI.Mcp for authenticated transport and UI-thread dispatch. Release builds stage a runtime XAML load before replacing accepted preview content.
- **Preview hosting:** actual platform Windows, registered application root factories and resource/design-preview factories retain the real runtime and source URI. Affine tools consume measured native geometry and effective layout properties.

## Invariants

1. Source edits, visual tools and MCP commands enter the same transaction boundary.
2. A failed transaction cannot partially change source, history or the live preview.
3. A no-op parse/serialize preserves source bytes; edits preserve unrelated comments, whitespace, bindings, resources and namespace declarations.
4. Preview errors do not discard authored XAML. Runtime acceptance and document validity are separate states.
5. Runtime objects and dispatcher-owned services stay on their UI thread. Portable engines do not depend on a rendering backend.
6. Collaboration operations have stable identities; delivery order and duplicate delivery cannot silently change the chosen conflict result.
7. Extensions register explicit capabilities for property editors, layout adapters and commands. Unknown markup is preserved.
8. Room acknowledgements follow file synchronization and atomic replacement, plus parent-directory synchronization on Unix. An indeterminate commit fences the store until restart. A resumed peer never promotes unacknowledged local operations into the server version. Native disconnect queues its final checkpoint write; its completion status is the durability confirmation.
9. Tab/source revisions and room actor identity fence asynchronous results. Joining/checkpoint rebasing requires explicit source review and saves the previous document and offline state. A prepared resolution becomes the saved peer only after UI acceptance succeeds and the worker commits it; a stale acceptance preserves the original remote review.
10. Generated animation/state ownership is explicit; unrelated programs, complex payloads, namespaces and application-defined semantics survive edits.
11. Pointer snapping prepares peer geometry once per gesture. Gesture commits require their original immutable AST identity and revision; equal numeric node IDs in another document do not establish identity.
12. Hierarchy drops, property cells and keyframe editors produce session transactions. UI virtualization, filtering and view switches cannot silently mutate the document or discard pending source text. Retained editors, outline models and gestures pin the document URI, revision and shared AST allocation, so reopening the same URI cannot reuse a stale authoring context.

## Internal implementation

| Module | Responsibility |
| --- | --- |
| Core `ast/parser`, `ast/reconciliation`, `ast/validation` | Lossless XML syntax, identity matching and validated snapshot deserialization |
| Core `document/editing`, `document/changes` | Validated patches/structural edits and ordered change classification |
| `workspace_shell` | Design/Split/XAML, navigation, measured zoom, pane sizing and layout commands |
| `inspector_controls` | TreeDataGrid row models, selection/expansion, lazy typed property cells and stale-write guards |
| `inspector_drag` | Authored hierarchy drop planning, source identity guards, native row events and atomic reparenting |
| `asset_toolbox` | Extensible catalog, filtering, vector tiles and host-owned activation/placement |
| `timeline_lane`, `workspace_timeline` | Ruler/keys, pointer and keyboard interaction, timeline view state and contextual editing |
| Project `uri/windows`, `workspace/storage` | Platform file-URI rules and owned atomic-save candidates |

The measured changes and benchmark tradeoffs are recorded in [performance evidence](performance.md).

## Reference workflows

The workspace follows the synchronized Design/XAML views, artboard, toolbox, document outline and Properties workflow described in [Microsoft's XAML designer overview](https://learn.microsoft.com/en-us/visualstudio/xaml-tools/creating-a-ui-by-using-xaml-designer-in-visual-studio). The animation workflow follows [Blend's objects and timeline editor](https://learn.microsoft.com/en-us/visualstudio/xaml-tools/creating-a-ui-by-using-blend-for-visual-studio) and [keyframe authoring](https://learn.microsoft.com/en-us/visualstudio/xaml-tools/animate-objects-in-xaml-designer).

Implementation, embedding examples, host contracts, deployment and validation commands are documented in the [Designer Studio guide](README.md).
