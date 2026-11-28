use super::*;
use crate::tests::{laid_out_editor, test_scope};
use crate::EditorCommandKind;
use ferroui_edit_core::editing::TextSegmentReadOnlySectionProvider;

fn key_down(area: &TextArea, key: Key) -> bool {
    let mut event = KeyEventArgs::new();
    event.key = key;
    event.set_routed_event(Some(InputElement::key_down_event()));
    area.raise_event(&event);
    event.handled()
}

fn editor_with_pairs(text: &str) -> Ref<TextEditor> {
    use ferroui_textmate::{LanguageConfiguration, Registry, Theme};
    let editor = laid_out_editor(text);
    let mut registry = Registry::new();
    registry.add_grammar_json(r#"{
      "scopeName":"source.pair-test","patterns":[
        {"begin":"//","end":"$","name":"comment.line"},
        {"begin":"/\\*","end":"\\*/","name":"comment.block"},
        {"begin":"\"","end":"\"","name":"string.quoted.double"},
        {"begin":"/(?=[a-z])","end":"/","name":"constant.other.regex"}
      ]
    }"#).unwrap();
    let grammar = registry.load_grammar("source.pair-test").unwrap();
    let configuration = LanguageConfiguration::from_json(r#"{
      "brackets":[["(",")"],["[","]"]],
      "autoClosingPairs":[["(",")"],["[","]"],
        {"open":"\"","close":"\"","notIn":["string","comment"]},
        {"open":"/**","close":" */","notIn":["string"]}],
      "surroundingPairs":[["[","]"]],"autoCloseBefore":" )]"
    }"#).unwrap();
    editor.install_textmate(grammar, Theme::from_json("{}").unwrap())
        .set_language_configuration(Some(&configuration)).unwrap();
    editor.document().clear_undo_history().unwrap();
    editor
}

fn editor_with_embedded_rules(text: &str) -> Ref<TextEditor> {
    use ferroui_textmate::{GrammarConfiguration, LanguageConfiguration, Registry, Theme};

    let editor = laid_out_editor(text);
    let mut registry = Registry::new();
    registry.add_grammar_json(r#"{
      "scopeName":"text.test","patterns":[
        {"begin":"<script>","end":"</script>","contentName":"meta.embedded.js",
          "patterns":[
            {"begin":"/\\*","end":"\\*/","name":"comment.block.js"},
            {"begin":"\"","end":"\"","name":"string.quoted.js"}
          ]},
        {"begin":"<style>","end":"</style>","contentName":"meta.embedded.style"}
      ]
    }"#).unwrap();
    let grammar = registry.load_grammar("text.test").unwrap();
    let host = LanguageConfiguration::from_json(r#"{
      "brackets":[["(",")"],["[","]"]],
      "indentationRules":{
        "increaseIndentPattern":"^\\s*<(?:script|style)>$",
        "decreaseIndentPattern":"^\\s*</(?:script|style)>"
      },
      "onEnterRules":[{"beforeText":"^host$","action":{"appendText":"host: "}}]
    }"#).unwrap();
    let javascript = LanguageConfiguration::from_json(r#"{
      "brackets":[["(",")"],["{","}"]],
      "autoClosingPairs":[["(",")"],["{","}"],["end",";"]],
      "indentationRules":{
        "increaseIndentPattern":"(?:block|[({])$",
        "decreaseIndentPattern":"^\\s*(?:end\\b|[})])",
        "indentNextLinePattern":"^\\s*next$",
        "unIndentedLinePattern":"^\\s*ignore"
      },
      "onEnterRules":[
        {"beforeText":"^\\s*/\\*\\*$","afterText":"^\\s*\\*/$","action":{"indent":"indentOutdent","appendText":" * "}},
        {"beforeText":"^\\s*\"text\"$","action":{"appendText":"masked"}}
      ]
    }"#).unwrap();
    let mut grammar_configuration = GrammarConfiguration { initial_language: 1, ..Default::default() };
    grammar_configuration.embedded_languages.insert("meta.embedded.js".into(), 2);
    grammar_configuration.embedded_languages.insert("meta.embedded.style".into(), 3);
    editor.install_textmate(grammar, Theme::from_json("{}").unwrap())
        .set_language_configurations(grammar_configuration, Some(&host), [(2, &javascript)]).unwrap();
    editor.document().clear_undo_history().unwrap();
    editor
}

#[test]
fn enter_uses_the_language_scope_and_skips_brackets_inside_strings() {
    let _scope = test_scope();
    for (original, prefix, expected) in [
        ("host", "host", "host\nhost: "),
        ("<script>/**  */</script>", "<script>/**", "<script>/**\n * \n  */</script>"),
        ("<script> \"text)\" </script>", "<script> \"text)\"", "<script> \"text)\"\nmasked </script>"),
        ("<style>host </style>", "<style>host", "<style>host\n</style>"),
    ] {
        let editor = editor_with_embedded_rules(original);
        editor.set_caret_offset(prefix.encode_utf16().count()).unwrap();
        assert!(key_down(&editor.text_area(), Key::Return));
        assert_eq!(editor.text(), expected, "Enter in {original:?}");
    }
}

#[test]
fn embedded_brackets_and_pair_policies_are_local_to_the_caret_language() {
    let _scope = test_scope();
    for (original, prefix, typed, expected) in [
        ("<script>\n\n</script>\n)", "<script>\n", "(", "<script>\n()\n</script>\n)"),
        ("<script>\n\n)\n</script>", "<script>\n", "(", "<script>\n(\n)\n</script>"),
        ("<style>\n\n</style>", "<style>\n", "(", "<style>\n(\n</style>"),
        ("<script>😀 </script>", "<script>😀", "{", "<script>😀{} </script>"),
    ] {
        let editor = editor_with_embedded_rules(original);
        editor.set_caret_offset(prefix.encode_utf16().count()).unwrap();
        assert!(editor.text_area().perform_text_input(typed).unwrap());
        assert_eq!(editor.text(), expected, "typing in {original:?}");
    }

    for (typed, expected) in [
        ("(", "()\n<script>\n()\n</script>"),
        ("[", "[\n<script>\n[\n</script>"),
    ] {
        let editor = editor_with_embedded_rules("\n<script>\n\n</script>");
        editor.set_selection(Selection::from_segments([SelectionSegment::empty(0), SelectionSegment::empty(10)], 1)).unwrap();
        assert!(editor.text_area().perform_text_input(typed).unwrap());
        assert_eq!(editor.text(), expected);
        assert_eq!(editor.document().undo_count(), 1);
    }
}

