//! Tests of the type converters. Ports of the parts of the upstream
//! converter tests that do not load markup
//! (`Converters/PointsListTypeConverterTests.cs`,
//! `Converters/FerroPropertyConverterTest.cs`), and direct tests.

use super::*;
use crate::templates::ControlTemplate;
use crate::test_support::{boxed, TestAssetLoader, TestServiceProvider};
use crate::xaml_il::runtime::IFerroXamlIlParentStackProvider;
use crate::{register_types, IXamlTypeResolver, XamlLoadException};
use ferroui_base::animation::TimeSpan;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::expression_nodes::CastTarget;
use ferroui_base::data::core::ValueType;
use ferroui_base::media::{Brushes, Color, Colors, FontFamily, IBrush, SolidColorBrush};
use ferroui_base::metadata::{from_markup_value, into_markup_value, service, IServiceProvider, MarkupTyped};
use ferroui_base::styling::{Selectors, Style};
use ferroui_base::utilities::Uri;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_properties, ferro_static_type, instantiate, AttachedProperty, BoxedValue,
    FerroObjectImpl, FerroProperty, Point, Ref, StaticType, StyledElement, StyledElementImpl, StyledProperty, TypeInfo,
};
use ferroui_controls::Button;
use std::any::{Any, TypeId};
use std::rc::Rc;

fn convert(converter: &dyn TypeConverter, text: &str) -> Result<Option<BoxedValue>, XamlLoadException> {
    converter.convert_from(None, None, Some(&boxed(text.to_string())))
}

fn context(provider: Rc<TestServiceProvider>) -> Rc<dyn ITypeDescriptorContext> {
    ServiceProviderTypeDescriptorContext::new(provider.sp())
}

// --- The contract ----------------------------------------------------------

struct Plain;

impl TypeConverter for Plain {}

#[test]
fn the_base_converter_converts_from_nothing_and_to_text() {
    let converter = Plain;
    assert!(!converter.can_convert_from(None, ValueType::of::<String>()));
    let error = converter.convert_from(None, None, Some(&boxed(1i32))).unwrap_err();
    assert_eq!(error.message(), "TypeConverter cannot convert from i32.");
    assert_eq!(converter.convert_from(None, None, None).unwrap_err().message(), "TypeConverter cannot convert from (null).");

    assert!(converter.can_convert_to(None, ValueType::of::<String>()));
    assert!(!converter.can_convert_to(None, ValueType::of::<i32>()));
    let text = converter.convert_to(None, None, Some(&boxed("x".to_string())), ValueType::of::<String>()).unwrap();
    assert_eq!(text.unwrap().downcast_ref::<String>().unwrap(), "x");
    assert!(converter.convert_to(None, None, Some(&boxed(1i32)), ValueType::of::<i32>()).is_err());
}

#[test]
fn every_text_converter_converts_from_text_only() {
    let converters: Vec<Rc<dyn TypeConverter>> = vec![
        BitmapTypeConverter::new(),
        FerroPropertyTypeConverter::new(),
        FerroUriTypeConverter::new(),
        FontFamilyTypeConverter::new(),
        IconTypeConverter::new(),
        PointsListTypeConverter::new(),
        TimeSpanTypeConverter::new(),
    ];
    for converter in converters {
        assert!(converter.can_convert_from(None, ValueType::of::<String>()));
        assert!(!converter.can_convert_from(None, ValueType::of::<i32>()));
    }
}

// --- PointsListTypeConverter ----------------------------------------------

#[test]
fn points_list_type_converter_should_parse() {
    for input in ["1,2 3,4", "1 2 3 4", "1 2,3 4", "1,2,3,4"] {
        let conv = PointsListTypeConverter::new();
        let points = convert(&*conv, input).unwrap().unwrap();
        let points = points.downcast_ref::<Vec<Point>>().unwrap();
        assert_eq!(points.len(), 2, "{input}");
        assert_eq!(points[0], Point::new(1.0, 2.0));
        assert_eq!(points[1], Point::new(3.0, 4.0));
    }
}

