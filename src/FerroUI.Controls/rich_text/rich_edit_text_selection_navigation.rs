//! Logical Home/End selection navigation over the shared model and text view.
//!
//! Contracts: pinned Microsoft.UI.Text.xml ITextSelection.HomeKey/EndKey and
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextselection-homekey
//! https://learn.microsoft.com/windows/win32/api/tom/nf-tom-itextselection-endkey
use super::*;

impl RichEditTextSelection {
    /// Moves to the logical start of a Line or the Story. Extension follows the
    /// active end and retains the anchor. A collapsed Line request first chooses
    /// the normalized selection start. Other units are rejected before effects.
    ///
    /// The result is the accepted signed UTF-16 displacement of the active end,
    /// including application changes made through cancellable selection events.
    /// Line uses complete untrimmed shaping when a view is associated, and the
    /// shared paragraph fallback otherwise. No editing/history unit is created.
    pub fn home_key(&self, unit: TextRangeUnit, extend: bool) -> Result<isize, EditError> {
        self.move_to_caret_boundary(unit, extend, false)
    }

    /// Moves to the logical end of a Line or the Story. A collapsed Line request
    /// chooses the normalized selection end and excludes its terminator. An
    /// extended Line request follows the active end, includes real hard paragraph
    /// marks, and retains that line-end affinity on repetition. Soft-break and
    /// wrap affinity retain FerroUI's existing caret policy. Story ends at the
    /// model's final caret; no synthetic terminal CR is exposed.
    /// See [`home_key`](Self::home_key) for extension, layout and result policy.
    pub fn end_key(&self, unit: TextRangeUnit, extend: bool) -> Result<isize, EditError> {
        self.move_to_caret_boundary(unit, extend, true)
    }

    fn move_to_caret_boundary(
        &self,
        unit: TextRangeUnit,
        extend: bool,
        to_end: bool,
    ) -> Result<isize, EditError> {
        if !matches!(unit, TextRangeUnit::Line | TextRangeUnit::Story) {
            return Err(DocumentError::InvalidFormat(
                "HomeKey and EndKey support only Line and Story units.",
            )
            .into());
        }
        let units = TextUnits::resolve(&self.editor, unit)?;
        // Formatting can run callbacks that change the directed selection.
        // Capture live endpoints after validated layout. MSFTEDIT collapses to
        // the directional range edge before choosing a line; extension instead
        // navigates from the directed active end.
        let before = self.selection();
        let source = if extend {
            before.active
        } else if to_end {
            before.range().end
        } else {
            before.range().start
        };
        let bounds = units.caret_bounds(source);
        let (target, line_end_start) = if extend && to_end && unit == TextRangeUnit::Line {
            units.extended_line_end(source, self.editor.line_end_navigation_start())
        } else {
            (if to_end { bounds.end } else { bounds.start }, None)
        };
        // Validate representability before a selection callback can run.
        units::signed_displacement(target, before.active)?;
        let requested = EditorSelection::new(if extend { before.anchor } else { target }, target);
        let published =
            self.publish_navigation(before, requested, self.editor.document().mutation_version())?;
        if published.uncontested {
            if let Some(start) = line_end_start {
                self.editor.remember_line_end_navigation(start);
            } else {
                self.editor.clear_navigation_affinity();
            }
        }
        units::signed_displacement(published.accepted.active, before.active)
    }
}

#[cfg(test)]
#[path = "rich_edit_text_selection_navigation_tests.rs"]
mod tests;
