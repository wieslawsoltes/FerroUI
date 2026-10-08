//! Tests specific to this port: the markup metadata declared in this module
//! is consistent and its members work when invoked through metadata only.

use super::TYPE_LISTS;
use crate::animation::{KeyFrame, KeySpline, TimeSpan, Transitions};
use crate::controls::{NameScope, NameScopeRef, ResourceDictionary, ResourceKey};
use crate::data::core::{ValueType, ValueTypes};
use crate::data::RelativeSourceMode;
use crate::input::{DragDrop, InputElement, Key, KeyGesture, KeyModifiers, TappedEventArgs};
use crate::interactivity::RoutedEvent;
use crate::media::{
    Brushes, Color, Colors, DashStyle, Geometry, GradientStop, GradientStops, IBrush, IDashStyle, PathFigures,
    SolidColorBrush,
};
use crate::metadata::{
    attributes, from_markup_value, into_markup_value, MarkupAttributeValue, MarkupDelegate, MarkupProperty,
    MarkupType, MarkupTypeKind, MarkupTyped, MarkupValue,
};
use crate::styling::{ControlTheme, Style, Styles};
use crate::{
    BoxedValue, FerroObject, OwnedFerroPropertyChangedEventArgs, Ref, StaticType, StyledElement, Thickness, TypeInfo,
};
use std::cell::{Cell, RefCell};
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
        .filter(|type_| type_.module_path().starts_with("ferroui_base"))
        .filter_map(|type_| type_.markup().map(|markup| (type_, markup)))
        .filter(|(_, markup)| markup.module_path.starts_with("ferroui_base::markup_types"))
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
        include_str!("named_values.rs"),
        include_str!("contracts.rs"),
        include_str!("plain.rs"),
        include_str!("classes.rs"),
        include_str!("well_known.rs"),
        include_str!("animation.rs"),
    ];
    let declared: usize = sources
        .iter()
        .flat_map(|source| source.lines())
        .filter(|line| {
line.starts_with("ferro_markup_type!(") || line.starts_with("ferro_markup_enum!(")
        })
        .count();
    let types = declared_types();
    // `Decimal` and `NumberStyles` declare their metadata next to the type and are
    // registered with the values.
    assert_eq!(types.len(), declared + 2);

    for type_ in &types {
        let found = MarkupType::find(type_.namespace(), type_.name)
            .unwrap_or_else(|| panic!("{} is registered", type_.full_name()));
        // The instantiations of a generic type share its name.
        assert!(std::ptr::eq(found, *type_) || type_.generic.is_some(), "{} is declared twice", type_.full_name());
        assert!(type_.namespace() == "System" || type_.namespace().starts_with("System.") || type_.namespace().starts_with("FerroUI"));
    }
}

