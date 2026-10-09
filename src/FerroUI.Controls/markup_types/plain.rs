//! Markup metadata of the plain (non object-model) classes and of the
//! collections of this crate that markup can name, construct or add to.
//!
//! A collection is a shared handle: the type itself is the handle.

use crate::documents::{Inline, InlineCollection};
use crate::primitives::popup_positioning::{
    CustomPopupPlacement, PopupAnchor, PopupGravity, PopupPositionerConstraintAdjustment,
};
use crate::primitives::CustomPopupPlacementCallbackValue;
use crate::templates::{DataTemplates, IDataTemplate};
use crate::{NativeMenuItemBase, Page, TableViewColumn};
use ferroui_base::collections::FerroList;
use crate::{
    ColumnDefinition, ColumnDefinitions, Control, Controls, ItemCollection, RowDefinition, RowDefinitions, TrayIcon,
    TrayIcons, WindowIcon,
};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_type;
use ferroui_base::metadata::{MarkupDelegate, MarkupType, MarkupTyped};
use ferroui_base::{BoxedValue, Point, Rect, Ref, Size, Thickness, Visual};
use std::cell::RefCell;
use std::rc::Rc;

/// `FerroList<T>.Capacity`.
fn list_capacity<T: Clone>(list: &FerroList<T>) -> i32 {
    i32::try_from(list.capacity()).unwrap_or(i32::MAX)
}

/// The setter of `FerroList<T>.Capacity`: a capacity below the number of
/// items is an error, as in the managed original.
fn set_list_capacity<T: Clone>(list: &FerroList<T>, capacity: i32) -> Result<(), &'static str> {
    match usize::try_from(capacity) {
        Ok(capacity) if capacity >= list.count() => Ok(list.set_capacity(capacity)),
        _ => Err("capacity was less than the current size."),
    }
}

// FerroUI.Controls

ferro_markup_type!(class Controls {
    namespace: "FerroUI.Controls",
    handles: [Controls, Option<Controls>],
    base: FerroList<Ref<Control>>,
    constructors: [() => Controls::new],
    methods: [fn Add(Ref<Control>) => |controls: &Controls, item: Ref<Control>| controls.add(item)],
});

ferro_markup_type!(class RowDefinitions {
    namespace: "FerroUI.Controls",
    handles: [RowDefinitions, Option<RowDefinitions>],
    base: FerroList<Ref<RowDefinition>>,
    parse: RowDefinitions::parse,
    constructors: [
        () => RowDefinitions::new,
        try (String) => |s: String| RowDefinitions::parse(&s),
    ],
    methods: [fn Add(Ref<RowDefinition>) => |definitions: &RowDefinitions, item: Ref<RowDefinition>| definitions.add(item)],
    // Declared on the base class of the definition lists in the managed original.
    attributes: [FerroList(Separators = [",", " "])],
});

ferro_markup_type!(class ColumnDefinitions {
    namespace: "FerroUI.Controls",
    handles: [ColumnDefinitions, Option<ColumnDefinitions>],
    base: FerroList<Ref<ColumnDefinition>>,
    parse: ColumnDefinitions::parse,
    constructors: [
        () => ColumnDefinitions::new,
        try (String) => |s: String| ColumnDefinitions::parse(&s),
    ],
    methods: [
        fn Add(Ref<ColumnDefinition>) =>
            |definitions: &ColumnDefinitions, item: Ref<ColumnDefinition>| definitions.add(item),
    ],
    // Declared on the base class of the definition lists in the managed original.
    attributes: [FerroList(Separators = [",", " "])],
});

ferro_markup_type!(class ItemCollection {
    namespace: "FerroUI.Controls",
    handles: [ItemCollection, Option<ItemCollection>],
    methods: [fn Add(Option<BoxedValue>) -> i32 => |items: &ItemCollection, item: Option<BoxedValue>| items.add(item)],
});

