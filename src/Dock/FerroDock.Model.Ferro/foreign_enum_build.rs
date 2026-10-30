//! Build-only bridge for enums owned by the UI-independent model crate.
// The standard shared scanner cannot read a handwritten MarkupTyped descriptor
// or apply its enum macro to a foreign enum (Rust's orphan rule). Generate the
// runtime descriptor and augment the shared TypeModel from the same original
// Rust value declaration. This bridge reads enum literals or transparent Int32
// constants; downstream compilation still uses FerroUI's shared metadata model.
use ferroui_build_scan::model::{AssemblyModel, EnumMemberModel, RustType, TypeKind, TypeModel};
use std::fmt::Write;
use std::path::Path;

pub(crate) fn dock_operation(out: &Path, model: &mut AssemblyModel) {
    let mut code =
        String::from("// Generated from original model enum declarations. Do not edit.\n");
    append_enum(
        ferrodock_model::core::DockCapabilityValueSource::DECLARATION_SOURCE,
        "DockCapabilityValueSource",
        "core",
        "FerroDock.Model.Core",
        model,
        &mut code,
    );
    append_enum(
        ferrodock_model::core::DockTrackingChangeReason::DECLARATION_SOURCE,
        "DockTrackingChangeReason",
        "core",
        "FerroDock.Model.Core",
        model,
        &mut code,
    );
    append_enum(
        ferrodock_model::core::TrackingMode::DECLARATION_SOURCE,
        "TrackingMode",
        "core",
        "FerroDock.Model.Core",
        model,
        &mut code,
    );
    append_enum(
        ferrodock_model::core::PinnedDockDisplayMode::DECLARATION_SOURCE,
        "PinnedDockDisplayMode",
        "core",
        "FerroDock.Model.Core",
        model,
        &mut code,
    );
    append_enum(
        ferrodock_model::core::DockWindowState::DECLARATION_SOURCE,
        "DockWindowState",
        "core",
        "FerroDock.Model.Core",
        model,
        &mut code,
    );
    append_enum(
        ferrodock_model::core::MdiWindowState::DECLARATION_SOURCE,
        "MdiWindowState",
        "core",
        "FerroDock.Model.Core",
        model,
        &mut code,
    );
    append_enum(
        ferrodock_model::core::DockOperation::DECLARATION_SOURCE,
        "DockOperation",
        "core",
        "FerroDock.Model.Core",
        model,
        &mut code,
    );
    append_enum(
        ferrodock_model::core::DockFloatingWindowHostMode::DECLARATION_SOURCE,
        "DockFloatingWindowHostMode",
        "core",
        "FerroDock.Model.Core",
        model,
        &mut code,
    );
    append_enum(
        ferrodock_model::core::DockWindowOwnerMode::DECLARATION_SOURCE,
        "DockWindowOwnerMode",
        "core",
        "FerroDock.Model.Core",
        model,
        &mut code,
    );
    append_enum(
        ferrodock_model::command_bars::DockCommandBarKind::DECLARATION_SOURCE,
        "DockCommandBarKind",
        "command_bars",
        "FerroDock.Model.CommandBars",
        model,
        &mut code,
    );
    append_enum(
        ferrodock_model::command_bars::DockCommandBarMergeMode::DECLARATION_SOURCE,
        "DockCommandBarMergeMode",
        "command_bars",
        "FerroDock.Model.CommandBars",
        model,
        &mut code,
    );
    append_enum(
        ferrodock_model::core::DocumentLayoutMode::DECLARATION_SOURCE,
        "DocumentLayoutMode",
        "core",
        "FerroDock.Model.Core",
        model,
        &mut code,
    );
    append_enum(
        ferrodock_model::core::DocumentTabLayout::DECLARATION_SOURCE,
        "DocumentTabLayout",
        "core",
        "FerroDock.Model.Core",
        model,
        &mut code,
    );
    append_enum(
        ferrodock_model::core::DocumentCloseButtonShowMode::DECLARATION_SOURCE,
        "DocumentCloseButtonShowMode",
        "core",
        "FerroDock.Model.Core",
        model,
        &mut code,
    );
    append_enum(
        ferrodock_model::core::Alignment::DECLARATION_SOURCE,
        "Alignment",
        "core",
        "FerroDock.Model.Core",
        model,
        &mut code,
    );
    append_enum(
        ferrodock_model::core::GripMode::DECLARATION_SOURCE,
        "GripMode",
        "core",
        "FerroDock.Model.Core",
        model,
        &mut code,
    );
    append_enum(
        ferrodock_model::core::DockCapability::DECLARATION_SOURCE,
        "DockCapability",
        "core",
        "FerroDock.Model.Core",
        model,
        &mut code,
    );
    append_enum(
        ferrodock_model::core::Orientation::DECLARATION_SOURCE,
        "Orientation",
        "core",
        "FerroDock.Model.Core",
        model,
        &mut code,
    );
    append_enum(
        ferrodock_model::core::DockMode::DECLARATION_SOURCE,
        "DockMode",
        "core",
        "FerroDock.Model.Core",
        model,
        &mut code,
    );
    append_enum(
        ferrodock_model::core::GridResizeDirection::DECLARATION_SOURCE,
        "GridResizeDirection",
        "core",
        "FerroDock.Model.Core",
        model,
        &mut code,
    );
    append_enum(
        ferrodock_model::core::SplitViewDisplayMode::DECLARATION_SOURCE,
        "SplitViewDisplayMode",
        "core",
        "FerroDock.Model.Core",
        model,
        &mut code,
    );
    append_enum(
        ferrodock_model::core::SplitViewPanePlacement::DECLARATION_SOURCE,
        "SplitViewPanePlacement",
        "core",
        "FerroDock.Model.Core",
        model,
        &mut code,
    );
    crate::foreign_flag_build::docking_window_state(model, &mut code);
    ferroui_build_scan::export::write_if_changed(&out.join("foreign_enums.rs"), &code)
        .expect("write original enum metadata");
}
fn integer_literal(value: &syn::Expr) -> i32 {
    let number = match value {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Int(value),
            ..
        }) => value.base10_parse::<i64>().expect("enum integer literal"),
        syn::Expr::Unary(syn::ExprUnary {
            op: syn::UnOp::Neg(_),
            expr,
            ..
        }) => match &**expr {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Int(value),
                ..
            }) => -value.base10_parse::<i64>().expect("negative enum literal"),
            _ => panic!("negative enum literal expected"),
        },
        _ => panic!("Enum metadata requires explicit integer literals"),
    };
    i32::try_from(number).expect("enum signed Int32 value")
}
fn enum_declaration(file: &syn::File, name: &str) -> (Vec<(String, i32)>, bool) {
    if let Some(declaration) = file.items.iter().find_map(|item| match item {
        syn::Item::Enum(item) if item.ident == name => Some(item),
        _ => None,
    }) {
        let members = declaration
            .variants
            .iter()
            .map(|variant| {
                assert!(
                    matches!(variant.fields, syn::Fields::Unit),
                    "source enum unit members"
                );
                let (_, value) = variant
                    .discriminant
                    .as_ref()
                    .expect("explicit enum discriminant");
                (variant.ident.to_string(), integer_literal(value))
            })
            .collect();
        return (members, false);
    }
    let declaration = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Struct(item) if item.ident == name => Some(item),
            _ => None,
        })
        .expect("original enum or open numeric declaration");
    assert!(
        declaration.attrs.iter().any(|a| a.path().is_ident("repr")
            && a.parse_args::<syn::Ident>()
                .is_ok_and(|i| i == "transparent")),
        "open enums retain transparent storage"
    );
    let syn::Fields::Unnamed(fields) = &declaration.fields else {
        panic!("open enum signed storage")
    };
    assert_eq!(
        fields.unnamed.len(),
        1,
        "open enum has one underlying integer"
    );
    assert!(
        matches!(&fields.unnamed[0].ty,syn::Type::Path(p) if p.path.is_ident("i32")),
        "open enum storage is Int32"
    );
    let mut members = Vec::new();
    for item in &file.items {
        let syn::Item::Impl(item) = item else {
            continue;
        };
        if item.trait_.is_some()
            || !matches!(&*item.self_ty,syn::Type::Path(p) if p.path.is_ident(name))
        {
            continue;
        }
        for item in &item.items {
            let syn::ImplItem::Const(item) = item else {
                continue;
            };
            if !matches!(item.vis, syn::Visibility::Public(_))
                || !matches!(&item.ty,syn::Type::Path(p) if p.path.is_ident("Self"))
            {
                continue;
            }
            let syn::Expr::Call(call) = &item.expr else {
                panic!("open enum literal member")
            };
            assert!(
                matches!(&*call.func,syn::Expr::Path(p) if p.path.is_ident("Self")),
                "original enum member constructor"
            );
            assert_eq!(call.args.len(), 1, "enum literal member has one value");
            members.push((item.ident.to_string(), integer_literal(&call.args[0])));
        }
    }
    (members, true)
}

