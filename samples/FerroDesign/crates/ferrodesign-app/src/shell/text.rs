//! A native FerroUI TextBox supplies the caret, text selection, IME and clipboard.
use super::*;
use ferroui_base::input::KeyModifiers;
use ferroui_base::interactivity::RoutingStrategies;
use ferroui_base::media::MatrixTransform;
use ferroui_base::{Matrix, RelativePoint};
use crate::renderer::typography;

pub(super) struct InlineLayout {
    id: NodeId,
    revision: u64,
    origin: ferroui_base::Point,
    width: f64,
    height: f64,
}

/// The native TextBox's bounded/coalesced undo history retains only editable
/// text data. Artwork, children and embedded image paints are not copied into
/// each typing checkpoint.
#[derive(Clone)]
struct InlineTextSnapshot {
    id: NodeId,
    text: String,
    runs: Vec<core::TextRun>,
    paragraphs: Vec<core::TextParagraph>,
}

impl InlineTextSnapshot {
    fn capture(node: &Node) -> Self {
        Self { id: node.id, text: node.text.clone(), runs: node.text_runs.clone(), paragraphs: node.text_paragraphs.clone() }
    }

    fn restore(&self, node: &mut Node, value: &str) -> Option<bool> {
        if self.id != node.id || self.text != value { return None; }
        let changed = node.text != self.text || node.text_runs != self.runs || node.text_paragraphs != self.paragraphs;
        if changed {
            node.text.clone_from(&self.text);
            node.text_runs.clone_from(&self.runs);
            node.text_paragraphs.clone_from(&self.paragraphs);
        }
        Some(changed)
    }
}

impl EditorShell {
    pub(super) fn wire_inline_text(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.field("InlineTextBox").text_changing(move |_, event| {
            let Some(shell) = weak.upgrade() else {
                return;
            };
            if shell.inline_updating.get() {
                return;
            }
            let Some(id) = shell.inline_text.get() else {
                return;
            };
            let field = shell.field("InlineTextBox");
            // IME preedit can remove the native selection before it commits.
            // Keep confirmed text and rich-text runs in the document so a
            // lifecycle save during composition cannot persist that removal.
            if field.is_composing_text() {
                return;
            }
            let value = field.text().unwrap_or_default();
            let mut editor = shell.editor.borrow_mut();
            let changed = editor
                .document
                .nodes
                .get_mut(&id)
                .map(|node| apply_text_box_change(node, &field, value, event.text_change()))
                .unwrap_or(false);
            if !changed { return; }
            if editor.has_auto_layout() {
                // Imported layout documents may contain geometry that has not
                // been arranged yet. Preserve the existing first-edit layout.
                editor.document.layout_all();
                editor.refresh();
            } else {
                editor.refresh_preview_nodes(&[id]);
            }
            let node = editor.document.nodes.get(&id).cloned();
            drop(editor);
            if let Some(node) = node {
                typography::refresh_text_box_ranges(&field, &node);
                field.set_text_undo_state(Some(Rc::new(InlineTextSnapshot::capture(&node))));
            }
            if changed {
                shell.set_inline_draft_dirty(true);
            }
            shell.position_inline_text();
            shell.canvas.invalidate_visual();
        });
        let weak = Rc::downgrade(self);
        self.field("InlineTextBox")
            .add_handler(InputElement::lost_focus_event(), move |_, _| {
                if let Some(shell) = weak.upgrade() {
                    if !shell.inline_updating.get() {
                        shell.finish_inline_text(true);
                    }
                }
            });
        let weak = Rc::downgrade(self);
        self.field("InlineTextBox").add_handler_with(
            InputElement::key_down_event(),
            move |_, event| {
                if event.key == Key::Escape
                    || (event.key == Key::Enter
                        && event
                            .key_modifiers
                            .intersects(KeyModifiers::CONTROL | KeyModifiers::META))
                {
                    if let Some(shell) = weak.upgrade() {
                        shell.finish_inline_text(event.key != Key::Escape);
                        shell.canvas.focus();
                    }
                    event.set_handled(true);
                }
            },
            RoutingStrategies::TUNNEL,
            false,
        );
    }

