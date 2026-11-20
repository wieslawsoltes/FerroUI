use crate::{apply_rsx, project_rust, xaml_to_rsx};
use ferroui_designer_core::{Document, TextPatch};

fn apply(source: &str, mut changes: Vec<TextPatch>) -> String {
    changes.sort_by_key(|patch| patch.span.start);
    let mut source = source.to_owned();
    for patch in changes.into_iter().rev() {
        source.replace_range(patch.span.range(), &patch.replacement);
    }
    source
}

#[test]
fn selected_rsx_round_trips_unicode_literals_without_touching_surrounding_code() {
    let source = "// 🦀\r\nfn a() { rsx! { Button {} }; }\r\nfn b() { ferroui_view::rsx! { StackPanel { spacing: 12.0, /* keep */ TextBlock { text: r#\"Można 🧪\"# } } }; }\r\n";
    let projection = project_rust(source, source.find("Można").unwrap()).unwrap();
    assert_eq!(projection.blocks.len(), 2);
    assert_eq!(projection.omitted_count, 0);
    assert_eq!(&source[projection.nodes[1].name.range()], "TextBlock");
    assert_eq!(
        &source[projection.nodes[1].properties["Text"].range()],
        "r#\"Można 🧪\"#"
    );
    let xaml = projection.xaml();
    let changes = projection
        .apply_xaml(&xaml.replace("Można 🧪", "Zapisz &amp; wyjdź"))
        .unwrap();
    assert_eq!(
        apply(source, changes),
        source.replace("r#\"Można 🧪\"#", "\"Zapisz & wyjdź\"")
    );
    assert!(projection
        .apply_xaml(
            &xaml
                .replace("TextBlock", "Button")
                .replace("Text=", "Content=")
        )
        .is_err());
}

#[test]
fn dynamic_rust_survives_visual_literal_editing() {
    let source = r#"rsx! { StackPanel { spacing: 8.0,
        Button { "OK", on_click: move |_| save(data.get()) }
        TextBlock { text: title }
        {dynamic(render_rows)}
    } }"#;
    let projection = project_rust(source, 0).unwrap();
    assert!(projection.xaml().contains("Content=\"OK\""));
    assert_eq!(projection.omitted_count, 3);
    assert!(projection.apply_xaml(&projection.xaml()).is_err());
    let patch = projection.set_property(1, "Content", "Save").unwrap();
    assert_eq!(
        apply(source, vec![patch]),
        source.replace("\"OK\"", "\"Save\"")
    );
    assert!(projection
        .set_property(2, "Text", "Unsafe replacement")
        .is_err());
}

#[test]
fn xaml_edits_preserve_comments_lexical_names_and_inline_content() {
    let xaml = "<Grid xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'>\n  <!-- keep -->\n  <Button x:Name='confirm' Grid.Row='1'>Open</Button>\n</Grid>";
    let document = Document::new(xaml).unwrap();
    let rsx = xaml_to_rsx(&document).unwrap();
    assert!(rsx.contains("Grid.row: 1"));
    assert!(rsx.contains("content: \"Open\""));
    assert!(apply_rsx(&document, &rsx).unwrap().is_empty());
    let draft = rsx
        .replace("Open", "Save & close")
        .replace("confirm", "save")
        .replace("row: 1", "row: 2");
    let edits = apply_rsx(&document, &draft).unwrap();
    assert_eq!(
        apply(xaml, edits),
        xaml.replace("Open", "Save &amp; close")
            .replace("confirm", "save")
            .replace("Row='1'", "Row='2'")
    );
}

#[test]
fn structural_conversion_replaces_only_the_selected_view() {
    let source = "pub fn view() -> Element {\n    rsx! {\n        StackPanel { /* retired elsewhere */ }\n    }\n}\n";
    let source = source.replace(" /* retired elsewhere */ ", "");
    let project = project_rust(&source, 0).unwrap();
    let xaml = r#"<StackPanel xmlns="https://github.com/ferroui" Spacing="10"><TextBlock Text="New"/></StackPanel>"#;
    let edited = apply(&source, project.apply_xaml(xaml).unwrap());
    assert!(edited.starts_with("pub fn view() -> Element {\n    rsx! {\n        StackPanel {\n"));
    assert!(edited.ends_with("        }\n    }\n}\n"));
    assert!(project_rust(&edited, 0)
        .unwrap()
        .xaml()
        .contains("Text=\"New\""));
}