#[test]
fn configured_indentation_handles_scopes_temporary_levels_and_brackets() {
    let _scope = test_scope();
    for (original, prefix, expected) in [
        ("<script>block </script>", "<script>block", "<script>block\n     </script>"),
        ("<style>block </style>", "<style>block", "<style>block\n</style>"),
        ("<script>  block\r\nignore block\r\n</script>", "<script>  block\r\nignore block", "<script>  block\r\nignore block\r\n    \r\n</script>"),
        ("<script>\r\n\tnext\r\n\t\tnext\r\n\t\t\twork\r\n</script>", "<script>\r\n\tnext\r\n\t\tnext\r\n\t\t\twork", "<script>\r\n\tnext\r\n\t\tnext\r\n\t\t\twork\r\n    \r\n</script>"),
        ("<script>\n    bodyend\n</script>", "<script>\n    body", "<script>\n    body\nend\n</script>"),
        ("<script>\n  \"(\n</script>", "<script>\n  \"(", "<script>\n  \"(\n  \n</script>"),
        ("<script>call()</script>", "<script>call(", "<script>call(\n    \n)</script>"),
    ] {
        let editor = editor_with_embedded_rules(original);
        let mut options = editor.options();
        options.convert_tabs_to_spaces = true;
        options.indentation_size = 4;
        editor.set_options(options);
        let caret = prefix.encode_utf16().count();
        editor.set_caret_offset(caret).unwrap();
        assert!(key_down(&editor.text_area(), Key::Return));
        assert_eq!(editor.text(), expected, "Enter in {original:?}");
        assert_eq!(editor.document().undo_count(), 1);
        assert!(editor.undo().unwrap());
        assert_eq!(editor.text(), original);
        assert_eq!(editor.caret_offset(), caret);
    }
}

#[test]
fn configured_indentation_replaces_only_unowned_leading_spaces() {
    let _scope = test_scope();
    let original = "<script>\r\n    block\r\n    😀 value\r\n</script>";
    let editor = editor_with_embedded_rules(original);
    let mut options = editor.options();
    options.convert_tabs_to_spaces = true;
    options.indentation_size = 4;
    editor.set_options(options);
    let first = "<script>\r\n    block".len();
    let last = first + "\r\n  ".len();
    let selection = Selection::from_segments([SelectionSegment::empty(first), SelectionSegment::empty(last)], 1);
    editor.set_selection(selection.clone()).unwrap();
    let document = editor.document();
    let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
    protected.add(last, 1).unwrap();
    editor.session().set_read_only_section_provider(protected.clone());
    assert!(matches!(editor.text_area().perform_text_input("\n"), Err(EditError::ReadOnly)));
    assert_eq!(editor.text(), original);
    assert!(!document.can_undo());

    protected.clear();
    assert!(key_down(&editor.text_area(), Key::Return));
    let prefix = "<script>\r\n    block\r\n        \r\n  \r\n  ";
    assert_eq!(editor.text(), format!("{prefix}      😀 value\r\n</script>"));
    assert_eq!(editor.caret_offset(), prefix.len());
    assert_eq!(editor.document().undo_count(), 1);
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), original);
    assert_eq!(editor.selection(), selection);
}

#[test]
fn reindent_command_tracks_nested_and_inline_language_scopes() {
    use ferroui_base::input::ICommand;

    let _scope = test_scope();
    let original = concat!(
        "<script>block\r\nwork😀\r\n next\r\nwork\r\n   ignore block\r\n    end</script>\r\n",
        " <style>\r\n unowned\r\n    </style>\r\nhost",
    );
    let editor = editor_with_embedded_rules(original);
    let mut options = editor.options();
    options.convert_tabs_to_spaces = true;
    options.indentation_size = 4;
    editor.session().set_options(options);
    let command = editor.command(EditorCommandKind::Reindent);
    assert!(command.can_execute(None));
    command.execute(None);
    let expected = concat!(
        "<script>block\r\n        work😀\r\n        next\r\n            work\r\n        ignore block\r\n    end</script>\r\n",
        "<style>\r\n unowned\r\n</style>\r\nhost",
    );
    assert_eq!(editor.text(), expected);
    assert_eq!(editor.document().undo_count(), 1);
    command.execute(None);
    assert_eq!(editor.document().undo_count(), 1, "reindent must be idempotent");
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), original);
}

#[test]
fn grammar_reindent_protects_selected_ranges_and_rejects_stale_plans() {
    let _scope = test_scope();
    let original = "<script>\r\n\tblock\r\nbad😀\r\n end\r\n   endpoint\r\n</script>";
    let editor = editor_with_embedded_rules(original);
    let start = "<script>\r\n\tblock\r\n".len();
    let end = start + "bad😀\r\n end\r\n".encode_utf16().count();
    let selection = Selection::from_segments([SelectionSegment::new(end, start)], 0);
    editor.set_selection(selection.clone()).unwrap();
    let document = editor.document();
    let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
    protected.add(end - " end\r\n".len(), 1).unwrap();
    editor.session().set_read_only_section_provider(protected.clone());
    assert!(matches!(editor.reindent_selection(), Err(EditError::ReadOnly)));
    assert_eq!(editor.text(), original);
    assert_eq!(editor.selection(), selection);
    assert_eq!(document.undo_count(), 0);

    protected.clear();
    editor.reindent_selection().unwrap();
    assert_eq!(editor.text(), "<script>\r\n\tblock\r\n\t\tbad😀\r\n\tend\r\n\tendpoint\r\n</script>");
    assert!(editor.selection().segments()[0].anchor > editor.selection().segments()[0].caret);
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), original);
    assert_eq!(editor.selection(), selection);

    let weak = editor.downgrade();
    let _switch = document.update_started(move || {
        weak.upgrade().unwrap().set_document(ferroui_edit_core::document::TextDocument::from_text("replacement"));
    });
    assert!(matches!(editor.reindent_selection(), Err(EditError::StaleEditPlan)));
    assert_eq!(editor.text(), "replacement");
    assert_eq!(document.text(), original);
}

