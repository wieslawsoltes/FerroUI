# Property editors and value authoring

The property inspector and the component Definition inspector use the same native editor
pipeline. Each row carries an immutable source snapshot, type metadata, visible resources,
and a guarded semantic commit callback. Editors do not receive a runtime object to mutate.

## Interaction model

Frequent properties use compact inline fields. The **⋯** value action opens a staged editor
with four sources: **Value**, **Binding / expression**, **Resource**, and **Object / collection**.
The selected source determines which semantic command is sent on **Apply value**. Cancel,
Escape and light dismissal discard the local draft. Reset is a distinct action that removes
the authored attribute or property element and reveals the inherited/default value.

For one authored literal or object value, the Resource page also offers **Create resource** with
an explicit key and owner scope. It extracts the accepted authored value and replaces the original
property with a resource reference in one transaction. An uncommitted field draft is not silently
used as the resource value. Unsupported scalar conversions, invisible scopes and shadowed keys
report an error without changing the document.

| Property family | Native editing experience |
| --- | --- |
| Boolean and nullable Boolean | Three-state checkbox, with mixed and inherited state retained by the row |
| Integers and floating-point values | Typed numeric input, modifier-key adjustment and scrubbing where exact; large integers remain validated text to avoid floating-point rounding |
| Registered enums | Members come from `MarkupType` rather than the property name; unknown authored text stays visible |
| Registered flags | Member checklist in the staged value editor, with a complete serialized value committed once |
| Point, Size, Vector, Rect, Matrix | Labeled components; Enter commits the whole value and Escape restores the original draft |
| RelativePoint and RelativeRect | Component fields with explicit pixel/percentage units and uniform-unit validation |
| Thickness and CornerRadius | Compact spacing/corner fields; existing linked spacing controls remain available |
| GridLength | Pixel, star and Auto modes |
| Color and solid brush | Native color swatch and existing ColorView flyout with alpha and Apply |
| Gradient brush | Object editor with gradient stops, brush coordinates, opacity and editable source |
| Transform | Object editor with transform groups and translate, rotate, scale, skew and matrix objects |
| Font family and weight | Editable family text, system family choices, named weight choices and validation |
| TimeSpan and Duration | Native/portable syntax validation while preserving authored text |
| Bindings and other markup extensions | Exact expression source plus explicit source-element, path and mode controls for Binding |
| Resource references | Scope-visible resources, static/dynamic lookup choice and definition identity validation |
| Custom objects and collections | Source-preserving object draft, reflected property types, custom attributes, insertion/removal/reordering and XAML fallback |

The source of a value is independent of its type. A Point may be inherited, mixed, a literal,
a binding, or an object property element. Opening a row never replaces these forms. Effective
runtime values may initialize a local field draft, but the source origin and reset state remain
the authored ones. Property elements are shown as object values rather than flattened strings.

Binding construction quotes path strings and preserves the qualified Binding name, converter,
fallback and other non-source arguments. An explicit source change removes conflicting
`Source`, `RelativeSource` and `ElementName` arguments before writing the selected source.
Other markup extensions stay as their exact expression text until the user requests a change.

## Type resolution and validation

`resolve_property_editor_metadata(context, property)` prefers the actual `FerroProperty`
type handle. `MarkupType` supplies enum members, flags, nullable forms, parsers and reflected
descriptions. Source-only definition fields can use a fully qualified runtime type identity;
known framework value types also have an unambiguous short-name fallback. A custom type named
`Point` in an unrelated namespace is not assigned the framework Point serializer.

The native path checks a registered property's validation callback when a conversion to its
runtime value type is available. Portable `PropertyValueShape`, `PropertyKind::Typed` and
`PropertyKind::Flags` keep corresponding source validation usable without native controls.
The integer validator checks the actual Rust range rather than a float approximation.

Resource pickers first restrict resources by lexical visibility and shadowing. Registered
object identities, inheritance and casts exclude known incompatible object types. Unknown
application conversions and primitive/string conversions remain available; their loader or
custom editor owns the conversion contract. This does not claim that every visible resource
can be assigned to every property.

An Apply callback rechecks document URI, source revision, AST identity, selection, definition
scope and editing permission. A popup retained across an undo, selection change, document
switch or definition-scope change cannot write through its stale row. Source replacement is
one document transaction and one undo step. Collaboration and native reload continue through
the existing session pipeline.

## Extending the designer

Register a `PropertyEditorFactory` through `DesignerWorkspace::register_property_editor`.
The most recently registered factory is tried first. Return `None` for unsupported properties
so the next factory or the built-in resolver can handle them.

