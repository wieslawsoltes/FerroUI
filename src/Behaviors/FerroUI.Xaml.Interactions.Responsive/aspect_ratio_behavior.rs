//! Port of `AspectRatioBehavior.cs`.

use crate::AspectRatioClassSetter;
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

/// The aspect ratio class setters of an [`AspectRatioBehavior`]: a list of
/// [`AspectRatioClassSetter`].
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference types the collection as the generic list
/// of the framework of `AspectRatioClassSetter`; here it is a named list (as `ConditionCollection`
/// of the interactivity crate), so that markup can add the setters of a document to it.
///
/// It is a reference object: clones refer to the same collection and compare by identity.
#[derive(Clone, PartialEq)]
pub struct AspectRatioClassSetters(FerroList<Ref<AspectRatioClassSetter>>);

impl AspectRatioClassSetters {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self(FerroList::new())
    }

    /// The list of the collection.
    pub fn list(&self) -> &FerroList<Ref<AspectRatioClassSetter>> {
        &self.0
    }
}

impl Default for AspectRatioClassSetters {
    fn default() -> Self {
        Self::new()
    }
}

impl Deref for AspectRatioClassSetters {
    type Target = FerroList<Ref<AspectRatioClassSetter>>;

    fn deref(&self) -> &FerroList<Ref<AspectRatioClassSetter>> {
        &self.0
    }
}

impl std::fmt::Debug for AspectRatioClassSetters {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AspectRatioClassSetters({})", self.0.count())
    }
}

/// Observes bounds changes of a control (or a specified source) and conditionally adds or removes
/// classes based on [`AspectRatioClassSetter`] rules.
#[repr(C)]
pub struct AspectRatioBehavior {
    base: StyledElementBehaviorOfT,
    disposable: RefCell<Option<Rc<dyn IDisposable>>>,
    setters: OnceCell<AspectRatioClassSetters>,
}

ferro_class!(AspectRatioBehavior: StyledElementBehaviorOfT);
ferro_class_info!(AspectRatioBehavior {
    new: AspectRatioBehavior::new,
    markup: {
        content: Setters,
        property_attributes: [
            SourceControl: [ResolveByName],
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(AspectRatioBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl AspectRatioBehavior {
        /// Identifies the `SourceControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the behavior does not own its source, so the
        /// value is an element reference.
        pub fn source_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<AspectRatioBehavior, _>("SourceControl", None)
        }

        /// Identifies the `TargetControl` property (an element reference, as `SourceControl`).
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<AspectRatioBehavior, _>("TargetControl", None)
        }

        /// Identifies the `Setters` property.
        pub fn setters_property() -> DirectProperty<AspectRatioBehavior, AspectRatioClassSetters> {
            FerroProperty::register_direct::<AspectRatioBehavior, _>(
                "Setters",
                |behavior| behavior.setters(),
                None,
                AspectRatioClassSetters::new(),
            )
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for AspectRatioBehavior {
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

impl AspectRatioBehavior {
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

    /// Gets or sets the control whose bounds are observed. If not set, the associated object is
    /// used. This is a property of the object model; markup resolves a name given for it to the
    /// named control.
    pub fn source_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::source_control_property()))
    }

    pub fn set_source_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::source_control_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets or sets the target control to add or remove classes from. If not set, the associated
    /// object or the target control of the setter is used. This is a property of the object
    /// model; markup resolves a name given for it to the named control.
    pub fn target_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::target_control_property()))
    }

    pub fn set_target_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::target_control_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Gets aspect ratio class setters collection. This is a property of the object model, and
    /// the content of the behavior in markup.
    pub fn setters(&self) -> AspectRatioClassSetters {
        self.setters.get_or_init(AspectRatioClassSetters::new).clone()
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

    fn execute(&self, source_control: Option<&Ref<Control>>, setters: Option<&AspectRatioClassSetters>, bounds: Rect) {
        let (Some(_), Some(setters)) = (source_control, setters) else {
            return;
        };

        let ratio = if bounds.height > 0.0 { bounds.width / bounds.height } else { f64::INFINITY };

        for setter in setters.snapshot().iter() {
            let ratio_condition_triggered = Self::get_result(setter.min_ratio_operator(), ratio, setter.min_ratio())
                && Self::get_result(setter.max_ratio_operator(), ratio, setter.max_ratio());

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

            if ratio_condition_triggered {
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
