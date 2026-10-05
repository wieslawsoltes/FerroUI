# FerroUI porting guide

How C# from the upstream project (Avalonia, MIT) maps to Rust in this repository. Every port follows these rules so that code written by different people fits together and can be traced back file by file.

## Ground rules

1. **Exact port.** One upstream file → one Rust file, same members, same semantics, same edge cases. Port the matching upstream unit tests with the code (`#[cfg(test)] mod tests` at the bottom of the file, or `<file>_tests.rs` next to it).
2. **No upstream name in code.** "Avalonia" (any casing) and "Avn" must not appear in identifiers, file names, comments, doc comments or strings. `Avalonia` → `Ferro` (`AvaloniaObject` → `FerroObject`, `AvaloniaProperty` → `FerroProperty`, `AvaloniaList` → `FerroList`), `Avn` → `Frn`. The upstream name may appear only under `docs/`, in `NOTICE.md` files and in `scripts/`. Finish every task with `grep -rni avalonia <your files>` returning nothing.
3. **Attribution.** Code copied or derived from third-party sources gets a `NOTICE.md` next to it with the upstream copyright line and license text.
4. **Idiomatic, safe, fast.** No `unsafe` outside the class model (`type_system.rs`, the root vtable in `ferro_object.rs`), one audited hot-path downcast in `property_store/effective_value.rs`, and FFI crates. No panics on user input (parsing returns `Result`); panics are for programmer errors where C# throws `InvalidOperationException`/`ArgumentException`. No allocations on hot paths that upstream avoids. No blocking waits, no threads assumed, no `std::time::Instant` or `std::fs` in core paths (the browser target has neither).
5. **Things not ported yet** are left out, not stubbed with `todo!()`. Record every skipped member in the report/tracking data.

## Layout

| Upstream | FerroUI |
|---|---|
| `src/Avalonia.Base/Media/SolidColorBrush.cs` | `src/FerroUI.Base/media/solid_color_brush.rs` |
| namespace `Avalonia.Media` | module `ferroui_base::media` (re-exported so `ferroui::media::SolidColorBrush` works) |
| root namespace `Avalonia` types | private module + `pub use` at crate root (`ferroui_base::Point`) |
| project `Avalonia.Controls` | crate `ferroui-controls` in `src/FerroUI.Controls/` (`[lib] path = "lib.rs"`, sources directly in the crate dir) |
| partial classes / tiny related types | may share one file; note it in the report |

Module files use `snake_case`. A module's `mod.rs` only declares and re-exports.

## Types

