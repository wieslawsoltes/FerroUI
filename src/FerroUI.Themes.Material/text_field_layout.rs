// Copyright 2026 FerroUI contributors.
// SPDX-License-Identifier: Apache-2.0
//! Material text-field layout, based on the component geometry in AndroidX
//! `internal/TextFieldImpl.kt`, revision e4d933c98df7baa7ea2b83eea6e086915961e0ff.
//! Layout remains in logical start/end coordinates; FerroUI's normal flow
//! direction policy mirrors the panel and counter-mirrors text for RTL.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl};
use ferroui_base::{
    AttachedProperty, FerroObjectImpl, FerroProperty, Rect, Ref, Size, StyledElementImpl,
    StyledProperty, Thickness, VisualImpl, ferro_class, ferro_class_info, ferro_impl_classes,
    ferro_properties, instantiate,
};
use ferroui_controls::{Control, ControlImpl, Panel, PanelImpl, TextBlock};

/// A slot of [`MaterialTextFieldLayout`]. Each slot has at most one direct child.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MaterialTextFieldSlot {
    Container,
    Label,
    Leading,
    Trailing,
    Prefix,
    Suffix,
    #[default]
    Editor,
    Placeholder,
    Supporting,
    InnerLeading,
    InnerTrailing,
    Clear,
    Reveal,
}
ferroui_base::ferro_markup_enum!(
    MaterialTextFieldSlot {
        Container, Label, Leading, Trailing, Prefix, Suffix, Editor, Placeholder,
        Supporting, InnerLeading, InnerTrailing, Clear, Reveal
    },
    { namespace: "FerroUI.Themes.Material" }
);

/// Arranges a real text editor together with Material decoration. In particular,
/// outlined labels interpolate from the editor's start edge to the 16dp outline
/// inset, independently of leading-icon and prefix widths. Supporting content
/// contributes to the control's height without stretching the painted container.
#[repr(C)]
pub struct MaterialTextFieldLayout {
    base: Panel,
}
ferro_class!(MaterialTextFieldLayout: Panel);
ferro_class_info!(MaterialTextFieldLayout {
    new: MaterialTextFieldLayout::new
});
ferro_impl_classes!(MaterialTextFieldLayout:
    FerroObjectImpl, StyledElementImpl, VisualImpl, InteractiveImpl, InputElementImpl,
    ControlImpl, PanelImpl
);

ferro_properties! { impl MaterialTextFieldLayout {
    pub fn slot_property() -> AttachedProperty<MaterialTextFieldSlot> {
        FerroProperty::register_attached::<MaterialTextFieldLayout, Control, _>(
            "Slot", MaterialTextFieldSlot::Editor)
    }
    /// 0 is the resting label and 1 is minimized. A spatial spring may overshoot.
    pub fn label_progress_property() -> StyledProperty<f64> {
        FerroProperty::register::<MaterialTextFieldLayout, _>("LabelProgress", 0.0)
    }
    pub fn is_outlined_property() -> StyledProperty<bool> {
        FerroProperty::register::<MaterialTextFieldLayout, _>("IsOutlined", false)
    }
    pub fn single_line_property() -> StyledProperty<bool> {
        FerroProperty::register::<MaterialTextFieldLayout, _>("SingleLine", true)
    }
    pub fn content_padding_property() -> StyledProperty<Thickness> {
        FerroProperty::register::<MaterialTextFieldLayout, _>("ContentPadding", Thickness::uniform(16.0))
    }
    pub fn container_min_height_property() -> StyledProperty<f64> {
        FerroProperty::register::<MaterialTextFieldLayout, _>("ContainerMinHeight", 56.0)
    }
} }

type Slots = [Option<Ref<Control>>; 13];

