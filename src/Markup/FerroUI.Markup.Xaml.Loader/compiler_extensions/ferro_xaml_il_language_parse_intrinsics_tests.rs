//! Tests of `ferro_xaml_il_language_parse_intrinsics.rs`: one test per branch of `try_convert`.

use std::rc::Rc;

use xamlx::ast::{
    IXamlAstValueNode, XamlAstExtensions, XamlAstNewClrObjectNode, XamlAstNodeExtensions,
    XamlAstTextNode, XamlConstantNode, XamlLineInfo, XamlStaticExtensionNode,
    XamlStaticOrTargetedReturnMethodCallNode,
};
use xamlx::diagnostics::XamlDiagnosticSeverity;
use xamlx::exceptions::XamlResult;
use xamlx::testing::FakeCustomAttribute;
use xamlx::type_system::{IXamlType, XamlValue};

use super::{
    split, FerroXamlIlLanguageParseIntrinsics, IXamlCompileTimeValueParser, XamlCompileTimeValue,
    XamlCompileTimeValueParsers,
};
use crate::compiler_extensions::ast_nodes::FerroXamlIlGridUnitType as GridUnitType;
use crate::compiler_extensions::ast_nodes::*;
use crate::testing::{create_test_framework, TestFramework};

fn try_convert(
    fw: &TestFramework,
    text: &str,
    type_: &Rc<dyn IXamlType>,
) -> XamlResult<Option<Rc<dyn IXamlAstValueNode>>> {
    let context = fw.create_context();
    let node: Rc<dyn IXamlAstValueNode> = XamlAstTextNode::with_type(
        &XamlLineInfo::new(3, 5),
        text,
        true,
        Some(fw.t("System.String")),
    );
    FerroXamlIlLanguageParseIntrinsics::try_convert(&context, &node, text, type_, &fw.types)
}

fn convert(fw: &TestFramework, text: &str, type_name: &str) -> Rc<dyn IXamlAstValueNode> {
    match try_convert(fw, text, &fw.t(type_name)) {
        Ok(Some(node)) => node,
        Ok(None) => panic!(
            "{text:?} was not converted to {type_name}: {:?}",
            fw.reported_diagnostics()
        ),
        Err(e) => panic!("{text:?} failed to convert to {type_name}: {e}"),
    }
}

/// Asserts that the conversion fails with exactly one intrinsics diagnostic.
fn assert_parse_error(fw: &TestFramework, text: &str, type_: &Rc<dyn IXamlType>, title: &str) {
    let before = fw.reported_diagnostics().len();
    let result = try_convert(fw, text, type_).expect("no hard error");
    assert!(result.is_none(), "{text:?} unexpectedly converted");
    let diagnostics = fw.reported_diagnostics();
    assert_eq!(diagnostics.len(), before + 1, "{diagnostics:?}");
    let diagnostic = &diagnostics[before];
    assert_eq!(diagnostic.code, "FRN2005");
    assert_eq!(diagnostic.severity, XamlDiagnosticSeverity::Error);
    assert_eq!(diagnostic.min_severity, XamlDiagnosticSeverity::Warning);
    assert_eq!(diagnostic.title, title);
    assert_eq!(diagnostic.line_number, Some(3));
    assert_eq!(diagnostic.line_position, Some(5));
    assert_eq!(diagnostic.document.as_deref(), Some("test.xaml"));
}

/// Asserts that the conversion is declined without any diagnostic.
fn assert_not_converted(fw: &TestFramework, text: &str, type_: &Rc<dyn IXamlType>) {
    let before = fw.reported_diagnostics().len();
    let result = try_convert(fw, text, type_).expect("no hard error");
    assert!(result.is_none(), "{text:?} unexpectedly converted");
    assert_eq!(fw.reported_diagnostics().len(), before);
}

fn clr_type_name(node: &Rc<dyn IXamlAstValueNode>) -> String {
    node.type_().get_clr_type().expect("clr type").full_name()
}