#[test]
fn grammar_reindent_reaches_offscreen_lines_and_retries_after_a_tokenizer_limit() {
    use ferroui_textmate::TokenizeOptions;

    let _scope = test_scope();
    let original = format!("<script>\nblock\n{}const literal = \"\n    literal block\n\";\nend\n</script>", "work\n".repeat(160));
    let editor = editor_with_embedded_rules(&original);
    let installation = editor.textmate_installation().unwrap();
    installation.set_tokenize_options(TokenizeOptions { max_steps: 0, ..TokenizeOptions::default() });
    installation.invalidate_from(1);
    assert!(matches!(editor.reindent_selection(), Err(EditError::Indentation(_))));
    assert_eq!(editor.text(), original);
    assert_eq!(editor.document().undo_count(), 0);

    installation.set_tokenize_options(TokenizeOptions::default());
    editor.reindent_selection().unwrap();
    let expected = format!("<script>\n\tblock\n{}\t\tconst literal = \"\n    literal block\n\";\n\tend\n</script>", "\t\twork\n".repeat(160));
    assert_eq!(editor.text(), expected);
    assert_eq!(editor.document().undo_count(), 1);
}

#[test]
fn typed_closers_use_structural_indentation_in_one_undo_step() {
    let _scope = test_scope();
    let original = "<script>\r\n\tblock\r\n\t\twork😀\r\n\t\t\r\n</script>\r\n<script>\r\n\tblock\r\n\t\t\r\n</script>";
    let editor = editor_with_embedded_rules(original);
    let first = "<script>\r\n\tblock\r\n\t\twork😀\r\n\t\t".encode_utf16().count();
    let last = original.encode_utf16().count() - "\r\n</script>".len();
    let selection = Selection::from_segments([SelectionSegment::empty(first), SelectionSegment::empty(last)], 1);
    editor.set_selection(selection.clone()).unwrap();
    assert!(editor.text_area().perform_text_input("}").unwrap());
    let expected = "<script>\r\n\tblock\r\n\t\twork😀\r\n\t}\r\n</script>\r\n<script>\r\n\tblock\r\n\t}\r\n</script>";
    assert_eq!(editor.text(), expected);
    assert_eq!(editor.selection(), selection);
    assert_eq!(editor.document().undo_count(), 1);
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), original);
    assert_eq!(editor.selection(), selection);
    assert!(editor.redo().unwrap());
    assert_eq!(editor.text(), expected);
    assert_eq!(editor.selection(), selection);
}

#[test]
fn typed_indent_does_not_rewrite_protected_literal_or_intentional_prefixes() {
    let _scope = test_scope();
    let original = "<script>\n\tblock\n\t\t\n</script>";
    let prefix = "<script>\n\tblock\n\t\t";
    for from_input_method in [true, false] {
        let editor = editor_with_embedded_rules(original);
        editor.set_caret_offset(prefix.len()).unwrap();
        if !from_input_method {
            let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(editor.document()));
            protected.add(prefix.len() - 2, 1).unwrap();
            editor.session().set_read_only_section_provider(protected);
        }
        assert!(editor.text_area().perform_committed_input("}", from_input_method).unwrap());
        assert_eq!(editor.text(), format!("{prefix}}}\n</script>"));
        assert_eq!(editor.document().undo_count(), 1);
    }

    for (prefix, expected) in [
        ("<script>\n\tblock\n\t\t}", "<script>\n\tblock\n\t\t}}\n</script>"),
        ("<script>\n\tblock\n\"\n\t\t", "<script>\n\tblock\n\"\n\t\t}\n</script>"),
    ] {
        let editor = editor_with_embedded_rules(&format!("{prefix}\n</script>"));
        editor.set_caret_offset(prefix.len()).unwrap();
        assert!(editor.text_area().perform_text_input("}").unwrap());
        assert_eq!(editor.text(), expected);
    }
}

#[test]
fn typed_indent_preserves_pair_anchors_and_multi_character_undo_groups() {
    let _scope = test_scope();
    let one_shot = "<script>\n\tnext\n\t\t\n</script>";
    let editor = editor_with_embedded_rules(one_shot);
    editor.set_caret_offset("<script>\n\tnext\n\t\t".len()).unwrap();
    assert!(editor.text_area().perform_text_input("{").unwrap());
    assert_eq!(editor.text(), "<script>\n\tnext\n\t{}\n</script>");
    assert_eq!(editor.document().undo_count(), 1);
    assert!(key_down(&editor.text_area(), Key::Back));
    assert_eq!(editor.text(), "<script>\n\tnext\n\t\n</script>");

    let original = "<script>\r\n\tblock\r\n\t\ten\r\n</script>";
    let editor = editor_with_embedded_rules(original);
    let caret = "<script>\r\n\tblock\r\n\t\ten".len();
    editor.set_caret_offset(caret).unwrap();
    let anchor = editor.document().create_anchor(caret - 1).unwrap();
    assert!(editor.text_area().perform_text_input("d").unwrap());
    assert_eq!(editor.text(), "<script>\r\n\tblock\r\n\tend;\r\n</script>");
    assert_eq!(anchor.offset(), Ok(caret - 2));
    assert_eq!(editor.document().undo_count(), 2);
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), "<script>\r\n\tblock\r\n\tend\r\n</script>");
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), original);
}

#[test]
fn typed_outdent_replaces_a_directed_multiline_selection() {
    let _scope = test_scope();
    let prefix = "<script>\r\n\tblock\r\n\t\t";
    let selected = "😀work\r\n    replace";
    let suffix = ";\r\n</script>";
    let original = format!("{prefix}{selected}{suffix}");
    let editor = editor_with_embedded_rules(&original);
    let selection = Selection::from_segments([
        SelectionSegment::new(prefix.len() + selected.encode_utf16().count(), prefix.len()),
    ], 0);
    editor.set_selection(selection.clone()).unwrap();
    assert!(editor.text_area().perform_text_input("}").unwrap());
    let typed = "<script>\r\n\tblock\r\n\t}";
    assert_eq!(editor.text(), format!("{typed}{suffix}"));
    assert_eq!(editor.caret_offset(), typed.len());
    assert_eq!(editor.document().undo_count(), 1);
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), original);
    assert_eq!(editor.selection(), selection);
}