#[test]
fn points_list_type_converter_rejects_malformed_lists() {
    let conv = PointsListTypeConverter::new();
    assert!(convert(&*conv, "").unwrap().unwrap().downcast_ref::<Vec<Point>>().unwrap().is_empty());
    assert_eq!(convert(&*conv, "1,2,3").unwrap_err().message(), "Invalid PointsList.");
    assert!(convert(&*conv, "1,a").is_err());
    assert!(conv.convert_from(None, None, Some(&boxed(5i32))).is_err());
    assert!(PointsListTypeConverter::parse("1 2").unwrap() == vec![Point::new(1.0, 2.0)]);
}

// --- TimeSpanTypeConverter ------------------------------------------------

#[test]
fn time_span_type_converter_accepts_seconds_and_time_spans() {
    let conv = TimeSpanTypeConverter::new();
    let value = |text: &str| *convert(&*conv, text).unwrap().unwrap().downcast_ref::<TimeSpan>().unwrap();
    assert_eq!(value("0.25"), TimeSpan::from_seconds(0.25));
    assert_eq!(value("2"), TimeSpan::from_seconds(2.0));
    assert_eq!(value("0:0:1.5"), TimeSpan::from_seconds(1.5));
    assert_eq!(value("00:01:00"), TimeSpan::from_seconds(60.0));
    assert!(convert(&*conv, "abc").is_err());
    assert!(convert(&*conv, "1:xx").is_err());
}

// --- FerroUriTypeConverter ------------------------------------------------

#[test]
fn uri_type_converter_keeps_rooted_paths_relative() {
    let conv = FerroUriTypeConverter::new();
    let value = |text: &str| convert(&*conv, text).unwrap().unwrap().downcast_ref::<Uri>().unwrap().clone();

    assert!(!value("/Assets/icon.png").is_absolute_uri());
    assert_eq!(value("/Assets/icon.png").original_string(), "/Assets/icon.png");
    assert!(!value("Assets/icon.png").is_absolute_uri());
    assert!(value("ferres://app/Assets/icon.png").is_absolute_uri());
    assert_eq!(value("ferres://app/Assets/icon.png").scheme(), "ferres");

    // Anything that is not text converts to null.
    assert_eq!(conv.convert_from(None, None, Some(&boxed(1i32))), Ok(None));
    assert_eq!(conv.convert_from(None, None, None), Ok(None));
    // A scheme without a rest is not a URI.
    assert_eq!(convert(&*conv, "http:").unwrap_err().message(), "Unable to parse URI: http:");
}

// --- FontFamilyTypeConverter ----------------------------------------------

#[test]
fn font_family_type_converter_parses_with_the_base_uri_of_the_context() {
    let conv = FontFamilyTypeConverter::new();
    let family = convert(&*conv, "Courier New").unwrap().unwrap();
    assert_eq!(family.downcast_ref::<FontFamily>().unwrap().name(), "Courier New");

    let with_context = context(TestServiceProvider::new().with_base_uri("ferres://app/Views/Main.xaml"));
    let family = conv.convert_from(Some(&with_context), None, Some(&boxed("Arial".to_string()))).unwrap().unwrap();
    assert_eq!(family.downcast_ref::<FontFamily>().unwrap().name(), "Arial");

    assert!(conv.convert_from(None, None, Some(&boxed(1i32))).is_err());
}

// --- ColorToBrushConverter ------------------------------------------------

#[test]
fn color_to_brush_converter_converts_a_color_for_a_brush_target() {
    let brush_type = Some(ValueType::of::<Rc<dyn IBrush>>());
    let color = Some(boxed(Colors::RED));

    let brush = ColorToBrushConverter::convert_to(color.clone(), brush_type);
    let brush = from_markup_value::<Rc<dyn IBrush>>(&brush).unwrap();
    assert_eq!(brush.as_solid_color_brush().unwrap().color(), Colors::RED);
    // The nullable brush type is the type of brush properties.
    let brush = ColorToBrushConverter::convert_to(color.clone(), Some(ValueType::of::<Option<Rc<dyn IBrush>>>()));
    assert!(brush.unwrap().is::<Rc<dyn IBrush>>());

    // Everything else is returned unchanged.
    assert!(ColorToBrushConverter::convert_to(color.clone(), Some(ValueType::of::<Color>())).unwrap().is::<Color>());
    assert!(ColorToBrushConverter::convert_to(color.clone(), None).unwrap().is::<Color>());
    assert!(ColorToBrushConverter::convert_to(Some(boxed(1i32)), brush_type).unwrap().is::<i32>());
    assert!(ColorToBrushConverter::convert_to(None, brush_type).is_none());
}

