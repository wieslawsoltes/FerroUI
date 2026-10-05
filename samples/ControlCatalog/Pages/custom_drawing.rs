//! Port of `Pages/CustomDrawing.xaml.cs`: the class of the document
//! `Pages/CustomDrawing.xaml`.

use super::CustomDrawingExampleControl;
use crate::markup::{content_page_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::ContentPage;
use std::cell::RefCell;
use std::f64::consts::PI;
use std::rc::Rc;

#[repr(C)]
pub struct CustomDrawing {
    base: ContentPage,
    // The field of the named element: assigned once the document is loaded.
    custom_drawing_control: RefCell<Option<Ref<CustomDrawingExampleControl>>>,
}

content_page_class!(CustomDrawing);
ferro_class_info!(CustomDrawing {
    new: CustomDrawing::new,
    markup: {
        methods: [
            fn RotateMinus(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CustomDrawing>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.rotate_minus(&sender, e.as_routed_event_args())
                },
            fn RotatePlus(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CustomDrawing>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.rotate_plus(&sender, e.as_routed_event_args())
                },
            fn ZoomIn(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CustomDrawing>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.zoom_in(&sender, e.as_routed_event_args())
                },
            fn ZoomOut(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CustomDrawing>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.zoom_out(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(CustomDrawing, "/Pages/CustomDrawing.xaml");

impl CustomDrawing {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct(), custom_drawing_control: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        *this.custom_drawing_control.borrow_mut() =
            this.find_control::<CustomDrawingExampleControl>("CustomDrawingControl");
        this
    }

    fn custom_drawing_control(&self) -> Option<Ref<CustomDrawingExampleControl>> {
        self.custom_drawing_control.borrow().clone()
    }

    /// # Panics
    /// Panics if the control does not exist yet (a null reference in the
    /// managed original).
    fn rotate_minus(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let control = self.custom_drawing_control().expect("the custom drawing control");
        control.set_rotation(control.rotation() - PI / 20.0);
    }

    fn rotate_plus(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(control) = self.custom_drawing_control() else { return };
        control.set_rotation(control.rotation() + PI / 20.0);
    }

    fn zoom_in(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(control) = self.custom_drawing_control() else { return };
        control.set_scale(control.scale() * 1.2);
    }

    fn zoom_out(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(control) = self.custom_drawing_control() else { return };
        control.set_scale(control.scale() / 1.2);
    }
}
