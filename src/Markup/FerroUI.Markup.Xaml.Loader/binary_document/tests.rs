use super::*;
use xamlx::ast::*;

const COMPLEX: &str = r#"<r:View xmlns:r="urn:root" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" xmlns:a="urn:outer" x:TypeArguments="a:Item(r:Pair(x:String,x:Int32))" x:Name="Root">
  <r:View.Title><r:Span xml:space="preserve">  Świat  </r:Span></r:View.Title>
  <a:Item xmlns:a="urn:inner" r:Owner.Attached="{a:Outer {a:Inner}, Target={a:Nested Path=Value}}"/>
</r:View>"#;

fn theme_fixture() -> String {
    use std::fmt::Write;
    let mut xaml = String::from(
        "<Styles xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'><Styles.Resources>",
    );
    for row in 0..24 {
        writeln!(
            xaml,
            "<SolidColorBrush x:Key='AccentBrush{row}' Color='#ff{row:06x}'/>"
        )
        .unwrap();
    }
    xaml.push_str("</Styles.Resources>");
    for row in 0..24 {
        writeln!(xaml, "<Style Selector='Button.variant{row}'><Setter Property='Background' Value='{{DynamicResource AccentBrush{row}}}'/></Style>").unwrap();
    }
    xaml.push_str("</Styles>");
    xaml
}

#[test]
fn lexical_scopes_and_shared_type_references_survive_replay() {
    let doc = decode(&compile(COMPLEX).unwrap()).unwrap();
    let root = doc.root().unwrap().cast::<XamlAstObjectNode>().unwrap();
    assert_eq!(
        root.namespace_scope().unwrap().lookup("a"),
        Some("urn:outer")
    );
    let root_type = root
        .type_
        .borrow()
        .cast::<XamlAstXmlTypeReference>()
        .unwrap();
    let generic = root_type.generic_arguments.borrow()[0].clone();
    assert_eq!(generic.name(), "Item");
    assert_eq!(
        generic.generic_arguments.borrow()[0]
            .generic_arguments
            .borrow()
            .len(),
        2
    );
    let children = root.children.borrow();
    let title = children
        .iter()
        .find_map(|n| n.cast::<XamlAstXamlPropertyValueNode>())
        .unwrap();
    assert!(!title.is_attribute_syntax);
    let property = title
        .property()
        .cast::<XamlAstNamePropertyReference>()
        .unwrap();
    assert!(property.target_type.borrow().same_node(&root_type));
    assert!(property.declaring_type.borrow().same_node(&root_type));
    let span = title.values.borrow()[0]
        .cast::<XamlAstObjectNode>()
        .unwrap();
    let text = span.children.borrow()[0].cast::<XamlAstTextNode>().unwrap();
    assert!(text.preserve_whitespace);
    assert_eq!(text.text(), "  Świat  ");
    let inner = children
        .iter()
        .find_map(|n| n.cast::<XamlAstObjectNode>())
        .unwrap();
    assert_eq!(inner.line(), 3);
    assert_eq!(
        inner.namespace_scope().unwrap().lookup("a"),
        Some("urn:inner")
    );
    let attached = inner
        .children
        .borrow()
        .iter()
        .find_map(|n| n.cast::<XamlAstXamlPropertyValueNode>())
        .unwrap();
    assert!(attached.is_attribute_syntax);
    let ext = attached.values.borrow()[0]
        .cast::<XamlAstObjectNode>()
        .unwrap();
    assert!(ext.type_.borrow().is_markup_extension());
    assert_eq!(ext.arguments.borrow().len(), 1);
    assert_eq!(ext.children.borrow().len(), 1);
}

