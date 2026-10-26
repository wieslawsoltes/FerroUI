use crate::*;
use serde_json::json;
use std::collections::BTreeMap;

const NS: &str =
    "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";
fn service(source: &str) -> LanguageService {
    let mut service = LanguageService::default();
    service.open("file:///View.xaml", source, 1).unwrap();
    service
}
#[test]
fn utf16_positions_reject_split_surrogates_and_preserve_crlf() {
    let text = "α🦀\r\n漢x\n";
    let index = LineIndex::new(text);
    assert_eq!(
        index
            .offset(Position {
                line: 0,
                character: 3
            })
            .unwrap(),
        6
    );
    assert!(index
        .offset(Position {
            line: 0,
            character: 2
        })
        .is_err());
    assert_eq!(
        index.position(8).unwrap(),
        Position {
            line: 1,
            character: 0
        }
    );
    assert_eq!(
        index
            .offset(Position {
                line: 1,
                character: 1
            })
            .unwrap(),
        11
    );
    assert_eq!(index.line_count(), 3);
}
#[test]
fn incremental_line_index_matches_fresh_indices_after_multiline_and_unicode_edits() {
    let mut index = LineIndex::new("a🦀b\r\nsecond\nthird");
    index
        .apply(&TextChange {
            range: Some(Range {
                start: Position {
                    line: 0,
                    character: 1,
                },
                end: Position {
                    line: 0,
                    character: 3,
                },
            }),
            range_length: Some(2),
            text: "界\n🙂\r\n".into(),
        })
        .unwrap();
    let fresh = LineIndex::new(index.source());
    for offset in 0..=index.source().len() {
        if index.source().is_char_boundary(offset) {
            assert_eq!(index.position(offset), fresh.position(offset));
        }
    }
    let last = index.position(index.source().len()).unwrap();
    index
        .apply(&TextChange {
            range: Some(Range {
                start: Position::default(),
                end: last,
            }),
            range_length: None,
            text: "\r\n".into(),
        })
        .unwrap();
    assert_eq!(index.line_count(), 2);
    assert_eq!(
        index
            .offset(Position {
                line: 1,
                character: 0
            })
            .unwrap(),
        2
    );
}
#[test]
fn incremental_changes_are_atomic_versioned_and_retain_invalid_drafts() {
    let mut service = service("<Button Content='🦀'/>");
    let source = service
        .document("file:///View.xaml")
        .unwrap()
        .source()
        .to_owned();
    let offset = source.find('🦀').unwrap();
    let position = service
        .index("file:///View.xaml")
        .unwrap()
        .position(offset)
        .unwrap();
    let changes = vec![
        TextChange {
            range: Some(Range {
                start: Position::default(),
                end: Position::default(),
            }),
            range_length: None,
            text: " ".into(),
        },
        TextChange {
            range: Some(Range {
                start: Position {
                    character: position.character + 2,
                    ..position
                },
                end: Position {
                    character: position.character + 2,
                    ..position
                },
            }),
            range_length: None,
            text: "x".into(),
        },
    ];
    assert!(service.change("file:///View.xaml", 2, &changes).is_err());
    assert_eq!(service.version("file:///View.xaml"), Some(1));
    assert_eq!(
        service.document("file:///View.xaml").unwrap().source(),
        source
    );
    service.replace("file:///View.xaml", "<Bu", 2).unwrap();
    assert!(!service.document("file:///View.xaml").unwrap().is_valid());
    assert_eq!(
        service.diagnostics("file:///View.xaml").unwrap()[0].code,
        "xml"
    );
    assert!(service.replace("file:///View.xaml", "<Grid/>", 2).is_err());
    assert!(service
        .completions("file:///View.xaml", 3)
        .unwrap()
        .iter()
        .any(|c| c.label == "Button"));
}
#[test]
fn schema_comes_from_framework_and_completion_inherits_properties_and_enums() {
    let schema = Schema::ferroui();
    assert!(schema.types.len() > 300);
    assert!(schema.properties("Button", None).contains_key("Width"));
    let service = service("<Button W");
    let completions = service.completions("file:///View.xaml", 9).unwrap();
    assert!(completions
        .iter()
        .any(|c| c.label == "Width" && c.insert_text == "Width=\"\""));
    let source = "<StackPanel Orientation='H'/>";
    let service = self::service(source);
    let completions = service
        .completions("file:///View.xaml", source.find("H'").unwrap() + 1)
        .unwrap();
    assert!(completions.iter().any(|c| c.label == "Horizontal"));
}
#[test]
fn custom_schema_hover_and_value_diagnostics_resolve_xml_namespaces() {
    let mut service = service("<c:Widget xmlns:c='using:Custom' Mode='bad'/>");
    let mut properties = BTreeMap::new();
    properties.insert(
        "Mode".into(),
        PropertyInfo {
            name: "Mode".into(),
            type_name: "Choice".into(),
            values: vec!["A".into(), "B".into()],
            documentation: "Pick a mode".into(),
            ..Default::default()
        },
    );
    service.schema_mut().register(TypeInfo {
        name: "Widget".into(),
        namespace: "using:Custom".into(),
        constructible: true,
        properties,
        ..Default::default()
    });
    assert!(service
        .diagnostics("file:///View.xaml")
        .unwrap()
        .iter()
        .any(|d| d.code == "enum-value"));
    let source = service.document("file:///View.xaml").unwrap().source();
    let hover = service
        .hover("file:///View.xaml", source.find("Mode").unwrap())
        .unwrap()
        .unwrap();
    assert!(hover.contents.contains("Pick a mode"));
}
#[test]
fn semantic_diagnostics_cover_types_properties_values_and_markup() {
    let source=format!("<StackPanel {NS} Orientation='Diagonal'><Button Opacity='bad' Nope='1' Content='{{Binding'/><Missing/></StackPanel>");
    let service = service(&source);
    let codes: Vec<_> = service
        .diagnostics("file:///View.xaml")
        .unwrap()
        .into_iter()
        .map(|d| d.code)
        .collect();
    for code in [
        "enum-value",
        "numeric-value",
        "unknown-property",
        "markup",
        "unknown-type",
    ] {
        assert!(codes.iter().any(|c| c == code), "missing {code}: {codes:?}");
    }
}
#[test]
fn name_rename_rewrites_actual_references_but_not_strings_or_other_namescopes() {
    let source=format!("<Grid {NS}><Button Name='Target'/><TextBlock Text='{{Binding ElementName=Target}}'/><TextBlock Text='{{Binding #Target.Width}}'/><Button Tag='{{x:Reference Target}}' RelativePanel.RightOf='Target' SourceObject='Target' Content='Target'/><ControlTemplate><Border Name='Target'/><TextBlock Tag='{{x:Reference Target}}'/></ControlTemplate></Grid>");
    let mut service = service(&source);
    let offset = source.find("Name='Target'").unwrap() + 6;
    let edit = service
        .rename("file:///View.xaml", offset, "Renamed")
        .unwrap();
    assert_eq!(edit.document_changes.len(), 1);
    assert_eq!(edit.document_changes[0].edits.len(), 6);
    service.apply_workspace_edit(&edit).unwrap();
    let renamed = service.document("file:///View.xaml").unwrap().source();
    assert!(renamed.contains("Content='Target'"));
    assert!(renamed.contains("<Border Name='Target'/>"));
    assert!(renamed.contains("#Renamed.Width"));
    assert!(renamed.contains("SourceObject='Renamed'"));
    assert!(service.apply_workspace_edit(&edit).is_err());
}
#[test]
fn copy_reference_rewriter_preserves_external_and_plain_text_values() {
    let source=format!("<Grid {NS}><Button x:Name='Inner'/><TextBlock Text='{{Binding ElementName=Inner}}' Tag='{{x:Reference External}}' Name='Text'/></Grid>");
    let map = BTreeMap::from([
        ("Inner".into(), "InnerCopy".into()),
        ("External".into(), "ExternalCopy".into()),
    ]);
    let rewritten = rewrite_names(&source, &map).unwrap();
    assert!(rewritten.contains("ElementName=InnerCopy"));
    assert!(rewritten.contains("x:Reference External}"));
}
#[test]
fn resource_definitions_references_and_rename_cross_documents_atomically() {
    let resources=format!("<ResourceDictionary {NS}><SolidColorBrush x:Key='Accent' Color='Red'/></ResourceDictionary>");
    let usage = format!("<Button {NS} Background='{{StaticResource Accent}}'/>");
    let mut service = LanguageService::default();
    service
        .open("file:///Resources.xaml", &resources, 3)
        .unwrap();
    service.open("file:///View.xaml", &usage, 7).unwrap();
    let offset = usage.find("Accent").unwrap();
    let definition = service.definitions("file:///View.xaml", offset).unwrap();
    assert_eq!(definition.len(), 1);
    assert_eq!(definition[0].uri, "file:///Resources.xaml");
    assert_eq!(
        service
            .references("file:///View.xaml", offset)
            .unwrap()
            .len(),
        2
    );
    let edit = service
        .rename("file:///View.xaml", offset, "Primary")
        .unwrap();
    assert_eq!(edit.document_changes.len(), 2);
    service.apply_workspace_edit(&edit).unwrap();
    assert!(service
        .document("file:///Resources.xaml")
        .unwrap()
        .source()
        .contains("x:Key='Primary'"));
    assert!(service
        .document("file:///View.xaml")
        .unwrap()
        .source()
        .contains("StaticResource Primary"));
}
#[test]
fn resources_shadow_ancestor_resources_without_duplicate_scope_diagnostics() {
    let source=format!("<Grid {NS}><Grid.Resources><SolidColorBrush x:Key='Accent'/></Grid.Resources><Border><Border.Resources><SolidColorBrush x:Key='Accent'/></Border.Resources><Button Background='{{StaticResource Accent}}'/></Border><Button Background='{{StaticResource Accent}}'/></Grid>");
    let service = service(&source);
    let references: Vec<_> = source
        .match_indices("StaticResource Accent")
        .map(|(i, _)| i + 15)
        .collect();
    let inner = service
        .definitions("file:///View.xaml", references[0])
        .unwrap();
    let outer = service
        .definitions("file:///View.xaml", references[1])
        .unwrap();
    assert_ne!(inner, outer);
    assert!(!service
        .diagnostics("file:///View.xaml")
        .unwrap()
        .iter()
        .any(|d| d.code == "duplicate-symbol"));
}
#[test]
fn markup_signature_reports_named_parameter() {
    let source = "<Button Content='{Binding Path=Title, ElementName=Target}'/>";
    let service = service(source);
    let help = service
        .signature_help("file:///View.xaml", source.find("Target").unwrap() + 6)
        .unwrap()
        .unwrap();
    assert_eq!(help.active_parameter, 1);
    assert!(help.signatures[0]
        .parameters
        .contains(&"ElementName".into()));
}
#[test]
fn formatting_preserves_quotes_comments_mixed_content_and_xml_space() {
    let source="<Grid><!-- keep --><TextBlock Text = 'x'/><TextBlock>hello <Run Text='world'/> !</TextBlock><Border xml:space='preserve'>  <Button/>  </Border></Grid>";
    let mut service = service(source);
    let edits = service
        .format("file:///View.xaml", FormattingOptions::default())
        .unwrap();
    let formatted = apply_text_edits(source, &edits).unwrap();
    assert!(formatted.contains("\n    <!-- keep -->"));
    assert!(formatted.contains("Text = 'x'"));
    assert!(formatted.contains("hello <Run Text='world'/> !"));
    assert!(formatted.contains("preserve'>  <Button/>  </Border>"));
    service.replace("file:///View.xaml", &formatted, 2).unwrap();
    assert!(service
        .format("file:///View.xaml", FormattingOptions::default())
        .unwrap()
        .is_empty());
}
#[test]
fn symbols_folding_and_tokens_have_valid_utf16_positions() {
    let source = format!("<Grid {NS}>\n <!-- 🦀 -->\n <Button Name='Action' Width='10'/>\n</Grid>");
    let service = service(&source);
    assert_eq!(
        service.symbols("file:///View.xaml").unwrap()[0].children[0].name,
        "Action"
    );
    assert!(!service.folding("file:///View.xaml").unwrap().is_empty());
    let tokens = service.semantic_tokens("file:///View.xaml").unwrap();
    assert!(tokens.iter().any(|t| t.token_type == 3));
    assert!(tokens.iter().any(|t| t.token_type == 6));
    for pair in tokens.windows(2) {
        if pair[0].line == pair[1].line {
            assert!(pair[0].start + pair[0].length <= pair[1].start);
        }
    }
}
#[test]
fn cross_document_edits_fail_atomically_when_one_version_is_stale() {
    let mut service = service("<Grid/>");
    service.open("file:///Other.xaml", "<Border/>", 3).unwrap();
    let edit = WorkspaceEdit {
        document_changes: vec![
            DocumentEdits {
                uri: "file:///View.xaml".into(),
                version: 1,
                edits: vec![TextEdit {
                    range: service
                        .index("file:///View.xaml")
                        .unwrap()
                        .range(ferroui_designer_core::SourceSpan::new(0, 7))
                        .unwrap(),
                    new_text: "<Button/>".into(),
                }],
            },
            DocumentEdits {
                uri: "file:///Other.xaml".into(),
                version: 2,
                edits: vec![],
            },
        ],
    };
    assert!(service.apply_workspace_edit(&edit).is_err());
    assert_eq!(
        service.document("file:///View.xaml").unwrap().source(),
        "<Grid/>"
    );
}
#[test]
fn lsp_initialization_incremental_sync_completion_and_shutdown_use_real_service() {
    let mut server = lsp::LspServer::default();
    let init = server.handle(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}));
    assert_eq!(
        init[0]["result"]["capabilities"]["textDocumentSync"]["change"],
        2
    );
    let published=server.handle(json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file:///V.xaml","languageId":"xaml","version":1,"text":"<Bu"}}}));
    assert_eq!(published[0]["method"], "textDocument/publishDiagnostics");
    let completion=server.handle(json!({"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{"textDocument":{"uri":"file:///V.xaml"},"position":{"line":0,"character":3}}}));
    assert!(completion[0]["result"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["label"] == "Button"));
    server.handle(json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"file:///V.xaml","version":2},"contentChanges":[{"text":"<Button/>"}]}}));
    assert!(server
        .service
        .document("file:///V.xaml")
        .unwrap()
        .is_valid());
    assert!(
        server.handle(json!({"jsonrpc":"2.0","id":3,"method":"shutdown"}))[0]["result"].is_null()
    );
    assert!(server
        .handle(json!({"jsonrpc":"2.0","method":"exit"}))
        .is_empty());
}
#[test]
fn lsp_cancel_and_protocol_errors_are_explicit() {
    let mut server = lsp::LspServer::default();
    assert_eq!(
        server.handle(json!({"jsonrpc":"2.0","id":1,"method":"textDocument/hover"}))[0]["error"]
            ["code"],
        -32002
    );
    server.handle(json!({"jsonrpc":"2.0","id":2,"method":"initialize"}));
    server.handle(json!({"jsonrpc":"2.0","method":"$/cancelRequest","params":{"id":9}}));
    assert_eq!(
        server.handle(
            json!({"jsonrpc":"2.0","id":9,"method":"workspace/symbol","params":{"query":"Button"}})
        )[0]["error"]["code"],
        -32800
    );
    assert_eq!(
        server.handle(json!({"jsonrpc":"2.0","id":10,"method":"does/notExist"}))[0]["error"]
            ["code"],
        -32601
    );
}
#[test]
fn lsp_frames_count_utf8_bytes_and_reject_ambiguous_lengths() {
    let message = json!({"jsonrpc":"2.0","id":1,"method":"test","params":{"text":"🦀"}});
    let mut bytes = Vec::new();
    lsp::write_frame(&mut bytes, &message).unwrap();
    assert_eq!(
        lsp::read_frame(&mut std::io::Cursor::new(bytes)).unwrap(),
        Some(message)
    );
    assert!(lsp::read_frame(&mut std::io::Cursor::new(
        b"Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}".to_vec()
    ))
    .is_err());
}
#[test]
fn stdio_server_runs_initialize_and_shutdown_end_to_end() {
    let mut input = Vec::new();
    for message in [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
        json!({"jsonrpc":"2.0","id":2,"method":"shutdown"}),
        json!({"jsonrpc":"2.0","method":"exit"}),
    ] {
        lsp::write_frame(&mut input, &message).unwrap();
    }
    let mut output = Vec::new();
    lsp::serve(std::io::Cursor::new(input), &mut output).unwrap();
    let mut reader = std::io::Cursor::new(output);
    assert_eq!(lsp::read_frame(&mut reader).unwrap().unwrap()["id"], 1);
    assert_eq!(lsp::read_frame(&mut reader).unwrap().unwrap()["id"], 2);
    assert!(lsp::read_frame(&mut reader).unwrap().is_none());
}

