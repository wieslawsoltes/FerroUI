//! Markup metadata of the contracts of this crate that are types of
//! members markup can set: each is published under the name of the
//! interface of the managed original, with the handle types the members of
//! this port use for it.
//!
//! A contract whose handle does not compare (`PartialEq`) cannot be held in
//! an untyped value yet; it is declared so that the type is known by name.

use crate::chrome::IWindowDrawnDecorationsTemplate;
use crate::primitives::IPopupHost;
use crate::selection::ISelectionModel;
use crate::templates::{
    IControlTemplate, IDataTemplate, IRecyclingDataTemplate, ITemplateOf, ITreeDataTemplate, ITypedDataTemplate,
};
use crate::documents::Inline;
use crate::{Control, Panel};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_type;
use ferroui_base::metadata::{IAddChild, MarkupType, MarkupTyped};
use ferroui_base::styling::ITemplate;
use ferroui_base::Ref;
use std::rc::Rc;

// FerroUI.Metadata

// The instantiations of `IAddChild<T>` for the children of this crate. The
// contract belongs to the base crate, so each is declared on a marker.

/// Carries the metadata of `IAddChild<T>` for a child type of this crate.
#[doc(hidden)]
pub struct AddChildOf<T>(std::marker::PhantomData<T>);

ferro_markup_type!(interface AddChildOf<Ref<Control>> as "IAddChild`1" {
    namespace: "FerroUI.Metadata",
    handles: [Rc<dyn IAddChild<Ref<Control>>>, Option<Rc<dyn IAddChild<Ref<Control>>>>],
    this: Rc<dyn IAddChild<Ref<Control>>>,
    generic: "IAddChild`1" [Ref<Control>],
    methods: [
        fn AddChild(Ref<Control>) => |this: &Rc<dyn IAddChild<Ref<Control>>>, child: Ref<Control>| this.add_child(child),
    ],
});

ferro_markup_type!(interface AddChildOf<Ref<Inline>> as "IAddChild`1" {
    namespace: "FerroUI.Metadata",
    handles: [Rc<dyn IAddChild<Ref<Inline>>>, Option<Rc<dyn IAddChild<Ref<Inline>>>>],
    this: Rc<dyn IAddChild<Ref<Inline>>>,
    generic: "IAddChild`1" [Ref<Inline>],
    methods: [
        fn AddChild(Ref<Inline>) => |this: &Rc<dyn IAddChild<Ref<Inline>>>, child: Ref<Inline>| this.add_child(child),
    ],
});

// FerroUI.Controls.Templates

ferro_markup_type!(interface dyn IDataTemplate as "IDataTemplate" {
    namespace: "FerroUI.Controls.Templates",
    handles: [Rc<dyn IDataTemplate>, Option<Rc<dyn IDataTemplate>>],
});

ferro_markup_type!(interface dyn ITreeDataTemplate as "ITreeDataTemplate" {
    namespace: "FerroUI.Controls.Templates",
    handles: [Rc<dyn ITreeDataTemplate>, Option<Rc<dyn ITreeDataTemplate>>],
    interfaces: [Rc<dyn IDataTemplate>],
});

ferro_markup_type!(interface dyn IRecyclingDataTemplate as "IRecyclingDataTemplate" {
    namespace: "FerroUI.Controls.Templates",
    handles: [Rc<dyn IRecyclingDataTemplate>, Option<Rc<dyn IRecyclingDataTemplate>>],
    interfaces: [Rc<dyn IDataTemplate>],
});

// `DataType` (with `[DataType]`) is left out: the port identifies the type by a
// `TypeId`, and a type-typed member is published as a runtime type only.
ferro_markup_type!(interface dyn ITypedDataTemplate as "ITypedDataTemplate" {
    namespace: "FerroUI.Controls.Templates",
    handles: [Rc<dyn ITypedDataTemplate>, Option<Rc<dyn ITypedDataTemplate>>],
    interfaces: [Rc<dyn IDataTemplate>],
});

