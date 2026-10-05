//! Port of `Pages/PointersPage.xaml.cs`: the class of the document
//! `Pages/PointersPage.xaml`.

use crate::markup::xaml_class;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{
    IPointer, InputElement, InputElementImpl, PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs,
};
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Border, ControlImpl, MultiPageImpl, PageImpl, SelectingMultiPageImpl, TabbedPage, TextBlock,
};
use std::rc::Rc;

#[repr(C)]
pub struct PointersPage {
    base: TabbedPage,
}

ferro_class!(PointersPage: TabbedPage);
ferro_impl_classes!(
    PointersPage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    PageImpl,
    MultiPageImpl,
    SelectingMultiPageImpl
);
ferro_class_info!(PointersPage {
    new: PointersPage::new,
    markup: {
        methods: [
            fn Border_PointerUpdated(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<PointersPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerEventArgs>() {
                        this.border_pointer_updated(&sender, e)
                    }
                },
            fn Border_PointerCaptureLost(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<PointersPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerCaptureLostEventArgs>() {
                        this.border_pointer_capture_lost(&sender, e)
                    }
                },
            fn Border_PointerReleased(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<PointersPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerReleasedEventArgs>() {
                        this.border_pointer_released(&sender, e)
                    }
                },
            fn Border_PointerPressed(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<PointersPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerPressedEventArgs>() {
                        this.border_pointer_pressed(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(PointersPage, "/Pages/PointersPage.xaml");

/// `sender as Border`.
fn as_border(sender: &Option<BoxedValue>) -> Option<Ref<Border>> {
    sender.as_ref().and_then(|sender| ValueTypes::as_object(&**sender)).and_then(|sender| sender.cast::<Border>())
}

/// `border.Child as TextBlock`.
fn child_text_block(border: &Border) -> Option<Ref<TextBlock>> {
    border.child().and_then(|child| child.cast::<TextBlock>())
}

/// `pointer.Captured == sender`: the reference comparison of the captured element and the
/// sender.
fn is_captured_by(pointer: &Rc<dyn IPointer>, sender: &Option<BoxedValue>) -> bool {
    match (pointer.captured(), sender) {
        (None, None) => true,
        (Some(captured), Some(sender)) => ValueTypes::as_object(&**sender)
            .and_then(|sender| sender.cast::<InputElement>())
            .is_some_and(|sender| captured == sender),
        _ => false,
    }
}

/// The text of a boolean, as the original prints it.
fn boolean_text(value: bool) -> &'static str {
    if value { "True" } else { "False" }
}

impl PointersPage {
    pub fn construct() -> Self {
        Self { base: TabbedPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn border_pointer_updated(&self, sender: &Option<BoxedValue>, e: &PointerEventArgs) {
        let Some(border) = as_border(sender) else {
            return;
        };
        if let Some(text_block) = child_text_block(&border) {
            let position = e.get_position(Some(&border));
            text_block.set_text(Some(&format!(
                "Type: {:?}\nCaptured: {}\nPointerId: {}\nPosition: {} {}",
                e.pointer().type_(),
                boolean_text(is_captured_by(e.pointer(), sender)),
                e.pointer().id(),
                position.x as i32,
                position.y as i32
            )));
            e.set_handled(true);
        }
    }

    fn border_pointer_capture_lost(&self, sender: &Option<BoxedValue>, e: &PointerCaptureLostEventArgs) {
        let Some(border) = as_border(sender) else {
            return;
        };
        if let Some(text_block) = child_text_block(&border) {
            text_block.set_text(Some(&format!(
                "Type: {:?}\nCaptured: {}\nPointerId: {}\nPosition: ??? ???",
                e.pointer().type_(),
                boolean_text(is_captured_by(e.pointer(), sender)),
                e.pointer().id()
            )));
            e.set_handled(true);
        }
    }

    /// # Panics
    /// Panics if the pointer is captured by another element (the invalid operation exception
    /// of the managed original).
    fn border_pointer_released(&self, sender: &Option<BoxedValue>, e: &PointerReleasedEventArgs) {
        if is_captured_by(e.pointer(), sender) {
            e.pointer().capture(None);
            e.set_handled(true);
        } else if e.pointer().captured().is_some() {
            panic!("How?");
        }
    }

    fn border_pointer_pressed(&self, sender: &Option<BoxedValue>, e: &PointerPressedEventArgs) {
        let border: Option<Ref<InputElement>> = as_border(sender).map(|border| border.upcast());
        e.pointer().capture(border.as_ref());
        e.set_handled(true);
        e.prevent_gesture_recognition();
    }
}
