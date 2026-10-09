//! The tests of the scanner: a fixture crate read from `tests/fixtures/scanner`
//! (source files that are never compiled), and the base and the controls
//! crates of the workspace read as files.
//!
//! None of the tests is from upstream: the scanner has no counterpart there
//! (upstream's compiler reads the types of an assembly from its metadata).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::*;
use crate::model::{CallForm, XamlMetadata};

const BORDER: &str = "::fixture::controls::border::Border";
const DECORATOR: &str = "::fixture::controls::decorator::Decorator";
const GRID: &str = "::fixture::controls::grid::Grid";
const LAYOUT: &str = "::fixture::controls::grid::Layout";
const BRUSH: &str = "::fixture::media::brush::Brush";
const IBRUSH: &str = "::fixture::media::brush::IBrush";
const SETTER: &str = "::fixture::markup_types::plain::Setter";
const THICKNESS: &str = "::fixture::media::Thickness";
const CONTROL: &str = "::ferroui_base::Control";

fn fixture_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).with_file_name("FerroUI.Build.Scan").join("tests").join("fixtures").join("scanner")
}

fn fixture() -> Scan {
    scan_crate(&ScanOptions::new("fixture", fixture_directory().join("lib.rs")))
}

fn ty(text: &str) -> RustType {
    RustType::resolved(text)
}

fn unresolved(text: &str, paths: &[&str]) -> RustType {
    RustType { text: text.to_string(), unresolved: paths.iter().map(|path| path.to_string()).collect() }
}

/// A callable that is a path: as written, and resolved.
fn path(written: &str, resolved: &str) -> CallableModel {
    CallableModel { path: Some(written.to_string()), resolved: Some(resolved.to_string()), dereferenced: None }
}

/// A callable that is not a path.
fn closure() -> CallableModel {
    CallableModel::default()
}

fn attribute(name: &str, arguments: Vec<AttributeValueModel>) -> AttributeModel {
    AttributeModel { name: name.to_string(), arguments, properties: Vec::new() }
}

fn text(value: &str) -> AttributeValueModel {
    AttributeValueModel::Str(value.to_string())
}

fn positional(type_: RustType) -> ParameterModel {
    ParameterModel { name: None, type_, attributes: Vec::new() }
}

/// An accessor whose callable is not called by a path (form C).
fn accessor(callable: CallableModel, typed_function: Option<&str>) -> AccessorModel {
    AccessorModel { fallible: false, callable, typed_function: typed_function.map(str::to_string), call: Some(CallForm::Invoker) }
}

/// The public function `name` of the border of the fixture, as form B names it.
fn border_function(name: &str) -> Option<CallForm> {
    Some(CallForm::Path(format!("::fixture::Border::{name}")))
}

fn registered(name: &str, kind: RegisteredKind, value_type: RustType, owner: &str, accessor: &str) -> RegisteredModel {
    RegisteredModel {
        name: Some(name.to_string()),
        kind,
        value_type,
        owner: Some(ty(owner)),
        host: None,
        accessor: accessor.to_string(),
        function_of: None,
        visibility: "pub".to_string(),
        registration: RegistrationModel::Declared,
        source: None,
        assign_binding: false,
        inherits: false,
        read_only: false,
        added_owners: Vec::new(),
    }
}

fn dependent_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).with_file_name("FerroUI.Build.Scan").join("tests").join("fixtures").join("dependent")
}

/// The type of the model with the markup name `name`.
fn the_type<'a>(scan: &'a Scan, name: &str) -> &'a TypeModel {
    let found: Vec<&TypeModel> = scan.model.types.iter().filter(|type_| type_.name == name).collect();
    match found.as_slice() {
        [type_] => *type_,
        _ => panic!(
            "{} types are named `{name}`; the model has: {:?}\n{}",
            found.len(),
            scan.model.types.iter().map(TypeModel::full_name).collect::<Vec<_>>(),
            listing(&scan.diagnostics)
        ),
    }
}

fn listing(diagnostics: &[Diagnostic]) -> String {
    diagnostics.iter().map(|diagnostic| format!("  {diagnostic}")).collect::<Vec<_>>().join("\n")
}

/// The path of `file` below `directory`, with `/` between its parts.
fn relative(file: &Path, directory: &Path) -> String {
    let below = file.strip_prefix(directory).unwrap_or(file);
    below.components().map(|part| part.as_os_str().to_string_lossy().into_owned()).collect::<Vec<_>>().join("/")
}

/// The types of the fixture: every declaration is a type of the model, in the order the
/// files are read, with its kind, its namespace (explicit, or the one of the module), its
/// Rust path and the shortest path another crate names it by.
#[test]
fn fixture_types_are_the_declared_ones() {
    let scan = fixture();
    let types: Vec<(String, TypeKind, &str, Option<&str>)> =
        scan.model.types.iter().map(|type_| (type_.full_name(), type_.kind, type_.rust_path.text.as_str(), type_.public_path.as_deref())).collect();
    let expected: Vec<(String, TypeKind, &str, Option<&str>)> = [
        ("Fixture.Controls.Border", TypeKind::Class, BORDER, Some("::fixture::Border")),
        ("Fixture.Controls.Decorator", TypeKind::Class, DECORATOR, Some("::fixture::Decorator")),
        ("Fixture.Controls.Grid", TypeKind::Class, GRID, Some("::fixture::controls::Grid")),
        ("Fixture.Controls.Layout", TypeKind::Static, LAYOUT, Some("::fixture::controls::grid::Layout")),
        ("Fixture.Controls.TextBlock", TypeKind::Class, "::fixture::controls::text_block::TextBlock", Some("::fixture::controls::text_block::TextBlock")),
        (
            "Fixture.Controls.Documents.Run",
            TypeKind::Class,
            "::fixture::controls::text_block::documents::Run",
            Some("::fixture::controls::text_block::documents::Run"),
        ),
        ("Fixture.Controls.Documents.Span", TypeKind::Class, "::fixture::controls::text_block::documents::gated::Span", None),
        ("Fixture.Media.Brush", TypeKind::Class, BRUSH, Some("::fixture::Brush")),
        ("Fixture.Deep", TypeKind::Static, "::fixture::placed::inner::Deep", None),
        ("Fixture.Placed", TypeKind::Class, "::fixture::placed::Placed", Some("::fixture::Placed")),
        ("Fixture.DoubleTransition", TypeKind::Class, "::fixture::macros::DoubleTransition", None),
        ("Fixture.Root", TypeKind::Class, "::fixture::placed::inner::Root", None),
        ("Fixture.Styling.Setter", TypeKind::Class, SETTER, None),
        ("Fixture.Media.Colors", TypeKind::Static, "Colors", None),
        ("Fixture.Media.Dock", TypeKind::Enum, "::fixture::media::Dock", Some("::fixture::Dock")),
        ("Fixture.Data.Mode", TypeKind::Enum, "::fixture::media::Mode", Some("::fixture::Mode")),
        ("Fixture.Media.Routes", TypeKind::Enum, "::fixture::media::Routes", Some("::fixture::Routes")),
        ("Fixture.Media.Thickness", TypeKind::Struct, THICKNESS, Some("::fixture::Thickness")),
        ("Fixture.Media.IBrush", TypeKind::Interface, "dyn ::fixture::media::brush::IBrush", Some("::fixture::IBrush")),
    ]
    .into_iter()
    .map(|(name, kind, rust_path, public_path)| (name.to_string(), kind, rust_path, public_path))
    .collect();
    assert_eq!(types, expected, "\n{}", listing(&scan.diagnostics));
    assert_eq!(scan.model.find_type("Fixture.Controls.Border").map(|type_| type_.name.as_str()), Some("Border"));
    assert_eq!(scan.model.find_rust_type(GRID).map(|type_| type_.name.as_str()), Some("Grid"));
}