Existing factories that implement `create(&PropertyEditorContext)` remain supported.
New factories should implement `create_with_metadata` to inspect the type, value source,
nullable state, object source, resources, element names and validation callback. Use
`metadata.apply(&context, edit)` for native validation and the guarded semantic commit.

```rust,ignore
use ferroui_base::{Ref, input::{InputElement, Key}};
use ferroui_controls::{Control, DataValidationErrors, TextBox};
use ferroui_designer::{
    PropertyEditorContext, PropertyEditorFactory, PropertyEditorMetadata,
    PropertyValueEdit,
};
use std::rc::Rc;

struct ApplicationValueEditor;

impl PropertyEditorFactory for ApplicationValueEditor {
    fn create_with_metadata(
        &self,
        context: &PropertyEditorContext,
        metadata: &PropertyEditorMetadata,
    ) -> Option<Ref<Control>> {
        if context.runtime_type.as_deref() != Some("my_application::Angle") {
            return None;
        }
        let context = context.clone();
        let metadata = metadata.clone();
        let field = TextBox::new();
        field.set_text(context.value.as_deref());
        let weak = field.downgrade();
        field.add_handler(InputElement::key_down_event(), move |_, event| {
            let Some(field) = weak.upgrade() else { return; };
            if event.key == Key::Escape {
                field.set_text(context.value.as_deref());
                DataValidationErrors::clear_errors(&field);
            } else if event.key == Key::Enter {
                let edit = PropertyValueEdit::Literal(field.text().unwrap_or_default());
                match metadata.apply(&context, edit) {
                    Ok(()) => DataValidationErrors::clear_errors(&field),
                    Err(message) => DataValidationErrors::set_errors(
                        &field, Some(vec![Rc::new(message)]),
                    ),
                }
            } else {
                return;
            }
            event.set_handled(true);
        });
        Some(field.upcast::<Control>())
    }
}
```

Hosts that embed only the editor controls can call `create_property_editor` directly, supply
their own metadata and `PropertyValueCommit`, and retain their own document transaction system.
`PropertyVectorEditor` and `PropertyObjectDraft` are independently reusable. The latter edits
a temporary AST and retains unknown attributes, namespace aliases, comments and collection
order; its complete source is committed only when the owning host accepts it.

Object draft fields resolve registered styled/direct properties and ordinary markup properties
without constructing application objects. The same prioritized factory chain is available inside
those drafts. Their callbacks modify only the local object AST; the outer Apply command remains
the sole document transaction. Nested contexts have no original-document target IDs, so custom
editors should use the supplied callback rather than look up a runtime object. Factories may
replace a nested scalar or property element atomically, and unknown property types retain a
text/XAML path. Resource references inside a local draft can be authored as expressions.

The semantic commands are `Literal`, `Expression`, `Object`, `Resource { node, dynamic }`,
`ExtractResource { owner, key }` and `Reset`. Resource commands carry a source identity instead of an unverified display key.
Object commands carry authored XAML, not serialized runtime object graphs. Applications with
special value formats, converters, assets or modal pickers provide an editor factory and normal
runtime type registration. There is no automatic code generation for arbitrary Rust types.

## Design references

The [historical WPF property editor architecture](https://learn.microsoft.com/en-us/previous-versions/visualstudio/visual-studio-2010/bb514507(v=vs.100))
separates inline, extended and dialog views from the host's property/value model. FerroUI uses
this separation for reusable controls, consistent chrome and guarded source callbacks.

[Qt Widgets Designer](https://doc.qt.io/qt-6/designer-widget-mode.html) uses type-appropriate
inputs and compound fields. [Qt Design Studio's binding workflow](https://doc.qt.io/qtdesignstudio/quick-property-bindings.html)
informs the explicit source/path controls alongside an expression field.

[Unity UI Builder's custom attribute and property drawer extension points](https://docs.unity.com/en-us/engine/6000.0/manual/uitoolkits/uielements/uie-structure-ui/uie-custom-controls/custom-control-customize-uxml-attributes)
reinforce keeping application-specific editing as a supported extension contract. FerroUI
uses registered runtime metadata plus prioritized editor factories instead of adopting a
foreign IDE's serialization or plugin ABI.

Qt's [property-sheet extension contract](https://doc.qt.io/qt-6/qdesignerpropertysheetextension.html)
also makes the undo boundary explicit: direct property-sheet assignment bypasses the designer's
undo-aware cursor. FerroUI enforces the corresponding rule through its revision/selection/scope
guards and semantic commit callback, including for legacy and metadata-aware custom factories.
