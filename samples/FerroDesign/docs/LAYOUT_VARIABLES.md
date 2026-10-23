# Auto-layout spacing variables

Auto-layout frames can bind their four padding sides, main gap and cross gap to
local Number variables. Open **Local variables** from the navigation rail,
select a variable, choose the spacing property under **Bind selected layers**,
and select **Bind this variable**. The existing variable modes, alias resolver,
native controls, layout engine and document history handle the resulting edit.

## Properties and values

| Native property | Canonical document key | Effective value |
| --- | --- | --- |
| Top padding | `layout.padding.top` | `AutoLayout.padding.top` |
| Right padding | `layout.padding.right` | `AutoLayout.padding.right` |
| Bottom padding | `layout.padding.bottom` | `AutoLayout.padding.bottom` |
| Left padding | `layout.padding.left` | `AutoLayout.padding.left` |
| Layout gap | `layout.gap` | `AutoLayout.gap` |
| Layout cross gap | `layout.cross_gap` | `AutoLayout.cross_gap` |

Each key has one owner in the existing `Node.variable_bindings` map. There is no
uniform-padding key whose application order could override individual sides.
Use the same variable on all four keys for uniform responsive padding, or bind
independent variables to any subset. Bindings serialize in ordinary editable
JSON; no migration, parallel token model or rendering representation is added.

Values must resolve to finite, nonnegative Numbers in **every addressable
mode**, including aliases and Default fallback. Negative values, wrong types,
missing targets and unsupported keys reject the whole authoring operation.
The target must be a container with auto layout. Explicit binding and unbinding
reject a locked layer or ancestor. Existing bindings continue to follow
variable-definition changes on locked and hidden layers, as local variables
already do elsewhere in the document.

The model currently has one active variable mode for the document. Selecting
**Use this mode** applies the mode across collections; this feature does not
introduce independent active modes per collection or per frame.

## Reflow and transaction ownership

A changed binding, variable definition or active mode is evaluated on a private
candidate document. The operation resolves variables, runs the existing complete
row/column/wrap/grid and nested Hug/Fill arrangement, and validates the result
before one history commit. Arrangement errors, including finite inputs whose
combined geometry overflows, reject the candidate. Other edits cannot join a
currently held canvas transaction through variable authoring.

Undo and redo restore the complete authored document: variable definitions,
mode, bindings, padding, gaps, child positions, Hug dimensions and component
metadata. Runtime revision counters continue to advance to invalidate render
caches; those counters are intentionally absent from portable JSON.

An identical binding, mode selection or definition is a no-op. It preserves
history, redo, render identity and unsettled imported geometry. It does not
use an unchanged operation as an opportunity to normalize unrelated layout.

Legacy imported documents can retain invalid bindings for diagnosis. The
low-level `Document::apply_variables` consumes recognized spacing keys but
does not write a negative, nonfinite or wrongly typed value into geometry.
The validated editor API rejects those values. This legacy safeguard is not
an acceptance of invalid authored variables.

## Literal edits, unlinking and components

A different numeric value in the detailed padding dialog detaches only the
binding of the side whose effective number changed. Untouched sides retain
their exact stored precision and binding. The compact uniform Padding field
and gap fields use the same ownership rule. Equal numeric submission preserves
the binding; **Unbind** explicitly relinquishes it while retaining its current
resolved value. Deleting an otherwise unused spacing variable likewise freezes
the resolved layout and removes its bindings.

The detailed padding dialog's linked checkbox is an editing mode. Toggling it
does not add, remove or modify variable bindings. A subsequent linked numeric
edit updates all four values and detaches the sides whose values changed.
Removing auto layout through the generic editor API removes its spacing
bindings while preserving unrelated width, fill and other variable keys.

Binding or unbinding instance spacing, or changing a bound spacing literal,
records the existing `layout` component override. Component synchronization
preserves the overridden layout **and its spacing binding ownership**, including
an explicitly removed binding. The current component model protects the whole
layout; independent per-side component override keys are outside this feature.
Converting a supported container to vector artwork removes spacing bindings
because the resulting path no longer owns auto layout.

## Native UI and performance

The variables table and spacing controls are actual FerroUI controls inside the
compiled XAML shell. The canvas continues to use FerroUI's public geometry,
brush and text APIs with Vello. There is no browser DOM layout editor.

Generic node updates capture at most six `(variable ID, number)` pairs for
spacing ownership. They do not clone image payloads, text, track arrays or the
binding map. A node without bindings skips this capture. Variable changes still
use the document-wide validation/reflow transaction because a single spacing
value can resize nested ancestors and move distant siblings. This feature does
not claim constant-time reflow or a frame-rate improvement.

## Coverage and scope

`layout_variables_tests.rs` covers typed canonical keys, alias/mode propagation,
exact authored undo/redo, inactive-mode validation, invalid target batches,
locks, changed-side ownership, generic edits, unbinding/deletion, component
synchronization, transaction exclusion, overflow, empty/grid/Hug layouts,
legacy invalid values and real redo retention after unchanged operations.
Native regressions execute the compiled-XAML rail button, property choice,
Bind/Unbind buttons and fractional padding fields. The
[`layout-variables` browser scenario](../tests/layout-variables-browser.mjs)
uses trusted pointer/keyboard input and checks reflow, exact history,
mode activation, literal ownership and persisted JSON.

Runtime results are recorded by exact source in [VALIDATION.md](VALIDATION.md).
Authored tests and syntax checks do not establish a completed native or browser
gate. On-canvas padding handles, variable expressions, per-frame modes,
per-side component overrides and numerical visual parity with Figma remain
outside this implementation.