/// A class with styled, direct and added properties, an alias and a property declared
/// with `ferro_property!` in its `impl` block; its class info with every kind of member,
/// written in two files (`new` and `interfaces` next to the class, `markup` apart).
#[test]
fn class_with_every_kind_of_member_is_read_exactly() {
    let scan = fixture();
    let brush_ref = format!("::ferroui_base::Ref<{BRUSH}>");
    let control_ref = format!("Option<::ferroui_base::Ref<{CONTROL}>>");
    let function = |name: &str| path(&format!("Border::{name}"), &format!("{BORDER}::{name}"));

    let mut expected = TypeModel::new("Border", TypeKind::Class, ty(BORDER), "fixture::controls::border");
    expected.object_model = true;
    expected.class_markup = true;
    expected.namespace = "Fixture.Controls".to_string();
    expected.explicit_namespace = Some("Fixture.Controls".to_string());
    expected.public_path = Some("::fixture::Border".to_string());
    expected.base = Some(ty(DECORATOR));
    expected.interfaces = vec![ty(&format!("::std::rc::Rc<dyn {IBRUSH}>")), ty("::std::rc::Rc<dyn ::ferroui_base::IOther>")];
    expected.default_constructor = Some(function("new"));
    expected.content_property = Some("Child".to_string());
    expected.constructors = vec![
        MemberModel {
            parameters: vec![positional(ty("String"))],
            is_static: true,
            callable: function("with_name"),
            typed_function: Some("__markup_new_0".to_string()),
            call: border_function("with_name"),
            ..MemberModel::default()
        },
        MemberModel {
            parameters: vec![
                ParameterModel {
                    name: Some("property".to_string()),
                    type_: unresolved("&'static FerroProperty", &["FerroProperty"]),
                    attributes: vec![attribute("InheritDataTypeFrom", vec![AttributeValueModel::Int(2)])],
                },
                ParameterModel { name: Some("dock".to_string()), type_: ty("::fixture::media::Dock"), attributes: Vec::new() },
            ],
            is_static: true,
            callable: closure(),
            typed_function: Some("__markup_new_1".to_string()),
            call: Some(CallForm::Invoker),
            ..MemberModel::default()
        },
    ];
    expected.properties = vec![
        PropertyModel {
            name: "Tag".to_string(),
            parameters: Vec::new(),
            value_type: ty("Option<::ferroui_base::BoxedValue>"),
            getter: Some(AccessorModel { call: border_function("tag"), ..accessor(function("tag"), Some("__markup_get_Tag")) }),
            // A function visible in its crate is called by its path from that crate only.
            setter: Some(AccessorModel {
                call: Some(CallForm::CratePath("::fixture::Border::set_tag".to_string())),
                ..accessor(function("set_tag"), Some("__markup_set_Tag"))
            }),
            attributes: vec![attribute("DependsOn", vec![text("Child")]), attribute("AssignBinding", Vec::new())],
        },
        PropertyModel {
            name: "Brushes".to_string(),
            parameters: Vec::new(),
            value_type: ty(&format!("Vec<{brush_ref}>")),
            getter: Some(AccessorModel { fallible: true, ..accessor(closure(), Some("__markup_get_Brushes")) }),
            setter: None,
            attributes: Vec::new(),
        },
    ];
    expected.static_properties = vec![PropertyModel {
        name: "Default".to_string(),
        parameters: Vec::new(),
        value_type: ty(THICKNESS),
        getter: Some(AccessorModel { call: border_function("default_thickness"), ..accessor(function("default_thickness"), Some("__markup_static_get_Default")) }),
        // A private function: the invoker.
        setter: Some(AccessorModel { fallible: true, ..accessor(function("set_default_thickness"), Some("__markup_static_set_Default")) }),
        attributes: Vec::new(),
    }];
    expected.indexers = vec![PropertyModel {
        name: String::new(),
        parameters: vec![positional(ty("i32"))],
        value_type: ty(&brush_ref),
        // The function takes no index: the invoker.
        getter: Some(accessor(function("brush_at"), None)),
        setter: None,
        attributes: vec![attribute("Indexed", Vec::new())],
    }];
    expected.registered = vec![
        registered("Background", RegisteredKind::Styled, ty(&format!("Option<::std::rc::Rc<dyn {IBRUSH}>>")), BORDER, "background_property"),
        RegisteredModel {
            visibility: "pub(crate)".to_string(),
            read_only: true,
            ..registered("Thickness", RegisteredKind::Direct, ty("f64"), BORDER, "thickness_property")
        },
        // The owner of an added owner is the one `add_owner::<Border>` names.
        RegisteredModel {
            registration: RegistrationModel::AddedOwner,
            source: Some(path("Decorator::child_property", &format!("{DECORATOR}::child_property"))),
            ..registered("Child", RegisteredKind::Styled, ty(&control_ref), BORDER, "child_property")
        },
        RegisteredModel {
            owner: None,
            visibility: String::new(),
            registration: RegistrationModel::Alias,
            source: Some(path("Self::child_property", &format!("{BORDER}::child_property"))),
            ..registered("Child", RegisteredKind::Styled, ty(&control_ref), BORDER, "content_property")
        },
        registered("Brush", RegisteredKind::Styled, ty(&format!("Option<{brush_ref}>")), BORDER, "brush_property"),
    ];
    expected.methods = vec![
        MemberModel {
            name: "Add".to_string(),
            parameters: vec![positional(ty(&brush_ref))],
            callable: function("add_brush"),
            typed_function: Some("__markup_Add_0".to_string()),
            call: Some(CallForm::Invoker),
            ..MemberModel::default()
        },
        MemberModel {
            name: "Parse".to_string(),
            parameters: vec![positional(ty("String"))],
            return_type: Some(ty(&format!("::ferroui_base::Ref<{BORDER}>"))),
            is_static: true,
            fallible: true,
            attributes: vec![attribute("Browsable", vec![AttributeValueModel::Bool(false)])],
            callable: function("parse"),
            typed_function: Some("__markup_Parse_1".to_string()),
            call: Some(CallForm::Invoker),
            ..MemberModel::default()
        },
        MemberModel {
            name: "Count".to_string(),
            return_type: Some(ty("i32")),
            attributes: vec![attribute("Obsolete", Vec::new())],
            callable: closure(),
            typed_function: Some("__markup_Count_2".to_string()),
            call: Some(CallForm::Invoker),
            ..MemberModel::default()
        },
    ];
    expected.fields = vec![
        MemberModel {
            name: "PressedEvent".to_string(),
            return_type: Some(ty("::ferroui_base::RoutedEvent<::ferroui_base::RoutedEventArgs>")),
            is_static: true,
            callable: function("pressed_event"),
            typed_function: Some("__markup_field_PressedEvent".to_string()),
            // The accessor `ferro_routed_event!` writes: form A.
            call: Some(CallForm::Structural),
            ..MemberModel::default()
        },
        // A closure that dereferences what such an accessor returns is form A too.
        MemberModel {
            name: "ReleasedEvent".to_string(),
            return_type: Some(ty("::ferroui_base::RoutedEvent<::ferroui_base::RoutedEventArgs>")),
            is_static: true,
            callable: CallableModel { path: None, resolved: None, dereferenced: Some(format!("{BORDER}::pressed_event")) },
            typed_function: Some("__markup_field_ReleasedEvent".to_string()),
            call: Some(CallForm::Structural),
            ..MemberModel::default()
        },
        // The same closure over a function no declaration macro writes: the invoker.
        MemberModel {
            name: "NewEvent".to_string(),
            return_type: Some(ty(&format!("::ferroui_base::Ref<{BORDER}>"))),
            is_static: true,
            callable: CallableModel { path: None, resolved: None, dereferenced: Some(format!("{BORDER}::new")) },
            typed_function: Some("__markup_field_NewEvent".to_string()),
            call: Some(CallForm::Invoker),
            ..MemberModel::default()
        },
    ];
    expected.events = vec![
        MemberModel {
            name: "Closed".to_string(),
            parameters: vec![positional(ty("Option<::ferroui_base::BoxedValue>")), positional(ty("::ferroui_base::EventArgs"))],
            callable: closure(),
            call: Some(CallForm::Invoker),
            ..MemberModel::default()
        },
        MemberModel { name: "Opened".to_string(), fallible: true, callable: function("on_opened"), call: Some(CallForm::Invoker), ..MemberModel::default() },
    ];
    expected.attributes = vec![
        attribute("UsableDuringInitialization", Vec::new()),
        AttributeModel {
            name: "TemplatePart".to_string(),
            arguments: vec![text("PART_Bar"), AttributeValueModel::Type(ty(&format!("::ferroui_base::Ref<{DECORATOR}>")))],
            properties: vec![
                ("IsRequired".to_string(), AttributeValueModel::Bool(true)),
                ("Weight".to_string(), AttributeValueModel::Float("-1.5".to_string())),
                ("Initial".to_string(), AttributeValueModel::Int('x' as i64)),
            ],
        },
        attribute("Separators", vec![AttributeValueModel::Array(vec![text(","), text(" ")]), AttributeValueModel::Null]),
    ];
    expected.property_attributes = vec![
        ("Child".to_string(), vec![attribute("Content", Vec::new()), attribute("DependsOn", vec![text("Tag")])]),
        ("Background".to_string(), Vec::new()),
    ];
    assert_eq!(the_type(&scan, "Border"), &expected, "\n{}", listing(&scan.diagnostics));

    // The class the added owner names has the owner; a type that is not imported in the
    // file of the declaration stays as written.
    let mut expected = TypeModel::new("Decorator", TypeKind::Class, ty(DECORATOR), "fixture::controls::decorator");
    expected.object_model = true;
    expected.namespace = "Fixture.Controls".to_string();
    expected.public_path = Some("::fixture::Decorator".to_string());
    expected.base = Some(ty(CONTROL));
    expected.default_constructor = Some(path("Decorator::new", &format!("{DECORATOR}::new")));
    expected.registered = vec![
        RegisteredModel { added_owners: vec![BORDER.to_string()], ..registered("Child", RegisteredKind::Styled, ty(&control_ref), DECORATOR, "child_property") },
        RegisteredModel {
            assign_binding: true,
            inherits: true,
            ..registered("Padding", RegisteredKind::Styled, unresolved("Thickness", &["Thickness"]), DECORATOR, "padding_property")
        },
    ];
    assert_eq!(the_type(&scan, "Decorator"), &expected, "\n{}", listing(&scan.diagnostics));
}

/// Attached properties, a class with virtual members, a static owner type whose markup
/// metadata is declared apart (`type_info:`), and a file that imports a glob of another
/// crate: the names that may come from the glob are not resolved.
#[test]
fn attached_properties_and_static_types_are_read_exactly() {
    let scan = fixture();
    let control = unresolved("Control", &["Control"]);

    let mut expected = TypeModel::new("Grid", TypeKind::Class, ty(GRID), "fixture::controls::grid");
    expected.object_model = true;
    expected.namespace = "Fixture.Controls".to_string();
    expected.public_path = Some("::fixture::controls::Grid".to_string());
    expected.base = Some(control.clone());
    expected.registered =
        vec![RegisteredModel { kind: RegisteredKind::Attached, host: Some(control), ..registered("Row", RegisteredKind::Attached, ty("i32"), GRID, "row_property") }];
    assert_eq!(the_type(&scan, "Grid"), &expected, "\n{}", listing(&scan.diagnostics));

    let mut expected = TypeModel::new("Layout", TypeKind::Static, ty(LAYOUT), "fixture::controls::grid");
    expected.object_model = true;
    expected.namespace = "Fixture.Controls".to_string();
    expected.public_path = Some("::fixture::controls::grid::Layout".to_string());
    expected.type_info = Some(ty(LAYOUT));
    expected.registered = vec![
        RegisteredModel { host: Some(ty(GRID)), assign_binding: true, ..registered("Spacing", RegisteredKind::Attached, ty("f64"), LAYOUT, "spacing_property") },
        RegisteredModel {
            name: None,
            owner: None,
            registration: RegistrationModel::Unknown,
            ..registered("", RegisteredKind::Attached, ty("f64"), LAYOUT, "odd_property")
        },
    ];
    expected.methods = vec![MemberModel {
        name: "Reset".to_string(),
        is_static: true,
        callable: path("Layout::reset", &format!("{LAYOUT}::reset")),
        typed_function: Some("__markup_Reset_0".to_string()),
        call: Some(CallForm::Invoker),
        ..MemberModel::default()
    }];
    expected.property_attributes = vec![("Spacing".to_string(), vec![attribute("ResolveByName", Vec::new())])];
    assert_eq!(the_type(&scan, "Layout"), &expected, "\n{}", listing(&scan.diagnostics));

    // A class of a nested module under a `cfg` condition, and one a macro of the crate declares.
    let mut expected = TypeModel::new("Span", TypeKind::Class, ty("::fixture::controls::text_block::documents::gated::Span"), "fixture::controls::text_block::documents::gated");
    expected.object_model = true;
    expected.namespace = "Fixture.Controls.Documents".to_string();
    expected.cfg = vec!["feature = \"inlines\"".to_string()];
    expected.base = Some(ty("::fixture::controls::text_block::documents::Run"));
    assert_eq!(the_type(&scan, "Span"), &expected, "\n{}", listing(&scan.diagnostics));

    let mut expected = TypeModel::new("DoubleTransition", TypeKind::Class, ty("::fixture::macros::DoubleTransition"), "fixture::macros");
    expected.object_model = true;
    expected.namespace = "Fixture".to_string();
    expected.base = Some(ty("::ferroui_base::animation::TransitionBase"));
    expected.default_constructor = Some(path("DoubleTransition::new", "::fixture::macros::DoubleTransition::new"));
    assert_eq!(the_type(&scan, "DoubleTransition"), &expected, "\n{}", listing(&scan.diagnostics));
    assert_eq!(scan.statistics.expanded, BTreeMap::from([("transition_class".to_string(), 2)]));

    let text = the_type(&scan, "TextBlock");
    assert_eq!(text.registered, vec![registered("Text", RegisteredKind::Styled, ty("Option<String>"), "::fixture::controls::text_block::TextBlock", "text_property")]);
}