#[test]
fn realistic_theme_and_nested_syntax_round_trip_byte_for_byte() {
    let theme = theme_fixture();
    for s in [COMPLEX, theme.as_str()] {
        let doc = crate::compiler_extensions::FerroXamlIlCompiler::parse_source_syntax(s).unwrap();
        let stored = encode_stored(&doc).unwrap();
        let bytes = encode(&doc).unwrap();
        assert_eq!(encode_stored(&decode(&bytes).unwrap()).unwrap(), stored);
        assert_eq!(encode(&decode(&stored).unwrap()).unwrap(), bytes);
    }
    let nested = format!(
        "<A xmlns='urn:a'>{}x{}</A>",
        "<B>".repeat(80),
        "</B>".repeat(80)
    );
    assert!(decode(&compile(&nested).unwrap()).is_ok());
}

#[test]
fn invalid_images_and_zlib_trailing_data_fail_before_transformation() {
    let short = compile("<A xmlns='urn:a'/>").unwrap();
    for n in 0..short.len() {
        assert!(decode(&short[..n]).is_err());
    }
    for offset in [0, 4, 5, 6] {
        let mut bad = short.clone();
        bad[offset] = 0xff;
        assert!(decode(&bad).is_err());
    }
    let mut compressed = compile(&theme_fixture()).unwrap();
    assert_eq!(compressed[5], 1);
    compressed.push(0);
    assert!(decode(&compressed).is_err());
    compressed[6..10].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode(&compressed).is_err());
    // Empty strings/scopes, one object claiming a child before any node exists.
    assert!(decode(b"FXD\0\x01\0\0\0\0\0\x01\x03\0\0\0\xff\xff\xff\xff\x0f").is_err());
}

#[test]
fn compact_dag_cannot_request_exponential_transform_work() {
    // Each XML generic type names the preceding type twice. The file itself has
    // linear size while recursively visiting all generic arguments would double.
    let mut image = b"FXD\0\x01\0\x01\x01T\0\0\0\x19".to_vec();
    image.extend([TYPE, 1, 1, 0, 0, 0, 0, 0]);
    for _ in 1..24 {
        image.extend([TYPE, 1, 1, 0, 0, 0, 0, 2, 0, 0]);
    }
    image.extend([OBJECT, 1, 1, 0, 0, 0, 0]);
    let error = decode(&image).err().unwrap().message();
    assert!(error.contains("expanded graph"), "{error}");
}

#[test]
fn encoder_rejects_shared_occurrences_invalid_children_and_expansion_bombs() {
    let doc = crate::compiler_extensions::FerroXamlIlCompiler::parse_source_syntax(
        "<A xmlns='urn:a'><B/></A>",
    )
    .unwrap();
    let root = doc.root().unwrap().cast::<XamlAstObjectNode>().unwrap();
    let shared = root.children.borrow()[0].clone();
    root.children.borrow_mut().push(shared);
    assert!(encode(&doc)
        .unwrap_err()
        .message()
        .contains("shared mutable occurrence"));
    root.children.borrow_mut().clear();
    root.children
        .borrow_mut()
        .push(root.type_.borrow().as_node());
    assert!(encode(&doc)
        .unwrap_err()
        .message()
        .contains("invalid object child"));
    root.children.borrow_mut().clear();
    let mut ty = root
        .type_
        .borrow()
        .cast::<XamlAstXmlTypeReference>()
        .unwrap();
    for _ in 0..24 {
        ty = XamlAstXmlTypeReference::with_generic_arguments(
            &*root,
            Some("urn:a"),
            "Pair",
            vec![ty.clone(), ty],
        );
    }
    root.type_.replace(ty);
    assert!(encode(&doc)
        .unwrap_err()
        .message()
        .contains("expanded graph"));
}

