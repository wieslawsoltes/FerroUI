//! Port of `Pages/Gestures/GesturePinchZoomPage.xaml.cs`: the class of the
//! document `Pages/Gestures/GesturePinchZoomPage.xaml`.

use crate::markup::xaml_class;
use ferroui_base::input::{
    InputElement, InputElementImpl, PinchEndedEventArgs, PinchEventArgs, ScrollGestureEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::rendering::composition::{CompositionVisual, ElementComposition};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, Vector3D,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{Button, ContentControlImpl, Control, ControlImpl, Image, UserControl};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
pub struct GesturePinchZoomPage {
    base: UserControl,
    is_init: Cell<bool>,
    current_scale: Cell<f64>,
}

ferro_class!(GesturePinchZoomPage: UserControl);
ferro_impl_classes!(
    GesturePinchZoomPage: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(GesturePinchZoomPage { new: GesturePinchZoomPage::new });
xaml_class!(GesturePinchZoomPage, "/Pages/Gestures/GesturePinchZoomPage.xaml");

impl VisualImpl for GesturePinchZoomPage {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        if this.is_init.get() {
            return;
        }

        this.is_init.set(true);

        let image = this.pinch_image();
        this.set_pinch_handlers(Some(image.clone().upcast()));

        // The handler belongs to a child of the page: it holds the page weakly.
        let weak = this.to_ref().downgrade();
        this.reset_button().click(move |_, _| {
            let composition_visual = ElementComposition::get_element_visual(&image);
            if let Some(composition_visual) = composition_visual {
                if let Some(this) = weak.upgrade() {
                    this.current_scale.set(1.0);
                }
                let props = composition_visual.visual_props();
                props.set_scale(&*composition_visual, Vector3D::new(1.0, 1.0, 1.0));
                props.set_offset(&*composition_visual, Vector3D::default());
                image.invalidate_measure();
            }
        });
    }
}

impl GesturePinchZoomPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), is_init: Cell::new(false), current_scale: Cell::new(0.0) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn reset_button(&self) -> Ref<Button> {
        self.get_control::<Button>("ResetButton")
    }

    fn pinch_image(&self) -> Ref<Image> {
        self.get_control::<Image>("PinchImage")
    }

    fn set_pinch_handlers(&self, control: Option<Ref<Control>>) {
        let Some(control) = control else {
            return;
        };

        self.current_scale.set(1.0);
        // The locals the handlers share. The handlers belong to `control`, a child of the page: they
        // hold the page and the control weakly.
        let current_offset = Rc::new(Cell::new(Vector3D::default()));
        let composition_visual: Rc<RefCell<Option<Rc<CompositionVisual>>>> = Rc::new(RefCell::new(None));
        let weak = self.to_ref().downgrade();
        let weak_control = control.downgrade();

        fn init_composition(
            composition_visual: &RefCell<Option<Rc<CompositionVisual>>>,
            visual: &Control,
        ) -> Option<Rc<CompositionVisual>> {
            let current = composition_visual.borrow().clone();
            if current.is_some() {
                return current;
            }

            let created = ElementComposition::get_element_visual(visual);
            *composition_visual.borrow_mut() = created.clone();
            created
        }

        {
            let (weak, weak_control) = (weak.clone(), weak_control.clone());
            let (current_offset, composition_visual) = (current_offset.clone(), composition_visual.clone());
            control.layout_updated(move || {
                let (Some(this), Some(control)) = (weak.upgrade(), weak_control.upgrade()) else {
                    return;
                };
                if let Some(visual) = init_composition(&composition_visual, &control) {
                    let scale = this.current_scale.get();
                    visual.visual_props().set_scale(&*visual, Vector3D::new(scale, scale, 1.0));

                    if current_offset.get() == Vector3D::default() {
                        current_offset.set(visual.visual_props().offset());
                    }
                }
            });
        }

        {
            let (weak, weak_control) = (weak.clone(), weak_control.clone());
            let composition_visual = composition_visual.clone();
            control.add_handler(InputElement::pinch_event(), move |_s, e: &PinchEventArgs| {
                let (Some(this), Some(control)) = (weak.upgrade(), weak_control.upgrade()) else {
                    return;
                };
                if let Some(visual) = init_composition(&composition_visual, &control) {
                    // The scale of the event is narrowed to single precision, as the original narrows it.
                    let mut scale = this.current_scale.get() * f64::from(e.scale() as f32);

                    if scale <= 1.0 {
                        scale = 1.0;
                        visual.visual_props().set_offset(&*visual, Vector3D::default());
                    }

                    visual.visual_props().set_scale(&*visual, Vector3D::new(scale, scale, 1.0));

                    e.set_handled(true);
                }
            });
        }

        {
            let (weak, weak_control) = (weak.clone(), weak_control.clone());
            let composition_visual = composition_visual.clone();
            control.add_handler(InputElement::pinch_ended_event(), move |_s, _e: &PinchEndedEventArgs| {
                let (Some(this), Some(control)) = (weak.upgrade(), weak_control.upgrade()) else {
                    return;
                };
                if let Some(visual) = init_composition(&composition_visual, &control) {
                    this.current_scale.set(visual.visual_props().scale().x);
                }
            });
        }

        control.add_handler(InputElement::scroll_gesture_event(), move |_s, e: &ScrollGestureEventArgs| {
            let (Some(this), Some(control)) = (weak.upgrade(), weak_control.upgrade()) else {
                return;
            };
            if let Some(visual) = init_composition(&composition_visual, &control) {
                let current_scale = this.current_scale.get();
                if current_scale != 1.0 {
                    let mut offset = current_offset.get();
                    offset += Vector3D::new(e.delta().x, e.delta().y, 0.0);

                    let bounds = control.bounds();
                    let current_size = bounds.size() * current_scale;

                    // The vertical offset is narrowed to single precision, as the original narrows it.
                    offset = Vector3D::new(
                        offset.x.clamp(0.0, current_size.width - bounds.width),
                        f64::from(offset.y.clamp(0.0, current_size.height - bounds.height) as f32),
                        0.0,
                    );
                    current_offset.set(offset);

                    visual.visual_props().set_offset(&*visual, offset * -1.0);
                    e.set_handled(true);
                }
            }
        });
    }
}