#[test]
fn nested_markup_arguments_cannot_steal_sibling_resource_keys_or_string_literals() {
    let source = format!(
        r#"<Grid {NS}><Grid.Resources><Color x:Key='First'/><Color x:Key='Second'/></Grid.Resources><Button Name='Target' Content="{{Binding Path=#Target.Text, Converter={{StaticResource First}}, ConverterParameter={{StaticResource ResourceKey=Second}}, StringFormat='ElementName=External #Ghost'}}"/></Grid>"#
    );
    let mut service = service(&source);
    let first = source.find("First'").unwrap();
    let references = service.references("file:///View.xaml", first).unwrap();
    assert_eq!(references.len(), 2);
    let edit = service
        .rename("file:///View.xaml", first, "Renamed")
        .unwrap();
    service.apply_workspace_edit(&edit).unwrap();
    let changed = service.document("file:///View.xaml").unwrap().source();
    assert!(changed.contains("StaticResource Renamed}"));
    assert!(changed.contains("ResourceKey=Second}"));
    assert!(
        !name_occurrences(service.document("file:///View.xaml").unwrap())
            .iter()
            .any(|reference| matches!(reference.name.as_str(), "External" | "Ghost"))
    );
}

#[test]
fn quoted_resource_keys_preserve_raw_entity_spans_and_rename_safely() {
    let source = format!(
        r#"<Grid {NS}><Grid.Resources><Color x:Key='A&amp;B'/></Grid.Resources><Button Background="{{StaticResource &quot;A&amp;B&quot;}}"/><Button Background="{{DynamicResource ResourceKey='A&amp;B'}}"/></Grid>"#
    );
    let mut service = service(&source);
    let at = source.find("A&amp;B'").unwrap();
    assert_eq!(
        service.references("file:///View.xaml", at).unwrap().len(),
        3
    );
    let edit = service
        .rename("file:///View.xaml", at, "Hot 'pink' & 🦀")
        .unwrap();
    service.apply_workspace_edit(&edit).unwrap();
    assert!(service.document("file:///View.xaml").unwrap().is_valid());
    assert_eq!(
        service.references("file:///View.xaml", at).unwrap().len(),
        3
    );
    assert!(!service
        .diagnostics("file:///View.xaml")
        .unwrap()
        .iter()
        .any(|diagnostic| matches!(diagnostic.code.as_str(), "unresolved-reference" | "markup")));
}

