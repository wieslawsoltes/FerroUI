use super::SelectableMixin;
use crate::i_selectable::ISelectable;
use crate::primitives::SelectingItemsControl;
use crate::{Control, ControlImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    Ref, StyledElementImpl, StyledProperty, VisualImpl,
};
use std::cell::Cell;
use std::rc::Rc;

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

impl ISelectable for TestControl {
    fn is_selected(&self) -> bool {
        self.get_value(Self::is_selected_property())
    }

    fn set_is_selected(&self, value: bool) {
        self.set_value(Self::is_selected_property(), value)
    }
}

impl TestControl {
    ferro_property!(
        fn is_selected_property() -> StyledProperty<bool> {
            FerroProperty::register::<TestControl, _>("IsSelected", false)
        }
    );

    fn class_init() {
        thread_local! {
            static DONE: Cell<bool> = const { Cell::new(false) };
        }
        if DONE.replace(true) {
            return;
        }

        SelectableMixin::attach::<TestControl>(Self::is_selected_property());
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
    let target = TestControl::new();

    target.set_is_selected(true);

    assert_eq!(*target.classes().snapshot(), vec![":selected".to_string()]);
}

#[test]
fn clearing_is_selected_should_remove_selected_class() {
    let target = TestControl::new();

    target.set_is_selected(true);
    target.set_is_selected(false);

    assert_eq!(target.classes().count(), 0);
}

#[test]
fn setting_is_selected_should_raise_is_selected_changed_event() {
    let target = TestControl::new();
    let raised = Rc::new(Cell::new(false));

    let r = raised.clone();
    target.add_handler(SelectingItemsControl::is_selected_changed_event(), move |_, _| r.set(true));

    target.set_is_selected(true);

    assert!(raised.get());
}
