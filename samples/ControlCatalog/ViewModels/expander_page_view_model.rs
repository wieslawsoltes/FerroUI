//! Port of `ViewModels/ExpanderPageViewModel.cs`.

use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::{ferro_markup_type, BoxedValue, CornerRadius, FerroProperty};
use mini_mvvm::ViewModelBase;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The view model of the expander page.
pub struct ExpanderPageViewModel {
    base: ViewModelBase,
    corner_radius: RefCell<BoxedValue>,
    rounded: Cell<bool>,
}

impl PartialEq for ExpanderPageViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for ExpanderPageViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl ExpanderPageViewModel {
    pub fn new() -> Rc<ExpanderPageViewModel> {
        Rc::new(Self {
            base: ViewModelBase::new(),
            corner_radius: RefCell::new(FerroProperty::unset_value()),
            rounded: Cell::new(false),
        })
    }

    /// The corner radius of the expanders: a `CornerRadius`, or the unset
    /// value.
    pub fn corner_radius(&self) -> BoxedValue {
        self.corner_radius.borrow().clone()
    }

    fn set_corner_radius(&self, value: BoxedValue) {
        let equal = {
            let current = self.corner_radius.borrow();
            Rc::ptr_eq(&*current, &value) || (**current).any_value_eq(&*value)
        };
        if !equal {
            *self.corner_radius.borrow_mut() = value;
            self.base.raise_property_changed("CornerRadius");
        }
    }

    pub fn rounded(&self) -> bool {
        self.rounded.get()
    }

    pub fn set_rounded(&self, value: bool) {
        if self.base.raise_and_set_if_changed_cell(&self.rounded, value, "Rounded") {
            self.set_corner_radius(if self.rounded.get() {
                Rc::new(CornerRadius::uniform(25.0))
            } else {
                FerroProperty::unset_value()
            });
        }
    }
}

ferro_markup_type!(class ExpanderPageViewModel {
    this: Rc<ExpanderPageViewModel>,
    handles: [ExpanderPageViewModel, Rc<ExpanderPageViewModel>, Option<Rc<ExpanderPageViewModel>>],
    constructors: [() => ExpanderPageViewModel::new],
    properties: [
        CornerRadius: BoxedValue { get: |this: &Rc<ExpanderPageViewModel>| this.corner_radius() },
        Rounded: bool {
            get: |this: &Rc<ExpanderPageViewModel>| this.rounded(),
            set: |this: &Rc<ExpanderPageViewModel>, value: bool| this.set_rounded(value)
        },
    ],
    notify_property_changed: ExpanderPageViewModel,
});

#[cfg(test)]
mod tests {
    // Not ports: the upstream sample has no tests.
    use super::*;

    #[test]
    fn rounded_switches_the_corner_radius_between_unset_and_25() {
        let view_model = ExpanderPageViewModel::new();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let sink = seen.clone();
        view_model.property_changed().add(Rc::new(move |name: &str| sink.borrow_mut().push(name.to_string())));

        assert!(Rc::ptr_eq(&view_model.corner_radius(), &FerroProperty::unset_value()));
        view_model.set_rounded(true);
        assert_eq!(Some(&CornerRadius::uniform(25.0)), view_model.corner_radius().downcast_ref::<CornerRadius>());
        view_model.set_rounded(true);
        view_model.set_rounded(false);
        assert!(Rc::ptr_eq(&view_model.corner_radius(), &FerroProperty::unset_value()));
        assert_eq!(vec!["Rounded", "CornerRadius", "Rounded", "CornerRadius"], *seen.borrow());
    }
}