#[test]
fn stored_and_compressed_images_round_trip_near_the_xml_nesting_limit() {
    // The parser's element-depth limit is 256. Property elements add another syntax
    // node on the way to the nested object. Run the XML parser on a suitable stack.
    std::thread::Builder::new()
        .stack_size(16 << 20)
        .spawn(|| {
            let xaml = format!(
                "<Root xmlns='urn:a'>{}text{}</Root>",
                "<Child>".repeat(255),
                "</Child>".repeat(255)
            );
            let doc = crate::compiler_extensions::FerroXamlIlCompiler::parse_source_syntax(&xaml)
                .unwrap();
            let stored = encode_stored(&doc).unwrap();
            assert_eq!(
                encode_stored(&decode(&encode(&doc).unwrap()).unwrap()).unwrap(),
                stored
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn archive_replays_packed_syntax_and_borrows_independent_images() {
    use super::pack::{compile_documents, ThemePack};

    let theme = theme_fixture();
    let docs = [
        ("Themes/Many.xaml", theme.as_str()),
        ("Nested.xaml", COMPLEX),
    ];
    let image = compile_documents(&docs).unwrap();
    assert_eq!(compile_documents(&[docs[1], docs[0]]).unwrap(), image);
    let pack = ThemePack::open(&image).unwrap();
    assert_eq!(
        pack.paths().collect::<Vec<_>>(),
        ["Nested.xaml", "Themes/Many.xaml"]
    );
    assert!(pack.contains("Themes/Many.xaml"));
    assert!(pack.document("Themes/Missing.xaml").is_err());
    pack.validate().unwrap();
    for &(path, xml) in &docs {
        let packed = pack.document(path).unwrap();
        let expected =
            crate::compiler_extensions::FerroXamlIlCompiler::parse_source_syntax(xml).unwrap();
        assert_eq!(
            encode_stored(&decode(packed).unwrap()).unwrap(),
            encode_stored(&expected).unwrap()
        );
    }
    let mut damaged = image.clone();
    *damaged.last_mut().unwrap() ^= 0xff;
    let pack = ThemePack::open(&damaged).unwrap();
    assert!(decode(pack.document("Nested.xaml").unwrap()).is_ok());
    assert!(pack.validate().is_err());
}

#[test]
fn archive_rejects_invalid_paths_ranges_and_expansion_before_materializing() {
    use super::pack::{compile_documents, ThemePack};
    let xaml = "<A xmlns='urn:a'/>";
    for path in ["/Absolute.xaml", "../escape", "a/./b", "a//b", "a\\b", ""] {
        assert!(compile_documents(&[(path, xaml)]).is_err());
    }
    assert!(compile_documents(&[("a", xaml), ("a", xaml)]).is_err());
    let image = compile_documents(&[("a", xaml), ("b", xaml)]).unwrap();
    for i in 0..image.len() {
        assert!(ThemePack::open(&image[..i]).is_err());
    }
    let mut bad = image.clone();
    bad[22] = b'a'; // duplicate second path in directory
    assert!(ThemePack::open(&bad).is_err());
    let mut bad = image.clone();
    bad[16..20].copy_from_slice(&u32::MAX.to_le_bytes()); // first image range
    assert!(ThemePack::open(&bad).is_err());
    let mut bad = image;
    bad.push(0);
    assert!(ThemePack::open(&bad).is_err());
}

#[test]
fn corrupt_columnar_syntax_never_overreads_adjacent_lanes() {
    let syntax =
        crate::compiler_extensions::FerroXamlIlCompiler::parse_source_syntax("<A xmlns='urn:a'/>")
            .unwrap();
    let image = super::encode::encode_columnar(&syntax).unwrap();
    assert_eq!(image[5], 2);
    // All truncated variants and any trailing operand bytes must be rejected.
    for end in 0..image.len() {
        assert!(decode(&image[..end]).is_err());
    }
    let mut trailing = image.clone();
    trailing.push(0);
    assert!(decode(&trailing).is_err());
    // Every byte can be hostile. An accepted mutation must still produce a
    // bounded syntax graph that the canonical encoder can serialize.
    for i in 6..image.len() {
        for byte in [0, 1, 0x80, 0xff] {
            let mut mutated = image.clone();
            mutated[i] = byte;
            if let Ok(graph) = decode(&mutated) {
                encode_stored(&graph).unwrap();
            }
        }
    }
}
