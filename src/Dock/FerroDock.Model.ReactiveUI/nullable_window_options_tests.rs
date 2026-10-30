//! Source null-window guards and application of live options through real reactive setters.
use crate::{core::DockWindow, impl_factory, Factory, FactoryImpl};
use ferrodock_model::{core::*, FactoryBase, FactoryBaseImpl};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::{
    cell::{Cell, RefCell},
    panic::{catch_unwind, AssertUnwindSafe},
    rc::Rc,
};
fn failure(action: impl FnOnce()) -> String {
    let error = catch_unwind(AssertUnwindSafe(action)).expect_err("source argument rejection");
    error
        .downcast_ref::<String>()
        .cloned()
        .or_else(|| error.downcast_ref::<&str>().map(|s| (*s).into()))
        .unwrap()
}
struct OptionsFactory {
    component: Factory,
    delegate: Cell<bool>,
    calls: Cell<usize>,
    window: RefCell<Option<Rc<dyn IDockWindow>>>,
    options: RefCell<Option<Rc<DockWindowOptions>>>,
}
impl OptionsFactory {
    fn new() -> Rc<Self> {
        Rc::new_cyclic(|weak| Self {
            component: Factory::construct(weak.clone()),
            delegate: Cell::new(false),
            calls: Cell::new(0),
            window: Default::default(),
            options: Default::default(),
        })
    }
}
impl FactoryBaseImpl for OptionsFactory {
    fn factory_base(&self) -> &FactoryBase {
        self.component.base()
    }
    fn apply_window_options(
        &self,
        window: Option<Rc<dyn IDockWindow>>,
        options: Option<Rc<DockWindowOptions>>,
    ) {
        self.calls.set(self.calls.get() + 1);
        self.window.replace(window.clone());
        self.options.replace(options.clone());
        if self.delegate.get() {
            self.component.base().apply_window_options(window, options);
        }
    }
}
impl FactoryImpl for OptionsFactory {
    fn factory(&self) -> &Factory {
        &self.component
    }
}
impl_factory!(OptionsFactory);
#[test]
fn direct_apply_rejects_null_before_fields_change_and_factory_null_options_short_circuit() {
    let f = Factory::new();
    let options = DockWindowOptions::new();
    let parent = DockWindow::new();
    options.set_parent_window(Some(parent.clone()));
    options.set_is_modal(true);
    assert_eq!(
        failure(|| options.apply_to(None)),
        "Value cannot be null. (Parameter 'window')"
    );
    f.base().apply_window_options(None, None);
    FactoryBaseImpl::apply_window_options(f.as_ref(), None, None);
    assert_eq!(
        failure(|| f.base().apply_window_options(None, Some(options.clone()))),
        "Value cannot be null. (Parameter 'window')"
    );
    assert!(options.is_modal());
    assert!(Rc::ptr_eq(
        &options.parent_window().unwrap(),
        &(parent as Rc<dyn IDockWindow>)
    ));
}
#[test]
fn missing_options_preserve_every_existing_window_field_without_notifications() {
    let f = Factory::new();
    let window = DockWindow::new();
    let parent = DockWindow::new();
    window.set_owner_mode(DockWindowOwnerMode::ParentWindow);
    window.set_parent_window(Some(parent.clone()));
    window.set_is_modal(true);
    window.set_show_in_taskbar(Some(false));
    let changes = Rc::new(Cell::new(0));
    let count = changes.clone();
    window
        .property_changed()
        .add(Rc::new(move |_| count.set(count.get() + 1)));
    f.base().apply_window_options(Some(window.clone()), None);
    assert_eq!(changes.get(), 0);
    assert_eq!(window.owner_mode(), DockWindowOwnerMode::ParentWindow);
    assert!(window.is_modal());
    assert_eq!(window.show_in_taskbar(), Some(false));
    assert!(Rc::ptr_eq(
        &window.parent_window().unwrap(),
        &(parent as Rc<dyn IDockWindow>)
    ));
}
#[test]
fn actual_protected_override_accepts_null_and_delegation_preserves_original_options_and_window() {
    let f = OptionsFactory::new();
    let options = DockWindowOptions::new();
    options.set_is_modal(true);
    FactoryBaseImpl::apply_window_options(f.as_ref(), None, Some(options.clone()));
    assert_eq!(f.calls.get(), 1);
    assert!(f.window.borrow().is_none());
    assert!(Rc::ptr_eq(f.options.borrow().as_ref().unwrap(), &options));
    f.delegate.set(true);
    assert_eq!(
        failure(|| FactoryBaseImpl::apply_window_options(f.as_ref(), None, Some(options.clone()))),
        "Value cannot be null. (Parameter 'window')"
    );
    let window = DockWindow::new();
    FactoryBaseImpl::apply_window_options(f.as_ref(), Some(window.clone()), Some(options.clone()));
    assert_eq!(f.calls.get(), 3);
    assert!(Rc::ptr_eq(
        f.window.borrow().as_ref().unwrap(),
        &(window.clone() as Rc<dyn IDockWindow>)
    ));
    assert!(window.is_modal());
    assert!(Rc::ptr_eq(f.options.borrow().as_ref().unwrap(), &options));
}
#[test]
fn setter_callbacks_change_later_live_option_reads_in_original_order() {
    for remove_taskbar in [false, true] {
        let f = Factory::new();
        let window = DockWindow::new();
        let first = DockWindow::new();
        let second = DockWindow::new();
        let options = DockWindowOptions::new();
        options.set_owner_mode(DockWindowOwnerMode::DockableWindow);
        options.set_parent_window(Some(first.clone()));
        options.set_is_modal(true);
        options.set_show_in_taskbar(Some(false));
        window.set_show_in_taskbar(Some(true));
        let changes = Rc::new(RefCell::new(Vec::new()));
        let log = changes.clone();
        let live = options.clone();
        let replacement = second.clone();
        window.property_changed().add(Rc::new(move |name| {
            log.borrow_mut().push(name.to_owned());
            match name {
                "OwnerMode" => {
                    live.set_owner_mode(DockWindowOwnerMode::ParentWindow);
                    live.set_parent_window(Some(replacement.clone()));
                    live.set_is_modal(false);
                }
                "ParentWindow" => live.set_is_modal(true),
                "IsModal" => {
                    live.set_show_in_taskbar(if remove_taskbar { None } else { Some(false) })
                }
                _ => {}
            }
        }));
        f.base()
            .apply_window_options(Some(window.clone()), Some(options.clone()));
        let mut expected = vec!["OwnerMode", "ParentWindow", "IsModal"];
        if !remove_taskbar {
            expected.push("ShowInTaskbar");
        }
        assert_eq!(&*changes.borrow(), &expected);
        assert_eq!(window.owner_mode(), DockWindowOwnerMode::DockableWindow);
        assert_eq!(options.owner_mode(), DockWindowOwnerMode::ParentWindow);
        assert!(window.is_modal());
        assert!(Rc::ptr_eq(
            &window.parent_window().unwrap(),
            &(second as Rc<dyn IDockWindow>)
        ));
        assert_eq!(window.show_in_taskbar(), Some(remove_taskbar));
        assert!(Rc::ptr_eq(
            &options.parent_window().unwrap(),
            &window.parent_window().unwrap()
        ));
    }
}
