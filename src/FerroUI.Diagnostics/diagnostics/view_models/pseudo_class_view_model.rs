//! Port of `Diagnostics/ViewModels/PseudoClassViewModel.cs`.

use super::ViewModelBase;
use ferroui_base::controls::{Classes, IPseudoClasses};
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::{ferro_markup_type, Ref, StyledElement};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A pseudo-class a control declares, with whether it is set on the control. Setting
/// [`is_active`](Self::is_active) sets or clears the pseudo-class on the control.
pub struct PseudoClassViewModel {
    base: ViewModelBase,
    pseudo_classes: Classes,
    source: Ref<StyledElement>,
    name: String,
    is_active: RefCell<bool>,
    is_updating: Cell<bool>,
}

impl PartialEq for PseudoClassViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for PseudoClassViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl PseudoClassViewModel {
    pub fn new(name: &str, source: &StyledElement) -> Rc<Self> {
        let this = Rc::new(Self {
            base: ViewModelBase::new(),
            // `_source.Classes`, as the pseudo-classes of the control.
            pseudo_classes: source.classes(),
            source: source.to_ref(),
            name: name.to_string(),
            is_active: RefCell::new(false),
            is_updating: Cell::new(false),
        });

        this.update();

        this
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn is_active(&self) -> bool {
        *self.is_active.borrow()
    }

    pub fn set_is_active(&self, value: bool) {
        self.base.raise_and_set_if_changed(&self.is_active, value, "IsActive");

        if !self.is_updating.get() {
            let pseudo_classes: &dyn IPseudoClasses = &self.pseudo_classes;
            pseudo_classes.set(&self.name, value);
        }
    }

    pub fn update(&self) {
        // `try { .. } finally { _isUpdating = false; }`: nothing in between can fail.
        self.is_updating.set(true);

        self.set_is_active(self.source.classes().contains(&self.name));

        self.is_updating.set(false);
    }
}

ferro_markup_type!(class PseudoClassViewModel {
    this: Rc<PseudoClassViewModel>,
    handles: [PseudoClassViewModel, Rc<PseudoClassViewModel>, Option<Rc<PseudoClassViewModel>>],
    properties: [
        Name: String { get: PseudoClassViewModel::name },
        IsActive: bool { get: PseudoClassViewModel::is_active, set: PseudoClassViewModel::set_is_active },
    ],
    methods: [
        fn Update() => PseudoClassViewModel::update,
    ],
    notify_property_changed: PseudoClassViewModel,
});
