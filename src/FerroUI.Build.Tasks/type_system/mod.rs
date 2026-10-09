//! The build-time type system of the compiler: the type-system contracts of
//! XamlX ([`IXamlTypeSystem`](xamlx::type_system::IXamlTypeSystem),
//! `IXamlType` and its members) over the type models of a set of crates
//! ([`AssemblyModel`](crate::model::AssemblyModel)) and the closed table of
//! runtime library types (docs/porting/xaml.md, 9.5.2 and 9.5.5).
//!
//! It is what the transformers of the compiler resolve the types of a
//! document against when the crates that declare them are not linked: the
//! models are read from the sources ([`crate::scanner`]) or from the
//! `.xamlmeta` of the dependencies. Nothing of a scanned crate is linked, so
//! no member can be invoked: a member carries the declaration it is the
//! projection of ([`MemberSource`]), which the emitter writes its call from.
//!
//! # What it mirrors
//!
//! The run-time type system of the loader (`runtime::type_system`) projects
//! the registries of the process; this one projects the same declarations
//! from their text, by the same rules:
//!
//! | Declaration | Projection |
//! |---|---|
//! | a registered property | a member of the owner its registration names (`register::<Owner, _>`, `add_owner::<Owner>`), whichever type has the accessor: the property `Name` (`get_Name` / `set_Name`) and the static field `NameProperty`; an attached property on the type that registers it: the static accessors `GetName` / `SetName` (`AttachedProperty<T>` field), on a class that adds itself as an owner: an instance property (`StyledProperty<T>` field); a direct property registered without a setter has no `set_Name` |
//! | a second accessor of a property (an alias) | nothing: the property is projected once |
//! | a declared static `GetName` / `SetName` of an attached property | replaces the plain accessor and carries the attributes of the property |
//! | `new:` of a class | the public constructor without parameters; a class without any constructor has one that is not public |
//! | `properties:`, `static_properties:`, `indexers:`, `methods:`, `fields:`, `events:`, `constructors:`, `parse:` | as the run-time type system projects them (`Item` and `[DefaultMember("Item")]` for an indexer, `add_Name` with the handler type for an event, the static `Parse(string)`) |
//! | an enumeration | its members as literal fields (`Int32`, or `Int64` when a value does not fit), `[Flags]` for a set of flags |
//! | `generic: "Name`1" [T]` | the instantiation of the definition `Name`1`, which is synthesised for the declared instantiations |
//! | metadata of a runtime library type (`namespace: "System"`) | the members of that type of the table |
//! | metadata of the name and the module of a type of the object model | part of that type |
//! | a class the list of registered classes of its crate leaves out | a type with its name that is found neither by the name nor by its handle; the base of the classes that derive from it |
//! | `base:` an instantiation of the notifying list | the base, when the crate registers the cast from the collection to the list; else the collection is a type of its own |
//! | the `MarkupAssembly` | the assembly, with its xmlns definitions as attributes |
//!
//! The runtime library types, the lists markup creates and the property
//! definition types are defined from the table both type systems share
//! (`ferroui_markup_xaml_loader::core_table`).
//!
//! # Rust type text to type (9.5.2)
//!
//! [`ModelTypeSystem::resolve`] maps normalised type text: the text is made
//! the one text of its Rust type, canonical through the export tables of the
//! models and with every type alias replaced by the type it stands for
//! ([`ModelSet::expanded`](crate::model_set::ModelSet::expanded)), and then
//! it is, in this order: a handle of the table (`bool`, `Option<bool>`,
//! `String`, `BoxedValue`, `&'static TypeInfo`, `Vec<f64>`, ..); a handle a
//! declaration states (`handles:`), the handle or the optional handle of a
//! registered class of the object model (`Ref<X>`, `Option<Ref<X>>`), an
//! enumeration, a handle a crate registers for the type next to the
//! declaration (`MarkupType::register_handle`); the optional form of a value
//! type or an enumeration (`System.Nullable<T>`); the optional form of a
//! handle of a reference type; and else an opaque type that carries the
//! text, which is assignable only to itself and to `System.Object` and has no
//! members.
//!
//! # What it cannot know
//!
//! What only the build or the process decides is not in the sources: whether
//! a `cfg` condition holds, whether an instance of a class its crate does not
//! register was created (the class is known from then on), the element types
//! of bindable arrays registered at run time, and which of two declarations
//! of one name is looked up first. What the registration functions of a crate
//! state is read: the list of its classes, the handles and the casts. The
//! lists of the types with markup metadata a crate registers are not read:
//! every declared one is taken as registered. The drift test of the XAML test
//! crate (`type_system_drift`) compares the two type systems and fails on a
//! difference that is not one of these.

mod model_type_system;
mod types;

pub use model_type_system::{ModelTypeSystem, Position};
pub use types::{
    MemberSource, ModelAssembly, ModelConstructor, ModelCustomAttribute, ModelEvent, ModelField, ModelMembers, ModelMethod, ModelProperty, ModelType,
    ModelTypeKind, ModelTypeOrigin,
};

#[cfg(test)]
mod tests;