// The tray icons of an application (the value of `TrayIcon.Icons`).
ferro_markup_type!(class TrayIcons {
    namespace: "FerroUI.Controls",
    handles: [Rc<TrayIcons>, Option<Rc<TrayIcons>>],
    this: Rc<TrayIcons>,
    base: FerroList<Ref<TrayIcon>>,
    constructors: [() => TrayIcons::new],
    methods: [fn Add(Ref<TrayIcon>) => |icons: &Rc<TrayIcons>, item: Ref<TrayIcon>| icons.add(item)],
});

ferro_markup_type!(class WindowIcon {
    namespace: "FerroUI.Controls",
    handles: [Rc<WindowIcon>, Option<Rc<WindowIcon>>],
});

// FerroUI.Controls.Documents

ferro_markup_type!(class InlineCollection {
    namespace: "FerroUI.Controls.Documents",
    handles: [InlineCollection, Option<InlineCollection>],
    base: FerroList<Ref<Inline>>,
    constructors: [() => InlineCollection::new],
    methods: [
        fn Add(Ref<Inline>) => |inlines: &InlineCollection, inline: Ref<Inline>| inlines.add(inline),
        fn Add(String) => |inlines: &InlineCollection, text: String| inlines.add_text(&text),
        fn Add(Ref<Control>) => |inlines: &InlineCollection, control: Ref<Control>| inlines.add_control(control),
    ],
    attributes: [WhitespaceSignificantCollection],
});

// FerroUI.Controls.Templates

ferro_markup_type!(class DataTemplates {
    namespace: "FerroUI.Controls.Templates",
    handles: [DataTemplates, Option<DataTemplates>],
    base: FerroList<Rc<dyn IDataTemplate>>,
    constructors: [() => DataTemplates::new],
    methods: [
        // A typed template without a data type is an error, as in the managed original.
        try fn Add(Rc<dyn IDataTemplate>) => |templates: &DataTemplates, item: Rc<dyn IDataTemplate>| {
            if item.as_typed_data_template().is_some_and(|typed| typed.data_type().is_none()) {
                return Err(
                    "DataTemplate inside of DataTemplates must have a DataType set. Set DataType property or use \
                     ItemTemplate with single template instead.",
                );
            }
            Ok(templates.add(item))
        },
    ],
});

// FerroUI.Controls.Primitives.PopupPositioning

// The parameters of a custom placement as a method named in markup receives them: one shared
// object, which the method changes (the record of the managed original is a reference type).
ferro_markup_type!(class CustomPopupPlacement {
    namespace: "FerroUI.Controls.Primitives.PopupPositioning",
    handles: [Rc<RefCell<CustomPopupPlacement>>, Option<Rc<RefCell<CustomPopupPlacement>>>],
    this: Rc<RefCell<CustomPopupPlacement>>,
    properties: [
        PopupSize: Size { get: |placement: &Rc<RefCell<CustomPopupPlacement>>| placement.borrow().popup_size() },
        Deflate: Thickness { get: |placement: &Rc<RefCell<CustomPopupPlacement>>| placement.borrow().deflate() },
        Target: Ref<Visual> { get: |placement: &Rc<RefCell<CustomPopupPlacement>>| placement.borrow().target() },
        AnchorRectangle: Rect {
            get: |placement: &Rc<RefCell<CustomPopupPlacement>>| placement.borrow().anchor_rectangle,
            set: |placement: &Rc<RefCell<CustomPopupPlacement>>, value: Rect| placement.borrow_mut().anchor_rectangle = value
        },
        Anchor: PopupAnchor {
            get: |placement: &Rc<RefCell<CustomPopupPlacement>>| placement.borrow().anchor(),
            set: |placement: &Rc<RefCell<CustomPopupPlacement>>, value: PopupAnchor| placement.borrow_mut().set_anchor(value)
        },
        Gravity: PopupGravity {
            get: |placement: &Rc<RefCell<CustomPopupPlacement>>| placement.borrow().gravity(),
            set: |placement: &Rc<RefCell<CustomPopupPlacement>>, value: PopupGravity| placement.borrow_mut().set_gravity(value)
        },
        ConstraintAdjustment: PopupPositionerConstraintAdjustment {
            get: |placement: &Rc<RefCell<CustomPopupPlacement>>| placement.borrow().constraint_adjustment,
            set: |placement: &Rc<RefCell<CustomPopupPlacement>>, value: PopupPositionerConstraintAdjustment| {
                placement.borrow_mut().constraint_adjustment = value
            }
        },
        Offset: Point {
            get: |placement: &Rc<RefCell<CustomPopupPlacement>>| placement.borrow().offset,
            set: |placement: &Rc<RefCell<CustomPopupPlacement>>, value: Point| placement.borrow_mut().offset = value
        },
    ],
});

