use crate::*;
use serde_json::json;
use std::collections::BTreeMap;

// In-memory language fixtures use virtual URIs; filesystem tests below use file_uri.
const NS: &str =
    "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";
fn service(source: &str) -> LanguageService {
    let mut service = LanguageService::default();
    service.open("untitled:/View.xaml", source, 1).unwrap();
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
fn sparse_utf16_index_matches_a_character_walk_after_repeated_edits() {
    fn check(index: &LineIndex) {
        let source = index.source();
        let mut starts = vec![0];
        for (offset, byte) in source.bytes().enumerate() {
            if byte == b'\n' || byte == b'\r' && source.as_bytes().get(offset + 1) != Some(&b'\n') {
                starts.push(offset + 1);
            }
        }
        assert_eq!(index.line_count(), starts.len());
        for (line, start) in starts.iter().copied().enumerate() {
            let end = starts.get(line + 1).copied().unwrap_or(source.len());
            let text = source[start..end].trim_end_matches(['\r', '\n']);
            let mut units = 0;
            for (offset, ch) in text.char_indices() {
                let position = Position {
                    line: line as u32,
                    character: units,
                };
                assert_eq!(index.position(start + offset).unwrap(), position);
                assert_eq!(index.offset(position).unwrap(), start + offset);
                if ch.len_utf16() == 2 {
                    assert!(index
                        .offset(Position {
                            character: units + 1,
                            ..position
                        })
                        .is_err());
                }
                units += ch.len_utf16() as u32;
            }
            let position = Position {
                line: line as u32,
                character: units,
            };
            assert_eq!(index.offset(position).unwrap(), start + text.len());
            assert_eq!(index.position(start + text.len()).unwrap(), position);
            assert!(index
                .offset(Position {
                    character: units + 1,
                    ..position
                })
                .is_err());
        }
    }
    let mut index = LineIndex::new("ASCII α🦀\r\n漢字\r🙂\nlast");
    let replacements = ["", "x", "🦀", "é漢", "\r", "\n", "\r\n", "🙂\ra\nβ"];
    let mut random = 0x91a2_b3c4_u64;
    for _ in 0..512 {
        check(&index);
        let positions: Vec<_> = (0..=index.source().len())
            .filter(|offset| index.source().is_char_boundary(*offset))
            .filter_map(|offset| {
                let position = index.position(offset).unwrap();
                (index.offset(position).unwrap() == offset).then_some((offset, position))
            })
            .collect();
        let mut next = |bound: usize| {
            random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
            (random >> 32) as usize % bound
        };
        let mut start = next(positions.len());
        let mut end = next(positions.len());
        if start > end {
            std::mem::swap(&mut start, &mut end);
        }
        let text = replacements[next(replacements.len())];
        let range_length = index.source()[positions[start].0..positions[end].0]
            .encode_utf16()
            .count() as u32;
        let mut expected = index.source().to_owned();
        expected.replace_range(positions[start].0..positions[end].0, text);
        index
            .apply(&TextChange {
                range: Some(Range {
                    start: positions[start].1,
                    end: positions[end].1,
                }),
                range_length: Some(range_length),
                text: text.into(),
            })
            .unwrap();
        assert_eq!(index.source(), expected);
    }
    check(&index);
    assert!(index
        .range(ferroui_designer_core::SourceSpan::new(1, 0))
        .is_err());
}

#[test]
fn large_text_edit_batches_preserve_order_and_validate_before_emission() {
    let source = "α🦀x".repeat(10_000);
    let edits: Vec<_> = (0..10_000)
        .rev()
        .map(|i| TextEdit {
            range: Range {
                start: Position {
                    line: 0,
                    character: i * 4 + 1,
                },
                end: Position {
                    line: 0,
                    character: i * 4 + 3,
                },
            },
            new_text: "漢".into(),
        })
        .collect();
    assert_eq!(
        apply_text_edits(&source, &edits).unwrap(),
        "α漢x".repeat(10_000)
    );
    let oversized = TextEdit {
        range: Range::default(),
        new_text: "x".repeat(16 * 1024 * 1024),
    };
    assert!(apply_text_edits("x", &[oversized]).is_err());
}

#[test]
fn unicode_index_splices_partial_blocks_and_shifts_distant_suffixes() {
    let mut index = LineIndex::new(&"aα🦀\r\n".repeat(1_000));
    for (line, count, text) in [
        (0, 0, "界"),
        (100, 4, "🙂"),
        (3, 2, "\n漢\r"),
        (999, 0, "🦀"),
    ] {
        index
            .apply(&TextChange {
                range: Some(Range {
                    start: Position { line, character: 0 },
                    end: Position {
                        line,
                        character: count,
                    },
                }),
                range_length: None,
                text: text.into(),
            })
            .unwrap();
        let fresh = LineIndex::new(index.source());
        for offset in 0..=index.source().len() {
            if index.source().is_char_boundary(offset) {
                let expected = fresh.position(offset).unwrap();
                assert_eq!(index.position(offset).unwrap(), expected);
                assert_eq!(index.offset(expected), fresh.offset(expected));
            }
        }
    }
    assert!(index
        .offset(Position {
            line: 1,
            character: u32::MAX
        })
        .is_err());
}

#[test]
fn semantic_tokens_split_cr_crlf_and_lf_without_spanning_lines() {
    let source = "<Grid><!-- α\r🦀\r\n漢\nend --><Button Content='one\rtwo'/></Grid>";
    let service = service(source);
    let index = service.index("untitled:/View.xaml").unwrap();
    let tokens = service.semantic_tokens("untitled:/View.xaml").unwrap();
    assert!(tokens
        .iter()
        .any(|token| token.line == 1 && token.length == 2));
    for token in tokens {
        let start = index
            .offset(Position {
                line: token.line,
                character: token.start,
            })
            .unwrap();
        let end = index
            .offset(Position {
                line: token.line,
                character: token.start + token.length,
            })
            .unwrap();
        assert!(!source[start..end].contains(['\r', '\n']));
    }
}
#[test]
fn incremental_changes_are_atomic_versioned_and_retain_invalid_drafts() {
    let mut service = service("<Button Content='🦀'/>");
    let source = service
        .document("untitled:/View.xaml")
        .unwrap()
        .source()
        .to_owned();
    let offset = source.find('🦀').unwrap();
    let position = service
        .index("untitled:/View.xaml")
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
    assert!(service.change("untitled:/View.xaml", 2, &changes).is_err());
    assert_eq!(service.version("untitled:/View.xaml"), Some(1));
    assert_eq!(
        service.document("untitled:/View.xaml").unwrap().source(),
        source
    );
    service.replace("untitled:/View.xaml", "<Bu", 2).unwrap();
    assert!(!service.document("untitled:/View.xaml").unwrap().is_valid());
    assert_eq!(
        service.diagnostics("untitled:/View.xaml").unwrap()[0].code,
        "xml"
    );
    assert!(service
        .replace("untitled:/View.xaml", "<Grid/>", 2)
        .is_err());
    assert!(service
        .completions("untitled:/View.xaml", 3)
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
    let completions = service.completions("untitled:/View.xaml", 9).unwrap();
    assert!(completions
        .iter()
        .any(|c| c.label == "Width" && c.insert_text == "Width=\"\""));
    let source = "<StackPanel Orientation='H'/>";
    let service = self::service(source);
    let completions = service
        .completions("untitled:/View.xaml", source.find("H'").unwrap() + 1)
        .unwrap();
    assert!(completions.iter().any(|c| c.label == "Horizontal"));
}
#[test]
fn native_animation_programs_and_transition_objects_have_complete_schema_diagnostics() {
    let source = format!(
        r#"<StackPanel {NS}>
  <StackPanel.Styles>
    <Style Selector="Border">
      <Style.Animations>
        <Animation Duration="0:0:2" Delay="0:0:0.2" DelayBetweenIterations="0:0:0.1"
                   IterationCount="2" PlaybackDirection="Alternate" PlaybackBehavior="Always"
                   FillMode="Both" SpeedRatio="1.5">
          <Animation.Easing><SplineEasing X1="0.2" Y1="0" X2="0.8" Y2="1"/></Animation.Easing>
          <KeyFrame Cue="0%" KeySpline="0.2,0,0.8,1"><Setter Property="Opacity" Value="0"/></KeyFrame>
          <KeyFrame KeyTime="0:0:2"><Setter Property="Opacity" Value="1"/></KeyFrame>
        </Animation>
      </Style.Animations>
    </Style>
  </StackPanel.Styles>
  <Border Classes="accent">
    <Border.Transitions>
      <Transitions><DoubleTransition Property="Opacity" Duration="0:0:0.2" Delay="0:0:0.1" Easing="CubicEaseOut"/></Transitions>
    </Border.Transitions>
  </Border>
</StackPanel>"#
    );
    let service = service(&source);
    let diagnostics = service.diagnostics("untitled:/View.xaml").unwrap();
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
    let invalid =
        self::service(&source.replace("PlaybackBehavior=\"Always\"", "PlaybackBehavior=\"Never\""));
    assert!(invalid
        .diagnostics("untitled:/View.xaml")
        .unwrap()
        .iter()
        .any(|diagnostic| diagnostic.code == "enum-value"));
}
#[test]
fn animation_completion_uses_macro_properties_and_plain_markup_metadata() {
    let schema = Schema::ferroui();
    let properties = schema.properties("Animation", Some("using:FerroUI.Animation"));
    for name in [
        "Duration",
        "Delay",
        "DelayBetweenIterations",
        "IterationCount",
        "PlaybackDirection",
        "PlaybackBehavior",
        "FillMode",
        "Easing",
        "SpeedRatio",
    ] {
        assert!(properties.contains_key(name), "missing Animation.{name}");
    }
    assert!(schema
        .properties("DoubleTransition", None)
        .contains_key("Duration"));
    assert!(schema.type_info("Setter", None).unwrap().constructible);
    assert!(
        schema
            .type_info("SplineEasing", None)
            .unwrap()
            .constructible
    );
    let style = schema.properties("Style", None);
    assert!(style.contains_key("Animations"));
    assert!(!style.contains_key("get") && !style.contains_key("try_set"));

    let source = "<Animation PlaybackBehavior='A'/>";
    let service = service(source);
    let completions = service
        .completions("untitled:/View.xaml", source.find("A'").unwrap() + 1)
        .unwrap();
    assert!(completions.iter().any(|item| item.label == "Always"));
    let source = "<Animation DelayB";
    let service = self::service(source);
    let completions = service
        .completions("untitled:/View.xaml", source.len())
        .unwrap();
    assert!(completions
        .iter()
        .any(|item| item.label == "DelayBetweenIterations"));
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
        .diagnostics("untitled:/View.xaml")
        .unwrap()
        .iter()
        .any(|d| d.code == "enum-value"));
    let source = service.document("untitled:/View.xaml").unwrap().source();
    let hover = service
        .hover("untitled:/View.xaml", source.find("Mode").unwrap())
        .unwrap()
        .unwrap();
    assert!(hover.contents.contains("Pick a mode"));
}
#[test]
fn semantic_diagnostics_cover_types_properties_values_and_markup() {
    let source=format!("<StackPanel {NS} Orientation='Diagonal'><Button Opacity='bad' Nope='1' Content='{{Binding'/><Missing/></StackPanel>");
    let service = service(&source);
    let codes: Vec<_> = service
        .diagnostics("untitled:/View.xaml")
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
        .rename("untitled:/View.xaml", offset, "Renamed")
        .unwrap();
    assert_eq!(edit.document_changes.len(), 1);
    assert_eq!(edit.document_changes[0].edits.len(), 6);
    service.apply_workspace_edit(&edit).unwrap();
    let renamed = service.document("untitled:/View.xaml").unwrap().source();
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
        .open("untitled:/Resources.xaml", &resources, 3)
        .unwrap();
    service.open("untitled:/View.xaml", &usage, 7).unwrap();
    let offset = usage.find("Accent").unwrap();
    let definition = service.definitions("untitled:/View.xaml", offset).unwrap();
    assert_eq!(definition.len(), 1);
    assert_eq!(definition[0].uri, "untitled:/Resources.xaml");
    assert_eq!(
        service
            .references("untitled:/View.xaml", offset)
            .unwrap()
            .len(),
        2
    );
    let edit = service
        .rename("untitled:/View.xaml", offset, "Primary")
        .unwrap();
    assert_eq!(edit.document_changes.len(), 2);
    service.apply_workspace_edit(&edit).unwrap();
    assert!(service
        .document("untitled:/Resources.xaml")
        .unwrap()
        .source()
        .contains("x:Key='Primary'"));
    assert!(service
        .document("untitled:/View.xaml")
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
        .definitions("untitled:/View.xaml", references[0])
        .unwrap();
    let outer = service
        .definitions("untitled:/View.xaml", references[1])
        .unwrap();
    assert_ne!(inner, outer);
    assert!(!service
        .diagnostics("untitled:/View.xaml")
        .unwrap()
        .iter()
        .any(|d| d.code == "duplicate-symbol"));
}
#[test]
fn markup_signature_reports_named_parameter() {
    let source = "<Button Content='{Binding Path=Title, ElementName=Target}'/>";
    let service = service(source);
    let help = service
        .signature_help("untitled:/View.xaml", source.find("Target").unwrap() + 6)
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
        .format("untitled:/View.xaml", FormattingOptions::default())
        .unwrap();
    let formatted = apply_text_edits(source, &edits).unwrap();
    assert!(formatted.contains("\n    <!-- keep -->"));
    assert!(formatted.contains("Text = 'x'"));
    assert!(formatted.contains("hello <Run Text='world'/> !"));
    assert!(formatted.contains("preserve'>  <Button/>  </Border>"));
    service
        .replace("untitled:/View.xaml", &formatted, 2)
        .unwrap();
    assert!(service
        .format("untitled:/View.xaml", FormattingOptions::default())
        .unwrap()
        .is_empty());
}
#[test]
fn symbols_folding_and_tokens_have_valid_utf16_positions() {
    let source = format!("<Grid {NS}>\n <!-- 🦀 -->\n <Button Name='Action' Width='10'/>\n</Grid>");
    let service = service(&source);
    assert_eq!(
        service.symbols("untitled:/View.xaml").unwrap()[0].children[0].name,
        "Action"
    );
    assert!(!service.folding("untitled:/View.xaml").unwrap().is_empty());
    let tokens = service.semantic_tokens("untitled:/View.xaml").unwrap();
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
    service
        .open("untitled:/Other.xaml", "<Border/>", 3)
        .unwrap();
    let edit = WorkspaceEdit {
        document_changes: vec![
            DocumentEdits {
                uri: "untitled:/View.xaml".into(),
                version: 1,
                edits: vec![TextEdit {
                    range: service
                        .index("untitled:/View.xaml")
                        .unwrap()
                        .range(ferroui_designer_core::SourceSpan::new(0, 7))
                        .unwrap(),
                    new_text: "<Button/>".into(),
                }],
            },
            DocumentEdits {
                uri: "untitled:/Other.xaml".into(),
                version: 2,
                edits: vec![],
            },
        ],
    };
    assert!(service.apply_workspace_edit(&edit).is_err());
    assert_eq!(
        service.document("untitled:/View.xaml").unwrap().source(),
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
    let published=server.handle(json!({"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"untitled:/V.xaml","languageId":"xaml","version":1,"text":"<Bu"}}}));
    assert_eq!(published[0]["method"], "textDocument/publishDiagnostics");
    let completion=server.handle(json!({"jsonrpc":"2.0","id":2,"method":"textDocument/completion","params":{"textDocument":{"uri":"untitled:/V.xaml"},"position":{"line":0,"character":3}}}));
    assert!(completion[0]["result"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["label"] == "Button"));
    server.handle(json!({"jsonrpc":"2.0","method":"textDocument/didChange","params":{"textDocument":{"uri":"untitled:/V.xaml","version":2},"contentChanges":[{"text":"<Button/>"}]}}));
    assert!(server
        .service
        .document("untitled:/V.xaml")
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
    let references = service.references("untitled:/View.xaml", first).unwrap();
    assert_eq!(references.len(), 2);
    let edit = service
        .rename("untitled:/View.xaml", first, "Renamed")
        .unwrap();
    service.apply_workspace_edit(&edit).unwrap();
    let changed = service.document("untitled:/View.xaml").unwrap().source();
    assert!(changed.contains("StaticResource Renamed}"));
    assert!(changed.contains("ResourceKey=Second}"));
    assert!(
        !name_occurrences(service.document("untitled:/View.xaml").unwrap())
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
        service.references("untitled:/View.xaml", at).unwrap().len(),
        3
    );
    let edit = service
        .rename("untitled:/View.xaml", at, "Hot 'pink' & 🦀")
        .unwrap();
    service.apply_workspace_edit(&edit).unwrap();
    assert!(service.document("untitled:/View.xaml").unwrap().is_valid());
    assert_eq!(
        service.references("untitled:/View.xaml", at).unwrap().len(),
        3
    );
    assert!(!service
        .diagnostics("untitled:/View.xaml")
        .unwrap()
        .iter()
        .any(|diagnostic| matches!(diagnostic.code.as_str(), "unresolved-reference" | "markup")));
}

#[test]
fn rename_does_not_guess_which_ambiguous_resource_a_reference_uses() {
    let mut service = service("<Button Background='{StaticResource Shared}'/>");
    let first = format!("<ResourceDictionary {NS}><Color x:Key='Shared'/></ResourceDictionary>");
    service.open("untitled:/A.xaml", &first, 1).unwrap();
    service.open("untitled:/B.xaml", &first, 1).unwrap();
    assert_eq!(
        service
            .definitions("untitled:/View.xaml", 37)
            .unwrap()
            .len(),
        2
    );
    assert!(service
        .rename("untitled:/View.xaml", 37, "Different")
        .is_err());
    let edit = service
        .rename(
            "untitled:/A.xaml",
            first.find("Shared").unwrap(),
            "Different",
        )
        .unwrap();
    assert_eq!(edit.document_changes.len(), 1);
    assert_eq!(edit.document_changes[0].uri, "untitled:/A.xaml");
}

#[test]
fn imported_resources_resolve_in_merge_order_and_cycles_terminate() {
    let first = format!("<ResourceDictionary {NS}><Color x:Key='Shared'/></ResourceDictionary>");
    let view = "<Grid><Grid.Resources><ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceInclude Source='A.xaml'/><ResourceInclude Source='B.xaml'/></ResourceDictionary.MergedDictionaries></ResourceDictionary></Grid.Resources><Button Background='{StaticResource Shared}'/></Grid>";
    let mut service = service(view);
    service.open("untitled:/A.xaml", &first, 1).unwrap();
    service.open("untitled:/B.xaml", &first, 1).unwrap();
    let definitions = service
        .definitions("untitled:/View.xaml", view.find("Shared").unwrap())
        .unwrap();
    assert_eq!(definitions.len(), 1);
    assert_eq!(definitions[0].uri, "untitled:/B.xaml");
    service.replace("untitled:/B.xaml", "<ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceInclude Source='A.xaml'/></ResourceDictionary.MergedDictionaries></ResourceDictionary>", 2).unwrap();
    service.replace("untitled:/A.xaml", "<ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceInclude Source='B.xaml'/></ResourceDictionary.MergedDictionaries></ResourceDictionary>", 2).unwrap();
    assert!(service
        .definitions("untitled:/View.xaml", view.find("Shared").unwrap())
        .unwrap()
        .is_empty());
}

#[test]
fn private_control_resources_are_not_visible_in_other_documents() {
    let source = format!("<Grid {NS}><Border><Border.Resources><Color x:Key='Private'/></Border.Resources></Border></Grid>");
    let mut service = service("<Button Background='{StaticResource Private}'/>");
    service.open("untitled:/Other.xaml", &source, 1).unwrap();
    assert!(service
        .definitions("untitled:/View.xaml", 37)
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
            "untitled:/View.xaml",
            service
                .document("untitled:/View.xaml")
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
            "untitled:/View.xaml",
            "<c:Widget xmlns:c='using:Custom' Wid",
            2,
        )
        .unwrap();
    assert!(service
        .completions("untitled:/View.xaml", 36)
        .unwrap()
        .iter()
        .any(|item| item.label == "Width"));
}

fn binding_schema(service: &mut LanguageService) -> Location {
    let location = Location {
        uri: "untitled:/ViewModel.rs".into(),
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
        .completions("untitled:/View.xaml", at)
        .unwrap()
        .into_iter()
        .find(|item| item.label == "Title")
        .unwrap();
    assert_eq!(&source[completion.span.range()], "Ti");
    assert!(service
        .hover("untitled:/View.xaml", at)
        .unwrap()
        .unwrap()
        .contents
        .contains("Customer display name"));
    assert_eq!(
        service.definitions("untitled:/View.xaml", at).unwrap(),
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
            "untitled:/View.xaml",
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
            "untitled:/View.xaml",
            "<Grid><ResourceInclude Source='A.xaml'/><ResourceInclude Source='B.xaml'/></Grid>",
            1,
        )
        .unwrap();
    let reply = server.handle(json!({"jsonrpc":"2.0","id":2,"method":"workspace/willRenameFiles","params":{"files":[{"oldUri":"untitled:/A.xaml","newUri":"untitled:/NewA.xaml"},{"oldUri":"untitled:/B.xaml","newUri":"untitled:/NewB.xaml"}]}}));
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
    let directory = directory.canonicalize().unwrap();
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

#[cfg(unix)]
#[test]
fn symlink_document_aliases_share_versions_symbols_and_project_snapshots() {
    use ferroui_designer_project::{file_uri, Solution};
    let directory =
        std::env::temp_dir().join(format!("ferroui-language-alias-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    let directory = directory.canonicalize().unwrap();
    let alias_root = directory.join("alias");
    std::os::unix::fs::symlink(&directory, &alias_root).unwrap();
    let source =
        "<Grid><Button Name='Target'/><TextBlock Text='{Binding ElementName=Target}'/></Grid>";
    std::fs::write(directory.join("View.xaml"), source).unwrap();
    let uri = file_uri(directory.join("View.xaml")).unwrap();
    let alias = file_uri(alias_root.join("View.xaml")).unwrap();
    let mut solution = Solution::discover(&directory).unwrap();
    // Descriptors can retain the user's root spelling instead of discovery's
    // canonical path, as on macOS /var versus /private/var.
    solution.root = alias_root.clone();
    for project in &mut solution.projects {
        project.root = alias_root.clone();
        project.manifest_path = alias_root.join("Cargo.toml");
        for entry in &mut project.documents {
            entry.path = alias_root.join(entry.path.file_name().unwrap());
            entry.uri = file_uri(&entry.path).unwrap();
        }
    }
    let mut service = LanguageService::default();
    service.index_solution(&solution).unwrap();
    service.open(&alias, source, 1).unwrap();
    assert_eq!(service.uris().collect::<Vec<_>>(), vec![uri.as_str()]);
    assert_eq!(service.version(&uri), Some(1));
    assert!(!service
        .diagnostics(&alias)
        .unwrap()
        .iter()
        .any(|d| d.code == "unresolved-reference"));
    let offset = source.rfind("Target").unwrap();
    assert_eq!(service.definitions(&alias, offset).unwrap()[0].uri, uri);
    assert_eq!(service.references(&alias, offset).unwrap().len(), 2);
    let edit = service.rename(&alias, offset, "Renamed").unwrap();
    assert_eq!(edit.document_changes[0].uri, uri);
    let mut duplicate = edit.clone();
    duplicate.document_changes.push(DocumentEdits {
        uri: alias.clone(),
        ..edit.document_changes[0].clone()
    });
    assert!(service.apply_workspace_edit(&duplicate).is_err());
    assert_eq!(service.version(&alias), Some(1));
    service.apply_workspace_edit(&edit).unwrap();
    assert!(service
        .document(&alias)
        .unwrap()
        .source()
        .contains("Renamed"));
    service.mark_saved(&alias, None).unwrap();
    service.replace(&alias, "<Grid/>", 3).unwrap();
    service.close(&alias).unwrap();
    assert_eq!(service.document(&alias).unwrap().source(), source);
    assert_eq!(service.version(&alias), Some(-1));
    assert_eq!(service.remove_solution(&directory), 1);
    assert!(service.document(&uri).is_none());
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn file_rename_notifications_validate_whole_batches_and_support_swaps() {
    let mut server = lsp::LspServer::default();
    server.handle(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}));
    server
        .service
        .open("untitled:/A.xaml", "<Button/>", 1)
        .unwrap();
    server
        .service
        .open("untitled:/B.xaml", "<Border/>", 2)
        .unwrap();
    for files in [
        json!([{ "oldUri":"untitled:/A.xaml", "newUri":"untitled:/Moved.xaml" }, { "oldUri":"untitled:/B.xaml" }]),
        json!([{ "oldUri":"untitled:/A.xaml", "newUri":"untitled:/Moved.xaml" }, { "oldUri":"untitled:/B.xaml", "newUri":"untitled:/Moved.xaml" }]),
    ] {
        let messages = server.handle(
            json!({"jsonrpc":"2.0","method":"workspace/didRenameFiles","params":{"files":files}}),
        );
        assert_eq!(messages[0]["method"], "window/logMessage");
        assert!(server.service.document("untitled:/Moved.xaml").is_none());
        assert_eq!(server.service.version("untitled:/A.xaml"), Some(1));
        assert_eq!(server.service.version("untitled:/B.xaml"), Some(2));
    }
    server
        .service
        .rename_uris(&[
            ("untitled:/A.xaml".into(), "untitled:/B.xaml".into()),
            ("untitled:/B.xaml".into(), "untitled:/A.xaml".into()),
        ])
        .unwrap();
    assert_eq!(
        server
            .service
            .document("untitled:/A.xaml")
            .unwrap()
            .source(),
        "<Border/>"
    );
    assert_eq!(
        server
            .service
            .document("untitled:/B.xaml")
            .unwrap()
            .source(),
        "<Button/>"
    );
    server
        .service
        .rename_uri("untitled:/A.xaml", "untitled:/A.xaml")
        .unwrap();
}

#[test]
fn inline_merged_dictionaries_have_independent_keys_and_last_dictionary_wins() {
    let source = format!("<Grid {NS}><Grid.Resources><ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceDictionary><Color x:Key='Shared'/></ResourceDictionary><ResourceDictionary><Color x:Key='Shared'/></ResourceDictionary></ResourceDictionary.MergedDictionaries></ResourceDictionary></Grid.Resources><Button Background='{{StaticResource Shared}}'/></Grid>");
    let service = service(&source);
    assert!(!service
        .diagnostics("untitled:/View.xaml")
        .unwrap()
        .iter()
        .any(|diagnostic| diagnostic.code == "duplicate-symbol"));
    let reference = source.rfind("Shared").unwrap();
    let definition = service
        .definitions("untitled:/View.xaml", reference)
        .unwrap();
    assert_eq!(definition.len(), 1);
    assert_eq!(
        service
            .index("untitled:/View.xaml")
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
        .signature_help("untitled:/View.xaml", source.find("Key}").unwrap() + 3)
        .unwrap()
        .unwrap();
    assert!(nested.signatures[0].label.starts_with("{StaticResource"));
    let parent = service
        .signature_help("untitled:/View.xaml", source.find("OneWay}").unwrap() + 6)
        .unwrap()
        .unwrap();
    assert_eq!(parent.active_parameter, 4);
    assert!(service
        .signature_help("untitled:/View.xaml", source.find("OneWay}").unwrap() + 7)
        .unwrap()
        .is_none());
    assert!(service
        .signature_help("untitled:/View.xaml", source.len())
        .unwrap()
        .is_none());
}

#[test]
fn binding_assignments_still_validate_property_spelling() {
    let service = service("<Button Widht='{Binding Size}'/>");
    assert!(service
        .diagnostics("untitled:/View.xaml")
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