/// The markup types that are not classes of the object model: a struct, a plain class, an
/// interface, a static type of a name the file does not declare, and the enumerations,
/// with the values of their members where the crate declares them.
#[test]
fn markup_types_and_enumerations_are_read_exactly() {
    let scan = fixture();
    let rc = |inner: &str| format!("::std::rc::Rc<{inner}>");
    let border_ref = format!("::ferroui_base::Ref<{BORDER}>");

    let mut expected = TypeModel::new("Thickness", TypeKind::Struct, ty(THICKNESS), "fixture::media");
    expected.namespace = "Fixture.Media".to_string();
    expected.public_path = Some("::fixture::Thickness".to_string());
    expected.handles = vec![ty(THICKNESS)];
    expected.parse = Some(path("Thickness::parse", &format!("{THICKNESS}::parse")));
    expected.parse_call = Some(CallForm::Invoker);
    expected.constructors = vec![
        MemberModel {
            parameters: vec![positional(ty("f64"))],
            is_static: true,
            callable: path("Thickness::uniform", &format!("{THICKNESS}::uniform")),
            typed_function: Some("__markup_new_0".to_string()),
            call: Some(CallForm::Invoker),
            ..MemberModel::default()
        },
        MemberModel {
            parameters: vec![positional(ty("f64")), positional(ty("f64"))],
            is_static: true,
            fallible: true,
            callable: closure(),
            typed_function: Some("__markup_new_1".to_string()),
            call: Some(CallForm::Invoker),
            ..MemberModel::default()
        },
    ];
    expected.properties = vec![PropertyModel {
        name: "Left".to_string(),
        parameters: Vec::new(),
        value_type: ty("f64"),
        getter: Some(accessor(closure(), Some("__markup_get_Left"))),
        setter: None,
        attributes: Vec::new(),
    }];
    assert_eq!(the_type(&scan, "Thickness"), &expected, "\n{}", listing(&scan.diagnostics));

    let mut expected = TypeModel::new("Setter", TypeKind::Class, ty(SETTER), "fixture::markup_types::plain");
    expected.namespace = "Fixture.Styling".to_string();
    expected.explicit_namespace = Some("Fixture.Styling".to_string());
    expected.handles = vec![ty(SETTER), ty(&rc(SETTER)), ty(&format!("Option<{}>", rc(SETTER)))];
    expected.this = Some(ty(&rc(SETTER)));
    expected.base = Some(ty(&format!("::ferroui_base::collections::FerroList<{border_ref}>")));
    expected.interfaces = vec![ty(&rc(&format!("dyn {IBRUSH}")))];
    expected.generic = Some(GenericModel { definition: "FerroList`1".to_string(), arguments: vec![ty(&border_ref)] });
    expected.content_property = Some("Value".to_string());
    expected.constructors = vec![MemberModel {
        is_static: true,
        callable: path("Setter::empty", &format!("{SETTER}::empty")),
        typed_function: Some("__markup_new_0".to_string()),
        call: Some(CallForm::Invoker),
        ..MemberModel::default()
    }];
    expected.notify_property_changed = Some(ty(SETTER));
    assert_eq!(the_type(&scan, "Setter"), &expected, "\n{}", listing(&scan.diagnostics));

    let mut expected = TypeModel::new("IBrush", TypeKind::Interface, ty(&format!("dyn {IBRUSH}")), "fixture::media");
    expected.namespace = "Fixture.Media".to_string();
    expected.public_path = Some("::fixture::IBrush".to_string());
    expected.handles = vec![ty(&rc(&format!("dyn {IBRUSH}"))), ty(&format!("Option<{}>", rc(&format!("dyn {IBRUSH}"))))];
    assert_eq!(the_type(&scan, "IBrush"), &expected, "\n{}", listing(&scan.diagnostics));

    let mut expected = TypeModel::new("Colors", TypeKind::Static, unresolved("Colors", &["Colors"]), "fixture::markup_types::plain");
    expected.namespace = "Fixture.Media".to_string();
    expected.explicit_namespace = Some("Fixture.Media".to_string());
    expected.static_properties = vec![PropertyModel {
        name: "Red".to_string(),
        parameters: Vec::new(),
        value_type: ty("u32"),
        getter: Some(accessor(CallableModel { path: Some("Colors::red".to_string()), resolved: None, dereferenced: None }, Some("__markup_static_get_Red"))),
        setter: None,
        attributes: Vec::new(),
    }];
    assert_eq!(the_type(&scan, "Colors"), &expected, "\n{}", listing(&scan.diagnostics));

    let member = |name: &str, variant: &str, value: i64| EnumMemberModel { name: name.to_string(), rust_variant: Some(variant.to_string()), rust_value: None, value: Some(value) };
    let dock = the_type(&scan, "Dock");
    assert_eq!(dock.enum_members, vec![member("Left", "Left", 0), member("Bottom", "Bottom", 4), member("Right", "Right", 5), member("Top", "Top", -1)]);
    assert!(!dock.is_flags && dock.handles.is_empty());

    let mode = the_type(&scan, "Mode");
    assert_eq!(mode.enum_members, vec![member("DataContext", "DataContext", 0), member("Self", "Self_", 1)]);
    assert_eq!(mode.explicit_namespace.as_deref(), Some("Fixture.Data"));
    assert_eq!(mode.attributes, vec![attribute("Flagged", Vec::new())]);

    let flag = |name: &str, constant: &str, value: Option<i64>| EnumMemberModel { name: name.to_string(), rust_variant: None, rust_value: Some(constant.to_string()), value };
    let routes = the_type(&scan, "Routes");
    assert!(routes.is_flags);
    assert_eq!(
        routes.enum_members,
        vec![
            flag("Direct", "Routes::DIRECT", Some(1)),
            flag("Tunnel", "Routes::TUNNEL", Some(2)),
            flag("Both", "Routes::BOTH", Some(6)),
            flag("Odd", "Routes::ODD", None),
        ]
    );
}

/// The assembly and the namespace table of `register_types.rs`: literals, a text constant
/// of the crate and the constants of the base crate an assembly names; a value that is
/// computed is left out and reported.
#[test]
fn assembly_and_namespace_table_are_read() {
    let scan = fixture();
    let model = &scan.model;
    let pair = |first: &str, second: &str| (first.to_string(), second.to_string());
    assert_eq!((model.name.as_str(), model.crate_name.as_str()), ("Fixture", "fixture"));
    assert_eq!(
        model.namespaces,
        vec![
            pair("fixture", "Fixture"),
            pair("fixture::controls", "Fixture.Controls"),
            pair("fixture::controls::text_block::documents", "Fixture.Controls.Documents"),
            pair("fixture::media", "Fixture.Media"),
        ]
    );
    assert_eq!(
        model.xmlns_definitions,
        vec![
            XmlnsDefinitionModel { xml_namespace: crate::FERRO_XML_NAMESPACE.to_string(), namespace: "Fixture.Controls".to_string() },
            XmlnsDefinitionModel { xml_namespace: "https://example.org/fixture".to_string(), namespace: "Fixture.Media".to_string() },
        ]
    );
    assert_eq!(model.xmlns_prefixes, vec![XmlnsPrefixModel { xml_namespace: "https://example.org/fixture".to_string(), prefix: "f".to_string() }]);
    assert_eq!(model.metadata, vec![pair(crate::CREATE_SOURCE_INFO, "true")]);
    assert!(model.documents.is_empty() && model.dependencies.is_empty());
}

/// What the scanner does not read is reported with the file and the line, and nothing
/// else is: a form a macro does not have, a declaration inside a function, a macro of the
/// crate that is not expanded, a path that is not resolved, a registration that is not
/// read, a runtime type written by hand, a computed value of the assembly, a public path
/// the crate states differently.
#[test]
fn what_is_not_read_is_reported_with_its_place() {
    let scan = fixture();
    let directory = fixture_directory();
    let reported: Vec<(String, usize, Severity, &str)> =
        scan.diagnostics.iter().map(|diagnostic| (relative(&diagnostic.file, &directory), diagnostic.line, diagnostic.severity, diagnostic.code)).collect();
    let expected: Vec<(String, usize, Severity, &str)> = [
        ("controls/border.rs", 57, Severity::Note, codes::POSITION),
        ("controls/decorator.rs", 20, Severity::Warning, codes::UNRESOLVED),
        ("controls/grid.rs", 8, Severity::Warning, codes::UNRESOLVED),
        ("controls/grid.rs", 17, Severity::Warning, codes::UNRESOLVED),
        ("controls/grid.rs", 33, Severity::Warning, codes::REGISTRATION),
        ("controls/grid.rs", 37, Severity::Warning, codes::REGISTRATION),
        ("macros.rs", 32, Severity::Warning, codes::LOCAL_MACRO),
        ("markup_types/classes.rs", 12, Severity::Warning, codes::UNRESOLVED),
        ("markup_types/plain.rs", 26, Severity::Warning, codes::UNRESOLVED),
        ("markup_types/plain.rs", 32, Severity::Error, codes::FORM),
        ("markup_types/plain.rs", 35, Severity::Error, codes::FORM),
        ("odd/inner.rs", 10, Severity::Note, codes::HAND_WRITTEN),
        ("register_types.rs", 14, Severity::Error, codes::ASSEMBLY),
        ("rust_paths.rs", 6, Severity::Note, codes::PUBLIC_PATH),
    ]
    .into_iter()
    .map(|(file, line, severity, code)| (file.to_string(), line, severity, code))
    .collect();
    assert_eq!(reported, expected, "\n{}", listing(&scan.diagnostics));

    let message = |code: &str, line: usize| {
        scan.diagnostics.iter().find(|diagnostic| diagnostic.code == code && diagnostic.line == line).map(|diagnostic| diagnostic.message.as_str()).unwrap_or_default()
    };
    assert_eq!(
        message(codes::FORM, 32),
        "`ferro_markup_type!`: expected `class`, `struct`, `interface` or `static`, found `record`: the declaration is not read"
    );
    assert_eq!(message(codes::FORM, 35), "`ferro_markup_type!`: `surprise` is not a part of markup metadata: the declaration is not read");
    assert!(message(codes::UNRESOLVED, 20).starts_with("the path `Thickness` of the type `Thickness` is not resolved"), "{}", message(codes::UNRESOLVED, 20));
    assert!(message(codes::REGISTRATION, 37).contains("`wrong_property` returns `Rc<Something>`"), "{}", message(codes::REGISTRATION, 37));
    assert!(message(codes::ASSEMBLY, 14).starts_with("`computed()` in the xmlns definitions of the assembly"), "{}", message(codes::ASSEMBLY, 14));
    assert!(message(codes::PUBLIC_PATH, 6).contains("`::fixture::controls::grid::Grid`; the scanner finds `::fixture::controls::Grid`"), "{}", message(codes::PUBLIC_PATH, 6));
    let text = scan.diagnostics[9].to_string();
    assert!(text.ends_with("plain.rs(32): error FRN9010: `ferro_markup_type!`: expected `class`, `struct`, `interface` or `static`, found `record`: the declaration is not read"), "{text}");
    // Neither of the two declarations is in the model.
    assert!(scan.model.types.iter().all(|type_| type_.name != "Odd" && type_.name != "Other" && type_.name != "Local" && type_.name != "First"));
}