// The delegate type of the `CustomPopupPlacementCallback` properties: a delegate with one
// `Invoke` method, so that markup accepts the name of a method of the root object with that
// signature as the value of such a property.
ferro_markup_type!(class CustomPopupPlacementCallbackValue as "CustomPopupPlacementCallback" {
    namespace: "FerroUI.Controls.Primitives.PopupPositioning",
    handles: [CustomPopupPlacementCallbackValue, Option<CustomPopupPlacementCallbackValue>],
    base: MarkupDelegate,
    methods: [
        fn Invoke(Rc<RefCell<CustomPopupPlacement>>) =>
            |callback: &CustomPopupPlacementCallbackValue, placement: Rc<RefCell<CustomPopupPlacement>>| {
                (callback.0)(&mut placement.borrow_mut())
            },
    ],
});

/// The callback that calls a method named in markup: the method receives the parameters as
/// one shared object, and what it leaves in them is the placement.
fn custom_popup_placement_callback(method: &MarkupDelegate) -> CustomPopupPlacementCallbackValue {
    let method = method.clone();
    CustomPopupPlacementCallbackValue(Rc::new(move |placement: &mut CustomPopupPlacement| {
        let shared = Rc::new(RefCell::new(placement.clone()));
        method.invoke(&[Some(Rc::new(shared.clone()) as BoxedValue)]);
        *placement = shared.borrow().clone();
    }))
}

// FerroUI.Collections

/// Carries the metadata of the notifying list of items of this crate (the
/// list type belongs to the base crate).
#[doc(hidden)]
pub struct FerroListOf<T>(std::marker::PhantomData<T>);

// The lists the named collections of this crate derive from.

ferro_markup_type!(class FerroListOf<Ref<Control>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Ref<Control>>, Option<FerroList<Ref<Control>>>],
    this: FerroList<Ref<Control>>,
    generic: "FerroList`1" [Ref<Control>],
    constructors: [() => FerroList::<Ref<Control>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Ref<Control>>| list.count() as i32 },
    ],
    methods: [fn Add(Ref<Control>) => |list: &FerroList<Ref<Control>>, item: Ref<Control>| list.add(item)],
});

ferro_markup_type!(class FerroListOf<Ref<Inline>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Ref<Inline>>, Option<FerroList<Ref<Inline>>>],
    this: FerroList<Ref<Inline>>,
    generic: "FerroList`1" [Ref<Inline>],
    constructors: [() => FerroList::<Ref<Inline>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Ref<Inline>>| list.count() as i32 },
    ],
    methods: [fn Add(Ref<Inline>) => |list: &FerroList<Ref<Inline>>, item: Ref<Inline>| list.add(item)],
});

ferro_markup_type!(class FerroListOf<Rc<dyn IDataTemplate>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Rc<dyn IDataTemplate>>, Option<FerroList<Rc<dyn IDataTemplate>>>],
    this: FerroList<Rc<dyn IDataTemplate>>,
    generic: "FerroList`1" [Rc<dyn IDataTemplate>],
    constructors: [() => FerroList::<Rc<dyn IDataTemplate>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Rc<dyn IDataTemplate>>| list.count() as i32 },
    ],
    methods: [fn Add(Rc<dyn IDataTemplate>) => |list: &FerroList<Rc<dyn IDataTemplate>>, item: Rc<dyn IDataTemplate>| list.add(item)],
});

