// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    media::{BoxShadows, Brushes, DrawingContext, IPen, Pen},
    FerroObjectImpl, Rect, Ref, StyledElementImpl, VisualImpl, VisualImplExt,
};
use ferroui_controls::{Control, ControlImpl};
use std::{cell::Cell, rc::Rc};

/// Adorner that highlights a selected control with the original yellow rectangle.
#[repr(C)]
pub(crate) struct SelectionOverlayAdorner {
    base: Control,
    highlight_rect: Cell<Option<Rect>>,
}
ferro_class!(SelectionOverlayAdorner: Control);
ferro_impl_classes!(SelectionOverlayAdorner: FerroObjectImpl, StyledElementImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ControlImpl);
ferro_class_info!(SelectionOverlayAdorner {
    new: SelectionOverlayAdorner::new,
    markup: { properties: [
        HighlightRect: Option<Rect> {
            get: |this: &Ref<SelectionOverlayAdorner>| this.highlight_rect(),
            set: |this: &Ref<SelectionOverlayAdorner>, value| this.set_highlight_rect(value),
        },
    ] },
});
impl VisualImpl for SelectionOverlayAdorner {
    fn render(this: &Self, context: &mut DrawingContext<'_>) {
        Self::parent_render(this, context);
        if let Some(rect) = this.highlight_rect() {
            PEN.with(|pen| {
                context.draw_rectangle(None, Some(pen), rect, 0.0, 0.0, &BoxShadows::default());
            });
        }
    }
}
ferroui_base::ferro_thread_local! {
    static PEN: Rc<dyn IPen> = Pen::with_brush(Some(Brushes::yellow()), 2.0).into();
}
impl SelectionOverlayAdorner {
    pub(crate) fn construct() -> Self {
        Self {
            base: Control::construct(),
            highlight_rect: Cell::new(None),
        }
    }
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub(crate) fn highlight_rect(&self) -> Option<Rect> {
        self.highlight_rect.get()
    }
    pub(crate) fn set_highlight_rect(&self, value: Option<Rect>) {
        self.highlight_rect.set(value);
    }
}
