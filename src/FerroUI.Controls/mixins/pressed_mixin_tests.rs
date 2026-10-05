use super::PressedMixin;
use crate::mouse_test_helper::MouseTestHelper;
use crate::{Control, ControlImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt, Ref, StyledElementImpl,
    VisualImpl,
};
use std::cell::Cell;

#[repr(C)]
struct TestControl {
    base: Control,
}

ferro_class!(TestControl: Control);
ferro_impl_classes!(
    TestControl: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl FerroObjectImpl for TestControl {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        Self::class_init();
    }
}

impl TestControl {
    fn class_init() {
        thread_local! {
            static DONE: Cell<bool> = const { Cell::new(false) };
        }
        if DONE.replace(true) {
            return;
        }

        PressedMixin::attach::<TestControl>();
    }

    fn new() -> Ref<Self> {
        instantiate(Self { base: Control::construct() })
    }
}

#[test]
fn selected_class_should_not_initially_be_added() {
    let target = TestControl::new();

    assert_eq!(target.classes().count(), 0);
}

#[test]
fn setting_is_selected_should_add_selected_class() {
    let mouse = MouseTestHelper::new();
    let target = TestControl::new();

    mouse.down(&target);

    assert_eq!(*target.classes().snapshot(), vec![":pressed".to_string()]);
}