/// The text of a source file without its comments, and with the text of its string and
/// character literals left out: what is left names a macro only where it is invoked.
fn code_of(text: &str) -> String {
    let characters: Vec<char> = text.chars().collect();
    let at = |index: usize| characters.get(index).copied();
    let is_word = |character: char| character.is_alphanumeric() || character == '_';
    let mut code = String::with_capacity(text.len());
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index];
        if character == '/' && at(index + 1) == Some('/') {
            while index < characters.len() && characters[index] != '\n' {
                index += 1;
            }
            continue;
        }
        if character == '/' && at(index + 1) == Some('*') {
            let mut depth = 1;
            index += 2;
            while index < characters.len() && depth > 0 {
                if characters[index] == '/' && at(index + 1) == Some('*') {
                    depth += 1;
                    index += 2;
                } else if characters[index] == '*' && at(index + 1) == Some('/') {
                    depth -= 1;
                    index += 2;
                } else {
                    index += 1;
                }
            }
            code.push(' ');
            continue;
        }
        // A raw string: `r"…"`, `r#"…"#`, `br#"…"#`.
        let after_word = index > 0 && is_word(characters[index - 1]);
        if character == 'r' && !(after_word && characters[index - 1] != 'b') {
            let mut hashes = 0;
            while at(index + 1 + hashes) == Some('#') {
                hashes += 1;
            }
            if at(index + 1 + hashes) == Some('"') {
                index += 2 + hashes;
                'raw: while index < characters.len() {
                    if characters[index] == '"' && (0..hashes).all(|hash| at(index + 1 + hash) == Some('#')) {
                        index += 1 + hashes;
                        break 'raw;
                    }
                    index += 1;
                }
                code.push_str("\"\"");
                continue;
            }
        }
        if character == '"' {
            index += 1;
            while index < characters.len() {
                match characters[index] {
                    '\\' => index += 2,
                    '"' => {
                        index += 1;
                        break;
                    }
                    _ => index += 1,
                }
            }
            code.push_str("\"\"");
            continue;
        }
        if character == '\'' {
            // A character literal (`'x'`, `'\n'`, `'\u{1F600}'`); anything else is a lifetime.
            if at(index + 1) == Some('\\') {
                index += 3;
                while index < characters.len() && characters[index] != '\'' {
                    index += 1;
                }
                index += 1;
                code.push_str("' '");
                continue;
            }
            if at(index + 2) == Some('\'') {
                index += 3;
                code.push_str("' '");
                continue;
            }
        }
        code.push(character);
        index += 1;
    }
    code
}

/// The number of invocations `name!(..)`, `name! {..}` or `name![..]` in `code`.
fn invocations_in(code: &str, name: &str) -> usize {
    let is_word = |character: char| character.is_alphanumeric() || character == '_';
    let mut count = 0;
    let mut from = 0;
    while let Some(found) = code[from..].find(name) {
        let start = from + found;
        let end = start + name.len();
        from = end;
        if code[..start].chars().next_back().is_some_and(is_word) {
            continue;
        }
        let mut rest = code[end..].chars().skip_while(|character| character.is_whitespace());
        if rest.next() != Some('!') {
            continue;
        }
        if matches!(rest.find(|character| !character.is_whitespace()), Some('(' | '{' | '[')) {
            count += 1;
        }
    }
    count
}

/// The files in which the scanner met another number of invocations of a declaration
/// macro than the text of the file has, each with both numbers.
fn skipped_invocations(scan: &Scan) -> Vec<String> {
    let mut differences = Vec::new();
    for file in &scan.files {
        let text = std::fs::read_to_string(&file.path).unwrap_or_else(|error| panic!("{}: {error}", file.path.display()));
        let code = code_of(&text);
        for name in DECLARATION_MACRO_NAMES {
            let written = invocations_in(&code, name);
            let met = file.invocations.get(*name).copied().unwrap_or_default();
            if written != met.total() {
                differences.push(format!(
                    "{}: `{name}!` is invoked {written} times in the text, the scanner met {} ({} read, {} not read: form, {} not read: position, {} in macro definitions, {} in test code)",
                    file.path.display(),
                    met.total(),
                    met.read,
                    met.failed,
                    met.unread,
                    met.in_macro_definitions,
                    met.in_test_code
                ));
            }
        }
    }
    differences
}

/// The counter of invocations the tests compare the scanner with reads comments, strings,
/// character literals and lifetimes as the language does.
#[test]
fn text_counter_leaves_comments_and_literals_out() {
    let text = concat!(
        "// ferro_class!(A: B)\n",
        "/* ferro_class!(A: B) /* nested ferro_class!(A: B) */ still */ ferro_class!(C: D);\n",
        "/// ferro_class!(A: B)\n",
        "const A: &str = \"ferro_class!(A: B) \\\" ferro_class!(A: B)\";\n",
        "const B: &str = r#\"ferro_class!(A: B) \" ferro_class!(A: B)\"#;\n",
        "const C: char = '\"'; ferro_class ! { E: F }\n",
        "const D: char = '\\''; fn f<'a>(x: &'a str) { ferro_class![G: H]; }\n",
        "my_ferro_class!(I: J); ferro_class_info!(K {}); ferro_class; macro_rules! ferro_class { () => {} }\n",
    );
    let code = code_of(text);
    assert_eq!(invocations_in(&code, "ferro_class"), 3, "{code}");
    assert_eq!(invocations_in(&code, "ferro_class_info"), 1, "{code}");
    assert_eq!(invocations_in(&code, "ferro_property"), 0, "{code}");
}

/// Every invocation of a declaration macro in the files of the fixture is met by the
/// scanner, and each is counted by what became of it.
#[test]
fn every_invocation_of_the_fixture_is_met() {
    let scan = fixture();
    let directory = fixture_directory();
    assert_eq!(skipped_invocations(&scan), Vec::<String>::new());

    let files: Vec<String> = scan.files.iter().map(|file| relative(&file.path, &directory)).collect();
    assert_eq!(
        files,
        [
            "lib.rs",
            "controls/mod.rs",
            "controls/border.rs",
            "controls/decorator.rs",
            "controls/grid.rs",
            "controls/text_block.rs",
            "macros.rs",
            "markup_types/mod.rs",
            "markup_types/classes.rs",
            "markup_types/plain.rs",
            "media/mod.rs",
            "media/brush.rs",
            "odd/placed_elsewhere.rs",
            "odd/inner.rs",
            "register_types.rs",
            "rust_paths.rs",
        ]
    );
    let module = |file: &str| scan.files.iter().find(|scanned| relative(&scanned.path, &directory) == file).map(|scanned| scanned.module.as_str());
    assert_eq!(module("odd/placed_elsewhere.rs"), Some("fixture::placed"));
    assert_eq!(module("odd/inner.rs"), Some("fixture::placed::inner"));
    assert_eq!(module("controls/text_block.rs"), Some("fixture::controls::text_block"));

    let counts = |read, failed, in_macro_definitions, unread, in_test_code| InvocationCounts { read, failed, in_macro_definitions, unread, in_test_code };
    let expected = BTreeMap::from([
        ("ferro_class".to_string(), counts(8, 0, 2, 1, 1)),
        ("ferro_class_info".to_string(), counts(3, 0, 1, 0, 0)),
        ("ferro_markup_enum".to_string(), counts(3, 0, 0, 0, 0)),
        // One in the definition of the macro the crate exports (`typed_list!`).
        ("ferro_markup_type".to_string(), counts(5, 2, 1, 0, 0)),
        ("ferro_properties".to_string(), counts(5, 0, 0, 0, 0)),
        ("ferro_property".to_string(), counts(5, 0, 0, 0, 0)),
        ("ferro_static_type".to_string(), counts(2, 0, 0, 0, 0)),
    ]);
    assert_eq!(scan.statistics.invocations, expected);
    let border = scan.files.iter().find(|file| relative(&file.path, &directory) == "controls/border.rs").expect("the file of the border");
    assert_eq!(
        border.invocations,
        BTreeMap::from([
            ("ferro_class".to_string(), counts(1, 0, 0, 1, 0)),
            ("ferro_class_info".to_string(), counts(1, 0, 0, 0, 0)),
            ("ferro_properties".to_string(), counts(1, 0, 0, 0, 0)),
            ("ferro_property".to_string(), counts(5, 0, 0, 0, 0)),
        ])
    );
}