    pub(super) fn begin_inline_text(self: &Rc<Self>, id: NodeId) {
        if self.inline_text.get() == Some(id) {
            self.field("InlineTextBox").focus();
            return;
        }
        self.finish_inline_text(true);
        let node = {
            let editor = self.editor.borrow();
            let Some(node) = editor.document.nodes.get(&id) else {
                return;
            };
            if node.kind != NodeKind::Text || node.locked || !node.visible {
                return;
            }
            let mut parent = node.parent;
            while let Some(id) = parent {
                let Some(ancestor) = editor.document.nodes.get(&id) else {
                    break;
                };
                if ancestor.locked || !ancestor.visible {
                    return;
                }
                parent = ancestor.parent;
            }
            node.clone()
        };
        self.canvas.leave_vector_edit();
        {
            let mut editor = self.editor.borrow_mut();
            editor.selection = vec![id];
            editor.begin_node_transaction(&[id], "Edit text");
        }
        self.inline_text.set(Some(id));
        self.inline_layout.borrow_mut().take();
        self.inline_updating.set(true);
        self.inspector_tab.set(0);
        self.refresh_selection();
        let field = self.field("InlineTextBox");
        field.set_is_undo_enabled(false);
        field.set_text(Some(&node.text));
        typography::configure_text_box(&field, &node);
        field.set_text_undo_state(Some(Rc::new(InlineTextSnapshot::capture(&node))));
        field.set_is_undo_enabled(true);
        field.set_opacity(node.opacity);
        field.set_render_transform_origin(RelativePoint::TOP_LEFT);
        self.position_inline_text();
        self.visible("InlineTextHost", true);
        self.canvas.set_editing_text(Some(id));
        field.focus();
        field.select_all();
        self.inline_updating.set(false);
    }

    pub(super) fn position_inline_text(&self) {
        let Some(id) = self.inline_text.get() else {
            return;
        };
        let editor = self.editor.borrow();
        let Some(node) = editor.document.nodes.get(&id) else {
            return;
        };
        let Some(t) = editor.document.world_transform(id) else {
            return;
        };
        let viewport = self.canvas.viewport.get();
        let zoom = viewport.zoom;
        let field = self.field("InlineTextBox");
        let mut cached = self.inline_layout.borrow_mut();
        if cached.as_ref().is_none_or(|cached| cached.id != id || cached.revision != node.revision) {
            let brush = field.foreground().unwrap_or_else(|| typography::editor_brush(node));
            let (origin, width, height) = typography::inline_geometry(node, brush);
            *cached = Some(InlineLayout { id, revision: node.revision, origin, width, height });
        }
        let cached = cached.as_ref().expect("inline layout");
        field.set_width(cached.width);
        field.set_height(cached.height);
        let origin = t.apply(Point::new(cached.origin.x, cached.origin.y));
        field.set_render_transform(Some(
            MatrixTransform::with_matrix(Matrix::new(
                t.a * zoom,
                t.b * zoom,
                t.c * zoom,
                t.d * zoom,
                origin.x * zoom + viewport.pan.x,
                origin.y * zoom + viewport.pan.y,
            ))
            .into(),
        ));
    }

    pub(super) fn finish_inline_text(self: &Rc<Self>, commit: bool) {
        if self.inline_text.take().is_none() {
            return;
        }
        self.inline_layout.borrow_mut().take();
        self.inline_updating.set(true);
        let changed = {
            let mut editor = self.editor.borrow_mut();
            if commit {
                editor.commit_transaction()
            } else {
                editor.cancel_transaction();
                false
            }
        };
        let field = self.field("InlineTextBox");
        field.set_is_undo_enabled(false);
        field.set_text_undo_state(None);
        self.visible("InlineTextHost", false);
        self.canvas.set_editing_text(None);
        self.inline_updating.set(false);
        if changed {
            self.changed();
        } else {
            self.refresh_selection();
        }
        self.set_inline_draft_dirty(false);
    }
}

fn apply_text_box_change(node: &mut Node, field: &TextBox, value: String, change: Option<&ferroui_controls::TextBoxTextChange>) -> bool {
    if field.is_restoring_text_history() {
        if let Some(snapshot) = field.text_undo_state().and_then(|value| value.downcast::<InlineTextSnapshot>().ok()) {
            if let Some(changed) = snapshot.restore(node, &value) { return changed; }
        }
    }
    apply_native_text_change(node, value, change)
}

/// Preserve the native edit location even when repeated words or paragraphs
/// make an old/new-string comparison ambiguous. External Text assignments and
/// coercions retain the checked whole-string fallback.
fn apply_native_text_change(node: &mut Node, value: String, change: Option<&ferroui_controls::TextBoxTextChange>) -> bool {
    if let Some(change) = change.filter(|change| change.previous_text() == node.text) {
        let range = change.replaced_range();
        if let Some(end) = range.start.checked_add(change.inserted_length()) {
            if let (Ok(old), Ok(new)) = (
                core::text_range_from_utf16(&node.text, range.start, range.end),
                core::text_range_from_utf16(&value, range.start, end),
            ) {
                if old.start == new.start && node.text[..old.start] == value[..new.start]
                    && node.text[old.end..] == value[new.end..] {
                    return node.replace_text_range_preserving_runs(value, old, new.end).unwrap_or(false);
                }
            }
        }
    }
    node.replace_text_preserving_runs(value)
}