fn append_enum(
    source: &str,
    name: &str,
    module: &str,
    namespace: &str,
    model: &mut AssemblyModel,
    code: &mut String,
) {
    // The build dependency carries its own declaration text, so this also works
    // for independently packaged libraries without a sibling source checkout.
    let file = syn::parse_file(source).expect("parse original model enum declaration");
    let (members, open_numeric) = enum_declaration(&file, name);
    let original = format!("::ferrodock_model::{module}::{name}");
    let carrier = format!("::ferrodock_model_ferro::{name}Metadata");
    // Nullable ownership is derived from rust_path by the shared compiler.
    // The declaration must name the original enum; public_path names only its
    // native descriptor carrier and typed constant access.
    let mut type_ = TypeModel::new(
        name,
        TypeKind::Enum,
        RustType::resolved(&original),
        "ferrodock_model_ferro",
    );
    type_.namespace = namespace.into();
    type_.explicit_namespace = Some(namespace.into());
    type_.public_path = Some(carrier);
    type_.handles = vec![RustType::resolved(&original)];
    if open_numeric {
        type_.enum_i32_factory = Some("__markup_enum_i32".into());
    }
    for (member, value) in &members {
        type_.enum_members.push(EnumMemberModel {
            name: member.clone(),
            rust_variant: Some(member.clone()),
            rust_value: None,
            value: Some(i64::from(*value)),
        });
    }
    assert!(!members.is_empty(), "source enum has members");

    code.push_str(&format!("/// Model integration metadata descriptor; values remain the original domain enum.\n#[doc(hidden)]\npub struct {name}Metadata;\n#[allow(non_upper_case_globals)]\nimpl {name}Metadata {{\n"));
    for (name, _) in &members {
        writeln!(
            code,
            "/// Original {name} enum value.\npub const {name}: {original} = {original}::{name};",
        )
        .unwrap();
    }
    if open_numeric {
        writeln!(code,"/// Typed source numeric cast used by the shared XAML emitter.\n#[doc(hidden)]\npub const fn __markup_enum_i32(value:i32)->{original} {{ {original}::from_raw(value) }}").unwrap();
    }
    code.push_str("}\n");
    for (name, value) in &members {
        let number = if open_numeric {
            format!("{original}::{name}.to_raw()")
        } else {
            format!("{original}::{name} as i32")
        };
        writeln!(code, "const _: () = assert!({number} == {value});").unwrap();
    }
    writeln!(code,"impl ::ferroui_base::metadata::MarkupTyped for {name}Metadata {{\nconst MARKUP: &'static ::ferroui_base::metadata::MarkupType = {{\nstatic MARKUP: ::ferroui_base::metadata::MarkupType = {{\nlet mut m = ::ferroui_base::metadata::MarkupType::new(\"{name}\", ::ferroui_base::metadata::MarkupTypeKind::Enum, module_path!());\nm.explicit_namespace=Some(\"{namespace}\");\nm.handles=&[::ferroui_base::data::core::ValueType::of::<{original}>];\nm.nullable=Some(::ferroui_base::data::core::ValueType::of::<Option<{original}>>);\nm.enum_members=&[").unwrap();
    for (name, value) in &members {
        writeln!(code,"::ferroui_base::metadata::MarkupEnumMember {{name:\"{name}\",value:{value},get:||::std::rc::Rc::new({original}::{name}),rust_variant:Some(\"{name}\")}},").unwrap();
    }
    code.push_str("];\n");
    if open_numeric {
        code.push_str("::ferroui_base::__ferro_compiler_metadata! {m.enum_i32_factory=Some(\"__markup_enum_i32\");}\n");
    }
    let from_value = if open_numeric {
        format!("i32::try_from(value).ok().map({original}::from_raw)")
    } else {
        format!("i32::try_from(value).ok().and_then(|value|{original}::try_from(value).ok())")
    };
    let to_value = if open_numeric {
        "i64::from(v.to_raw())"
    } else {
        "*v as i64"
    };
    writeln!(code, "m.enum_from_value=Some(|value|{from_value}.map(|v|::std::rc::Rc::new(v) as ::ferroui_base::BoxedValue));\nm.enum_to_value=Some(|value|value.downcast_ref::<{original}>().map(|v|{to_value}));\nm.parse=Some(|args|crate::enum_text::parse(<{name}Metadata as ::ferroui_base::metadata::MarkupTyped>::MARKUP,args));\nm\n}};\n&MARKUP\n}};\n}}").unwrap();
    model.types.push(type_);
}