#[test]
fn color_to_brush_converter_converts_a_solid_color_brush_back() {
    let color_type = Some(ValueType::of::<Color>());
    let immutable: Rc<dyn IBrush> = Brushes::red();
    let back = ColorToBrushConverter::convert_back_to(Some(boxed(immutable.clone())), color_type);
    assert_eq!(back.unwrap().downcast_ref::<Color>(), Some(&Colors::RED));

    // A brush object is a solid color brush too.
    let object = SolidColorBrush::with_color(Colors::BLUE);
    let back = ColorToBrushConverter::convert_back_to(Some(boxed(object)), color_type);
    assert_eq!(back.unwrap().downcast_ref::<Color>(), Some(&Colors::BLUE));

    // Everything else is returned unchanged.
    let unchanged = ColorToBrushConverter::convert_back_to(Some(boxed(immutable)), Some(ValueType::of::<String>()));
    assert!(unchanged.unwrap().is::<Rc<dyn IBrush>>());
    assert!(ColorToBrushConverter::convert_back_to(Some(boxed(1i32)), color_type).unwrap().is::<i32>());
}

#[test]
fn color_to_brush_converter_is_a_value_converter() {
    register_types();
    let converter = ColorToBrushConverter::new();
    let converted = converter.convert(Some(&boxed(Colors::RED)), ValueType::of::<Rc<dyn IBrush>>(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap();
    assert!(converted.unwrap().is::<Rc<dyn IBrush>>());
    let back = converter.convert_back(converted_brush().as_ref(), ValueType::of::<Color>(), None, &ferroui_base::utilities::CultureInfo::invariant_culture()).unwrap();
    assert!(back.unwrap().is::<Color>());

    let markup = <ColorToBrushConverter as MarkupTyped>::MARKUP;
    assert!((markup.constructors[0].invoke)(&[]).unwrap().is_some());
    let convert = markup.find_methods("Convert").next().unwrap();
    assert!(convert.is_static);
    let result = (convert.invoke)(&[into_markup_value(Colors::RED), into_markup_value(ValueType::of::<Rc<dyn IBrush>>())]);
    assert!(result.unwrap().unwrap().is::<Rc<dyn IBrush>>());
    assert!(markup.find_methods("ConvertBack").next().is_some());
}

fn converted_brush() -> Option<BoxedValue> {
    let brush: Rc<dyn IBrush> = Brushes::red();
    Some(boxed(brush))
}

// --- Bitmap and icon converters --------------------------------------------

#[test]
fn bitmap_and_icon_converters_report_assets_that_do_not_exist() {
    let scope = TestAssetLoader::new().install();
    let with_context = context(TestServiceProvider::new().with_base_uri("ferres://app/Views/Main.xaml"));

    let bitmap = BitmapTypeConverter::new();
    let error = bitmap.convert_from(Some(&with_context), None, Some(&boxed("/Assets/missing.png".to_string()))).unwrap_err();
    assert!(error.message().contains("could not be found"), "{}", error.message());
    assert!(bitmap.convert_from(None, None, Some(&boxed(1i32))).is_err());

    let icon = IconTypeConverter::new();
    let error = icon.convert_from(Some(&with_context), None, Some(&boxed("missing.ico".to_string()))).unwrap_err();
    assert!(error.message().contains("could not be found"), "{}", error.message());
    // Neither text nor a bitmap.
    let error = icon.convert_from(None, None, Some(&boxed(1i32))).unwrap_err();
    assert_eq!(error.message(), "Specified method is not supported.");
    scope.dispose();
}

#[test]
fn asset_uris_keep_rooted_paths_relative_and_recognise_files() {
    use super::bitmap_type_converter::{asset_uri, local_file_path};

    assert!(!asset_uri("/Assets/a.png").unwrap().is_absolute_uri());
    assert!(local_file_path(&asset_uri("/Assets/a.png").unwrap()).is_none());
    assert!(local_file_path(&asset_uri("ferres://app/a.png").unwrap()).is_none());
    assert_eq!(local_file_path(&asset_uri("file:///tmp/my%20icon.png").unwrap()).unwrap(), "/tmp/my icon.png");
}

// --- FerroPropertyTypeConverter -------------------------------------------

#[repr(C)]
struct Class1 {
    base: StyledElement,
}

ferro_class!(Class1: StyledElement);
ferro_impl_classes!(Class1: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl Class1 {
        fn foo_property() -> StyledProperty<String> {
            FerroProperty::register::<Class1, _>("Foo", String::new())
        }
    }
}

impl Class1 {
    fn new() -> Ref<Self> {
        instantiate(Self { base: StyledElement::construct() })
    }
}

struct AttachedOwner;

ferro_static_type!(AttachedOwner);

ferro_properties! {
    impl AttachedOwner {
        fn attached_property() -> AttachedProperty<String> {
            FerroProperty::register_attached::<AttachedOwner, Class1, _>("Attached", String::new())
        }
    }
}

struct TestTypeResolver;

impl IXamlTypeResolver for TestTypeResolver {
    fn resolve(&self, qualified_type_name: &str) -> Result<CastTarget, XamlLoadException> {
        match qualified_type_name {
            "Class1" => Ok(CastTarget::Class(<Class1 as StaticType>::TYPE)),
            "AttachedOwner" => Ok(CastTarget::Class(<AttachedOwner as StaticType>::TYPE)),
            "local:Class1" => Ok(CastTarget::Class(<Class1 as StaticType>::TYPE)),
            "Number" => Ok(CastTarget::Value(ValueType::of::<i32>())),
            other => Err(XamlLoadException::with_message(format!("Unable to resolve type {other}"))),
        }
    }
}

/// A context with a type resolver and a parent stack that only enumerates,
/// as the upstream test mocks them.
struct TestContext {
    parents: Vec<BoxedValue>,
}

struct Parents(Vec<BoxedValue>);

impl IFerroXamlIlParentStackProvider for Parents {
    fn parents(&self) -> Vec<BoxedValue> {
        self.0.clone()
    }
}

impl IServiceProvider for TestContext {
    fn get_service(&self, service_type: TypeId) -> Option<Rc<dyn Any>> {
        service(service_type, || -> Rc<dyn IXamlTypeResolver> { Rc::new(TestTypeResolver) }).or_else(|| {
            service(service_type, || -> Rc<dyn IFerroXamlIlParentStackProvider> { Rc::new(Parents(self.parents.clone())) })
        })
    }
}

impl ITypeDescriptorContext for TestContext {}

fn create_context(style: Option<Ref<Style>>) -> Rc<dyn ITypeDescriptorContext> {
    // Ensure properties are registered.
    let _ = Class1::new();
    <AttachedOwner as StaticType>::TYPE.ensure_class_init();
    Rc::new(TestContext { parents: style.into_iter().map(boxed).collect() })
}

fn class1_style() -> Ref<Style> {
    let style = Style::new();
    style.set_selector(Some(Selectors::of_type::<Class1>()));
    style
}

fn convert_property(context: &Rc<dyn ITypeDescriptorContext>, text: &str) -> Result<&'static FerroProperty, XamlLoadException> {
    let target = FerroPropertyTypeConverter::new();
    let result = target.convert_from(Some(context), None, Some(&boxed(text.to_string())))?;
    Ok(*result.unwrap().downcast_ref::<&'static FerroProperty>().unwrap())
}

fn foo() -> &'static FerroProperty {
    Class1::foo_property()
}

fn attached() -> &'static FerroProperty {
    AttachedOwner::attached_property()
}