ferro_markup_type!(interface dyn IControlTemplate as "IControlTemplate" {
    namespace: "FerroUI.Controls.Templates",
    handles: [Rc<dyn IControlTemplate>, Option<Rc<dyn IControlTemplate>>],
    attributes: [ControlTemplateScope],
});

// The instantiations of `ITemplate<TControl>` that are types of properties.

ferro_markup_type!(interface dyn ITemplateOf<Option<Ref<Control>>> as "ITemplate`1" {
    namespace: "FerroUI.Controls",
    handles: [Rc<dyn ITemplateOf<Option<Ref<Control>>>>, Option<Rc<dyn ITemplateOf<Option<Ref<Control>>>>>],
    interfaces: [Rc<dyn ITemplate>],
    generic: "ITemplate`1" [Option<Ref<Control>>],
});

ferro_markup_type!(interface dyn ITemplateOf<Ref<Control>> as "ITemplate`1" {
    namespace: "FerroUI.Controls",
    handles: [Rc<dyn ITemplateOf<Ref<Control>>>, Option<Rc<dyn ITemplateOf<Ref<Control>>>>],
    interfaces: [Rc<dyn ITemplate>],
    generic: "ITemplate`1" [Ref<Control>],
});

ferro_markup_type!(interface dyn ITemplateOf<Option<Ref<Panel>>> as "ITemplate`1" {
    namespace: "FerroUI.Controls",
    handles: [Rc<dyn ITemplateOf<Option<Ref<Panel>>>>, Option<Rc<dyn ITemplateOf<Option<Ref<Panel>>>>>],
    interfaces: [Rc<dyn ITemplate>],
    generic: "ITemplate`1" [Option<Ref<Panel>>],
});

// FerroUI.Controls.Chrome

ferro_markup_type!(interface dyn IWindowDrawnDecorationsTemplate as "IWindowDrawnDecorationsTemplate" {
    namespace: "FerroUI.Controls.Chrome",
    handles: [Rc<dyn IWindowDrawnDecorationsTemplate>, Option<Rc<dyn IWindowDrawnDecorationsTemplate>>],
    attributes: [ControlTemplateScope],
});

// FerroUI.Controls.Selection

ferro_markup_type!(interface dyn ISelectionModel as "ISelectionModel" {
    namespace: "FerroUI.Controls.Selection",
    handles: [Rc<dyn ISelectionModel>, Option<Rc<dyn ISelectionModel>>],
});

// FerroUI.Controls.Primitives

ferro_markup_type!(interface dyn IPopupHost as "IPopupHost" {
    namespace: "FerroUI.Controls.Primitives",
    handles: [Rc<dyn IPopupHost>, Option<Rc<dyn IPopupHost>>],
});

// System.Collections

// The collection handle of the items controls is the enumerable of the managed original
// (`IEnumerable? ItemsSource`): a collection created in markup is assignable to a property
// that holds the handle.
// Deviation (DEVIATIONS.md, Run-time type system of markup): upstream assigns any enumerable;
// here a collection is cast to the handle by the cast registered for its type.
ferro_markup_type!(interface crate::ItemsSource as "IEnumerable" {
    namespace: "System.Collections",
    handles: [crate::ItemsSource, Option<crate::ItemsSource>],
});

/// The types declared in this file.
// FerroUI.Controls.Notifications: the data type of the notification templates of the themes.
ferro_markup_type!(interface dyn crate::notifications::INotification as "INotification" {
    namespace: "FerroUI.Controls.Notifications",
    handles: [
        Rc<dyn crate::notifications::INotification>,
        Option<Rc<dyn crate::notifications::INotification>>
    ],
    this: Rc<dyn crate::notifications::INotification>,
    properties: [
        Title: Option<String> { get: |n: &Rc<dyn crate::notifications::INotification>| n.title() },
        Message: Option<String> { get: |n: &Rc<dyn crate::notifications::INotification>| n.message() },
        Type: crate::notifications::NotificationType { get: |n: &Rc<dyn crate::notifications::INotification>| n.type_() },
    ],
});

