//! A feature of the showcase, an item of the source cache of the DynamicData page.
//!
//! Addition of the port with no upstream counterpart (docs/porting/reactive.md, section 7.3): the
//! reference example has no DynamicData page.

use ferroui_base::ferro_markup_type;
use ferroui_reactiveui::{reactive_object, IReactiveObjectExt, ReactiveObject};
use std::cell::Cell;
use std::rc::Rc;

/// A feature of the showcase: its key, its name, the area it belongs to, and whether the user
/// marked it as a favourite. The favourite flag notifies its changes, which the DynamicData page
/// refreshes its items on (`auto_refresh`).
pub struct ShowcaseFeature {
    reactive: ReactiveObject,
    id: i32,
    name: String,
    area: String,
    is_favourite: Cell<bool>,
}
reactive_object!(ShowcaseFeature);

impl PartialEq for ShowcaseFeature {
    /// The same feature (reference equality, as for the objects of the reference).
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ShowcaseFeature {
    /// A feature that is not a favourite.
    pub fn new(id: i32, name: impl Into<String>, area: impl Into<String>) -> Rc<Self> {
        Rc::new(Self { reactive: ReactiveObject::new(), id, name: name.into(), area: area.into(), is_favourite: Cell::new(false) })
    }

    /// The features the page starts with: what the example shows, by area.
    pub fn defaults() -> Vec<Rc<Self>> {
        [
            ("RoutedViewHost shell navigation", "Routing"),
            ("ViewModelViewHost view contracts", "Routing"),
            ("Bind, OneWayBind and compiled bindings", "Binding"),
            ("Observation of control properties", "Binding"),
            ("ReactiveCommand can-execute and asynchronous execution", "Commands"),
            ("Interactions handled by the active view", "Commands"),
            ("ObservableAsPropertyHelper computed state", "State"),
            ("WhenActivated disposal of live subscriptions", "Activation"),
            ("AutoSuspendHelper session persistence", "State"),
            ("DynamicData SourceCache with SortAndBind", "Collections"),
        ]
        .into_iter()
        .zip(1..)
        .map(|((name, area), id)| Self::new(id, name, area))
        .collect()
    }

    /// The key of the feature in the source cache (`Id`).
    pub fn id(&self) -> i32 {
        self.id
    }

    /// The name of the feature (`Name`).
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// The area of the feature (`Area`).
    pub fn area(&self) -> String {
        self.area.clone()
    }

    /// Whether the feature is a favourite (`IsFavourite`).
    pub fn is_favourite(&self) -> bool {
        self.is_favourite.get()
    }

    /// Marks the feature as a favourite or not, raising the change of `IsFavourite`.
    pub fn set_is_favourite(&self, value: bool) {
        let _ = self.raise_and_set_if_changed(&self.is_favourite, value, "IsFavourite");
    }

    /// Whether the feature matches a search text: its name or its area contains the text, without
    /// regard to case. An empty or blank text matches every feature.
    pub fn matches(&self, search_text: &str) -> bool {
        let search_text = search_text.trim().to_lowercase();
        search_text.is_empty() || self.name.to_lowercase().contains(&search_text) || self.area.to_lowercase().contains(&search_text)
    }
}

ferro_markup_type!(class ShowcaseFeature {
    this: Rc<ShowcaseFeature>,
    handles: [ShowcaseFeature, Rc<ShowcaseFeature>, Option<Rc<ShowcaseFeature>>],
    properties: [
        Id: i32 { get: |this: &Rc<ShowcaseFeature>| this.id() },
        Name: String { get: |this: &Rc<ShowcaseFeature>| this.name() },
        Area: String { get: |this: &Rc<ShowcaseFeature>| this.area() },
        IsFavourite: bool {
            get: |this: &Rc<ShowcaseFeature>| this.is_favourite(),
            set: |this: &Rc<ShowcaseFeature>, value: bool| this.set_is_favourite(value)
        },
    ],
    notify_property_changed: ShowcaseFeature,
});
