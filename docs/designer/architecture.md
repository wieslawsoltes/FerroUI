# FerroUI visual designer

The visual designer is a reusable Rust editing system with a FerroUI workspace. It uses a source-preserving XAML document as the authority for visual edits, source edits, undo, collaboration and automation. The implementation is tracked in this branch from its first architecture commit.

## Module boundaries

- **Document engine:** lossless XAML syntax, source spans, stable node identity, validated atomic edits and undo/redo. Invalid source remains editable while the last valid preview is retained.
- **Design tools:** extensible property metadata and editors, panel layout adapters, selection, geometry, snapping, manipulation and drag/drop. Tools produce document transactions instead of mutating the preview behind the source editor.
- **Animation:** deterministic keyframe evaluation, timeline editing, easing, repeat/reverse, recording, and XAML import/export.
- **Collaboration:** replicated document operations, deterministic conflict resolution, causal identity, presence and transport-independent synchronization.
- **Workspace:** FerroUI controls for the tool palette, object tree, properties, XAML editor, artboard and timeline. The workspace is embeddable; the sample is a host of the same components.
- **Runtime integration:** reuse FerroUI.Inspect for visual/logical inspection, FerroUI.HotReload for accepted debug preview transactions, and FerroUI.Mcp for authenticated transport and UI-thread dispatch. Release builds stage a runtime XAML load before replacing accepted preview content.

## Invariants

1. Source edits, visual tools and MCP commands enter the same transaction boundary.
2. A failed transaction cannot partially change source, history or the live preview.
3. A no-op parse/serialize preserves source bytes; edits preserve unrelated comments, whitespace, bindings, resources and namespace declarations.
4. Preview errors do not discard authored XAML. Runtime acceptance and document validity are separate states.
5. Runtime objects and dispatcher-owned services stay on their UI thread. Portable engines do not depend on a rendering backend.
6. Collaboration operations have stable identities; delivery order and duplicate delivery cannot silently change the chosen conflict result.
7. Extensions register explicit capabilities for property editors, layout adapters and commands. Unknown markup is preserved.

## Reference workflows

The workspace follows the synchronized Design/XAML views, artboard, toolbox, document outline and Properties workflow described in [Microsoft's XAML designer overview](https://learn.microsoft.com/en-us/visualstudio/xaml-tools/creating-a-ui-by-using-xaml-designer-in-visual-studio). The animation workflow follows [Blend's objects and timeline editor](https://learn.microsoft.com/en-us/visualstudio/xaml-tools/creating-a-ui-by-using-blend-for-visual-studio) and [keyframe authoring](https://learn.microsoft.com/en-us/visualstudio/xaml-tools/animate-objects-in-xaml-designer).

Implementation, embedding examples, engine contracts and validation commands are documented in the [Designer Studio guide](README.md). The product scope and remaining workflows are explicitly listed there.