#[test]
fn rename_does_not_guess_which_ambiguous_resource_a_reference_uses() {
    let mut service = service("<Button Background='{StaticResource Shared}'/>");
    let first = format!("<ResourceDictionary {NS}><Color x:Key='Shared'/></ResourceDictionary>");
    service.open("file:///A.xaml", &first, 1).unwrap();
    service.open("file:///B.xaml", &first, 1).unwrap();
    assert_eq!(
        service.definitions("file:///View.xaml", 37).unwrap().len(),
        2
    );
    assert!(service
        .rename("file:///View.xaml", 37, "Different")
        .is_err());
    let edit = service
        .rename("file:///A.xaml", first.find("Shared").unwrap(), "Different")
        .unwrap();
    assert_eq!(edit.document_changes.len(), 1);
    assert_eq!(edit.document_changes[0].uri, "file:///A.xaml");
}

#[test]
fn imported_resources_resolve_in_merge_order_and_cycles_terminate() {
    let first = format!("<ResourceDictionary {NS}><Color x:Key='Shared'/></ResourceDictionary>");
    let view = "<Grid><Grid.Resources><ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceInclude Source='A.xaml'/><ResourceInclude Source='B.xaml'/></ResourceDictionary.MergedDictionaries></ResourceDictionary></Grid.Resources><Button Background='{StaticResource Shared}'/></Grid>";
    let mut service = service(view);
    service.open("file:///A.xaml", &first, 1).unwrap();
    service.open("file:///B.xaml", &first, 1).unwrap();
    let definitions = service
        .definitions("file:///View.xaml", view.find("Shared").unwrap())
        .unwrap();
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].uri, "file:///B.xaml");
    service.replace("file:///B.xaml", "<ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceInclude Source='A.xaml'/></ResourceDictionary.MergedDictionaries></ResourceDictionary>", 2).unwrap();
    service.replace("file:///A.xaml", "<ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceInclude Source='B.xaml'/></ResourceDictionary.MergedDictionaries></ResourceDictionary>", 2).unwrap();
    assert!(service
        .definitions("file:///View.xaml", view.find("Shared").unwrap())
        .unwrap()
        .is_empty());
}