#[test]
fn generated_closers_follow_nested_typing_and_backspace_without_consuming_manual_text() {
    let _scope = test_scope();
    let editor = editor_with_pairs("");
    let area = editor.text_area();
    for token in ["(", "[", "]", ")"] {
        assert!(area.perform_text_input(token).unwrap());
    }
    assert_eq!(editor.text(), "([])");
    assert_eq!(editor.caret_offset(), 4);
    assert_eq!(editor.document().undo_count(), 2);
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), "()");

    let editor = editor_with_pairs("()");
    let area = editor.text_area();
    editor.set_caret_offset(1).unwrap();
    assert!(area.perform_text_input(")").unwrap());
    assert_eq!(editor.text(), "())");
    editor.set_caret_offset(0).unwrap();
    assert!(area.perform_text_input("[").unwrap());
    // The opening parenthesis is not allowed as following text in this package.
    assert_eq!(editor.text(), "[())");
    editor.set_caret_offset(editor.document().text_length()).unwrap();
    assert!(area.perform_text_input("[").unwrap());
    assert!(key_down(&area, Key::Back));
    assert_eq!(editor.text(), "[())");
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), "[())[]");
}

#[test]
fn bracket_typing_reuses_only_compatible_unmatched_closers_after_a_caret() {
    let _scope = test_scope();
    for (original, offset, typed, expected) in [
        ("\r\nemoji 😀 )", 0, "(", "(\r\nemoji 😀 )"),
        (")\n", 2, "(", ")\n()"),
        ("\n()", 0, "(", "()\n()"),
        ("\n(\n[\n)\n]", 0, "[", "[\n(\n[\n)\n]"),
        ("\n// )\n\")\"\n/regex)/\n/*\n)\n*/", 0, "(", "()\n// )\n\")\"\n/regex)/\n/*\n)\n*/"),
    ] {
        let editor = editor_with_pairs(original);
        editor.set_caret_offset(offset).unwrap();
        assert!(editor.text_area().perform_text_input(typed).unwrap());
        assert_eq!(editor.text(), expected, "typing into {original:?}");
        assert_eq!(editor.caret_offset(), offset + typed.encode_utf16().count());
    }

    let editor = editor_with_pairs("\n)\n");
    editor.set_selection(Selection::from_segments([SelectionSegment::empty(0), SelectionSegment::empty(3)], 1)).unwrap();
    assert!(editor.text_area().perform_text_input("(").unwrap());
    assert_eq!(editor.text(), "(\n)\n(");
    assert_eq!(editor.selection().segments(), &[SelectionSegment::empty(1), SelectionSegment::empty(5)]);
}

#[test]
fn visible_highlighting_schedules_bracket_balance_beyond_the_viewport() {
    let _scope = test_scope();
    let text = format!("\n{})", "word\n".repeat(720));
    let editor = editor_with_pairs(&text);
    let document = editor.document();
    let installation = editor.textmate_installation().unwrap();
    crate::tests::render_editor(&editor);
    assert!(!installation.cached_through(&document, document.line_count()));
    Dispatcher::ui_thread().run_jobs(None);
    assert!(installation.cached_through(&document, document.line_count()));

    editor.set_caret_offset(0).unwrap();
    assert!(editor.text_area().perform_text_input("(").unwrap());
    assert_eq!(editor.text(), format!("({text}"));
}

#[test]
fn pair_typing_checks_token_context_and_keeps_ime_text_literal() {
    let _scope = test_scope();
    for (original, caret, typed, expected, next_caret) in [
        ("// ", 3, "\"", "// \"", 4),
        ("/*\n ", 4, "\"", "/*\n \"", 5),
        ("\"hello ", 7, "\"", "\"hello \"", 8),
        ("const s = ", 10, "\"", "const s = \"\"", 11),
        ("/**", 3, "*", "/***", 4),
        ("/*", 2, "*", "/** */", 3),
    ] {
        let editor = editor_with_pairs(original);
        editor.set_caret_offset(caret).unwrap();
        editor.text_area().perform_text_input(typed).unwrap();
        assert_eq!(editor.text(), expected, "typing into {original:?}");
        assert_eq!(editor.caret_offset(), next_caret);
    }

    let editor = editor_with_pairs("");
    editor.text_area().perform_committed_input("(", true).unwrap();
    assert_eq!(editor.text(), "(");
}

#[test]
fn multi_character_pairs_have_separate_undo_steps_and_restore_utf16_carets() {
    let _scope = test_scope();
    let editor = editor_with_pairs("😀/*\r\nx/*");
    let original = Selection::from_segments([SelectionSegment::empty(4), SelectionSegment::empty(9)], 1);
    let after_opener = Selection::from_segments([SelectionSegment::empty(5), SelectionSegment::empty(11)], 1);
    let after_closer = Selection::from_segments([SelectionSegment::empty(5), SelectionSegment::empty(14)], 1);
    editor.set_selection(original.clone()).unwrap();
    assert!(editor.text_area().perform_text_input("*").unwrap());
    assert_eq!(editor.text(), "😀/** */\r\nx/** */");
    assert_eq!(editor.selection(), after_closer);
    assert_eq!(editor.document().undo_count(), 2);

    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), "😀/**\r\nx/**");
    assert_eq!(editor.selection(), after_opener);
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), "😀/*\r\nx/*");
    assert_eq!(editor.selection(), original);
    assert!(editor.redo().unwrap());
    assert_eq!(editor.text(), "😀/**\r\nx/**");
    assert_eq!(editor.selection(), after_opener);
    assert!(editor.redo().unwrap());
    assert_eq!(editor.text(), "😀/** */\r\nx/** */");
    assert_eq!(editor.selection(), after_closer);
}

#[test]
fn closer_follow_up_yields_to_a_document_change_or_a_new_read_only_policy() {
    use std::cell::Cell;

    let _scope = test_scope();
    let editor = editor_with_pairs("/*");
    let document = editor.document();
    let listener_document = document.clone();
    let intervened = Cell::new(false);
    let listener = document.update_finished(move || {
        if !intervened.replace(true) {
            listener_document.insert(0, " ").unwrap();
        }
    });
    editor.set_caret_offset(2).unwrap();
    assert!(editor.text_area().perform_text_input("*").unwrap());
    assert_eq!(editor.text(), " /**");
    assert_eq!(editor.caret_offset(), 4);
    listener.dispose();

    let editor = editor_with_pairs("/*");
    let document = editor.document();
    let weak = editor.downgrade();
    let first = Cell::new(true);
    let _listener = document.update_started(move || {
        if !first.replace(false) {
            weak.upgrade().unwrap().set_is_read_only(true);
        }
    });
    editor.set_caret_offset(2).unwrap();
    assert!(editor.text_area().perform_text_input("*").unwrap());
    assert_eq!(editor.text(), "/**");
    assert_eq!(editor.caret_offset(), 3);
    assert_eq!(document.undo_count(), 1);
}