#[cfg(all(test, not(target_os = "emscripten")))]
mod exact_edit_tests {
    use super::*;

    #[test]
    fn native_inline_undo_redo_restores_deleted_unicode_character_and_paragraph_styles() {
        let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
        let scope = ferroui_base::FerroLocator::enter_scope();
        let mut original = Node::new(NodeKind::Text, "Rich undo", 0.0, 0.0, 100.0, 100.0);
        original.text = "A\n🦀\nA\n".into();
        for (index, range) in core::text_paragraph_ranges(&original.text).into_iter().filter(|r| !r.is_empty()).enumerate() {
            original.apply_paragraph_style(range.clone(), &core::ParagraphStylePatch {
                indent: core::TextStyleChange::Set((index + 1) as f64 * 10.0), ..Default::default()
            }).unwrap();
            original.apply_text_style(range, &core::TextStylePatch {
                font_weight: core::TextStyleChange::Set(400 + index as u16 * 200), ..Default::default()
            }).unwrap();
        }
        let node = Rc::new(RefCell::new(original.clone()));
        let field = TextBox::new();
        field.set_accepts_return(true);
        field.set_text(Some(&original.text));
        field.set_text_undo_state(Some(Rc::new(InlineTextSnapshot::capture(&original))));
        let document = node.clone();
        let weak = field.downgrade();
        field.text_changing(move |_, event| {
            let field = weak.upgrade().unwrap();
            let mut node = document.borrow_mut();
            assert!(apply_text_box_change(&mut node, &field, field.text().unwrap(), event.text_change()));
            field.set_text_undo_state(Some(Rc::new(InlineTextSnapshot::capture(&node))));
        });
        field.set_selection_start(0);
        field.set_selection_end(2);
        field.set_selected_text(None);
        let edited = node.borrow().clone();
        assert_eq!(edited.text, "🦀\nA\n");
        assert_eq!(edited.text_paragraphs[0].indent, Some(20.0));
        assert_eq!(edited.text_runs[0].font_weight, Some(600));
        node.borrow_mut().x = 37.0;
        field.undo();
        assert_eq!(node.borrow().text, original.text);
        assert_eq!(node.borrow().text_runs, original.text_runs);
        assert_eq!(node.borrow().text_paragraphs, original.text_paragraphs);
        assert_eq!(node.borrow().x, 37.0, "Text history must not restore unrelated artwork");
        field.redo();
        assert_eq!(node.borrow().text, edited.text);
        assert_eq!(node.borrow().text_runs, edited.text_runs);
        assert_eq!(node.borrow().text_paragraphs, edited.text_paragraphs);
        assert!(!field.can_redo());
        scope.dispose();
    }

    #[test]
    fn native_textbox_repeated_paragraph_edit_rebases_the_document_at_the_reported_location() {
        let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
        let scope = ferroui_base::FerroLocator::enter_scope();
        let mut node = Node::new(NodeKind::Text, "Repeated paragraphs", 0.0, 0.0, 100.0, 100.0);
        node.text = "A\nA\n".into();
        for (start, indent) in [(0, 10.0), (2, 20.0)] {
            node.apply_paragraph_style(start..start + 2, &core::ParagraphStylePatch {
                indent: core::TextStyleChange::Set(indent), ..Default::default()
            }).unwrap();
        }
        let node = Rc::new(RefCell::new(node));
        let field = TextBox::new();
        field.set_accepts_return(true);
        field.set_text(Some("A\nA\n"));
        let document = node.clone();
        let weak = field.downgrade();
        field.text_changing(move |_, event| {
            let field = weak.upgrade().unwrap();
            assert!(event.text_change().is_some());
            assert!(apply_native_text_change(&mut document.borrow_mut(), field.text().unwrap(), event.text_change()));
        });
        field.set_caret_index(0);
        field.set_selected_text(Some("A\n"));
        assert_eq!(node.borrow().text, "A\nA\nA\n");
        assert_eq!(node.borrow().text_paragraphs.iter().map(|p| p.indent).collect::<Vec<_>>(),
            [Some(10.0), Some(10.0), Some(20.0)]);
        field.set_selection_start(0);
        field.set_selection_end(2);
        field.set_selected_text(None);
        assert_eq!(node.borrow().text, "A\nA\n");
        assert_eq!(node.borrow().text_paragraphs.iter().map(|p| p.indent).collect::<Vec<_>>(),
            [Some(10.0), Some(20.0)]);
        scope.dispose();
    }
}
