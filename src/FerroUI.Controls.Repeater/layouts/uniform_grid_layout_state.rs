// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License. See NOTICE.md in the containing module.

use super::uniform_grid_layout::GridLayoutParameters;
use super::{
    ElementRealizationOptions, FlowLayoutAlgorithm, UniformGridLayoutItemsStretch,
    VirtualizingLayoutContext,
};
use ferroui_base::layout::Orientation;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObject, FerroObjectImpl,
    Ref, Size,
};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct UniformGridLayoutState {
    base: FerroObject,
    pub flow_algorithm: FlowLayoutAlgorithm,
    effective_size: Cell<Size>,
    effective_size_valid: Cell<bool>,
}
ferro_class!(UniformGridLayoutState: FerroObject);
ferro_class_info!(UniformGridLayoutState {
    new: UniformGridLayoutState::new
});
ferro_impl_classes!(UniformGridLayoutState: FerroObjectImpl);
impl UniformGridLayoutState {
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            flow_algorithm: FlowLayoutAlgorithm::new(),
            effective_size: Cell::new(Size::default()),
            effective_size_valid: Cell::new(false),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn initialize_for_context(&self, context: &Ref<VirtualizingLayoutContext>) {
        self.flow_algorithm.initialize_for_context(context);
        context.set_layout_state(Some(Rc::new(self.to_ref())));
    }
    pub fn uninitialize_for_context(&self, context: &Ref<VirtualizingLayoutContext>) {
        self.flow_algorithm.uninitialize_for_context(context);
    }
    pub fn effective_item_width(&self) -> f64 {
        self.effective_size.get().width
    }
    pub fn effective_item_height(&self) -> f64 {
        self.effective_size.get().height
    }
    pub fn effective_size(&self) -> Size {
        self.effective_size.get()
    }
    pub fn invalidate_element_size(&self) {
        self.effective_size_valid.set(false);
    }
    pub(crate) fn ensure_element_size(
        &self,
        available: Size,
        context: &Ref<VirtualizingLayoutContext>,
        parameters: GridLayoutParameters,
    ) {
        if context.item_count() == 0 {
            return;
        }
        if let Some(element) = self.flow_algorithm.get_element_if_realized(0) {
            element.measure(Self::calculate_available_size(available, parameters));
            self.set_size(element.desired_size(), available, parameters);
        } else if !self.effective_size_valid.get() {
            let first = context
                .get_or_create_element_at_with_options(0, ElementRealizationOptions::FORCE_CREATE);
            first.measure(Self::calculate_available_size(available, parameters));
            self.set_size(first.desired_size(), available, parameters);
            context.recycle_element(&first);
        }
        self.effective_size_valid.set(true);
    }
    fn calculate_available_size(available: Size, p: GridLayoutParameters) -> Size {
        if p.orientation == Orientation::Horizontal
            && !p.item_width.is_nan()
            && available.width.is_finite()
        {
            let extra = if p.stretch == UniformGridLayoutItemsStretch::None {
                0.0
            } else {
                Self::extra_pixels(p.maximum, available.width, p.item_width, p.column_spacing)
            };
            Size::new((p.item_width + extra).max(0.0), available.height)
        } else if p.orientation == Orientation::Vertical
            && !p.item_height.is_nan()
            && available.height.is_finite()
        {
            let extra = if p.stretch == UniformGridLayoutItemsStretch::None {
                0.0
            } else {
                Self::extra_pixels(p.maximum, available.height, p.item_height, p.row_spacing)
            };
            Size::new(available.width, (p.item_height + extra).max(0.0))
        } else {
            available
        }
    }
    fn extra_pixels(maximum: u32, available: f64, item: f64, spacing: f64) -> f64 {
        let count = ((available / (item + spacing)).max(1.0) as u32)
            .max(1)
            .min(maximum.max(1));
        let used = count as f64 * (item + spacing) - spacing;
        // WinUI intentionally distributes only whole pixels using integer division.
        let remaining = (available - used) as i64;
        (remaining / count as i64) as f64
    }
    fn set_size(&self, desired: Size, available: Size, p: GridLayoutParameters) {
        let mut width = if p.item_width.is_nan() {
            desired.width
        } else {
            p.item_width
        };
        let mut height = if p.item_height.is_nan() {
            desired.height
        } else {
            p.item_height
        };
        let horizontal = p.orientation == Orientation::Horizontal;
        let minor = if horizontal { width } else { height };
        let available_minor = if horizontal {
            available.width
        } else {
            available.height
        };
        let spacing = if horizontal {
            p.column_spacing
        } else {
            p.row_spacing
        };
        let extra = if available_minor.is_finite() {
            Self::extra_pixels(p.maximum, available_minor, minor, spacing)
        } else {
            0.0
        };
        if p.stretch == UniformGridLayoutItemsStretch::Fill {
            if horizontal {
                width += extra;
            } else {
                height += extra;
            }
        } else if p.stretch == UniformGridLayoutItemsStretch::Uniform {
            let major = if horizontal { height } else { width };
            // A zero-sized template has no aspect ratio to preserve.
            let extra_major = if minor == 0.0 {
                0.0
            } else {
                major * extra / minor
            };
            if horizontal {
                width += extra;
                height += extra_major;
            } else {
                height += extra;
                width += extra_major;
            }
        }
        self.effective_size
            .set(Size::new(width.max(0.0), height.max(0.0)));
    }
}