#[test]
fn typing_a_multi_character_pair_keeps_the_callers_undo_group() {
    let _scope = test_scope();
    let editor = editor_with_pairs("/*");
    editor.set_caret_offset(2).unwrap();
    let update = editor.document().begin_update().unwrap();
    assert!(editor.text_area().perform_text_input("*").unwrap());
    assert_eq!(editor.text(), "/** */");
    update.finish().unwrap();
    assert_eq!(editor.document().undo_count(), 1);
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), "/*");
}

#[test]
fn surrounding_preserves_directed_utf16_selection_and_read_only_contents() {
    let _scope = test_scope();
    let editor = editor_with_pairs("a😀b x");
    let document = editor.document();
    let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
    protected.add(1, 2).unwrap();
    editor.session().set_read_only_section_provider(protected);
    let selection = Selection::from_segments([SelectionSegment::new(3, 1), SelectionSegment::new(5, 6)], 1);
    editor.set_selection(selection.clone()).unwrap();
    let inside = document.create_anchor(2).unwrap();
    editor.text_area().perform_text_input("[").unwrap();
    assert_eq!(editor.text(), "a[😀]b [x]");
    assert_eq!(editor.selection(), Selection::from_segments([SelectionSegment::new(4, 2), SelectionSegment::new(8, 9)], 1));
    assert_eq!(inside.offset().unwrap(), 3);
    assert_eq!(document.undo_count(), 1);
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), "a😀b x");
    assert_eq!(editor.selection(), selection);
}

#[test]
fn multicaret_pairs_preflight_protection_and_document_replacement() {
    let _scope = test_scope();
    let editor = editor_with_pairs("first\nsecond");
    let document = editor.document();
    let selection = Selection::from_segments([SelectionSegment::empty(5), SelectionSegment::empty(12)], 1);
    editor.set_selection(selection.clone()).unwrap();
    let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
    protected.add(4, 3).unwrap();
    editor.session().set_read_only_section_provider(protected.clone());
    assert!(matches!(editor.text_area().perform_text_input("("), Err(EditError::ReadOnly)));
    assert_eq!(editor.text(), "first\nsecond");
    assert_eq!(editor.selection(), selection);
    assert!(!document.can_undo());
    protected.clear();

    let weak = editor.downgrade();
    let _change_document = document.update_started(move || {
        weak.upgrade().unwrap().set_document(ferroui_edit_core::document::TextDocument::from_text("replacement"));
    });
    assert!(matches!(editor.text_area().perform_text_input("("), Err(EditError::StaleEditPlan)));
    assert_eq!(editor.text(), "replacement");
    assert_eq!(document.text(), "first\nsecond");
    assert!(!document.can_undo());
}

#[test]
fn enter_discards_a_prepared_edit_after_the_view_rebinds() {
    let _scope = test_scope();
    let text = "  abXXcd\r\n  efgh";
    for (selection, no_strategy, protected) in [
        (Selection::empty(4), false, false),
        (Selection::empty(4), true, false),
        (Selection::simple(2, 8), false, true),
        (Selection::from_rectangle_segments([SelectionSegment::new(2, 4), SelectionSegment::new(12, 14)], 1), false, false),
    ] {
        let editor = laid_out_editor(text);
        let document = editor.document();
        editor.set_selection(selection).unwrap();
        if no_strategy {
            editor.session().set_indentation_strategy(None);
        }
        if protected {
            let policy = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
            policy.add(4, 2).unwrap();
            editor.session().set_read_only_section_provider(policy);
        }
        document.clear_undo_history().unwrap();
        let weak = editor.downgrade();
        let _rebind = document.update_started(move || {
            weak.upgrade().unwrap().set_document(crate::TextDocument::from_text("replacement"));
        });

        assert!(matches!(editor.text_area().perform_text_input("\n"), Err(EditError::StaleEditPlan)));
        assert_eq!(editor.text(), "replacement");
        assert_eq!(document.text(), text);
        assert!(!document.can_undo());
    }
}

#[test]
fn configured_enter_updates_each_caret_in_one_protected_crlf_edit() {
    use ferroui_textmate::{LanguageConfiguration, Registry, Theme};

    let _scope = test_scope();
    let original = "tag\r\n\topen😀replace done\r\n    plain";
    let editor = laid_out_editor(original);
    let mut registry = Registry::new();
    registry.add_grammar_json(r#"{"scopeName":"source.enter-test","patterns":[]}"#).unwrap();
    let grammar = registry.load_grammar("source.enter-test").unwrap();
    let installation = editor.install_textmate(grammar.clone(), Theme::from_json("{}").unwrap());
    let configuration = LanguageConfiguration::from_json(r#"{
      "onEnterRules":[{"beforeText":"^\\s*open😀$","afterText":"^ done$","previousLineText":"^tag$",
         "action":{"indent":"indentOutdent","appendText":"  » ","removeText":1}}]
    }"#).unwrap();
    installation.set_language_configuration(Some(&configuration)).unwrap();
    let mut options = editor.options();
    options.convert_tabs_to_spaces = true;
    editor.set_options(options);
    let start = "tag\r\n\topen😀".encode_utf16().count();
    let end = start + "replace".len();
    let selection = Selection::from_segments([
        SelectionSegment::new(end, start),
        SelectionSegment::empty(original.encode_utf16().count()),
    ], 0);
    editor.set_selection(selection.clone()).unwrap();
    let document = editor.document();
    document.clear_undo_history().unwrap();
    let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
    protected.add(start, 1).unwrap();
    editor.session().set_read_only_section_provider(protected.clone());
    assert!(matches!(editor.text_area().perform_text_input("\n"), Err(EditError::ReadOnly)));
    assert_eq!(editor.text(), original);
    assert_eq!(editor.selection(), selection);
    assert!(!document.can_undo());

    protected.clear();
    assert!(key_down(&editor.text_area(), Key::Return));
    let prefix = "tag\r\n\topen😀\r\n  » ";
    let expected = format!("{prefix}\r\n done\r\n    plain\r\n    ");
    assert_eq!(editor.text(), expected);
    assert_eq!(editor.caret_offset(), prefix.encode_utf16().count());
    assert_eq!(editor.selection().segments()[1].caret, expected.encode_utf16().count());
    assert_eq!(document.undo_count(), 1);
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), original);
    assert_eq!(editor.selection(), selection);

    installation.set_grammar(grammar);
    assert!(!installation.input_languages().has_enter_rules());
}

