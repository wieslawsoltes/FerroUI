// Copyright 2026 FerroUI contributors.
// SPDX-License-Identifier: Apache-2.0
//! Native top-app-bar row layout, including whole-bar centering and baseline padding.
use crate::app_bar::MaterialAppBar;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    FerroObjectImpl, Rect, Ref, Size, StyledElementImpl, VisualImpl, WeakRef, ferro_class,
    ferro_class_info, ferro_impl_classes, instantiate,
};
use ferroui_controls::{Border, Control, ControlImpl, Panel, PanelImpl, TextBlock};
use std::cell::{Cell, RefCell};

/// The app-bar template's four slots: navigation, compact title, actions, expanded title clip.
#[repr(C)]
pub struct MaterialAppBarLayout {
    base: Panel,
    owner: RefCell<Option<WeakRef<MaterialAppBar>>>,
    top_height: Cell<f64>,
    bottom_height: Cell<f64>,
}
ferro_class!(MaterialAppBarLayout: Panel);
ferro_class_info!(MaterialAppBarLayout {
    new: MaterialAppBarLayout::new
});
ferro_impl_classes!(MaterialAppBarLayout: FerroObjectImpl, StyledElementImpl, VisualImpl,
    InteractiveImpl, InputElementImpl, ControlImpl, PanelImpl);

impl MaterialAppBarLayout {
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
            owner: RefCell::new(None),
            top_height: Cell::new(64.0),
            bottom_height: Cell::new(0.0),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub(crate) fn set_owner(&self, owner: &MaterialAppBar) {
        *self.owner.borrow_mut() = Some(owner.to_ref().downgrade());
        self.invalidate_measure();
    }
    fn owner(&self) -> Option<Ref<MaterialAppBar>> {
        self.owner.borrow().as_ref().and_then(WeakRef::upgrade)
    }
}

impl LayoutableImpl for MaterialAppBarLayout {
    fn measure_override(this: &Self, available: Size) -> Size {
        let Some(owner) = this.owner() else {
            return Size::default();
        };
        let slots = this.children().snapshot();
        if slots.len() < 4 {
            return Size::default();
        }
        let insets = owner.safe_insets();
        let available_width = (available.width - insets.left - insets.right).max(0.0);
        slots[0].measure(Size::new(available_width, f64::INFINITY));
        slots[2].measure(Size::new(available_width, f64::INFINITY));
        let start = slots[0].desired_size().width.max(12.0);
        let end = slots[2].desired_size().width;
        slots[1].measure(Size::new(
            (available_width - start - end).max(0.0),
            f64::INFINITY,
        ));
        let compact = slots[1].desired_size();
        let expanded = owner.size_variant().has_two_rows();
        slots[3].set_is_visible(expanded);
        slots[3].measure(Size::new(available_width, f64::INFINITY));
        let top_height = if expanded {
            owner.collapsed_height()
        } else {
            owner.effective_expanded_height()
        }
        .max(compact.height);
        let bottom_height = if expanded {
            (owner.effective_expanded_height() - owner.collapsed_height())
                .max(slots[3].desired_size().height)
        } else {
            0.0
        };
        this.top_height.set(top_height);
        this.bottom_height.set(bottom_height);
        owner.set_layout_heights(top_height, bottom_height);
        let natural_width = (start + compact.width + end).max(slots[3].desired_size().width + 24.0);
        Size::new(
            if available_width.is_finite() {
                available_width
            } else {
                natural_width
            } + insets.left
                + insets.right,
            (top_height + bottom_height + owner.height_offset()).max(0.0) + insets.top,
        )
    }

    fn arrange_override(this: &Self, size: Size) -> Size {
        let Some(owner) = this.owner() else {
            return size;
        };
        let slots = this.children().snapshot();
        if slots.len() < 4 {
            return size;
        }
        let insets = owner.safe_insets();
        let width = (size.width - insets.left - insets.right).max(0.0);
        let expanded = owner.size_variant().has_two_rows();
        let top_height = if expanded {
            this.top_height.get()
        } else {
            (this.top_height.get() + owner.height_offset()).max(0.0)
        };
        let nav = slots[0].desired_size();
        let actions = slots[2].desired_size();
        slots[0].arrange(Rect::new(
            insets.left,
            insets.top + (top_height - nav.height) / 2.0,
            nav.width,
            nav.height,
        ));
        slots[2].arrange(Rect::new(
            insets.left + width - actions.width,
            insets.top + (top_height - actions.height) / 2.0,
            actions.width,
            actions.height,
        ));
        let title = slots[1].desired_size();
        let start = nav.width.max(12.0);
        let compact_x = title_x(
            width,
            title.width,
            start,
            actions.width,
            owner.is_title_centered(),
        );
        slots[1].arrange(Rect::new(
            insets.left + compact_x,
            insets.top + (top_height - title.height) / 2.0,
            title.width,
            title.height,
        ));
        if expanded {
            let visible_height = (this.bottom_height.get() + owner.height_offset()).max(0.0);
            slots[3].arrange(Rect::new(
                insets.left,
                insets.top + top_height,
                width,
                visible_height,
            ));
            if let Some(clip) = slots[3].cast::<Border>() {
                if let Some(title) = clip.child() {
                    let desired = title.desired_size();
                    let baseline = last_baseline(&title);
                    let padding = owner.size_variant().title_metrics().2;
                    let from_bottom = padding - (desired.height - baseline);
                    let adjusted = from_bottom
                        .min(this.bottom_height.get() - desired.height)
                        .max(0.0);
                    let x = title_x(width, desired.width, 12.0, 4.0, owner.is_title_centered());
                    title.arrange(Rect::new(
                        x,
                        visible_height - desired.height - adjusted,
                        desired.width,
                        desired.height,
                    ));
                }
            }
        }
        size
    }
}

fn title_x(width: f64, title_width: f64, start: f64, end: f64, centered: bool) -> f64 {
    let initial = if centered {
        (width - title_width) / 2.0
    } else {
        0.0
    };
    initial
        .max(start)
        .min((width - end - title_width).max(start))
}

fn last_baseline(title: &Control) -> f64 {
    let mut result = 0.0;
    let mut offset = 0.0;
    for text in title
        .get_visual_descendants()
        .filter_map(|item| item.cast::<TextBlock>())
        .filter(|text| text.is_visible())
    {
        let layout = text.text_layout();
        let baseline = layout.text_lines().last().map_or(0.0, |line| {
            layout.height() - line.height() + line.baseline()
        });
        result = offset + baseline;
        offset += text.desired_size().height;
    }
    result
}
