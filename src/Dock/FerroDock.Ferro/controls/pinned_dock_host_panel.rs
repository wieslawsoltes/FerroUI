// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::PinnedDockControl;
use crate::model_projection;
use ferrodock_model::{
    controls::IRootDock,
    core::{Alignment, IDockable, PinnedDockDisplayMode},
};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, input::InputElementImpl,
    instantiate, interactivity::InteractiveImpl, layout::LayoutableImpl, FerroObjectImpl,
    FerroProperty, Rect, Ref, Size, StyledElementImpl, StyledProperty, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl, Panel, PanelImpl};
use std::rc::Rc;
/// Original panel arranging main content and pinned previews inline or overlaid.
#[repr(C)]
pub struct PinnedDockHostPanel {
    base: Panel,
}
ferro_class!(PinnedDockHostPanel:Panel);
ferro_class_info!(PinnedDockHostPanel {
    new: PinnedDockHostPanel::new
});
ferro_impl_classes!(PinnedDockHostPanel:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
ferro_properties! {impl PinnedDockHostPanel {
    /// Identifies PinnedDockDisplayMode.
    pub fn pinned_dock_display_mode_property()->StyledProperty<PinnedDockDisplayMode>{FerroProperty::register::<Self,PinnedDockDisplayMode>("PinnedDockDisplayMode",PinnedDockDisplayMode::Overlay)}
    /// Identifies PinnedDockAlignment.
    pub fn pinned_dock_alignment_property()->StyledProperty<Alignment>{FerroProperty::register::<Self,Alignment>("PinnedDockAlignment",Alignment::Unset)}
}}
impl LayoutableImpl for PinnedDockHostPanel {
    fn measure_override(this: &Self, available: Size) -> Size {
        if this.children().count() == 0 {
            return Size::default();
        }
        let (main, pinned) = this.resolve_children();
        let alignment = this.get_effective_alignment(pinned.as_ref());
        if this.get_effective_display_mode() != PinnedDockDisplayMode::Inline
            || pinned.is_none()
            || alignment == Alignment::Unset
            || !this.has_pinned_dockable(pinned.as_ref().expect("source pinned child"))
        {
            return this.measure_overlay(available);
        }
        let pinned = pinned.expect("source pinned child");
        pinned.measure(available);
        let pinned_size = Self::get_pinned_size(&pinned);
        let remaining = Self::get_remaining_size(available, pinned_size, alignment);
        if let Some(main) = &main {
            main.measure(remaining);
        }
        let main_size = main.map_or(Size::default(), |m| m.desired_size());
        Self::get_desired_size(main_size, pinned_size, alignment)
    }
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        if this.children().count() == 0 {
            return final_size;
        }
        let (main, pinned) = this.resolve_children();
        let alignment = this.get_effective_alignment(pinned.as_ref());
        if this.get_effective_display_mode() != PinnedDockDisplayMode::Inline
            || pinned.is_none()
            || alignment == Alignment::Unset
            || !this.has_pinned_dockable(pinned.as_ref().expect("source pinned child"))
        {
            this.arrange_overlay(final_size);
            return final_size;
        }
        let pinned = pinned.expect("source pinned child");
        let size = Self::get_pinned_size(&pinned);
        let width = min(size.width, final_size.width);
        let height = min(size.height, final_size.height);
        let remaining_width = max(0.0, final_size.width - width);
        let remaining_height = max(0.0, final_size.height - height);
        let (pinned_rect, main_rect) = match alignment {
            Alignment::Left => (
                Rect::new(0.0, 0.0, width, final_size.height),
                Rect::new(width, 0.0, remaining_width, final_size.height),
            ),
            Alignment::Right => (
                Rect::new(final_size.width - width, 0.0, width, final_size.height),
                Rect::new(0.0, 0.0, remaining_width, final_size.height),
            ),
            Alignment::Top => (
                Rect::new(0.0, 0.0, final_size.width, height),
                Rect::new(0.0, height, final_size.width, remaining_height),
            ),
            Alignment::Bottom => (
                Rect::new(0.0, final_size.height - height, final_size.width, height),
                Rect::new(0.0, 0.0, final_size.width, remaining_height),
            ),
            Alignment::Unset => {
                this.arrange_overlay(final_size);
                return final_size;
            }
        };
        pinned.arrange(pinned_rect);
        if let Some(main) = main {
            main.arrange(main_rect);
        }
        final_size
    }
}
impl PinnedDockHostPanel {
    fn static_constructor() {
        ferroui_base::layout::Layoutable::affects_measure::<Self>(&[
            Self::pinned_dock_display_mode_property().as_property(),
            Self::pinned_dock_alignment_property().as_property(),
        ]);
        ferroui_base::layout::Layoutable::affects_arrange::<Self>(&[
            Self::pinned_dock_display_mode_property().as_property(),
            Self::pinned_dock_alignment_property().as_property(),
        ]);
    }
    /// Constructs native state for inheritance.
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
        }
    }
    /// Creates the original pinned host panel.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    /// Gets PinnedDockDisplayMode.
    pub fn pinned_dock_display_mode(&self) -> PinnedDockDisplayMode {
        self.get_value(Self::pinned_dock_display_mode_property())
    }
    /// Sets PinnedDockDisplayMode.
    pub fn set_pinned_dock_display_mode(&self, value: PinnedDockDisplayMode) {
        self.set_value(Self::pinned_dock_display_mode_property(), value);
    }
    /// Gets PinnedDockAlignment.
    pub fn pinned_dock_alignment(&self) -> Alignment {
        self.get_value(Self::pinned_dock_alignment_property())
    }
    /// Sets PinnedDockAlignment.
    pub fn set_pinned_dock_alignment(&self, value: Alignment) {
        self.set_value(Self::pinned_dock_alignment_property(), value);
    }
    fn resolve_children(&self) -> (Option<Ref<Control>>, Option<Ref<Control>>) {
        let (mut main, mut pinned) = (None, None);
        for child in self.children().snapshot().iter().cloned() {
            if pinned.is_none() && child.clone().cast::<PinnedDockControl>().is_some() {
                pinned = Some(child);
                continue;
            }
            if main.is_none() {
                main = Some(child);
            }
        }
        if main.is_none() && pinned.is_some() {
            for child in self.children().snapshot().iter().cloned() {
                if Some(&child) != pinned.as_ref() {
                    main = Some(child);
                    break;
                }
            }
        }
        if main.is_none() {
            main = pinned.clone();
        }
        (main, pinned)
    }
    fn has_pinned_dockable(&self, pinned: &Control) -> bool {
        if !pinned.is_visible() {
            return false;
        }
        let Some(root) = model_projection::root(self.data_context().as_ref()) else {
            return true;
        };
        let Some(pinned) = root.pinned_dock() else {
            return false;
        };
        pinned.visible_dockables().is_some_and(|v| v.count() > 0)
    }
    fn get_effective_display_mode(&self) -> PinnedDockDisplayMode {
        if let Some(root) = model_projection::root(self.data_context().as_ref()) {
            if let Some(mode) = Self::get_pinned_dockable(&*root)
                .and_then(|d| d.pinned_dock_display_mode_override())
            {
                return mode;
            }
        }
        self.pinned_dock_display_mode()
    }
    fn get_pinned_dockable(root: &dyn IRootDock) -> Option<Rc<dyn IDockable>> {
        let pinned = root.pinned_dock()?;
        if let Some(active) = pinned.active_dockable() {
            if active.clone().as_splitter().is_none()
                && pinned
                    .visible_dockables()
                    .is_some_and(|l| l.contains(&Some((active).clone())))
            {
                return Some(active);
            }
        }
        if let Some(list) = pinned.visible_dockables() {
            for dockable in list.iter() {
                if dockable
                    .clone()
                    .and_then(|item| item.as_splitter())
                    .is_none()
                {
                    return dockable;
                }
            }
        }
        None
    }
    fn get_effective_alignment(&self, pinned: Option<&Ref<Control>>) -> Alignment {
        if let Some(control) = pinned.and_then(|p| p.clone().cast::<PinnedDockControl>()) {
            if control.pinned_dock_alignment() != Alignment::Unset {
                return control.pinned_dock_alignment();
            }
        }
        if let Some(pinned) =
            model_projection::root(self.data_context().as_ref()).and_then(|r| r.pinned_dock())
        {
            if pinned.alignment() != Alignment::Unset {
                return pinned.alignment();
            }
        }
        self.pinned_dock_alignment()
    }
    fn measure_overlay(&self, available: Size) -> Size {
        let mut desired = Size::default();
        for child in self.children().snapshot().iter() {
            child.measure(available);
            desired = Size::new(
                max(desired.width, child.desired_size().width),
                max(desired.height, child.desired_size().height),
            );
        }
        desired
    }
    fn arrange_overlay(&self, size: Size) {
        let rect = Rect::new(0.0, 0.0, size.width, size.height);
        for child in self.children().snapshot().iter() {
            child.arrange(rect);
        }
    }
    fn get_remaining_size(available: Size, pinned: Size, alignment: Alignment) -> Size {
        match alignment {
            Alignment::Left | Alignment::Right => {
                Size::new(max(0.0, available.width - pinned.width), available.height)
            }
            Alignment::Top | Alignment::Bottom => {
                Size::new(available.width, max(0.0, available.height - pinned.height))
            }
            Alignment::Unset => available,
        }
    }
    fn get_desired_size(main: Size, pinned: Size, alignment: Alignment) -> Size {
        match alignment {
            Alignment::Left | Alignment::Right => {
                Size::new(main.width + pinned.width, max(main.height, pinned.height))
            }
            Alignment::Top | Alignment::Bottom => {
                Size::new(max(main.width, pinned.width), main.height + pinned.height)
            }
            Alignment::Unset => Size::new(
                max(main.width, pinned.width),
                max(main.height, pinned.height),
            ),
        }
    }
    fn get_pinned_size(pinned: &Control) -> Size {
        if pinned.is_visible() {
            pinned.desired_size()
        } else {
            Size::default()
        }
    }
}
fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        a
    } else if b.is_nan() {
        b
    } else if a > b || a == b && a.is_sign_positive() {
        a
    } else {
        b
    }
}
fn min(a: f64, b: f64) -> f64 {
    if a.is_nan() {
        a
    } else if b.is_nan() {
        b
    } else if a < b || a == b && a.is_sign_negative() {
        a
    } else {
        b
    }
}

#[cfg(test)]
#[path = "pinned_dock_host_panel_tests.rs"]
mod tests;
