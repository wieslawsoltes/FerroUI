//! Discoverable selection layout commands use the session's native planners.
use super::*;
use crate::{LayoutMatch, LayoutReset};
use ferroui_controls::MenuItem;

impl DesignerWorkspace {
    /// Remove the selected controls' local layout values in one source transaction.
    pub fn reset_selection_layout(self: &Rc<Self>, reset: LayoutReset) -> Result<(), String> {
        self.ensure_selection_layout_workspace()?;
        self.session.borrow_mut().reset_selection_layout(reset)?;
        self.refresh(true);
        Ok(())
    }

    /// Match the first selected control's native dimensions through the active layout adapters.
    pub fn match_selection_size(self: &Rc<Self>, mode: LayoutMatch) -> Result<(), String> {
        self.ensure_selection_layout_workspace()?;
        self.session.borrow_mut().match_selection_size(mode)?;
        self.refresh(true);
        Ok(())
    }

    fn ensure_selection_layout_workspace(&self) -> Result<(), String> {
        if self.pending_source.get() {
            return Err("Apply or cancel the XAML draft before changing layout".into());
        }
        if self.runtime_selection.get().is_some() {
            return Err("Select authored control layers to change their layout".into());
        }
        Ok(())
    }

    pub(super) fn connect_selection_layout(self: &Rc<Self>) {
        for (name, reset) in reset_commands() {
            let weak = Rc::downgrade(self);
            self.root.get_control::<MenuItem>(name).click(move |_, _| {
                if let Some(workspace) = weak.upgrade() {
                    workspace.report(workspace.reset_selection_layout(reset));
                }
            });
        }
        for (name, mode) in match_commands() {
            let weak = Rc::downgrade(self);
            self.root.get_control::<MenuItem>(name).click(move |_, _| {
                if let Some(workspace) = weak.upgrade() {
                    workspace.report(workspace.match_selection_size(mode));
                }
            });
        }
    }

    pub(super) fn refresh_selection_layout(&self) {
        let session = self.session.borrow();
        let enabled = self.ensure_selection_layout_workspace().is_ok()
            && session.can_manipulate()
            && !session.selection().is_empty()
            && session.selection().iter().all(|node| {
                session.ensure_layer_manipulable(*node).is_ok()
                    && session.preview.visual(*node).is_some()
            });
        self.root
            .get_control::<MenuItem>("LayoutResetMenu")
            .set_is_enabled(enabled);
        self.root
            .get_control::<MenuItem>("LayoutMatchMenu")
            .set_is_enabled(enabled && session.selection().len() > 1);
        for (name, _) in reset_commands() {
            self.root
                .get_control::<MenuItem>(name)
                .set_is_enabled(enabled);
        }
        for (name, _) in match_commands() {
            self.root
                .get_control::<MenuItem>(name)
                .set_is_enabled(enabled && session.selection().len() > 1);
        }
    }
}

fn reset_commands() -> [(&'static str, LayoutReset); 6] {
    [
        ("LayoutResetMargin", LayoutReset::Margin),
        ("LayoutResetAlignment", LayoutReset::Alignment),
        ("LayoutResetWidth", LayoutReset::Width),
        ("LayoutResetHeight", LayoutReset::Height),
        ("LayoutResetSize", LayoutReset::Size),
        ("LayoutResetAll", LayoutReset::All),
    ]
}

fn match_commands() -> [(&'static str, LayoutMatch); 3] {
    [
        ("LayoutMatchWidth", LayoutMatch::Width),
        ("LayoutMatchHeight", LayoutMatch::Height),
        ("LayoutMatchSize", LayoutMatch::Size),
    ]
}
