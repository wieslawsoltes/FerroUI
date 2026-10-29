//! Port of `AdaptiveBehavior.cs`.

use crate::AdaptiveClassSetter;
use ferroui_base::collections::FerroList;
use ferroui_base::controls::IPseudoClasses;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::reactive::{IDisposable, ObservableExt};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, DirectProperty, ElementRef, FerroObject,
    FerroObjectExtensions, FerroObjectImpl, FerroProperty, Nullable, Rect, Ref, StaticType, StyledElementImpl,
    StyledProperty, Visual,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{
    ComparisonConditionType, StyledElementBehaviorImpl, StyledElementBehaviorImplExt, StyledElementBehaviorOfT,
};
use std::cell::{OnceCell, RefCell};
use std::ops::Deref;
use std::rc::Rc;

/// The adaptive class setters of an [`AdaptiveBehavior`]: a list of [`AdaptiveClassSetter`].
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference types the collection as the generic list
/// of the framework of `AdaptiveClassSetter`; here it is a named list (as `ConditionCollection`
/// of the interactivity crate), so that markup can add the setters of a document to it.
///
/// It is a reference object: clones refer to the same collection and compare by identity.
#[derive(Clone, PartialEq)]
pub struct AdaptiveClassSetters(FerroList<Ref<AdaptiveClassSetter>>);

impl AdaptiveClassSetters {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self(FerroList::new())
    }

    /// The list of the collection.
    pub fn list(&self) -> &FerroList<Ref<AdaptiveClassSetter>> {
        &self.0
    }
}

impl Default for AdaptiveClassSetters {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for AdaptiveClassSetters {
    type Target = FerroList<Ref<AdaptiveClassSetter>>;

    fn deref(&self) -> &FerroList<Ref<AdaptiveClassSetter>> {
        &self.0
    }
}

impl std::fmt::Debug for AdaptiveClassSetters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AdaptiveClassSetters({})", self.0.count())
    }
}

/// Observes the [`Visual::bounds`] of the associated control, or of the
/// [`source_control`](Self::source_control), and sets or removes style classes when the
/// conditions of its [`AdaptiveClassSetter`]s are met.
#[repr(C)]
pub struct AdaptiveBehavior {
    base: StyledElementBehaviorOfT,
    disposable: RefCell<Option<Rc<dyn IDisposable>>>,
    setters: OnceCell<AdaptiveClassSetters>,
}

