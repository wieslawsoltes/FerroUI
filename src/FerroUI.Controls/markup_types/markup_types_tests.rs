//! Tests specific to this port: the markup metadata declared in this module
//! is consistent and its members work when invoked through metadata only.

use super::TYPE_LISTS;
use crate::chrome::WindowDrawnDecorationsContent;
use crate::documents::{Inline, InlineCollection, Run};
use crate::presenters::ContentPresenter;
use crate::primitives::popup_positioning::{CustomPopupPlacement, PopupAnchor};
use crate::primitives::CustomPopupPlacementCallbackValue;
use crate::{
    Border, Button, ColumnDefinition, ContentControl, Control, Decorator, Dock, GridLength, Panel, SizeToContent,
    TickList, Window, WindowTransparencyLevel,
};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::interactivity::{RoutedEvent, RoutedEventArgs};
use ferroui_base::metadata::{
    attributes, from_markup_value, into_markup_value, MarkupAttributeValue, MarkupDelegate, MarkupProperty, MarkupType,
    MarkupTypeKind, MarkupTyped, MarkupValue,
};
use ferroui_base::{BoxedValue, LocatorExtensions, Point, Ref, Size, StaticType, Thickness, TypeInfo, Visual};
use std::cell::RefCell;
use std::rc::Rc;

fn boxed<T: PartialEq + 'static>(value: T) -> MarkupValue {
    Some(Rc::new(value))
}

fn text(value: &str) -> MarkupValue {
    boxed(value.to_string())
}

fn unbox<T: Clone + 'static>(value: &MarkupValue) -> T {
    from_markup_value::<T>(value).unwrap_or_else(|| panic!("a value of type {}", std::any::type_name::<T>()))
}

fn declared_types() -> Vec<&'static MarkupType> {
    crate::register_types();
    TYPE_LISTS.iter().flat_map(|types| types.iter().copied()).collect()
}

/// The classes of this crate that declare a `markup` part.
fn class_markups() -> Vec<(&'static TypeInfo, &'static MarkupType)> {
    crate::register_types();
    TypeInfo::registered_types()
        .into_iter()
        .filter(|type_| type_.module_path().starts_with("ferroui_controls"))
        .filter_map(|type_| type_.markup().map(|markup| (type_, markup)))
        .filter(|(_, markup)| markup.module_path.starts_with("ferroui_controls::markup_types"))
        .collect()
}

fn class_markup<T: StaticType>() -> &'static MarkupType {
    crate::register_types();
    T::TYPE.markup().unwrap_or_else(|| panic!("{} declares markup", T::TYPE))
}