fn constant(node: &Rc<dyn IXamlAstValueNode>) -> XamlValue {
    node.cast::<XamlConstantNode>()
        .unwrap_or_else(|| panic!("{} is not a constant", node.type_name()))
        .constant
        .clone()
}

fn vector_like(node: &Rc<dyn IXamlAstValueNode>) -> Vec<f64> {
    node.cast::<FerroXamlIlVectorLikeConstantAstNode>()
        .unwrap_or_else(|| panic!("{} is not a vector-like constant", node.type_name()))
        .values()
        .to_vec()
}

/// The name of the called method and its arguments.
fn method_call(node: &Rc<dyn IXamlAstValueNode>) -> (String, String, Vec<Rc<dyn IXamlAstValueNode>>) {
    let call = node
        .cast::<XamlStaticOrTargetedReturnMethodCallNode>()
        .unwrap_or_else(|| panic!("{} is not a method call", node.type_name()));
    let method = call.base.method();
    let arguments = call.base.arguments.borrow().clone();
    (method.declaring_type().full_name(), method.name(), arguments)
}

fn new_object(node: &Rc<dyn IXamlAstValueNode>) -> Rc<XamlAstNewClrObjectNode> {
    node.cast::<XamlAstNewClrObjectNode>()
        .unwrap_or_else(|| panic!("{} is not an object creation", node.type_name()))
}

#[test]
fn time_span() {
    let fw = create_test_framework();
    for (text, ticks) in [
        ("0:0:1.5", 15_000_000i64),
        (" 00:01:00 ", 600_000_000),
        // A lone integer is a number of days.
        ("5", 5 * 864_000_000_000),
        // Shorthand seconds.
        ("0.25", 2_500_000),
        ("1,000.5", 10_005_000_000),
    ] {
        let node = convert(&fw, text, "System.TimeSpan");
        assert_eq!(clr_type_name(&node), "System.TimeSpan");
        let (declaring_type, method, arguments) = method_call(&node);
        assert_eq!((declaring_type.as_str(), method.as_str()), ("System.TimeSpan", "FromTicks"));
        assert_eq!(arguments.len(), 1);
        assert_eq!(constant(&arguments[0]), XamlValue::Int64(ticks), "{text}");
        assert_eq!(clr_type_name(&arguments[0]), "System.Int64");
    }

    let time_span = fw.t("System.TimeSpan");
    assert_parse_error(&fw, "abc", &time_span, "Unable to parse abc as a time span");
    assert_parse_error(&fw, "1:x", &time_span, "Unable to parse 1:x as a time span");
    // The seconds shorthand rejects what `TimeSpan.FromSeconds` rejects.
    assert!(try_convert(&fw, "NaN", &time_span).is_err());
}

#[test]
fn font_family() {
    let fw = create_test_framework();
    let node = convert(&fw, "Arial, Helvetica", "FerroUI.Media.FontFamily");
    let font_family = node.cast::<FerroXamlIlFontFamilyAstNode>().expect("font family node");
    assert_eq!(font_family.text(), "Arial, Helvetica");
    assert_eq!(clr_type_name(&node), "FerroUI.Media.FontFamily");
    assert!(Rc::ptr_eq(font_family.types(), &fw.types));
}

#[test]
fn thickness() {
    let fw = create_test_framework();
    let node = convert(&fw, "1,2,3,4", "FerroUI.Thickness");
    assert_eq!(vector_like(&node), [1.0, 2.0, 3.0, 4.0]);
    assert_eq!(clr_type_name(&node), "FerroUI.Thickness");
    let constructor = node
        .cast::<FerroXamlIlVectorLikeConstantAstNode>()
        .expect("node")
        .constructor()
        .clone();
    assert!(constructor.equals(&*fw.types.thickness_full_constructor));

    assert_eq!(vector_like(&convert(&fw, "5", "FerroUI.Thickness")), [5.0; 4]);
    assert_eq!(
        vector_like(&convert(&fw, "1 2", "FerroUI.Thickness")),
        [1.0, 2.0, 1.0, 2.0]
    );
    assert_parse_error(
        &fw,
        "1,2,3",
        &fw.t("FerroUI.Thickness"),
        "Unable to parse \"1,2,3\" as a thickness",
    );
}