ferro_class!(AdaptiveBehavior: StyledElementBehaviorOfT);
ferro_class_info!(AdaptiveBehavior {
    new: AdaptiveBehavior::new,
    markup: {
        content: Setters,
        property_attributes: [
            SourceControl: [ResolveByName],
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(AdaptiveBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl AdaptiveBehavior {
        /// Identifies the `SourceControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the behavior does not own its source, so the
        /// value is an element reference.
        pub fn source_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<AdaptiveBehavior, _>("SourceControl", None)
        }

        /// Identifies the `TargetControl` property (an element reference, as `SourceControl`).
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<AdaptiveBehavior, _>("TargetControl", None)
        }

        /// Identifies the `Setters` property.
        pub fn setters_property() -> DirectProperty<AdaptiveBehavior, AdaptiveClassSetters> {
            FerroProperty::register_direct::<AdaptiveBehavior, _>(
                "Setters",
                |behavior| behavior.setters(),
                None,
                AdaptiveClassSetters::new(),
            )
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for AdaptiveBehavior {
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

impl AdaptiveBehavior {
    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            disposable: RefCell::new(None),
            setters: OnceCell::new(),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the source control whose [`Visual::bounds_property`] property is
    /// observed; if it is not set, the associated object is used. This is a property of the object
    /// model; markup resolves a name given for it to the named control.
    pub fn source_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::source_control_property()))
    }

    pub fn set_source_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::source_control_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets the target control that class name that should be added or removed when
    /// triggered, if not set the associated object is used or the
    /// [`target_control`](AdaptiveClassSetter::target_control) of the [`AdaptiveClassSetter`].
    /// This is a property of the object model; markup resolves a name given for it to the named
    /// control.
    pub fn target_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::target_control_property()))
    }

    pub fn set_target_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::target_control_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets adaptive class setters collection. This is a property of the object model, and the
    /// content of the behavior in markup.
    pub fn setters(&self) -> AdaptiveClassSetters {
        self.setters.get_or_init(AdaptiveClassSetters::new).clone()
    }

    fn start_observing(&self) {
        let source_control = match self.source_control() {
            Some(source_control) => Some(source_control),
            None => self.associated_object_as::<Control>(),
        };

        if let Some(source_control) = source_control {
            let disposable = self.observe_bounds(&source_control);
            *self.disposable.borrow_mut() = Some(disposable);
        }
    }

    fn stop_observing(&self) {
        let disposable = self.disposable.borrow_mut().take();
        if let Some(disposable) = disposable {
            disposable.dispose();
        }
    }

    fn observe_bounds(&self, source_control: &Ref<Control>) -> Rc<dyn IDisposable> {
        self.execute(Some(source_control), Some(&self.setters()), source_control.bounds());

        // The behavior and the source are held weakly: the subscription lives in the source.
        let behavior = self.to_ref().downgrade();
        let source = source_control.downgrade();
        let source_object: &FerroObject = source_control;
        FerroObjectExtensions::get_observable(source_object, Visual::bounds_property()).subscribe_fn(
            move |bounds: Rect| {
                if let (Some(behavior), Some(source)) = (behavior.upgrade(), source.upgrade()) {
                    behavior.execute(Some(&source), Some(&behavior.setters()), bounds);
                }
            },
        )
    }

    fn execute(&self, source_control: Option<&Ref<Control>>, setters: Option<&AdaptiveClassSetters>, bounds: Rect) {
        let (Some(_), Some(setters)) = (source_control, setters) else {
            return;
        };

        for setter in setters.snapshot().iter() {
            let is_min_or_max_width_set = setter.is_set(AdaptiveClassSetter::min_width_property().as_property())
                || setter.is_set(AdaptiveClassSetter::max_width_property().as_property());
            let width_condition_triggered = Self::get_result(setter.min_width_operator(), bounds.width, setter.min_width())
                && Self::get_result(setter.max_width_operator(), bounds.width, setter.max_width());

            let is_min_or_max_height_set = setter.is_set(AdaptiveClassSetter::min_height_property().as_property())
                || setter.is_set(AdaptiveClassSetter::max_height_property().as_property());
            let height_condition_triggered =
                Self::get_result(setter.min_height_operator(), bounds.height, setter.min_height())
                    && Self::get_result(setter.max_height_operator(), bounds.height, setter.max_height());

            let is_add_class_triggered = match (is_min_or_max_width_set, is_min_or_max_height_set) {
                (true, false) => width_condition_triggered,
                (false, true) => height_condition_triggered,
                (true, true) => width_condition_triggered && height_condition_triggered,
                (false, false) => false,
            };

            let target_control = match setter.target_control() {
                Some(target_control) => Some(target_control),
                None => match self.target_control() {
                    Some(target_control) => Some(target_control),
                    None => self.associated_object_as::<Control>(),
                },
            };

            let Some(target_control) = target_control else {
                panic!("Value cannot be null. (Parameter 'targetControl')");
            };

            let class_name = setter.class_name();
            let is_pseudo_class = setter.is_pseudo_class();

            if is_add_class_triggered {
                Self::add(&target_control, class_name.as_deref(), is_pseudo_class);
            } else {
                Self::remove(&target_control, class_name.as_deref(), is_pseudo_class);
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

    fn add(target_control: &Control, class_name: Option<&str>, is_pseudo_class: bool) {
        let Some(class_name) = class_name.filter(|class_name| !class_name.is_empty()) else {
            return;
        };
        let classes = target_control.classes();
        if classes.contains(class_name) {
            return;
        }

        if is_pseudo_class {
            classes.add_pseudo(class_name);
        } else {
            classes.add(class_name);
        }
    }

    fn remove(target_control: &Control, class_name: Option<&str>, is_pseudo_class: bool) {
        let Some(class_name) = class_name.filter(|class_name| !class_name.is_empty()) else {
            return;
        };
        let classes = target_control.classes();
        if !classes.contains(class_name) {
            return;
        }

        if is_pseudo_class {
            classes.remove_pseudo(class_name);
        } else {
            classes.remove(class_name);
        }
    }
}