#[test]
fn private_control_resources_are_not_visible_in_other_documents() {
    let source = format!("<Grid {NS}><Border><Border.Resources><Color x:Key='Private'/></Border.Resources></Border></Grid>");
    let mut service = service("<Button Background='{StaticResource Private}'/>");
    service.open("file:///Other.xaml", &source, 1).unwrap();
    assert!(service
        .definitions("file:///View.xaml", 37)
        .unwrap()
        .is_empty());
}

#[test]
fn qualified_custom_type_completion_and_builtin_base_members_share_namespace_rules() {
    let mut service = service("<Grid xmlns:c='using:Custom'><c:Wi");
    service.schema_mut().register(TypeInfo {
        name: "Widget".into(),
        namespace: "using:Custom".into(),
        constructible: true,
        base: Some("Button".into()),
        ..Default::default()
    });
    let items = service
        .completions(
            "file:///View.xaml",
            service
                .document("file:///View.xaml")
                .unwrap()
                .source()
                .len(),
        )
        .unwrap();
    assert_eq!(
        items
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        vec!["c:Widget"]
    );
    service
        .replace(
            "file:///View.xaml",
            "<c:Widget xmlns:c='using:Custom' Wid",
            2,
        )
        .unwrap();
    assert!(service
        .completions("file:///View.xaml", 36)
        .unwrap()
        .iter()
        .any(|item| item.label == "Width"));
}