impl MaterialTextFieldLayout {
    fn static_constructor() {
        Layoutable::affects_measure::<Self>(&[
            Self::label_progress_property().as_property(),
            Self::is_outlined_property().as_property(),
            Self::single_line_property().as_property(),
            Self::content_padding_property().as_property(),
            Self::container_min_height_property().as_property(),
        ]);
        Panel::affects_parent_measure::<Self>(&[Self::slot_property().as_property()]);
    }
    pub fn construct() -> Self {
        Self {
            base: Panel::construct(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn get_slot(child: &Control) -> MaterialTextFieldSlot {
        child.get_value(Self::slot_property())
    }
    pub fn set_slot(child: &Control, value: MaterialTextFieldSlot) {
        child.set_value(Self::slot_property(), value);
    }
    pub fn label_progress(&self) -> f64 {
        self.get_value(Self::label_progress_property())
    }
    pub fn set_label_progress(&self, value: f64) {
        self.set_value(Self::label_progress_property(), value);
    }
    pub fn is_outlined(&self) -> bool {
        self.get_value(Self::is_outlined_property())
    }
    pub fn set_is_outlined(&self, value: bool) {
        self.set_value(Self::is_outlined_property(), value);
    }
    pub fn single_line(&self) -> bool {
        self.get_value(Self::single_line_property())
    }
    pub fn set_single_line(&self, value: bool) {
        self.set_value(Self::single_line_property(), value);
    }
    pub fn content_padding(&self) -> Thickness {
        self.get_value(Self::content_padding_property())
    }
    pub fn set_content_padding(&self, value: Thickness) {
        self.set_value(Self::content_padding_property(), value);
    }
    pub fn container_min_height(&self) -> f64 {
        self.get_value(Self::container_min_height_property())
    }
    pub fn set_container_min_height(&self, value: f64) {
        self.set_value(Self::container_min_height_property(), value);
    }

    fn slots(&self) -> Slots {
        let mut slots = std::array::from_fn(|_| None);
        for child in self.children().snapshot().iter() {
            let slot = Self::get_slot(&child) as usize;
            slots[slot] = Some(child.clone());
        }
        slots
    }

    fn progress(&self) -> f64 {
        let p = self.label_progress();
        if p.is_finite() {
            p.clamp(-0.25, 1.25)
        } else {
            0.0
        }
    }

    fn label_typography(&self, slots: &Slots) {
        if let Some(label) = slots[MaterialTextFieldSlot::Label as usize]
            .as_ref()
            .and_then(|label| label.cast::<TextBlock>())
        {
            let p = self.progress();
            label.set_current_value(TextBlock::font_size_property(), 16.0 - 4.0 * p);
            label.set_current_value(TextBlock::line_height_property(), 24.0 - 8.0 * p);
            label.set_current_value(TextBlock::letter_spacing_property(), 0.5 - 0.1 * p);
        }
    }

    fn padding(&self, slots: &Slots) -> Thickness {
        use MaterialTextFieldSlot::*;
        let p = self.content_padding();
        let label_inset = if visible(slots, Label) && !self.is_outlined() {
            8.0 * self.progress().clamp(0.0, 1.0)
        } else {
            0.0
        };
        Thickness::new(
            (positive(p.left) - if visible(slots, Leading) { 12.0 } else { 0.0 }).max(0.0),
            positive(p.top) + label_inset,
            (positive(p.right) - if visible(slots, Trailing) { 12.0 } else { 0.0 }).max(0.0),
            (positive(p.bottom) - label_inset).max(0.0),
        )
    }
}

impl LayoutableImpl for MaterialTextFieldLayout {
    fn measure_override(this: &Self, available: Size) -> Size {
        use MaterialTextFieldSlot::*;
        let slots = this.slots();
        this.label_typography(&slots);
        let padding = this.padding(&slots);
        let mut remaining = available.width.max(0.0);
        let mut icons_width = 0.0_f64;
        let mut icons_height = 0.0_f64;
        for slot in [
            InnerLeading,
            Leading,
            InnerTrailing,
            Trailing,
            Reveal,
            Clear,
        ] {
            measure(&slots, slot, Size::new(remaining, available.height));
            let size = desired(&slots, slot);
            icons_width += size.width;
            icons_height = icons_height.max(size.height);
            remaining = (remaining - size.width).max(0.0);
        }
        let expanded_label_width = (remaining - padding.left - padding.right).max(0.0);
        let label_width = if this.is_outlined() && available.width.is_finite() {
            let minimized_width = (available.width
                - positive(this.content_padding().left)
                - positive(this.content_padding().right))
            .max(0.0);
            expanded_label_width
                + (minimized_width - expanded_label_width) * this.progress().clamp(0.0, 1.0)
        } else {
            expanded_label_width
        };
        measure(&slots, Label, Size::new(label_width, available.height));
        remaining = (remaining - padding.left - padding.right).max(0.0);
        let mut affix_width = 0.0_f64;
        for slot in [Prefix, Suffix] {
            measure(&slots, slot, Size::new(remaining, available.height));
            if visible(&slots, slot) {
                let width = desired(&slots, slot).width + 2.0;
                affix_width += width;
                remaining = (remaining - width).max(0.0);
            }
        }
        let editor_available = Size::new(
            remaining,
            (available.height - padding.top - padding.bottom).max(0.0),
        );
        measure(&slots, Editor, editor_available);
        measure(&slots, Placeholder, editor_available);
        let editor = desired(&slots, Editor);
        let placeholder = desired(&slots, Placeholder);
        let line_height = editor
            .height
            .max(placeholder.height)
            .max(desired(&slots, Prefix).height)
            .max(desired(&slots, Suffix).height)
            .max(24.0);
        let field_height = positive(this.container_min_height())
            .max(icons_height)
            .max(line_height + padding.top + padding.bottom);
        let text_width = icons_width
            + affix_width
            + editor.width.max(placeholder.width)
            + padding.left
            + padding.right;
        let label_width = desired(&slots, Label).width
            + if this.is_outlined() {
                positive(this.content_padding().left) + positive(this.content_padding().right)
            } else {
                icons_width + padding.left + padding.right
            };
        measure(&slots, Container, Size::new(available.width, field_height));
        let width = text_width
            .max(label_width)
            .max(desired(&slots, Container).width);
        let support_width = (available.width - 32.0).max(0.0);
        measure(
            &slots,
            Supporting,
            Size::new(support_width, available.height),
        );
        let supporting = desired(&slots, Supporting);
        // Native cutout labels reserve half the minimized line height above the
        // outline, even while resting, so adjacent controls never cover the label.
        let top = if this.is_outlined() && visible(&slots, Label) {
            8.0
        } else {
            0.0
        };
        let bottom = if visible(&slots, Supporting) {
            supporting.height.max(16.0) + 4.0
        } else {
            0.0
        };
        Size::new(
            width.max(if visible(&slots, Supporting) {
                supporting.width + 32.0
            } else {
                0.0
            }),
            field_height + top + bottom,
        )
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        use MaterialTextFieldSlot::*;
        let slots = this.slots();
        let padding = this.padding(&slots);
        let has_label = visible(&slots, Label);
        let top = if this.is_outlined() && has_label {
            8.0
        } else {
            0.0
        };
        let support_height = if visible(&slots, Supporting) {
            desired(&slots, Supporting).height.max(16.0)
        } else {
            0.0
        };
        let support_gap = if visible(&slots, Supporting) {
            4.0
        } else {
            0.0
        };
        let field_height = (final_size.height - top - support_height - support_gap).max(0.0);
        arrange(
            &slots,
            Container,
            Rect::new(0.0, top, final_size.width, field_height),
        );
        let mut start = 0.0;
        let mut end = final_size.width;
        for slot in [InnerLeading, Leading] {
            let size = desired(&slots, slot);
            arrange(
                &slots,
                slot,
                Rect::new(
                    start,
                    top + (field_height - size.height) / 2.0,
                    size.width,
                    size.height,
                ),
            );
            start += size.width;
        }
        for slot in [InnerTrailing, Trailing, Reveal, Clear] {
            let size = desired(&slots, slot);
            end -= size.width;
            arrange(
                &slots,
                slot,
                Rect::new(
                    end,
                    top + (field_height - size.height) / 2.0,
                    size.width,
                    size.height,
                ),
            );
        }
        let label_start = start + padding.left;
        start += padding.left;
        end -= padding.right;
        let content_height = (field_height - padding.top - padding.bottom).max(0.0);
        let y = top + padding.top;
        for slot in [Prefix, Suffix] {
            let size = desired(&slots, slot);
            let slot_x = if slot == Prefix {
                start
            } else {
                end - size.width
            };
            let slot_y = if this.single_line() {
                y + (content_height - size.height) / 2.0
            } else {
                y
            };
            arrange(
                &slots,
                slot,
                Rect::new(slot_x, slot_y, size.width, size.height),
            );
            if visible(&slots, slot) {
                if slot == Prefix {
                    start += size.width + 2.0;
                } else {
                    end -= size.width + 2.0;
                }
            }
        }
        let editor_rect = Rect::new(start, y, (end - start).max(0.0), content_height);
        arrange(&slots, Editor, editor_rect);
        arrange(&slots, Placeholder, editor_rect);
        let label = desired(&slots, Label);
        let p = this.progress();
        let start_y = if this.single_line() {
            (field_height - label.height) / 2.0
        } else {
            positive(this.content_padding().top)
        };
        let target_y = if this.is_outlined() {
            -label.height / 2.0
        } else {
            8.0
        };
        let target_x = if this.is_outlined() {
            positive(this.content_padding().left)
        } else {
            label_start
        };
        let label_rect = Rect::new(
            label_start + (target_x - label_start) * p,
            top + start_y + (target_y - start_y) * p,
            label.width,
            label.height,
        );
        arrange(&slots, Label, label_rect);
        if let Some(border) = slots[Container as usize]
            .as_ref()
            .and_then(|control| control.cast::<crate::MaterialTextFieldBorder>())
        {
            border.set_notch_start(label_rect.x);
            border.set_notch_width(label_rect.width);
            border.set_notch_open(if this.is_outlined() && has_label {
                p
            } else {
                0.0
            });
        }
        arrange(
            &slots,
            Supporting,
            Rect::new(
                16.0,
                top + field_height + support_gap,
                (final_size.width - 32.0).max(0.0),
                support_height,
            ),
        );
        final_size
    }
}

fn positive(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
fn visible(slots: &Slots, slot: MaterialTextFieldSlot) -> bool {
    slots[slot as usize]
        .as_ref()
        .is_some_and(|child| child.is_visible())
}
fn desired(slots: &Slots, slot: MaterialTextFieldSlot) -> Size {
    slots[slot as usize]
        .as_ref()
        .map_or(Size::default(), |child| child.desired_size())
}
fn measure(slots: &Slots, slot: MaterialTextFieldSlot, available: Size) {
    if let Some(child) = &slots[slot as usize] {
        child.measure(available);
    }
}
fn arrange(slots: &Slots, slot: MaterialTextFieldSlot, rect: Rect) {
    if let Some(child) = &slots[slot as usize] {
        child.arrange(rect);
    }
}
