# FerroUI visual designer

The visual designer is a reusable Rust editing system with a FerroUI workspace. It uses a source-preserving XAML document as the authority for visual edits, source edits, undo, collaboration and automation. The implementation is tracked in this branch from its first architecture commit.

## Module boundaries

- **Document engine:** lossless XAML syntax, source spans, stable node identity, validated atomic edits and undo/redo. Invalid source remains editable while the last valid preview is retained.
- **Design tools:** extensible property metadata and editors, panel layout adapters, selection, geometry, snapping, manipulation and drag/drop. Tools produce document transactions instead of mutating the preview behind the source editor.
- **Animation and interaction authoring:** independent source-identified programs, extension codecs, deterministic keyframes, timeline editing and full native timing; source-preserving visual-state/transition and behavior/trigger/action models.
- **Collaboration:** replicated document operations, deterministic conflict resolution, causal identity, presence and transport-independent synchronization.
- **Collaboration hosting:** private durable rooms, actor-bound member roles, invitations, epoch checkpoints, offline outboxes and a background service over the existing isolated MCP transport. Network and disk work stay on workers; source acceptance stays on the UI thread.
- **Projects and language:** Cargo/loose solutions, versioned multi-document buffers, resource mapping, shell-free cancellable tasks, runtime-derived schema and shared editor/LSP semantics. Cross-document edits pin every affected version.
- **Workspace:** FerroUI controls for the tool palette, object tree, properties, XAML editor, artboard and timeline. The workspace is embeddable; the sample is a host of the same components.
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

## Reference workflows

The workspace follows the synchronized Design/XAML views, artboard, toolbox, document outline and Properties workflow described in [Microsoft's XAML designer overview](https://learn.microsoft.com/en-us/visualstudio/xaml-tools/creating-a-ui-by-using-xaml-designer-in-visual-studio). The animation workflow follows [Blend's objects and timeline editor](https://learn.microsoft.com/en-us/visualstudio/xaml-tools/creating-a-ui-by-using-blend-for-visual-studio) and [keyframe authoring](https://learn.microsoft.com/en-us/visualstudio/xaml-tools/animate-objects-in-xaml-designer).

Implementation, embedding examples, host contracts, deployment and validation commands are documented in the [Designer Studio guide](README.md).
