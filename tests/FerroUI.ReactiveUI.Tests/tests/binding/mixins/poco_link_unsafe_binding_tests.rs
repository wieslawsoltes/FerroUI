//! `PocoLinkUnsafeBindingTests` (`ReactiveUI.Binding.Tests/Mixins/PocoLinkUnsafeBindingTests.cs`).
//!
//! The view does not notify its own properties (it is `INotifyPropertyChanged`, as the bindings of
//! a view need, but never raises); the bound property is a property of an object it holds, which
//! notifies.

use crate::tests::binding::test_models::TestFixture;
use ferroui_reactiveui::*;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::events::Event;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const VIEW_MODEL_VALUE: &str = "model";

const EDITED_VALUE: &str = "edited";

/// A view model that counts the writes of its `Name` (`CountingViewModel`).
pub struct CountingViewModel {
    name: RefCell<String>,
    name_writes: Cell<usize>,
    property_changed: Event<str>,
}

impl CountingViewModel {
    fn new() -> Rc<Self> {
        Rc::new(Self { name: RefCell::new(String::new()), name_writes: Cell::new(0), property_changed: Event::new() })
    }

    pub fn name(&self) -> String {
        self.name.borrow().clone()
    }

    pub fn set_name(&self, value: String) {
        self.name_writes.set(self.name_writes.get() + 1);
        *self.name.borrow_mut() = value;
        self.property_changed.raise("Name");
    }
}

impl INotifyPropertyChanged for CountingViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

/// A view that raises no notification of its own, holding an editor that does (`SilentView`).
pub struct SilentView {
    view_model: RefCell<Option<Rc<CountingViewModel>>>,
    editor: Rc<TestFixture>,
    property_changed: Event<str>,
}

impl SilentView {
    fn new() -> Rc<Self> {
        Rc::new(Self { view_model: RefCell::new(None), editor: TestFixture::new(), property_changed: Event::new() })
    }

    pub fn view_model(&self) -> Option<Rc<CountingViewModel>> {
        self.view_model.borrow().clone()
    }

    pub fn set_view_model(&self, view_model: Option<Rc<CountingViewModel>>) {
        *self.view_model.borrow_mut() = view_model;
    }

    pub fn editor(&self) -> Rc<TestFixture> {
        self.editor.clone()
    }
}

impl INotifyPropertyChanged for SilentView {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

view_for!(SilentView, CountingViewModel);

#[test]
fn bind_unsafe_view_without_notification_does_not_write_back_on_create() {
    let view_model = CountingViewModel::new();
    view_model.set_name(VIEW_MODEL_VALUE.to_string());
    let view = SilentView::new();
    view.set_view_model(Some(view_model.clone()));
    view_model.name_writes.set(0);

    let binding = view
        .bind(view_model.clone(), settable_prop!(CountingViewModel => name), settable_prop!(SilentView => editor.is_not_null_string))
        .expect("no hook refuses the binding");
    let writes_on_create = view_model.name_writes.get();
    let view_on_create = view.editor().is_not_null_string();
    view.editor().set_is_not_null_string(EDITED_VALUE.to_string());

    assert_eq!(writes_on_create, 0);
    assert_eq!(view_on_create, VIEW_MODEL_VALUE);
    assert_eq!(view_model.name(), EDITED_VALUE);
    binding.dispose();
}