#[test]
fn convert_from_finds_fully_qualified_property() {
    let context = create_context(Some(class1_style()));
    assert!(convert_property(&context, "Class1.Foo").unwrap() == foo());
}

#[test]
fn convert_from_uses_selector_target_type() {
    let context = create_context(Some(class1_style()));
    assert!(convert_property(&context, "Foo").unwrap() == foo());
}

#[test]
fn convert_from_finds_attached_property() {
    let context = create_context(Some(class1_style()));
    assert!(convert_property(&context, "AttachedOwner.Attached").unwrap() == attached());
}

#[test]
fn convert_from_finds_attached_property_with_parentheses() {
    let context = create_context(Some(class1_style()));
    assert!(convert_property(&context, "(AttachedOwner.Attached)").unwrap() == attached());
}

#[test]
fn convert_from_throws_for_nonexistent_property() {
    let context = create_context(Some(class1_style()));
    let ex = convert_property(&context, "Nonexistent").unwrap_err();
    assert_eq!(ex.message(), "Could not find property 'Class1.Nonexistent'.");
}

#[test]
fn convert_from_throws_for_nonexistent_attached_property() {
    let context = create_context(Some(class1_style()));
    let ex = convert_property(&context, "AttachedOwner.NonExistent").unwrap_err();
    assert_eq!(ex.message(), "Could not find property 'AttachedOwner.NonExistent'.");
}

