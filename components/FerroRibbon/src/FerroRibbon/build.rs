mod enum_declarations;
use ferroui_build_scan::export::write_if_changed;
use ferroui_build_scan::model::{AssemblyModel, EnumMemberModel, RustType, TypeKind, TypeModel};
use std::path::PathBuf;
fn main() {
    println!("cargo::rerun-if-env-changed=FERRORIBBON_BUILD_FINGERPRINT");
    if let Ok(value) = std::env::var("FERRORIBBON_BUILD_FINGERPRINT") {
        assert!(
            value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()),
            "invalid package source fingerprint"
        );
        println!("cargo::rustc-env=FERRORIBBON_BUILD_FINGERPRINT={value}");
    }
    let manifest = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").expect("manifest"));
    let out = PathBuf::from(std::env::var_os("OUT_DIR").expect("output"));
    let generic =
        std::fs::read_to_string(manifest.join("Themes/Generic.xaml")).expect("generic markup");
    let source_items = std::fs::read_to_string(manifest.join("Themes/Source/Items.xaml"))
        .expect("source_items markup");
    let source_ribbon = std::fs::read_to_string(manifest.join("Themes/Source/Ribbon.xaml"))
        .expect("source_ribbon markup");
    let source_inputs = std::fs::read_to_string(manifest.join("Themes/Source/Inputs.xaml"))
        .expect("source_inputs markup");
    let source_chrome = std::fs::read_to_string(manifest.join("Themes/Source/Chrome.xaml"))
        .expect("source_chrome markup");
    let shared =
        std::fs::read_to_string(manifest.join("Themes/Shared.xaml")).expect("shared markup");
    let screen_tips =
        std::fs::read_to_string(manifest.join("Themes/ScreenTips.xaml")).expect("ScreenTip markup");
    let simple_items = std::fs::read_to_string(manifest.join("Themes/SimpleItems.xaml"))
        .expect("simple items markup");
    let buttons =
        std::fs::read_to_string(manifest.join("Themes/Buttons.xaml")).expect("button markup");
    let split = std::fs::read_to_string(manifest.join("Themes/Split.xaml")).expect("split markup");
    let gallery =
        std::fs::read_to_string(manifest.join("Themes/Gallery.xaml")).expect("gallery markup");
    let toolbars =
        std::fs::read_to_string(manifest.join("Themes/Toolbars.xaml")).expect("toolbar markup");
    let segmented =
        std::fs::read_to_string(manifest.join("Themes/Segmented.xaml")).expect("segmented markup");
    let color_dialog = std::fs::read_to_string(manifest.join("Themes/ColorDialog.xaml"))
        .expect("color dialog markup");
    let color_picker = std::fs::read_to_string(manifest.join("Themes/ColorPicker.xaml"))
        .expect("color picker markup");
    let color_palette = std::fs::read_to_string(manifest.join("Themes/ColorPalette.xaml"))
        .expect("color palette markup");
    let grid_picker = std::fs::read_to_string(manifest.join("Themes/GridPicker.xaml"))
        .expect("grid picker markup");
    let slider =
        std::fs::read_to_string(manifest.join("Themes/Slider.xaml")).expect("slider markup");
    let spinner =
        std::fs::read_to_string(manifest.join("Themes/Spinner.xaml")).expect("spinner markup");
    let combo = std::fs::read_to_string(manifest.join("Themes/Combo.xaml")).expect("combo markup");
    let text_input =
        std::fs::read_to_string(manifest.join("Themes/TextInput.xaml")).expect("text input markup");
    let checked =
        std::fs::read_to_string(manifest.join("Themes/Checked.xaml")).expect("checked markup");
    let groups =
        std::fs::read_to_string(manifest.join("Themes/Groups.xaml")).expect("group markup");
    let ribbon =
        std::fs::read_to_string(manifest.join("Themes/Ribbon.xaml")).expect("ribbon markup");
    let title = std::fs::read_to_string(manifest.join("Themes/Title.xaml")).expect("title markup");
    let status =
        std::fs::read_to_string(manifest.join("Themes/Status.xaml")).expect("status markup");
    let zoom = std::fs::read_to_string(manifest.join("Themes/Zoom.xaml")).expect("zoom markup");
    let search =
        std::fs::read_to_string(manifest.join("Themes/Search.xaml")).expect("search markup");
    let backstage =
        std::fs::read_to_string(manifest.join("Themes/Backstage.xaml")).expect("backstage markup");
    let result = ferroui_build::Build::from_env()
        .type_system(ferroui_build::TypeSystem::Model)
        .input("Themes/Generic.xaml")
        .input("Themes/Source/Items.xaml")
        .input("Themes/Source/Ribbon.xaml")
        .input("Themes/Source/Inputs.xaml")
        .input("Themes/Source/Chrome.xaml")
        .input("Themes/Title.xaml")
        .input("Themes/Status.xaml")
        .input("Themes/Zoom.xaml")
        .input("Themes/Search.xaml")
        .input("Themes/Backstage.xaml")
        .input("Themes/Shared.xaml")
        .input("Themes/ScreenTips.xaml")
        .input("Themes/SimpleItems.xaml")
        .input("Themes/Buttons.xaml")
        .input("Themes/Checked.xaml")
        .input("Themes/TextInput.xaml")
        .input("Themes/Spinner.xaml")
        .input("Themes/Slider.xaml")
        .input("Themes/ColorDialog.xaml")
        .input("Themes/ColorPicker.xaml")
        .input("Themes/ColorPalette.xaml")
        .input("Themes/GridPicker.xaml")
        .input("Themes/Gallery.xaml")
        .input("Themes/Segmented.xaml")
        .input("Themes/Toolbars.xaml")
        .input("Themes/Combo.xaml")
        .input("Themes/Split.xaml")
        .input("Themes/Groups.xaml")
        .input("Themes/Ribbon.xaml")
        .compile_group(
            ferroui_build::XamlGroup::new("compiled_shared").documents(&[
                ("Themes/Generic.xaml", &generic),
                ("Themes/Source/Items.xaml", &source_items),
                ("Themes/Source/Ribbon.xaml", &source_ribbon),
                ("Themes/Source/Inputs.xaml", &source_inputs),
                ("Themes/Source/Chrome.xaml", &source_chrome),
                ("Themes/Title.xaml", &title),
                ("Themes/Status.xaml", &status),
                ("Themes/Zoom.xaml", &zoom),
                ("Themes/Search.xaml", &search),
                ("Themes/Backstage.xaml", &backstage),
                ("Themes/Shared.xaml", &shared),
                ("Themes/ScreenTips.xaml", &screen_tips),
                ("Themes/SimpleItems.xaml", &simple_items),
                ("Themes/Buttons.xaml", &buttons),
                ("Themes/Checked.xaml", &checked),
                ("Themes/TextInput.xaml", &text_input),
                ("Themes/Spinner.xaml", &spinner),
                ("Themes/Slider.xaml", &slider),
                ("Themes/ColorDialog.xaml", &color_dialog),
                ("Themes/ColorPicker.xaml", &color_picker),
                ("Themes/ColorPalette.xaml", &color_palette),
                ("Themes/GridPicker.xaml", &grid_picker),
                ("Themes/Gallery.xaml", &gallery),
                ("Themes/Segmented.xaml", &segmented),
                ("Themes/Toolbars.xaml", &toolbars),
                ("Themes/Combo.xaml", &combo),
                ("Themes/Split.xaml", &split),
                ("Themes/Groups.xaml", &groups),
                ("Themes/Ribbon.xaml", &ribbon),
            ]),
        )
        .execute();
    for line in &result.lines {
        if !line.starts_with("cargo::metadata=xamlmeta=") {
            println!("{line}");
        }
    }
    assert!(
        result.errors.is_empty(),
        "metadata export failed: {:?}",
        result.errors
    );
    println!("cargo::rerun-if-changed=enum_declarations.rs");
    let input = std::fs::read_to_string(out.join("ferroribbon.xamlmeta")).expect("compiled model");
    let mut model = AssemblyModel::parse(&input).expect("standard model");
    for declaration in enum_declarations::ENUMS {
        let enum_declarations::EnumDeclaration {
            name,
            namespace,
            rust_type: original,
            members,
        } = *declaration;
        let original = format!("::{original}");
        let mut descriptor = TypeModel::new(
            name,
            TypeKind::Enum,
            RustType::resolved(&original),
            "ferroribbon",
        );
        descriptor.namespace = namespace.into();
        descriptor.explicit_namespace = Some(namespace.into());
        descriptor.public_path = Some(format!("::ferroribbon::metadata::enums::{name}Metadata"));
        descriptor.handles = vec![RustType::resolved(&original)];
        descriptor.enum_i32_factory = Some("__markup_from_i32".into());
        descriptor.enum_members = members
            .iter()
            .map(|&(name, value)| EnumMemberModel {
                name: name.into(),
                rust_variant: Some(name.into()),
                rust_value: None,
                value: Some(i64::from(value)),
            })
            .collect();
        model.types.push(descriptor);
    }
    let path = out.join("ferroribbon.xamlmeta");
    // The source enum is declared alongside the native panel, outside Core/Enums.cs.
    let mut part = TypeModel::new(
        "RibbonItemContentPart",
        TypeKind::Enum,
        RustType::resolved("::ferroribbon::primitives::RibbonItemContentPart"),
        "ferroribbon",
    );
    part.namespace = "FerroRibbon.Controls.Primitives".into();
    part.explicit_namespace = Some(part.namespace.clone());
    part.public_path =
        Some("::ferroribbon::metadata::item_content_part::RibbonItemContentPartMetadata".into());
    part.handles = vec![RustType::resolved(
        "::ferroribbon::primitives::RibbonItemContentPart",
    )];
    part.enum_members = [("All", 0), ("IconOnly", 1), ("LabelAndChevron", 2)]
        .into_iter()
        .map(|(name, value)| EnumMemberModel {
            name: name.into(),
            rust_variant: Some(name.into()),
            rust_value: None,
            value: Some(value),
        })
        .collect();
    part.enum_i32_factory = Some("__markup_from_i32".into());
    model.types.push(part);
    let mut simplified = TypeModel::new(
        "RibbonSimplifiedLabel",
        TypeKind::Enum,
        RustType::resolved("::ferroribbon::controls::RibbonSimplifiedLabel"),
        "ferroribbon",
    );
    simplified.namespace = "FerroRibbon.Controls".into();
    simplified.explicit_namespace = Some(simplified.namespace.clone());
    simplified.public_path =
        Some("::ferroribbon::metadata::simplified_label::RibbonSimplifiedLabelMetadata".into());
    simplified.handles = vec![RustType::resolved(
        "::ferroribbon::controls::RibbonSimplifiedLabel",
    )];
    simplified.enum_members = [("Auto", 0), ("Show", 1), ("Hide", 2)]
        .into_iter()
        .map(|(name, value)| EnumMemberModel {
            name: name.into(),
            rust_variant: Some(name.into()),
            rust_value: None,
            value: Some(value),
        })
        .collect();
    simplified.enum_i32_factory = Some("__markup_from_i32".into());
    model.types.push(simplified);
    let mut dialog_button = TypeModel::new(
        "ContentDialogButton",
        TypeKind::Enum,
        RustType::resolved("::ferroribbon::compat::ContentDialogButton"),
        "ferroribbon",
    );
    dialog_button.namespace = "FerroRibbon.Controls".into();
    dialog_button.explicit_namespace = Some(dialog_button.namespace.clone());
    dialog_button.public_path =
        Some("::ferroribbon::metadata::content_dialog_button::ContentDialogButtonMetadata".into());
    dialog_button.handles = vec![RustType::resolved(
        "::ferroribbon::compat::ContentDialogButton",
    )];
    dialog_button.enum_members = [("None", 0), ("Primary", 1), ("Secondary", 2), ("Close", 3)]
        .into_iter()
        .map(|(name, value)| EnumMemberModel {
            name: name.into(),
            rust_variant: Some(name.into()),
            rust_value: None,
            value: Some(value),
        })
        .collect();
    dialog_button.enum_i32_factory = Some("__markup_from_i32".into());
    model.types.push(dialog_button);
    // WinUI/Uno Orientation has a different integer order from FerroUI's.
    // Publish the explicit compatibility descriptor, including unnamed values.
    let mut orientation = TypeModel::new(
        "Orientation",
        TypeKind::Enum,
        RustType::resolved("::ferroribbon::compat::Orientation"),
        "ferroribbon",
    );
    orientation.namespace = "FerroRibbon.Compatibility".into();
    orientation.explicit_namespace = Some(orientation.namespace.clone());
    orientation.public_path =
        Some("::ferroribbon::metadata::orientation::OrientationMetadata".into());
    orientation.handles = vec![RustType::resolved("::ferroribbon::compat::Orientation")];
    orientation.enum_members = [("Vertical", 0), ("Horizontal", 1)]
        .into_iter()
        .map(|(name, value)| EnumMemberModel {
            name: name.into(),
            rust_variant: Some(name.into()),
            rust_value: None,
            value: Some(value),
        })
        .collect();
    orientation.enum_i32_factory = Some("__markup_from_i32".into());
    model.types.push(orientation);
    let definition = model
        .types
        .iter_mut()
        .find(|t| t.name == "RibbonSizeDefinition" && t.namespace == "FerroRibbon.Layout")
        .expect("size definition descriptor");
    definition.rust_path = RustType::resolved("::ferroribbon_core::layout::RibbonSizeDefinition");
    // The declaration marker emits typed accessors but the struct's nullable
    // owner must be the actual Core value, not that marker (as with Core enums).
    let color = model
        .types
        .iter_mut()
        .find(|t| t.name == "RibbonColor" && t.namespace == "FerroRibbon.Theming")
        .expect("color descriptor");
    color.rust_path = RustType::resolved("::ferroribbon_core::theming::RibbonColor");
    write_if_changed(&path, &model.to_json()).expect("write complete export");
    println!("cargo::metadata=xamlmeta={}", path.display());
}