#[test]
fn insert_gates_overstrike_and_typed_text_replaces_one_grapheme_per_commit() {
    let _scope = test_scope();
    let editor = laid_out_editor("ae\u{301}😀z");
    let area = editor.text_area();
    editor.set_caret_offset(1).unwrap();
    let bar = area.cursor_rectangle();
    assert!(!key_down(&area, Key::Insert));
    assert!(!area.overstrike_mode());

    let toggle = editor.command(EditorCommandKind::ToggleOverstrike);
    assert!(!toggle.can_execute(None));
    let mut options = editor.options();
    options.allow_toggle_overstrike_mode = true;
    editor.set_options(options);
    assert!(toggle.can_execute(None));
    assert!(key_down(&area, Key::Insert));
    assert!(area.overstrike_mode());
    assert!(area.cursor_rectangle().width > bar.width);

    assert!(area.perform_text_input("XY").unwrap());
    assert_eq!(editor.text(), "aXY😀z");
    let mut input = TextInputEventArgs::new();
    input.text = Some("b".into());
    input.set_routed_event(Some(InputElement::text_input_event()));
    area.raise_event(&input);
    assert!(input.handled());
    assert_eq!(editor.text(), "aXYbz");
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), "aXY😀z");
    assert_eq!(editor.caret_offset(), 3);

    toggle.execute(None);
    assert!(!area.overstrike_mode());
    assert_eq!(area.cursor_rectangle().width, bar.width);
}

#[test]
fn overstrike_keeps_delimiters_and_virtual_cells_and_replaces_a_selection_normally() {
    let _scope = test_scope();
    let editor = laid_out_editor("abc\r\ndef");
    let area = editor.text_area();
    area.set_overstrike_mode(true);
    editor.select(0, 2).unwrap();
    assert!(area.perform_text_input("X").unwrap());
    assert_eq!(editor.text(), "Xc\r\ndef");
    editor.set_caret_offset(2).unwrap();
    assert!(area.perform_text_input("!").unwrap());
    assert_eq!(editor.text(), "Xc!\r\ndef");
    assert!(key_down(&area, Key::Return));
    assert_eq!(editor.text(), "Xc!\r\n\r\ndef");

    let mut options = editor.options();
    options.enable_virtual_space = true;
    editor.set_options(options);
    editor
        .session()
        .set_caret_position(editor.document().text_length(), 3)
        .unwrap();
    assert!(area.perform_text_input("z").unwrap());
    assert_eq!(editor.text(), "Xc!\r\n\r\ndef   z");
}

#[test]
fn overstrike_preflights_every_caret_before_making_one_undo_step() {
    let _scope = test_scope();
    let editor = laid_out_editor("abc xyz");
    let area = editor.text_area();
    area.set_overstrike_mode(true);
    let selection =
        Selection::from_segments([SelectionSegment::empty(1), SelectionSegment::empty(5)], 1);
    editor.set_selection(selection.clone()).unwrap();
    let document = editor.document();
    document.clear_undo_history().unwrap();
    let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
    protected.add(5, 1).unwrap();
    editor
        .session()
        .set_read_only_section_provider(protected.clone());

    assert!(matches!(
        area.perform_text_input("!"),
        Err(EditError::ReadOnly)
    ));
    assert_eq!(editor.text(), "abc xyz");
    assert_eq!(editor.selection(), selection);
    assert!(!document.can_undo());

    protected.clear();
    assert!(area.perform_text_input("!").unwrap());
    assert_eq!(editor.text(), "a!c x!z");
    assert_eq!(document.undo_count(), 1);
    assert!(document.undo().unwrap());
    assert_eq!(editor.text(), "abc xyz");
    assert_eq!(editor.selection(), selection);
}

#[test]
fn typing_and_delete_preserve_protected_text_in_an_ordinary_selection() {
    let _scope = test_scope();
    let original = "abcdEFghij";
    // Document, caret and undo results recorded against the pinned editor.
    for (anchor, caret, replacement, text, expected_caret) in [
        (1, 9, Some("X"), "aEFXj", 4),
        (9, 1, Some("X"), "aEFXj", 4),
        (1, 9, None, "aEFj", 1),
        (9, 1, None, "aEFj", 1),
        (1, 6, Some("X"), "aXEFghij", 2),
        (5, 9, Some("X"), "abcdEFXj", 7),
        (4, 6, Some("X"), original, 6),
        (4, 6, None, original, 6),
    ] {
        let editor = laid_out_editor(original);
        let area = editor.text_area();
        let document = editor.document();
        document.clear_undo_history().unwrap();
        let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
        protected.add(4, 2).unwrap();
        editor.session().set_read_only_section_provider(protected);
        let selection = Selection::simple(anchor, caret);
        editor.set_selection(selection.clone()).unwrap();

        if let Some(replacement) = replacement {
            assert!(area.perform_text_input(replacement).unwrap());
        } else {
            let key = if anchor < caret { Key::Delete } else { Key::Back };
            assert!(key_down(&area, key));
        }
        assert_eq!(editor.text(), text, "anchor {anchor}, caret {caret}");
        assert_eq!(editor.caret_offset(), expected_caret);
        if text == original {
            assert_eq!(editor.selection(), selection);
            assert!(!document.can_undo());
        } else {
            assert!(editor.selection().is_empty());
            assert_eq!(document.undo_count(), 1);
            assert!(editor.undo().unwrap());
            assert_eq!(editor.text(), original);
            assert_eq!(editor.selection(), selection);
        }
    }
}

#[test]
fn contextual_input_undo_restores_the_original_directed_selection() {
    let _scope = test_scope();
    let editor = laid_out_editor("p🌐wrod界q");
    let area = editor.text_area();
    let document = editor.document();
    document.clear_undo_history().unwrap();
    let selection = Selection::simple(7, 3);
    editor.set_selection(selection.clone()).unwrap();

    assert!(area.perform_surrounding_edit(&editor, 2, 1, "word", || true).unwrap());
    assert_eq!(editor.text(), "pwordq");
    assert_eq!(editor.caret_offset(), 5);
    assert_eq!(document.undo_count(), 1);
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), "p🌐wrod界q");
    assert_eq!(editor.selection(), selection);
    assert!(editor.redo().unwrap());
    assert_eq!(editor.text(), "pwordq");
    assert_eq!(editor.caret_offset(), 5);
}

