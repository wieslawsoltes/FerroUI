// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{ProportionalStackPanel, ProportionalStackPanelSplitter};
use ferroui_base::input::{Cursor, StandardCursorType};
use ferroui_base::layout::Orientation;
use ferroui_base::Ref;
use ferroui_controls::{presenters::ContentPresenter, Control};
pub(crate) struct PanelNavigationHelper;
impl PanelNavigationHelper {
    pub(crate) fn get_parent_panel(control: &Control) -> Option<Ref<ProportionalStackPanel>> {
        if let Some(presenter) = control
            .parent()
            .and_then(|parent| parent.cast::<ContentPresenter>())
        {
            presenter.visual_parent().and_then(|parent| parent.cast())
        } else {
            control.visual_parent().and_then(|parent| parent.cast())
        }
    }
    pub(crate) fn get_sibling_in_direction(
        control: &Control,
        panel: &ProportionalStackPanel,
        direction: isize,
    ) -> Option<Ref<Control>> {
        let children = panel.children();
        let item = control
            .parent()
            .and_then(|parent| parent.cast::<ContentPresenter>())
            .map(|presenter| presenter.upcast::<Control>())
            .unwrap_or_else(|| control.to_ref());
        let mut index = children.index_of(&item).map_or(-1, |index| index as isize) + direction;
        while index >= 0 && (index as usize) < children.count() {
            let child = children.get(index as usize);
            if !ProportionalStackPanel::get_is_collapsed(&child)
                && ProportionalStackPanelSplitter::is_splitter(Some(&child)).is_none()
            {
                break;
            }
            index += direction;
        }
        if index >= 0 && (index as usize) < children.count() {
            Some(children.get(index as usize))
        } else {
            None
        }
    }
    pub(crate) fn update_control_visual_state(
        control: &Control,
        panel: &ProportionalStackPanel,
        thickness: f64,
        enabled: bool,
    ) {
        let cursor = if panel.orientation() == Orientation::Vertical {
            control.set_height(thickness);
            control.set_width(f64::NAN);
            if enabled {
                StandardCursorType::SizeNorthSouth
            } else {
                StandardCursorType::Arrow
            }
        } else {
            control.set_width(thickness);
            control.set_height(f64::NAN);
            if enabled {
                StandardCursorType::SizeWestEast
            } else {
                StandardCursorType::Arrow
            }
        };
        control.set_cursor(Some(Cursor::new(cursor)));
    }
}