#[test]
fn point_vector_size() {
    let fw = create_test_framework();
    for (type_name, kind) in [
        ("FerroUI.Point", "point"),
        ("FerroUI.Vector", "vector"),
        ("FerroUI.Size", "size"),
    ] {
        let node = convert(&fw, "1.5, 2", type_name);
        assert_eq!(vector_like(&node), [1.5, 2.0]);
        assert_eq!(clr_type_name(&node), type_name);
        assert_parse_error(
            &fw,
            "1",
            &fw.t(type_name),
            &format!("Unable to parse \"1\" as a {kind}"),
        );
    }
}

#[test]
fn matrix() {
    let fw = create_test_framework();
    let node = convert(&fw, "1,2,3,4,5,6", "FerroUI.Matrix");
    assert_eq!(vector_like(&node), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    assert_eq!(clr_type_name(&node), "FerroUI.Matrix");
    assert_parse_error(
        &fw,
        "1,2,3",
        &fw.t("FerroUI.Matrix"),
        "Unable to parse \"1,2,3\" as a matrix",
    );
}

#[test]
fn corner_radius() {
    let fw = create_test_framework();
    // Top left, top right, bottom right, bottom left.
    assert_eq!(
        vector_like(&convert(&fw, "1,2,3,4", "FerroUI.CornerRadius")),
        [1.0, 2.0, 3.0, 4.0]
    );
    assert_eq!(
        vector_like(&convert(&fw, "1,2", "FerroUI.CornerRadius")),
        [1.0, 1.0, 2.0, 2.0]
    );
    assert_parse_error(
        &fw,
        "x",
        &fw.t("FerroUI.CornerRadius"),
        "Unable to parse \"x\" as a corner radius",
    );
}

#[test]
fn vector_like_node_validates_the_constructor() {
    let fw = create_test_framework();
    let li = XamlLineInfo::new(1, 1);
    let wrong_count = FerroXamlIlVectorLikeConstantAstNode::new(
        &li,
        &fw.types,
        fw.types.point.clone(),
        fw.types.point_full_constructor.clone(),
        vec![1.0],
    );
    assert_eq!(
        wrong_count.err().map(|e| e.message()),
        Some("Constructor that takes 1 parameters is expected, got 2 instead.".to_string())
    );
    let wrong_type = FerroXamlIlVectorLikeConstantAstNode::new(
        &li,
        &fw.types,
        fw.types.relative_point.clone(),
        fw.types.relative_point_full_constructor.clone(),
        vec![1.0, 2.0, 3.0],
    );
    assert_eq!(
        wrong_type.err().map(|e| e.message()),
        Some("Expected parameter of type System.Double, got FerroUI.RelativeUnit instead.".to_string())
    );
}

#[test]
fn color() {
    let fw = create_test_framework();
    for (text, value) in [(" Red ", 0xffff0000u32), ("#80112233", 0x80112233), ("#123", 0xff112233)] {
        let node = convert(&fw, text, "FerroUI.Media.Color");
        assert_eq!(clr_type_name(&node), "FerroUI.Media.Color");
        let (declaring_type, method, arguments) = method_call(&node);
        assert_eq!((declaring_type.as_str(), method.as_str()), ("FerroUI.Media.Color", "FromUInt32"));
        assert_eq!(constant(&arguments[0]), XamlValue::UInt32(value));
        assert_eq!(clr_type_name(&arguments[0]), "System.UInt32");
    }
    // The message carries the trimmed text.
    assert_parse_error(
        &fw,
        " nope ",
        &fw.t("FerroUI.Media.Color"),
        "Unable to parse \"nope\" as a color",
    );
}

#[test]
fn relative_point() {
    let fw = create_test_framework();
    for (text, x, y, unit) in [("50%,25%", 0.5, 0.25, 0), ("10,20", 10.0, 20.0, 1)] {
        let node = convert(&fw, text, "FerroUI.RelativePoint");
        assert_eq!(clr_type_name(&node), "FerroUI.RelativePoint");
        let object = new_object(&node);
        assert!(object
            .constructor
            .equals(&*fw.types.relative_point_full_constructor));
        let arguments = object.arguments.borrow().clone();
        assert_eq!(arguments.len(), 3);
        assert_eq!(constant(&arguments[0]), XamlValue::Double(x));
        assert_eq!(constant(&arguments[1]), XamlValue::Double(y));
        assert_eq!(constant(&arguments[2]), XamlValue::Int32(unit));
        assert_eq!(clr_type_name(&arguments[2]), "FerroUI.RelativeUnit");
    }
    assert_parse_error(
        &fw,
        "50%,20",
        &fw.t("FerroUI.RelativePoint"),
        "Unable to parse \"50%,20\" as a relative point",
    );
}

#[test]
fn grid_length() {
    let fw = create_test_framework();
    for (text, value, unit) in [
        ("2*", 2.0, GridUnitType::Star),
        ("*", 1.0, GridUnitType::Star),
        ("auto", 0.0, GridUnitType::Auto),
        ("120", 120.0, GridUnitType::Pixel),
    ] {
        let node = convert(&fw, text, "FerroUI.Controls.GridLength");
        assert_eq!(clr_type_name(&node), "FerroUI.Controls.GridLength");
        let grid_length = node
            .cast::<FerroXamlIlGridLengthAstNode>()
            .expect("grid length node")
            .grid_length();
        assert_eq!(grid_length.value, value, "{text}");
        assert_eq!(grid_length.grid_unit_type, unit, "{text}");
    }
    for invalid in ["x", "-1", "-2*", "NaN", "Infinity", ""] {
        assert_parse_error(
            &fw,
            invalid,
            &fw.t("FerroUI.Controls.GridLength"),
            &format!("Unable to parse \"{invalid}\" as a grid length"),
        );
    }
}

struct DoublingGridLengthParser;

impl IXamlCompileTimeValueParser for DoublingGridLengthParser {
    fn try_parse(
        &self,
        type_: &Rc<dyn IXamlType>,
        text: &str,
    ) -> Option<Result<XamlCompileTimeValue, String>> {
        if type_.name() != "GridLength" {
            return None;
        }
        Some(match text.parse::<f64>() {
            Ok(value) => Ok(XamlCompileTimeValue::GridLength(FerroXamlIlGridLength {
                value: value * 2.0,
                grid_unit_type: GridUnitType::Pixel,
            })),
            Err(_) => Err(format!("{text} is no number")),
        })
    }
}

#[test]
fn registered_compile_time_parser_takes_precedence_for_grid_lengths() {
    let fw = create_test_framework();
    fw.configuration
        .get_or_create_extra::<XamlCompileTimeValueParsers>()
        .add(Rc::new(DoublingGridLengthParser));

    let node = convert(&fw, "21", "FerroUI.Controls.GridLength");
    let grid_length = node
        .cast::<FerroXamlIlGridLengthAstNode>()
        .expect("grid length node")
        .grid_length();
    assert_eq!(grid_length.value, 42.0);
    // Also inside a definition.
    let node = convert(&fw, "4", "FerroUI.Controls.RowDefinition");
    let arguments = new_object(&node).arguments.borrow().clone();
    let grid_length = arguments[0]
        .cast::<FerroXamlIlGridLengthAstNode>()
        .expect("grid length argument")
        .grid_length();
    assert_eq!(grid_length.value, 8.0);
    // Its verdict is final: "*" is valid for the built-in grammar only.
    assert_parse_error(
        &fw,
        "*",
        &fw.t("FerroUI.Controls.GridLength"),
        "Unable to parse \"*\" as a grid length",
    );
}

#[test]
fn column_and_row_definition() {
    let fw = create_test_framework();
    for type_name in ["FerroUI.Controls.ColumnDefinition", "FerroUI.Controls.RowDefinition"] {
        let node = convert(&fw, "3*", type_name);
        assert_eq!(clr_type_name(&node), type_name);
        let object = new_object(&node);
        let parameters = object.constructor.parameters();
        assert_eq!(parameters.len(), 1);
        assert!(parameters[0].equals(&*fw.types.grid_length));
        let arguments = object.arguments.borrow().clone();
        let grid_length = arguments[0]
            .cast::<FerroXamlIlGridLengthAstNode>()
            .expect("grid length argument")
            .grid_length();
        assert_eq!(grid_length.value, 3.0);
        assert_eq!(grid_length.grid_unit_type, GridUnitType::Star);
        assert_parse_error(&fw, "x", &fw.t(type_name), "Unable to parse \"x\" as a grid length");
    }
}

#[test]
fn cursor() {
    let fw = create_test_framework();
    let node = convert(&fw, "Hand", "FerroUI.Input.Cursor");
    assert_eq!(clr_type_name(&node), "FerroUI.Input.Cursor");
    let object = new_object(&node);
    assert!(object.constructor.equals(&*fw.types.cursor_type_constructor));
    let arguments = object.arguments.borrow().clone();
    assert_eq!(constant(&arguments[0]), XamlValue::Int32(9));
    assert_eq!(clr_type_name(&arguments[0]), "FerroUI.Input.StandardCursorType");

    // An unknown (or differently cased) cursor name is left to other conversions.
    assert_not_converted(&fw, "hand", &fw.t("FerroUI.Input.Cursor"));
    assert_not_converted(&fw, "Nope", &fw.t("FerroUI.Input.Cursor"));
}

#[test]
fn brush() {
    let fw = create_test_framework();
    let node = convert(&fw, "Blue", "FerroUI.Media.IBrush");
    assert_eq!(clr_type_name(&node), "FerroUI.Media.Immutable.ImmutableSolidColorBrush");
    let object = new_object(&node);
    assert!(object
        .constructor
        .equals(&*fw.types.immutable_solid_color_brush_constructor_color));
    let arguments = object.arguments.borrow().clone();
    assert_eq!(constant(&arguments[0]), XamlValue::UInt32(0xff0000ff));

    // Not a colour (the brush text is not trimmed): no error, other conversions may apply.
    assert_not_converted(&fw, "nope", &fw.t("FerroUI.Media.IBrush"));
    assert_not_converted(&fw, " Blue", &fw.t("FerroUI.Media.IBrush"));
    // Only types assignable to IBrush qualify.
    assert_not_converted(&fw, "Blue", &fw.t("FerroUI.Controls.Control"));
}

#[test]
fn text_trimming() {
    let fw = create_test_framework();
    let node = convert(&fw, "characterELLIPSIS", "FerroUI.Media.TextTrimming");
    assert_eq!(clr_type_name(&node), "FerroUI.Media.TextTrimming");
    let (declaring_type, method, arguments) = method_call(&node);
    assert_eq!(
        (declaring_type.as_str(), method.as_str()),
        ("FerroUI.Media.TextTrimming", "get_CharacterEllipsis")
    );
    assert!(arguments.is_empty());
    assert_not_converted(&fw, "Nope", &fw.t("FerroUI.Media.TextTrimming"));
}

#[test]
fn text_decorations() {
    let fw = create_test_framework();
    let node = convert(&fw, "underline", "FerroUI.Media.TextDecorationCollection");
    assert_eq!(clr_type_name(&node), "FerroUI.Media.TextDecorationCollection");
    let (declaring_type, method, arguments) = method_call(&node);
    assert_eq!(
        (declaring_type.as_str(), method.as_str()),
        ("FerroUI.Media.TextDecorations", "get_Underline")
    );
    assert!(arguments.is_empty());
}

#[test]
fn window_transparency_level() {
    let fw = create_test_framework();
    let node = convert(&fw, "AcrylicBlur", "FerroUI.Controls.WindowTransparencyLevel");
    let (declaring_type, method, _) = method_call(&node);
    assert_eq!(
        (declaring_type.as_str(), method.as_str()),
        ("FerroUI.Controls.WindowTransparencyLevel", "get_AcrylicBlur")
    );
    assert_not_converted(&fw, "Nope", &fw.t("FerroUI.Controls.WindowTransparencyLevel"));
}

#[test]
fn uri() {
    let fw = create_test_framework();
    for (text, expected_text, kind) in [
        (" https://example.org/a ", "https://example.org/a", 0),
        ("relative/path.png", "relative/path.png", 0),
        ("/Assets/icon.png", "/Assets/icon.png", 2),
    ] {
        let node = convert(&fw, text, "System.Uri");
        assert_eq!(clr_type_name(&node), "System.Uri");
        let object = new_object(&node);
        assert!(object.constructor.equals(&*fw.types.uri_constructor));
        let arguments = object.arguments.borrow().clone();
        assert_eq!(constant(&arguments[0]), XamlValue::String(expected_text.to_string()));
        assert_eq!(constant(&arguments[1]), XamlValue::Int32(kind));
        assert_eq!(clr_type_name(&arguments[1]), "System.UriKind");
    }
    assert_parse_error(
        &fw,
        "   ",
        &fw.t("System.Uri"),
        "Unable to parse text \"\" as a RelativeOrAbsolute uri",
    );
    assert_parse_error(
        &fw,
        "http:",
        &fw.t("System.Uri"),
        "Unable to parse text \"http:\" as a RelativeOrAbsolute uri",
    );
}

#[test]
fn theme_variant() {
    let fw = create_test_framework();
    let node = convert(&fw, " Dark ", "FerroUI.Styling.ThemeVariant");
    let static_node = node.cast::<XamlStaticExtensionNode>().expect("x:Static node");
    assert_eq!(*static_node.member.borrow(), "Dark");
    assert_eq!(clr_type_name(&node), "FerroUI.Styling.ThemeVariant");
    // The lookup is case sensitive and only static properties of the variant type count.
    assert_not_converted(&fw, "dark", &fw.t("FerroUI.Styling.ThemeVariant"));
    assert_not_converted(&fw, "Key", &fw.t("FerroUI.Styling.ThemeVariant"));
}

#[test]
fn points_list() {
    let fw = create_test_framework();
    let points = fw
        .t("System.Collections.Generic.IList`1")
        .make_generic_type(&[fw.t("FerroUI.Point")])
        .expect("IList<Point>");
    let node = try_convert(&fw, "1,2 3,4  5 6", &points)
        .expect("no error")
        .expect("converted");
    let array = node.cast::<FerroXamlIlArrayConstantAstNode>().expect("array node");
    assert!(node.type_().get_clr_type().expect("clr type").equals(&*points));
    assert!(array.element_type().equals(&*fw.types.point));
    let values: Vec<Vec<f64>> = array.values().iter().map(vector_like).collect();
    assert_eq!(values, [[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]]);

    assert_parse_error(
        &fw,
        "1,2,3",
        &points,
        "Unable to parse text \"1,2,3\" as a Points list",
    );
}

#[test]
fn read_only_list_becomes_a_list_typed_array() {
    let fw = create_test_framework();
    let level = fw.t("FerroUI.Controls.WindowTransparencyLevel");
    let read_only_list = fw
        .t("System.Collections.Generic.IReadOnlyList`1")
        .make_generic_type(std::slice::from_ref(&level))
        .expect("IReadOnlyList<WindowTransparencyLevel>");
    let node = try_convert(&fw, "Mica, AcrylicBlur", &read_only_list)
        .expect("no error")
        .expect("converted");
    let array = node.cast::<FerroXamlIlArrayConstantAstNode>().expect("array node");
    assert_eq!(
        clr_type_name(&node),
        "System.Collections.Generic.IList`1[FerroUI.Controls.WindowTransparencyLevel]"
    );
    let getters: Vec<String> = array.values().iter().map(|v| method_call(v).1).collect();
    assert_eq!(getters, ["get_Mica", "get_AcrylicBlur"]);
}

#[test]
fn array() {
    let fw = create_test_framework();
    let double = fw.t("System.Double");
    let array_type = double.make_array_type(1).expect("double[]");
    let node = try_convert(&fw, "1, 2.5,,3", &array_type)
        .expect("no error")
        .expect("converted");
    let array = node.cast::<FerroXamlIlArrayConstantAstNode>().expect("array node");
    assert!(node.type_().get_clr_type().expect("clr type").equals(&*array_type));
    assert!(array.element_type().equals(&*double));
    let values: Vec<XamlValue> = array.values().iter().map(constant).collect();
    assert_eq!(
        values,
        [XamlValue::Double(1.0), XamlValue::Double(2.5), XamlValue::Double(3.0)]
    );

    // An element that cannot be parsed fails the way the element conversion fails.
    assert!(try_convert(&fw, "1,x", &array_type).is_err());
}

#[test]
fn ferro_list_with_list_attribute() {
    let fw = create_test_framework();
    // DefinitionList<T> declares the separators "," and " ".
    let node = convert(&fw, "100,* Auto", "FerroUI.Controls.ColumnDefinitions");
    assert_eq!(clr_type_name(&node), "FerroUI.Controls.ColumnDefinitions");
    let list = node
        .cast::<FerroXamlIlFerroListConstantAstNode>()
        .expect("list node");
    assert!(list.element_type().equals(&*fw.types.column_definition));
    assert_eq!(list.constructor().parameters().len(), 0);
    assert_eq!(list.list_add_method().name(), "Add");
    assert!(list.list_add_method().parameters()[0].equals(&*fw.types.column_definition));
    assert_eq!(list.list_set_capacity_method().name(), "set_Capacity");
    let lengths: Vec<(f64, GridUnitType)> = list
        .values()
        .iter()
        .map(|value| {
            let arguments = new_object(value).arguments.borrow().clone();
            let length = arguments[0]
                .cast::<FerroXamlIlGridLengthAstNode>()
                .expect("grid length")
                .grid_length();
            (length.value, length.grid_unit_type)
        })
        .collect();
    assert_eq!(
        lengths,
        [
            (100.0, GridUnitType::Pixel),
            (1.0, GridUnitType::Star),
            (0.0, GridUnitType::Auto)
        ]
    );
}

fn list_texts(node: &Rc<dyn IXamlAstValueNode>) -> Vec<String> {
    node.cast::<FerroXamlIlFerroListConstantAstNode>()
        .expect("list node")
        .values()
        .iter()
        .map(|v| v.cast::<XamlAstTextNode>().expect("text").text())
        .collect()
}

#[test]
fn ferro_list_default_split() {
    let fw = create_test_framework();
    // Default: "," separated, empty entries removed before the entries are trimmed.
    let node = convert(&fw, " a ,b,, ,c ", "FerroUI.Controls.Classes");
    assert_eq!(list_texts(&node), ["a", "b", "", "c"]);
    assert_eq!(clr_type_name(&node), "FerroUI.Controls.Classes");
}

#[test]
fn ferro_list_split_options_and_separators() {
    let fw = create_test_framework();
    let string = fw.t("System.String");
    let list_attribute = fw.t("FerroUI.Metadata.FerroListAttribute");
    let ferro_list = fw
        .t("FerroUI.Collections.FerroList`1")
        .make_generic_type(std::slice::from_ref(&string))
        .expect("FerroList<string>");

    // RemoveEmptyEntries only: the XOR turns trimming on in the split itself.
    let remove_empty = fw.controls.define_class("Tests", "RemoveEmptyList");
    remove_empty.set_base_type(ferro_list.clone());
    remove_empty.add_constructor(vec![]);
    remove_empty.add_attribute(FakeCustomAttribute::with_properties(
        list_attribute.clone(),
        vec![],
        vec![
            ("Separators", XamlValue::Array(vec![XamlValue::String(";".to_string())])),
            ("SplitOptions", XamlValue::Int32(1)),
        ],
    ));
    assert_eq!(
        list_texts(&convert(&fw, " a ; ;b;", "Tests.RemoveEmptyList")),
        ["a", "b"]
    );

    // No options: empty entries are kept, nothing is trimmed by the option...
    let keep_all = fw.controls.define_class("Tests", "KeepAllList");
    keep_all.set_base_type(ferro_list.clone());
    keep_all.add_constructor(vec![]);
    keep_all.add_attribute(FakeCustomAttribute::with_properties(
        list_attribute.clone(),
        vec![],
        vec![("SplitOptions", XamlValue::Int32(0))],
    ));
    // ...but the XOR again passes TrimEntries to the split.
    assert_eq!(list_texts(&convert(&fw, " a ,,b", "Tests.KeepAllList")), ["a", "", "b"]);

    // Null separators split at white space.
    let white_space = fw.controls.define_class("Tests", "WhiteSpaceList");
    white_space.set_base_type(ferro_list);
    white_space.add_constructor(vec![]);
    white_space.add_attribute(FakeCustomAttribute::with_properties(
        list_attribute,
        vec![],
        vec![("Separators", XamlValue::Null)],
    ));
    assert_eq!(
        list_texts(&convert(&fw, "a b,c\td", "Tests.WhiteSpaceList")),
        ["a", "b,c", "d"]
    );
}

#[test]
fn list_elements_that_cannot_be_converted_decline_the_conversion() {
    let fw = create_test_framework();
    let controls = fw
        .t("System.Collections.Generic.IList`1")
        .make_generic_type(&[fw.t("FerroUI.Controls.Control")])
        .expect("IList<Control>");
    assert_not_converted(&fw, "a,b", &controls);
}

#[test]
fn enumerables_that_are_no_list_are_not_converted() {
    let fw = create_test_framework();
    // IEnumerable<string> elements convert, but the target is none of the supported shapes.
    let dictionary_like = fw.controls.define_class("Tests", "StringBag");
    dictionary_like.add_interface(
        fw.t("System.Collections.Generic.IEnumerable`1")
            .make_generic_type(&[fw.t("System.String")])
            .expect("IEnumerable<string>"),
    );
    assert_not_converted(&fw, "a,b", &fw.t("Tests.StringBag"));
}

#[test]
fn unrelated_types_are_not_converted() {
    let fw = create_test_framework();
    assert_not_converted(&fw, "anything", &fw.t("FerroUI.Controls.Control"));
    assert_not_converted(&fw, "1", &fw.t("System.Int32"));
}

#[test]
fn split_follows_the_string_split_rules() {
    let separators = [",".to_string(), " ".to_string()];
    assert_eq!(split("a, b", Some(&separators), 0), ["a", "", "b"]);
    assert_eq!(split("a, b", Some(&separators), 1), ["a", "b"]);
    assert_eq!(split("", Some(&separators), 0), [""]);
    assert!(split("", Some(&separators), 1).is_empty());
    assert_eq!(split(" a ;b", Some(&[";".to_string()]), 2), ["a", "b"]);
    assert_eq!(split(" ; b", Some(&[";".to_string()]), 3), ["b"]);
    // Empty separators are ignored; without a usable separator white space splits.
    assert_eq!(split("a b", Some(&[String::new()]), 0), ["a", "b"]);
    assert_eq!(split("a\tb  c", None, 1), ["a", "b", "c"]);
    // The first matching separator in array order wins.
    assert_eq!(
        split("a--b-c", Some(&["-".to_string(), "--".to_string()]), 0),
        ["a", "", "b", "c"]
    );
}
