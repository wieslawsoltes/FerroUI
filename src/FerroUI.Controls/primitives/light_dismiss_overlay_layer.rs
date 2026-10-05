use super::VisualLayerManager;
use crate::{Border, ControlImpl, TopLevel};
use ferroui_base::input::{InputElement, InputElementImpl};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::reactive::IDisposable;
use ferroui_base::rendering::ICustomHitTest;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Ref, StyledElementImpl,
    Visual, VisualImpl, WeakRef,
};
use std::cell::RefCell;
use std::rc::Rc;

/// A layer that is used to dismiss a popup when the user clicks outside
/// (internal upstream).
#[repr(C)]
pub struct LightDismissOverlayLayer {
    base: Border,
    registrations: RefCell<Vec<Rc<Registration>>>,
}

ferro_class!(LightDismissOverlayLayer: Border);
ferroui_base::ferro_class_info!(LightDismissOverlayLayer { new: LightDismissOverlayLayer::new });
ferro_impl_classes!(
    LightDismissOverlayLayer: StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl FerroObjectImpl for LightDismissOverlayLayer {}

impl VisualImpl for LightDismissOverlayLayer {
    fn custom_hit_test(this: &Self, point: Point) -> Option<bool> {
        Some(ICustomHitTest::hit_test(this, point))
    }

    fn custom_hit_test_geometry(
        this: &Self,
        geometry: &Ref<ferroui_base::media::Geometry>,
    ) -> Option<ferroui_base::media::IntersectionResult> {
        Some(ICustomHitTest::hit_test_geometry(this, geometry))
    }
}

impl ICustomHitTest for LightDismissOverlayLayer {
    fn hit_test(&self, point: Point) -> bool {
        LightDismissOverlayLayer::hit_test(self, point)
    }
}

impl LightDismissOverlayLayer {
    fn static_constructor() {
        let transparent: Rc<dyn IBrush> = Brushes::transparent();
        Border::background_property().override_default_value::<LightDismissOverlayLayer>(Some(transparent));
        Visual::is_visible_property().override_default_value::<LightDismissOverlayLayer>(false);
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self { base: Border::construct(), registrations: RefCell::new(Vec::new()) })
    }

    /// The element of the most recent registration that still receives
    /// input while the layer is shown.
    pub fn input_pass_through_element(&self) -> Option<Ref<InputElement>> {
        let last = self.registrations.borrow().last().cloned();
        last.and_then(|registration| registration.input_pass_through_element())
    }

    /// Returns the light dismiss overlay for a specified visual, or `None`
    /// if none found.
    pub fn get_light_dismiss_overlay_layer(visual: &Visual) -> Option<Ref<LightDismissOverlayLayer>> {
        let manager = match visual.to_ref().cast::<TopLevel>() {
            Some(top_level) => {
                top_level.get_template_descendants().into_iter().find_map(|v| v.cast::<VisualLayerManager>())
            }
            None => visual.find_ancestor_of_type::<VisualLayerManager>(false),
        };

        manager.map(|manager| manager.light_dismiss_overlay_layer())
    }

    /// Shows the layer until the returned registration is disposed.
    /// `input_pass_through_element` keeps receiving input while this is
    /// the most recent registration.
    pub fn register(&self, input_pass_through_element: Option<&Ref<InputElement>>) -> Rc<dyn IDisposable> {
        let registration = Rc::new(Registration {
            owner: RefCell::new(Some(self.to_ref().downgrade())),
            input_pass_through_element: input_pass_through_element.map(Ref::downgrade),
        });
        self.registrations.borrow_mut().push(registration.clone());
        self.update_state();
        registration
    }

    /// Whether the layer is hit at the specified point, in coordinates
    /// relative to the layer.
    pub fn hit_test(&self, point: Point) -> bool {
        if let Some(v) = self.input_pass_through_element() {
            if let Some(ie) = self.visual_root().and_then(|root| root.cast::<InputElement>()) {
                let this: Ref<Visual> = self.to_ref().upcast();
                if let Some(hit) = ie.input_hit_test_filtered(point, &|x: &Visual| !x.to_ref().ptr_eq(&this), true) {
                    return !v.is_visual_ancestor_of(&hit);
                }
            }
        }

        true
    }

    fn unregister(&self, registration: &Registration) {
        self.registrations.borrow_mut().retain(|r| !std::ptr::eq(Rc::as_ptr(r), registration));
        self.update_state();
    }

    fn update_state(&self) {
        let is_visible = !self.registrations.borrow().is_empty();
        self.set_is_visible(is_visible);
    }
}

struct Registration {
    owner: RefCell<Option<WeakRef<LightDismissOverlayLayer>>>,
    input_pass_through_element: Option<WeakRef<InputElement>>,
}

impl Registration {
    fn input_pass_through_element(&self) -> Option<Ref<InputElement>> {
        self.input_pass_through_element.as_ref().and_then(WeakRef::upgrade)
    }
}

impl IDisposable for Registration {
    fn dispose(&self) {
        let owner = self.owner.borrow_mut().take();
        if let Some(owner) = owner.and_then(|owner| owner.upgrade()) {
            owner.unregister(self);
        }
    }
}