fn binding_schema(service: &mut LanguageService) -> Location {
    let location = Location {
        uri: "file:///ViewModel.rs".into(),
        range: Range {
            start: Position {
                line: 8,
                character: 4,
            },
            end: Position {
                line: 8,
                character: 9,
            },
        },
    };
    for (name, properties) in [
        (
            "ViewModel",
            vec![PropertyInfo {
                name: "Customer".into(),
                type_name: "Customer".into(),
                ..Default::default()
            }],
        ),
        (
            "Customer",
            vec![PropertyInfo {
                name: "Title".into(),
                type_name: "String".into(),
                documentation: "Customer display name".into(),
                definition: Some(location.clone()),
                ..Default::default()
            }],
        ),
    ] {
        service.schema_mut().register(TypeInfo {
            name: name.into(),
            namespace: "using:Models".into(),
            properties: properties
                .into_iter()
                .map(|property| (property.name.clone(), property))
                .collect(),
            ..Default::default()
        });
    }
    location
}

#[test]
fn typed_binding_completion_hover_and_definitions_follow_nested_property_types() {
    let source = format!("<Grid {NS} xmlns:vm='using:Models' x:DataType='vm:ViewModel'><TextBlock Text='{{Binding Customer.Title}}'/></Grid>");
    let mut service = service(&source);
    let location = binding_schema(&mut service);
    let at = source.find("Customer.Title").unwrap() + "Customer.Ti".len();
    let completion = service
        .completions("file:///View.xaml", at)
        .unwrap()
        .into_iter()
        .find(|item| item.label == "Title")
        .unwrap();
    assert_eq!(&source[completion.span.range()], "Ti");
    assert!(service
        .hover("file:///View.xaml", at)
        .unwrap()
        .unwrap()
        .contents
        .contains("Customer display name"));
    assert_eq!(
        service.definitions("file:///View.xaml", at).unwrap(),
        vec![location]
    );
}