#[test]
fn convert_from_resolves_the_owner_with_its_namespace_prefix() {
    let context = create_context(None);
    assert!(convert_property(&context, "local:Class1.Foo").unwrap() == foo());
    // An owner that cannot be resolved is the error of the resolver.
    assert_eq!(convert_property(&context, "x:Missing.Foo").unwrap_err().message(), "Unable to resolve type x:Missing");
    // An owner that is not a class has no registered properties.
    assert_eq!(convert_property(&context, "Number.Foo").unwrap_err().message(), "Could not find type 'Number'.");
}

#[test]
fn convert_from_without_a_style_looks_on_the_control_class() {
    let context = create_context(None);
    assert!(convert_property(&context, "Tag").unwrap() == ferroui_controls::Control::tag_property().as_property());
    assert_eq!(convert_property(&context, "Foo").unwrap_err().message(), "Could not find property 'Control.Foo'.");
}

#[test]
fn convert_from_prefers_the_target_type_of_the_nearest_control_template() {
    let template = ControlTemplate::new();
    template.set_target_type(Some(<Button as StaticType>::TYPE));
    let template: BoxedValue = template;
    let context: Rc<dyn ITypeDescriptorContext> =
        Rc::new(TestContext { parents: vec![boxed(class1_style()), template] });
    let _ = Class1::new();
    let _ = Button::new();

    let property = convert_property(&context, "IsDefault").unwrap();
    assert_eq!(property.name(), "IsDefault");
    assert_eq!(convert_property(&context, "Foo").unwrap_err().message(), "Could not find property 'Button.Foo'.");
}

#[test]
fn convert_from_reports_malformed_text_and_a_missing_context() {
    let context = create_context(None);
    assert_eq!(convert_property(&context, "Foo%").unwrap_err().message(), "Unexpected '%'.");
    assert_eq!(convert_property(&context, "").unwrap_err().message(), "Expected property name.");

    // Without a context an owner cannot be resolved.
    let target = FerroPropertyTypeConverter::new();
    let error = target.convert_from(None, None, Some(&boxed("local:Class1.Foo".to_string()))).unwrap_err();
    assert_eq!(error.message(), "Could not find type 'local:Class1'.");
    // Without an owner the control class is used.
    assert!(target.convert_from(None, None, Some(&boxed("Tag".to_string()))).unwrap().is_some());
    assert!(target.convert_from(None, None, Some(&boxed(1i32))).is_err());
}

// --- Metadata --------------------------------------------------------------