pub(super) const TYPES: &[&MarkupType] = &[
    <dyn crate::notifications::INotification as MarkupTyped>::MARKUP,
    <AddChildOf<Ref<Control>> as MarkupTyped>::MARKUP,
    <AddChildOf<Ref<Inline>> as MarkupTyped>::MARKUP,
    <dyn IDataTemplate as MarkupTyped>::MARKUP,
    <dyn ITreeDataTemplate as MarkupTyped>::MARKUP,
    <dyn IRecyclingDataTemplate as MarkupTyped>::MARKUP,
    <dyn ITypedDataTemplate as MarkupTyped>::MARKUP,
    <dyn IControlTemplate as MarkupTyped>::MARKUP,
    <dyn ITemplateOf<Option<Ref<Control>>> as MarkupTyped>::MARKUP,
    <dyn ITemplateOf<Ref<Control>> as MarkupTyped>::MARKUP,
    <dyn ITemplateOf<Option<Ref<Panel>>> as MarkupTyped>::MARKUP,
    <dyn IWindowDrawnDecorationsTemplate as MarkupTyped>::MARKUP,
    <dyn ISelectionModel as MarkupTyped>::MARKUP,
    <dyn IPopupHost as MarkupTyped>::MARKUP,
    <crate::ItemsSource as MarkupTyped>::MARKUP,
];

/// Registers the nullable forms of the contract handles that can be held in
/// untyped values with the untyped value conversions of the current thread.
pub(super) fn register_value_types() {
    ValueTypes::register_nullable::<Rc<dyn crate::notifications::INotification>>();
    ValueTypes::register_nullable::<Rc<dyn IAddChild<Ref<Control>>>>();
    ValueTypes::register_nullable::<Rc<dyn IAddChild<Ref<Inline>>>>();
    ValueTypes::register_nullable::<Rc<dyn IDataTemplate>>();
    ValueTypes::register_nullable::<Rc<dyn IControlTemplate>>();
    ValueTypes::register_nullable::<Rc<dyn ITemplateOf<Option<Ref<Control>>>>>();
    ValueTypes::register_nullable::<Rc<dyn ITemplateOf<Ref<Control>>>>();
    ValueTypes::register_nullable::<Rc<dyn ITemplateOf<Option<Ref<Panel>>>>>();
    ValueTypes::register_nullable::<Rc<dyn IWindowDrawnDecorationsTemplate>>();
    ValueTypes::register_nullable::<Rc<dyn ISelectionModel>>();
    ValueTypes::register_nullable::<Rc<dyn IPopupHost>>();
    // The data template contracts derive from the data template.
    ValueTypes::register_nullable::<Rc<dyn ITreeDataTemplate>>();
    ValueTypes::register_nullable::<Rc<dyn IRecyclingDataTemplate>>();
    ValueTypes::register_nullable::<Rc<dyn ITypedDataTemplate>>();
    ValueTypes::register_cast::<Rc<dyn ITreeDataTemplate>, Rc<dyn IDataTemplate>>(|t| t.clone());
    ValueTypes::register_cast::<Rc<dyn IRecyclingDataTemplate>, Rc<dyn IDataTemplate>>(|t| t.clone());
    ValueTypes::register_cast::<Rc<dyn ITypedDataTemplate>, Rc<dyn IDataTemplate>>(|t| t.clone());
    // A typed template is a template.
    ValueTypes::register_cast::<Rc<dyn ITemplateOf<Option<Ref<Control>>>>, Rc<dyn ITemplate>>(|t| t.clone());
    ValueTypes::register_cast::<Rc<dyn ITemplateOf<Ref<Control>>>, Rc<dyn ITemplate>>(|t| t.clone());
    ValueTypes::register_cast::<Rc<dyn ITemplateOf<Option<Ref<Panel>>>>, Rc<dyn ITemplate>>(|t| t.clone());
}