#[test]
fn named_element_binding_completion_uses_actual_control_type() {
    let source =
        "<Grid><TextBox Name='Input'/><TextBlock Text='{Binding Te, ElementName=Input}'/></Grid>";
    let service = service(source);
    let items = service
        .completions(
            "file:///View.xaml",
            source.find("Binding Te").unwrap() + "Binding Te".len(),
        )
        .unwrap();
    assert!(items.iter().any(|item| item.label == "Text"));
}

#[test]
fn lsp_file_rename_groups_edits_for_multiple_imports_in_one_document() {
    let mut server = lsp::LspServer::default();
    server.handle(json!({"jsonrpc":"2.0","id":1,"method":"initialize"}));
    server
        .service
        .open(
            "file:///View.xaml",
            "<Grid><ResourceInclude Source='A.xaml'/><ResourceInclude Source='B.xaml'/></Grid>",
            1,
        )
        .unwrap();
    let reply = server.handle(json!({"jsonrpc":"2.0","id":2,"method":"workspace/willRenameFiles","params":{"files":[{"oldUri":"file:///A.xaml","newUri":"file:///NewA.xaml"},{"oldUri":"file:///B.xaml","newUri":"file:///NewB.xaml"}]}}));
    let changes = reply[0]["result"]["documentChanges"].as_array().unwrap();
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0]["edits"].as_array().unwrap().len(), 2);
}