/// The numbers of the scan of the fixture, the functions of its `impl` blocks, and the
/// normalisation of a further type text against its modules.
#[test]
fn statistics_functions_and_normalisation_of_the_fixture() {
    let scan = fixture();
    let statistics = &scan.statistics;
    assert_eq!((statistics.files, statistics.modules), (16, 18), "{}", scan.summary());
    assert_eq!((statistics.classes, statistics.static_types, statistics.markup_types, statistics.enums), (10, 2, 4, 3), "{}", scan.summary());
    assert_eq!((statistics.types_without_namespace, statistics.types_without_public_path), (0, 6), "{}", scan.summary());
    assert_eq!((statistics.registered, statistics.styled, statistics.direct, statistics.attached), (11, 7, 1, 3), "{}", scan.summary());
    assert_eq!(
        (statistics.added_owners, statistics.aliases, statistics.unknown_registrations, statistics.registered_without_name),
        (1, 1, 1, 1),
        "{}",
        scan.summary()
    );
    assert_eq!(
        (statistics.constructors, statistics.properties, statistics.indexers, statistics.methods, statistics.fields, statistics.events),
        (5, 5, 1, 4, 3, 2),
        "{}",
        scan.summary()
    );
    assert_eq!((statistics.callables, statistics.callable_paths, statistics.callable_paths_resolved), (28, 20, 19), "{}", scan.summary());
    assert_eq!((statistics.enum_members, statistics.enum_members_without_value), (10, 1), "{}", scan.summary());
    assert_eq!(statistics.type_texts_unresolved, 5, "{}", scan.summary());
    assert_eq!(
        statistics.unresolved_paths,
        BTreeMap::from([("Colors".to_string(), 1), ("Control".to_string(), 2), ("FerroProperty".to_string(), 1), ("Thickness".to_string(), 1)])
    );
    assert_eq!((statistics.rust_paths_listed, statistics.rust_paths_agreeing), (4, 3), "{}", scan.summary());
    assert!(scan.summary().contains("ferro_class!: 12 invocations (8 read, 0 not read: form, 1 not read: position, 2 in macro definitions, 1 in test code)"), "{}", scan.summary());

    let functions: Vec<(&str, &str, &str, bool, Vec<&str>, Option<&str>, Option<&str>, usize)> = scan
        .functions
        .iter()
        .map(|function| {
            (
                function.owner.as_str(),
                function.name.as_str(),
                function.visibility.as_str(),
                function.receiver,
                function.parameters.iter().map(String::as_str).collect(),
                function.return_type.as_deref(),
                function.declared_by.as_deref(),
                function.line,
            )
        })
        .collect();
    assert_eq!(
        functions,
        vec![
            (BORDER, "new", "pub", false, vec![], Some("Ref<Border>"), None, 43),
            (BORDER, "pressed_event", "pub", false, vec![], Some("RoutedEvent<RoutedEventArgs>"), Some("ferro_routed_event"), 51),
            (BORDER, "thickness", "pub(crate)", true, vec!["f64"], Some("f64"), None, 55),
            (BORDER, "with_name", "pub", false, vec!["String"], Some("Ref<Border>"), None, 61),
            (BORDER, "tag", "pub", true, vec![], Some("Option<BoxedValue>"), None, 66),
            (BORDER, "set_tag", "pub(crate)", true, vec!["Option<BoxedValue>"], None, None, 70),
            (BORDER, "default_thickness", "pub", false, vec![], Some("Thickness"), None, 72),
            (BORDER, "set_default_thickness", "", false, vec!["Thickness"], Some("Result<(), String>"), None, 76),
            (BORDER, "brush_at", "pub", true, vec![], Some("Ref<Brush>"), None, 81),
            (DECORATOR, "new", "pub", false, vec![], Some("Ref<Decorator>"), None, 31),
        ]
    );
    assert_eq!(scan.functions[0].module, "fixture::controls::border");

    // The call forms (9.5.3): of the 20 callables that are paths, one is the accessor a
    // declaration macro writes, three are public functions with the arguments of the
    // declaration and one is visible in the crate only; of the closures one dereferences
    // such an accessor; the others go through the invoker, each for its reason.
    assert_eq!(
        statistics.call_forms,
        crate::call_forms::CallFormStatistics {
            structural: 2,
            path: 3,
            crate_path: 1,
            invoker: 17,
            closures: 7,
            unresolved: 1,
            not_inherent: 7,
            signature: 1,
            private: 1,
            no_public_path: 0,
        },
        "{}",
        scan.summary()
    );
    // The public functions of the types another crate can name, for the crates built on this one.
    let exported: Vec<(&str, &str, Vec<&str>)> = scan
        .model
        .functions
        .iter()
        .map(|functions| (functions.owner.as_str(), functions.public_path.as_str(), functions.functions.iter().map(|function| function.name.as_str()).collect()))
        .collect();
    assert_eq!(
        exported,
        vec![
            (BORDER, "::fixture::Border", vec!["new", "pressed_event", "with_name", "tag", "default_thickness", "brush_at"]),
            (DECORATOR, "::fixture::Decorator", vec!["new"]),
        ]
    );
    let pressed = &scan.model.functions[0].functions[1];
    assert_eq!((pressed.receiver, pressed.parameters, pressed.declared_by.as_deref()), (false, 0, Some("ferro_routed_event")));

    assert_eq!(scan.normalise("fixture::controls::border", "Option<Rc<dyn IBrush>>"), Some(ty(&format!("Option<::std::rc::Rc<dyn {IBRUSH}>>"))));
    assert_eq!(scan.normalise("fixture::markup_types::classes", "&Ref<Border>"), Some(ty(&format!("&::ferroui_base::Ref<{BORDER}>"))));
    assert_eq!(scan.normalise("fixture::controls::grid", "Ref<Control>"), Some(unresolved("Ref<Control>", &["Ref", "Control"])));
    assert_eq!(scan.normalise("fixture::controls::missing", "f64"), None);
}

/// The model of the fixture is read back from its `.xamlmeta` text as it was scanned, and
/// the reader of the compiler reads the same text (it has no documents).
#[test]
fn fixture_model_round_trips_through_its_file() {
    let scan = fixture();
    let text = scan.model.to_json();
    assert_eq!(crate::model::AssemblyModel::parse(&text).as_ref(), Ok(&scan.model));
    let metadata = XamlMetadata::parse(&text).expect("the reader of format 1 reads a file of format 2");
    assert_eq!((metadata.name.as_str(), metadata.crate_name.as_str(), metadata.documents.len()), ("Fixture", "fixture", 0));
    assert!(text.contains("\"format\": 2"));
    assert!(text.contains("\"unresolved\": [\n"), "an unresolved type is written with its unresolved paths");
}

/// The export table of the fixture: every path another crate can write for a type, with
/// the module that declares it; a private module is on no such path.
#[test]
fn export_table_of_the_fixture_has_every_public_path() {
    let scan = fixture();
    let declared = |path: &str| scan.model.exports.iter().find(|export| export.path == path).map(|export| export.declared.as_str());
    assert_eq!(declared("::fixture::Border"), Some(BORDER), "{:?}", scan.model.exports);
    assert_eq!(declared("::fixture::controls::Border"), Some(BORDER));
    assert_eq!(declared("::fixture::Decorator"), Some(DECORATOR));
    assert_eq!(declared("::fixture::controls::decorator::Decorator"), Some(DECORATOR));
    assert_eq!(declared("::fixture::IBrush"), Some(IBRUSH));
    assert_eq!(declared("::fixture::media::IBrush"), Some(IBRUSH));
    assert_eq!(declared("::fixture::Thickness"), Some(THICKNESS));
    assert_eq!(declared("::fixture::media::Dock"), Some("::fixture::media::Dock"));
    assert_eq!(declared("::fixture::controls::border::Border"), None);
    assert_eq!(declared("::fixture::media::brush::IBrush"), None);
    let mut paths: Vec<&str> = scan.model.exports.iter().map(|export| export.path.as_str()).collect();
    let listed = paths.len();
    paths.dedup();
    assert_eq!(paths.len(), listed, "a path is listed once");
    // The types of the object model, and no other, are marked as such.
    let object_model: Vec<&str> = scan.model.types.iter().filter(|type_| type_.object_model).map(|type_| type_.name.as_str()).collect();
    assert_eq!(object_model, ["Border", "Decorator", "Grid", "Layout", "TextBlock", "Run", "Span", "Brush", "Deep", "Placed", "DoubleTransition", "Root"]);
}

/// A crate built on another one, scanned alone and with the model of the other: the names
/// behind the glob import of the other crate, the declaring modules of its types and the
/// name of a property of it the crate adds an owner to are resolved only with the model.
#[test]
fn dependent_crate_is_resolved_with_the_model_of_the_crate_it_is_built_on() {
    let root = dependent_directory().join("lib.rs");
    let brush = format!("Option<::std::rc::Rc<dyn {IBRUSH}>>");

    let alone = scan_crate(&ScanOptions::new("dependent", root.clone()));
    assert_eq!(
        alone.statistics.unresolved_paths,
        BTreeMap::from([("Border".to_string(), 1), ("Dock".to_string(), 1), ("IBrush".to_string(), 2)]),
        "\n{}",
        listing(&alone.diagnostics)
    );
    assert_eq!(alone.statistics.registered_without_name, 2, "{}", alone.summary());
    let card = the_type(&alone, "Card");
    assert_eq!(card.base, Some(unresolved("Border", &["Border"])));
    assert_eq!(card.registered[0].source, Some(CallableModel { path: Some("Border::background_property".to_string()), resolved: None, dereferenced: None }));
    let margin = the_type(&alone, "Margin");
    assert_eq!(margin.properties[2].value_type, ty("Option<::ferroui_base::Ref<::fixture::Decorator>>"));

    let scan = scan_crate(&ScanOptions::new("dependent", root).with_dependencies(vec![fixture().model]));
    assert!(scan.statistics.unresolved_paths.is_empty(), "{}\n{}", scan.summary(), listing(&scan.diagnostics));
    assert_eq!(scan.statistics.type_texts_unresolved, 0, "{}", scan.summary());
    assert_eq!(scan.statistics.registered_without_name, 0, "{}", scan.summary());
    let card = the_type(&scan, "Card");
    assert!(card.object_model);
    assert_eq!(card.base, Some(ty(BORDER)));
    let names: Vec<(Option<&str>, RegistrationModel)> = card.registered.iter().map(|registered| (registered.name.as_deref(), registered.registration)).collect();
    assert_eq!(
        names,
        [(Some("Background"), RegistrationModel::AddedOwner), (Some("Dock"), RegistrationModel::Declared), (Some("Background"), RegistrationModel::Alias)]
    );
    assert_eq!(card.registered[0].value_type, ty(&brush));
    assert_eq!(card.registered[0].source, Some(path("Border::background_property", &format!("{BORDER}::background_property"))));
    assert_eq!(card.registered[1].value_type, ty("::fixture::media::Dock"));
    assert_eq!(card.registered[1].owner, Some(ty("::dependent::Card")));
    assert_eq!(card.registered[2].value_type, ty(&brush));
    let margin = the_type(&scan, "Margin");
    let types: Vec<&RustType> = margin.properties.iter().map(|property| &property.value_type).collect();
    assert_eq!(types, [&ty(THICKNESS), &ty("Option<::fixture::media::Dock>"), &ty(&format!("Option<::ferroui_base::Ref<{DECORATOR}>>"))]);
    assert_eq!(
        scan.model.exports.iter().map(|export| export.path.as_str()).collect::<Vec<_>>(),
        ["::dependent::Card", "::dependent::Cards", "::dependent::panel::Margin"]
    );

    // A macro the other crate exports that declares through a declaration macro is in the
    // model of that crate with its rules, and its invocation here is expanded with them:
    // the list is a type of this crate, `$crate` is the path of the other crate. Without
    // the model the invocation declares nothing.
    let exported = fixture().model.macros;
    assert_eq!(exported.iter().map(|exported| (exported.name.as_str(), exported.rules.len())).collect::<Vec<_>>(), [("typed_list", 1)]);
    assert!(alone.model.types.iter().all(|type_| type_.name != "List`1"));
    let cards = the_type(&scan, "List`1");
    assert_eq!(cards.rust_path, ty("::dependent::Cards"));
    let card = "::ferroui_base::Ref<::dependent::Card>";
    assert_eq!(cards.handles, [ty(&format!("Vec<{card}>")), ty(&format!("Option<Vec<{card}>>"))]);
    assert_eq!(cards.generic.as_ref().map(|generic| (generic.definition.as_str(), generic.arguments.clone())), Some(("List`1", vec![ty(card)])));
    let properties: Vec<(&str, &RustType)> = cards.properties.iter().map(|property| (property.name.as_str(), &property.value_type)).collect();
    assert_eq!(properties, [("Count", &ty("i32")), ("Dock", &ty("Option<::fixture::media::Dock>"))]);

    // The call forms: a function of the other crate is known from the model of that crate
    // only. With it, the public function is called by its path, the accessor of a routed
    // event is form A, and a function that model does not list (a private one) and the
    // function of this crate no `impl` block has are called through the invoker.
    let forms = |scan: &Scan| {
        let margin = the_type(scan, "Margin");
        let default = &margin.static_properties[0];
        (
            default.getter.as_ref().and_then(|getter| getter.call.clone()),
            default.setter.as_ref().and_then(|setter| setter.call.clone()),
            margin.fields[0].call.clone(),
            margin.properties[2].getter.as_ref().and_then(|getter| getter.call.clone()),
        )
    };
    let invoker = Some(CallForm::Invoker);
    assert_eq!(forms(&alone), (invoker.clone(), invoker.clone(), invoker.clone(), invoker.clone()));
    assert_eq!(forms(&scan), (border_function("default_thickness"), invoker.clone(), Some(CallForm::Structural), invoker));
    assert_eq!((scan.statistics.call_forms.structural, scan.statistics.call_forms.path, scan.statistics.call_forms.not_inherent), (1, 1, 2), "{}", scan.summary());
}

