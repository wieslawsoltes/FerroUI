//! Port of `SplitView/SplitViewStateBehavior.cs`.

use crate::SplitViewStateSetter;
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, DirectProperty, ElementRef, FerroObject,
    FerroObjectExtensions, FerroObjectImpl, FerroProperty, Nullable, Rect, Ref, StaticType, StyledElementImpl,
    StyledProperty, Visual,
};
use ferroui_controls::{Control, SplitView};
use ferroui_xaml_interactivity::{
    ComparisonConditionType, StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementBehaviorOfT,
};
use std::cell::{OnceCell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

/// The split view state setters of a [`SplitViewStateBehavior`]: a list of
/// [`SplitViewStateSetter`].
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference types the collection as the generic list
/// of the framework of `SplitViewStateSetter`; here it is a named list, so that markup can add the
/// setters of a document to it.
///
/// It is a reference object: clones refer to the same collection and compare by identity.
#[derive(Clone, PartialEq)]
pub struct SplitViewStateSetters(FerroList<Ref<SplitViewStateSetter>>);

impl SplitViewStateSetters {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self(FerroList::new())
    }

    /// The list of the collection.
    pub fn list(&self) -> &FerroList<Ref<SplitViewStateSetter>> {
        &self.0
    }
}

impl Default for SplitViewStateSetters {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for SplitViewStateSetters {
    type Target = FerroList<Ref<SplitViewStateSetter>>;

    fn deref(&self) -> &FerroList<Ref<SplitViewStateSetter>> {
        &self.0
    }
}

impl std::fmt::Debug for SplitViewStateSetters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SplitViewStateSetters({})", self.0.count())
    }
}

ferroui_base::ferro_markup_type!(class SplitViewStateSetters {
    handles: [SplitViewStateSetters, Option<SplitViewStateSetters>],
    constructors: [() => SplitViewStateSetters::new],
    properties: [
        Count: i32 { get: |list: &SplitViewStateSetters| list.count() as i32 },
    ],
    methods: [
        fn Add(Ref<SplitViewStateSetter>) =>
            |list: &SplitViewStateSetters, item: Ref<SplitViewStateSetter>| list.add(item)
    ],
});

/// Updates [`SplitView`] properties based on size conditions.
#[repr(C)]
pub struct SplitViewStateBehavior {
    base: StyledElementBehaviorOfT,
    disposable: RefCell<Option<Rc<dyn IDisposable>>>,
    setters: OnceCell<SplitViewStateSetters>,
}

ferro_class!(SplitViewStateBehavior: StyledElementBehaviorOfT);
ferro_class_info!(SplitViewStateBehavior {
    new: SplitViewStateBehavior::new,
    markup: {
        content: Setters,
        property_attributes: [
            SourceControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(SplitViewStateBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl SplitViewStateBehavior {
        /// Identifies the `SourceControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the behavior does not own its source, so the
        /// value is an element reference.
        pub fn source_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<SplitViewStateBehavior, _>("SourceControl", None)
        }

        /// Identifies the `Setters` property.
        pub fn setters_property() -> DirectProperty<SplitViewStateBehavior, SplitViewStateSetters> {
            FerroProperty::register_direct::<SplitViewStateBehavior, _>(
                "Setters",
                |behavior| behavior.setters(),
                None,
                SplitViewStateSetters::new(),
            )
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for SplitViewStateBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            Self::parent_on_attached_to_visual_tree(this);

            this.stop_observing();
            this.start_observing();
        }

        fn on_detached_from_visual_tree(this: &Self) {
            Self::parent_on_detached_from_visual_tree(this);

            this.stop_observing();
        }
    }
}

impl SplitViewStateBehavior {
    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<SplitView as StaticType>::TYPE),
            disposable: RefCell::new(None),
            setters: OnceCell::new(),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the control whose bounds are observed. If not set, the associated object is
    /// used. This is a property of the object model; markup resolves a name given for it to the
    /// named control.
    pub fn source_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::source_control_property()))
    }

    pub fn set_source_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::source_control_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets split view state setters collection. This is a property of the object model, and the
    /// content of the behavior in markup.
    pub fn setters(&self) -> SplitViewStateSetters {
        self.setters.get_or_init(SplitViewStateSetters::new).clone()
    }

    fn start_observing(&self) {
        let source = match self.source_control() {
            Some(source) => Some(source),
            None => self.associated_object_as::<Control>(),
        };

        if let Some(source) = source {
            let disposable = self.observe_bounds(&source);
            *self.disposable.borrow_mut() = Some(disposable);
        }
    }

    fn stop_observing(&self) {
        let disposable = self.disposable.borrow_mut().take();
        if let Some(disposable) = disposable {
            disposable.dispose();
        }
    }

    fn observe_bounds(&self, source: &Ref<Control>) -> Rc<dyn IDisposable> {
        self.execute(Some(&self.setters()), source.bounds());

        // The behavior is held weakly: the subscription lives in the source.
        let behavior = self.to_ref().downgrade();
        let source_object: &FerroObject = source;
        FerroObjectExtensions::get_observable(source_object, Visual::bounds_property()).subscribe_fn(
            move |bounds: Rect| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.execute(Some(&behavior.setters()), bounds);
                }
            },
        )
    }

    fn execute(&self, setters: Option<&SplitViewStateSetters>, bounds: Rect) {
        let Some(associated_object) = self.associated_object_as::<SplitView>() else {
            return;
        };
        let Some(setters) = setters else {
            return;
        };

        for setter in setters.snapshot().iter() {
            let width_set = setter.is_set(SplitViewStateSetter::min_width_property().as_property())
                || setter.is_set(SplitViewStateSetter::max_width_property().as_property());
            let width_match = Self::get_result(setter.min_width_operator(), bounds.width, setter.min_width())
                && Self::get_result(setter.max_width_operator(), bounds.width, setter.max_width());

            let height_set = setter.is_set(SplitViewStateSetter::min_height_property().as_property())
                || setter.is_set(SplitViewStateSetter::max_height_property().as_property());
            let height_match = Self::get_result(setter.min_height_operator(), bounds.height, setter.min_height())
                && Self::get_result(setter.max_height_operator(), bounds.height, setter.max_height());

            let is_triggered = match (width_set, height_set) {
                (true, false) => width_match,
                (false, true) => height_match,
                (true, true) => width_match && height_match,
                (false, false) => false,
            };

            if !is_triggered {
                continue;
            }

            let target = match setter.target_split_view() {
                Some(target) => target,
                None => associated_object.clone(),
            };

            if setter.is_set(SplitViewStateSetter::display_mode_property().as_property()) {
                target.set_display_mode(setter.display_mode());
            }

            if setter.is_set(SplitViewStateSetter::pane_placement_property().as_property()) {
                target.set_pane_placement(setter.pane_placement());
            }

            if setter.is_set(SplitViewStateSetter::is_pane_open_property().as_property()) {
                target.set_is_pane_open(setter.is_pane_open());
            }
        }
    }

    fn get_result(comparison_condition_type: ComparisonConditionType, property: f64, value: f64) -> bool {
        match comparison_condition_type {
            ComparisonConditionType::Equal => property == value,
            ComparisonConditionType::NotEqual => property != value,
            ComparisonConditionType::LessThan => property < value,
            ComparisonConditionType::LessThanOrEqual => property <= value,
            ComparisonConditionType::GreaterThan => property > value,
            ComparisonConditionType::GreaterThanOrEqual => property >= value,
        }
    }
}
