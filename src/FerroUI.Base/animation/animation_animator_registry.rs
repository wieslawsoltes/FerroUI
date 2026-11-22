use crate::animation::animators::*;
use crate::animation::{Animation, AnimatorFactory, IAnimationSetter, ICustomAnimator, InterpolatingAnimator};
use crate::media::Transform;
use crate::{FerroProperty, PropertyValue};
use std::any::TypeId;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

type Condition = Rc<dyn Fn(&'static FerroProperty) -> bool>;

#[derive(Clone)]
struct Registration {
    condition: Condition,
    type_: TypeId,
    factory: AnimatorFactory,
    custom: bool,
}

struct Registry {
    /// Animator selection, in order of precedence.
    animators: Vec<Registration>,
    /// The animators assigned to individual setters.
    setter_animators: Vec<(Weak<dyn IAnimationSetter>, TypeId, AnimatorFactory)>,
}

fn by_type<T: PropertyValue>() -> Condition {
    Rc::new(|property| property.property_type() == TypeId::of::<T>())
}

fn is_transform_component(property: &'static FerroProperty) -> bool {
    property.property_type() == TypeId::of::<f64>()
        && Transform::TYPE.is_assignable_from(property.owner_type())
}

fn registration<A: Animator + Default>(condition: Condition) -> Registration {
    Registration {
        condition,
        type_: TypeId::of::<A>(),
        factory: Rc::new(|| Rc::new(A::default())),
        custom: false,
    }
}

fn register_animator<A: Animator + Default>(animators: &mut Vec<Registration>) {
    animators.insert(0, registration::<A>(by_type::<A::Value>()));
}

impl Registry {
    fn new() -> Self {
        let mut animators: Vec<Registration> = vec![
            registration::<TransformAnimator>(Rc::new(is_transform_component)),
            registration::<BoolAnimator>(by_type::<bool>()),
            registration::<ByteAnimator>(by_type::<u8>()),
            registration::<Int16Animator>(by_type::<i16>()),
            registration::<Int32Animator>(by_type::<i32>()),
            registration::<Int64Animator>(by_type::<i64>()),
            registration::<UInt16Animator>(by_type::<u16>()),
            registration::<UInt32Animator>(by_type::<u32>()),
            registration::<UInt64Animator>(by_type::<u64>()),
            registration::<FloatAnimator>(by_type::<f32>()),
            registration::<DoubleAnimator>(by_type::<f64>()),
        ];

        register_animator::<EffectAnimator>(&mut animators);
        register_animator::<BoxShadowAnimator>(&mut animators);
        register_animator::<BoxShadowsAnimator>(&mut animators);
        register_animator::<BaseBrushAnimator>(&mut animators);
        register_animator::<CornerRadiusAnimator>(&mut animators);
        register_animator::<ColorAnimator>(&mut animators);
        register_animator::<VectorAnimator>(&mut animators);
        register_animator::<PointAnimator>(&mut animators);
        register_animator::<RectAnimator>(&mut animators);
        register_animator::<RelativePointAnimator>(&mut animators);
        register_animator::<RelativeScalarAnimator>(&mut animators);
        register_animator::<SizeAnimator>(&mut animators);
        register_animator::<ThicknessAnimator>(&mut animators);

        Self { animators, setter_animators: Vec::new() }
    }
}

crate::ferro_thread_local! {
    static REGISTRY: RefCell<Registry> = RefCell::new(Registry::new());
}

impl Animation {
    /// Sets the value of the `Animator` attached property for a setter: the
    /// custom animator that animates the setter's property instead of the
    /// one selected by the property's type.
    pub fn set_animator(setter: &Rc<dyn IAnimationSetter>, value: Rc<dyn ICustomAnimator>) {
        let type_ = value.wrapper_type();
        let factory: AnimatorFactory = Rc::new(move || value.clone().create_wrapper());
        REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();
            registry.setter_animators.retain(|(key, _, _)| key.upgrade().is_some_and(|key| !same_setter(&key, setter)));
            registry.setter_animators.push((Rc::downgrade(setter), type_, factory));
        });
    }

    /// The animator assigned to `setter` with [`set_animator`](Self::set_animator).
    pub(crate) fn get_animator(setter: &Rc<dyn IAnimationSetter>) -> Option<(TypeId, AnimatorFactory)> {
        REGISTRY.with(|registry| {
            registry
                .borrow()
                .setter_animators
                .iter()
                .find(|(key, _, _)| key.upgrade().is_some_and(|key| same_setter(&key, setter)))
                .map(|(_, type_, factory)| (*type_, factory.clone()))
        })
    }

    /// Registers a user-provided animator for properties of its value type.
    /// It takes precedence over the animators registered before it.
    pub fn register_custom_animator<TAnimator: InterpolatingAnimator + Default>() {
        let type_ = TypeId::of::<crate::animation::i_custom_animator::AnimatorWrapper<TAnimator::Value>>();
        let factory: AnimatorFactory = Rc::new(|| {
            let animator: Rc<dyn ICustomAnimator> = Rc::new(TAnimator::default());
            animator.create_wrapper()
        });
        REGISTRY.with(|registry| {
            registry.borrow_mut().animators.insert(0, Registration {
                condition: by_type::<TAnimator::Value>(), type_, factory, custom: true,
            });
        });
    }

    /// Whether a setter in this animation uses an explicit or registered application animator.
    pub fn uses_custom_animators(&self) -> bool {
        self.children().snapshot().iter().any(|keyframe| {
            keyframe.setters().snapshot().iter().any(|setter| {
                Self::get_animator(setter).is_some() || setter.property()
                    .and_then(registered_animator).is_some_and(|registration| registration.custom)
            })
        })
    }

    /// Properties whose registered animator targets a visual's render transform.
    pub fn transform_components(&self) -> Vec<&'static FerroProperty> {
        let mut properties = Vec::new();
        for frame in self.children().snapshot().iter() {
            for setter in frame.setters().snapshot().iter() {
                let Some(property) = setter.property().filter(|p| is_transform_component(p)) else {
                    continue;
                };
                let animator = Self::get_animator(setter)
                    .or_else(|| Self::get_animator_type(property));
                if animator.is_some_and(|(kind, _)| kind == TypeId::of::<TransformAnimator>())
                    && !properties.contains(&property)
                {
                    properties.push(property);
                }
            }
        }
        properties
    }

    /// Selects the animator for a property: the first registration whose
    /// condition the property meets.
    pub(crate) fn get_animator_type(property: &'static FerroProperty) -> Option<(TypeId, AnimatorFactory)> {
        registered_animator(property).map(|entry| (entry.type_, entry.factory))
    }

    /// Whether a styled property has a native or application-registered animator.
    pub fn can_animate_property(property: &'static FerroProperty) -> bool {
        !property.is_direct() && registered_animator(property).is_some()
    }
}

fn registered_animator(property: &'static FerroProperty) -> Option<Registration> {
    // Evaluate conditions outside the registry borrow.
    let animators = REGISTRY.with(|registry| registry.borrow().animators.clone());
    animators.into_iter().find(|entry| (entry.condition)(property))
}

fn same_setter(a: &Rc<dyn IAnimationSetter>, b: &Rc<dyn IAnimationSetter>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}
