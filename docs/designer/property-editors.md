# Property editors and value authoring

The property inspector and the component Definition inspector use the same native editor
pipeline. Each row carries an immutable source snapshot, type metadata, visible resources,
and a guarded semantic commit callback. Editors do not receive a runtime object to mutate.

## Find properties and follow their source

The **View** button beside property search arranges rows by **Category**, **Name (A–Z)** or
**Value source**. Categories and value-source groups retain independent collapse states;
searching temporarily expands matching groups without changing those preferences.
The source groups distinguish local literals, bindings, resource references, object values,
other expressions, mixed selection values and inherited/default values. Runtime-only objects
remain read-only and identify runtime values and bindings without claiming an authored value.
Effective runtime priority and binding diagnostics remain available in the row tooltip.

The **Show** selector filters to authored values, bindings, resources, objects, expressions,
mixed values or inherited/default values. Search text applies in addition to this filter.
An empty result provides **Clear filters and search**, which keeps the chosen arrangement.

| Search token | Matching rows |
| --- | --- |
| `@modified` or `@local` | Properties with an authored attribute or object value; includes authored bindings and resources |
| `@binding` | Native Binding, ReflectionBinding, CompiledBinding and TemplateBinding expressions |
| `@resource` | Native StaticResource and DynamicResource expressions |
| `@object` | Authored property-element values |
| `@mixed` | Different authored values in the current selection |
| `@default` or `@inherited` | No authored value on the current selection |
| `@expression` | All binding, resource and other markup-extension expressions |

Combine a token with property/category words, for example `@resource background` or
`@modified layout`. Expression classification resolves lexical namespace aliases and the
framework's registered extension identities; an application extension named `Binding` or
`StaticResource` remains an application expression. Escaped `{}` literals remain literals.

Right-click an authored property name for **Show authored XAML**. A uniquely resolvable
resource or named binding source also offers **Go to referenced definition**. Resource
navigation follows the selected preview's theme and lexical resource scopes; binding navigation
uses the language service's XAML name scopes. Ambiguous, missing or unindexed external targets
are not guessed. Source navigation preserves canvas selection and source history, and a menu
retained across a source, selection or definition-scope change cannot act on the replacement
document. Pending or invalid source text must be resolved before source navigation.

`PropertyInspectorView`, `PropertyInspectorSort` and `PropertyInspectorFilter` are public host
preferences. Use `DesignerWorkspace::property_inspector_view`,
`set_property_inspector_view` and `clear_property_filters` to control the view. Hosts can call
`reveal_property_source(property)` and `go_to_property_definition(property)` directly.
Filtering and arrangement are available in read-only sessions and never author XAML or apply
pending source drafts.

These workflows follow Visual Studio's
[category/name/source arrangements and property markers](https://learn.microsoft.com/en-us/visualstudio/xaml-tools/creating-a-ui-by-using-xaml-designer-in-visual-studio?view=visualstudio)
and Qt Designer's
[modified-property editing and reset conventions](https://doc.qt.io/qt-6/designer-widget-mode.html#the-property-editor).

## Interaction model

The [binding and inspector workflow guide](authoring-workflows.md) describes the structured
binding editor, discoverable view/source filters, property-source navigation and semantic
element-name editing added to this pipeline.

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

Inherited floating-point sentinels such as an unset canvas edge or an automatic size stay
empty in the numeric editor. They do not become an editable `NaN` or infinity draft simply
because the property is inspected. Explicitly entering **Auto** for a native floating-point
property validates the property and writes the loader's `NaN` spelling in the same transaction.
This also applies to direct session/MCP property edits and supported native control-theme
setters. Grid lengths retain `Auto`, text retains its exact text, and unresolved application
types retain their existing serialization. Existing source is never normalized on inspection.

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

### Structured bindings

The Binding page provides explicit DataContext, named-element, Self, TemplatedParent,
ancestor and source-value modes. Ancestor editing includes the qualified XAML type,
one-based level and native visual/logical tree choice. Mode, update-source trigger and
delay use the values supported by FerroUI's binding engine. The expanded conversion
section edits Converter, ConverterParameter, ConverterCulture, StringFormat,
FallbackValue and TargetNullValue. Each optional value distinguishes **Unset**, **Text**
and **Expression**, so removing an argument, supplying an empty string and using
`{x:Null}` are separate operations.

Fields are a local draft. **Apply value** validates the complete draft and sends one
guarded expression commit; **Update binding expression** previews the generated source
inside the editor. If the expression and its previously loaded fields are both edited,
the editor requires **Read expression** before rebuilding. This prevents stale fields
from overwriting a newer expression. Unsupported/custom extensions remain available
through exact source editing; explicitly building a new framework binding replaces them.

Untouched arguments keep their original whitespace, quotes, aliases and nested extension
source. An unchanged binding remains byte-identical. Structured source changes remove
the other mutually exclusive source arguments together. Malformed fields, negative
delays, ancestor levels below one and conflicting source arguments report errors before
the host callback runs. Framework names are resolved in the actual XAML namespace scope,
including registered `using:`/`clr-namespace:` aliases and `Extension` suffixes.
RelativeSource is resolved independently, because an alias for a binding object need not
expose its markup extension. If no usable namespace is declared, the editor keeps the
existing expression and requests a namespace declaration before generating that source.

`PropertyBindingDraft` is a reusable source-only draft with semantic setters;
`PropertyBindingEditor` is the native control used by property, definition and nested
object editors. Unscoped constructors preserve expressions but require an explicitly
resolved RelativeSource identity before generating a relative source. Workspace metadata
supplies that identity automatically. Standalone hosts use
`with_relative_source_extension` or `PropertyEditorMetadata::binding_relative_source`,
rather than assume that two extension types share a prefix. Neither API instantiates an
application object or writes to a document.

Native validation covers Self, ancestor and named-element bindings, including qualified
ancestor type conversion, one-step undo and rejection of a retained property popup after
undo. The rendering fixture captures the actual property workbench and resulting preview.

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
