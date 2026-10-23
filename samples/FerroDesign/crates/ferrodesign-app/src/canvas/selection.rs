//! Indexed canvas selection, explicit group preference, and selection-only
//! keyboard navigation. Mask tests share the retained public renderer geometry.
use super::*;

pub(super) struct CachedSelectionContext {
    token: core::EditorRenderToken,
    selection: Vec<NodeId>,
    context: core::SelectionContext,
}

pub(super) struct CachedPrototypeSelection {
    token: core::EditorRenderToken,
    root: NodeId,
    index: core::IsolatedSelectionIndex,
    builds: u64,
}

impl DesignCanvas {
    pub fn prototype_selection_index_stats(&self) -> serde_json::Value {
        let cached = self.prototype_selection.borrow();
        serde_json::json!({
            "root": cached.as_ref().map(|cache| cache.root),
            "builds": cached.as_ref().map_or(0, |cache| cache.builds),
            "indexedNodes": cached.as_ref().map_or(0, |cache| cache.index.len()),
        })
    }

    pub(super) fn prototype_pick_in_root(
        &self,
        editor: &Editor,
        root: NodeId,
        point: Point,
    ) -> Option<NodeId> {
        let token = editor.render_change_token();
        let mut cached = self.prototype_selection.borrow_mut();
        if cached
            .as_ref()
            .is_none_or(|entry| entry.token != token || entry.root != root)
        {
            let index = core::IsolatedSelectionIndex::new(&editor.document, root)?;
            let builds = cached
                .as_ref()
                .map_or(1, |entry| entry.builds.saturating_add(1));
            *cached = Some(CachedPrototypeSelection {
                token,
                root,
                index,
                builds,
            });
        }
        let mut painter = self.painter.borrow_mut();
        cached.as_ref()?.index.hit_test_filtered(
            &editor.document,
            point,
            3.0 / self.viewport.get().zoom,
            true,
            |id| painter.mask_contains(editor, &[root], id, point),
        )
    }

    pub(super) fn pick(&self, point: Point) -> Option<NodeId> {
        self.pick_with_modifiers(point, KeyModifiers::empty(), false)
    }