#[test]
fn exact_context_edit_replaces_a_later_line_range_away_from_the_caret() {
    use ferroui_base::input::text_input::TextSelection;

    let _scope = test_scope();
    let original = "head\r\n🌐teh tail";
    let editor = laid_out_editor(original);
    let area = editor.text_area();
    let document = editor.document();
    document.clear_undo_history().unwrap();
    editor.set_caret_offset(original.encode_utf16().count()).unwrap();
    let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
    protected.add(11, 5).unwrap();
    editor.session().set_read_only_section_provider(protected);
    let edit = TextInputMethodEdit::new(TextSelection::new(2, 5), "the", TextSelection::new(5, 2));

    assert!(area.perform_local_input_edit(&editor, &edit, || true).unwrap());
    assert_eq!(editor.text(), "head\r\n🌐the tail");
    assert_eq!(editor.selection(), Selection::simple(11, 8));
    assert_eq!(document.undo_count(), 1);
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), original);
    assert_eq!(editor.caret_offset(), original.encode_utf16().count());
}

#[test]
fn contextual_input_rejects_protected_text_and_canceled_edits_without_deleting_anything() {
    let _scope = test_scope();
    let editor = laid_out_editor("wrod tail");
    let area = editor.text_area();
    let document = editor.document();
    document.clear_undo_history().unwrap();
    editor.set_caret_offset(4).unwrap();
    let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
    protected.add(1, 1).unwrap();
    editor.session().set_read_only_section_provider(protected.clone());
    assert!(matches!(area.perform_surrounding_edit(&editor, 4, 0, "word", || true), Err(EditError::ReadOnly)));
    assert_eq!(editor.text(), "wrod tail");
    assert_eq!(editor.caret_offset(), 4);
    assert!(!document.can_undo());

    protected.clear();
    let _entering = area.add_disposable_handler(TextArea::text_entering_event(), |_, event| event.set_handled(true));
    assert!(!area.perform_surrounding_edit(&editor, 4, 0, "word", || true).unwrap());
    assert_eq!(editor.text(), "wrod tail");
    assert!(!document.can_undo());
}

#[test]
fn contextual_input_rejects_a_callback_that_moves_the_selection() {
    let _scope = test_scope();
    let editor = laid_out_editor("wrod tail");
    let area = editor.text_area();
    editor.set_caret_offset(4).unwrap();
    let target = editor.downgrade();
    let _entering = area.add_disposable_handler(TextArea::text_entering_event(), move |_, _| {
        target.upgrade().unwrap().set_caret_offset(0).unwrap();
    });
    let result = area.perform_surrounding_edit(&editor, 4, 0, "word", || true);
    assert!(matches!(result, Err(EditError::StaleEditPlan)));
    assert_eq!(editor.text(), "wrod tail");
    assert_eq!(editor.caret_offset(), 0);
}

#[test]
fn enter_respects_protected_text_and_indents_only_writable_lines() {
    let _scope = test_scope();
    let original = "  aaXXbbzz\r\n";
    // Document, caret and undo outcomes from the pinned reference editor.
    for (source, protection, selection, indent, text, caret, retained) in [
        (original, (4, 2), (2, 8), true, "  XX\r\n  zz\r\n", 8, None),
        (original, (4, 2), (8, 2), true, "  XX\r\n  zz\r\n", 8, None),
        (original, (4, 2), (2, 5), true, "  \r\nXXbbzz\r\n", 4, None),
        ("  aa    zz\r\n", (5, 2), (4, 4), true, "  aa\r\n    zz\r\n", 6, None),
        ("  aazz\r\n", (4, 2), (4, 4), true, "  aa\r\nzz\r\n", 6, None),
        (original, (4, 2), (5, 5), true, original, 5, None),
        (original, (4, 2), (4, 6), true, original, 6, Some((4, 6))),
        ("  aaXX\r\nzz\r\n", (4, 4), (4, 8), true, "  aaXX\r\n  zz\r\n", 8, Some((4, 8))),
        ("  prev\r\n\r\nzz", (8, 2), (10, 8), true, "  prev\r\n  \r\nzz", 10, Some((12, 10))),
        (original, (4, 2), (2, 8), false, "  XX\r\nzz\r\n", 6, None),
        ("  aa😀bbzz\r\n", (4, 2), (8, 2), true, "  😀\r\n  zz\r\n", 8, None),
    ] {
        let editor = laid_out_editor(source);
        let document = editor.document();
        document.clear_undo_history().unwrap();
        let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
        protected.add(protection.0, protection.1).unwrap();
        editor.session().set_read_only_section_provider(protected);
        if !indent {
            editor.session().set_indentation_strategy(None);
        }
        let selection = Selection::simple(selection.0, selection.1);
        editor.set_selection(selection.clone()).unwrap();

        let area = editor.text_area();
        if selection.is_empty() {
            assert!(area.perform_text_input("\n").unwrap());
        } else {
            assert!(key_down(&area, Key::Return));
        }
        assert_eq!(editor.text(), text, "{source:?}, {selection:?}");
        assert_eq!(editor.caret_offset(), caret, "{source:?}, {selection:?}");
        let expected_selection = retained.map_or_else(
            || Selection::empty(caret),
            |(anchor, caret)| Selection::simple(anchor, caret),
        );
        assert_eq!(editor.selection(), expected_selection);
        if text == source {
            assert!(!document.can_undo());
        } else {
            assert_eq!(document.undo_count(), 1);
            assert!(editor.undo().unwrap());
            assert_eq!(editor.text(), source);
            assert_eq!(editor.selection(), selection);
            assert!(editor.redo().unwrap());
            assert_eq!(editor.text(), text);
        }
    }
}