#[test]
fn markup_and_unmapped_rust_are_reported_without_silently_converting() {
    let xaml = Document::new(
        r#"<Button xmlns="https://github.com/ferroui" Content="{Binding Greeting}"/>"#,
    )
    .unwrap();
    assert!(xaml_to_rsx(&xaml).is_err());
    let xaml =
        Document::new(r#"<TextBlock xmlns="https://github.com/ferroui" Text="{}{Literal}"/>"#)
            .unwrap();
    let rsx = xaml_to_rsx(&xaml).unwrap();
    assert!(rsx.contains("\"{Literal}\""));
    assert_eq!(project_rust(&rsx, 0).unwrap().omitted_count, 0);
    assert!(apply_rsx(&xaml, "another! { TextBlock {} }").is_err());
    assert!(apply_rsx(&xaml, "rsx! { TextBlock { text: compute() } }").is_err());
}

#[test]
fn embedded_image_paths_round_trip_and_keep_unrelated_rust() {
    let source = r#"<Image xmlns="https://github.com/ferroui" Source="../Assets/%C5%81%C3%B3d%C5%BA%20%23%25.png" Stretch="Uniform"/>"#;
    let xaml = Document::new(source).unwrap();
    let rsx = xaml_to_rsx(&xaml).unwrap();
    assert!(rsx.contains(
        r#"::ferroui_view::embedded_image(::core::include_bytes!("../Assets/Łódź #%.png"))"#
    ));
    assert!(apply_rsx(&xaml, &rsx).unwrap().is_empty());

    let rust =
        format!("// Preserve this comment\nfn view() {{ let _ = {rsx}; }}\nfn untouched() {{}}\n");
    let projected = project_rust(&rust, rust.find("include_bytes").unwrap()).unwrap();
    assert_eq!(projected.omitted_count, 0);
    let replacement = projected
        .set_property(0, "Source", "../Assets/Summer%20palette.png")
        .unwrap();
    assert_eq!(
        apply(&rust, vec![replacement]),
        rust.replace("Łódź #%.png", "Summer palette.png")
    );
    let annotated = r###"fn view() { let _ = rsx! { Image {
        source: ::ferroui_view::embedded_image(
            /* keep the attribution 🎨 */ ::core::include_bytes!(r#"../Assets/Łódź #%.png"#),
        ),
    } }; }"###;
    let projected_image = project_rust(annotated, annotated.find("include_bytes").unwrap()).unwrap();
    assert!(projected_image.nodes[0].can_set_image_source());
    let patch = projected_image.set_property(0, "Source", "../Assets/Summer%20palette.png").unwrap();
    assert_eq!(apply(annotated, vec![patch]), annotated.replace("Łódź #%.png", "Summer palette.png"));
    for value in [
        "../Assets/foo%2Fbar.png",
        "%2Fetc/hosts",
        "file:///tmp/foo.png",
        "{Binding Logo}",
    ] {
        assert!(
            projected.set_property(0, "Source", value).is_err(),
            "{value}"
        );
    }

    let runtime = r#"rsx! { Image { source: load_thumbnail() } }"#;
    let projected = project_rust(runtime, 0).unwrap();
    assert!(!projected.nodes[0].can_set_image_source());
    assert!(projected
        .edit_property(0, "Source", Some("icon.png"))
        .is_err());
}

#[test]
fn comments_in_counterpart_drafts_require_the_authored_format() {
    let source = "<Button Content=\"Save\"/>";
    let xaml = Document::new(source).unwrap();
    let rsx = xaml_to_rsx(&xaml).unwrap();
    assert!(apply_rsx(&xaml, &format!("// keep this comment\n{rsx}")).is_err());
    let projection = project_rust(&rsx, 0).unwrap();
    assert!(projection
        .apply_xaml(&format!("<!-- keep this comment -->{source}"))
        .is_err());
}

#[test]
fn alternate_rsx_literals_preserve_authored_xaml_design_metadata() {
    let source = r#"<StackPanel xmlns="https://github.com/ferroui"
        xmlns:preview="http://schemas.microsoft.com/expression/blend/2008"
        xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" mc:Ignorable="preview"
        preview:Fixtures='{"default":"Ready","fixtures":[{"name":"Ready","data":{"Title":"Example"}}]}'>
        <Button Content="Save" preview:Content="Fixture action"/>
        <preview:TextBlock Text="Only visible while designing"/>
    </StackPanel>"#;
    let xaml = Document::new(source).unwrap();
    let rsx = xaml_to_rsx(&xaml).unwrap();
    assert!(rsx.contains("Save"));
    assert!(!rsx.contains("Fixture action"));
    assert!(!rsx.contains("Only visible"));
    let edit = apply_rsx(&xaml, &rsx.replace("Save", "Review")).unwrap();
    assert_eq!(
        apply(source, edit),
        source.replace("Content=\"Save\"", "Content=\"Review\"")
    );
    assert!(
        apply_rsx(
            &xaml,
            r#"rsx! { StackPanel { TextBlock { text: "Replacement" } } }"#
        )
        .is_err()
    );
    assert!(project_rust(&rsx, 0).unwrap().apply_xaml(source).is_err());
}

#[test]
fn text_shorthand_maps_to_the_text_literal() {
    let rust = r#"rsx! { TextBlock { "Hello" } }"#;
    let projection = project_rust(rust, 0).unwrap();
    assert!(projection.xaml().contains("Text=\"Hello\""));
    let edit = projection.set_property(0, "Text", "Goodbye").unwrap();
    assert_eq!(apply(rust, vec![edit]), rust.replace("Hello", "Goodbye"));
    let xaml = Document::new("<TextBlock>Hello</TextBlock>").unwrap();
    assert!(xaml_to_rsx(&xaml).unwrap().contains("text: \"Hello\""));
}