/// A class of a scanned crate by its full name; the failure lists the diagnostics that
/// may say why it is missing.
fn class<'a>(scan: &'a Scan, full_name: &str) -> &'a TypeModel {
    scan.model.find_type(full_name).unwrap_or_else(|| {
        let name = full_name.rsplit('.').next().unwrap_or(full_name);
        let similar: Vec<String> = scan.model.types.iter().filter(|type_| type_.name == name).map(|type_| format!("{} ({})", type_.full_name(), type_.rust_path.text)).collect();
        let related: Vec<String> = scan.diagnostics.iter().filter(|diagnostic| diagnostic.message.contains(&format!("`{name}`"))).map(|diagnostic| diagnostic.to_string()).collect();
        panic!("the model of {} has no type `{full_name}`; types named `{name}`: {similar:?}; diagnostics that name it:\n{}", scan.model.crate_name, related.join("\n"))
    })
}

/// The registered property `name` of the type or of a base class of it in the same crate.
fn inherited<'a>(scan: &'a Scan, type_: &'a TypeModel, name: &str) -> Option<(&'a TypeModel, &'a RegisteredModel)> {
    let mut current = type_;
    for _ in 0..32 {
        if let Some(registered) = current.registered(name) {
            return Some((current, registered));
        }
        current = scan.model.find_rust_type(&current.base.as_ref()?.text)?;
    }
    None
}