#[test]
fn converters_are_constructed_and_invoked_through_their_metadata() {
    register_types();
    let sp = TestServiceProvider::new().with_base_uri("ferres://app/Main.xaml").sp();

    let markups = [
        <BitmapTypeConverter as MarkupTyped>::MARKUP,
        <FerroPropertyTypeConverter as MarkupTyped>::MARKUP,
        <FerroUriTypeConverter as MarkupTyped>::MARKUP,
        <FontFamilyTypeConverter as MarkupTyped>::MARKUP,
        <IconTypeConverter as MarkupTyped>::MARKUP,
        <PointsListTypeConverter as MarkupTyped>::MARKUP,
        <TimeSpanTypeConverter as MarkupTyped>::MARKUP,
    ];
    for markup in markups {
        assert_eq!(markup.namespace(), "FerroUI.Markup.Xaml.Converters");
        assert_eq!((markup.base.unwrap())(), ValueType::of::<Rc<dyn TypeConverter>>());
        let converter = (markup.constructors[0].invoke)(&[]).unwrap();
        let can = markup.find_methods("CanConvertFrom").next().unwrap();
        // A service provider is accepted as the context.
        let result = (can.invoke)(&[
            converter.clone(),
            into_markup_value(sp.clone()),
            into_markup_value(ValueType::of::<String>()),
        ]);
        assert_eq!(result.unwrap().unwrap().downcast_ref::<bool>(), Some(&true), "{}", markup.name);
        assert!(markup.find_methods("ConvertFrom").next().is_some());
    }

    let markup = <PointsListTypeConverter as MarkupTyped>::MARKUP;
    let converter = (markup.constructors[0].invoke)(&[]).unwrap();
    let convert_from = markup.find_methods("ConvertFrom").next().unwrap();
    let points = (convert_from.invoke)(&[converter, None, None, into_markup_value("1,2 3,4".to_string())]).unwrap();
    assert_eq!(points.unwrap().downcast_ref::<Vec<Point>>().unwrap().len(), 2);

    let markup = <FontFamilyTypeConverter as MarkupTyped>::MARKUP;
    let converter = (markup.constructors[0].invoke)(&[]).unwrap();
    let convert_from = markup.find_methods("ConvertFrom").next().unwrap();
    let family =
        (convert_from.invoke)(&[converter, into_markup_value(sp), None, into_markup_value("Arial".to_string())]).unwrap();
    assert!(family.unwrap().is::<FontFamily>());

    // The contract types carry the name of the component model.
    let contract = <dyn TypeConverter as MarkupTyped>::MARKUP;
    assert_eq!(contract.full_name(), "System.ComponentModel.TypeConverter");
    assert_eq!(<dyn ITypeDescriptorContext as MarkupTyped>::MARKUP.full_name(), "System.ComponentModel.ITypeDescriptorContext");
    let _: Option<&'static TypeInfo> = None;
}

#[test]
fn a_failing_conversion_through_metadata_is_a_failed_invocation() {
    use ferroui_base::metadata::MarkupInvokeError;

    register_types();
    let markup = <PointsListTypeConverter as MarkupTyped>::MARKUP;
    let converter = (markup.constructors[0].invoke)(&[]).unwrap();
    let convert_from = markup.find_methods("ConvertFrom").next().unwrap();
    let result = (convert_from.invoke)(&[converter, None, None, into_markup_value("1,2,3".to_string())]);
    assert_eq!(result, Err(MarkupInvokeError::Failed("Invalid PointsList.".to_string())));

    // A property converter without a type resolver in its context.
    let markup = <FerroPropertyTypeConverter as MarkupTyped>::MARKUP;
    let converter = (markup.constructors[0].invoke)(&[]).unwrap();
    let convert_from = markup.find_methods("ConvertFrom").next().unwrap();
    let sp = TestServiceProvider::new().sp();
    let result = (convert_from.invoke)(&[converter, into_markup_value(sp), None, into_markup_value("a:B.C".to_string())]);
    assert_eq!(result, Err(MarkupInvokeError::Failed("Service IXamlTypeResolver hasn't been registered".to_string())));
}
