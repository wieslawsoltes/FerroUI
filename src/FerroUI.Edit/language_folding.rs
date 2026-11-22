use ferroui_edit_core::{document::TextSnapshot, folding::NewFolding};
use ferroui_textmate::{FoldMarker, FoldingRules};

const FOLDING_LIMIT: usize = 5000;

struct Boundary {
    depth: Depth,
    end_above: usize,
}

enum Depth {
    Root,
    Indent(usize),
    MarkerEnd(usize),
}

/// Computes indentation and marker folds in a bottom-up pass, so region
/// markers and off-side blank lines can delimit ordinary indentation folds.
pub(crate) fn compute_foldings(
    snapshot: &TextSnapshot,
    rules: &FoldingRules,
    tab_size: usize,
) -> Result<Vec<NewFolding>, Box<dyn std::error::Error>> {
    let mut boundaries = vec![Boundary {
        depth: Depth::Root,
        end_above: snapshot.line_count() + 1,
    }];
    let mut marker_indices: Vec<usize> = Vec::new();
    let mut folds = Vec::new();
    let tab_size = tab_size.max(1);
    for number in (1..=snapshot.line_count()).rev() {
        let line = snapshot.get_line_by_number(number)?;
        let text = snapshot.get_text_utf16(line.offset, line.length)?;
        let Some(indent) = indentation(&text, tab_size) else {
            if rules.off_side {
                boundaries.last_mut().unwrap().end_above = number;
            }
            continue;
        };
        match rules.marker(&text)? {
            Some(FoldMarker::Start { end }) if !marker_indices.is_empty() => {
                let index = marker_indices.pop().expect("a closing marker");
                let Depth::MarkerEnd(closing_offset) = boundaries[index].depth else {
                    unreachable!("marker index points to a closing marker");
                };
                let mut fold = NewFolding::new(line.end_offset(), closing_offset);
                let title = String::from_utf16_lossy(&text[end..]);
                let title = title.trim();
                if !title.is_empty() {
                    fold.name = Some(title.to_owned());
                }
                fold.is_definition = true;
                folds.push((indent, fold));
                boundaries.truncate(index);
                boundaries.push(Boundary {
                    depth: Depth::Indent(indent),
                    end_above: number,
                });
                continue;
            }
            Some(FoldMarker::End) => {
                marker_indices.push(boundaries.len());
                boundaries.push(Boundary {
                    depth: Depth::MarkerEnd(line.end_offset()),
                    end_above: number,
                });
                continue;
            }
            _ => {}
        }
        if matches!(boundaries.last().unwrap().depth, Depth::Indent(current) if current > indent) {
            while matches!(boundaries.last().unwrap().depth, Depth::Indent(current) if current > indent)
            {
                boundaries.pop();
            }
            let end = boundaries.last().unwrap().end_above - 1;
            if end > number {
                let closing = snapshot.get_line_by_number(end)?;
                folds.push((
                    indent,
                    NewFolding::new(line.end_offset(), closing.end_offset()),
                ));
            }
        }
        let next = boundaries.last_mut().unwrap();
        if matches!(next.depth, Depth::Indent(current) if current == indent) {
            next.end_above = number;
        } else {
            boundaries.push(Boundary {
                depth: Depth::Indent(indent),
                end_above: number,
            });
        }
    }
    if folds.len() > FOLDING_LIMIT {
        folds.select_nth_unstable_by_key(FOLDING_LIMIT, |(indent, fold)| {
            (*indent, fold.start_offset)
        });
        folds.truncate(FOLDING_LIMIT);
        folds.sort_unstable_by_key(|(_, fold)| fold.start_offset);
    } else {
        folds.reverse();
    }
    Ok(folds.into_iter().map(|(_, fold)| fold).collect())
}

