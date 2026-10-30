//! Callback-ownership regressions for separate-window and overlay framework test hosts.

use super::*;
use crate::ControlImpl;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::ITransform;
use ferroui_base::{FerroObjectImpl, Matrix, StyledElementImpl, VisualImpl, WeakRef};

struct Fixture {
    flyout: Ref<Flyout>,
    first: Ref<Button>,
    second: Ref<Button>,
    panel: Ref<StackPanel>,
    window: Ref<Window>,
    _app: UnitTestApplicationScope,
}

impl Fixture {
    fn new(overlay: bool) -> Self {
        let app = create_services_with_focus(overlay);
        let first = Button::new();
        let second = Button::new();
        let panel = StackPanel::new();
        panel.children().add(first.clone());
        panel.children().add(second.clone());
        let window = prepared_window(Some(panel.clone().upcast()));
        window.show();
        let flyout = Flyout::new();
        flyout.set_show_mode(FlyoutShowMode::Transient);
        Self {
            flyout,
            first,
            second,
            panel,
            window,
            _app: app,
        }
    }

    fn assert_owner(&self, owner: &Ref<Button>) {
        assert!(self.flyout.is_open());
        assert!(self.flyout.popup().is_open());
        assert_eq!(self.flyout.target(), Some(owner.clone().upcast()));
        assert_eq!(
            self.flyout.popup().placement_target(),
            Some(owner.clone().upcast())
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.window.close();
    }
}

fn popup_request_opening_new_target_replaces_older_publication(overlay: bool) {
    let f = Fixture::new(overlay);
    let once = Cell::new(true);
    let weak = f.flyout.downgrade();
    let target = f.second.clone();
    let _ = f.flyout.opening(move |_| {
        if once.replace(false) {
            weak.upgrade().unwrap().show_at(&target);
        }
    });
    let opened = tracker();
    let observed = opened.clone();
    let _ = f.flyout.opened(move || observed.set(observed.get() + 1));
    f.flyout.show_at(&f.first);
    f.assert_owner(&f.second);
    assert_eq!(
        opened.get(),
        1,
        "only the accepted opening publishes Opened"
    );
    f.panel.children().remove(f.first.clone());
    f.assert_owner(&f.second);
}

fn popup_request_opened_new_target_owns_detach_handlers(overlay: bool) {
    let f = Fixture::new(overlay);
    let once = Cell::new(true);
    let weak = f.flyout.downgrade();
    let target = f.second.clone();
    let _ = f.flyout.opened(move || {
        if once.replace(false) {
            weak.upgrade().unwrap().show_at(&target);
        }
    });
    f.flyout.show_at(&f.first);
    f.assert_owner(&f.second);
    f.panel.children().remove(f.first.clone());
    f.assert_owner(&f.second);
    f.panel.children().remove(f.second.clone());
    assert!(!f.flyout.is_open());
    assert!(f.flyout.target().is_none());
}

fn popup_request_closed_callback_owns_reopened_target(overlay: bool) {
    let f = Fixture::new(overlay);
    f.flyout.show_at(&f.first);
    let once = Cell::new(true);
    let weak = f.flyout.downgrade();
    let target = f.second.clone();
    let _ = f.flyout.closed(move || {
        if once.replace(false) {
            weak.upgrade().unwrap().show_at(&target);
        }
    });
    f.flyout.hide();
    f.assert_owner(&f.second);
    f.panel.children().remove(f.first.clone());
    f.assert_owner(&f.second);
}

fn popup_request_opening_hide_retires_pending_open(overlay: bool) {
    let f = Fixture::new(overlay);
    let weak = f.flyout.downgrade();
    let _ = f.flyout.opening(move |_| weak.upgrade().unwrap().hide());
    let opened = tracker();
    let observed = opened.clone();
    let _ = f.flyout.opened(move || observed.set(observed.get() + 1));
    f.flyout.show_at(&f.first);
    assert!(!f.flyout.is_open());
    assert!(!f.flyout.popup().is_open());
    assert!(f.flyout.target().is_none());
    assert!(f.flyout.popup().parent().is_none());
    assert_eq!(opened.get(), 0);
}

fn popup_request_opening_detach_reattach_retires_pending_open(overlay: bool) {
    let f = Fixture::new(overlay);
    let panel = f.panel.clone();
    let target = f.first.clone();
    let _ = f.flyout.opening(move |_| {
        panel.children().remove(target.clone());
        panel.children().add(target.clone());
    });
    f.flyout.show_at(&f.first);
    assert!(!f.flyout.is_open());
    assert!(!f.flyout.popup().is_open());
    assert!(f.flyout.target().is_none());
    assert!(f.flyout.popup().parent().is_none());
    f.flyout.set_is_open(true);
    assert!(
        !f.flyout.is_open(),
        "retired attachment cannot be a reopen target"
    );
}

fn popup_request_closing_nested_show_wins(overlay: bool) {
    let f = Fixture::new(overlay);
    f.flyout.show_at(&f.first);
    let once = Cell::new(true);
    let weak = f.flyout.downgrade();
    let target = f.second.clone();
    let _ = f.flyout.closing(move |_| {
        if once.replace(false) {
            weak.upgrade().unwrap().show_at(&target);
        }
    });
    f.flyout.hide();
    f.assert_owner(&f.second);
    f.panel.children().remove(f.first.clone());
    f.assert_owner(&f.second);
}

fn popup_request_canceled_hide_keeps_session_handlers(overlay: bool) {
    let f = Fixture::new(overlay);
    let _ = f.flyout.closing(|args| args.set_cancel(true));
    let once = Cell::new(true);
    let weak = f.flyout.downgrade();
    let _ = f.flyout.opened(move || {
        if once.replace(false) {
            weak.upgrade().unwrap().hide();
        }
    });
    f.flyout.show_at(&f.first);
    f.assert_owner(&f.first);
    f.panel.children().remove(f.first.clone());
    assert!(
        !f.flyout.is_open(),
        "detachment is still an unconditional close"
    );
    assert!(f.flyout.target().is_none());
}

fn popup_request_retarget_close_preserves_nested_closed_show(overlay: bool) {
    let f = Fixture::new(overlay);
    let third = Button::new();
    f.panel.children().add(third.clone());
    f.flyout.show_at(&f.first);
    let once = Cell::new(true);
    let weak = f.flyout.downgrade();
    let accepted = third.clone();
    let _ = f.flyout.closed(move || {
        if once.replace(false) {
            weak.upgrade().unwrap().show_at(&accepted);
        }
    });
    f.flyout.show_at(&f.second);
    f.assert_owner(&third);
}

fn popup_request_property_callback_new_target_preserves_is_open(overlay: bool) {
    let f = Fixture::new(overlay);
    let once = Cell::new(true);
    let weak = f.flyout.downgrade();
    let target = f.second.clone();
    let _ = f.flyout.property_changed(move |change| {
        if change.property().name() == "IsOpen"
            && change.get_new_value::<bool>()
            && once.replace(false)
        {
            weak.upgrade().unwrap().show_at(&target);
        }
    });
    f.flyout.show_at(&f.first);
    f.assert_owner(&f.second);
    f.panel.children().remove(f.first.clone());
    f.assert_owner(&f.second);
    f.flyout.set_is_open(false);
    assert!(!f.flyout.is_open());
    assert!(!f.flyout.popup().is_open());
}

fn popup_request_same_target_reposition_keeps_detach_subscription(overlay: bool) {
    let f = Fixture::new(overlay);
    f.flyout.show_at(&f.first);
    let anchor = Rect::new(7.0, 11.0, 0.0, 0.0);
    assert!(f.flyout.show_at_rect_core(&f.first, anchor));
    f.assert_owner(&f.first);
    assert_eq!(f.flyout.popup().placement_rect(), Some(anchor));
    f.panel.children().remove(f.first.clone());
    assert!(!f.flyout.is_open());
    assert!(f.flyout.target().is_none());
}

fn popup_request_canceled_outer_open_cannot_overwrite_nested_is_open(overlay: bool) {
    let f = Fixture::new(overlay);
    f.flyout.set_default_placement_target(Some(&f.first));
    let once = Cell::new(true);
    let weak = f.flyout.downgrade();
    let target = f.second.clone();
    let _ = f.flyout.opening(move |args| {
        if once.replace(false) {
            weak.upgrade().unwrap().show_at(&target);
            args.set_cancel(true);
        }
    });
    f.flyout.set_is_open(true);
    f.assert_owner(&f.second);
}

fn popup_request_open_property_handler_can_hide(overlay: bool) {
    let f = Fixture::new(overlay);
    let once = Cell::new(true);
    let weak = f.flyout.downgrade();
    let _ = f.flyout.property_changed(move |change| {
        if change.property().name() == "IsOpen"
            && change.get_new_value::<bool>()
            && once.replace(false)
        {
            weak.upgrade().unwrap().set_is_open(false);
        }
    });
    f.flyout.show_at(&f.first);
    assert!(!f.flyout.is_open());
    assert!(!f.flyout.popup().is_open());
    assert!(f.flyout.target().is_none());
}

fn popup_request_close_property_handler_can_reopen(overlay: bool) {
    let f = Fixture::new(overlay);
    f.flyout.show_at(&f.first);
    let once = Cell::new(true);
    let weak = f.flyout.downgrade();
    let target = f.second.clone();
    let _ = f.flyout.property_changed(move |change| {
        if change.property().name() == "IsOpen"
            && !change.get_new_value::<bool>()
            && once.replace(false)
        {
            weak.upgrade().unwrap().show_at(&target);
        }
    });
    f.flyout.hide();
    f.assert_owner(&f.second);
    f.panel.children().remove(f.first.clone());
    f.assert_owner(&f.second);
}

fn popup_request_popup_host_closed_callback_can_reopen(overlay: bool) {
    let f = Fixture::new(overlay);
    f.flyout.show_at(&f.first);
    let once = Cell::new(true);
    let weak = f.flyout.downgrade();
    let target = f.second.clone();
    let _ = f.flyout.popup().popup_host_changed(move |host| {
        if host.is_none() && once.replace(false) {
            weak.upgrade().unwrap().show_at(&target);
        }
    });
    f.flyout.hide();
    f.assert_owner(&f.second);
    assert!(f.flyout.popup().host().is_some());
    f.panel.children().remove(f.first.clone());
    f.assert_owner(&f.second);
}

fn popup_request_popup_host_opened_callback_can_retarget(overlay: bool) {
    let f = Fixture::new(overlay);
    let once = Cell::new(true);
    let weak = f.flyout.downgrade();
    let target = f.second.clone();
    let _ = f.flyout.popup().popup_host_changed(move |host| {
        if host.is_some() && once.replace(false) {
            weak.upgrade().unwrap().show_at(&target);
        }
    });
    f.flyout.show_at(&f.first);
    f.assert_owner(&f.second);
    f.panel.children().remove(f.first.clone());
    f.assert_owner(&f.second);
}

fn popup_request_redundant_popup_open_keeps_completion_events(overlay: bool) {
    let f = Fixture::new(overlay);
    let popup = f.flyout.popup();
    let weak = popup.downgrade();
    let _ = popup.opened(move || weak.upgrade().unwrap().open());
    let opened = tracker();
    let observed = opened.clone();
    let _ = popup.opened(move || observed.set(observed.get() + 1));
    let hosts = tracker();
    let observed = hosts.clone();
    let _ = popup.popup_host_changed(move |host| {
        if host.is_some() {
            observed.set(observed.get() + 1);
        }
    });
    f.flyout.show_at(&f.first);
    f.assert_owner(&f.first);
    assert_eq!(opened.get(), 1);
    assert_eq!(hosts.get(), 1);
}

fn popup_request_popup_open_during_closing_retires_close(overlay: bool) {
    let f = Fixture::new(overlay);
    f.flyout.show_at(&f.first);
    let popup = f.flyout.popup();
    let weak = popup.downgrade();
    let once = Cell::new(true);
    let _ = popup.closing(move |_| {
        if once.replace(false) {
            weak.upgrade().unwrap().open();
        }
    });
    popup.close();
    f.assert_owner(&f.first);
    assert!(popup.host().is_some());
}

fn popup_request_popup_open_during_flyout_hide_keeps_session_ownership(overlay: bool) {
    let f = Fixture::new(overlay);
    f.flyout.show_at(&f.first);
    let popup = f.flyout.popup();
    let weak = popup.downgrade();
    let once = Cell::new(true);
    let _ = popup.closing(move |_| {
        if once.replace(false) {
            weak.upgrade().unwrap().open();
        }
    });
    f.flyout.hide();
    f.assert_owner(&f.first);
    f.panel.children().remove(f.first.clone());
    assert!(!f.flyout.is_open());
    assert!(!popup.is_open());
    assert!(f.flyout.target().is_none());
}

fn popup_request_parent_callback_reopen_keeps_the_new_parent(overlay: bool) {
    let f = Fixture::new(overlay);
    f.flyout.show_at(&f.first);
    let popup = f.flyout.popup();
    let popup_weak = popup.downgrade();
    let weak = f.flyout.downgrade();
    let target = f.second.clone();
    let once = Cell::new(true);
    let _ = popup.property_changed(move |change| {
        if change.property().name() == "Parent"
            && popup_weak.upgrade().unwrap().parent().is_none()
            && once.replace(false)
        {
            weak.upgrade().unwrap().show_at(&target);
        }
    });
    f.flyout.hide();
    f.assert_owner(&f.second);
    assert_eq!(
        popup.parent(),
        Some(f.second.clone().upcast::<ferroui_base::StyledElement>())
    );
    f.panel.children().remove(f.first.clone());
    f.assert_owner(&f.second);
}

fn popup_request_parent_callback_closed_session_retires_old_assignment(overlay: bool) {
    let f = Fixture::new(overlay);
    let popup = f.flyout.popup();
    popup.set_popup_parent(Some(&f.first.clone().upcast()));
    assert!(popup.placement_target().is_none());
    let popup_weak = popup.downgrade();
    let weak = f.flyout.downgrade();
    let target = f.second.clone();
    let once = Rc::new(Cell::new(true));
    let observed = once.clone();
    let _ = popup.property_changed(move |change| {
        if change.property().name() == "Parent"
            && popup_weak.upgrade().unwrap().parent().is_none()
            && once.replace(false)
        {
            let flyout = weak.upgrade().unwrap();
            flyout.show_at(&target);
            flyout.hide();
        }
    });
    // Both the actual parent and placement target return to None during the
    // callback; only a distinct assignment ticket remembers the newer request.
    popup.set_popup_parent(Some(&f.panel.clone().upcast()));
    assert!(!observed.get(), "parent callback must run");
    assert!(!f.flyout.is_open());
    assert!(!popup.is_open());
    assert!(f.flyout.target().is_none());
    assert!(popup.placement_target().is_none());
    assert!(popup.parent().is_none());
    assert!(popup.templated_parent().is_none());
}

fn popup_request_same_parent_callback_completes_templated_parent(overlay: bool) {
    let f = Fixture::new(overlay);
    let template_owner = Control::new();
    f.first.set_templated_parent(template_owner.clone());
    let popup = f.flyout.popup();
    let weak = popup.downgrade();
    let target = f.first.clone().upcast::<Control>();
    let once = Rc::new(Cell::new(true));
    let observed = once.clone();
    let _ = popup.property_changed(move |change| {
        let popup = weak.upgrade().unwrap();
        if change.property().name() == "Parent"
            && popup.parent() == Some(target.clone().upcast())
            && once.replace(false)
        {
            popup.set_popup_parent(Some(&target));
        }
    });
    f.flyout.show_at(&f.first);
    assert!(!observed.get(), "same-parent callback must run");
    f.assert_owner(&f.first);
    assert_eq!(popup.templated_parent(), Some(template_owner.upcast()));
}

fn popup_request_superseded_control_request_does_not_open_ancestor_menu(overlay: bool) {
    let f = Fixture::new(overlay);
    let ancestor = crate::ContextMenu::new();
    ancestor.items().add(Some(Control::boxed(MenuItem::new())));
    f.panel.set_context_menu(ancestor.clone());
    f.first
        .set_context_flyout(f.flyout.clone().upcast::<FlyoutBase>());
    let once = Cell::new(true);
    let weak = f.flyout.downgrade();
    let target = f.second.clone();
    let _ = f.flyout.opening(move |_| {
        if once.replace(false) {
            weak.upgrade().unwrap().show_at(&target);
        }
    });
    let request = ContextRequestedEventArgs::new();
    f.first.raise_event(&request);
    assert!(request.handled());
    f.assert_owner(&f.second);
    assert!(!ancestor.is_open());
}

/// Arm a virtual transform read at a particular host-setup boundary. The
/// second pending read follows host child assignment when transforms inherit;
/// the first committed read occurs while target subscriptions are assembled.
struct HostStageTransform {
    popup: WeakRef<crate::primitives::Popup>,
    committed: bool,
    reads: Cell<usize>,
    threshold: usize,
    callback: RefCell<Option<Box<dyn FnOnce()>>>,
}

impl ITransform for HostStageTransform {
    fn value(&self) -> Matrix {
        if self
            .popup
            .upgrade()
            .is_some_and(|popup| popup.is_open() && popup.host().is_some() == self.committed)
        {
            let reads = self.reads.get() + 1;
            self.reads.set(reads);
            if reads == self.threshold {
                let callback = self.callback.borrow_mut().take();
                if let Some(callback) = callback {
                    callback();
                }
            }
        }
        Matrix::IDENTITY
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn popup_request_pending_host_cleanup_preserves_new_presenter(overlay: bool) {
    let f = Fixture::new(overlay);
    let popup = f.flyout.popup();
    popup.set_inherits_transform(true);
    let called = tracker();
    let observed = called.clone();
    let weak = f.flyout.downgrade();
    let target = f.second.clone();
    f.first
        .set_render_transform(Some(Rc::new(HostStageTransform {
            popup: popup.downgrade(),
            committed: false,
            reads: Cell::new(0),
            threshold: 2,
            callback: RefCell::new(Some(Box::new(move || {
                observed.set(observed.get() + 1);
                weak.upgrade().unwrap().show_at(&target);
            }))),
        })));
    f.flyout.show_at(&f.first);
    assert_eq!(called.get(), 1, "the host setup callback was exercised");
    f.assert_owner(&f.second);
    let host = popup.host().unwrap();
    let child = popup.child().unwrap();
    assert!(host.as_control().is_visual_ancestor_of(&child));
    if let Some(presenter) = host.presenter() {
        assert_eq!(
            child.inheritance_parent(),
            Some(presenter.upcast::<ferroui_base::FerroObject>())
        );
    }
    f.panel.children().remove(f.first.clone());
    f.assert_owner(&f.second);
    assert!(host.as_control().is_visual_ancestor_of(&child));
}

fn popup_request_transform_subscription_callback_keeps_current_target_handlers(overlay: bool) {
    let f = Fixture::new(overlay);
    let popup = f.flyout.popup();
    let called = tracker();
    let observed = called.clone();
    let weak = f.flyout.downgrade();
    let target = f.second.clone();
    f.first
        .set_render_transform(Some(Rc::new(HostStageTransform {
            popup: popup.downgrade(),
            committed: true,
            reads: Cell::new(0),
            threshold: 1,
            callback: RefCell::new(Some(Box::new(move || {
                observed.set(observed.get() + 1);
                weak.upgrade().unwrap().show_at(&target);
            }))),
        })));
    let first_handlers = f.first.property_changed_subscriber_count();
    let second_handlers = f.second.property_changed_subscriber_count();
    f.flyout.show_at(&f.first);
    assert_eq!(
        called.get(),
        1,
        "the subscription-time transform read was exercised"
    );
    f.assert_owner(&f.second);
    f.flyout.hide();
    assert_eq!(f.first.property_changed_subscriber_count(), first_handlers);
    assert_eq!(
        f.second.property_changed_subscriber_count(),
        second_handlers
    );
}

#[repr(C)]
struct MeasureCallback {
    base: Control,
    callback: RefCell<Option<Box<dyn FnOnce()>>>,
}
ferroui_base::ferro_class!(MeasureCallback: Control);
ferroui_base::ferro_impl_classes!(MeasureCallback:
    FerroObjectImpl, StyledElementImpl, VisualImpl, InteractiveImpl,
    InputElementImpl, ControlImpl
);
impl LayoutableImpl for MeasureCallback {
    fn measure_override(this: &Self, _available: Size) -> Size {
        let callback = this.callback.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
        Size::new(30.0, 20.0)
    }
}

fn popup_request_measure_callback_preserves_new_anchor(overlay: bool) {
    let f = Fixture::new(overlay);
    let old_anchor = Rect::new(1.0, 2.0, 0.0, 0.0);
    let accepted_anchor = Rect::new(7.0, 11.0, 0.0, 0.0);
    let weak = f.flyout.downgrade();
    let target = f.second.clone();
    let child: Ref<MeasureCallback> = ferroui_base::instantiate(MeasureCallback {
        base: Control::construct(),
        callback: RefCell::new(Some(Box::new(move || {
            weak.upgrade()
                .unwrap()
                .show_at_rect_core(&target, accepted_anchor);
        }))),
    });
    f.flyout.set_content(Some(Control::boxed(child)));
    f.flyout.show_at_rect_core(&f.first, old_anchor);
    f.assert_owner(&f.second);
    assert_eq!(f.flyout.popup().placement_rect(), Some(accepted_anchor));
}

flyout_tests! {
    popup_request_opening_new_target_replaces_older_publication,
    popup_request_opened_new_target_owns_detach_handlers,
    popup_request_closed_callback_owns_reopened_target,
    popup_request_opening_hide_retires_pending_open,
    popup_request_opening_detach_reattach_retires_pending_open,
    popup_request_closing_nested_show_wins,
    popup_request_canceled_hide_keeps_session_handlers,
    popup_request_retarget_close_preserves_nested_closed_show,
    popup_request_property_callback_new_target_preserves_is_open,
    popup_request_same_target_reposition_keeps_detach_subscription,
    popup_request_canceled_outer_open_cannot_overwrite_nested_is_open,
    popup_request_open_property_handler_can_hide,
    popup_request_close_property_handler_can_reopen,
    popup_request_popup_host_closed_callback_can_reopen,
    popup_request_popup_host_opened_callback_can_retarget,
    popup_request_redundant_popup_open_keeps_completion_events,
    popup_request_popup_open_during_closing_retires_close,
    popup_request_popup_open_during_flyout_hide_keeps_session_ownership,
    popup_request_parent_callback_reopen_keeps_the_new_parent,
    popup_request_parent_callback_closed_session_retires_old_assignment,
    popup_request_same_parent_callback_completes_templated_parent,
    popup_request_superseded_control_request_does_not_open_ancestor_menu,
    popup_request_pending_host_cleanup_preserves_new_presenter,
    popup_request_transform_subscription_callback_keeps_current_target_handlers,
    popup_request_measure_callback_preserves_new_anchor,
}