#[test]
fn rectangular_typing_and_delete_edit_only_writable_spans() {
    let _scope = test_scope();
    let original = "abcdef\r\nghijkl\r\nmnopqr";
    let box_rows = [(1, 5), (9, 13), (17, 21)];
    let box_protection = [(2, 2), (9, 4)];
    // Text and caret columns recorded against the pinned reference editor.
    for (positions, protected_spans, text, expected, expected_carets) in [
        (box_rows, &box_protection[..], "X", "acdXf\r\nghijkl\r\nmXr", &[2, 9, 17][..]),
        (box_rows, &[(1, 4)], "X", "abcdef\r\ngXl\r\nmXr", &[2, 10, 15]),
        (box_rows, &[(1, 4), (9, 4), (17, 4)], "X", original, &[2, 10, 18]),
        ([(2, 2), (10, 10), (18, 18)], &[(8, 3)], "X", "abXcdef\r\nghijkl\r\nmnXopqr", &[3, 12, 20]),
        (box_rows, &box_protection, "", "acdf\r\nghijkl\r\nmr", &[1, 7, 15]),
        (box_rows, &box_protection, "X\r\nY", "acdXf\r\nghijkl\r\nmnopqr", &[17]),
    ] {
        let editor = laid_out_editor(original);
        let area = editor.text_area();
        let document = editor.document();
        document.clear_undo_history().unwrap();
        let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
        for &(start, length) in protected_spans {
            protected.add(start, length).unwrap();
        }
        editor.session().set_read_only_section_provider(protected);
        let selection = Selection::from_rectangle_segments(
            positions.map(|(start, end)| SelectionSegment::new(start, end)),
            2,
        );
        editor.set_selection(selection.clone()).unwrap();

        if text.is_empty() {
            assert!(key_down(&area, Key::Delete));
        } else {
            assert!(area.perform_text_input(text).unwrap());
        }
        assert_eq!(editor.text(), expected, "replacement {text:?}");
        let updated = editor.selection();
        assert_eq!(updated.is_rectangular(), expected_carets.len() != 1);
        assert!(updated.is_empty());
        assert_eq!(updated.segments().iter().map(|part| part.caret).collect::<Vec<_>>(), expected_carets);
        assert_eq!(editor.caret_offset(), *expected_carets.last().unwrap());
        if expected == original {
            assert!(!document.can_undo());
        } else {
            assert_eq!(document.undo_count(), 1);
            assert!(editor.undo().unwrap());
            assert_eq!(editor.text(), original);
            assert_eq!(editor.selection(), selection);
        }
    }
}

#[test]
fn rectangular_enter_matches_the_reference_document_and_caret() {
    let _scope = test_scope();
    let plain = "abcdef\r\nghijkl\r\nmnopqr";
    let indented = "abcdef\r\n    ghijkl\r\n mnopqr";
    let box_rows = [(1, 5), (9, 13), (17, 21)];
    let column = [(2, 2), (10, 10), (18, 18)];
    let indented_rows = [(3, 5), (11, 13), (23, 25)];
    // Offsets and outcomes from the pinned reference editor.
    for (original, positions, protected, expected, caret) in [
        (plain, &box_rows[..], &[][..], "af\r\ngl\r\nmnopqr", 9),
        (plain, &box_rows, &[(2, 2), (9, 4)], "acdf\r\nghijkl\r\nmnopqr", 15),
        (plain, &column, &[], plain, 18),
        (plain, &column, &[(8, 3)], plain, 18),
        (plain, &[(1, 5)], &[], "a\r\nf\r\nghijkl\r\nmnopqr", 0),
        (plain, &[(2, 2)], &[], "ab\r\ncdef\r\nghijkl\r\nmnopqr", 0),
        (plain, &[(1, 5)], &[(2, 2)], "acd\r\nf\r\nghijkl\r\nmnopqr", 3),
        ("  abcdef\r\n    ghijkl\r\nmnopqr", &[(3, 5), (13, 15)], &[], "  adef\r\n  hijkl\r\nmnopqr", 10),
        (indented, &indented_rows, &[], "abcf\r\n   hijkl\r\n   mnopqr", 21),
        (indented, &indented_rows, &[(20, 1)], "abcf\r\n   hijkl\r\n mnopqr", 19),
        (indented, &[(2, 2), (10, 10), (22, 22)], &[], "abcdef\r\n    ghijkl\r\n    mnopqr", 25),
        ("  abcdef\r\n    ghijkl\r\n mnopqr", &[(14, 14)], &[], "  abcdef\r\n  \r\nghijkl\r\n mnopqr", 12),
        ("    abcdef\r\n    ghijkl\r\nmnopqr", &[(1, 3), (13, 15)], &[], "  abcdef\r\n  ghijkl\r\nmnopqr", 12),
    ] {
        let editor = laid_out_editor(original);
        let document = editor.document();
        document.clear_undo_history().unwrap();
        let provider = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
        for &(start, length) in protected {
            provider.add(start, length).unwrap();
        }
        editor.session().set_read_only_section_provider(provider);
        let selection = Selection::from_rectangle_segments(
            positions.iter().map(|&(start, end)| SelectionSegment::new(start, end)),
            positions.len() - 1,
        );
        editor.set_selection(selection.clone()).unwrap();
        assert!(key_down(&editor.text_area(), Key::Return));
        assert_eq!(editor.text(), expected, "original {original:?}, selection {positions:?}");
        assert_eq!(editor.selection(), Selection::empty(caret), "text {expected:?}");
        if original == expected {
            assert!(!document.can_undo());
        } else {
            assert_eq!(document.undo_count(), 1);
            assert!(editor.undo().unwrap());
            assert_eq!(editor.text(), original);
            assert_eq!(editor.selection(), selection);
        }
    }
}

#[test]
fn overstrike_paints_the_glyph_on_the_next_wrapped_row_at_a_soft_break() {
    let _scope = test_scope();
    let text = "abcdefghijk".repeat(8);
    let editor = laid_out_editor(&text);
    editor.set_word_wrap(true);
    let area = editor.text_area();
    area.measure(Size::new(120.0, 480.0));
    area.arrange(Rect::new(0.0, 0.0, 120.0, 480.0));
    let trailing = area
        .position_at_point(Point::new(10_000.0, 2.0), false)
        .unwrap();
    editor.set_caret_position(trailing).unwrap();
    let thin_caret = area.cursor_rectangle();
    let offset = editor.caret_offset();
    area.set_overstrike_mode(true);
    let block = area.cursor_rectangle();
    assert!(block.y > thin_caret.y);
    assert!(block.width > thin_caret.width);

    area.raise_event(&FocusChangedEventArgs::new(InputElement::got_focus_event()));
    let operations = crate::tests::render_editor(&editor);
    assert!(operations.iter().any(|op| op == "PushOpacity 0.5"));

    assert!(area.perform_text_input("!").unwrap());
    assert_eq!(
        editor.text(),
        format!("{}!{}", &text[..offset], &text[offset + 1..])
    );
    assert!(editor.undo().unwrap());
    assert_eq!(editor.caret_position(), trailing);
    assert_eq!(area.cursor_rectangle(), block);
}