ferro_markup_type!(class FerroListOf<Ref<RowDefinition>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Ref<RowDefinition>>, Option<FerroList<Ref<RowDefinition>>>],
    this: FerroList<Ref<RowDefinition>>,
    generic: "FerroList`1" [Ref<RowDefinition>],
    constructors: [() => FerroList::<Ref<RowDefinition>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Ref<RowDefinition>>| list.count() as i32 },
    ],
    methods: [fn Add(Ref<RowDefinition>) => |list: &FerroList<Ref<RowDefinition>>, item: Ref<RowDefinition>| list.add(item)],
});

ferro_markup_type!(class FerroListOf<Ref<ColumnDefinition>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Ref<ColumnDefinition>>, Option<FerroList<Ref<ColumnDefinition>>>],
    this: FerroList<Ref<ColumnDefinition>>,
    generic: "FerroList`1" [Ref<ColumnDefinition>],
    constructors: [() => FerroList::<Ref<ColumnDefinition>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Ref<ColumnDefinition>>| list.count() as i32 },
    ],
    methods: [fn Add(Ref<ColumnDefinition>) => |list: &FerroList<Ref<ColumnDefinition>>, item: Ref<ColumnDefinition>| list.add(item)],
});

ferro_markup_type!(class FerroListOf<Ref<TrayIcon>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Ref<TrayIcon>>, Option<FerroList<Ref<TrayIcon>>>],
    this: FerroList<Ref<TrayIcon>>,
    generic: "FerroList`1" [Ref<TrayIcon>],
    constructors: [() => FerroList::<Ref<TrayIcon>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Ref<TrayIcon>>| list.count() as i32 },
    ],
    methods: [fn Add(Ref<TrayIcon>) => |list: &FerroList<Ref<TrayIcon>>, item: Ref<TrayIcon>| list.add(item)],
});

// The commands of a command bar.
ferro_markup_type!(interface dyn crate::ICommandBarElement as "ICommandBarElement" {
    namespace: "FerroUI.Controls",
    handles: [Rc<dyn crate::ICommandBarElement>, Option<Rc<dyn crate::ICommandBarElement>>],
});

ferro_markup_type!(class FerroListOf<Rc<dyn crate::ICommandBarElement>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Rc<dyn crate::ICommandBarElement>>, Option<FerroList<Rc<dyn crate::ICommandBarElement>>>],
    this: FerroList<Rc<dyn crate::ICommandBarElement>>,
    generic: "FerroList`1" [Rc<dyn crate::ICommandBarElement>],
    constructors: [() => FerroList::<Rc<dyn crate::ICommandBarElement>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Rc<dyn crate::ICommandBarElement>>| list.count() as i32 },
    ],
    methods: [
        fn Add(Rc<dyn crate::ICommandBarElement>) =>
            |list: &FerroList<Rc<dyn crate::ICommandBarElement>>, item: Rc<dyn crate::ICommandBarElement>| list.add(item),
    ],
});

// The items of a native menu.
ferro_markup_type!(class FerroListOf<Ref<NativeMenuItemBase>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Ref<NativeMenuItemBase>>, Option<FerroList<Ref<NativeMenuItemBase>>>],
    this: FerroList<Ref<NativeMenuItemBase>>,
    generic: "FerroList`1" [Ref<NativeMenuItemBase>],
    constructors: [() => FerroList::<Ref<NativeMenuItemBase>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Ref<NativeMenuItemBase>>| list.count() as i32 },
    ],
    methods: [
        fn Add(Ref<NativeMenuItemBase>) =>
            |list: &FerroList<Ref<NativeMenuItemBase>>, item: Ref<NativeMenuItemBase>| list.add(item),
    ],
});

// The pages of a multi page (`MultiPage.Pages`).
ferro_markup_type!(class FerroListOf<Ref<Page>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Ref<Page>>, Option<FerroList<Ref<Page>>>],
    this: FerroList<Ref<Page>>,
    generic: "FerroList`1" [Ref<Page>],
    constructors: [() => FerroList::<Ref<Page>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Ref<Page>>| list.count() as i32 },
    ],
    methods: [fn Add(Ref<Page>) => |list: &FerroList<Ref<Page>>, item: Ref<Page>| list.add(item)],
});