    pub(super) fn pick_with_modifiers(
        &self,
        point: Point,
        modifiers: KeyModifiers,
        descend: bool,
    ) -> Option<NodeId> {
        let editor = self.editor.borrow();
        let roots = editor
            .document
            .page(editor.active_page)?
            .children
            .as_slice();
        let mut painter = self.painter.borrow_mut();
        let hit = editor.hit_test_filtered(point, 3.0 / self.viewport.get().zoom, |id| {
            painter.mask_contains(&editor, roots, id, point)
        })?;
        if !self.select_groups.get()
            || modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::META)
        {
            return Some(hit);
        }
        let token = editor.render_change_token();
        let mut cache = self.selection_context.borrow_mut();
        if cache
            .as_ref()
            .is_none_or(|entry| entry.token != token || entry.selection != editor.selection)
        {
            *cache = Some(CachedSelectionContext {
                token,
                selection: editor.selection.clone(),
                context: core::SelectionContext::new(&editor.document, &editor.selection),
            });
        }
        cache
            .as_ref()?
            .context
            .grouped_target(&editor.document, hit, descend)
    }

    fn selection_anchor(&self) -> Option<Point> {
        self.selection_pointer
            .get()
            .map(|screen| self.viewport.get().screen_to_world(screen))
            .or_else(|| {
                self.editor
                    .borrow()
                    .selection_bounds()
                    .map(|bounds| bounds.center())
            })
    }

    /// Layers under the latest canvas pointer, in front-to-back painter order.
    /// A Select Layer menu can include locked layers; pointer cycling cannot.
    /// This read-only query reuses exact prepared mask geometry/alpha coverage.
    pub fn selection_candidates(&self, include_locked: bool) -> Vec<NodeId> {
        let Some(point) = self.selection_anchor() else {
            return Vec::new();
        };
        self.selection_candidates_at(point, include_locked)
    }

    fn selection_candidates_at(&self, point: Point, include_locked: bool) -> Vec<NodeId> {
        let editor = self.editor.borrow();
        let Some(page) = editor.document.page(editor.active_page) else {
            return Vec::new();
        };
        let mut painter = self.painter.borrow_mut();
        editor
            .scene
            .hit_test_candidates(
                &editor.document,
                point,
                3.0 / self.viewport.get().zoom,
                include_locked,
            )
            .into_iter()
            .filter(|id| painter.mask_contains(&editor, &page.children, *id, point))
            .collect()
    }

    pub(super) fn cycle_selection_at(&self, point: Point, direction: core::SelectionCycle) {
        self.cancel_active_edit();
        if self.editor.borrow().has_active_transaction() {
            return;
        }
        let candidates = self.selection_candidates_at(point, false);
        let changed = self
            .editor
            .borrow_mut()
            .cycle_selection(&candidates, direction);
        if changed {
            self.leave_vector_edit();
            self.hover.set(None);
            self.invalidate_visual();
            self.emit(CanvasEvent::Selection);
        }
    }

    /// Common native action hook for menus, command search and keyboard input.
    /// Recognized commands consume input even at a hierarchy boundary. They do
    /// not begin/commit history, and never take ownership of an external draft.
    pub fn selection_command(&self, action: &str) -> bool {
        let navigation = match action {
            "select-parent" => Some(core::SelectionNavigation::Parent),
            "select-children" => Some(core::SelectionNavigation::Children),
            "select-next-sibling" => Some(core::SelectionNavigation::NextSibling),
            "select-previous-sibling" => Some(core::SelectionNavigation::PreviousSibling),
            "select-behind" | "select-in-front" | "select-groups" | "toggle-select-groups" => None,
            _ => return false,
        };
        if self.presentation.get().is_some() {
            return true;
        }
        self.cancel_active_edit();
        self.finish_image_crop();
        if self.editor.borrow().has_active_transaction() {
            return true;
        }
        if matches!(action, "select-groups" | "toggle-select-groups") {
            self.select_groups.set(!self.select_groups.get());
            self.hover.set(None);
            self.invalidate_visual();
            self.emit(CanvasEvent::Selection);
            self.emit(CanvasEvent::Notice(if self.select_groups.get() {
                "Select groups enabled. Double-click to enter a group; Ctrl/Command-click selects a nested layer.".into()
            } else {
                "Direct layer selection enabled.".into()
            }));
        } else if let Some(navigation) = navigation {
            let left_vector = self.vector_node.get().is_some();
            self.leave_vector_edit();
            let changed = self.editor.borrow_mut().navigate_selection(navigation);
            if changed || left_vector {
                self.hover.set(None);
                self.invalidate_visual();
                self.emit(CanvasEvent::Selection);
            }
        } else if let Some(point) = self.selection_anchor() {
            self.cycle_selection_at(
                point,
                if action == "select-behind" {
                    core::SelectionCycle::Behind
                } else {
                    core::SelectionCycle::InFront
                },
            );
        }
        true
    }

    pub(super) fn selection_key(&self, event: &KeyEventArgs) -> bool {
        let command = event
            .key_modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::META);
        let shift = event.key_modifiers.contains(KeyModifiers::SHIFT);
        let alt = event.key_modifiers.contains(KeyModifiers::ALT);
        if command {
            return false;
        }
        if event.key == Key::Tab && !alt && self.vector_node.get().is_none() {
            return self.selection_command(if shift {
                "select-previous-sibling"
            } else {
                "select-next-sibling"
            });
        }
        if event.key == Key::Enter && !alt {
            self.cancel_active_edit();
            if self.editor.borrow().has_active_transaction() {
                return true;
            }
            if shift {
                return self.selection_command("select-parent");
            }
            if self.vector_node.get().is_some() {
                self.leave_vector_edit();
                self.emit(CanvasEvent::Selection);
            } else if !self.edit_vector() {
                let text = {
                    let editor = self.editor.borrow();
                    editor.selection.first().copied().filter(|id| {
                        editor.selection.len() == 1
                            && !editor.document.effective_locked(*id)
                            && editor
                                .document
                                .nodes
                                .get(id)
                                .is_some_and(|node| node.kind == NodeKind::Text)
                    })
                };
                if let Some(id) = text {
                    self.emit(CanvasEvent::EditText(id));
                } else {
                    self.selection_command("select-children");
                }
            }
            return true;
        }
        if alt && self.vector_node.get().is_none() {
            match event.key {
                Key::OemOpenBrackets => return self.selection_command("select-behind"),
                Key::OemCloseBrackets => return self.selection_command("select-in-front"),
                _ => {}
            }
        }
        false
    }
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod tests;
