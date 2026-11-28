use ferroui_build_scan::{
    export::write_if_changed,
    model::{AssemblyModel, EnumMemberModel, RustType, TypeKind, TypeModel},
};
fn main() {
    let result = ferroui_build::Build::from_env().export_metadata().execute();
    for line in result.lines {
        if !line.starts_with("cargo::metadata=xamlmeta=") {
            println!("{line}");
        }
    }
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    let path = std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap())
        .join("open_enum_values.xamlmeta");
    let mut model = AssemblyModel::parse(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let mut choice = TypeModel::new(
        "Choice",
        TypeKind::Enum,
        RustType::resolved("::open_enum_values::Choice"),
        "open_enum_values",
    );
    choice.namespace = "OpenEnums".into();
    choice.explicit_namespace = Some(choice.namespace.clone());
    choice.public_path = Some("::open_enum_values::ChoiceMetadata".into());
    choice.handles = vec![RustType::resolved("::open_enum_values::Choice")];
    choice.enum_i32_factory = Some("from_i32".into());
    choice.enum_members = [("None", 0), ("Primary", 1)]
        .into_iter()
        .map(|(n, v)| EnumMemberModel {
            name: n.into(),
            rust_variant: Some(n.into()),
            rust_value: None,
            value: Some(v),
        })
        .collect();
    model.types.push(choice);
    let mut bits = TypeModel::new(
        "Bits",
        TypeKind::Enum,
        RustType::resolved("::open_enum_values::Bits"),
        "open_enum_values",
    );
    bits.namespace = "OpenEnums".into();
    bits.explicit_namespace = Some(bits.namespace.clone());
    bits.public_path = Some("::open_enum_values::BitsMetadata".into());
    bits.handles = vec![RustType::resolved("::open_enum_values::Bits")];
    bits.is_flags = true;
    bits.enum_i32_factory = Some("must_not_be_called".into());
    bits.enum_members = [("One", 1), ("Two", 2)]
        .into_iter()
        .map(|(n, v)| EnumMemberModel {
            name: n.into(),
            rust_variant: None,
            rust_value: None,
            value: Some(v),
        })
        .collect();
    model.types.push(bits);
    write_if_changed(&path, &model.to_json()).unwrap();
    println!("cargo::metadata=xamlmeta={}", path.display());
}