// The columns of a table view (`TableView.Columns`).
ferro_markup_type!(class FerroListOf<Ref<TableViewColumn>> as "FerroList`1" {
    namespace: "FerroUI.Collections",
    handles: [FerroList<Ref<TableViewColumn>>, Option<FerroList<Ref<TableViewColumn>>>],
    this: FerroList<Ref<TableViewColumn>>,
    generic: "FerroList`1" [Ref<TableViewColumn>],
    constructors: [() => FerroList::<Ref<TableViewColumn>>::new],
    properties: [
        Capacity: i32 { get: list_capacity, try_set: set_list_capacity },
        Count: i32 { get: |list: &FerroList<Ref<TableViewColumn>>| list.count() as i32 },
    ],
    methods: [fn Add(Ref<TableViewColumn>) => |list: &FerroList<Ref<TableViewColumn>>, item: Ref<TableViewColumn>| list.add(item)],
});

/// The types declared in this file.
pub(super) const TYPES: &[&MarkupType] = &[
    <dyn crate::ICommandBarElement as MarkupTyped>::MARKUP,
    <FerroListOf<Rc<dyn crate::ICommandBarElement>> as MarkupTyped>::MARKUP,
    <Controls as MarkupTyped>::MARKUP,
    <RowDefinitions as MarkupTyped>::MARKUP,
    <ColumnDefinitions as MarkupTyped>::MARKUP,
    <ItemCollection as MarkupTyped>::MARKUP,
    <TrayIcons as MarkupTyped>::MARKUP,
    <FerroListOf<Ref<TrayIcon>> as MarkupTyped>::MARKUP,
    <WindowIcon as MarkupTyped>::MARKUP,
    <InlineCollection as MarkupTyped>::MARKUP,
    <DataTemplates as MarkupTyped>::MARKUP,
    <CustomPopupPlacement as MarkupTyped>::MARKUP,
    <CustomPopupPlacementCallbackValue as MarkupTyped>::MARKUP,
    <FerroListOf<Ref<Control>> as MarkupTyped>::MARKUP,
    <FerroListOf<Ref<Inline>> as MarkupTyped>::MARKUP,
    <FerroListOf<Rc<dyn IDataTemplate>> as MarkupTyped>::MARKUP,
    <FerroListOf<Ref<RowDefinition>> as MarkupTyped>::MARKUP,
    <FerroListOf<Ref<ColumnDefinition>> as MarkupTyped>::MARKUP,
    <FerroListOf<Ref<NativeMenuItemBase>> as MarkupTyped>::MARKUP,
    <FerroListOf<Ref<Page>> as MarkupTyped>::MARKUP,
    <FerroListOf<Ref<TableViewColumn>> as MarkupTyped>::MARKUP,
];