fn indentation(text: &[u16], tab_size: usize) -> Option<usize> {
    let mut columns = 0usize;
    for unit in text {
        match *unit {
            0x20 => columns = columns.saturating_add(1),
            0x09 => columns = columns.saturating_add(tab_size - columns % tab_size),
            _ => return Some(columns),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TextEditor;
    use ferroui_base::threading::Dispatcher;
    use ferroui_edit_core::document::TextDocument;
    use ferroui_textmate::{LanguageConfiguration, Registry, Theme};
    use std::rc::Rc;

    const CONFIG: &str = r#"{
        "folding":{"markers":{"start":{"pattern":"^\\s*// #region\\b","flags":"i"},"end":"^\\s*// #endregion\\b"}}
    }"#;

    #[test]
    fn indentation_and_markers_share_ranges_with_utf16_offsets() {
        let source = "// #REGION 😀\r\nfn x {\r\n  // #region inner\r\n   body\r\n  // #endregion\r\n}\r\n// #endregion\r\n// #region pending\r\nnext";
        let snapshot = TextSnapshot::from_text(source);
        let configuration = LanguageConfiguration::from_json(CONFIG).unwrap();
        let ranges =
            compute_foldings(&snapshot, &configuration.folding_rules().unwrap(), 4).unwrap();
        let offsets: Vec<_> = ranges
            .iter()
            .map(|range| (range.start_offset, range.end_offset, range.name.as_deref()))
            .collect();
        let end = |line| snapshot.get_line_by_number(line).unwrap().end_offset();
        assert_eq!(
            offsets,
            vec![
                (end(1), end(7), Some("😀")),
                (end(2), end(5), None),
                (end(3), end(5), Some("inner")),
            ]
        );
        assert!(ranges[0].is_definition);
    }

    #[test]
    fn off_side_folds_exclude_trailing_blank_lines_and_expand_tabs() {
        let source = TextSnapshot::from_text("start\n\tchild\n    sibling\n\nnext");
        let config = LanguageConfiguration::from_json(r#"{"folding":{"offSide":true}}"#).unwrap();
        let ranges = compute_foldings(&source, &config.folding_rules().unwrap(), 4).unwrap();
        let end = |line| source.get_line_by_number(line).unwrap().end_offset();
        assert_eq!(ranges, vec![NewFolding::new(end(1), end(3))]);
        let ranges = compute_foldings(&source, &FoldingRules::default(), 4).unwrap();
        assert_eq!(ranges, vec![NewFolding::new(end(1), end(4))]);
    }

    #[test]
    fn automatic_folding_tracks_edits_configuration_and_document_replacement() {
        let _scope = crate::tests::test_scope();
        let editor = TextEditor::new();
        let mut registry = Registry::new();
        registry
            .add_grammar_json(r#"{"scopeName":"source.fold","patterns":[]}"#)
            .unwrap();
        let installation = editor.install_textmate(
            registry.load_grammar("source.fold").unwrap(),
            Theme::default(),
        );
        editor
            .set_text("// #region title\n  body\n// #endregion")
            .unwrap();
        let manager = editor.install_language_folding();
        let configuration = LanguageConfiguration::from_json(CONFIG).unwrap();
        installation
            .set_language_configuration(Some(&configuration))
            .unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        let folds = manager.all_foldings();
        assert_eq!(folds.len(), 1);
        assert_eq!(folds[0].title().as_deref(), Some("title"));
        let start = folds[0].start_offset();
        folds[0].set_is_folded(true);
        editor.document().insert(0, "\n").unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        let updated = manager.all_foldings();
        assert_eq!(updated[0].start_offset(), start + 1);
        assert!(updated[0].is_folded());

        editor.set_document(TextDocument::from_text("root\n  child\n"));
        Dispatcher::ui_thread().run_jobs(None);
        let replacement = editor.folding_manager().unwrap();
        assert!(!Rc::ptr_eq(&manager, &replacement));
        let folds = replacement.all_foldings();
        assert_eq!(folds.len(), 1);
        assert!(!folds[0].is_folded());

        editor.document().insert(0, "new ").unwrap();
        editor.uninstall_folding();
        let manual = editor.install_folding();
        manual.create_folding(0, 1).unwrap();
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(manual.all_foldings()[0].start_offset(), 0);
    }
}
