//! The view model of one row of the DynamicData page: a feature of the source cache, transformed
//! for the list.
//!
//! Addition of the port with no upstream counterpart (docs/porting/reactive.md, section 7.3): the
//! reference example has no DynamicData page.

use crate::models::ShowcaseFeature;
use ferroui_base::ferro_markup_type;
use ferroui_base::input::ICommand as FerroCommand;
use ferroui_reactiveui::{reactive_object, ReactiveCommand, ReactiveObject};
use ferroui_reactiveui_controls::ReactiveCommandAdapter;
use ferroui_system::IDisposable;
use std::rc::Rc;

/// The view model of one row of the DynamicData page: the feature it shows, the command that
/// toggles its favourite flag, and the command that removes it from the source cache through the
/// page's `RemoveFeature`.
pub struct FeatureItemViewModel {
    reactive: ReactiveObject,
    /// The feature of the source cache.
    feature: Rc<ShowcaseFeature>,
    /// Toggles the favourite flag of the feature (`ToggleFavourite`).
    toggle_favourite: ReactiveCommand<(), ()>,
    /// Removes the feature from the source cache (`Remove`).
    remove: ReactiveCommand<(), ()>,
    /// The two commands as commands of the controls of FerroUI, for the bindings of markup.
    toggle_favourite_command: Rc<dyn FerroCommand>,
    remove_command: Rc<dyn FerroCommand>,
}
reactive_object!(FeatureItemViewModel);

impl PartialEq for FeatureItemViewModel {
    /// The same row (reference equality).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl FeatureItemViewModel {
    /// The row of `feature`, removed from the source cache by `remove_feature` with the key of the
    /// feature.
    pub fn new(feature: Rc<ShowcaseFeature>, remove_feature: ReactiveCommand<i32, ()>) -> Rc<Self> {
        let toggled = feature.clone();
        let toggle_favourite = ReactiveCommand::create(move || toggled.set_is_favourite(!toggled.is_favourite()));
        let key = feature.id();
        let remove = ReactiveCommand::create_from_observable(move || remove_feature.execute(key));
        Rc::new(Self {
            reactive: ReactiveObject::new(),
            toggle_favourite_command: ReactiveCommandAdapter::command_of(Rc::new(toggle_favourite.clone())),
            remove_command: ReactiveCommandAdapter::command_of(Rc::new(remove.clone())),
            feature,
            toggle_favourite,
            remove,
        })
    }

    /// The feature of the row (`Feature`).
    pub fn feature(&self) -> Rc<ShowcaseFeature> {
        self.feature.clone()
    }

    /// The key of the feature (`Id`).
    pub fn id(&self) -> i32 {
        self.feature.id()
    }

    /// The name of the feature (`Name`).
    pub fn name(&self) -> String {
        self.feature.name()
    }

    /// The area of the feature (`Area`).
    pub fn area(&self) -> String {
        self.feature.area()
    }

    /// Whether the feature is a favourite (`IsFavourite`): the flag the page sorts by.
    pub fn is_favourite(&self) -> bool {
        self.feature.is_favourite()
    }

    /// The command that toggles the favourite flag (`ToggleFavourite`).
    pub fn toggle_favourite(&self) -> ReactiveCommand<(), ()> {
        self.toggle_favourite.clone()
    }

    /// The command that removes the feature (`Remove`).
    pub fn remove(&self) -> ReactiveCommand<(), ()> {
        self.remove.clone()
    }
}

impl IDisposable for FeatureItemViewModel {
    /// Disposes the commands of the row.
    fn dispose(&self) {
        self.toggle_favourite.dispose();
        self.remove.dispose();
    }
}

ferro_markup_type!(class FeatureItemViewModel {
    this: Rc<FeatureItemViewModel>,
    handles: [FeatureItemViewModel, Rc<FeatureItemViewModel>, Option<Rc<FeatureItemViewModel>>],
    properties: [
        Feature: Rc<ShowcaseFeature> { get: |this: &Rc<FeatureItemViewModel>| this.feature() },
        Name: String { get: |this: &Rc<FeatureItemViewModel>| this.name() },
        Area: String { get: |this: &Rc<FeatureItemViewModel>| this.area() },
        ToggleFavourite: Rc<dyn FerroCommand> { get: |this: &Rc<FeatureItemViewModel>| this.toggle_favourite_command.clone() },
        Remove: Rc<dyn FerroCommand> { get: |this: &Rc<FeatureItemViewModel>| this.remove_command.clone() },
    ],
    notify_property_changed: FeatureItemViewModel,
});