/// Registers the nullable forms of the types that can be held in untyped
/// values with the untyped value conversions of the current thread.
pub(super) fn register_value_types() {
    ValueTypes::register_nullable::<Rc<dyn crate::ICommandBarElement>>();
    ValueTypes::register_nullable::<FerroList<Rc<dyn crate::ICommandBarElement>>>();
    ValueTypes::register_nullable::<Controls>();
    ValueTypes::register_nullable::<RowDefinitions>();
    ValueTypes::register_nullable::<ColumnDefinitions>();
    ValueTypes::register_nullable::<ItemCollection>();
    ValueTypes::register_nullable::<InlineCollection>();
    ValueTypes::register_nullable::<DataTemplates>();
    ValueTypes::register_nullable::<FerroList<Ref<Control>>>();
    ValueTypes::register_nullable::<FerroList<Ref<Inline>>>();
    ValueTypes::register_nullable::<FerroList<Rc<dyn IDataTemplate>>>();
    ValueTypes::register_nullable::<FerroList<Ref<RowDefinition>>>();
    ValueTypes::register_nullable::<FerroList<Ref<ColumnDefinition>>>();
    ValueTypes::register_nullable::<FerroList<Ref<NativeMenuItemBase>>>();
    ValueTypes::register_nullable::<FerroList<Ref<Page>>>();
    ValueTypes::register_nullable::<FerroList<Ref<TableViewColumn>>>();
    ValueTypes::register_nullable::<Rc<WindowIcon>>();
    ValueTypes::register_nullable::<Rc<RefCell<CustomPopupPlacement>>>();
    ValueTypes::register_nullable::<CustomPopupPlacementCallbackValue>();
    // The delegate of a method named in markup is a callback: the value of a
    // `CustomPopupPlacementCallback` property.
    ValueTypes::register_cast::<MarkupDelegate, CustomPopupPlacementCallbackValue>(custom_popup_placement_callback);
    // A named collection is the list it derives from: the same list, so that the members
    // of the list (`Capacity`) are reached through the collection.
    // The collection handles of items controls: a property that holds one takes the handle
    // (and null) from an untyped value, and a binding delivers a handle to it.
    ValueTypes::register_nullable::<crate::ItemsSource>();
    ValueTypes::register_nullable::<crate::primitives::SelectedItemsList>();
    crate::ItemsSource::register_binding_conversion::<crate::ItemsSource>();
    // A list of untyped items is an enumerable (a list of the runtime library the run-time
    // loader creates for markup, `List<T>` or `ArrayList`): the handle shares the list.
    ValueTypes::register_cast::<Rc<FerroList<Option<BoxedValue>>>, crate::ItemsSource>(|list| {
        crate::ItemsSource::from(list.clone())
    });
    // A vector of untyped values is an enumerable too (the errors of a control, an
    // `IEnumerable<object>` in the managed original, which the template of the errors binds
    // to the items of an items control): the handle holds the items of the vector. The
    // conversions are what a binding delivers the vector with, to the handle and to its
    // nullable form.
    ValueTypes::register_cast::<Vec<BoxedValue>, crate::ItemsSource>(|items| crate::ItemsSource::from(items.clone()));
    ValueTypes::register_conversion::<Vec<BoxedValue>, Option<crate::ItemsSource>>(|items| {
        Some(Some(crate::ItemsSource::from(items.clone())))
    });
    ValueTypes::register_cast::<Vec<Option<BoxedValue>>, crate::ItemsSource>(|items| {
        crate::ItemsSource::from(items.clone())
    });
    ValueTypes::register_conversion::<Vec<Option<BoxedValue>>, Option<crate::ItemsSource>>(|items| {
        Some(Some(crate::ItemsSource::from(items.clone())))
    });
    ValueTypes::register_nullable::<Rc<TrayIcons>>();
    ValueTypes::register_nullable::<FerroList<Ref<TrayIcon>>>();
    ValueTypes::register_cast::<Rc<TrayIcons>, FerroList<Ref<TrayIcon>>>(|c| (***c).clone());
    ValueTypes::register_cast::<Controls, FerroList<Ref<Control>>>(|c| (**c).clone());
    ValueTypes::register_cast::<InlineCollection, FerroList<Ref<Inline>>>(|c| (**c).clone());
    ValueTypes::register_cast::<DataTemplates, FerroList<Rc<dyn IDataTemplate>>>(|c| (**c).clone());
    ValueTypes::register_cast::<RowDefinitions, FerroList<Ref<RowDefinition>>>(|c| (***c).clone());
    ValueTypes::register_cast::<ColumnDefinitions, FerroList<Ref<ColumnDefinition>>>(|c| (***c).clone());
    // The named collections whose Rust type dereferences to the list: generated code
    // passes a reference to such a collection where a member of the list takes the list.
    ValueTypes::register_deref::<Controls, FerroList<Ref<Control>>>(|c| c);
    ValueTypes::register_deref::<InlineCollection, FerroList<Ref<Inline>>>(|c| c);
    ValueTypes::register_deref::<DataTemplates, FerroList<Rc<dyn IDataTemplate>>>(|c| c);
    ValueTypes::register_deref::<RowDefinitions, FerroList<Ref<RowDefinition>>>(|c| c);
    ValueTypes::register_deref::<ColumnDefinitions, FerroList<Ref<ColumnDefinition>>>(|c| c);
}