#[test]
fn indexed_projects_survive_close_and_support_multiple_workspace_roots() {
    use ferroui_designer_project::{file_uri, Solution};
    let directory =
        std::env::temp_dir().join(format!("ferroui-language-project-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("one")).unwrap();
    std::fs::create_dir_all(directory.join("two")).unwrap();
    let first = format!("<ResourceDictionary {NS}><Color x:Key='Saved'/></ResourceDictionary>");
    std::fs::write(directory.join("one/Colors.xaml"), &first).unwrap();
    std::fs::write(directory.join("two/View.xaml"), "<Grid/>").unwrap();
    let one = Solution::discover(directory.join("one")).unwrap();
    let two = Solution::discover(directory.join("two")).unwrap();
    let uri = file_uri(directory.join("one/Colors.xaml")).unwrap();
    let other = file_uri(directory.join("two/View.xaml")).unwrap();
    let mut service = LanguageService::default();
    assert_eq!(service.index_solution(&one).unwrap(), 1);
    assert_eq!(service.index_solution(&two).unwrap(), 1);
    assert_eq!(
        service
            .resource_uri(&other, "ferres://one/Colors.xaml")
            .unwrap(),
        uri
    );
    service
        .open(&uri, first.replace("Saved", "Draft"), 1)
        .unwrap();
    service.close(&uri).unwrap();
    assert_eq!(service.document(&uri).unwrap().source(), first);
    assert_eq!(service.version(&uri), Some(-1));
    service.open(&other, "<Button/>", 1).unwrap();
    assert_eq!(service.remove_solution(&one.root), 1);
    assert!(service.document(&uri).is_none());
    assert_eq!(service.remove_solution(&two.root), 0);
    assert_eq!(service.document(&other).unwrap().source(), "<Button/>");
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn inline_merged_dictionaries_have_independent_keys_and_last_dictionary_wins() {
    let source = format!("<Grid {NS}><Grid.Resources><ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceDictionary><Color x:Key='Shared'/></ResourceDictionary><ResourceDictionary><Color x:Key='Shared'/></ResourceDictionary></ResourceDictionary.MergedDictionaries></ResourceDictionary></Grid.Resources><Button Background='{{StaticResource Shared}}'/></Grid>");
    let service = service(&source);
    assert!(!service
        .diagnostics("file:///View.xaml")
        .unwrap()
        .iter()
        .any(|diagnostic| diagnostic.code == "duplicate-symbol"));
    let reference = source.rfind("Shared").unwrap();
    let definition = service.definitions("file:///View.xaml", reference).unwrap();
    assert_eq!(definition.len(), 1);
    assert_eq!(
        service
            .index("file:///View.xaml")
            .unwrap()
            .span(definition[0].range)
            .unwrap()
            .start,
        source.match_indices("Shared").nth(1).unwrap().0
    );
}

#[test]
fn standalone_cr_and_inserted_lf_merge_without_stale_line_starts() {
    let mut index = LineIndex::new("a\rb\r\n🦀\n");
    assert_eq!(index.line_count(), 4);
    assert_eq!(
        index
            .offset(Position {
                line: 1,
                character: 0
            })
            .unwrap(),
        2
    );
    index
        .apply(&TextChange {
            range: Some(Range {
                start: Position {
                    line: 1,
                    character: 0,
                },
                end: Position {
                    line: 1,
                    character: 0,
                },
            }),
            range_length: None,
            text: "\n".into(),
        })
        .unwrap();
    assert_eq!(index.source(), "a\r\nb\r\n🦀\n");
    assert_eq!(index.line_count(), 4);
    assert_eq!(
        index
            .offset(Position {
                line: 1,
                character: 0
            })
            .unwrap(),
        3
    );
}

#[test]
fn incremental_line_index_matches_full_rescan_across_boundary_edits() {
    let mut index = LineIndex::new("α\r\n🦀\rb\nend");
    let mut seed = 31u64;
    for iteration in 0..250 {
        let mut positions: Vec<_> = index
            .source()
            .char_indices()
            .map(|(offset, _)| index.position(offset).unwrap())
            .collect();
        positions.push(index.position(index.source().len()).unwrap());
        positions.sort();
        positions.dedup();
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let left = positions[seed as usize % positions.len()];
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let right = positions[seed as usize % positions.len()];
        let range = Range {
            start: left.min(right),
            end: left.max(right),
        };
        let text = ["x", "\r", "\n", "\r\n", "界🦀", "", "first\r\nlast"][iteration % 7];
        index
            .apply(&TextChange {
                range: Some(range),
                range_length: None,
                text: text.into(),
            })
            .unwrap();
        let fresh = LineIndex::new(index.source());
        assert_eq!(
            index.line_count(),
            fresh.line_count(),
            "iteration {iteration}: {:?}",
            index.source()
        );
        for offset in index
            .source()
            .char_indices()
            .map(|(offset, _)| offset)
            .chain(std::iter::once(index.source().len()))
        {
            let position = fresh.position(offset).unwrap();
            assert_eq!(index.position(offset).unwrap(), position);
            assert_eq!(index.offset(position), fresh.offset(position));
        }
    }
}

#[test]
fn signatures_follow_nested_extensions_and_stop_after_the_closing_brace() {
    let source =
        "<Button Content=\"{Binding Path=Name, Converter={StaticResource Key}, Mode=OneWay}\"/>";
    let service = service(source);
    let nested = service
        .signature_help("file:///View.xaml", source.find("Key}").unwrap() + 3)
        .unwrap()
        .unwrap();
    assert!(nested.signatures[0].label.starts_with("{StaticResource"));
    let parent = service
        .signature_help("file:///View.xaml", source.find("OneWay}").unwrap() + 6)
        .unwrap()
        .unwrap();
    assert_eq!(parent.active_parameter, 4);
    assert!(service
        .signature_help("file:///View.xaml", source.find("OneWay}").unwrap() + 7)
        .unwrap()
        .is_none());
    assert!(service
        .signature_help("file:///View.xaml", source.len())
        .unwrap()
        .is_none());
}

#[test]
fn binding_assignments_still_validate_property_spelling() {
    let service = service("<Button Widht='{Binding Size}'/>");
    assert!(service
        .diagnostics("file:///View.xaml")
        .unwrap()
        .iter()
        .any(|diagnostic| diagnostic.code == "unknown-property"));
}

#[test]
fn encoded_identifier_references_share_decoded_names_and_exact_lexical_ranges() {
    let source = r##"<Grid><Button Name="A&#66;"/><Button TargetName="A&#66;" Content="{Binding #A&#66;.Text}" Tag="{Binding ElementName=A&#66;}"/><TextBlock Text="{x:Reference A&#66;}" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"/></Grid>"##;
    let document = ferroui_designer_core::Document::new(source).unwrap();
    let occurrences = name_occurrences(&document);
    assert_eq!(occurrences.len(), 5);
    for occurrence in &occurrences {
        assert_eq!(occurrence.name, "AB");
        assert_eq!(&source[occurrence.span.range()], "A&#66;");
    }
    let rewritten = rewrite_names(source, &BTreeMap::from([("AB".into(), "Copy".into())])).unwrap();
    assert_eq!(rewritten.matches("Copy").count(), 5);
    assert!(!rewritten.contains("A&#66;"));
}
