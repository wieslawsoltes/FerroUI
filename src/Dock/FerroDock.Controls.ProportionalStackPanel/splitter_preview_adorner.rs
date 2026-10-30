// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_base::controls::{IResourceHost, ResourceKey};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::media::{BoxShadows, Brushes, DrawingContext, IBrush};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Rect, Ref, StyledElement,
    StyledElementImpl, VisualImpl, VisualImplExt,
};
use ferroui_controls::primitives::AdornerLayer;
use ferroui_controls::{Control, ControlImpl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
#[repr(C)]
pub(crate) struct SplitterPreviewAdorner {
    base: Control,
    orientation: Cell<Orientation>,
    thickness: Cell<f64>,
    offset: Cell<f64>,
    background: RefCell<Option<Rc<dyn IBrush>>>,
}
ferro_class!(SplitterPreviewAdorner:Control);
ferro_impl_classes!(SplitterPreviewAdorner:FerroObjectImpl,StyledElementImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl SplitterPreviewAdorner {
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self {
            base: Control::construct(),
            orientation: Cell::new(Orientation::Horizontal),
            thickness: Cell::new(0.0),
            offset: Cell::new(0.0),
            background: RefCell::new(None),
        })
    }
    pub(crate) fn orientation(&self) -> Orientation {
        self.orientation.get()
    }
    pub(crate) fn thickness(&self) -> f64 {
        self.thickness.get()
    }
    pub(crate) fn offset(&self) -> f64 {
        self.offset.get()
    }
    pub(crate) fn background(&self) -> Option<Rc<dyn IBrush>> {
        self.background.borrow().clone()
    }
    pub(crate) fn set_orientation(&self, value: Orientation) {
        self.orientation.set(value);
    }
    pub(crate) fn set_thickness(&self, value: f64) {
        self.thickness.set(value);
    }
    pub(crate) fn set_offset(&self, value: f64) {
        self.offset.set(value);
    }
    pub(crate) fn set_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.background.replace(value);
    }
}
impl VisualImpl for SplitterPreviewAdorner {
    fn render(this: &Self, context: &mut DrawingContext) {
        Self::parent_render(this, context);
        let Some(host) = AdornerLayer::get_adorned_element(this) else {
            return;
        };
        let brush = this
            .background()
            .or_else(|| {
                let element: &StyledElement = &host;
                let resources: &dyn IResourceHost = element;
                let value = resources
                    .find_resource(&ResourceKey::from("DockApplicationAccentBrushIndicator"))?;
                let value =
                    ValueTypes::try_cast(&value, ValueType::of::<Option<Rc<dyn IBrush>>>())?;
                value
                    .downcast_ref::<Option<Rc<dyn IBrush>>>()
                    .cloned()
                    .flatten()
            })
            .unwrap_or_else(|| Brushes::gray());
        let rect = if this.orientation() == Orientation::Vertical {
            Rect::new(0.0, this.offset(), host.bounds().width, this.thickness())
        } else {
            Rect::new(this.offset(), 0.0, this.thickness(), host.bounds().height)
        };
        context.draw_rectangle(Some(&brush), None, rect, 0.0, 0.0, &BoxShadows::default());
    }
}