/// The base crate and the controls crate of the workspace, read as files (nothing of them
/// is linked into this test). The test checks what must hold whatever the crates
/// declare, and prints the numbers of each scan and what the scanner did not read, for
/// the reader to judge the coverage (`cargo test -p ferroui-build-scan real_crates -- --nocapture`).
#[test]
fn real_crates_are_scanned_without_skipping_a_declaration() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("the directory of the crates").to_path_buf();
    let base = scan_crate(&ScanOptions::new("ferroui_base", source.join("FerroUI.Base").join("lib.rs")));
    let mut options = ScanOptions::new("ferroui_controls", source.join("FerroUI.Controls").join("lib.rs"));
    options.extern_crates = vec!["ferroui_base".to_string()];
    let controls = scan_crate(&options);

    for scan in [&base, &controls] {
        println!("==== {} ====\n{}\n", scan.model.crate_name, scan.summary());
        let not_read: Vec<&Diagnostic> = scan
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error || matches!(diagnostic.code, codes::LOCAL_MACRO | codes::HAND_WRITTEN | codes::REGISTRATION | codes::PUBLIC_PATH))
            .collect();
        println!("---- {}: what is not read, or read differently ({}) ----", scan.model.crate_name, not_read.len());
        for diagnostic in not_read {
            println!("{diagnostic}");
        }
        let positions = scan.diagnostics.iter().filter(|diagnostic| diagnostic.code == codes::POSITION).count();
        println!("---- {}: {positions} declarations inside functions or other items (not listed) ----\n", scan.model.crate_name);
        let written = std::env::temp_dir().join(format!("ferroui-build-scan-{}.xamlmeta", scan.model.crate_name));
        match std::fs::write(&written, scan.model.to_json()) {
            Ok(()) => println!("the model of {} is written to {}\n", scan.model.crate_name, written.display()),
            Err(error) => println!("the model of {} is not written to {}: {error}\n", scan.model.crate_name, written.display()),
        }
    }

    for scan in [&base, &controls] {
        let crate_name = &scan.model.crate_name;
        // The files are read: a file the reader cannot parse hides everything it declares.
        let files: Vec<&Diagnostic> = scan.diagnostics.iter().filter(|diagnostic| diagnostic.code == codes::FILE && diagnostic.severity == Severity::Error).collect();
        assert!(files.is_empty(), "{crate_name}: {} files are not read:\n{}", files.len(), files.iter().map(|diagnostic| diagnostic.to_string()).collect::<Vec<_>>().join("\n"));
        assert!(scan.files.len() > 100, "{crate_name}: only {} files are read: the `mod` declarations are not followed", scan.files.len());

        // No invocation of a declaration macro is skipped: the text of every file has as
        // many as the scanner met there (read, reported, in a macro definition or in test code).
        let skipped = skipped_invocations(scan);
        assert!(
            skipped.is_empty(),
            "{crate_name}: the scanner skipped invocations of declaration macros (or the text counter of this test misreads a comment or a literal) in {} places:\n{}",
            skipped.len(),
            skipped.join("\n")
        );
        for name in ["ferro_class", "ferro_properties", "ferro_property", "ferro_class_info", "ferro_markup_type", "ferro_markup_enum"] {
            let counts = scan.statistics.invocations.get(name).copied().unwrap_or_default();
            assert!(counts.read > 0, "{crate_name}: no `{name}!` is read ({counts:?})");
        }

        // The model is its file.
        let text = scan.model.to_json();
        let read = crate::model::AssemblyModel::parse(&text).unwrap_or_else(|error| panic!("{crate_name}: the model is not read back: {error}"));
        assert!(read == scan.model, "{crate_name}: the model read back from its text differs from the scanned one");

        // The assembly and the namespaces.
        assert!(!scan.model.namespaces.is_empty(), "{crate_name}: the namespace table (`const NAMESPACES`) is not read");
        assert!(!scan.model.xmlns_definitions.is_empty(), "{crate_name}: the xmlns definitions of the assembly are not read");

        // Every registered property has an accessor and a value type; every class but the
        // root has a base.
        for type_ in &scan.model.types {
            for registered in &type_.registered {
                assert!(!registered.accessor.is_empty() && !registered.value_type.text.is_empty(), "{crate_name}: {}: {registered:?}", type_.full_name());
            }
        }
    }
    assert_eq!(base.model.name, "FerroUI.Base");
    assert_eq!(controls.model.name, "FerroUI.Controls");

    // Well-known classes and properties.
    let border = class(&controls, "FerroUI.Controls.Border");
    assert_eq!(border.kind, TypeKind::Class);
    assert_eq!(border.rust_path, ty("::ferroui_controls::border::Border"));
    assert_eq!(border.public_path.as_deref(), Some("::ferroui_controls::Border"));
    assert_eq!(border.base, Some(ty("::ferroui_controls::decorator::Decorator")));
    assert!(border.default_constructor.is_some(), "Border has `new:` ({border:?})");
    let (owner, child) = inherited(&controls, border, "Child").unwrap_or_else(|| panic!("Border.Child is not found on Border or its base classes: {:?}", border.registered));
    assert_eq!(owner.name, "Decorator");
    assert_eq!((child.kind, child.registration, child.accessor.as_str()), (RegisteredKind::Styled, RegistrationModel::Declared, "child_property"));
    assert_eq!(child.value_type, ty("Option<::ferroui_base::Ref<::ferroui_controls::control::Control>>"));
    assert_eq!(child.owner, Some(ty("::ferroui_controls::decorator::Decorator")));
    let background = border.registered("Background").unwrap_or_else(|| panic!("Border.Background: {:?}", border.registered));
    assert_eq!(background.kind, RegisteredKind::Styled);
    assert!(background.value_type.is_resolved() && background.value_type.text.contains("IBrush"), "{:?}", background.value_type);

    let control = class(&controls, "FerroUI.Controls.Control");
    let tag = control.registered("Tag").unwrap_or_else(|| panic!("Control.Tag: {:?}", control.registered));
    assert_eq!((tag.kind, tag.accessor.as_str(), tag.visibility.as_str()), (RegisteredKind::Styled, "tag_property", "pub"));
    assert!(tag.value_type.is_resolved() && tag.value_type.text.starts_with("Option<::ferroui_base::") && tag.value_type.text.ends_with("BoxedValue>"), "{:?}", tag.value_type);

    let text_block = class(&controls, "FerroUI.Controls.TextBlock");
    let text = text_block.registered("Text").unwrap_or_else(|| panic!("TextBlock.Text: {:?}", text_block.registered));
    assert_eq!((text.kind, &text.value_type), (RegisteredKind::Styled, &ty("Option<String>")));

    let grid = class(&controls, "FerroUI.Controls.Grid");
    let row = grid.registered("Row").unwrap_or_else(|| panic!("Grid.Row: {:?}", grid.registered));
    assert_eq!((row.kind, &row.value_type, row.accessor.as_str()), (RegisteredKind::Attached, &ty("i32"), "row_property"));
    assert_eq!(row.owner, Some(ty("::ferroui_controls::grid::Grid")));
    assert!(row.host.as_ref().is_some_and(|host| host.is_resolved() && host.text.ends_with("::Control")), "the host of Grid.Row: {:?}", row.host);

    let visual = class(&base, "FerroUI.Visual");
    let bounds = visual.registered("Bounds").unwrap_or_else(|| panic!("Visual.Bounds: {:?}", visual.registered));
    assert_eq!((bounds.kind, bounds.accessor.as_str()), (RegisteredKind::Direct, "bounds_property"));
    assert_eq!(bounds.owner, Some(ty("::ferroui_base::visual::Visual")));
    assert!(bounds.value_type.is_resolved() && bounds.value_type.text.ends_with("Rect"), "{:?}", bounds.value_type);

    let button = class(&controls, "FerroUI.Controls.Button");
    let click = button.fields.iter().find(|field| field.name == "ClickEvent").unwrap_or_else(|| panic!("Button.ClickEvent: {:?}", button.fields));
    assert_eq!(click.typed_function.as_deref(), Some("__markup_field_ClickEvent"));
    assert!(click.is_static && click.return_type.as_ref().is_some_and(|type_| type_.text.contains("RoutedEvent<")), "{click:?}");

    // The numbers are of the size of the crates.
    assert!(controls.statistics.classes > 100, "{}", controls.summary());
    assert!(controls.statistics.registered > 500, "{}", controls.summary());
    assert!(base.statistics.classes > 50 && base.statistics.registered > 50, "{}", base.summary());

    // The call forms (9.5.3): a form is chosen for every callable but `new:` of a class and
    // the source of an added owner; every path is resolved, and the functions of the base
    // crate its model lists are what the controls call by path.
    for scan in [&base, &controls] {
        let forms = &scan.statistics.call_forms;
        assert_eq!(forms.unresolved, 0, "{}", forms.summary());
        assert_eq!(forms.closures + forms.structural + forms.path + forms.crate_path + forms.not_inherent + forms.signature + forms.private + forms.no_public_path, forms.total());
        assert!(forms.path > 100 && forms.structural > 20, "{}", forms.summary());
    }
    assert!(base.model.functions.len() > 100, "the model of ferroui_base lists the public functions of {} types", base.model.functions.len());

    // The export table of the base crate names its types by the paths the controls write.
    assert!(base.model.exports.len() > base.model.types.len(), "the export table of ferroui_base has {} paths", base.model.exports.len());
    let set = crate::model_set::ModelSet::new(vec![base.model.clone()]);
    let ref_path = set.canonical_path("::ferroui_base::Ref");
    assert!(ref_path.starts_with("::ferroui_base::") && ref_path.ends_with("::Ref"), "{ref_path}");
    assert_eq!(set.find_rust_type("::ferroui_base::Visual").map(|(_, type_)| type_.full_name()), Some("FerroUI.Visual".to_string()));

    // The controls again, with the model of the base crate attached: what the first scan
    // left open because it lives in the base crate is resolved against that model.
    let linked = scan_crate(&options.with_dependencies(vec![base.model.clone()]));
    println!("==== ferroui_controls, with the model of ferroui_base ====\n{}\n", linked.summary());
    let unresolved: Vec<String> = linked.diagnostics.iter().filter(|diagnostic| diagnostic.code == codes::UNRESOLVED).map(|diagnostic| diagnostic.to_string()).collect();
    let mut nameless: Vec<String> = Vec::new();
    for type_ in &linked.model.types {
        for registered in type_.registered.iter().filter(|registered| registered.name.is_none()) {
            nameless.push(format!("{}::{} ({:?}, source {:?})", type_.rust_path.text, registered.accessor, registered.registration, registered.source));
        }
    }
    println!("---- ferroui_controls with ferroui_base: {} unresolved paths, {} registered properties without a name ----", unresolved.len(), nameless.len());
    for line in unresolved.iter().chain(&nameless) {
        println!("{line}");
    }
    assert!(
        linked.statistics.unresolved_paths.is_empty() && linked.statistics.type_texts_unresolved == 0,
        "ferroui_controls with the model of ferroui_base: {} type texts have an unresolved path ({:?}):\n{}",
        linked.statistics.type_texts_unresolved,
        linked.statistics.unresolved_paths,
        unresolved.join("\n")
    );
    assert!(nameless.is_empty(), "ferroui_controls with the model of ferroui_base: registered properties without a name:\n{}", nameless.join("\n"));
    assert!(
        linked.statistics.call_forms.path > controls.statistics.call_forms.path,
        "with the model of ferroui_base the controls call no more functions by path: {}",
        linked.statistics.call_forms.summary()
    );
    assert_eq!(linked.statistics.registered_without_name, 0);
    assert_eq!(linked.statistics.registered, controls.statistics.registered);

    // An accessor that `ferro_property!(for Owner; ..)` declares among the members of another
    // type is listed under the owner and found by the path of the type whose function it is:
    // the theme variant properties of `StyledElement`, which `Application` adds itself to
    // through `ThemeVariant::.._property()`.
    let styled_element = class(&base, "FerroUI.StyledElement");
    let actual = styled_element.registered("ActualThemeVariant").unwrap_or_else(|| panic!("StyledElement.ActualThemeVariant: {:?}", styled_element.registered));
    assert_eq!(actual.accessor, "actual_theme_variant_property");
    assert_eq!(actual.function_of.as_deref(), Some("::ferroui_base::styling::theme_variant::ThemeVariant"));
    let application = class(&linked, "FerroUI.Application");
    for (name, accessor) in [("ActualThemeVariant", "actual_theme_variant_property"), ("RequestedThemeVariant", "requested_theme_variant_property")] {
        let added = application.registered(name).unwrap_or_else(|| panic!("Application.{name}: {:?}", application.registered));
        assert_eq!((added.registration, added.accessor.as_str(), &added.function_of), (RegistrationModel::AddedOwner, accessor, &None));
        assert_eq!(added.owner, Some(ty("::ferroui_controls::application::Application")));
    }
    let both = crate::model_set::ModelSet::new(vec![base.model.clone(), linked.model.clone()]);
    let found = both.find_accessor("::ferroui_base::styling::ThemeVariant::actual_theme_variant_property").map(|(type_, registered)| (type_.full_name(), registered.name.clone()));
    assert_eq!(found, Some(("FerroUI.StyledElement".to_string(), Some("ActualThemeVariant".to_string()))));

    // The accessors a macro of the crate writes among the members of an `impl` block are read.
    let animation = class(&base, "FerroUI.Animation.Animation");
    let duration = animation.registered("Duration").unwrap_or_else(|| panic!("Animation.Duration: {:?}", animation.registered));
    assert_eq!((duration.kind, duration.accessor.as_str(), duration.read_only), (RegisteredKind::Direct, "duration_property", false));
    let xy_focus = class(&base, "FerroUI.Input.XYFocus");
    let down = xy_focus.registered("Down").unwrap_or_else(|| panic!("XYFocus.Down: {:?}", xy_focus.registered));
    assert_eq!((down.kind, down.accessor.as_str()), (RegisteredKind::Attached, "down_property"));
    assert!(down.host.as_ref().is_some_and(|host| host.text.ends_with("::InputElement")), "{:?}", down.host);

    // A property is stated with the owner its registration names, whichever type has the accessor.
    let adorner_layer = class(&linked, "FerroUI.Controls.Primitives.AdornerLayer");
    let saved = adorner_layer.registered("SavedAdornerLayer").unwrap_or_else(|| panic!("AdornerLayer.SavedAdornerLayer: {:?}", adorner_layer.registered));
    assert_eq!(saved.owner.as_ref().map(|owner| both.canonical(&owner.text)), Some("::ferroui_base::visual::Visual".to_string()));

    // The values of the members that are constants of the type, and of the discriminants
    // that are constant expressions.
    let value_of = |scan: &Scan, type_name: &str, member: &str| {
        let type_ = class(scan, type_name);
        type_.enum_members.iter().find(|candidate| candidate.name == member).unwrap_or_else(|| panic!("{type_name}.{member}: {:?}", type_.enum_members)).value
    };
    assert_eq!(value_of(&base, "FerroUI.Input.Key", "Return"), Some(6));
    assert_eq!(value_of(&base, "FerroUI.Input.Key", "Enter"), Some(6));
    assert_eq!(value_of(&base, "FerroUI.Data.BindingPriority", "Unset"), Some(i64::from(i32::MAX)));
    assert_eq!(value_of(&base, "FerroUI.Input.KeyModifiers", "None"), Some(0));
    assert_eq!(value_of(&base, "FerroUI.Input.RawInputModifiers", "KeyboardMask"), Some(15));
    assert_eq!(base.statistics.enum_members_without_value, 0, "{}", base.summary());
    assert_eq!(controls.statistics.enum_members_without_value, 0, "{}", controls.summary());

    // The classes the lists of registered classes leave out are marked, and only those.
    for scan in [&base, &linked] {
        assert!(scan.model.types.iter().all(|type_| type_.object_model || !type_.unregistered), "{}", scan.summary());
        assert!(scan.statistics.unregistered * 10 < scan.statistics.classes, "{}", scan.summary());
    }
    assert!(!class(&linked, "FerroUI.Controls.Border").unregistered && !class(&base, "FerroUI.Visual").unregistered);

    // The aliases and the handles the crates register.
    let pages = linked.model.aliases.iter().find(|alias| alias.path == "::ferroui_controls::page::multi_page::PageList").expect("the alias PageList");
    assert!(pages.target.text.contains("FerroList<") && pages.target.text.ends_with("::Page>>"), "{:?}", pages.target);
    let handles: Vec<(&str, &str)> = linked.model.handles.iter().map(|handle| (handle.handle.text.as_str(), handle.type_.text.as_str())).collect();
    assert!(
        handles.iter().any(|(handle, type_)| *handle == "Option<::ferroui_controls::assigned_binding::AssignedBinding>" && type_.starts_with("dyn ::ferroui_base::") && type_.ends_with("::BindingBase")),
        "{handles:?}"
    );
    assert_eq!(linked.model.types.len(), controls.model.types.len());

    // The type texts are canonical: a type of the base crate is named by its declaring module.
    let decorator = class(&linked, "FerroUI.Controls.Decorator");
    let child = decorator.registered("Child").unwrap_or_else(|| panic!("Decorator.Child: {:?}", decorator.registered));
    assert_eq!(child.value_type, ty(&format!("Option<{ref_path}<::ferroui_controls::control::Control>>")));
    let background = class(&linked, "FerroUI.Controls.Border").registered("Background").expect("Border.Background");
    assert_eq!(set.canonical(&background.value_type.text), background.value_type.text);
    // The model with the dependencies is its file, too.
    let read = crate::model::AssemblyModel::parse(&linked.model.to_json()).expect("the linked model is read back");
    assert!(read == linked.model, "the linked model read back from its text differs from the scanned one");
}

fn registration_directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).with_file_name("FerroUI.Build.Scan").join("tests").join("fixtures").join("registration")
}

/// The scan of the third fixture crate: what a crate registers next to its declarations.
fn registration() -> Scan {
    scan_crate(&ScanOptions::new("registration", registration_directory().join("lib.rs")))
}

const PANEL: &str = "::registration::panel::Panel";
const PANEL_LIST: &str = "::ferroui_base::collections::FerroList<::ferroui_base::Ref<::registration::panel::Panel>>";