/// The plain property `name` of a class or of one of its base classes.
fn find_plain_property(type_: &'static TypeInfo, name: &str) -> Option<&'static MarkupProperty> {
    let mut current = Some(type_);
    while let Some(type_) = current {
        if let Some(property) = type_.markup().and_then(|markup| markup.find_property(name)) {
            return Some(property);
        }
        current = type_.base_type();
    }
    None
}

fn has_registered_property(type_: &'static TypeInfo, name: &str) -> bool {
    let mut current = Some(type_);
    while let Some(type_) = current {
        if type_.find_property(name).is_some() {
            return true;
        }
        current = type_.base_type();
    }
    false
}

#[test]
fn every_declaration_of_this_module_is_registered() {
    let sources = [
        include_str!("enums.rs"),
        include_str!("values.rs"),
        include_str!("contracts.rs"),
        include_str!("plain.rs"),
        include_str!("classes.rs"),
        include_str!("converters.rs"),
    ];
    let declared: usize = sources
        .iter()
        .flat_map(|source| source.lines())
        .filter(|line| line.starts_with("ferro_markup_type!(") || line.starts_with("ferro_markup_enum!("))
        .count();
    let types = declared_types();
    assert_eq!(types.len(), declared);

    for type_ in &types {
        // The collection handle of the items controls is published as the enumerable of
        // the runtime library.
        assert!(type_.namespace().starts_with("FerroUI") || type_.namespace() == "System.Collections", "{type_:?}");
        let found = MarkupType::find(type_.namespace(), type_.name)
            .unwrap_or_else(|| panic!("{} is registered", type_.full_name()));
        // The instantiations of a generic type share its name.
        assert!(std::ptr::eq(found, *type_) || type_.generic.is_some(), "{} is declared twice", type_.full_name());
    }
}

#[test]
fn handles_resolve_to_their_type() {
    for type_ in declared_types() {
        // A type that only owns static members has no values.
        if type_.kind == MarkupTypeKind::Static {
            continue;
        }
        assert!(!type_.handles.is_empty(), "{type_:?} has no handle");
        for handle in type_.handles {
            let found = MarkupType::find_by_handle(handle().id()).unwrap();
            assert!(std::ptr::eq(found, type_), "{} is a handle of {type_:?} and of {found:?}", handle());
        }
        if let Some(nullable) = type_.nullable {
            assert!(matches!(type_.kind, MarkupTypeKind::Struct | MarkupTypeKind::Enum));
            let found = MarkupType::find_by_nullable_handle(nullable().id()).unwrap();
            assert!(std::ptr::eq(found, type_));
            assert!(ValueTypes::accepts_null(nullable()), "Option<{}> takes null", type_.name);
        }
    }
}

#[test]
fn class_markup_belongs_to_its_class() {
    let classes = class_markups();
    assert!(classes.len() > 50, "{}", classes.len());
    for (type_, markup) in classes {
        assert_eq!(markup.name, type_.name());
        assert_eq!(markup.namespace(), type_.namespace(), "{type_}");
        assert!(std::ptr::eq((markup.type_info.unwrap())(), type_));
        assert_eq!(markup.handle().map(|handle| handle.id()), type_.handle());
        // The parameterless constructor of a class is its `new:`.
        assert!(markup.constructors.iter().all(|constructor| !constructor.parameters.is_empty()), "{type_}");
    }
}

#[test]
fn content_properties_and_dependencies_name_existing_properties() {
    let mut with_content = 0;
    for (type_, markup) in class_markups() {
        if let Some(content) = markup.content_property {
            assert!(
                has_registered_property(type_, content) || find_plain_property(type_, content).is_some(),
                "{type_}: content property {content}"
            );
            with_content += 1;
        }
        for property in markup.properties {
            for attribute in property.attributes.iter().filter(|a| a.name == attributes::DEPENDS_ON) {
                let MarkupAttributeValue::Str(target) = attribute.arguments[0] else { panic!("{type_}") };
                assert!(
                    has_registered_property(type_, target) || find_plain_property(type_, target).is_some(),
                    "{type_}.{}: depends on {target}",
                    property.name
                );
            }
        }
    }
    assert!(with_content >= 12, "{with_content}");
    for type_ in declared_types() {
        if let Some(content) = type_.content_property {
            assert!(type_.find_property(content).is_some(), "{type_:?}: content property {content}");
        }
    }
}

#[test]
fn template_parts_name_classes() {
    let mut parts = 0;
    for (type_, markup) in class_markups() {
        for attribute in markup.attributes.iter().filter(|a| a.name == attributes::TEMPLATE_PART) {
            let MarkupAttributeValue::Str(name) = attribute.arguments[0] else { panic!("{type_}") };
            assert!(name.starts_with("PART_"), "{type_}: {name}");
            let MarkupAttributeValue::Type(part) = attribute.arguments[1] else { panic!("{type_}") };
            let (part_type, nullable) = TypeInfo::find_by_handle(part().id()).unwrap_or_else(|| panic!("{type_}: {name}"));
            assert!(!nullable && part_type.is_assignable_from(part_type));
            parts += 1;
        }
        for attribute in markup.attributes.iter().filter(|a| a.name == attributes::PSEUDO_CLASSES) {
            assert!(!attribute.arguments.is_empty());
            for argument in attribute.arguments {
                let MarkupAttributeValue::Str(name) = argument else { panic!("{type_}") };
                assert!(name.starts_with(':'), "{type_}: {name}");
            }
        }
    }
    assert!(parts > 20, "{parts}");
}

#[test]
fn enumeration_members_are_found_by_name() {
    let enums: Vec<_> = declared_types().into_iter().filter(|t| t.kind == MarkupTypeKind::Enum).collect();
    assert_eq!(enums.len(), super::enums::TYPES.len());
    for type_ in enums {
        assert!(!type_.enum_members.is_empty(), "{type_:?}");
        for member in type_.enum_members {
            let found = type_.find_enum_member(&member.name.to_lowercase(), true).unwrap();
            assert_eq!(found.value, type_.find_enum_member(member.name, false).unwrap().value);
            let value = (member.get)();
            assert_eq!(Some(ValueType::of_value(&*value)), type_.handle(), "{type_:?}.{}", member.name);
            // The numeric value converts back to the member.
            let from_value = (type_.enum_from_value.unwrap())(member.value).unwrap();
            assert!(ValueTypes::identity_equals(Some(&from_value), Some(&value)), "{type_:?}.{}", member.name);
        }
    }

    let dock = <Dock as MarkupTyped>::MARKUP;
    assert_eq!(dock.full_name(), "FerroUI.Controls.Dock");
    let members: Vec<&str> = dock.enum_members.iter().map(|m| m.name).collect();
    assert_eq!(members, ["Left", "Bottom", "Right", "Top"]);
    assert_eq!(unbox::<Option<Dock>>(&Some((dock.find_enum_member("top", true).unwrap().get)())), Some(Dock::Top));

    let size_to_content = <SizeToContent as MarkupTyped>::MARKUP;
    assert!(size_to_content.is_flags);
    let both = size_to_content.find_enum_member("WidthAndHeight", false).unwrap();
    assert_eq!(both.value, 3);
}

/// Whether `value` is a value of `type_`: held in one of its handle types,
/// or assignable to one.
fn is_value_of(value: &BoxedValue, type_: &MarkupType) -> bool {
    let actual = ValueType::of_value(&**value);
    type_.handles.iter().any(|handle| handle() == actual || ValueTypes::is_assignable(actual, handle()))
}

#[test]
fn parameterless_constructors_create_a_value_of_the_type() {
    let mut count = 0;
    for type_ in declared_types() {
        for constructor in type_.constructors.iter().filter(|c| c.parameters.is_empty()) {
            let value = (constructor.invoke)(&[]).unwrap().unwrap();
            assert!(is_value_of(&value, type_), "{type_:?}: {}", value.type_name());
            count += 1;
        }
    }
    assert!(count >= 6, "{count}");
}

#[test]
fn text_convertible_types_are_parsed_by_type_lookup() {
    crate::register_types();
    // What a loader that does not link this crate does: find the type by
    // name (or by the type of a property) and call its `Parse`.
    let by_name = MarkupType::find("FerroUI.Controls", "GridLength").unwrap();
    let by_handle = MarkupType::find_by_handle(std::any::TypeId::of::<GridLength>()).unwrap();
    assert!(std::ptr::eq(by_name, by_handle));
    let parse = by_name.parse.unwrap();
    assert_eq!(unbox::<GridLength>(&parse(&[text("2*")]).unwrap()), GridLength::new(2.0, crate::GridUnitType::Star));
    assert_eq!(unbox::<GridLength>(&parse(&[text("Auto")]).unwrap()), GridLength::auto());
    assert_eq!(unbox::<GridLength>(&parse(&[text("12.5")]).unwrap()), GridLength::from_pixels(12.5));
    // The text form of a value parses to the value again.
    let star = GridLength::new(3.0, crate::GridUnitType::Star);
    assert_eq!(unbox::<GridLength>(&parse(&[text(&star.to_string())]).unwrap()), star);
    assert!(parse(&[text("x")]).is_err());
    // Constructors and read-only properties, as a loader uses them.
    let constructor = by_name.constructors.iter().find(|c| c.parameters.len() == 2).unwrap();
    let value = (constructor.invoke)(&[boxed(2.0f64), boxed(crate::GridUnitType::Star)]).unwrap();
    assert_eq!(unbox::<GridLength>(&value), GridLength::new(2.0, crate::GridUnitType::Star));
    let pixels = (by_name.constructors.iter().find(|c| c.parameters.len() == 1).unwrap().invoke)(&[boxed(5.0f64)]);
    assert_eq!(unbox::<GridLength>(&pixels.unwrap()), GridLength::from_pixels(5.0));
    let read = |name: &str| (by_name.find_property(name).unwrap().get.unwrap())(&[value.clone()]).unwrap();
    assert_eq!(unbox::<f64>(&read("Value")), 2.0);
    assert_eq!(unbox::<crate::GridUnitType>(&read("GridUnitType")), crate::GridUnitType::Star);
    assert_eq!(
        (unbox::<bool>(&read("IsStar")), unbox::<bool>(&read("IsAuto")), unbox::<bool>(&read("IsAbsolute"))),
        (true, false, false)
    );
    assert_eq!(unbox::<GridLength>(&(by_name.find_static_property("Auto").unwrap().get.unwrap())(&[]).unwrap()), GridLength::auto());

    for type_ in declared_types() {
        if type_.parse.is_some() {
            let known = ["GridLength", "RowDefinitions", "ColumnDefinitions", "WindowTransparencyLevel",
                "WindowTransparencyLevelCollection"];
            assert!(known.contains(&type_.name), "a sample for {type_:?}");
        }
    }

    let level = <WindowTransparencyLevel as MarkupTyped>::MARKUP;
    assert_eq!(unbox::<WindowTransparencyLevel>(&(level.find_static_property("Mica").unwrap().get.unwrap())(&[]).unwrap()), WindowTransparencyLevel::mica());
    let levels = MarkupType::find("FerroUI.Controls", "WindowTransparencyLevelCollection").unwrap();
    let parsed = (levels.parse.unwrap())(&[text("Mica, AcrylicBlur")]).unwrap();
    let expected = crate::WindowTransparencyLevelCollection::new(vec![
        WindowTransparencyLevel::mica(),
        WindowTransparencyLevel::acrylic_blur(),
    ]);
    assert_eq!(unbox::<crate::WindowTransparencyLevelCollection>(&parsed), expected);
    assert!((levels.parse.unwrap())(&[text("Mica, Glass")]).is_err());
    let ticks = <TickList as MarkupTyped>::MARKUP;
    assert!(from_markup_value::<Option<TickList>>(&(ticks.constructors[0].invoke)(&[]).unwrap()).is_some());
}

#[test]
fn inline_collections_are_added_to_through_metadata() {
    crate::register_types();
    let markup = <InlineCollection as MarkupTyped>::MARKUP;
    assert!(markup.find_attribute(attributes::WHITESPACE_SIGNIFICANT_COLLECTION).is_some());
    let collection = (markup.constructors[0].invoke)(&[]).unwrap();
    let inlines = unbox::<InlineCollection>(&collection);

    // `Add` per accepted item type.
    let add: Vec<_> = markup.find_methods("Add").collect();
    assert_eq!(add.len(), 3);
    let by_type = |type_: ValueType| add.iter().find(|m| (m.parameters[0])() == type_).unwrap();
    (by_type(ValueType::of::<String>()).invoke)(&[collection.clone(), text("a")]).unwrap();
    (by_type(ValueType::of::<Ref<Inline>>()).invoke)(&[collection.clone(), into_markup_value(Run::new())]).unwrap();
    assert_eq!(inlines.count(), 2);
    assert!((by_type(ValueType::of::<Ref<Inline>>()).invoke)(&[collection.clone(), text("b")]).is_err());

    // The collection is a value of the nullable property type.
    assert!(from_markup_value::<Option<InlineCollection>>(&collection).is_some());
}

#[test]
fn class_members_work_through_metadata() {
    let button = class_markup::<Button>();
    assert_eq!(button.namespace(), "FerroUI.Controls");
    let click = button.find_field("ClickEvent").unwrap();
    assert_eq!((click.type_)(), ValueType::of::<RoutedEvent<RoutedEventArgs>>());
    assert!(unbox::<RoutedEvent<RoutedEventArgs>>(&(click.get)()) == *Button::click_event());
    let pseudo_classes = button.find_attribute(attributes::PSEUDO_CLASSES).unwrap();
    assert_eq!(pseudo_classes.arguments, [MarkupAttributeValue::Str(":flyout-open"), MarkupAttributeValue::Str(":pressed")]);

    // The content property is inherited from the class that declares it.
    assert_eq!(class_markup::<ContentControl>().content_property, Some("Content"));
    assert_eq!(Button::TYPE.content_property(), Some("Content"));
    assert_eq!(Border::TYPE.content_property(), Some("Child"));
    assert_eq!(class_markup::<Decorator>().content_property, Some("Child"));
    let part = class_markup::<ContentControl>().find_attribute(attributes::TEMPLATE_PART).unwrap();
    assert_eq!(part.arguments[0], MarkupAttributeValue::Str("PART_ContentPresenter"));
    assert_eq!(part.arguments[1], MarkupAttributeValue::Type(|| ValueType::of::<Ref<ContentPresenter>>()));

    // Attributes of registered properties.
    let depends_on = class_markup::<ContentControl>().find_property_attributes("Content");
    assert_eq!(depends_on[0].name, attributes::DEPENDS_ON);
    assert_eq!(depends_on[0].arguments, [MarkupAttributeValue::Str("ContentTemplate")]);
    assert!(ContentControl::TYPE.find_property("ContentTemplate").is_some());
    let resolve = class_markup::<crate::primitives::Popup>().find_property_attributes("PlacementTarget");
    assert_eq!(resolve[0].name, attributes::RESOLVE_BY_NAME);
    let attached = class_markup::<crate::RelativePanel>();
    assert_eq!(attached.property_attributes.len(), 10);
    assert_eq!(attached.find_property_attributes("RightOf")[0].name, attributes::RESOLVE_BY_NAME);
    let items = class_markup::<crate::ItemsControl>().find_property_attributes("ItemTemplate");
    assert_eq!(items[0].name, attributes::INHERIT_DATA_TYPE_FROM_ITEMS);
    assert_eq!(items[0].arguments, [MarkupAttributeValue::Str("ItemsSource")]);
    for (type_, markup) in class_markups() {
        for (property, _) in markup.property_attributes {
            assert!(type_.find_property(property).is_some(), "{type_}.{property} is a registered property");
        }
    }

    // A constructor with arguments.
    let column = class_markup::<ColumnDefinition>();
    let constructor = column.constructors.iter().find(|c| c.parameters.len() == 1).unwrap();
    let value = (constructor.invoke)(&[boxed(GridLength::star())]).unwrap();
    assert_eq!(unbox::<Ref<ColumnDefinition>>(&value).width(), GridLength::star());

    // Plain properties with a setter.
    let content = class_markup::<WindowDrawnDecorationsContent>();
    let instance = into_markup_value(WindowDrawnDecorationsContent::new());
    let overlay = content.find_property("Overlay").unwrap();
    let border = Border::new();
    (overlay.set.unwrap())(&[instance.clone(), into_markup_value(border.clone())]).unwrap();
    let read = (overlay.get.unwrap())(&[instance.clone()]).unwrap();
    assert_eq!(unbox::<Ref<Control>>(&read), border.upcast::<Control>());
    (overlay.set.unwrap())(&[instance.clone(), None]).unwrap();
    assert_eq!((overlay.get.unwrap())(&[instance]).unwrap(), None);

    let presenter = into_markup_value(crate::presenters::TextPresenter::new());
    let font_size = class_markup::<crate::presenters::TextPresenter>().find_property("FontSize").unwrap();
    (font_size.set.unwrap())(&[presenter.clone(), boxed(20.0f64)]).unwrap();
    assert_eq!(unbox::<f64>(&(font_size.get.unwrap())(&[presenter]).unwrap()), 20.0);

    // Plain events are declared with the arguments they pass.
    let window = class_markup::<Window>();
    // A cancellable event passes the sender and its arguments.
    assert_eq!(window.find_event("Closing").unwrap().arguments.len(), 2);
    let top_level = class_markup::<crate::TopLevel>();
    assert_eq!(top_level.find_event("Opened").unwrap().arguments.len(), 2);
    assert!(top_level.find_event("Closed").is_some());
}

// The types the markup compiler looks up by name.

/// Every type of the namespaces of this crate that the markup compiler
/// resolves by its full name when it starts (its well-known types and the
/// types of its language configuration).
const WELL_KNOWN_TYPES: &[&str] = &[
    "FerroUI.Controls.ColumnDefinition",
    "FerroUI.Controls.ColumnDefinitions",
    "FerroUI.Controls.ContentControl",
    "FerroUI.Controls.Control",
    "FerroUI.Controls.GridLength",
    "FerroUI.Controls.GridUnitType",
    "FerroUI.Controls.ITemplate`1",
    "FerroUI.Controls.ItemsControl",
    "FerroUI.Controls.RowDefinition",
    "FerroUI.Controls.RowDefinitions",
    "FerroUI.Controls.Templates.IDataTemplate",
    "FerroUI.Controls.WindowIcon",
    "FerroUI.Controls.WindowTransparencyLevel",
    "FerroUI.Metadata.IAddChild`1",
];

#[test]
fn the_well_known_types_of_the_markup_compiler_are_found_by_name() {
    crate::register_types();
    let found = |full_name: &&str| {
        let (namespace, name) = full_name.rsplit_once('.').unwrap();
        if TypeInfo::find(namespace, name).is_some() {
            return true;
        }
        if name.contains('`') {
            // A generic definition is found through a declared instantiation.
            return declared_types()
                .iter()
                .any(|type_| type_.namespace() == namespace && type_.generic.is_some_and(|g| g.definition == name));
        }
        MarkupType::find(namespace, name).is_some_and(|type_| type_.generic.is_none())
    };
    let missing: Vec<&str> = WELL_KNOWN_TYPES.iter().copied().filter(|name| !found(name)).collect();
    assert!(missing.is_empty(), "{missing:?}");

    // `ITemplate<Control>` and the constructor of a grid length.
    let template = <dyn crate::templates::ITemplateOf<Ref<Control>> as MarkupTyped>::MARKUP;
    assert_eq!((template.generic.unwrap().arguments[0])(), ValueType::of::<Ref<Control>>());
    let grid_length = MarkupType::find("FerroUI.Controls", "GridLength").unwrap();
    assert!(grid_length.constructors.iter().any(|c| {
        c.parameters.iter().map(|p| p()).collect::<Vec<_>>()
            == [ValueType::of::<f64>(), ValueType::of::<crate::GridUnitType>()]
    }));
}

#[test]
fn collection_properties_are_added_to_through_metadata() {
    crate::register_types();
    let find = |namespace: &str, name: &str| MarkupType::find(namespace, name).unwrap();

    // The children of a panel: the content property is a collection.
    let panel = Panel::new();
    let markup = class_markup::<Panel>();
    assert_eq!(markup.content_property, Some("Children"));
    let children = (markup.find_property("Children").unwrap().get.unwrap())(&[into_markup_value(panel.clone())]).unwrap();
    let controls = find("FerroUI.Controls", "Controls");
    assert!(std::ptr::eq(MarkupType::find_by_handle(ValueType::of_value(&**children.as_ref().unwrap()).id()).unwrap(), controls));
    let child = Border::new();
    (controls.find_methods("Add").next().unwrap().invoke)(&[children, into_markup_value(child.clone())]).unwrap();
    assert_eq!(panel.children().count(), 1);
    assert_eq!(child.parent().unwrap(), panel);

    let list = MarkupType::find_by_handle((controls.base.unwrap())().id()).unwrap();
    assert_eq!((list.generic.unwrap().arguments[0])(), ValueType::of::<Ref<Control>>());

    // The resources of an application are typed with the contract.
    let resources = class_markup::<crate::Application>().find_property("Resources").unwrap();
    assert_eq!((resources.type_)(), ValueType::of::<Rc<dyn ferroui_base::controls::IResourceDictionary>>());

    // A typed routed event of this crate converts to the untyped routed event.
    let click = boxed(*Button::click_event());
    assert!(unbox::<RoutedEvent>(&click) == Button::click_event().as_routed_event());

    // The definitions of a grid, from text and item by item.
    let grid = crate::Grid::new();
    let instance = into_markup_value(grid.clone());
    let rows = find("FerroUI.Controls", "RowDefinitions");
    let parsed = (rows.parse.unwrap())(&[text("Auto,*,2*")]).unwrap();
    let property = class_markup::<crate::Grid>().find_property("RowDefinitions").unwrap();
    (property.set.unwrap())(&[instance.clone(), parsed]).unwrap();
    assert_eq!(grid.row_definitions().count(), 3);
    let read = (property.get.unwrap())(&[instance.clone()]).unwrap();
    (rows.find_methods("Add").next().unwrap().invoke)(&[read, into_markup_value(crate::RowDefinition::new())]).unwrap();
    assert_eq!(grid.row_definitions().count(), 4);
    let columns = find("FerroUI.Controls", "ColumnDefinitions");
    let from_text = columns.constructors.iter().find(|c| c.parameters.len() == 1).unwrap();
    let value = (from_text.invoke)(&[text("10,Auto")]).unwrap();
    (class_markup::<crate::Grid>().find_property("ColumnDefinitions").unwrap().set.unwrap())(&[instance, value]).unwrap();
    assert_eq!(grid.column_definitions().count(), 2);
    assert!((from_text.invoke)(&[text("x")]).is_err());

    // The items of an items control.
    let items_control = crate::ItemsControl::new();
    let markup = class_markup::<crate::ItemsControl>();
    assert_eq!(markup.content_property, Some("Items"));
    let items = (markup.find_property("Items").unwrap().get.unwrap())(&[into_markup_value(items_control.clone())]).unwrap();
    (find("FerroUI.Controls", "ItemCollection").find_methods("Add").next().unwrap().invoke)(&[items, text("a")]).unwrap();
    assert_eq!(items_control.item_count(), 1);

    // The data templates of a control are a collection too.
    let templates = (class_markup::<Control>().find_property("DataTemplates").unwrap().get.unwrap())(&[
        into_markup_value(Control::new()),
    ])
    .unwrap();
    assert!(from_markup_value::<crate::templates::DataTemplates>(&templates).is_some());

    // A native menu item from its header; the items of a native menu.
    let item = class_markup::<crate::NativeMenuItem>();
    let value = (item.constructors[0].invoke)(&[text("Open")]).unwrap();
    assert_eq!(unbox::<Ref<crate::NativeMenuItem>>(&value).header().as_deref(), Some("Open"));
    let menu = crate::NativeMenu::new();
    let items = (class_markup::<crate::NativeMenu>().find_property("Items").unwrap().get.unwrap())(&[
        into_markup_value(menu.clone()),
    ])
    .unwrap();
    let list = MarkupType::find_by_handle(ValueType::of_value(&**items.as_ref().unwrap()).id()).unwrap();
    assert_eq!(list.generic.unwrap().definition, "FerroList`1");
    (list.find_methods("Add").next().unwrap().invoke)(&[items, value]).unwrap();
    assert_eq!(menu.items().count(), 1);
}

#[test]
fn typed_routed_event_arguments_of_this_crate_are_declared() {
    crate::register_types();
    let size_changed = MarkupType::find("FerroUI.Controls", "SizeChangedEventArgs").unwrap();
    assert!(size_changed.find_property("NewSize").is_some());
    assert!(size_changed.find_property("PreviousSize").is_some());
    assert_eq!((size_changed.base.unwrap())(), ValueType::of::<Rc<dyn ferroui_base::interactivity::IRoutedEventArgs>>());
    // The base arguments are declared by the base crate only.
    let base = MarkupType::find("FerroUI.Interactivity", "RoutedEventArgs").unwrap();
    assert!(base.module_path.starts_with("ferroui_base"));
}

#[test]
fn data_templates_give_back_their_concrete_type_and_the_platform_can_be_mocked() {
    let template = crate::templates::FuncDataTemplate::new(|_| true, |_, _| None, false);
    let contract: Rc<dyn crate::templates::IDataTemplate> = template.clone();
    assert!(std::ptr::eq(
        contract.as_any().unwrap().downcast_ref::<crate::templates::FuncDataTemplate>().unwrap(),
        &*template
    ));

    let _app = crate::testing::UnitTestApplication::start(crate::testing::TestServices::mock_platform_wrapper());
    let platform = ferroui_base::FerroLocator::current_mutable()
        .get_service::<dyn ferroui_base::platform::IRuntimePlatform>()
        .expect("the mocked runtime platform");
    assert_eq!(platform.get_runtime_info(), ferroui_base::platform::RuntimePlatformInfo::default());
}

#[test]
fn the_methods_of_a_text_box_are_declared() {
    // A binding path that ends in one of them is a command (`{Binding $parent[TextBox].Cut}`).
    let markup = class_markup::<crate::TextBox>();
    for name in ["ClearSelection", "Cut", "Copy", "Paste", "Clear", "SelectAll", "Undo", "Redo"] {
        let methods: Vec<_> = markup.find_methods(name).collect();
        assert_eq!(methods.len(), 1, "{name}");
        assert!(!methods[0].is_static, "{name}");
        assert!(methods[0].parameters.is_empty(), "{name}");
        assert!(methods[0].return_type.is_none(), "{name}");
    }
    let scroll_to_line = markup.find_methods("ScrollToLine").next().expect("ScrollToLine");
    assert_eq!(scroll_to_line.parameters.iter().map(|p| p()).collect::<Vec<_>>(), [ValueType::of::<i32>()]);

    let _app = crate::testing::UnitTestApplication::start(crate::testing::TestServices::styled_window());
    let text_box = crate::TextBox::new();
    text_box.set_text(Some("text"));
    let clear = markup.find_methods("Clear").next().expect("Clear");
    (clear.invoke)(&[into_markup_value(text_box.clone())]).unwrap();
    assert!(text_box.text().unwrap_or_default().is_empty());
}

#[test]
fn attached_accessors_take_the_element_type_the_managed_original_declares() {
    crate::register_types();
    // `TextElement.FontSize` is registered for text elements; its accessors take a control.
    let text_element = crate::documents::TextElement::TYPE.markup().unwrap();
    let set = text_element.find_methods("SetFontSize").next().unwrap();
    assert!(set.is_static);
    assert_eq!((set.parameters[0])(), ValueType::of::<Ref<Control>>());
    assert_eq!((set.parameters[1])(), ValueType::of::<f64>());
    let control = ContentControl::new();
    let instance = into_markup_value(control.clone());
    (set.invoke)(&[instance.clone(), boxed(21.0f64)]).unwrap();
    assert_eq!(crate::documents::TextElement::get_font_size(&control), 21.0);
    let get = text_element.find_methods("GetFontSize").next().unwrap();
    assert_eq!((get.parameters[0])(), ValueType::of::<Ref<Control>>());
    assert_eq!(unbox::<f64>(&(get.invoke)(&[instance.clone()]).unwrap()), 21.0);
    for name in ["FontFamily", "FontFeatures", "FontStyle", "FontWeight", "FontStretch", "Foreground"] {
        assert!(text_element.find_methods(&format!("Set{name}")).next().is_some(), "{name}");
        assert!(text_element.find_methods(&format!("Get{name}")).next().is_some(), "{name}");
    }

    // `Canvas.Left` is registered for controls; its accessors take any object.
    let canvas = crate::Canvas::TYPE.markup().unwrap();
    let set = canvas.find_methods("SetLeft").next().unwrap();
    assert_eq!((set.parameters[0])(), ValueType::of::<Ref<ferroui_base::FerroObject>>());
    (set.invoke)(&[instance, boxed(4.0f64)]).unwrap();
    assert_eq!(crate::Canvas::get_left(&control), 4.0);
}

#[test]
fn the_lists_of_this_crate_have_a_capacity_reached_through_the_named_collections() {
    for type_ in declared_types() {
        let Some(generic) = type_.generic.filter(|generic| generic.definition == "FerroList`1") else { continue };
        let capacity = type_.find_property("Capacity").unwrap_or_else(|| panic!("{}: no Capacity", type_.name));
        assert_eq!((capacity.type_)(), ValueType::of::<i32>());
        assert_eq!((type_.find_methods("Add").next().unwrap().parameters[0])(), (generic.arguments[0])());
    }
    let controls = <crate::Controls as MarkupTyped>::MARKUP;
    let list = MarkupType::find_by_handle((controls.base.unwrap())().id()).unwrap();
    let capacity = list.find_property("Capacity").unwrap();
    let collection = (controls.constructors[0].invoke)(&[]).unwrap();
    (capacity.set.unwrap())(&[collection.clone(), boxed(4i32)]).unwrap();
    assert_eq!(unbox::<i32>(&(capacity.get.unwrap())(&[collection.clone()]).unwrap()), 4);
    (controls.find_methods("Add").next().unwrap().invoke)(&[collection.clone(), into_markup_value(Border::new())]).unwrap();
    assert!((capacity.set.unwrap())(&[collection.clone(), boxed(0i32)]).is_err());
    for named in [
        ValueType::of::<crate::RowDefinitions>(),
        ValueType::of::<crate::ColumnDefinitions>(),
        ValueType::of::<InlineCollection>(),
        ValueType::of::<crate::templates::DataTemplates>(),
        ValueType::of::<TickList>(),
    ] {
        let base = (MarkupType::find_by_handle(named.id()).unwrap().base.unwrap())();
        assert!(ValueTypes::is_assignable(named, base), "{named} -> {base}");
    }
}

#[test]
fn a_tick_list_is_filled_through_the_list_of_numbers_it_derives_from() {
    use ferroui_base::media::MediaCollection;

    crate::register_types();
    let ticks = <TickList as MarkupTyped>::MARKUP;
    let base = (ticks.base.unwrap())();
    assert_eq!(base, ValueType::of::<MediaCollection<f64>>());
    assert!(ValueTypes::is_assignable(ValueType::of::<TickList>(), base));

    // What the loader does for `Ticks="0,20,25"`: the constructor of the tick list, then
    // the members of the list of numbers with the tick list as the instance.
    let list = MarkupType::find_by_handle(base.id()).unwrap();
    let instance = (ticks.constructors[0].invoke)(&[]).unwrap();
    (list.find_property("Capacity").unwrap().set.unwrap())(&[instance.clone(), boxed(3i32)]).unwrap();
    let add = list.find_methods("Add").next().unwrap();
    for tick in [0.0f64, 20.0, 25.0] {
        (add.invoke)(&[instance.clone(), boxed(tick)]).unwrap();
    }
    let ticks = unbox::<TickList>(&instance);
    assert_eq!(ticks.to_vec(), [0.0, 20.0, 25.0]);
    // The value of a `Ticks` property.
    assert!(unbox::<Option<TickList>>(&instance) == Some(ticks));
}

#[test]
fn the_styled_window_services_lay_out_the_text_of_a_shown_window() {
    use ferroui_base::platform::{IFontManagerImpl, IPlatformRenderInterface, ITextShaperImpl};

    let _app = crate::testing::UnitTestApplication::start(crate::testing::TestServices::styled_window());
    let locator = ferroui_base::FerroLocator::current_mutable();
    assert!(locator.get_service::<dyn IFontManagerImpl>().is_some());
    assert!(locator.get_service::<dyn ITextShaperImpl>().is_some());
    assert!(locator.get_service::<dyn IPlatformRenderInterface>().is_some());

    let window = Window::new();
    let text_block = crate::TextBlock::new();
    text_block.set_text(Some("Hello"));
    window.set_content(Some(Control::boxed(text_block.clone())));
    window.show();
    assert!(text_block.bounds().width > 0.0);
    window.close();
}

#[test]
fn a_template_is_the_value_of_a_setter_of_a_property_that_holds_templates() {
    use crate::templates::{FuncTemplate, ITemplateOf};
    use ferroui_base::styling::{ITemplate, Setter, SetterValue};

    crate::register_types();
    // A typed template is a template.
    let typed = MarkupType::find_by_handle(std::any::TypeId::of::<Rc<dyn ITemplateOf<Option<Ref<Control>>>>>()).unwrap();
    assert!(typed.interfaces.iter().any(|i| i() == ValueType::of::<Rc<dyn ITemplate>>()));
    let template: Rc<dyn ITemplateOf<Option<Ref<Control>>>> = FuncTemplate::new(|| None);
    assert!(from_markup_value::<Rc<dyn ITemplate>>(&boxed(template.clone())).is_some());

    let value = <Setter as MarkupTyped>::MARKUP.find_property("Value").unwrap();
    let setter = Setter::empty();
    let instance = into_markup_value(setter.clone());
    // The focus adorner is a template: the setter holds it as the value.
    setter.set_property(Some(Control::focus_adorner_property().as_property()));
    (value.set.unwrap())(&[instance.clone(), boxed(Some(template.clone()))]).unwrap();
    assert!(matches!(setter.value(), Some(SetterValue::Value(_))));
    // The content is any object: the setter builds it from the template.
    setter.set_property(Some(ContentControl::content_property().as_property()));
    (value.set.unwrap())(&[instance, boxed(template)]).unwrap();
    assert!(matches!(setter.value(), Some(SetterValue::Template(_))));
}

#[test]
fn a_func_control_template_gives_back_its_concrete_type() {
    use crate::templates::{FuncControlTemplate, IControlTemplate};

    let template = FuncControlTemplate::new(|_, _| Border::new().upcast());
    let contract: Rc<dyn IControlTemplate> = template.clone();
    assert!(std::ptr::eq(contract.as_any().unwrap().downcast_ref::<FuncControlTemplate>().unwrap(), &*template));
}

#[test]
fn converters_are_created_configured_and_assignable_to_their_contract() {
    use crate::converters::*;
    use ferroui_base::data::converters::{IMultiValueConverter, IValueConverter};

    crate::register_types();
    let find = |name: &str| MarkupType::find("FerroUI.Controls.Converters", name).unwrap_or_else(|| panic!("{name}"));
    // Every converter has its parameterless constructor and is its contract.
    for name in [
        "CornerRadiusFilterConverter",
        "CornerRadiusToDoubleConverter",
        "EnumToBoolConverter",
        "MarginMultiplierConverter",
        "PlatformKeyGestureConverter",
    ] {
        let value = (find(name).constructors.iter().find(|c| c.parameters.is_empty()).unwrap().invoke)(&[]).unwrap();
        assert!(from_markup_value::<Rc<dyn IValueConverter>>(&value).is_some(), "{name}");
        assert!(from_markup_value::<Option<Rc<dyn IValueConverter>>>(&value).is_some_and(|c| c.is_some()), "{name}");
    }
    for name in
        ["BorderGapMaskConverter", "MenuScrollingVisibilityConverter", "StringFormatConverter", "TreeViewItemIndentConverter"]
    {
        let value = (find(name).constructors.iter().find(|c| c.parameters.is_empty()).unwrap().invoke)(&[]).unwrap();
        assert!(from_markup_value::<Rc<dyn IMultiValueConverter>>(&value).is_some(), "{name}");
    }

    // The settable properties.
    let margin = find("MarginMultiplierConverter");
    let converter = (margin.constructors[0].invoke)(&[]).unwrap();
    for name in ["Left", "Top", "Right", "Bottom"] {
        (margin.find_property(name).unwrap().set.unwrap())(&[converter.clone(), boxed(true)]).unwrap();
    }
    (margin.find_property("Indent").unwrap().set.unwrap())(&[converter.clone(), boxed(2.0f64)]).unwrap();
    let concrete = unbox::<Rc<MarginMultiplierConverter>>(&converter);
    assert!(concrete.left() && concrete.top() && concrete.right() && concrete.bottom());
    assert_eq!(concrete.indent(), 2.0);
    let filter = find("CornerRadiusFilterConverter");
    let converter = (filter.constructors[0].invoke)(&[]).unwrap();
    let corners = Corners::TOP_LEFT | Corners::TOP_RIGHT;
    (filter.find_property("Filter").unwrap().set.unwrap())(&[converter.clone(), boxed(corners)]).unwrap();
    assert_eq!(unbox::<Rc<CornerRadiusFilterConverter>>(&converter).filter(), corners);
    assert!(filter.find_property("Scale").is_some());
    assert!(find("CornerRadiusToDoubleConverter").find_property("Corner").is_some());

    // The singletons are static fields, assignable to a converter property.
    for name in ["MenuScrollingVisibilityConverter", "TreeViewItemIndentConverter"] {
        let instance = (find(name).find_field("Instance").unwrap().get)();
        assert!(from_markup_value::<Rc<dyn IMultiValueConverter>>(&instance).is_some(), "{name}");
    }
    assert!(Rc::ptr_eq(
        &unbox::<Rc<MenuScrollingVisibilityConverter>>(&(find("MenuScrollingVisibilityConverter").find_field("Instance").unwrap().get)()),
        &MenuScrollingVisibilityConverter::instance()
    ));
}

#[test]
fn the_collection_handles_of_items_controls_are_known_to_the_value_conversions() {
    crate::register_types();
    // On a thread that registered nothing itself.
    std::thread::spawn(|| {
        assert!(ValueTypes::accepts_null(ValueType::of::<Option<crate::ItemsSource>>()));
        assert!(ValueTypes::accepts_null(ValueType::of::<Option<crate::primitives::SelectedItemsList>>()));
        assert!(ValueTypes::is_assignable(
            ValueType::of::<crate::ItemsSource>(),
            ValueType::of::<Option<crate::ItemsSource>>()
        ));
    })
    .join()
    .unwrap();
}

#[test]
fn the_collection_handle_of_items_controls_is_the_enumerable_of_the_runtime_library() {
    use ferroui_base::collections::FerroList;

    crate::register_types();
    let enumerable = MarkupType::find("System.Collections", "IEnumerable").expect("the enumerable");
    assert!(enumerable.kind == MarkupTypeKind::Interface);
    assert!(std::ptr::eq(enumerable, <crate::ItemsSource as MarkupTyped>::MARKUP));
    assert!(std::ptr::eq(MarkupType::find_by_handle(std::any::TypeId::of::<crate::ItemsSource>()).unwrap(), enumerable));

    // A list of untyped items (what the run-time loader creates for a `List<T>` or an
    // `ArrayList` of markup) is assignable to an items source, which shares the list.
    let list = Rc::new(FerroList::<Option<BoxedValue>>::new());
    list.add(None);
    list.add(boxed("Hello".to_string()));
    let source = unbox::<Option<crate::ItemsSource>>(&boxed(list.clone())).expect("an items source");
    assert_eq!(source.count(), 2);
    assert!(source.get_at(0).is_none());
    list.add(boxed("World".to_string()));
    assert_eq!(source.count(), 3);
    assert!(source.downcast_ref::<FerroList<Option<BoxedValue>>>().is_some_and(|shared| *shared == *list));
}

#[test]
fn a_vector_of_untyped_values_converts_to_an_items_source() {
    crate::register_types();
    // The errors of a control are such a vector; a binding delivers it to the items source
    // of the items control of their template.
    let errors: Vec<BoxedValue> = vec![Rc::new("First".to_string()), Rc::new(2i32)];
    let value: BoxedValue = Rc::new(errors);
    for target in [ValueType::of::<crate::ItemsSource>(), ValueType::of::<Option<crate::ItemsSource>>()] {
        let converted = ValueTypes::try_convert(Some(&value), target).flatten().expect("an items source");
        assert_eq!(ValueType::of_value(&*converted), target);
        assert!(ValueTypes::is_assignable(ValueType::of::<Vec<BoxedValue>>(), target));
    }
    let converted = ValueTypes::try_convert(Some(&value), ValueType::of::<Option<crate::ItemsSource>>()).flatten();
    let source = converted
        .and_then(|converted| converted.downcast_ref::<Option<crate::ItemsSource>>().cloned())
        .flatten()
        .expect("an items source");
    assert_eq!(source.count(), 2);
    let first = source.get_at(0).expect("the first item");
    assert_eq!(first.downcast_ref::<String>().map(String::as_str), Some("First"));
    let second = source.get_at(1).expect("the second item");
    assert_eq!(second.downcast_ref::<i32>(), Some(&2));
    // Markup assigns it the same way.
    assert_eq!(unbox::<Option<crate::ItemsSource>>(&Some(value.clone())).expect("an items source").count(), 2);

    // The vector whose items may be null.
    let items: Vec<Option<BoxedValue>> = vec![None, Some(Rc::new(1i32))];
    let value: BoxedValue = Rc::new(items);
    let converted = ValueTypes::try_convert(Some(&value), ValueType::of::<Option<crate::ItemsSource>>()).flatten();
    let source = converted
        .and_then(|converted| converted.downcast_ref::<Option<crate::ItemsSource>>().cloned())
        .flatten()
        .expect("an items source");
    assert_eq!(source.count(), 2);
    assert!(source.get_at(0).is_none());
    assert!(source.get_at(1).is_some());
    let converted = ValueTypes::try_convert(Some(&value), ValueType::of::<crate::ItemsSource>()).flatten();
    assert!(converted.is_some_and(|converted| converted.is::<crate::ItemsSource>()));
}

#[test]
fn every_value_type_has_its_default_value_constructor() {
    for type_ in declared_types() {
        if type_.kind != MarkupTypeKind::Struct {
            continue;
        }
        let constructor = type_
            .constructors
            .iter()
            .find(|constructor| constructor.parameters.is_empty())
            .unwrap_or_else(|| panic!("{}: no parameterless constructor", type_.full_name()));
        let value = (constructor.invoke)(&[]).unwrap().unwrap();
        assert!(is_value_of(&value, type_), "{}", type_.full_name());
    }
}

#[test]
fn automation_properties_and_their_enumerations_are_known_to_markup() {
    use crate::automation::peers::{AutomationControlType, AutomationLandmarkType};
    use crate::automation::{AccessibilityView, AutomationLiveSetting, AutomationProperties, IsOffscreenBehavior};

    crate::register_types();
    // The enumerations are found by name; the peers namespace is not an XML namespace of the framework.
    for (namespace, name, member) in [
        ("FerroUI.Automation", "AccessibilityView", "Content"),
        ("FerroUI.Automation", "AutomationLiveSetting", "Assertive"),
        ("FerroUI.Automation", "IsOffscreenBehavior", "Offscreen"),
        ("FerroUI.Automation.Peers", "AutomationControlType", "Button"),
        ("FerroUI.Automation.Peers", "AutomationLandmarkType", "Navigation"),
    ] {
        let type_ = MarkupType::find(namespace, name).unwrap_or_else(|| panic!("{namespace}.{name}"));
        assert_eq!(type_.kind, MarkupTypeKind::Enum);
        assert!(type_.find_enum_member(member, false).is_some(), "{name}.{member}");
    }
    assert_eq!(<AutomationControlType as MarkupTyped>::MARKUP.enum_members.len(), 43);
    let namespaces: Vec<_> = crate::register_types::ASSEMBLY.xmlns_definitions.iter().map(|d| d.namespace).collect();
    assert!(namespaces.contains(&"FerroUI.Automation") && !namespaces.contains(&"FerroUI.Automation.Peers"));
    // The nullable forms of the enumerations are values of the properties that hold them.
    for nullable in [
        ValueType::of::<Option<AccessibilityView>>(),
        ValueType::of::<Option<AutomationLiveSetting>>(),
        ValueType::of::<Option<IsOffscreenBehavior>>(),
        ValueType::of::<Option<AutomationControlType>>(),
        ValueType::of::<Option<AutomationLandmarkType>>(),
    ] {
        assert!(ValueTypes::accepts_null(nullable), "{nullable}");
    }

    // The class of the attached properties is found by name with its properties, and a
    // property that names another element takes the handle of the element.
    let class = TypeInfo::find("FerroUI.Automation", "AutomationProperties").expect("the static class");
    assert!(std::ptr::eq(class, AutomationProperties::TYPE));
    let labeled_by = AutomationProperties::labeled_by_property().as_property();
    let label = Border::new();
    let target = Button::new();
    let handle: BoxedValue = Rc::new(label.clone().upcast::<Control>());
    target.set_value_untyped(labeled_by, handle.as_any(), ferroui_base::data::BindingPriority::LocalValue);
    assert_eq!(AutomationProperties::get_labeled_by(&target), Some(label.upcast::<Control>()));
}

#[test]
fn a_null_setter_value_clears_a_template_property() {
    use ferroui_base::diagnostics::FerroObjectDiagnosticExtensions;
    use ferroui_base::styling::testing::try_attach;
    use ferroui_base::styling::{Selectors, Setter, Style};

    crate::register_types();
    // `<Setter Property='FocusAdorner' Value='{x:Null}'/>` and `Background='{x:Null}'`.
    for property in [Control::focus_adorner_property().as_property(), Border::background_property().as_property()] {
        let setter = Setter::empty();
        setter.set_property(Some(property));
        let value = <Setter as MarkupTyped>::MARKUP.find_property("Value").unwrap();
        (value.set.unwrap())(&[into_markup_value(setter.clone()), None]).unwrap();
        let style = Style::with_selector(Selectors::of_type::<Border>());
        style.setters().add(setter);
        let border = Border::new();
        try_attach(&style, &border, None);
        let diagnostic = border.get_diagnostic(property);
        assert_eq!(diagnostic.priority(), ferroui_base::data::BindingPriority::Style, "{}", property.name());
        assert!(ValueTypes::normalize(diagnostic.value().clone()).is_none(), "{}", property.name());
    }
}


#[test]
fn the_delegate_of_a_method_is_a_custom_popup_placement_callback() {
    crate::register_types();
    let markup = <CustomPopupPlacementCallbackValue as MarkupTyped>::MARKUP;
    assert_eq!(markup.full_name(), "FerroUI.Controls.Primitives.PopupPositioning.CustomPopupPlacementCallback");
    // A delegate type: markup looks for a method of the root object with the signature of `Invoke`.
    assert_eq!(markup.base.map(|base| base()), Some(ValueType::of::<MarkupDelegate>()));
    let invoke: Vec<_> = markup.find_methods("Invoke").collect();
    assert_eq!(invoke.len(), 1);
    assert!(invoke[0].return_type.is_none());
    assert_eq!((invoke[0].parameters[0])(), ValueType::of::<Rc<RefCell<CustomPopupPlacement>>>());

    // The delegate of a method that changes the parameters it is called with.
    let method = MarkupDelegate::new(|arguments| {
        let placement = unbox::<Rc<RefCell<CustomPopupPlacement>>>(&arguments[0]);
        placement.borrow_mut().set_anchor(PopupAnchor::TOP);
        placement.borrow_mut().offset = Point::new(1.0, 2.0);
        None
    });
    let callback = unbox::<Option<CustomPopupPlacementCallbackValue>>(&boxed(method)).expect("a callback");
    let target: Ref<Visual> = Border::new().upcast();
    let mut placement = CustomPopupPlacement::new(Size::new(10.0, 20.0), Thickness::default(), target);
    (callback.0)(&mut placement);
    assert_eq!(placement.anchor(), PopupAnchor::TOP);
    assert_eq!(placement.offset, Point::new(1.0, 2.0));
    assert_eq!(placement.popup_size(), Size::new(10.0, 20.0));

    // The callback is called through its `Invoke` as well, with the shared form of the parameters.
    let shared = Rc::new(RefCell::new(placement));
    (invoke[0].invoke)(&[boxed(callback.clone()), boxed(shared.clone())]).unwrap();
    assert_eq!(shared.borrow().offset, Point::new(1.0, 2.0));
}