| C# | Rust |
|---|---|
| `class` deriving (transitively) from `AvaloniaObject` | `#[repr(C)] pub struct X { base: Base, .. }` + `ferro_class!` (see below) |
| other `class` (reference semantics needed) | plain struct behind `Rc<X>`; interior mutability with `Cell`/`RefCell` |
| `struct` / `record struct` | `#[derive(Clone, Copy, Debug, PartialEq)] struct` with public snake_case fields for plain data, methods for computed properties |
| `interface IFoo` | `pub trait IFoo` — the `I` prefix is kept for 1:1 traceability and because upstream often has both `IBrush` and `Brush`. Handles are `Rc<dyn IFoo>` |
| `enum` | `enum` with `#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]`, same variant names and numeric values (`#[repr(i32)]` when values matter) |
| `[Flags] enum` | `bitflags!` with `SCREAMING_CASE` members, including mask and flag members (`BindingValueType::HAS_VALUE`) |
| `T?` nullable reference | `Option<Ref<T>>` / `Option<Rc<dyn I>>` |
| `Nullable<T>` / `Optional<T>` | `Option<T>` |
| `object` | `BoxedValue` (`Rc<dyn Any>` holding exactly the property's value type) |
| `Type` | `&'static TypeInfo` (`Border::TYPE`, `obj.get_type()`) |
| `IDisposable` | `Rc<dyn IDisposable>` with explicit `dispose()` (not `Drop`: ignoring the handle must keep the subscription alive, as in C#) |
| `IObservable<T>` / `IObserver<T>` | `reactive::{IObservable, IObserver}`; `Rc<dyn IObservable<T>>` |
| `event EventHandler<T> Foo` | `HandlerList<dyn Fn(&T)>` field + `pub fn foo(&self, handler) -> Rc<dyn IDisposable>` |
| `Action` / `Func<..>` | `Rc<dyn Fn(..)>` (stored) or `impl Fn(..)` (parameter) |
| `string` | `String` / `&str`; `double` → `f64`; `float` → `f32`; `int` → `i32` |
| `IList<T>`, `List<T>` | `Vec<T>` for values and snapshots; observable collections, and any list a get-only property exposes by reference → `FerroList<T>` (a shared handle, see "Collections are handles") |
| exceptions on parse/validation paths | `Result<_, Error>`; programmer errors → `panic!` |
| `static` class with attached properties | unit struct + `ferro_static_type!(X);` |
| `ToString()` | `Display` (invariant culture) ; `Parse(string)` | `FromStr` + `parse(&str) -> Result` |

## Naming

- Methods and properties: `snake_case` of the C# name. Property `Foo { get; set; }` → `foo()` / `set_foo(v)`. `IsEnabled` → `is_enabled()`. `GetValue`/`SetValue` keep their names (`get_value`).
- Overloads get suffixes describing the difference (`contains` / `contains_rect`, `bind` / `bind_untyped`).
- Property definitions: C# `public static readonly StyledProperty<IBrush?> BackgroundProperty` → `Border::background_property()`.
- Routed events: `Button.ClickEvent` → `Button::click_event()`.
- Names visible to XAML stay PascalCase strings: property name `"Background"`, type name `Border`.

## Classes

```rust
#[repr(C)]
pub struct Decorator {
    base: Control,                       // always first, always named `base`, private
    child: RefCell<Option<Ref<Control>>>,
}

ferro_class!(Decorator: Control);        // no new virtual members
ferro_impl_classes!(Decorator: FerroObjectImpl, StyledElementImpl, VisualImpl, /* every level below */);

impl LayoutableImpl for Decorator {      // override: C# `protected override Size MeasureOverride(Size s)`
    fn measure_override(this: &Self, available_size: Size) -> Size {
        // `this` is C#'s `this`. Call the base implementation with
        // `Self::parent_measure_override(this, available_size)` (C# `base.MeasureOverride(s)`).
    }
}

ferro_class_info!(Decorator { new: Decorator::new });   // default constructor (and interface handles)

ferro_properties! {                      // every registered property of the class, in upstream order
    impl Decorator {
        pub fn child_property() -> StyledProperty<Option<Ref<Control>>> {
            FerroProperty::register::<Decorator, _>("Child", None)
        }
    }
}

impl Decorator {
    /// C# `static Decorator() { .. }`. Found by `ferro_class!`; never called by hand.
    fn static_constructor() {
        Self::affects_measure::<Decorator>(&[Self::child_property().as_property()]);
    }

    /// Field initialisation only (C# field initialisers). No `this` available.
    pub fn construct() -> Self { Self { base: Control::construct(), child: RefCell::new(None) } }

    /// C# `new Decorator()`.
    pub fn new() -> Ref<Self> { instantiate(Self::construct()) }

    pub fn child(&self) -> Option<Ref<Control>> { self.get_value(Self::child_property()) }
    pub fn set_child(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::child_property(), value.into().0)
    }
}
```

- **Constructor bodies** that need `this` (subscribing, creating children, setting properties) go in the `constructed` override: `impl FerroObjectImpl for X { fn constructed(this: &Self) { Self::parent_constructed(this); /* body */ } }`. Base first, as in C#.
- **Class initialisation.** A class is initialised once per thread, base classes first, by whichever comes first: its first instance (`instantiate`), the first call of one of its property accessors, or a lookup into it (`FerroPropertyRegistry::find_registered(type, name)`, `TypeInfo::properties()`). The initialisation, generated by `ferro_class!`, (1) makes the type known by name, (2) registers the handle type `Ref<X>` and the declared interface handles with the untyped value conversions (`ValueTypes`), (3) registers the properties of the `ferro_properties!` block in declaration order, (4) runs `static_constructor()`. Afterwards it costs one thread-local flag read per `instantiate` and nothing per accessor call. `X::TYPE.ensure_class_init()` forces it.
- **Static constructors** (`static X() { AffectsRender<X>(..); FooProperty.Changed.AddClassHandler<X>(..); }`) go in an inherent `fn static_constructor()` of the class, in the module of its `ferro_class!`. No flag, no call: the class initialisation runs it exactly once per thread, after the properties are registered. A static class that owns attached properties (`KeyboardNavigation`) is a unit struct with `ferro_static_type!(X);` and takes the same `ferro_properties!` block and `static_constructor()`.
- **Class facts for untyped code**: `ferro_class_info!(X { new: X::new, interfaces: [Rc<dyn IFoo>] });` next to `ferro_class!`. `new` (omit it for abstract classes and classes without a parameterless constructor) becomes `X::TYPE.default_constructor()`; `interfaces` lists the handle types `Ref<X>` converts to with `.into()` (or, written `Rc<dyn IFoo> => X::as_foo`, with the given `fn(Ref<X>) -> Rc<dyn IFoo>`: needed for a contract of another crate, for which `From` cannot be implemented), which untyped bindings and markup then accept wherever a property is typed with the interface (inherited by deriving classes).
- **Type table**: a crate has a `register_types.rs` with `pub fn register_types()` (see `src/FerroUI.Base/register_types.rs`) listing the dotted namespace of each module (`("ferroui_controls::primitives", "FerroUI.Controls.Primitives")`, declared once per module, not per class) and every class of the crate. It feeds `TypeInfo::find(namespace, name)` for types that were never used. Add new classes to it (in `ferroui-base` a test compares the list with the sources).
- **New virtual members**: declare them once in `ferro_class! { X: Base, virtuals XImpl: BaseImpl { fn name(this, arg: T) -> R; } }`. The declaring class implements every one (`impl XImpl for X`). `x.name(arg)` performs the virtual call. Inside any class code, always call virtuals through the method (`this.measure_override(s)`), never through the trait function, unless you intend a non-virtual call.
- **Handles**: `Ref<T>` is the object reference; `Ref<Border>` upcasts to `Ref<Control>` with `.upcast()` (free), downcasts with `.cast::<Border>()` (`as`) / `.is::<Border>()` (`is`). From inside a method, `self.to_ref()` gives the handle. Parent/back pointers are `WeakRef<T>`; children are strong.
- Parameters that accept "any control": `impl IntoRef<Control>`; nullable: `impl Into<Nullable<Control>>`.
- Fields: `Cell<T>` for `Copy` values, `RefCell<T>` otherwise. Never hold a `RefCell` borrow across a call that can raise events, call virtuals or run user code — copy/clone out first.

## Properties

```rust
ferro_properties! {
    impl TextElement {
        // StyledProperty with options (inherits, coerce, validate, default binding mode, assign binding)
        pub fn font_size_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<TextElement, _>("FontSize",
                StyledPropertyOptions::new(12.0).inherits(true))
        }
        // Attached:  FerroProperty::register_attached::<Owner, Host, T>("Name", default)
        // Direct:    FerroProperty::register_direct::<Owner, T>("Name", getter, Some(setter), unset_value)
        // AddOwner:  TextElement::font_size_property().add_owner::<TextBlock>()
    }
}
```

- All registered properties of a type (styled, direct, attached, added owners) are declared in its one `ferro_properties!` block, in the order of the upstream static fields. The block generates the same accessors as `ferro_property!` and registers all of them when the class is initialised, so a property is found by name (`find_registered`, string-path bindings, markup) whether or not its accessor ever ran and whether or not an instance exists. A lone `ferro_property!` outside a block still works but registers only when first called: do not use it for new code.
- Properties declared in a second place (an upstream partial class in another file, or accessors separated by other items) use `ferro_properties! { impl X, fn register_more_properties { .. } }`, and the main block names that function: `impl X, also [X::register_more_properties] { .. }`.
- `[AssignBinding]` on the upstream property → `StyledPropertyOptions::new(..).assign_binding(true)` (or `property.set_assign_binding(true)` for a direct property); read with `FerroProperty::assign_binding()`.
- Value types must be `Clone + PartialEq + 'static`.
- `GetValue(P)` → `self.get_value(Self::p_property())`; `SetValue` → `set_value`; `SetCurrentValue` → `set_current_value`; `ClearValue` → `clear_value`; direct properties use `set_and_raise(property, &self.field, value)`.
- `OnPropertyChanged(change)` override → `impl FerroObjectImpl for X { fn on_property_changed(this, change) { Self::parent_on_property_changed(this, change); if change.property() == Self::foo_property().as_property() { let (old, new) = change.get_old_and_new_value::<T>(); .. } } }`.
- `FooProperty.Changed.AddClassHandler<X>((x, e) => ..)` → `Self::foo_property().changed().add_class_handler::<X>(|x, e| ..)`.

### Migrating a class to `ferro_properties!` / `static_constructor()`

Classes written before these forms existed register each property on the first call of its accessor and carry a hand-written, flag-guarded `class_init()`. They keep compiling and behaving as before; convert them with three local edits (nothing else in the file moves):

1. Wrap the run of `ferro_property!(..);` invocations in `ferro_properties! { impl X { .. } }`. The invocations stay as they are; close the block after the last one and reopen `impl X {` for the rest. Accessors that are not adjacent go in a `, fn name` part (see above).
2. Rename `fn class_init()` to `fn static_constructor()`, delete its thread-local flag and every `Self::class_init()` call (in `constructed`, in constructors, in accessors). Delete accessor calls that were only there to force registration. Where another type's handlers must be registered first, call `Other::TYPE.ensure_class_init()`.
3. Add `ferro_class_info!` (default constructor, interfaces), replace a hand-written `impl StaticType` with `ferro_static_type!(X);`, and list the class in the crate's `register_types.rs`.

Before:

```rust
impl FerroObjectImpl for Geometry {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        Self::class_init();
    }
}

impl Geometry {
    ferro_property!(pub fn transform_property() -> StyledProperty<Option<Ref<Transform>>> {
        FerroProperty::register::<Geometry, _>("Transform", None)
    });

    fn class_init() {
        thread_local! {
            static DONE: Cell<bool> = const { Cell::new(false) };
        }
        if DONE.replace(true) {
            return;
        }

        Self::transform_property().changed().add_class_handler::<Geometry>(|x, e| x.transform_property_changed(e));
    }

    pub fn construct() -> Self { .. }
}
```

After:

```rust
impl FerroObjectImpl for Geometry {}

ferro_properties! { impl Geometry {
    ferro_property!(pub fn transform_property() -> StyledProperty<Option<Ref<Transform>>> {
        FerroProperty::register::<Geometry, _>("Transform", None)
    });
} }

impl Geometry {
    fn static_constructor() {
        Self::transform_property().changed().add_class_handler::<Geometry>(|x, e| x.transform_property_changed(e));
    }

    pub fn construct() -> Self { .. }
}
```

Things to check after converting: a test that asserted a property is *not* registered before the first instance no longer holds; `static_constructor` must be declared in (or be visible from) the module of `ferro_class!`, with exactly that name: a misspelt one is never called and shows up as a dead-code warning; property IDs follow declaration order, so code must not depend on the old first-use order.

## Markup metadata

Markup (and other untyped code) asks questions the managed original answers with reflection. A type answers them once, as constant data (`ferroui_base::metadata::MarkupType`); nothing runs before `main` and nothing costs anything until the data is read. Registered properties (styled, direct, attached) need **no** declaration: they are found through `TypeInfo::properties()`. Declare only what the property system does not know.

**Classes** state it in the `markup` part of `ferro_class_info!` (parts in any order, each optional):

```rust
ferro_class_info!(Panel {
    new: Panel::new,                                   // C# parameterless constructor
    interfaces: [Rc<dyn IAddChild<Ref<Control>>>],     // handles `Ref<Panel>` converts to with `.into()`
    markup: {
        content: Children,                             // [Content] on the property
        constructors: [(String) => Panel::with_name],  // constructors with arguments
        properties: [                                  // plain CLR properties (not registered ones)
            Children: Controls { get: Panel::children },                        // read-only: a collection handle
            Tag2: Option<BoxedValue> { get: Panel::tag2, set: Panel::set_tag2 } // attributes follow in [..]
                [DependsOn("Other"), AssignBinding],
        ],
        methods: [fn Add(Ref<Control>) => Panel::add_child, static fn Parse(String) -> Foo => Foo::parse_text],
        fields: [ClickEvent: RoutedEvent<RoutedEventArgs> => Button::click_event],  // static values / x:Static
        events: [Closed(EventArgs) => |this: &Ref<Window>, handler: MarkupDelegate| ..],   // plain (non-routed) events
        attributes: [UsableDuringInitialization, TemplatePart("PART_Bar", type(Ref<Border>), IsRequired = true)],
    },
});
```

- A callable is a path or closure. Instance members are called with `(&this, args..)` where `this` is the handle (`Ref<Class>`), so `Class::method` paths work as they are; a member that takes `&str`, `impl Into<..>` or returns a borrowed value is wrapped in a closure that takes/returns the declared type.
- Declared types are the exact Rust types the callable takes and returns; the invoker converts untyped arguments to them with the assignability casts of `ValueTypes` (base/interface handles, `Option<T>`, "any value") and reports anything else as `MarkupInvokeError`.
- Attributes are named as upstream without the `Attribute` suffix (`ferroui_base::metadata::attributes` lists the ones the framework reads); arguments are literals, `null`, `type(T)` and `Name = value` for named arguments. Upstream attribute → where it goes: `[Content]` → `content:`; `[TemplateContent]`, `[DependsOn]`, `[AssignBinding]` (plain properties; registered ones use `StyledPropertyOptions::assign_binding`), `[DataType]`, `[InheritDataTypeFrom..]`, `[ResolveByName]`, `[MarkupExtensionOption]` → the property's attribute list; `[UsableDuringInitialization]`, `[TrimSurroundingWhitespace]`, `[WhitespaceSignificantCollection]`, `[ControlTemplateScope]`, `[TemplatePart]`, `[PseudoClasses]`, `[TypeConverter]` → `attributes:` of the type.
- `ISupportInitialize`, `IAddChild<T>`, `INameScope` and the other contracts markup tests for are ordinary `interfaces:` entries.

**Other types** use `ferro_markup_type!` (value types, plain `Rc` classes, contracts, types of other crates) and `ferro_markup_enum!`; both implement `MarkupTyped`:

```rust
ferro_markup_type!(struct Thickness { handles: [Thickness], parse: Thickness::parse });
ferro_markup_type!(class Setter {
    handles: [Setter, Rc<Setter>, Option<Rc<Setter>>],   // the Rust types holding a value, untyped form first
    this: Rc<Setter>,                                    // what instance members receive
    interfaces: [Rc<dyn ISetter>],
    constructors: [() => Setter::empty],
    content: Value,
    properties: [ .. ],
});
ferro_markup_type!(interface dyn IBrush as "IBrush" { handles: [Rc<dyn IBrush>, Option<Rc<dyn IBrush>>] });
ferro_markup_enum!(Dock { Left, Bottom, Right, Top });
ferro_markup_enum!(flags RoutingStrategies { Direct = RoutingStrategies::DIRECT, Tunnel = RoutingStrategies::TUNNEL });
```

`namespace: "System"` overrides the namespace of the declaring module; `generic: "FerroList`1" [T]` names the generic definition an instantiation belongs to; `base:` names a base type by handle.

More parts of the declaration forms:

- **Attributes of registered properties.** Registered properties are not declared, so what the property system does not know about them goes in `property_attributes: [Content: [DependsOn("ContentTemplate")], Target: [ResolveByName]]` of the owner's `markup` (or of the `static` type that owns an attached property).
- **Static types** (`ferro_static_type!`, owners of attached properties) link their metadata to the runtime type: `ferro_markup_type!(static ToolTip2 { type_info: ToolTip2, property_attributes: [..], methods: [..] });` and are registered like other `MarkupTyped` types; `MarkupType::find_by_type_info(type)` finds it.
- **Renamed enumeration members**: `ferro_markup_enum!(RelativeSourceMode { DataContext, TemplatedParent, Self = Self_, FindAncestor });` (markup name `=` Rust variant).
- **Events.** `events: [Name(A, B) => callable]` lists the handler parameters exactly as upstream declares them; the handler (`MarkupDelegate`) is invoked with that many untyped values. An `EventHandler<TArgs>` event is `Name(Option<BoxedValue>, TArgs)`: where the Rust event passes no sender, the adapter closure passes the instance. An event without arguments is `Name()`. Event argument types must be boxable (`PartialEq + 'static`, shared handle for classes).
- **Text conversion.** `parse:` is the one way a type states its conversion from text, whether upstream has `Parse(string)` or a `[TypeConverter]` class that only parses (`BrushConverter`, `GeometryTypeConverter`, ...): the compiler finds it at the `Parse` step of the conversion order, with the same result. A `[TypeConverter(type(..))]` attribute is declared only when the converter is a real ported class with behaviour beyond parsing (the converters of `ferroui-markup-xaml`, which need the service provider).
- **Fallible members.** A callable that returns `Result<T, E>` is declared with `try`: `try fn Name(A) -> T => callable`, `static try fn`, `try (A) => callable` (constructor), `try_get:` / `try_set:`. `Err` becomes `MarkupInvokeError::Failed`; never `unwrap` inside a metadata callable.
- **Indexers and method attributes.** `indexers: [(i32) -> T { get: callable, set: callable }, (String) -> T { try_get: callable }]` declares `this[..]` (`MarkupType::indexers`; binding paths use them for `Path[0]` / `Path[key]`). A method takes an attribute list after its callable: `fn CanSave(Option<BoxedValue>) -> bool => Vm::can_save [DependsOn("Name")]`; the callable is then a plain path, or any expression in parentheses. Attribute arguments may be floating point literals (`MarkupAttributeValue::Float`).
- **Array arguments of attributes.** `Name(["a", "b"])` and `Key = [1, 2]` state an array (`MarkupAttributeValue::Array`; items are literals, `null`, `type(T)` or arrays): `attributes: [FerroList(Separators = [",", " "])]`.
- **Constructor parameters.** A constructor whose parameters carry attributes in the original names them all: `(property: &'static FerroProperty [InheritDataTypeFrom(2)]) => TemplateBinding::new` (`MarkupConstructor::parameter_info`: names and attributes by index). The positional form `(A, B) => callable` states nothing about the parameters.
- **Static properties and static fields.** `fields: [Name: T => callable]` are the `static readonly` fields and constants of the original; `static_properties: [Name: T { get: callable, set: callable } [..]]` are its static properties (`static T Name { get; set; }`; accessors without an instance, `try_get:` / `try_set:` as usual). `x:Static` finds both.
- **Typed binding paths.** Every `properties:` entry carries `MarkupProperty::typed_path_element`, generated from its accessors: for a declaration whose `this:` is a shared type `Rc<S>` (`S: PartialEq`; notifying when `S: INotifyPropertyChanged`) it adds the typed element (`CompiledBindingPathBuilder::typed_property` of generated code) to a path built at run time, so a single-property compiled binding of the runtime loader is a `TypedBindingExpression<S, V>` as in the original. Nothing is declared for it; a property with `try_set:` has no typed form.
- **Fallible initialization.** `ISupportInitialize::try_end_init` / `StyledElement::try_end_init` (the overridable member) and `StyledElement::try_apply_styling` report what `EndInit` throws in the original (`InitializationError`, a style with two setters for a property: `DuplicateSetterError`); `end_init` / `apply_styling` panic with the same message. Metadata publishes `try fn EndInit`.
- **View models for string-path bindings.** `{Binding Path}` (reflection bindings) resolves properties, methods and indexers through this metadata, walking `base:` as reflection walks base classes; nothing else needs to be declared. A method binds as a delegate (`MarkupDelegate` with its `method()`/`target()`), which becomes a command (`data::converters::MethodToCommandConverter`, with the `Can<Name>` companion and its `DependsOn` attributes) for a command-typed target. `notify_property_changed: T` gives change notifications; `ValueTypes::register_reference::<T>()` makes the object form (`Rc<T>` as the box itself) known and the source weakly held.
- **Value knowledge.** What `ValueTypes` must know about the metadata types of a crate (nullable forms, reference types, casts) is registered once with `ValueTypes::register_global(register_fn)` from `register_types()`; it then applies on every thread.
- **Types as values.** A `System.Type`-typed member is declared as `&'static TypeInfo` (classes only) or `ValueType` (any type); see `xaml.md`, decision 16.
- **Where declarations live.** Next to the type, or (to keep class files untouched) in the crate's `markup_types/` module; a class may have its `new:`/`interfaces:` in its own file and its `markup:` part in `markup_types/`. Each part may appear once per class. Generated parts of `markup_types/` come from the scripts under `scripts/` and are checked with their `--check` mode.

**Registration.** The crate's `register_types()` registers, besides its classes: its `MarkupAssembly` (assembly name, crate name, the `XmlnsDefinition` list mirroring upstream `AssemblyInfo.cs`) and the `MarkupTyped` types (`MarkupType::register_all(&[<Thickness as MarkupTyped>::MARKUP, ..])`). Class metadata is reached from `TypeInfo::markup()` and needs no registration. A crate's `register_types()` first calls the ones of the crates it is built on. `ferroui-controls` has the skeleton (`src/FerroUI.Controls/register_types.rs`); add every new class to its list.

**Recipe for a class** (after the `ferro_properties!` migration above): (1) give it `new:` unless abstract; (2) copy the upstream `[Content]` property name into `content:`; (3) list every public CLR property that is not backed by a registered property and that markup can set or add to (collections are read-only properties), with the attributes it carries upstream; (4) list `Add`/factory/handler methods and plain events markup uses; (5) copy the type-level attributes; (6) list the class in `register_types.rs`. Markup-visible names stay PascalCase and identical to upstream.

## Threading model

Everything is UI-thread-affine and `Rc`-based. Property definitions and registries are per-thread. Cross-thread work goes through the dispatcher (`Dispatcher::ui_thread().post(..)`), whose callbacks must be `Send`.

## Reporting (for every porting task)

File inventory (Rust path ← C# path, complete/partial), every skipped or deferred member, behavioural differences from C#, test counts and the `cargo test` summary line, the `grep` result for rule 2, and anything uncertain. Be factual.

## Conventions that emerged during the port

- **A class that implements an interface.** `Ref<T>` cannot itself be an `Rc<dyn ITrait>`. Convert with `.into()` to a small adapter handle (`Rc<dyn IBrush>` from `Ref<SolidColorBrush>`, `Rc<dyn IStyle>` from `Ref<Style>`); compare identity with the dedicated `*_ptr_eq` helpers or the trait's `PartialEq`, not `Rc::ptr_eq`. Do not implement the interface trait on the class struct or on `Ref<T>`: with the trait in scope its methods shadow the class's inherent ones.
- **Collections are handles.** A collection type is a reference object, as upstream: the type itself is a cheap-clone handle (`#[derive(Clone)] struct Controls(..)` around shared state) with identity `PartialEq` and `Deref` to what it wraps. `FerroList<T>` and `FerroDictionary<K, V>` are such handles, so a collection built on one derives both traits. A getter of a collection returns the handle by value (`panel.children()` → `Controls`, `style.setters()` → `FerroList<Rc<dyn SetterBase>>`), never a borrow and never `Rc<Collection>`: code keeps the handle, markup boxes it (`BoxedValue`) and adds children through it, and mutating through one clone is visible through every other. A get-only upstream property of type `List<T>`/`IList<T>` is a `FerroList<T>` handle; a settable collection property is a property of the handle type.
- **Reference types compare by identity.** A type that is a class upstream and does not define `Equals` has identity `PartialEq`, so that it can be a property or untyped value: `impl PartialEq for X { std::ptr::eq(self, other) }` for a plain class used as `Rc<X>` (`Setter`, `ReflectionBinding`, `RelativeSource`), pointer equality of the shared state for a handle type (`Selector`, `CompiledBindingPath`, event args with shared state), and `impl PartialEq for dyn IFoo` for a contract. A contract with adapter handles compares the object behind the adapter (a `reference_id()`/`as_object()` hook on the trait), never the address of the adapter; a contract without adapters compares addresses (`std::ptr::addr_eq`).
- **Plain `Rc` classes with settable properties** (bindings, `RelativeSource`, `Setter`) keep their state in `Cell`/`RefCell` fields and expose each upstream property as `foo()` / `set_foo(v)`, plus a chaining `with_foo(self: Rc<Self>, v) -> Rc<Self>` so that an object can be configured in one expression. `X::new(args)` is the upstream constructor with arguments and returns `Rc<X>`; the parameterless constructor is `X::new()` when there is no other, else `X::empty()`. A class deriving from such a class embeds it (`base`), derefs to it, and repeats the chaining forms. `Rc<X>` converts to the contract handle (`Rc<dyn BindingBase>`) by coercion; `Rc<T>` itself implements `BindingBase`, so `&binding` is accepted wherever a `&dyn BindingBase` is.
- **Event args** are passed to handlers by reference. Args that handlers change (a cancel flag, `Handled`) keep that state shared (`Rc<Cell<..>>`), so that a clone is another handle to the same args; immutable args are `Clone + PartialEq`. Untyped handler wiring clones the args into a `BoxedValue`: `IRoutedEventArgs::share()` for routed events (`Interactive::add_handler_untyped`), `NotifyCollectionChangedEventArgs::share()` for collection changes. Plain events take the args only; routed event handlers take `(sender, args)`.
- **C# `is`/`as` on interfaces** become defaulted hook methods on the nearest contract (`as_window_impl()`, `as_solid_color_brush()`, `as_any()`), or a virtual on the class (`InputElement::as_custom_keyboard_navigation()`).
- **Controls in untyped values.** A control stored in a `BoxedValue` (`Content`, `Header`, `Tag`) is boxed as exactly `Ref<Control>`: use `Control::boxed` / `Control::from_boxed`.
- **Static constructors** go in `fn static_constructor()` (see Classes). Handlers that must exist before the first instance need nothing extra: the first accessor call initialises the class.
- **Callback properties on platform contracts** (`Action`/`Func` properties) are getter/setter pairs of `Option<Rc<dyn Fn(..)>>`; events are `fn name(&self, handler) -> Rc<dyn IDisposable>`.
- **Async members** return `DispatcherTask<T>` or a boxed future driven by the dispatcher (`threading/dispatcher_task.rs`); nothing blocks and there is no async runtime.
- **Tests** that need a root use the crate's `test_support` harness (`TestRoot`, `TestSource`) and `Dispatcher::unit_test_scope()`.
- **Properties that reference an element they do not own.** A property whose upstream type is a reference to another element that the owner of the property does not own (`Popup.PlacementTarget`, `Label.Target`, `AdornerLayer.AdornedElement`, flyout, context-menu and tooltip targets) is stored as `Option<ElementRef<T>>` (`ferroui_base::ElementRef`: a weak reference with identity equality), because such a reference usually points at an ancestor and a strong value would make the two keep each other alive. The property definition calls `ValueTypes::register_element_ref::<T>()`; the getter returns `Option<Ref<T>>` (`ElementRef::resolve`), the setter takes `impl Into<Nullable<T>>` (`ElementRef::from_nullable`). Untyped access and bindings accept and produce `Ref<T>` / `Option<Ref<T>>`. A referenced element that has been dropped reads as `None`, without a change notification. Children, content and anything else the owner does own stay `Ref<T>`.
- **Top-level lifetime.** A top-level is kept alive by the closed callback of its platform implementation until the implementation reports that it has closed (as the delegates of the platform implementation root a window upstream), and releases the implementation and its callbacks in the closed path. A window whose implementation was created but which is never closed stays alive, as upstream; close it (or dispose the embeddable root). Handlers that come and go on the lost-focus, deactivated and position-changed notifications of a platform implementation are added through `TopLevel::platform_lost_focus` / `platform_deactivated` / `platform_position_changed`, never chained onto the callback property.
- **Third-party notices.** Attribution lives in the root `NOTICE.md`. A crate- or module-level `NOTICE.md` is added only when code derives from a third party other than the upstream project (WPF, WebKit, ...) whose licence requires its notice to travel with the code.