#[test]
fn handles_resolve_to_their_type() {
    for type_ in declared_types() {
        match type_.kind {
            MarkupTypeKind::Static => assert!(type_.handles.is_empty(), "{type_:?}"),
            _ => assert!(!type_.handles.is_empty(), "{type_:?} has no handle"),
        }
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
    assert!(classes.len() > 40, "{}", classes.len());
    for (type_, markup) in classes {
        assert_eq!(markup.name, type_.name());
        assert_eq!(markup.namespace(), type_.namespace(), "{type_}");
        assert!(std::ptr::eq((markup.type_info.unwrap())(), type_));
        assert_eq!(markup.handle().map(|handle| handle.id()), type_.handle());
    }
}

#[test]
fn content_properties_and_dependencies_name_existing_properties() {
    for (type_, markup) in class_markups() {
        if let Some(content) = markup.content_property {
            assert!(
                has_registered_property(type_, content) || find_plain_property(type_, content).is_some(),
                "{type_}: content property {content}"
            );
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
    for type_ in declared_types() {
        if let Some(content) = type_.content_property {
            assert!(type_.find_property(content).is_some(), "{type_:?}: content property {content}");
        }
        for property in type_.properties {
            for attribute in property.attributes.iter().filter(|a| a.name == attributes::DEPENDS_ON) {
                let MarkupAttributeValue::Str(target) = attribute.arguments[0] else { panic!("{type_:?}") };
                assert!(type_.find_property(target).is_some(), "{type_:?}.{}: depends on {target}", property.name);
            }
        }
    }
}

#[test]
fn enumeration_members_are_found_by_name() {
    let enums: Vec<_> = declared_types().into_iter().filter(|t| t.kind == MarkupTypeKind::Enum).collect();
    // `NumberStyles` is declared next to the type.
    assert_eq!(enums.len(), super::enums::TYPES.len() + 1);
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
    assert!(count >= 8, "{count}");
    for (type_, markup) in class_markups() {
        for constructor in markup.constructors {
            assert!(!constructor.parameters.is_empty(), "{type_}: the parameterless constructor is `new:`");
        }
    }
}

/// A text each type with a `Parse` accepts.
const PARSE_SAMPLES: &[(&str, &str)] = &[
    ("FerroUI.Media.Transformation.TransformOperations", "scaleX(0.125) translateX(-2px)"),
    ("FerroUI.Media.ITransform", "rotate(45deg)"),
    ("System.DateTime", "2024-03-05 14:30:15"),
    ("System.DateTimeOffset", "2024-03-05T14:30:15+02:00"),
    ("System.Decimal", "1.5"),
    ("FerroUI.Controls.Classes", "a b"),
    ("FerroUI.Thickness", "1,2,3,4"),
    ("FerroUI.Point", "1,2"),
    ("FerroUI.Vector", "1,2"),
    ("FerroUI.Size", "1,2"),
    ("FerroUI.Rect", "1,2,3,4"),
    ("FerroUI.Matrix", "1,0,0,1,0,0"),
    ("FerroUI.CornerRadius", "1,2,3,4"),
    ("FerroUI.RelativePoint", "50%,50%"),
    ("FerroUI.RelativeRect", "0%,0%,100%,100%"),
    ("FerroUI.RelativeScalar", "50%"),
    ("FerroUI.PixelPoint", "1,2"),
    ("FerroUI.PixelSize", "1,2"),
    ("FerroUI.PixelRect", "1,2,3,4"),
    ("FerroUI.Vector3D", "1,2,3"),
    ("FerroUI.Media.Color", "#FF102030"),
    ("FerroUI.Media.HslColor", "hsl(120, 50%, 50%)"),
    ("FerroUI.Media.HsvColor", "hsv(120, 50%, 50%)"),
    ("FerroUI.Media.FontWeight", "Bold"),
    ("FerroUI.Media.FontFamily", "Arial"),
    ("FerroUI.Media.FontFeature", "+kern"),
    ("FerroUI.Media.FontFeatureCollection", "+kern, -liga"),
    ("FerroUI.Media.TextDecorationCollection", "Underline"),
    ("FerroUI.Media.BoxShadow", "5 5 10 0 Red"),
    ("FerroUI.Media.BoxShadows", "5 5 10 0 Red, inset 1 1 2 0 Blue"),
    ("FerroUI.Media.IBrush", "Red"),
    ("FerroUI.Media.IEffect", "blur(5)"),
    ("FerroUI.Media.TextTrimming", "CharacterEllipsis"),
    ("FerroUI.Media.PathFigures", "M0,0 L10,10"),
    ("FerroUI.Input.KeyGesture", "Ctrl+A"),
    ("FerroUI.Animation.Cue", "50%"),
    ("FerroUI.Animation.IterationCount", "Infinite"),
    ("FerroUI.Animation.Easings.Easing", "LinearEasing"),
    ("FerroUI.Styling.ThemeVariant", "Dark"),
    ("System.TimeSpan", "0:0:1"),
    ("System.Uri", "https://example.org/a"),
    ("System.Globalization.CultureInfo", "de-DE"),
];

/// The types whose `Parse` needs the services of a platform.
const PARSE_NEEDS_PLATFORM: &[&str] = &["FerroUI.Input.Cursor"];

#[test]
fn parse_converts_a_sample_of_each_text_convertible_type() {
    let mut parsed = 0;
    for type_ in declared_types() {
        let Some(parse) = type_.parse else { continue };
        let name = type_.full_name();
        if PARSE_NEEDS_PLATFORM.contains(&name.as_str()) {
            continue;
        }
        let sample = PARSE_SAMPLES
            .iter()
            .find(|(type_name, _)| *type_name == name)
            .unwrap_or_else(|| panic!("a sample text for {name}"))
            .1;
        let value = parse(&[text(sample)]).unwrap_or_else(|e| panic!("{name}: {e}")).unwrap();
        assert!(is_value_of(&value, type_), "{name}: {}", value.type_name());
        // The same text gives the same value.
        if type_.kind == MarkupTypeKind::Struct {
            let again = parse(&[text(sample)]).unwrap();
            assert!(ValueTypes::identity_equals(Some(&value), again.as_ref()), "{name}");
        }
        assert!(parse(&[boxed(1i32)]).is_err(), "{name}: only text is parsed");
        parsed += 1;
    }
    assert_eq!(parsed, PARSE_SAMPLES.len());
}

// Values and their text forms.

#[test]
fn value_types_parse_and_construct_through_metadata() {
    crate::register_types();
    let thickness = <Thickness as MarkupTyped>::MARKUP;
    assert_eq!(thickness.full_name(), "FerroUI.Thickness");
    let parsed = (thickness.parse.unwrap())(&[text("1,2,3,4")]).unwrap();
    assert_eq!(unbox::<Thickness>(&parsed), Thickness::new(1.0, 2.0, 3.0, 4.0));
    // The text form of the value parses to the value again.
    let round_trip = (thickness.parse.unwrap())(&[text(&unbox::<Thickness>(&parsed).to_string())]).unwrap();
    assert_eq!(unbox::<Thickness>(&round_trip), Thickness::new(1.0, 2.0, 3.0, 4.0));
    assert!((thickness.parse.unwrap())(&[text("x")]).is_err());

    let uniform = thickness.constructors.iter().find(|c| c.parameters.len() == 1).unwrap();
    assert_eq!(unbox::<Thickness>(&(uniform.invoke)(&[boxed(2.0f64)]).unwrap()), Thickness::uniform(2.0));
    assert_eq!(unbox::<Option<Thickness>>(&parsed), Some(Thickness::new(1.0, 2.0, 3.0, 4.0)));
    // The components are read-only properties.
    let components: Vec<f64> = ["Left", "Top", "Right", "Bottom"]
        .iter()
        .map(|name| {
            let property = thickness.find_property(name).unwrap();
            assert!(property.set.is_none());
            unbox::<f64>(&(property.get.unwrap())(&[parsed.clone()]).unwrap())
        })
        .collect();
    assert_eq!(components, [1.0, 2.0, 3.0, 4.0]);

    let color = <Color as MarkupTyped>::MARKUP;
    let parsed = (color.parse.unwrap())(&[text("Red")]).unwrap();
    assert_eq!(unbox::<Color>(&parsed), Colors::RED);
    let from_rgb = color.find_methods("FromRgb").next().unwrap();
    assert!(from_rgb.is_static);
    let value = (from_rgb.invoke)(&[boxed(255u8), boxed(0u8), boxed(0u8)]).unwrap();
    assert_eq!(unbox::<Color>(&value), Colors::RED);

    let time_span = <TimeSpan as MarkupTyped>::MARKUP;
    assert_eq!(time_span.namespace(), "System");
    let parsed = (time_span.parse.unwrap())(&[text("0:0:1.5")]).unwrap();
    assert_eq!(unbox::<TimeSpan>(&parsed), TimeSpan::from_milliseconds(1500.0));

    let gesture = <KeyGesture as MarkupTyped>::MARKUP;
    let parsed = (gesture.parse.unwrap())(&[text("Ctrl+A")]).unwrap();
    assert_eq!(unbox::<KeyGesture>(&parsed), KeyGesture::new(Key::A, KeyModifiers::CONTROL));
    assert_eq!(unbox::<Option<KeyGesture>>(&parsed), Some(KeyGesture::new(Key::A, KeyModifiers::CONTROL)));
}

#[test]
fn static_values_are_read_through_metadata() {
    crate::register_types();
    let colors = MarkupType::find("FerroUI.Media", "Colors").unwrap();
    assert_eq!(colors.kind, MarkupTypeKind::Static);
    // The named values are static properties, as in the managed original.
    assert!(colors.fields.is_empty());
    assert_eq!(colors.static_properties.len(), 141);
    assert_eq!(unbox::<Color>(&(colors.find_static_property("CornflowerBlue").unwrap().get.unwrap())(&[]).unwrap()), Colors::CORNFLOWER_BLUE);

    // A named brush is declared as the managed original declares it (an immutable solid
    // color brush, which has `Color`) and is assignable to a brush-typed property.
    let brushes = MarkupType::find("FerroUI.Media", "Brushes").unwrap();
    assert!(brushes.find_field("Red").is_none());
    let field = brushes.find_static_property("Red").unwrap();
    assert_eq!((field.type_)(), ValueType::of::<Rc<dyn crate::media::IImmutableSolidColorBrush>>());
    let red = (field.get.unwrap())(&[]).unwrap();
    let declared = MarkupType::find_by_handle((field.type_)().id()).unwrap();
    assert_eq!((declared.namespace(), declared.name), ("FerroUI.Media", "IImmutableSolidColorBrush"));
    let solid = MarkupType::find("FerroUI.Media", "ISolidColorBrush").unwrap();
    assert!(declared.interfaces.iter().any(|i| i() == ValueType::of::<Rc<dyn crate::media::ISolidColorBrush>>()));
    assert!(declared.interfaces.iter().any(|i| i() == ValueType::of::<Rc<dyn IBrush>>()));
    let color = solid.find_property("Color").unwrap();
    assert_eq!((color.type_)(), ValueType::of::<Color>());
    assert_eq!(unbox::<Color>(&(color.get.unwrap())(&[red.clone()]).unwrap()), Colors::RED);
    let opacity = <dyn IBrush as MarkupTyped>::MARKUP.find_property("Opacity").unwrap();
    assert_eq!(unbox::<f64>(&(opacity.get.unwrap())(&[red.clone()]).unwrap()), 1.0);
    assert!(ValueTypes::is_assignable((field.type_)(), ValueType::of::<Option<Rc<dyn IBrush>>>()));
    let brush = unbox::<Option<Rc<dyn IBrush>>>(&red).unwrap();
    let expected: Rc<dyn IBrush> = Brushes::red();
    assert!(*brush == *expected);
    let parsed = (<dyn IBrush as MarkupTyped>::MARKUP.parse.unwrap())(&[text("#FFFF0000")]).unwrap();
    assert!(from_markup_value::<Rc<dyn IBrush>>(&parsed).is_some());

    let dash = class_markup::<DashStyle>().find_static_property("Dash").unwrap();
    assert!(from_markup_value::<Rc<dyn IDashStyle>>(&(dash.get.unwrap())(&[]).unwrap()).is_some());
}

#[test]
fn enumerations_follow_the_managed_member_names() {
    crate::register_types();
    let modifiers = <KeyModifiers as MarkupTyped>::MARKUP;
    assert!(modifiers.is_flags);
    assert_eq!(modifiers.namespace(), "FerroUI.Input");
    let members: Vec<(&str, i64)> = modifiers.enum_members.iter().map(|m| (m.name, m.value)).collect();
    assert_eq!(members, [("None", 0), ("Alt", 1), ("Control", 2), ("Shift", 4), ("Meta", 8)]);

    // Aliases of the managed enumeration are members too.
    let key = <Key as MarkupTyped>::MARKUP;
    let enter = key.find_enum_member("Enter", false).unwrap();
    assert_eq!((enter.get)().downcast_ref::<Key>(), Some(&Key::Return));
    assert_eq!(enter.value, key.find_enum_member("Return", false).unwrap().value);

    let mode = <RelativeSourceMode as MarkupTyped>::MARKUP;
    let self_ = mode.find_enum_member("Self", false).unwrap();
    assert_eq!((self_.get)().downcast_ref::<RelativeSourceMode>(), Some(&RelativeSourceMode::SelfMode));
    assert_eq!(unbox::<Option<RelativeSourceMode>>(&Some((self_.get)())), Some(RelativeSourceMode::SelfMode));
}

// Collections.

#[test]
fn collections_are_constructed_and_added_to_through_metadata() {
    crate::register_types();
    let stops = <GradientStops as MarkupTyped>::MARKUP;
    // A named collection is a type of its own that derives from the list of its items.
    assert!(stops.generic.is_none());
    let base = MarkupType::find_by_handle((stops.base.unwrap())().id()).unwrap();
    assert_eq!(base.generic.unwrap().definition, "FerroList`1");
    assert_eq!((base.generic.unwrap().arguments[0])(), ValueType::of::<Ref<GradientStop>>());
    assert!(base.constructors.iter().any(|c| c.parameters.is_empty()) && base.find_methods("Add").next().is_some());
    let points = <crate::media::Points as MarkupTyped>::MARKUP;
    assert_eq!((points.base.unwrap())(), ValueType::of::<crate::collections::FerroList<crate::Point>>());
    let list = <crate::media::MediaCollection<f64> as MarkupTyped>::MARKUP;
    assert_eq!(list.generic.unwrap().definition, "FerroList`1");
    assert_eq!((list.generic.unwrap().arguments[0])(), ValueType::of::<f64>());
    let collection = (stops.constructors[0].invoke)(&[]).unwrap();
    let add = stops.find_methods("Add").next().unwrap();
    (add.invoke)(&[collection.clone(), into_markup_value(GradientStop::with_color_and_offset(Colors::RED, 0.5))])
        .unwrap();
    assert_eq!(unbox::<GradientStops>(&collection).len(), 1);
    // Only a gradient stop can be added.
    assert!((add.invoke)(&[collection.clone(), into_markup_value(Style::new())]).is_err());

    let transitions = <Transitions as MarkupTyped>::MARKUP;
    let collection = (transitions.constructors[0].invoke)(&[]).unwrap();
    assert_eq!(unbox::<Transitions>(&collection).count(), 0);
    assert_eq!(unbox::<Option<Transitions>>(&None), None);

    let figures = <PathFigures as MarkupTyped>::MARKUP;
    let parsed = (figures.parse.unwrap())(&[text("M0,0 L10,10 M5,5 L1,1")]).unwrap();
    assert_eq!(unbox::<PathFigures>(&parsed).len(), 2);
}

#[test]
fn name_scopes_register_and_find_through_metadata() {
    crate::register_types();
    let markup = MarkupType::find("FerroUI.Controls", "INameScope").unwrap();
    let scope = boxed(NameScopeRef::new(NameScope::new()));
    let element = StyledElement::new();

    let register = markup.find_methods("Register").next().unwrap();
    (register.invoke)(&[scope.clone(), text("root"), into_markup_value(element.clone())]).unwrap();
    let find = markup.find_methods("Find").next().unwrap();
    let found = (find.invoke)(&[scope.clone(), text("root")]).unwrap();
    assert_eq!(unbox::<Ref<FerroObject>>(&found), element.upcast::<FerroObject>());
    assert_eq!((find.invoke)(&[scope.clone(), text("other")]).unwrap(), None);

    let completed = markup.find_property("IsCompleted").unwrap().get.unwrap();
    assert!(!unbox::<bool>(&completed(&[scope.clone()]).unwrap()));
    (markup.find_methods("Complete").next().unwrap().invoke)(&[scope.clone()]).unwrap();
    assert!(unbox::<bool>(&completed(&[scope]).unwrap()));
}

// Classes.

#[test]
fn styled_element_members_work_through_metadata() {
    let markup = class_markup::<StyledElement>();
    let element = StyledElement::new();
    let instance = into_markup_value(element.clone());

    // A plain property with a setter, typed with the contract as in the managed original.
    let resources = markup.find_property("Resources").unwrap();
    assert_eq!((resources.type_)(), ValueType::of::<Rc<dyn crate::controls::IResourceDictionary>>());
    let dictionary = ResourceDictionary::new();
    (resources.set.unwrap())(&[instance.clone(), into_markup_value(dictionary.clone())]).unwrap();
    assert_eq!(element.resources(), dictionary);
    let read = (resources.get.unwrap())(&[instance.clone()]).unwrap();
    // The contract adds resources: text keys, and an error for a key that is present.
    let contract = MarkupType::find("FerroUI.Controls", "IResourceDictionary").unwrap();
    let add = contract.find_methods("Add").next().unwrap();
    (add.invoke)(&[read.clone(), text("accent"), boxed(1i32)]).unwrap();
    assert!((add.invoke)(&[read.clone(), text("accent"), boxed(2i32)]).is_err());
    assert_eq!(dictionary.count(), 1);
    let merged = (contract.find_property("MergedDictionaries").unwrap().get.unwrap())(&[read.clone()]).unwrap();
    assert!(from_markup_value::<crate::collections::FerroList<Rc<dyn crate::controls::IResourceProvider>>>(&merged).is_some());
    assert!((contract.find_property("ThemeDictionaries").unwrap().get.unwrap())(&[read]).unwrap().is_some());

    // A read-only collection property.
    let styles = markup.find_property("Styles").unwrap();
    assert!(styles.set.is_none());
    assert_eq!(unbox::<Ref<Styles>>(&(styles.get.unwrap())(&[instance.clone()]).unwrap()), element.styles());

    // A plain `EventHandler` event: the handler receives the sender and empty arguments.
    let raised = Rc::new(Cell::new(0));
    let handler = MarkupDelegate::new({
        let (raised, element) = (raised.clone(), element.clone());
        move |arguments| {
            assert_eq!(unbox::<Ref<StyledElement>>(&arguments[0]), element);
            assert_eq!(unbox::<crate::utilities::EventArgs>(&arguments[1]), crate::utilities::EventArgs::EMPTY);
            raised.set(raised.get() + 1);
            None
        }
    });
    let data_context_changed = markup.find_event("DataContextChanged").unwrap();
    assert_eq!(
        data_context_changed.arguments.iter().map(|a| a()).collect::<Vec<_>>(),
        [ValueType::of::<Option<BoxedValue>>(), ValueType::of::<crate::utilities::EventArgs>()]
    );
    (data_context_changed.add)(&[instance.clone(), boxed(handler)]).unwrap();
    element.set_data_context(Some(Rc::new(1i32) as BoxedValue));
    assert_eq!(raised.get(), 1);

    // An event with arguments passes the instance as the sender.
    let seen = Rc::new(Cell::new(0));
    let handler = MarkupDelegate::new({
        let (seen, element) = (seen.clone(), element.clone());
        move |arguments| {
            assert_eq!(arguments.len(), 2);
            assert_eq!(unbox::<Ref<StyledElement>>(&arguments[0]), element);
            assert!(from_markup_value::<crate::controls::ResourcesChangedEventArgs>(&arguments[1]).is_some());
            seen.set(seen.get() + 1);
            None
        }
    });
    let resources_changed = markup.find_event("ResourcesChanged").unwrap();
    assert_eq!((resources_changed.arguments[0])(), ValueType::of::<Option<BoxedValue>>());
    (resources_changed.add)(&[instance.clone(), boxed(handler)]).unwrap();
    element.resources().add("key", Some(Rc::new(1i32) as BoxedValue));
    assert!(seen.get() >= 1);

    // The initialisation protocol is a contract the class implements.
    let initialize = MarkupType::find("System.ComponentModel", "ISupportInitialize").unwrap();
    let contract = boxed(unbox::<Rc<dyn crate::ISupportInitialize>>(&instance));
    (initialize.find_methods("BeginInit").next().unwrap().invoke)(&[contract.clone()]).unwrap();
    (initialize.find_methods("EndInit").next().unwrap().invoke)(&[contract]).unwrap();
    assert!(initialize.find_methods("EndInit").next().unwrap().parameters.is_empty());

    // Members are inherited by handles of deriving classes.
    let derived = into_markup_value(InputElement::new());
    assert!((resources.get.unwrap())(&[derived]).unwrap().is_some());

    // A typed routed event converts to the untyped routed event.
    let tapped = boxed(*InputElement::tapped_event());
    assert!(unbox::<RoutedEvent>(&tapped) == InputElement::tapped_event().as_routed_event());
}

#[test]
fn resource_dictionary_adds_by_key_through_metadata() {
    let markup = class_markup::<ResourceDictionary>();
    let dictionary = ResourceDictionary::new();
    let instance = into_markup_value(dictionary.clone());
    let add = markup.find_methods("Add").next().unwrap();
    assert_eq!(add.parameters.len(), 2);

    // Text and types are keys.
    (add.invoke)(&[instance.clone(), text("accent"), boxed(Colors::RED)]).unwrap();
    (add.invoke)(&[instance.clone(), boxed(StyledElement::TYPE), None]).unwrap();
    let value = dictionary.get(&ResourceKey::from("accent")).unwrap();
    assert_eq!(value.downcast_ref::<Color>(), Some(&Colors::RED));
    assert!(dictionary.contains_key(&ResourceKey::from(StyledElement::TYPE)));
    assert_eq!(dictionary.count(), 2);
}

#[test]
fn class_constructors_properties_and_static_values_work_through_metadata() {
    let brush = class_markup::<SolidColorBrush>();
    let constructor = brush.constructors.iter().find(|c| c.parameters.len() == 2).unwrap();
    let value = (constructor.invoke)(&[boxed(Colors::RED), boxed(0.5f64)]).unwrap();
    let typed = unbox::<Ref<SolidColorBrush>>(&value);
    assert_eq!((typed.color(), typed.opacity()), (Colors::RED, 0.5));
    // The class implements the brush contract.
    assert!(from_markup_value::<Rc<dyn IBrush>>(&value).is_some());

    let theme = class_markup::<ControlTheme>();
    let value = (theme.constructors[0].invoke)(&[boxed(StyledElement::TYPE)]).unwrap();
    let target = theme.find_property("TargetType").unwrap();
    assert!(std::ptr::eq(unbox::<&'static TypeInfo>(&(target.get.unwrap())(&[value.clone()]).unwrap()), StyledElement::TYPE));
    (target.set.unwrap())(&[value.clone(), None]).unwrap();
    assert_eq!((target.get.unwrap())(&[value.clone()]).unwrap(), None);
    let based_on = theme.find_property("BasedOn").unwrap();
    let base = ControlTheme::new();
    (based_on.set.unwrap())(&[value.clone(), into_markup_value(base.clone())]).unwrap();
    assert_eq!(unbox::<Ref<ControlTheme>>(&value).based_on(), Some(base));

    let frame = class_markup::<KeyFrame>();
    let instance = into_markup_value(KeyFrame::new());
    let spline = (class_markup::<KeySpline>().parse.unwrap())(&[text("0.1,0.2,0.3,0.4")]).unwrap();
    (frame.find_property("KeySpline").unwrap().set.unwrap())(&[instance.clone(), spline]).unwrap();
    let x1 = class_markup::<KeySpline>().find_property("ControlPointX1").unwrap();
    let spline = (frame.find_property("KeySpline").unwrap().get.unwrap())(&[instance]).unwrap();
    assert_eq!(unbox::<f64>(&(x1.get.unwrap())(&[spline]).unwrap()), 0.1);

    // Routed events are static values of their class.
    let input = class_markup::<InputElement>();
    let tapped = input.find_field("TappedEvent").unwrap();
    assert_eq!((tapped.type_)(), ValueType::of::<RoutedEvent<TappedEventArgs>>());
    assert!(unbox::<RoutedEvent<TappedEventArgs>>(&(tapped.get)()) == *InputElement::tapped_event());
    // The type of the value is an instantiation of the typed routed event.
    let typed = MarkupType::find_by_handle((tapped.type_)().id()).unwrap();
    assert_eq!(typed.generic.unwrap().definition, "RoutedEvent`1");
    assert_eq!((typed.base.unwrap())(), ValueType::of::<RoutedEvent>());
    assert_eq!(
        input.find_attribute(attributes::PSEUDO_CLASSES).unwrap().arguments[0],
        MarkupAttributeValue::Str(":disabled")
    );
    let drag_drop = <DragDrop as MarkupTyped>::MARKUP;
    assert_eq!(drag_drop.kind, MarkupTypeKind::Static);
    // The metadata of a static type is found from its runtime type.
    assert!(std::ptr::eq(MarkupType::find_by_type_info(DragDrop::TYPE).unwrap(), drag_drop));
    assert!(unbox::<RoutedEvent<crate::input::DragEventArgs>>(&(drag_drop.find_field("DropEvent").unwrap().get)())
        == *DragDrop::drop_event());

    let geometry = class_markup::<Geometry>();
    assert_eq!(geometry.namespace(), "FerroUI.Media");
    assert!(geometry.parse.is_some());
}

// The types the markup compiler looks up by name.

/// Every type of the namespaces of this crate that the markup compiler
/// resolves by its full name when it starts (its well-known types and the
/// types of its language configuration). The property definition types
/// (``FerroProperty`1``, ``StyledProperty`1``, ``AttachedProperty`1``) and
/// the attribute types are defined by the compiler's type system itself.
const WELL_KNOWN_TYPES: &[&str] = &[
    "FerroUI.Animation.Transitions",
    "FerroUI.Collections.FerroListConverter`1",
    "FerroUI.Collections.FerroList`1",
    "FerroUI.Controls.Classes",
    "FerroUI.Controls.IDeferredContent",
    "System.EventArgs",
    "System.ComponentModel.CancelEventArgs",
    "FerroUI.INamed",
    "FerroUI.Platform.IRuntimePlatform",
    "FerroUI.Platform.RuntimePlatformInfo",
    "FerroUI.Platform.FormFactorType",
    "FerroUI.Data.BindingValueType",
    "FerroUI.Data.ReflectionBinding",
    "FerroUI.Data.TemplateBinding",
    "FerroUI.Styling.Selector",
    "System.ComponentModel.ISupportInitialize",
    "FerroUI.Controls.INameScope",
    "FerroUI.Controls.IResourceDictionary",
    "FerroUI.Controls.IThemeVariantProvider",
    "FerroUI.Controls.NameScope",
    "FerroUI.Controls.ResourceDictionary",
    "FerroUI.CornerRadius",
    "FerroUI.Data.BindingBase",
    "FerroUI.Data.BindingExpressionBase",
    "FerroUI.Data.BindingPriority",
    "FerroUI.Data.CompiledBinding",
    "FerroUI.Data.CompiledBindingPath",
    "FerroUI.Data.CompiledBindingPathBuilder",
    "FerroUI.Data.Core.ClrPropertyInfo",
    "FerroUI.Data.Core.ClrPropertyInfo`2",
    "FerroUI.Data.Core.IPropertyInfo",
    "FerroUI.Data.Core.IPropertyInfo`2",
    "FerroUI.Data.Core.Plugins.IPropertyAccessor",
    "FerroUI.Data.MultiBinding",
    "FerroUI.Data.RelativeSource",
    "FerroUI.FerroObject",
    "FerroUI.FerroObjectExtensions",
    "FerroUI.FerroProperty",
    "FerroUI.Input.Cursor",
    "FerroUI.Input.StandardCursorType",
    "FerroUI.Interactivity.Interactive",
    "FerroUI.Interactivity.RoutedEvent",
    "FerroUI.Interactivity.RoutedEventArgs",
    "FerroUI.Interactivity.RoutingStrategies",
    "FerroUI.Matrix",
    "FerroUI.Media.Color",
    "FerroUI.Media.FontFamily",
    "FerroUI.Media.IBrush",
    "FerroUI.Media.IImage",
    "FerroUI.Media.IImageBrushSource",
    "FerroUI.Media.Imaging.Bitmap",
    "FerroUI.Media.Immutable.ImmutableSolidColorBrush",
    "FerroUI.Media.TextDecorationCollection",
    "FerroUI.Media.TextDecorations",
    "FerroUI.Media.TextTrimming",
    "FerroUI.Metadata.IAddChild",
    "FerroUI.Metadata.IAddChild`1",
    "FerroUI.Point",
    "FerroUI.RelativePoint",
    "FerroUI.RelativeUnit",
    "FerroUI.Size",
    "FerroUI.StyledElement",
    "FerroUI.StyledElementExtensions",
    "FerroUI.Styling.ContainerQuery",
    "FerroUI.Styling.ControlTheme",
    "FerroUI.Styling.IStyle",
    "FerroUI.Styling.Selectors",
    "FerroUI.Styling.Setter",
    "FerroUI.Styling.SetterBase",
    "FerroUI.Styling.Style",
    "FerroUI.Styling.StyleQueries",
    "FerroUI.Styling.Styles",
    "FerroUI.Styling.ThemeVariant",
    "FerroUI.Thickness",
    "FerroUI.UnsetValueType",
    "FerroUI.Utilities.TypeUtilities",
    "FerroUI.Vector",
    "System.UriKind",
    "System.Windows.Input.ICommand",
];

/// Whether a type is found by its full name the way the compiler's type
/// system finds it: a class by its runtime type, another type by its
/// metadata, a generic definition by one of its declared instantiations.
fn is_found_by_name(full_name: &str) -> bool {
    let (namespace, name) = full_name.rsplit_once('.').unwrap();
    if TypeInfo::find(namespace, name).is_some() {
        return true;
    }
    if name.contains('`') {
        return MarkupType::registered_types()
            .iter()
            .any(|type_| type_.namespace() == namespace && type_.generic.is_some_and(|g| g.definition == name));
    }
    MarkupType::find(namespace, name).is_some_and(|type_| type_.generic.is_none())
}

#[test]
fn the_well_known_types_of_the_markup_compiler_are_found_by_name() {
    crate::register_types();
    let missing: Vec<&str> = WELL_KNOWN_TYPES.iter().copied().filter(|name| !is_found_by_name(name)).collect();
    assert!(missing.is_empty(), "{missing:?}");
}

#[test]
fn the_well_known_members_have_the_signatures_the_compiler_asks_for() {
    crate::register_types();
    let find = |namespace: &str, name: &str| MarkupType::find(namespace, name).unwrap();
    let types = |types: &[crate::metadata::TypeOf]| types.iter().map(|t| t()).collect::<Vec<_>>();
    let object = ValueType::of::<Option<BoxedValue>>();
    let f64_ = ValueType::of::<f64>();
    let constructor = |type_: &MarkupType, parameters: &[ValueType]| {
        assert!(type_.constructors.iter().any(|c| types(c.parameters) == parameters), "{type_:?}{parameters:?}");
    };

    constructor(find("FerroUI", "Thickness"), &[f64_; 4]);
    constructor(find("FerroUI", "Point"), &[f64_; 2]);
    constructor(find("FerroUI", "Vector"), &[f64_; 2]);
    constructor(find("FerroUI", "Size"), &[f64_; 2]);
    constructor(find("FerroUI", "Matrix"), &[f64_; 6]);
    constructor(find("FerroUI", "CornerRadius"), &[f64_; 4]);
    constructor(find("FerroUI", "RelativePoint"), &[f64_, f64_, ValueType::of::<crate::RelativeUnit>()]);
    constructor(find("FerroUI.Input", "Cursor"), &[ValueType::of::<crate::input::StandardCursorType>()]);
    constructor(find("FerroUI.Media", "FontFamily"), &[ValueType::of::<Option<crate::utilities::Uri>>(), ValueType::of::<String>()]);
    constructor(find("FerroUI.Media.Immutable", "ImmutableSolidColorBrush"), &[ValueType::of::<u32>()]);
    constructor(find("System", "Uri"), &[ValueType::of::<String>(), ValueType::of::<crate::utilities::UriKind>()]);

    let scope = find("FerroUI.Controls", "INameScope");
    assert_eq!(types(scope.find_methods("Register").next().unwrap().parameters), [ValueType::of::<String>(), object]);
    assert!(scope.find_methods("Complete").next().unwrap().parameters.is_empty());
    let name_scope = MarkupType::find_by_type_info(NameScope::TYPE).unwrap();
    let set_name_scope = name_scope.find_methods("SetNameScope").next().unwrap();
    assert!(set_name_scope.is_static);
    assert_eq!(
        types(set_name_scope.parameters),
        [ValueType::of::<Ref<StyledElement>>(), ValueType::of::<Option<NameScopeRef>>()]
    );
    // The scope parameter is a handle of the name scope contract.
    assert!(std::ptr::eq(MarkupType::find_by_handle(ValueType::of::<Option<NameScopeRef>>().id()).unwrap(), scope));

    let dictionary = class_markup::<ResourceDictionary>();
    let deferred = ValueType::of::<Rc<dyn crate::controls::IDeferredContent>>();
    assert_eq!(types(dictionary.find_methods("AddDeferred").next().unwrap().parameters), [object, deferred]);
    assert_eq!(types(dictionary.find_methods("AddNotSharedDeferred").next().unwrap().parameters), [object, deferred]);
    assert_eq!(types(dictionary.find_methods("EnsureCapacity").next().unwrap().parameters), [ValueType::of::<i32>()]);
    assert_eq!((dictionary.find_property("Count").unwrap().type_)(), ValueType::of::<i32>());

    let classes = class_markup::<StyledElement>().find_property("Classes").unwrap();
    assert!(std::ptr::eq(MarkupType::find_by_handle((classes.type_)().id()).unwrap(), find("FerroUI.Controls", "Classes")));
    let get_class_property = find("FerroUI", "StyledElementExtensions").find_methods("GetClassProperty").next().unwrap();
    assert!(get_class_property.is_static);
    assert_eq!(types(get_class_property.parameters), [ValueType::of::<String>()]);
    assert_eq!((get_class_property.return_type.unwrap())(), ValueType::of::<&'static crate::FerroProperty>());
}

#[test]
fn the_well_known_members_work_through_metadata() {
    crate::register_types();
    let find = |namespace: &str, name: &str| MarkupType::find(namespace, name).unwrap();

    // Classes: the collection, the property of an element and class properties.
    let classes = find("FerroUI.Controls", "Classes");
    let element = StyledElement::new();
    let instance = into_markup_value(element.clone());
    let property = class_markup::<StyledElement>().find_property("Classes").unwrap();
    let collection = (property.get.unwrap())(&[instance.clone()]).unwrap();
    (classes.find_methods("Add").next().unwrap().invoke)(&[collection.clone(), text("a")]).unwrap();
    (classes.find_methods("Set").next().unwrap().invoke)(&[collection.clone(), text("b"), boxed(true)]).unwrap();
    (classes.find_methods("Set").next().unwrap().invoke)(&[collection.clone(), text("a"), boxed(false)]).unwrap();
    assert_eq!(*element.classes().snapshot(), ["b"]);
    // The property is read-only, as in the managed original: names are added to the collection.
    assert!(property.set.is_none());
    let parsed = (classes.parse.unwrap())(&[text("x y")]).unwrap();
    assert_eq!(*unbox::<crate::controls::Classes>(&parsed).snapshot(), ["x", "y"]);
    drop(instance);
    let get_class_property = find("FerroUI", "StyledElementExtensions").find_methods("GetClassProperty").next().unwrap();
    let class_property = unbox::<&'static crate::FerroProperty>(&(get_class_property.invoke)(&[text("active")]).unwrap());
    assert!(std::ptr::eq(class_property, crate::ClassBindingManager::get_class_property("active")));

    // The attached name scope.
    let name_scope = MarkupType::find_by_type_info(NameScope::TYPE).unwrap();
    let scope = boxed(NameScopeRef::new(NameScope::new()));
    let element = into_markup_value(StyledElement::new());
    (name_scope.find_methods("SetNameScope").next().unwrap().invoke)(&[element.clone(), scope.clone()]).unwrap();
    let read = (name_scope.find_methods("GetNameScope").next().unwrap().invoke)(&[element]).unwrap();
    assert!(unbox::<NameScopeRef>(&read) == unbox::<NameScopeRef>(&scope));
    // Only objects are registered.
    let register = find("FerroUI.Controls", "INameScope").find_methods("Register").next().unwrap();
    assert!((register.invoke)(&[scope.clone(), text("n"), boxed(1i32)]).is_err());
    // A name that is taken, or a completed scope, is an error and not a panic.
    (register.invoke)(&[scope.clone(), text("n"), into_markup_value(StyledElement::new())]).unwrap();
    let taken = (register.invoke)(&[scope.clone(), text("n"), into_markup_value(StyledElement::new())]).unwrap_err();
    assert_eq!(taken.to_string(), "Control with the name 'n' already registered.");
    (find("FerroUI.Controls", "INameScope").find_methods("Complete").next().unwrap().invoke)(&[scope.clone()]).unwrap();
    assert!((register.invoke)(&[scope, text("m"), into_markup_value(StyledElement::new())]).is_err());

    // A named element.
    let named = find("FerroUI", "INamed");
    let element = StyledElement::new();
    element.set_name(Some("root".to_string()));
    let contract = boxed(unbox::<Rc<dyn crate::INamed>>(&into_markup_value(element)));
    assert_eq!(unbox::<String>(&(named.find_property("Name").unwrap().get.unwrap())(&[contract]).unwrap()), "root");

    // An immutable brush from its color value is a brush.
    let brush = find("FerroUI.Media.Immutable", "ImmutableSolidColorBrush");
    let constructor = brush.constructors.iter().find(|c| (c.parameters[0])() == ValueType::of::<u32>()).unwrap();
    let value = (constructor.invoke)(&[boxed(0xFFFF0000u32)]).unwrap();
    let as_brush = unbox::<Option<Rc<dyn IBrush>>>(&value).unwrap();
    let expected: Rc<dyn IBrush> = Brushes::red();
    assert!(*as_brush == *expected);

    let font = find("FerroUI.Media", "FontFamily");
    let constructor = font.constructors.iter().find(|c| c.parameters.len() == 2).unwrap();
    let family = (constructor.invoke)(&[None, text("Arial")]).unwrap();
    assert_eq!(unbox::<crate::media::FontFamily>(&family), crate::media::FontFamily::new("Arial"));
    // `FontFamily.Name`: the primary family name, what a binding with the font family as
    // its data type reads.
    let name = font.find_property("Name").expect("the property");
    assert_eq!((name.type_)(), ValueType::of::<String>());
    assert!(name.set.is_none());
    assert_eq!(unbox::<String>(&(name.get.unwrap())(&[family.clone()]).unwrap()), "Arial");
    let fallbacks = (font.parse.unwrap())(&[text("Courier New, Arial")]).unwrap();
    assert_eq!(unbox::<String>(&(name.get.unwrap())(&[fallbacks]).unwrap()), "Courier New");

    let uri = find("System", "Uri");
    let constructor = uri.constructors.iter().find(|c| c.parameters.len() == 2).unwrap();
    let relative = (constructor.invoke)(&[text("a/b"), boxed(crate::utilities::UriKind::Relative)]).unwrap();
    assert!(!unbox::<crate::utilities::Uri>(&relative).is_absolute_uri());
    assert!((constructor.invoke)(&[text("a/b"), boxed(crate::utilities::UriKind::Absolute)]).is_err());

    // The count of a resource dictionary, and keys that are not supported.
    let dictionary = class_markup::<ResourceDictionary>();
    let instance = into_markup_value(ResourceDictionary::new());
    let add = dictionary.find_methods("Add").next().unwrap();
    (add.invoke)(&[instance.clone(), text("k"), boxed(1i32)]).unwrap();
    assert!((add.invoke)(&[instance.clone(), boxed(1.5f64), boxed(1i32)]).is_err());
    assert!((add.invoke)(&[instance.clone(), None, boxed(1i32)]).is_err());
    // A key that is present is an error, not a panic, for every way of adding.
    let duplicate = (add.invoke)(&[instance.clone(), text("k"), boxed(2i32)]).unwrap_err();
    assert_eq!(duplicate.to_string(), "An item with the same key has already been added. Key: k");
    struct Deferred;
    impl crate::controls::IDeferredContent for Deferred {
        fn build(&self, _: Option<&Rc<dyn crate::metadata::IServiceProvider>>) -> Option<BoxedValue> {
            None
        }
    }
    let content: Rc<dyn crate::controls::IDeferredContent> = Rc::new(Deferred);
    for name in ["AddDeferred", "AddNotSharedDeferred"] {
        let method = dictionary.find_methods(name).next().unwrap();
        assert!((method.invoke)(&[instance.clone(), text("k"), boxed(content.clone())]).is_err(), "{name}");
    }
    (dictionary.find_methods("AddDeferred").next().unwrap().invoke)(&[instance.clone(), text("d"), boxed(content)]).unwrap();
    let themes = (dictionary.find_property("ThemeDictionaries").unwrap().get.unwrap())(&[instance.clone()]).unwrap();
    let map = MarkupType::find_by_handle(ValueType::of_value(&**themes.as_ref().unwrap()).id()).unwrap();
    let add_theme = map.find_methods("Add").next().unwrap();
    let dark = boxed(crate::styling::ThemeVariant::dark());
    (add_theme.invoke)(&[themes.clone(), dark.clone(), into_markup_value(ResourceDictionary::new())]).unwrap();
    assert!((add_theme.invoke)(&[themes, dark, into_markup_value(ResourceDictionary::new())]).is_err());
    (dictionary.find_methods("EnsureCapacity").next().unwrap().invoke)(&[instance.clone(), boxed(8i32)]).unwrap();
    assert_eq!(unbox::<i32>(&(dictionary.find_property("Count").unwrap().get.unwrap())(&[instance]).unwrap()), 2);
}

#[test]
fn a_data_validation_exception_is_an_exception_with_its_error_data() {
    use crate::data::{BindingError, DataValidationException};

    crate::register_types();
    let type_ = MarkupType::find("FerroUI.Data", "DataValidationException").unwrap();
    assert_eq!((type_.base.unwrap())(), ValueType::of::<BindingError>());
    let constructor = type_.constructors.iter().find(|c| c.parameters.len() == 1).unwrap();
    assert_eq!((constructor.parameters[0])(), ValueType::of::<Option<BoxedValue>>());

    // `new DataValidationException(errorData)`, as `x:Arguments` passes the error data.
    let exception = (constructor.invoke)(&[text("Enter a valid email address.")]).unwrap();
    let data = (type_.find_property("ErrorData").unwrap().get.unwrap())(&[exception.clone()]).unwrap();
    assert_eq!(unbox::<String>(&data), "Enter a valid email address.");

    // It is an exception: the members of the base type take it, and a member that takes an
    // exception receives the error that wraps it.
    let message = MarkupType::find("System", "Exception").unwrap().find_property("Message").unwrap().get.unwrap();
    assert_eq!(unbox::<String>(&message(&[exception.clone()]).unwrap()), "Enter a valid email address.");
    let error = unbox::<Option<BindingError>>(&exception).expect("an error");
    let wrapped = error.inner().downcast_ref::<DataValidationException>().expect("the exception");
    assert!(wrapped.error_data().is_some_and(|data| data.downcast_ref::<String>().is_some()));

    // Without error data the message is empty.
    let empty = (constructor.invoke)(&[None]).unwrap();
    assert_eq!(unbox::<String>(&message(&[empty]).unwrap()), "");
}

// Styling, bindings and the members that return handles.

#[test]
fn selectors_and_style_queries_are_built_through_metadata() {
    crate::register_types();
    let selectors = MarkupType::find("FerroUI.Styling", "Selectors").unwrap();
    let call = |name: &str, arguments: &[MarkupValue]| {
        let method = selectors
            .find_methods(name)
            .find(|m| m.parameters.len() == arguments.len())
            .unwrap_or_else(|| panic!("Selectors.{name}"));
        assert!(method.is_static);
        (method.invoke)(arguments).unwrap_or_else(|e| panic!("Selectors.{name}: {e}"))
    };
    for name in ["Child", "Class", "Descendant", "Is", "Name", "Nesting", "Not", "NthChild", "NthLastChild", "OfType",
        "Or", "PropertyEquals", "Template"]
    {
        assert!(selectors.find_methods(name).next().is_some(), "Selectors.{name}");
    }

    // `InputElement.accent:focus > StyledElement`, built step by step.
    let of_type = call("OfType", &[None, boxed(InputElement::TYPE)]);
    let class = call("Class", &[of_type, text("accent")]);
    let pseudo = call("Class", &[class, text(":focus")]);
    let child = call("Child", &[pseudo]);
    let selector = call("Is", &[child, boxed(StyledElement::TYPE)]);
    let typed = unbox::<crate::styling::Selector>(&selector);
    assert_eq!(typed.to_string(), "InputElement.accent:focus > :is(StyledElement)");
    assert!(std::ptr::eq(typed.target_type().unwrap(), StyledElement::TYPE));

    // A property selector compares with a value of the type of the property.
    let property = boxed(InputElement::is_enabled_property().as_property());
    let equals = call("PropertyEquals", &[None, property.clone(), boxed(true)]);
    assert!(from_markup_value::<crate::styling::Selector>(&equals).is_some());
    let method = selectors.find_methods("PropertyEquals").next().unwrap();
    assert!((method.invoke)(&[None, property, text("x")]).is_err());

    // The selector of a style.
    let style = Style::new();
    let markup = class_markup::<Style>();
    (markup.find_property("Selector").unwrap().set.unwrap())(&[into_markup_value(style.clone()), selector.clone()]).unwrap();
    assert!(style.selector() == Some(typed));

    let queries = MarkupType::find("FerroUI.Styling", "StyleQueries").unwrap();
    let width = queries.find_methods("Width").next().unwrap();
    let operator = boxed(crate::styling::StyleQueryComparisonOperator::LessThanOrEquals);
    let query = (width.invoke)(&[None, operator, boxed(600.0f64)]).unwrap();
    assert!(from_markup_value::<crate::styling::StyleQuery>(&query).is_some());
    for name in ["Height", "Or", "And"] {
        assert!(queries.find_methods(name).next().is_some());
    }
}

#[test]
fn setters_and_style_collections_work_through_metadata() {
    crate::register_types();
    let setter = MarkupType::find("FerroUI.Styling", "Setter").unwrap();
    let property = InputElement::is_enabled_property().as_property();

    let empty = (setter.constructors.iter().find(|c| c.parameters.is_empty()).unwrap().invoke)(&[]).unwrap();
    (setter.find_property("Property").unwrap().set.unwrap())(&[empty.clone(), boxed(property)]).unwrap();
    let value = setter.find_property("Value").unwrap();
    (value.set.unwrap())(&[empty.clone(), boxed(false)]).unwrap();
    assert_eq!(unbox::<bool>(&(value.get.unwrap())(&[empty.clone()]).unwrap()), false);
    let full = setter.constructors.iter().find(|c| c.parameters.len() == 2).unwrap();
    let with_value = (full.invoke)(&[boxed(property), boxed(true)]).unwrap();

    // A binding assigned to the value is kept as the binding.
    let binding = into_markup_value(crate::data::ReflectionBinding::new("Name"));
    (value.set.unwrap())(&[empty.clone(), binding.clone()]).unwrap();
    let read = (value.get.unwrap())(&[empty.clone()]).unwrap();
    assert!(from_markup_value::<Rc<dyn crate::data::BindingBase>>(&read).is_some());

    // Setters are added to the setters of a style; a setter is a setter base.
    let style = Style::new();
    let instance = into_markup_value(style.clone());
    let setters = (class_markup::<crate::styling::StyleBase>().find_property("Setters").unwrap().get.unwrap())(&[
        instance.clone(),
    ])
    .unwrap();
    let list = MarkupType::find_by_handle(ValueType::of_value(&**setters.as_ref().unwrap()).id()).unwrap();
    assert_eq!(list.generic.unwrap().definition, "FerroList`1");
    (list.find_methods("Add").next().unwrap().invoke)(&[setters, with_value]).unwrap();
    assert_eq!(style.setters().count(), 1);

    // The children of a style and the styles of an element.
    let children = (class_markup::<crate::styling::StyleBase>().find_property("Children").unwrap().get.unwrap())(&[
        instance.clone(),
    ])
    .unwrap();
    let style_children = MarkupType::find("FerroUI.Styling", "StyleChildren").unwrap();
    (style_children.find_methods("Add").next().unwrap().invoke)(&[children, into_markup_value(Style::new())]).unwrap();
    assert_eq!(style.children().count(), 1);
    let styles = Styles::new();
    (class_markup::<Styles>().find_methods("Add").next().unwrap().invoke)(&[into_markup_value(styles.clone()), instance])
        .unwrap();
    assert_eq!(styles.count(), 1);

    // The merged and theme dictionaries of a resource dictionary.
    let dictionary = ResourceDictionary::new();
    let markup = class_markup::<ResourceDictionary>();
    let merged = (markup.find_property("MergedDictionaries").unwrap().get.unwrap())(&[into_markup_value(dictionary.clone())])
        .unwrap();
    let list = MarkupType::find_by_handle(ValueType::of_value(&**merged.as_ref().unwrap()).id()).unwrap();
    (list.find_methods("Add").next().unwrap().invoke)(&[merged, into_markup_value(ResourceDictionary::new())]).unwrap();
    assert_eq!(dictionary.merged_dictionaries().count(), 1);
    let themes = (markup.find_property("ThemeDictionaries").unwrap().get.unwrap())(&[into_markup_value(dictionary.clone())])
        .unwrap();
    let map = MarkupType::find_by_handle(ValueType::of_value(&**themes.as_ref().unwrap()).id()).unwrap();
    assert_eq!(map.generic.unwrap().definition, "FerroDictionary`2");
    (map.find_methods("Add").next().unwrap().invoke)(&[
        themes,
        boxed(crate::styling::ThemeVariant::dark()),
        into_markup_value(ResourceDictionary::new()),
    ])
    .unwrap();
    assert_eq!(dictionary.theme_dictionaries().count(), 1);

    // The key frames of an animation and the setters of a key frame.
    let animation = crate::animation::Animation::new();
    let markup = class_markup::<crate::animation::Animation>();
    assert_eq!(markup.content_property, Some("Children"));
    let frames = (markup.find_property("Children").unwrap().get.unwrap())(&[into_markup_value(animation.clone())]).unwrap();
    let key_frames = MarkupType::find("FerroUI.Animation", "KeyFrames").unwrap();
    (key_frames.find_methods("Add").next().unwrap().invoke)(&[frames, into_markup_value(KeyFrame::new())]).unwrap();
    assert_eq!(animation.children().count(), 1);
    assert_eq!(class_markup::<KeyFrame>().content_property, Some("Setters"));
}

#[test]
fn bindings_are_reference_types_with_settable_properties() {
    crate::register_types();
    let find = |name: &str| MarkupType::find("FerroUI.Data", name).unwrap();

    let reflection = find("ReflectionBinding");
    let binding = (reflection.constructors.iter().find(|c| c.parameters.len() == 1).unwrap().invoke)(&[text("Name")]).unwrap();
    let typed = unbox::<Rc<crate::data::ReflectionBinding>>(&binding);
    assert_eq!(typed.path(), "Name");
    let path = reflection.find_property("Path").unwrap();
    assert_eq!(path.attributes[0].name, attributes::CONSTRUCTOR_ARGUMENT);
    (reflection.find_property("Mode").unwrap().set.unwrap())(&[binding.clone(), boxed(crate::data::BindingMode::TwoWay)])
        .unwrap();
    assert_eq!(typed.mode(), crate::data::BindingMode::TwoWay);
    let source = find("RelativeSource");
    let relative = (source.constructors.iter().find(|c| c.parameters.len() == 1).unwrap().invoke)(&[boxed(
        RelativeSourceMode::TemplatedParent,
    )])
    .unwrap();
    (source.find_property("AncestorLevel").unwrap().set.unwrap())(&[relative.clone(), boxed(2i32)]).unwrap();
    (reflection.find_property("RelativeSource").unwrap().set.unwrap())(&[binding.clone(), relative]).unwrap();
    assert_eq!(typed.relative_source().unwrap().ancestor_level(), 2);
    (reflection.find_property("ElementName").unwrap().set.unwrap())(&[binding.clone(), text("root")]).unwrap();
    assert_eq!(typed.element_name().as_deref(), Some("root"));

    // A binding is a binding base: it is added to the bindings of a multi binding.
    let multi = find("MultiBinding");
    assert_eq!(multi.content_property, Some("Bindings"));
    let instance = (multi.constructors[0].invoke)(&[]).unwrap();
    let bindings = (multi.find_property("Bindings").unwrap().get.unwrap())(&[instance.clone()]).unwrap();
    let list = MarkupType::find_by_handle(ValueType::of_value(&**bindings.as_ref().unwrap()).id()).unwrap();
    (list.find_methods("Add").next().unwrap().invoke)(&[bindings, binding.clone()]).unwrap();
    assert_eq!(unbox::<Rc<crate::data::MultiBinding>>(&instance).bindings().count(), 1);

    let template = find("TemplateBinding");
    let property = template.find_property("Property").unwrap();
    assert!(property.attributes.iter().any(|a| a.name == attributes::INHERIT_DATA_TYPE_FROM));
    let value = (template.constructors.iter().find(|c| c.parameters.len() == 1).unwrap().invoke)(&[boxed(
        InputElement::is_enabled_property().as_property(),
    )])
    .unwrap();
    assert!((property.get.unwrap())(&[value.clone()]).unwrap().is_some());
    // The binding is a markup extension that provides itself.
    let provide = template.find_methods("ProvideValue").next().unwrap();
    assert!(provide.parameters.is_empty());
    assert_eq!((provide.return_type.unwrap())(), ValueType::of::<Rc<dyn crate::data::BindingBase>>());
    let provided = (provide.invoke)(&[value.clone()]).unwrap();
    let binding = unbox::<Rc<dyn crate::data::BindingBase>>(&provided);
    let concrete = binding.as_any().and_then(|any| any.downcast_ref::<crate::data::TemplateBinding>()).unwrap();
    assert!(std::ptr::eq(concrete, &*unbox::<Rc<crate::data::TemplateBinding>>(&value)));
    assert_eq!(MarkupType::find("FerroUI.Styling", "Setter").unwrap().content_property, Some("Value"));

    // A compiled binding from a path made by the builder.
    let builder = find("CompiledBindingPathBuilder");
    let mut current = (builder.constructors[0].invoke)(&[]).unwrap();
    for (name, arguments) in [("Self", vec![]), ("Ancestor", vec![boxed(StyledElement::TYPE), boxed(1i32)]), ("Not", vec![])] {
        let mut all = vec![current.clone()];
        all.extend(arguments);
        current = (builder.find_methods(name).next().unwrap().invoke)(&all).unwrap();
    }
    let path = (builder.find_methods("Build").next().unwrap().invoke)(&[current]).unwrap();
    assert_eq!(unbox::<crate::data::CompiledBindingPath>(&path).len(), 3);
    let compiled = find("CompiledBinding");
    let value = (compiled.constructors.iter().find(|c| c.parameters.len() == 1).unwrap().invoke)(&[path]).unwrap();
    assert!(unbox::<Rc<crate::data::CompiledBinding>>(&value).path().is_some());

    // The casts to the binding base are known on every thread.
    std::thread::spawn(|| {
        let binding = into_markup_value(crate::data::CompiledBinding::empty());
        assert!(from_markup_value::<Rc<dyn crate::data::BindingBase>>(&binding).is_some());
        assert!(from_markup_value::<Option<Rc<dyn crate::data::BindingBase>>>(&binding).unwrap().is_some());
    })
    .join()
    .unwrap();
}

#[test]
fn binding_and_setting_values_return_handles_through_metadata() {
    crate::register_types();
    let element = StyledElement::new();
    let instance = into_markup_value(element.clone());
    let object = class_markup::<FerroObject>();
    let disposable = ValueType::of::<Option<Rc<dyn crate::reactive::IDisposable>>>();

    // `SetValue(FerroProperty, object, BindingPriority) -> IDisposable`
    let set_value = object.find_methods("SetValue").next().unwrap();
    assert_eq!((set_value.return_type.unwrap())(), disposable);
    let name = boxed(StyledElement::name_property().as_property());
    let priority = boxed(crate::data::BindingPriority::LocalValue);
    (set_value.invoke)(&[instance.clone(), name.clone(), text("root"), priority.clone()]).unwrap();
    assert_eq!(element.name().as_deref(), Some("root"));
    assert!((set_value.invoke)(&[instance.clone(), name.clone(), boxed(1.5f64), priority.clone()]).is_err());
    // The unset value clears the value.
    (set_value.invoke)(&[instance.clone(), name.clone(), Some(crate::FerroProperty::unset_value()), priority]).unwrap();
    assert_eq!(element.name(), None);

    // `Bind(FerroProperty, BindingBase) -> BindingExpressionBase`
    let bind = object.find_methods("Bind").next().unwrap();
    assert_eq!((bind.return_type.unwrap())(), ValueType::of::<Rc<dyn crate::data::BindingExpressionBase>>());
    let binding = into_markup_value(crate::data::ReflectionBinding::new("Tag"));
    let tag = boxed(StyledElement::data_context_property().as_property());
    let expression = (bind.invoke)(&[instance.clone(), tag.clone(), binding.clone()]).unwrap();
    // The expression is disposable.
    assert!(from_markup_value::<Rc<dyn crate::reactive::IDisposable>>(&expression).is_some());

    // The static forms with an anchor, and class bindings.
    let extensions = MarkupType::find("FerroUI", "FerroObjectExtensions").unwrap();
    let bound = (extensions.find_methods("Bind").next().unwrap().invoke)(&[instance.clone(), tag, binding.clone(), None]).unwrap();
    assert!(from_markup_value::<Rc<dyn crate::reactive::IDisposable>>(&bound).is_some());
    let styled = MarkupType::find("FerroUI", "StyledElementExtensions").unwrap();
    let bind_class = styled.find_methods("BindClass").next().unwrap();
    assert_eq!((bind_class.return_type.unwrap())(), ValueType::of::<Rc<dyn crate::reactive::IDisposable>>());
    let bound = (bind_class.invoke)(&[instance, text("active"), binding, None]).unwrap();
    assert!(bound.is_some());
}

#[test]
fn routed_event_arguments_are_live_through_metadata() {
    crate::register_types();
    let markup = MarkupType::find("FerroUI.Interactivity", "RoutedEventArgs").unwrap();
    let arguments = crate::interactivity::RoutedEventArgs::new();
    let shared = boxed(crate::interactivity::IRoutedEventArgs::share(&arguments));
    let handled = markup.find_property("Handled").unwrap();
    assert!(!unbox::<bool>(&(handled.get.unwrap())(&[shared.clone()]).unwrap()));
    (handled.set.unwrap())(&[shared.clone(), boxed(true)]).unwrap();
    // The handle shares the state of the arguments it was made from.
    assert!(unbox::<bool>(&(handled.get.unwrap())(&[shared.clone()]).unwrap()));
    assert_eq!((markup.find_property("Source").unwrap().get.unwrap())(&[shared]).unwrap(), None);

    let platform = MarkupType::find("FerroUI.Platform", "RuntimePlatformInfo").unwrap();
    let info = boxed(crate::platform::RuntimePlatformInfo { is_desktop: true, ..Default::default() });
    let form_factor = (platform.find_property("FormFactor").unwrap().get.unwrap())(&[info]).unwrap();
    assert_eq!(unbox::<crate::platform::FormFactorType>(&form_factor), crate::platform::FormFactorType::Desktop);
}

#[test]
fn typed_routed_event_arguments_are_read_through_the_live_handle() {
    crate::register_types();
    let tapped = MarkupType::find("FerroUI.Input", "TappedEventArgs").unwrap();
    assert_eq!((tapped.base.unwrap())(), ValueType::of::<Rc<dyn crate::interactivity::IRoutedEventArgs>>());
    // A deriving arguments class names its base.
    let pressed = MarkupType::find("FerroUI.Input", "PointerPressedEventArgs").unwrap();
    assert_eq!((pressed.base.unwrap())(), ValueType::of::<crate::input::PointerEventArgs>());
    assert!(MarkupType::find("FerroUI.Input", "PointerEventArgs").unwrap().find_property("KeyModifiers").is_some());

    let scroll = MarkupType::find("FerroUI.Input", "ScrollGestureEventArgs").unwrap();
    let arguments = crate::input::ScrollGestureEventArgs::new(7, crate::Vector::new(1.0, 2.0));
    let shared = boxed(crate::interactivity::IRoutedEventArgs::share(&arguments));
    let delta = scroll.find_property("Delta").unwrap();
    assert_eq!(unbox::<crate::Vector>(&(delta.get.unwrap())(&[shared.clone()]).unwrap()), crate::Vector::new(1.0, 2.0));
    assert_eq!(unbox::<i32>(&(scroll.find_property("Id").unwrap().get.unwrap())(&[shared.clone()]).unwrap()), 7);
    // The members of the base arguments apply to the same handle.
    let base = MarkupType::find("FerroUI.Interactivity", "RoutedEventArgs").unwrap();
    (base.find_property("Handled").unwrap().set.unwrap())(&[shared.clone(), boxed(true)]).unwrap();
    assert!(arguments.handled());
    // Arguments of another type are an error, not a panic.
    let other = boxed(crate::interactivity::IRoutedEventArgs::share(&crate::interactivity::RoutedEventArgs::new()));
    assert!((delta.get.unwrap())(&[other]).is_err());
}

#[test]
fn contracts_give_back_the_objects_that_implement_them() {
    // An animation and a transition are objects of the object model.
    let animation = crate::animation::Animation::new();
    let contract: Rc<dyn crate::animation::IAnimation> = animation.clone().into();
    assert_eq!(contract.as_object().unwrap().cast::<crate::animation::Animation>(), Some(animation));
    let transition = crate::animation::DoubleTransition::new();
    let contract: Rc<dyn crate::animation::ITransition> = transition.clone().into();
    assert_eq!(contract.as_object().unwrap().cast::<crate::animation::DoubleTransition>(), Some(transition));

    // A setter is a plain shared object.
    let setter = crate::styling::Setter::empty();
    let base: Rc<dyn crate::styling::SetterBase> = setter.clone();
    assert!(std::ptr::eq(base.as_any().unwrap().downcast_ref::<crate::styling::Setter>().unwrap(), &*setter));
    let animation_setter: Rc<dyn crate::animation::IAnimationSetter> = setter.clone();
    assert!(animation_setter.as_any().unwrap().is::<crate::styling::Setter>());

    // Attaching a style outside of a styling pass applies its setters.
    let element = InputElement::new();
    let style = Style::with_setters(
        crate::styling::Selectors::of_type_info(None, InputElement::TYPE),
        [crate::styling::Setter::new(InputElement::focusable_property(), true)],
    );
    assert!(!element.focusable());
    crate::styling::testing::try_attach(&style, &element, None);
    assert!(element.focusable());
}

#[test]
fn bindings_and_property_infos_give_back_their_concrete_type() {
    let binding = crate::data::ReflectionBinding::new("Name");
    let base: Rc<dyn crate::data::BindingBase> = binding.clone();
    assert!(std::ptr::eq(base.as_any().unwrap().downcast_ref::<crate::data::ReflectionBinding>().unwrap(), &*binding));
    let info: Rc<dyn crate::data::core::IPropertyInfo> =
        Rc::new(crate::data::core::ClrPropertyInfo::new("P", None, None, ValueType::of::<i32>()));
    assert!(info.as_any().unwrap().is::<crate::data::core::ClrPropertyInfo>());
    let property: &dyn crate::data::core::IPropertyInfo = StyledElement::name_property().as_property();
    assert!(property.as_any().unwrap().is::<crate::FerroProperty>());

    // A style that has an owner cannot be added to owned styles: an error, not a panic.
    let host = StyledElement::new();
    let other = StyledElement::new();
    let style = Style::new();
    host.styles().add(style.clone());
    let add = class_markup::<Styles>().find_methods("Add").next().unwrap();
    let error = (add.invoke)(&[into_markup_value(other.styles()), into_markup_value(style)]).unwrap_err();
    assert_eq!(error.to_string(), "The Style already has a parent.");
}

// A gap of the untyped value conversions, for the core to fix.

struct CastBase {
    value: i32,
}

impl PartialEq for CastBase {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

struct CastDerived {
    base: Rc<CastBase>,
}

impl PartialEq for CastDerived {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

#[test]
fn registered_casts_of_reference_types_are_assignability() {
    ValueTypes::register_reference::<CastBase>();
    ValueTypes::register_reference::<CastDerived>();
    // The derived reference object is a base reference object.
    ValueTypes::register_boxed_cast::<CastDerived, Rc<CastBase>>(|object| {
        let any: Rc<dyn std::any::Any> = object.clone();
        any.downcast::<CastDerived>().ok().map(|derived| derived.base.clone())
    });

    let derived = Rc::new(CastDerived { base: Rc::new(CastBase { value: 7 }) });
    // Object form (the box is the object) and handle form.
    let object: BoxedValue = derived.clone();
    let handle: BoxedValue = Rc::new(derived.clone());

    let base_handle = ValueType::of::<Rc<CastBase>>();
    let nullable_base = ValueType::of::<Option<Rc<CastBase>>>();
    let base_object = ValueType::of::<CastBase>();
    for target in [base_handle, nullable_base, base_object] {
        assert!(ValueTypes::is_assignable(ValueType::of::<CastDerived>(), target), "object form -> {target}");
        assert!(ValueTypes::is_assignable(ValueType::of::<Rc<CastDerived>>(), target), "handle form -> {target}");
        assert!(ValueTypes::try_cast(&object, target).is_some(), "object form -> {target}");
        assert!(ValueTypes::try_cast(&handle, target).is_some(), "handle form -> {target}");
    }
    let cast = ValueTypes::try_cast(&object, base_handle).unwrap();
    assert_eq!(cast.downcast_ref::<Rc<CastBase>>().unwrap().value, 7);
    assert_eq!(from_markup_value::<Rc<CastBase>>(&Some(handle)).unwrap().value, 7);
}

#[test]
fn list_instantiations_have_a_capacity_reached_through_the_named_collections() {
    crate::register_types();
    // Every instantiation of the notifying list has `Capacity`, `Count` and `Add(T)`.
    let mut instantiations = 0;
    for type_ in declared_types() {
        let Some(generic) = type_.generic.filter(|generic| generic.definition == "FerroList`1") else { continue };
        instantiations += 1;
        let capacity = type_.find_property("Capacity").unwrap_or_else(|| panic!("{}: no Capacity", type_.name));
        assert_eq!((capacity.type_)(), ValueType::of::<i32>());
        assert!(capacity.get.is_some() && capacity.set.is_some());
        assert_eq!((type_.find_property("Count").unwrap().type_)(), ValueType::of::<i32>());
        let add = type_.find_methods("Add").next().unwrap();
        // The instance and the item.
        assert_eq!(add.parameters.len(), 1);
        assert_eq!((add.parameters[0])(), (generic.arguments[0])());
    }
    assert!(instantiations >= 19);

    // A named collection is created by its own constructor and is the list it derives from.
    let points = <crate::media::Points as MarkupTyped>::MARKUP;
    let list = MarkupType::find_by_handle((points.base.unwrap())().id()).unwrap();
    let capacity = list.find_property("Capacity").unwrap();
    let collection = (points.constructors[0].invoke)(&[]).unwrap();
    (capacity.set.unwrap())(&[collection.clone(), boxed(8i32)]).unwrap();
    assert_eq!(unbox::<i32>(&(capacity.get.unwrap())(&[collection.clone()]).unwrap()), 8);
    (points.find_methods("Add").next().unwrap().invoke)(&[collection.clone(), boxed(crate::Point::new(1.0, 2.0))])
        .unwrap();
    assert_eq!(unbox::<i32>(&(list.find_property("Count").unwrap().get.unwrap())(&[collection.clone()]).unwrap()), 1);
    // A capacity below the number of items is an error, not a panic.
    assert!((capacity.set.unwrap())(&[collection.clone(), boxed(0i32)]).is_err());
    assert!((capacity.set.unwrap())(&[collection.clone(), boxed(-1i32)]).is_err());

    // The classes of an element keep their list to themselves and declare its capacity.
    let classes = <crate::controls::Classes as MarkupTyped>::MARKUP;
    let capacity = classes.find_property("Capacity").unwrap();
    let collection = (classes.constructors[0].invoke)(&[]).unwrap();
    (capacity.set.unwrap())(&[collection.clone(), boxed(2i32)]).unwrap();
    assert_eq!(unbox::<i32>(&(capacity.get.unwrap())(&[collection.clone()]).unwrap()), 2);
    (classes.find_methods("Add").next().unwrap().invoke)(&[collection.clone(), text("a")]).unwrap();
    assert!((capacity.set.unwrap())(&[collection.clone(), boxed(0i32)]).is_err());

    // The named collections that derive from the list are assignable to it.
    for (named, base) in [
        (ValueType::of::<crate::media::Points>(), ValueType::of::<crate::collections::FerroList<crate::Point>>()),
        (ValueType::of::<GradientStops>(), ValueType::of::<crate::collections::FerroList<Ref<GradientStop>>>()),
        (ValueType::of::<Transitions>(), ValueType::of::<crate::collections::FerroList<Rc<dyn crate::animation::ITransition>>>()),
        (ValueType::of::<PathFigures>(), ValueType::of::<crate::collections::FerroList<Ref<crate::media::PathFigure>>>()),
    ] {
        assert!(ValueTypes::is_assignable(named, base), "{named} -> {base}");
    }
}

#[test]
fn a_setter_is_an_animation_setter_and_the_converters_are_static_values() {
    crate::register_types();
    let setter = <crate::styling::Setter as MarkupTyped>::MARKUP;
    assert!(setter.interfaces.iter().any(|i| i() == ValueType::of::<Rc<dyn crate::animation::IAnimationSetter>>()));
    let value = (setter.constructors[0].invoke)(&[]).unwrap();
    assert!(from_markup_value::<Rc<dyn crate::animation::IAnimationSetter>>(&value).is_some());
    // The setters of a key frame accept it.
    let setters = <crate::collections::FerroList<Rc<dyn crate::animation::IAnimationSetter>> as MarkupTyped>::MARKUP;
    let list = (setters.constructors[0].invoke)(&[]).unwrap();
    (setters.find_methods("Add").next().unwrap().invoke)(&[list.clone(), value]).unwrap();
    assert_eq!(unbox::<crate::collections::FerroList<Rc<dyn crate::animation::IAnimationSetter>>>(&list).count(), 1);

    for (type_, fields) in [
        ("ObjectConverters", &["IsNull", "IsNotNull", "Equal", "NotEqual", "AreAllNull", "AreAnyNull", "AreAllEqual"][..]),
        ("BoolConverters", &["And", "Or", "Not"][..]),
        ("StringConverters", &["IsNullOrEmpty", "IsNotNullOrEmpty"][..]),
    ] {
        let converters = MarkupType::find("FerroUI.Data.Converters", type_).unwrap();
        assert_eq!(converters.kind, MarkupTypeKind::Static);
        assert_eq!(converters.fields.iter().map(|f| f.name).collect::<Vec<_>>(), fields);
        for field in converters.fields {
            assert!((field.get)().is_some(), "{type_}.{}", field.name);
        }
    }
    let is_not_null = MarkupType::find("FerroUI.Data.Converters", "ObjectConverters").unwrap().find_field("IsNotNull").unwrap();
    assert_eq!((is_not_null.type_)(), ValueType::of::<Rc<dyn crate::data::converters::IValueConverter>>());
    let converter = unbox::<Rc<dyn crate::data::converters::IValueConverter>>(&(is_not_null.get)());
    let converted = converter.convert(Some(&(Rc::new(1i32) as BoxedValue)), ValueType::of::<bool>(), None, &crate::utilities::CultureInfo::invariant_culture()).unwrap();
    assert_eq!(unbox::<bool>(&converted), true);
}

#[test]
fn a_setter_value_is_classified_as_the_setter_instantiates_it() {
    use crate::styling::{ITemplate, Setter, SetterValue};

    struct Built;
    impl ITemplate for Built {
        fn build(&self) -> BoxedValue {
            Rc::new(1i32)
        }
    }

    crate::register_types();
    let markup = <Setter as MarkupTyped>::MARKUP;
    let value = markup.find_property("Value").unwrap();
    let setter = Setter::empty();
    let instance = into_markup_value(setter.clone());
    // A property of any object: a template builds its value.
    setter.set_property(Some(StyledElement::data_context_property().as_property()));
    let template: Rc<dyn ITemplate> = Rc::new(Built);
    (value.set.unwrap())(&[instance.clone(), boxed(template.clone())]).unwrap();
    assert!(matches!(setter.value(), Some(SetterValue::Template(t)) if *t == *template));
    // The same template in an "any value" box, the form of a value of exactly the type of the property.
    let any: Option<BoxedValue> = Some(Rc::new(template.clone()));
    (value.set.unwrap())(&[instance.clone(), Some(Rc::new(any))]).unwrap();
    assert!(matches!(setter.value(), Some(SetterValue::Template(t)) if *t == *template));
    // A binding in the box is the binding.
    let binding: Option<BoxedValue> = Some(Rc::new(crate::data::ReflectionBinding::new("Name")));
    (value.set.unwrap())(&[instance.clone(), Some(Rc::new(binding))]).unwrap();
    assert!(matches!(setter.value(), Some(SetterValue::BindingBase(_))));
    // Anything else is the value, as it was given.
    let text: Option<BoxedValue> = Some(Rc::new("x".to_string()));
    (value.set.unwrap())(&[instance.clone(), Some(Rc::new(text))]).unwrap();
    assert!(matches!(setter.value(), Some(SetterValue::Value(v)) if v.is::<Option<BoxedValue>>()));
}

#[test]
fn value_types_have_the_text_of_the_managed_original() {
    crate::register_types();
    let text_of = |value: BoxedValue| ValueTypes::try_to_string(&*value).unwrap();
    // A known color is its name, any other its hexadecimal form.
    assert_eq!(text_of(Rc::new(Colors::WHITE)), "White");
    assert_eq!(text_of(Rc::new(Color::from_argb(1, 2, 3, 4))), "#01020304");
    assert_eq!(text_of(Rc::new(Thickness::new(1.0, 2.0, 3.0, 4.0))), Thickness::new(1.0, 2.0, 3.0, 4.0).to_string());
    assert_eq!(text_of(Rc::new(crate::Point::new(1.0, 2.0))), "1, 2");
    assert_eq!(text_of(Rc::new(crate::media::FontFamily::new("Arial"))), "Arial");
    // An enumeration is the name of its member; a type without a text of its own is its name.
    assert_eq!(text_of(Rc::new(crate::media::FontStyle::Italic)), "Italic");
    assert_eq!(text_of(Rc::new(crate::media::GradientStops::new())), "FerroUI.Media.GradientStops");
}

#[test]
fn a_theme_variant_provider_has_its_key() {
    crate::register_types();
    let contract = MarkupType::find("FerroUI.Controls", "IThemeVariantProvider").unwrap();
    let key = contract.find_property("Key").unwrap();
    assert_eq!((key.type_)(), ValueType::of::<Option<crate::styling::ThemeVariant>>());
    // A resource dictionary is the provider: the key is set on the dictionary itself.
    let dictionary = ResourceDictionary::new();
    let instance = into_markup_value(dictionary.clone());
    (key.set.unwrap())(&[instance.clone(), boxed(crate::styling::ThemeVariant::dark())]).unwrap();
    assert!(dictionary.key() == Some(crate::styling::ThemeVariant::dark()));
    let read = (key.get.unwrap())(&[instance.clone()]).unwrap();
    assert!(unbox::<Option<crate::styling::ThemeVariant>>(&read) == Some(crate::styling::ThemeVariant::dark()));
    (key.set.unwrap())(&[instance, None]).unwrap();
    assert!(dictionary.key().is_none());
    assert!(ValueTypes::is_assignable(
        ValueType::of::<Rc<dyn crate::controls::IThemeVariantProvider>>(),
        ValueType::of::<Rc<dyn crate::controls::IResourceProvider>>()
    ));
}

#[test]
fn contracts_and_geometry_classes_convert_from_text() {
    crate::register_types();
    // A transform contract value from text is a list of transform operations; `none` is the identity.
    let parse = <dyn crate::media::ITransform as MarkupTyped>::MARKUP.parse.unwrap();
    let none = unbox::<Rc<dyn crate::media::ITransform>>(&parse(&[text("none")]).unwrap());
    assert_eq!(none.value(), crate::Matrix::IDENTITY);
    let rotated = unbox::<Rc<dyn crate::media::ITransform>>(&parse(&[text("rotate(90deg)")]).unwrap());
    assert!(rotated.value() != crate::Matrix::IDENTITY);
    assert!(parse(&[text("no transform")]).is_err());
    // The other contracts with a converter in the managed original.
    assert!(<dyn IBrush as MarkupTyped>::MARKUP.parse.is_some());
    assert!(<dyn crate::media::effects::IEffect as MarkupTyped>::MARKUP.parse.is_some());
    // The text content of a geometry element is its path data: the classes a document names
    // convert to themselves.
    assert!(class_markup::<Geometry>().parse.is_some());
    assert!(class_markup::<crate::media::StreamGeometry>().parse.is_some());
    assert!(class_markup::<crate::media::PathGeometry>().parse.is_some());
    let figures = class_markup::<crate::media::PathGeometry>().parse.unwrap();
    let geometry = figures(&[text("M 0,0 L 10,10 Z")]).unwrap();
    assert!(from_markup_value::<Ref<crate::media::PathGeometry>>(&geometry).is_some());
}

#[test]
fn easings_and_page_transitions_are_created_and_configured_through_metadata() {
    use crate::animation::easings::{Easing, IEasing, SplineEasing};
    use crate::animation::{CrossFade, IPageTransition, PageSlide, SlideAxis};

    crate::register_types();
    let easings = "FerroUI.Animation.Easings";
    // Every easing the easing text names is a class with a parameterless constructor that is
    // an easing and an easing contract.
    for name in [
        "BackEaseIn", "BackEaseInOut", "BackEaseOut", "BounceEaseIn", "BounceEaseInOut", "BounceEaseOut",
        "CircularEaseIn", "CircularEaseInOut", "CircularEaseOut", "CubicEaseIn", "CubicEaseInOut", "CubicEaseOut",
        "ElasticEaseIn", "ElasticEaseInOut", "ElasticEaseOut", "ExponentialEaseIn", "ExponentialEaseInOut",
        "ExponentialEaseOut", "LinearEasing", "QuadraticEaseIn", "QuadraticEaseInOut", "QuadraticEaseOut",
        "QuarticEaseIn", "QuarticEaseInOut", "QuarticEaseOut", "QuinticEaseIn", "QuinticEaseInOut", "QuinticEaseOut",
        "SineEaseIn", "SineEaseInOut", "SineEaseOut", "SplineEasing", "SpringEasing",
    ] {
        let type_ = MarkupType::find(easings, name).unwrap_or_else(|| panic!("{name}"));
        assert_eq!((type_.base.unwrap())(), ValueType::of::<Easing>(), "{name}");
        let value = (type_.constructors.iter().find(|c| c.parameters.is_empty()).unwrap().invoke)(&[]).unwrap();
        assert!(from_markup_value::<Easing>(&value).is_some(), "{name}");
        assert!(from_markup_value::<Rc<dyn IEasing>>(&value).is_some(), "{name}");
        // The text form of the easing class names the same easing.
        assert!(Easing::parse(name).is_ok(), "{name}");
    }

    // <SplineEasing X1='0.16' Y1='1' X2='0.3' Y2='1' />
    let spline = MarkupType::find(easings, "SplineEasing").unwrap();
    let easing = (spline.constructors.iter().find(|c| c.parameters.is_empty()).unwrap().invoke)(&[]).unwrap();
    for (name, value) in [("X1", 0.16), ("Y1", 1.0), ("X2", 0.3), ("Y2", 1.0)] {
        (spline.find_property(name).unwrap().set.unwrap())(&[easing.clone(), boxed(value)]).unwrap();
    }
    let concrete = unbox::<Rc<SplineEasing>>(&easing);
    assert_eq!((concrete.x1(), concrete.y1(), concrete.x2(), concrete.y2()), (0.16, 1.0, 0.3, 1.0));

    // <PageSlide Duration='0:0:0.3' Orientation='Horizontal' FillMode='Forward' SlideInEasing=..>
    let slide = MarkupType::find("FerroUI.Animation", "PageSlide").unwrap();
    let transition = (slide.constructors.iter().find(|c| c.parameters.is_empty()).unwrap().invoke)(&[]).unwrap();
    (slide.find_property("Duration").unwrap().set.unwrap())(&[transition.clone(), boxed(TimeSpan::from_seconds(0.3))])
        .unwrap();
    (slide.find_property("Orientation").unwrap().set.unwrap())(&[transition.clone(), boxed(SlideAxis::Vertical)]).unwrap();
    (slide.find_property("FillMode").unwrap().set.unwrap())(&[transition.clone(), boxed(crate::animation::FillMode::Forward)])
        .unwrap();
    // The easing element is assigned as it was created (its concrete handle).
    (slide.find_property("SlideInEasing").unwrap().set.unwrap())(&[transition.clone(), easing.clone()]).unwrap();
    let concrete = unbox::<Rc<PageSlide>>(&transition);
    assert_eq!(concrete.duration(), TimeSpan::from_seconds(0.3));
    assert_eq!(concrete.orientation(), SlideAxis::Vertical);
    assert!(from_markup_value::<Rc<dyn IPageTransition>>(&transition).is_some());

    // <CrossFade Duration='00:00:00.25' />
    let fade = MarkupType::find("FerroUI.Animation", "CrossFade").unwrap();
    let transition = (fade.constructors.iter().find(|c| c.parameters.is_empty()).unwrap().invoke)(&[]).unwrap();
    (fade.find_property("Duration").unwrap().set.unwrap())(&[transition.clone(), boxed(TimeSpan::from_seconds(0.25))])
        .unwrap();
    assert_eq!(unbox::<Rc<CrossFade>>(&transition).duration(), TimeSpan::from_seconds(0.25));
    assert!(from_markup_value::<Option<Rc<dyn IPageTransition>>>(&transition).is_some_and(|t| t.is_some()));
    for name in ["FadeInEasing", "FadeOutEasing", "FillMode"] {
        assert!(fade.find_property(name).is_some(), "{name}");
    }
    for name in ["Rotate3DTransition", "CompositePageTransition"] {
        let type_ = MarkupType::find("FerroUI.Animation", name).unwrap();
        let value = (type_.constructors.iter().find(|c| c.parameters.is_empty()).unwrap().invoke)(&[]).unwrap();
        assert!(from_markup_value::<Rc<dyn IPageTransition>>(&value).is_some(), "{name}");
    }
}

#[test]
fn the_converters_of_this_crate_are_declared() {
    use crate::data::converters::{IMultiValueConverter, IValueConverter, StringFormatValueConverter};

    crate::register_types();
    let find = |name: &str| MarkupType::find("FerroUI.Data.Converters", name).unwrap();
    let instance = (find("DefaultValueConverter").find_field("Instance").unwrap().get)();
    assert!(from_markup_value::<Rc<dyn IValueConverter>>(&instance).is_some());
    let format = find("StringFormatValueConverter");
    let converter = (format.constructors[0].invoke)(&[text("{0:0.0}"), None]).unwrap();
    assert_eq!(unbox::<Rc<StringFormatValueConverter>>(&converter).format(), "{0:0.0}");
    assert!(from_markup_value::<Rc<dyn IValueConverter>>(&converter).is_some());
    assert_eq!(unbox::<String>(&(format.find_property("Format").unwrap().get.unwrap())(&[converter]).unwrap()), "{0:0.0}");
    let multi = find("StringFormatMultiValueConverter");
    let converter = (multi.constructors[0].invoke)(&[text("{0} {1}"), None]).unwrap();
    assert!(from_markup_value::<Rc<dyn IMultiValueConverter>>(&converter).is_some());
}

#[test]
fn an_attached_property_a_class_exposes_on_its_instances_is_an_instance_property() {
    use crate::media::FlowDirection;
    use crate::Visual;

    crate::register_types();
    // `<Border FlowDirection='LeftToRight'/>`: `Visual.FlowDirection` is registered as an
    // attached property and is a property of every visual.
    let property = class_markup::<Visual>().find_property("FlowDirection").expect("the instance property");
    assert_eq!((property.type_)(), ValueType::of::<FlowDirection>());
    let visual = Visual::new();
    let instance = into_markup_value(visual.clone());
    (property.set.unwrap())(&[instance.clone(), boxed(FlowDirection::RightToLeft)]).unwrap();
    assert_eq!(visual.get_value(Visual::flow_direction_property()), FlowDirection::RightToLeft);
    assert_eq!(unbox::<FlowDirection>(&(property.get.unwrap())(&[instance]).unwrap()), FlowDirection::RightToLeft);
}

#[test]
fn the_marker_values_are_static_fields() {
    crate::register_types();
    let unset = <crate::FerroProperty as MarkupTyped>::MARKUP.find_field("UnsetValue").unwrap();
    assert_eq!((unset.type_)(), ValueType::of::<crate::UnsetValueType>());
    assert!((unset.get)().is_some_and(|value| value.is::<crate::UnsetValueType>()));
    let do_nothing = MarkupType::find("FerroUI.Data", "BindingOperations").unwrap().find_field("DoNothing").unwrap();
    assert!((do_nothing.get)().is_some_and(|value| value.is::<crate::DoNothingType>()));
}

#[test]
fn every_value_type_has_its_default_value_constructor() {
    for type_ in declared_types() {
        if type_.kind != MarkupTypeKind::Struct {
            continue;
        }
        // The unset marker is a class without a public constructor in the managed original.
        if type_.name == "UnsetValueType" {
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
fn the_types_written_as_elements_with_text_convert_from_their_own_text() {
    // What the theme documents write as `<Type x:Key='k'>text</Type>`: each type is found by
    // name and converts the text itself (the text conversion declared on the type).
    crate::register_types();
    for (namespace, name, sample) in [
        ("FerroUI", "Thickness", "1,2,3,4"),
        ("FerroUI", "CornerRadius", "4"),
        ("FerroUI", "Point", "1,2"),
        ("FerroUI", "Size", "1,2"),
        ("FerroUI", "Rect", "1,2,3,4"),
        ("FerroUI", "Vector", "1,2"),
        ("FerroUI", "Matrix", "1,0,0,1,0,0"),
        ("FerroUI", "RelativePoint", "50%,50%"),
        ("FerroUI.Media", "Color", "#FF102030"),
        ("FerroUI.Media", "FontFamily", "Arial"),
        ("FerroUI.Media", "FontWeight", "Bold"),
        ("FerroUI.Media", "BoxShadows", "0 0 5 0 Black"),
        ("FerroUI.Media.Transformation", "TransformOperations", "scaleX(0.125) translateX(-2px)"),
        ("FerroUI.Animation.Easings", "Easing", "CubicEaseOut"),
        ("FerroUI.Animation", "KeySpline", "0,0,0,1"),
    ] {
        let type_ = MarkupType::find(namespace, name)
            .or_else(|| crate::TypeInfo::find(namespace, name).and_then(|class| class.markup()))
            .unwrap_or_else(|| panic!("{namespace}.{name} is not found by name"));
        let parse = type_.parse.unwrap_or_else(|| panic!("{namespace}.{name} has no text conversion"));
        let value = parse(&[text(sample)]).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(value.is_some(), "{name}");
    }
    // The class markups of the geometries.
    for parse in [class_markup::<crate::media::StreamGeometry>().parse, class_markup::<crate::media::PathGeometry>().parse] {
        assert!(parse.is_some());
    }
    let parsed = (<TransformOperationsType as MarkupTyped>::MARKUP.parse.unwrap())(&[text("scaleX(0.125) translateX(-2px)")]).unwrap();
    let operations = unbox::<Rc<TransformOperationsType>>(&parsed);
    assert_eq!(operations.operations().len(), 2);
    assert!(from_markup_value::<Rc<dyn crate::media::ITransform>>(&parsed).is_some());
}

use crate::media::transformation::TransformOperations as TransformOperationsType;

// --- A setter whose value is null.

/// A style with one setter of `property` whose value was set to null through metadata.
fn style_with_null_setter(property: &'static crate::FerroProperty) -> Ref<Style> {
    use crate::styling::{Selectors, Setter};

    crate::register_types();
    let setter = Setter::empty();
    setter.set_property(Some(property));
    let value = <Setter as MarkupTyped>::MARKUP.find_property("Value").unwrap();
    (value.set.unwrap())(&[into_markup_value(setter.clone()), None]).unwrap();
    assert!(setter.value().is_none());
    let style = Style::with_selector(Selectors::of_type::<crate::Visual>());
    style.setters().add(setter);
    style
}

#[test]
fn a_null_setter_value_sets_the_null_of_a_property_that_admits_it() {
    use crate::diagnostics::FerroObjectDiagnosticExtensions;
    use crate::styling::testing::try_attach;
    use crate::Visual;

    // A contract handle, a class handle and any object.
    let brush = Visual::opacity_mask_property().as_property();
    let geometry = Visual::clip_property().as_property();
    let any = StyledElement::data_context_property().as_property();
    for property in [brush, geometry, any] {
        let visual = Visual::new();
        try_attach(&style_with_null_setter(property), &visual, None);
        let diagnostic = visual.get_diagnostic(property);
        // The value of the setter is in effect, and it is null.
        assert_eq!(diagnostic.priority(), crate::data::BindingPriority::Style, "{}", property.name());
        assert!(ValueTypes::normalize(diagnostic.value().clone()).is_none(), "{}", property.name());
    }
    assert!(Visual::new().opacity_mask().is_none());

    // The untyped `SetValue` takes null for such a property too.
    let visual = Visual::new();
    visual.set_opacity_mask(Some(Brushes::red() as Rc<dyn IBrush>));
    let set_value = class_markup::<FerroObject>().find_methods("SetValue").next().unwrap();
    (set_value.invoke)(&[into_markup_value(visual.clone()), boxed(brush), None, boxed(crate::data::BindingPriority::LocalValue)])
        .unwrap();
    assert!(visual.opacity_mask().is_none());
    // And reports a value type that has no null.
    let opacity = Visual::opacity_property().as_property();
    assert!((set_value.invoke)(&[into_markup_value(visual), boxed(opacity), None, boxed(crate::data::BindingPriority::LocalValue)])
        .is_err());
}

#[test]
#[should_panic(expected = "Setter value '(null)' is not a valid value for property 'Opacity'.")]
fn a_null_setter_value_is_not_a_value_of_a_number() {
    let visual = crate::Visual::new();
    crate::styling::testing::try_attach(&style_with_null_setter(crate::Visual::opacity_property().as_property()), &visual, None);
}

#[test]
#[should_panic(expected = "Setter value '(null)' is not a valid value for property 'Margin'.")]
fn a_null_setter_value_is_not_a_value_of_a_structure() {
    let visual = crate::Visual::new();
    let margin = crate::layout::Layoutable::margin_property().as_property();
    crate::styling::testing::try_attach(&style_with_null_setter(margin), &visual, None);
}


#[test]
fn property_changes_reach_a_handler_attached_through_metadata() {
    crate::register_types();
    // The event every object has (`FerroObject.PropertyChanged`), which markup attaches a
    // handler to (`PropertyChanged="Handler"` on an element).
    let markup = FerroObject::TYPE.markup().expect("the markup metadata of the class");
    let event = markup.find_event("PropertyChanged").unwrap();
    assert_eq!(
        event.arguments.iter().map(|a| a()).collect::<Vec<_>>(),
        [ValueType::of::<Option<BoxedValue>>(), ValueType::of::<OwnedFerroPropertyChangedEventArgs>()]
    );
    let arguments_type = MarkupType::find("FerroUI", "FerroPropertyChangedEventArgs").unwrap();
    assert!(std::ptr::eq(arguments_type, <OwnedFerroPropertyChangedEventArgs as MarkupTyped>::MARKUP));

    let element = StyledElement::new();
    let changes: Rc<RefCell<Vec<OwnedFerroPropertyChangedEventArgs>>> = Rc::new(RefCell::new(Vec::new()));
    let handler = MarkupDelegate::new({
        let (changes, element) = (changes.clone(), element.clone());
        move |arguments| {
            assert_eq!(arguments.len(), 2);
            assert_eq!(unbox::<Ref<StyledElement>>(&arguments[0]), element);
            changes.borrow_mut().push(unbox::<OwnedFerroPropertyChangedEventArgs>(&arguments[1]));
            None
        }
    });
    (event.add)(&[into_markup_value(element.clone()), boxed(handler)]).unwrap();
    element.set_data_context(Some(Rc::new(1i32) as BoxedValue));

    // The handler received the change held by value: the values are copies in untyped form.
    let changes = changes.borrow();
    let data_context = StyledElement::data_context_property().as_property();
    let change = changes.iter().find(|change| change.property() == data_context).expect("the change of the data context");
    assert!(change.sender() == element.clone().upcast::<FerroObject>());
    assert!(change.old_value().is_none());
    assert_eq!(change.new_value().and_then(|value| value.downcast_ref::<i32>().copied()), Some(1));
    assert!(change.is_effective_value_change());
    let new_value = (arguments_type.find_property("NewValue").unwrap().get.unwrap())(&[boxed(change.clone())]).unwrap();
    assert_eq!(unbox::<i32>(&new_value), 1);
}
