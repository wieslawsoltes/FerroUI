# Binding, inspector and precision-layout workflows

The follow-up comparison with Visual Studio/Blend, Figma and Qt Widgets Designer identified
four concrete gaps in the existing native XAML workspace. The implementation uses the same
property editor factories, language service, document transactions and native canvas camera.

| Reference workflow | Previous FerroUI behavior | Added workflow |
| --- | --- | --- |
| Visual Studio's structured binding creation and conversion options | Expression text, source element, path and mode | Structured native binding sources, update timing, converter, formatting and fallback values |
| Visual Studio's Category/Name/Source property arrangements; Qt's modified-property emphasis | Categories, text search and an authored-value search token | Discoverable arrangement and value-source filters, typed search tokens and source navigation |
| Figma's ruler guides; Blend's independent grid and snapline preferences | Fixed grid and object snaplines | Native rulers, editable guides, independent snapping controls and configurable nudge distances |
| Reference-aware element naming in a XAML authoring workflow | Editing Name could leave existing references behind | Namescope-aware rename, preserved aliases, reference updates and guarded reset in one undo transaction |

These are native FerroUI workflows. Bindings retain FerroUI's supported sources and update
semantics, and canvas measurements use logical layout units.

## Structured binding editor

Open a property's **⋯** value editor, choose **Binding / expression**, and use the binding
fields. The expression remains available for application-specific or unsupported markup.
The reusable `PropertyBindingDraft` owns only a local source draft. The containing value
editor's **Apply value** remains the single document transaction; Cancel or dismissal
discards the draft.

The source controls support DataContext, a named element, Self, TemplatedParent, an ancestor
type/level in the native visual or logical tree, and an explicit source expression. Choosing
a source removes conflicting `ElementName`, `RelativeSource` and `Source` arguments.

The advanced fields include binding mode, update-source trigger, delay in milliseconds,
converter, converter parameter/culture, string format, fallback value and target-null value.
Optional values distinguish absence, literal text and an expression. An empty literal is not
silently converted to an absent value. An explicit null stays an expression using its authored
XAML namespace.

The editor retains the original framework extension name, namespace aliases, unknown arguments
and untouched argument spelling. A binding opened and accepted without changes retains its
exact source. Editing raw expression text does not silently apply stale structured fields over
it: reload the fields from the expression before combining the two editing paths.

Framework binding identity comes from the accepted lexical namespace. An application extension
called `Binding` is not treated as the built-in type. Generated relative sources use a separately
resolved `RelativeSource` name, since a CLR namespace containing a binding type need not contain
the relative-source extension. If a needed type has no visible namespace alias, the editor
reports that requirement without guessing a prefix.

Native transaction callbacks still recheck document revision, syntax identity, selection,
permission and definition scope. Retaining a flyout across an undo or selection change cannot
write to the new selection.

## Inspector view and source navigation

The **View** control beside the property search box provides three arrangements:

- **Category** retains collapsible property categories.
- **Alphabetical** presents the properties in name order.
- **Value source** separates local literals, bindings, resources, object values, other
  expressions, mixed selections and inherited/default values.

Filters select all properties, authored values, bindings, resource references, objects,
expressions, mixed values or inherited/default values. Search text combines with the selected
filter. The source indicators and reset semantics remain attached to the original values.

| Search token | Matches |
| --- | --- |
| `@modified`, `@local` | Properties with an authored override |
| `@binding` | Native binding expressions |
| `@resource` | Native static/dynamic resource expressions |
| `@object` | Authored property-element values |
| `@expression` | Binding, resource and other markup expressions |
| `@mixed` | Different values across the current selection |
| `@default`, `@inherited` | No authored local value |

For example, `@binding text` narrows the inspector to binding-backed properties whose name or
category matches “text”. Every search term must match. A clear-filter action is available when
the selected arrangement and query produce no rows. Switching the view never authors XAML,
changes the document selection or creates history.

Property context actions can reveal the authored attribute/property element and navigate to a
uniquely resolved resource or named binding source. Resource lookup uses the actual preview
theme context; name lookup uses the language service's XAML namescopes. Missing, external or
ambiguous references remain explicit rather than choosing an arbitrary declaration. A pending
source draft or stale property menu cannot navigate using an old source range.

Hosts can use `PropertyInspectorView`, `PropertyInspectorSort` and `PropertyInspectorFilter`
through the workspace, and call `reveal_property_source` or `go_to_property_definition`.

Numeric editing also preserves the distinction between a useful editor value and a runtime
sentinel. Inherited `NaN`/infinite defaults do not become accidental local numeric overrides.
When an actual native floating-point property accepts **Auto**, the commit validates and
serializes its native `NaN` representation so that the preview can reload it. String values,
GridLength `Auto` and unresolved application types retain their own spelling. Direct session,
component and MCP property edits use the same native type check.

