//! Tests of element-reference properties: properties whose value is an
//! element that the owner of the property does not own.

use crate::primitives::{AdornerLayer, Popup};
use crate::testing::{TestServices, UnitTestApplication};
use crate::{Border, Control, Label, Window};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::InputElement;
use ferroui_base::layout::ILayoutManager;
use ferroui_base::reactive::{IObservable, IObserver, LightweightSubject};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{BoxedValue, ElementRef, Ref, Visual};
use std::rc::Rc;

#[test]
fn typed_set_and_get_round_trip_a_handle() {
    let _scope = Dispatcher::unit_test_scope();
    let target = Border::new();
    let popup = Popup::new();

    popup.set_placement_target(&target);

    assert_eq!(Some(target.clone().upcast::<Control>()), popup.placement_target());
    assert!(popup.get_value(Popup::placement_target_property()).unwrap().points_to(&target));

    popup.set_placement_target(None);

    assert!(popup.placement_target().is_none());
}

#[test]
fn untyped_set_accepts_a_boxed_handle_and_a_nullable_handle() {
    let _scope = Dispatcher::unit_test_scope();
    let target: Ref<Control> = Border::new().upcast();
    let popup = Popup::new();
    let property = Popup::placement_target_property().as_property();

    let _ = popup.set_value_untyped(property, &target, BindingPriority::LocalValue);
    assert_eq!(Some(target.clone()), popup.placement_target());

    let _ = popup.set_value_untyped(property, &Option::<Ref<Control>>::None, BindingPriority::LocalValue);
    assert!(popup.placement_target().is_none());

    let _ = popup.set_value_untyped(property, &Some(target.clone()), BindingPriority::LocalValue);
    assert_eq!(Some(target.clone()), popup.placement_target());

    popup.set_current_value_untyped(property, &Option::<Ref<Control>>::None);
    assert!(popup.placement_target().is_none());

    // The stored value is the element reference.
    let _ = popup.set_value_untyped(property, &target, BindingPriority::LocalValue);
    let stored = popup.get_value_untyped(property);
    let stored = stored.downcast_ref::<Option<ElementRef<Control>>>().expect("the value is an element reference");
    assert!(stored.as_ref().unwrap().points_to(&target));
}

#[test]
fn untyped_binding_converts_handles() {
    let _scope = Dispatcher::unit_test_scope();
    let target: Ref<Control> = Border::new().upcast();
    let popup = Popup::new();
    let source = Rc::new(LightweightSubject::<BoxedValue>::new());
    let observable: Rc<dyn IObservable<BoxedValue>> = source.clone();

    let binding = popup.bind_property_untyped(
        Popup::placement_target_property().as_property(),
        observable,
        BindingPriority::LocalValue,
    );

    source.on_next(Rc::new(target.clone()));
    assert_eq!(Some(target.clone()), popup.placement_target());

    source.on_next(Rc::new(Option::<Ref<Control>>::None));
    assert!(popup.placement_target().is_none());

    source.on_next(Rc::new(Some(target.clone())));
    assert_eq!(Some(target), popup.placement_target());

    binding.dispose();
}

#[test]
fn a_dropped_element_reads_as_nothing_and_is_not_kept_alive() {
    let _scope = Dispatcher::unit_test_scope();
    let popup = Popup::new();
    let label = Label::new();
    let adorner = Border::new();

    let weak = {
        let target = Border::new();
        popup.set_placement_target(&target);
        label.set_target(&target);
        AdornerLayer::set_adorned_element(&adorner, &target);

        assert_eq!(Some(target.clone().upcast::<InputElement>()), label.target());
        assert_eq!(Some(target.clone().upcast::<Visual>()), AdornerLayer::get_adorned_element(&adorner));
        target.downgrade()
    };

    assert!(weak.upgrade().is_none());
    assert!(popup.placement_target().is_none());
    assert!(label.target().is_none());
    assert!(AdornerLayer::get_adorned_element(&adorner).is_none());
}

/// The pattern of a control whose template holds a popup placed on the
/// control itself (a combo box): the placement target is an ancestor of
/// the popup. Nothing keeps the tree alive once the window has closed.
#[test]
fn popup_with_an_ancestor_placement_target_is_freed_after_the_window_closes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());

    let (weak_window, weak_owner, weak_popup) = {
        let window = Window::new();
        let owner = Border::new();
        let popup = Popup::new();
        popup.set_child(Border::new());
        popup.set_placement_target(&owner);
        owner.set_child(&popup);
        window.set_content(Some(Control::boxed(owner.clone())));
        window.show();
        window.layout_manager().execute_layout_pass();

        popup.open();
        assert!(popup.is_open());
        assert_eq!(Some(owner.clone().upcast::<Control>()), popup.placement_target());

        window.close();
        assert!(!popup.is_open());

        (window.downgrade(), owner.downgrade(), popup.downgrade())
    };
    Dispatcher::ui_thread().run_jobs(None);

    assert!(weak_window.upgrade().is_none());
    assert!(weak_owner.upgrade().is_none());
    assert!(weak_popup.upgrade().is_none());
}

/// The same with the window itself as the explicit placement target.
#[test]
fn popup_placed_on_its_window_is_freed_after_the_window_closes() {
    let _app = UnitTestApplication::start(TestServices::styled_window());

    let (weak_window, weak_popup) = {
        let window = Window::new();
        let popup = Popup::new();
        popup.set_child(Border::new());
        popup.set_placement_target(&window);
        window.set_content(Some(Control::boxed(popup.clone())));
        window.show();
        window.layout_manager().execute_layout_pass();

        popup.open();
        assert!(popup.is_open());

        window.close();

        (window.downgrade(), popup.downgrade())
    };
    Dispatcher::ui_thread().run_jobs(None);

    assert!(weak_window.upgrade().is_none());
    assert!(weak_popup.upgrade().is_none());
}