/// The accessors of registered properties that are not written in the block of their
/// owner: the ones a macro of the crate writes among the members of the type, and the one
/// that is a function of another type. A property is stated with the owner its
/// registration names.
#[test]
fn accessors_outside_the_block_of_their_owner_are_read() {
    let scan = registration();
    let panel = the_type(&scan, "Panel");
    let accessors: Vec<(&str, Option<&str>, RegisteredKind, Option<&str>)> = panel
        .registered
        .iter()
        .map(|registered| (registered.accessor.as_str(), registered.name.as_deref(), registered.kind, registered.function_of.as_deref()))
        .collect();
    assert_eq!(
        accessors,
        [
            ("items_property", Some("Items"), RegisteredKind::Styled, None),
            ("mark_property", Some("Mark"), RegisteredKind::Styled, Some("::registration::panel::Marker")),
            ("spacing_property", Some("Spacing"), RegisteredKind::Direct, None),
            ("gap_property", Some("Gap"), RegisteredKind::Direct, None),
        ],
        "\n{}",
        listing(&scan.diagnostics)
    );
    // The accessor a macro writes is read as the one written out: its type, the name the
    // invocation states, the owner and the missing setter of the registration.
    let spacing = panel.registered("Spacing").expect("Panel.Spacing");
    assert_eq!(
        spacing,
        &RegisteredModel { read_only: true, ..registered("Spacing", RegisteredKind::Direct, ty("f64"), PANEL, "spacing_property") },
        "\n{}",
        listing(&scan.diagnostics)
    );
    assert_eq!(scan.statistics.expanded.get("cell_property"), Some(&2));
    assert_eq!(panel.registered("Items").map(|items| &items.value_type), Some(&ty("::registration::panel::Panels")));

    // The owner added through the function of the other type has the name of the property,
    // and the declaration has the owner.
    let mark = panel.registered("Mark").expect("Panel.Mark");
    assert_eq!(mark.added_owners, ["::registration::panel::Slot"]);
    let slot = the_type(&scan, "Slot");
    let added = &slot.registered[0];
    assert_eq!((added.name.as_deref(), added.registration, &added.function_of), (Some("Mark"), RegistrationModel::AddedOwner, &None));
    assert_eq!(added.source, Some(path("Marker::mark_property", "::registration::panel::Marker::mark_property")));
    assert_eq!(added.owner, Some(ty("::registration::panel::Slot")));
    // The property the block of `Slot` registers with `Panel` as its owner.
    let saved = &slot.registered[1];
    assert_eq!((saved.name.as_deref(), saved.visibility.as_str(), &saved.owner, &saved.host), (Some("Saved"), "", &Some(ty(PANEL)), &Some(ty(PANEL))));
    assert_eq!(scan.statistics.registered_without_name, 0);

    let set = ModelSet::new(vec![scan.model.clone()]);
    let found = |path: &str| set.find_accessor(path).map(|(type_, registered)| (type_.name.clone(), registered.name.clone()));
    assert_eq!(found("::registration::Marker::mark_property"), Some(("Panel".to_string(), Some("Mark".to_string()))));
    assert_eq!(found("::registration::Panel::mark_property"), None);
    assert_eq!(found("::registration::Slot::mark_property"), Some(("Slot".to_string(), Some("Mark".to_string()))));
}

/// What the registration of the crate states: the classes its list leaves out are marked,
/// the handles and the casts of its registration function are in the model (with the names
/// the function imports for itself resolved), and so are the aliases whose type is one
/// the scanner resolves.
#[test]
fn registration_of_a_crate_is_read() {
    let scan = registration();
    let unregistered: Vec<&str> = scan.model.types.iter().filter(|type_| type_.unregistered).map(|type_| type_.name.as_str()).collect();
    assert_eq!(unregistered, ["Hidden"], "\n{}", listing(&scan.diagnostics));
    assert_eq!(scan.statistics.unregistered, 1);
    // A crate without a list of registered classes has no marked type.
    assert!(fixture().model.types.iter().all(|type_| !type_.unregistered));

    let aliases: Vec<(&str, &RustType)> = scan.model.aliases.iter().map(|alias| (alias.path.as_str(), &alias.target)).collect();
    assert_eq!(
        aliases,
        [("::registration::panel::PanelList", &ty(PANEL_LIST)), ("::registration::panel::Panels", &ty("Option<::registration::panel::PanelList>"))],
        "\n{}",
        listing(&scan.diagnostics)
    );
    let handles: Vec<(&RustType, &RustType)> = scan.model.handles.iter().map(|handle| (&handle.handle, &handle.type_)).collect();
    assert_eq!(
        handles,
        [
            (&ty("::registration::panel::Wrapper"), &ty("dyn ::registration::panel::IPanel")),
            (&ty("Option<::registration::panel::Wrapper>"), &ty("::registration::panel::PanelCollection")),
        ]
    );
    // The casts: the one of the registration function, then the ones of the table a macro
    // of a function writes (the expansions of `assignable!`, which `for_each_cast!` invokes).
    let shared = |text: &str| ty(&format!("::std::rc::Rc<{text}>"));
    let casts: Vec<(&RustType, &RustType)> = scan.model.casts.iter().map(|cast| (&cast.from, &cast.to)).collect();
    assert_eq!(
        casts,
        [
            (&ty("::registration::panel::PanelCollection"), &ty("::registration::panel::PanelList")),
            (&shared("::registration::panel::PanelCollection"), &shared("::registration::panel::Wrapper")),
            (&shared("::registration::panel::Wrapper"), &shared("dyn ::registration::panel::IPanel")),
        ],
        "\n{}",
        listing(&scan.diagnostics)
    );
    // The other registrations with the untyped value conversions: called, handed to a
    // function that calls them, in a closure that is the value of a constant, and in the
    // expansions of the table and of the macro whose rule repeats, once for each round.
    let value_types: Vec<(&str, Vec<&RustType>)> =
        scan.model.value_types.iter().map(|registration| (registration.registration.as_str(), registration.types.iter().collect())).collect();
    assert_eq!(
        value_types,
        [
            ("nullable", vec![&ty("::registration::panel::PanelCollection")]),
            ("reference", vec![&ty("::registration::panel::Wrapper")]),
            ("element_ref", vec![&ty(PANEL)]),
            ("upcast", vec![&ty("::registration::panel::Deep"), &ty(PANEL)]),
            ("reference", vec![&ty("::registration::panel::PanelCollection")]),
            ("reference", vec![&ty("::registration::panel::Deep")]),
            ("nullable", vec![&shared("::registration::panel::Wrapper")]),
            ("nullable", vec![&shared("dyn ::registration::panel::IPanel")]),
        ],
        "\n{}",
        listing(&scan.diagnostics)
    );
    // Not read: the cast whose target is left to inference, and what the macro whose rule
    // takes a block registers. The calls in the definitions of `assignable!` and of
    // `references!`, whose rule repeats, are read through their expansions.
    assert_eq!(scan.model.unread_value_types, [("cast".to_string(), 1), ("reference".to_string(), 1)]);
    // The model keeps them in its file.
    let read_back = crate::model::AssemblyModel::parse(&scan.model.to_json()).expect("the model is read back");
    assert_eq!(read_back.value_types, scan.model.value_types);
    assert_eq!(read_back.unread_value_types, scan.model.unread_value_types);

    // A trailing comma of a list of type arguments is not part of the text of a type.
    let collection = the_type(&scan, "PanelCollection");
    let pairs = collection.properties.iter().find(|property| property.name == "Pairs").expect("PanelCollection.Pairs");
    assert_eq!(pairs.value_type, ty(&format!("::std::collections::HashMap<String, ::ferroui_base::Ref<{PANEL}>>")));

    let directory = registration_directory();
    let reported: Vec<(String, usize, Severity, &str)> =
        scan.diagnostics.iter().map(|diagnostic| (relative(&diagnostic.file, &directory), diagnostic.line, diagnostic.severity, diagnostic.code)).collect();
    let expected: Vec<(String, usize, Severity, &str)> = [
        ("register_types.rs", 18, Severity::Warning, codes::OWNER),
        ("register_types.rs", 18, Severity::Note, codes::UNREGISTERED),
        ("register_types.rs", 30, Severity::Warning, codes::FORM),
    ]
    .into_iter()
    .map(|(file, line, severity, code)| (file.to_string(), line, severity, code))
    .collect();
    assert_eq!(reported, expected, "\n{}", listing(&scan.diagnostics));
    let message = |code: &str| scan.diagnostics.iter().find(|diagnostic| diagnostic.code == code).map(|diagnostic| diagnostic.message.as_str()).unwrap_or_default();
    assert!(message(codes::OWNER).starts_with("the list of registered classes names `crate::Missing`"), "{}", message(codes::OWNER));
    assert!(message(codes::UNREGISTERED).starts_with("`::registration::panel::Hidden` is declared and is not in the list of registered classes"), "{}", message(codes::UNREGISTERED));
    assert!(message(codes::FORM).starts_with("`MarkupType::register_handle` is called with a type that is not"), "{}", message(codes::FORM));

    // The model with what the crate registers is its file.
    let read = AssemblyModel::parse(&scan.model.to_json()).expect("the model is read back");
    assert!(read == scan.model, "the model read back from its text differs from the scanned one");
}

/// The values of the members of enumerations and of sets of flags that are constant
/// expressions: over literals, the other members, and the associated constants of the
/// type. A member that is not such an expression (and a variant after one) has no value.
#[test]
fn members_that_are_constant_expressions_have_their_values() {
    let scan = registration();
    let values = |name: &str| -> Vec<(String, Option<i64>)> { the_type(&scan, name).enum_members.iter().map(|member| (member.name.clone(), member.value)).collect() };
    let expected = |members: &[(&str, Option<i64>)]| -> Vec<(String, Option<i64>)> { members.iter().map(|(name, value)| (name.to_string(), *value)).collect() };
    assert_eq!(
        values("Modes"),
        expected(&[
            ("Disabled", Some(0)),
            ("Keyboard", Some(1)),
            ("Remote", Some(4)),
            ("Enabled", Some(7)),
            ("Every", Some(7)),
            ("Pointing", Some(6)),
            ("Computed", None),
        ]),
        "\n{}",
        listing(&scan.diagnostics)
    );
    assert_eq!(
        values("Key"),
        expected(&[
            ("None", Some(0)),
            ("Return", Some(6)),
            ("Enter", Some(6)),
            ("Pause", Some(7)),
            ("Shifted", Some(16)),
            ("Both", Some(22)),
            ("Last", Some(2147483647)),
            ("Odd", None),
            ("After", None),
            ("Other", None),
        ]),
        "\n{}",
        listing(&scan.diagnostics)
    );
    assert_eq!((scan.statistics.enum_members, scan.statistics.enum_members_without_value), (17, 4));
}