## Rulers, guides and snapping

Open **Guides and snapping** on the canvas toolbar. Show rulers, guide lines and the snap grid
independently. Snap to grid, guides and object edges are separate choices. The editor accepts
exact guide coordinates, grid spacing, screen-space snap tolerance and small/large nudge steps.
Apply changes the local view; Cancel discards them.

With rulers visible, drag from a ruler to create a guide. Drag an existing guide to move it,
or hold Alt while dragging to copy it. Returning a guide to its ruler removes it. Selected
guides also support Delete while the canvas owns keyboard focus. Escape cancels the current
drag. The staged editor offers add, exact position, remove and clear actions for keyboard use.

Guide coordinates are artboard coordinates. Rulers and guide strokes are projected through
the same camera used for selection and preview rendering, so they retain their screen size
while pan and zoom change the visible coordinates. Tick and grid generation is bounded even
at the camera's extreme offsets and zoom levels. Snapping tolerance is measured in screen
logical pixels and converted to artboard units for each accepted gesture.

Guide placement and movement remain local drafts until release. Source, scope, settings,
camera, viewport-size and pointer-capture changes retire an incompatible drag. Changing these
preferences also retires unfinished authored canvas gestures before changing their snap index.
The preview's authored controls, undo history and collaboration source remain unchanged.

Each retained document tab and reusable-definition scope owns its guide settings. Save As
retains the document's view under its new URI. Closing and reopening the application does not
claim to persist these settings to disk. Hosts can capture and restore matching in-memory
view snapshots, or persist the serializable `CanvasGuideSettings` model and apply it to a
newly opened document.

The public `CanvasGuideSettings`, `CanvasGuide`, `CanvasGuideAxis`, `CanvasGuidesEditor`,
`CanvasGuideEditSnapshot` and `CanvasGuideViewSnapshot` types expose the same functionality to
embedded hosts. `DesignSurface::set_canvas_guides` connects the settings to movement and
resizing. The workspace provides add/move/remove, accepted-snapshot apply and view restoration.
Settings validate finite values, bounded guide positions/counts and nonzero unique guide IDs.

## Reference-aware element names

Editing **Name** in the regular property inspector now invokes a semantic rename. It updates
the declaration and resolved references together, including named bindings, `#name` paths,
XAML references, recognized named-target attributes, selectors and encoded visual-state
targets. Ordinary strings and unrelated template namescopes keep their original values.

Existing `Name` or XAML directive aliases are retained. For example, editing a control declared
with `q:Name` updates that directive rather than adding another plain `Name` attribute. The
inspector presents one canonical Name row. The language reference grammar also recognizes
framework binding aliases while preserving unrelated application extensions with the same
local names.

The command rejects invalid identifiers, duplicate declarations, ambiguous existing names,
and a name that would unexpectedly capture an already unresolved reference. Resetting a name
is allowed only when it leaves no known reference or visual-state target unresolved. The
failure leaves the source untouched. Editing Name does not create an animation key when
recording is enabled.

`DesignerSession::rename_element` and the source planner `plan_element_names` provide
the same operation to hosts. The session's ordinary `set_property`/`reset_property` Name
paths use it, including property automation. One undo restores the original declaration,
references, aliases, comments and state metadata together.

## Native render evidence

These captures come from the actual themed workspace, its native property controls and the
same CPU renderer used by the designer render regressions.

| Workflow | Captures |
| --- | --- |
| Binding authoring | [Structured binding editor](images/designer-binding-editor.png) · [Live native binding preview](images/designer-binding-preview.png) |
| Property inspection | [Value-source arrangement](images/designer-property-sources.png) · [View options](images/designer-property-view-options.png) · [Resource filter](images/designer-property-resource-filter.png) |
| Precision layout | [Rulers and selected guides](images/designer-canvas-guides.png) · [Exact guide and snap settings](images/designer-canvas-guides-editor.png) |

## References

- [Visual Studio Properties window: Category, Alphabetical and Property Source](https://learn.microsoft.com/en-us/visualstudio/ide/properties-window?view=visualstudio)
- [Visual Studio binding authoring in XAML Designer](https://learn.microsoft.com/en-us/visualstudio/xaml-tools/walkthrough-binding-to-data-in-xaml-designer?view=visualstudio)
- [Blend artboard grid and snapping settings](https://learn.microsoft.com/en-us/visualstudio/ide/reference/xaml-designer?view=visualstudio)
- [Qt Widgets Designer property editors and modified values](https://doc.qt.io/qt-6/designer-widget-mode.html)
- [Figma ruler guides: creation, movement and deletion](https://help.figma.com/hc/en-us/articles/360040449713-Add-guides-to-the-canvas-or-frames)

The broader [workflow comparison](feature-comparison.md), [property-editor extension guide](property-editors.md)
and [canvas navigation guide](canvas-navigation.md) describe the existing complementary features.
