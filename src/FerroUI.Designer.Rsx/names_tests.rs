use crate::project_rust;
use ferroui_designer_core::TextPatch;

#[test]
fn a_name_change_patches_style_and_state_references_in_the_selected_view() {
    let source = r###"let sibling = rsx! { TextBlock { name: "Card", text: "#Card" } };
let dashboard = rsx! {
    StackPanel {
        styles: [ferroui_view::style(r#"Border#Card[Tag=#Card], Border#Card/template/Border#Card"#)
            .set(background(), read_palette())
            .nested(ferroui_view::style("^:not(#Other)")),
            ferroui_view::style(":is(Border)#Card").nested(ferroui_view::style("^:not(#Card)"))],
        visual_states: [ferroui_view::visual_state_group("Motion")
            .state(ferroui_view::visual_state("Ready")
                .set_on(r#"Card"#, width(), calculate_width(model)))],
        Border {
            name: /* keep the anchor */ r#"Card"#,
            TextBlock {
                styles: [ferroui_view::style("Border#C\u{61}rd > TextBlock[Tag=\x23Card]"),
                    ferroui_view::style("#Card")],
                text: label(model)
            }
        }
        for entry in rows { TextBlock { text: entry.title } }
    }
};"###
        .replace('\n', "\r\n");
    let caret = source.find("let dashboard").unwrap() + "let dashboard = rsx!".len();
    let preview = project_rust(&source, caret).unwrap();
    let patches = preview.edit_control_names(&[1], Some("Résumé")).unwrap();
    let after = TextPatch::apply_all(&source, patches).unwrap();
    let expected = source
        .replace("Border#Card[Tag=#Card]", "Border#Résumé[Tag=#Card]")
        .replace("Border#Card/template/", "Border#Résumé/template/")
        .replace(":is(Border)#Card", ":is(Border)#Résumé")
        .replace("^:not(#Card)", "^:not(#Résumé)")
        .replace(
            "name: /* keep the anchor */ r#\"Card\"#",
            "name: /* keep the anchor */ r#\"Résumé\"#",
        )
        .replace("set_on(r#\"Card\"#", "set_on(r#\"Résumé\"#")
        .replace("Border#C\\u{61}rd >", "Border#Résumé >");
    assert_eq!(after, expected);
    let round_trip = project_rust(&after, caret).unwrap();
    let patch = round_trip
        .edit_control_names(&[1], Some("FinalCard"))
        .unwrap();
    assert_eq!(patch.len(), 7);
}

#[test]
fn rename_rejects_an_existing_name_in_a_dynamic_branch_and_an_unresolved_target() {
    let source = r##"rsx! {
        StackPanel {
            styles: [ferroui_view::style("#Pending")],
            TextBlock { name: "Current" }
            if show_archive { TextBlock { name: "Archived" } }
        }
    }"##;
    let view = project_rust(source, 0).unwrap();
    assert!(view
        .edit_control_names(&[1], Some("Archived"))
        .unwrap_err()
        .contains("already uses"));
    assert!(view
        .edit_control_names(&[1], Some("Pending"))
        .unwrap_err()
        .contains("would capture"));
    assert!(view.edit_control_names(&[1], Some("with spaces")).is_err());
}

#[test]
fn an_opaque_style_blocks_only_a_name_it_could_refer_to() {
    let source = r#"rsx! { StackPanel {
        Border { name: "A", styles: build_styles(model), }
        Border { name: "B", }
    } }"#;
    let view = project_rust(source, 0).unwrap();
    let patches = view.edit_control_names(&[2], Some("C")).unwrap();
    assert_eq!(
        TextPatch::apply_all(source, patches).unwrap(),
        source.replace("\"B\"", "\"C\"")
    );
    assert!(view
        .edit_control_names(&[1], Some("C"))
        .unwrap_err()
        .contains("computed style selector"));
}

#[test]
fn resetting_a_name_protects_its_references_and_retains_comments() {
    let source = r##"rsx! { StackPanel {
        styles: [ferroui_view::style("#Current")],
        TextBlock { name: "Current", text: title, }
        TextBlock { name: /* note */ "Unused", text: "Leave this text", }
    } }"##;
    let view = project_rust(source, 0).unwrap();
    assert!(view
        .edit_control_names(&[1], None)
        .unwrap_err()
        .contains("is referenced"));
    let after = TextPatch::apply_all(source, view.edit_control_names(&[2], None).unwrap()).unwrap();
    assert!(after.contains("/* note */"));
    assert!(!after.contains("\"Unused\""));
    assert!(project_rust(&after, 0).is_ok());
}
