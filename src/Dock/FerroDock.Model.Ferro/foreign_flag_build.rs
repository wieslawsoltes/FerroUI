//! Exports the original domain flags through the shared compiler's metadata model.
use ferroui_build_scan::model::{AssemblyModel, EnumMemberModel, RustType, TypeKind, TypeModel};
use std::fmt::Write;
use syn::parse::{Parse, ParseStream};
use syn::{Attribute, Ident, Token};

struct Flags {
    name: Ident,
    members: Vec<(Ident, i32)>,
}
impl Parse for Flags {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let _ = input.call(Attribute::parse_outer)?;
        let _: syn::Visibility = input.parse()?;
        let _: Token![struct] = input.parse()?;
        let name = input.parse()?;
        let _: Token![:] = input.parse()?;
        let storage: Ident = input.parse()?;
        assert_eq!(storage, "i32", "source flags retain their signed storage");
        let body;
        syn::braced!(body in input);
        let mut members = Vec::new();
        while !body.is_empty() {
            let _ = body.call(Attribute::parse_outer)?;
            let _: Token![const] = body.parse()?;
            let name = body.parse()?;
            let _: Token![=] = body.parse()?;
            let value: syn::LitInt = body.parse()?;
            let _: Token![;] = body.parse()?;
            members.push((name, value.base10_parse()?));
        }
        Ok(Self { name, members })
    }
}

pub(crate) fn docking_window_state(model: &mut AssemblyModel, code: &mut String) {
    let file = syn::parse_file(ferrodock_model::core::DockingWindowState::DECLARATION_SOURCE)
        .expect("original flags declaration");
    let declaration = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Macro(item) if item.mac.path.is_ident("bitflags") => {
                Some(syn::parse2::<Flags>(item.mac.tokens.clone()).expect("source flags constants"))
            }
            _ => None,
        })
        .expect("original bitflags declaration");
    assert_eq!(declaration.name, "DockingWindowState");
    let original = "::ferrodock_model::core::DockingWindowState";
    let mut type_ = TypeModel::new(
        "DockingWindowState",
        TypeKind::Enum,
        RustType::resolved(original),
        "ferrodock_model_ferro",
    );
    type_.namespace = "FerroDock.Model.Core".into();
    type_.explicit_namespace = Some(type_.namespace.clone());
    type_.public_path = Some("::ferrodock_model_ferro::DockingWindowStateMetadata".into());
    type_.handles = vec![RustType::resolved(original)];
    type_.is_flags = true;
    code.push_str("/// Model integration descriptor; all values are the original domain flags.\n#[doc(hidden)]\npub struct DockingWindowStateMetadata;\n#[allow(non_upper_case_globals)]\nimpl DockingWindowStateMetadata {\n");
    for (constant, value) in &declaration.members {
        let constant = constant.to_string();
        // Source public names are PascalCase; the domain uses Rust flag conventions.
        let mut name = String::new();
        for word in constant.split('_') {
            let mut letters = word.chars();
            name.extend(letters.next());
            name.extend(letters.flat_map(char::to_lowercase));
        }
        type_.enum_members.push(EnumMemberModel {
            name: name.clone(),
            rust_variant: None,
            rust_value: Some(format!("{original}::{constant}")),
            value: Some(i64::from(*value)),
        });
        writeln!(
            code,
            "/// Original {name} constant.\npub const {name}: {original} = {original}::{constant};"
        )
        .unwrap();
    }
    writeln!(code,"/// Typed entry point used solely by the shared flags emitter.\n#[doc(hidden)]\npub fn __markup_flags(value: i64) -> {original} {{ {original}::from_bits_retain(value as i32) }}\n}}").unwrap();
    for (constant, value) in &declaration.members {
        writeln!(
            code,
            "const _: () = assert!({original}::{constant}.bits() == {value});"
        )
        .unwrap();
    }
    code.push_str("impl ::ferroui_base::metadata::MarkupTyped for DockingWindowStateMetadata {\nconst MARKUP: &'static ::ferroui_base::metadata::MarkupType = {\nstatic MARKUP: ::ferroui_base::metadata::MarkupType = {\nlet mut m = ::ferroui_base::metadata::MarkupType::new(\"DockingWindowState\", ::ferroui_base::metadata::MarkupTypeKind::Enum, module_path!());\nm.explicit_namespace = Some(\"FerroDock.Model.Core\");\nm.is_flags = true;\nm.handles = &[::ferroui_base::data::core::ValueType::of::<::ferrodock_model::core::DockingWindowState>];\nm.nullable = Some(::ferroui_base::data::core::ValueType::of::<Option<::ferrodock_model::core::DockingWindowState>>);\nm.enum_members = &[\n");
    for member in &type_.enum_members {
        writeln!(code,"::ferroui_base::metadata::MarkupEnumMember {{ name: \"{}\", value: {}, get: || ::std::rc::Rc::new(DockingWindowStateMetadata::{}), rust_variant: None }},", member.name,member.value.unwrap(),member.name).unwrap();
    }
    writeln!(code,"];\nm.enum_from_value = Some(|value| i32::try_from(value).ok().map(|bits| ::std::rc::Rc::new({original}::from_bits_retain(bits)) as ::ferroui_base::BoxedValue));\nm.enum_to_value = Some(|value| value.downcast_ref::<{original}>().map(|value| i64::from(value.bits())));\nm\n}}; &MARKUP\n}};\n}}").unwrap();
    model.types.push(type_);
}
