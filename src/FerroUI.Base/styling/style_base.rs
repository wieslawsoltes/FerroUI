use super::activators::IStyleActivator;
use super::{
    empty_styles, DuplicateSetterError, IStyle, Setter, SetterBase, StyleChildren, StyleInstance, ThemeVariant,
};
use crate::metadata::{from_markup_value, IAddChild};
use crate::controls::{
    IResourceNode, IResourceProvider, ResourceDictionary, ResourceHostRef, ResourceKey, ResourceValue,
    ResourcesChangedEventArgs, WeakResourceHost,
};
use crate::property_store::{FrameType, ValueFrame};
use crate::reactive::{Disposable, IDisposable};
use crate::animation::IAnimation;
use crate::collections::FerroList;
use crate::utilities::HandlerList;
use crate::{ferro_class, BoxedValue, FerroObject, FerroObjectImpl, ObjectType, Ref, StyledElement, Upcast, WeakRef};
use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

/// Base class for [`Style`](super::Style), [`ControlTheme`](super::ControlTheme)
/// and [`ContainerQuery`](super::ContainerQuery).
#[repr(C)]
pub struct StyleBase {
    base: FerroObject,
    owner: RefCell<Option<WeakResourceHost>>,
    children: OnceCell<StyleChildren>,
    resources: RefCell<Option<Ref<ResourceDictionary>>>,
    setters: OnceCell<FerroList<Rc<dyn SetterBase>>>,
    animations: OnceCell<FerroList<Rc<dyn IAnimation>>>,
    shared_instance: RefCell<Option<Rc<StyleInstance>>>,
    parent: RefCell<Option<WeakRef<StyleBase>>>,
    owner_changed: HandlerList<dyn Fn()>,
}

ferro_class! {
    StyleBase: FerroObject, virtuals StyleBaseImpl: FerroObjectImpl {
        /// Sets the parent style. Called when the style is added to or
        /// removed from the children of another style; validates that the
        /// style can be nested in `parent`.
        fn set_parent(this, parent: Option<&Ref<StyleBase>>);
        /// A string describing the style: its selector, target type or
        /// query.
        fn to_display_string(this) -> String;
    }
}

crate::ferro_class_info!(StyleBase {
    interfaces: [Rc<dyn IStyle>, Rc<dyn IResourceProvider>, Rc<dyn IAddChild<BoxedValue>>],
});

/// Implements the child-adding contract (the untyped one: the child is any
/// object) for the style behind a class handle.
struct StyleBaseAddChild(Ref<StyleBase>);

impl IAddChild<BoxedValue> for StyleBaseAddChild {
    fn add_child(&self, child: BoxedValue) {
        self.0.add_child(&child)
    }

    fn reference_id(&self) -> usize {
        &*self.0 as *const StyleBase as usize
    }
}

impl From<Ref<StyleBase>> for Rc<dyn IAddChild<BoxedValue>> {
    fn from(value: Ref<StyleBase>) -> Self {
        Rc::new(StyleBaseAddChild(value))
    }
}

impl FerroObjectImpl for StyleBase {}

impl StyleBaseImpl for StyleBase {
    fn set_parent(this: &Self, parent: Option<&Ref<StyleBase>>) {
        *this.parent.borrow_mut() = parent.map(Ref::downgrade);
    }

    fn to_display_string(this: &Self) -> String {
        this.get_type().name().to_string()
    }
}

impl StyleBase {
    /// Creates the class data; see [`FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            owner: RefCell::new(None),
            children: OnceCell::new(),
            resources: RefCell::new(None),
            setters: OnceCell::new(),
            animations: OnceCell::new(),
            shared_instance: RefCell::new(None),
            parent: RefCell::new(None),
            owner_changed: HandlerList::new(),
        }
    }

    /// The children of the style.
    pub fn children(&self) -> StyleChildren {
        self.children.get_or_init(|| StyleChildren::new(self.to_ref().downgrade())).clone()
    }

    /// The owner of the style: the resource host whose styles collection the
    /// style, or an ancestor of it, was added to.
    pub fn owner(&self) -> Option<ResourceHostRef> {
        let owner = self.owner.borrow().clone();
        owner.and_then(|o| o.upgrade())
    }

    fn set_owner(&self, value: Option<&ResourceHostRef>) {
        let old = self.owner();
        if old.as_ref() != value {
            *self.owner.borrow_mut() = value.map(ResourceHostRef::downgrade);
            if !self.owner_changed.is_empty() {
                for (_, handler) in self.owner_changed.snapshot().iter() {
                    handler();
                }
            }
        }
    }

    /// Raised when the owner of the style changes.
    pub fn owner_changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        self.subscribe_owner_changed(Rc::new(handler))
    }

    fn subscribe_owner_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        let token = self.owner_changed.add(handler);
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.owner_changed.remove(token);
            }
        })
    }

    /// The parent style, if the style is a nested style.
    pub fn parent(&self) -> Option<Ref<StyleBase>> {
        let parent = self.parent.borrow();
        parent.as_ref().and_then(WeakRef::upgrade)
    }

    /// The dictionary of style resources.
    pub fn resources(&self) -> Ref<ResourceDictionary> {
        if let Some(resources) = &*self.resources.borrow() {
            return resources.clone();
        }
        let resources = ResourceDictionary::new();
        self.set_resources(resources.clone());
        resources
    }

    pub fn set_resources(&self, value: Ref<ResourceDictionary>) {
        let had_resources = self.resources.borrow().as_ref().is_some_and(|r| r.has_resources());

        *self.resources.borrow_mut() = Some(value.clone());

        if let Some(owner) = self.owner() {
            value.add_owner(&owner);

            if had_resources || value.has_resources() {
                owner.notify_hosted_resources_changed(ResourcesChangedEventArgs::create());
            }
        }
    }

    /// The style's setters. The list is returned by handle: clones refer
    /// to the same list.
    pub fn setters(&self) -> FerroList<Rc<dyn SetterBase>> {
        self.setters.get_or_init(FerroList::new).clone()
    }

    /// Adds a setter to the style.
    pub fn add_setter(&self, setter: Rc<dyn SetterBase>) {
        self.setters().add(setter);
    }

    /// Removes all setters from the style.
    pub fn clear_setters(&self) {
        if let Some(setters) = self.setters.get() {
            setters.clear();
        }
    }

    /// Adds a setter to the style.
    pub fn add(&self, setter: Rc<dyn SetterBase>) {
        self.add_setter(setter)
    }

    /// The style's animations. The list is returned by handle: clones
    /// refer to the same list.
    pub fn animations(&self) -> FerroList<Rc<dyn IAnimation>> {
        self.animations.get_or_init(FerroList::new).clone()
    }

    /// Adds an animation to the style: it runs on every control the style
    /// applies to, while the style is active.
    pub fn add_animation(&self, animation: impl Into<Rc<dyn IAnimation>>) {
        self.animations().add(animation.into());
    }

    /// Removes all animations from the style.
    pub fn clear_animations(&self) {
        if let Some(animations) = self.animations.get() {
            animations.clear();
        }
    }

    /// Adds an untyped child: a setter is added to the setters, a style to
    /// the child styles. Panics for anything else.
    pub fn add_child(&self, child: &BoxedValue) {
        if let Some(setter) = child.downcast_ref::<Rc<Setter>>() {
            let setter: Rc<dyn SetterBase> = setter.clone();
            self.setters().add(setter);
            return;
        }
        let untyped = Some(child.clone());
        if let Some(setter) = from_markup_value::<Rc<dyn SetterBase>>(&untyped) {
            self.setters().add(setter);
        } else if let Some(style) = from_markup_value::<Rc<dyn IStyle>>(&untyped) {
            self.children().add(style);
        } else {
            panic!("Cannot add {} to a style.", child.type_name());
        }
    }

    /// Adds a child style.
    pub fn add_style(&self, style: impl Into<Rc<dyn IStyle>>) {
        self.children().add(style)
    }

    #[inline]
    pub(crate) fn has_children(&self) -> bool {
        self.children.get().is_some_and(|c| !c.is_empty())
    }

    #[inline]
    pub(crate) fn has_setters_or_animations(&self) -> bool {
        self.setters.get().is_some_and(|s| !s.is_empty()) || self.animations.get().is_some_and(|a| !a.is_empty())
    }

    /// Tries to find a resource in the style's resources and then in its
    /// child styles.
    pub fn try_get_resource(&self, key: &ResourceKey, theme_variant: Option<&ThemeVariant>) -> Option<ResourceValue> {
        let resources = self.resources.borrow().clone();
        if let Some(resources) = resources {
            if let Some(result) = resources.try_get_resource(key, theme_variant) {
                return Some(result);
            }
        }

        if let Some(children) = self.children.get() {
            if !children.is_empty() {
                for child in children.snapshot().iter() {
                    if let Some(result) = child.try_get_resource(key, theme_variant) {
                        return Some(result);
                    }
                }
            }
        }

        None
    }

    /// As [`try_attach_instance`](Self::try_attach_instance); a duplicate
    /// setter panics with the message of the error.
    #[allow(dead_code)]
    pub(crate) fn attach(
        &self,
        target: &StyledElement,
        activator: Option<Rc<dyn IStyleActivator>>,
        type_: FrameType,
        can_share_instance: bool,
    ) -> Rc<dyn ValueFrame> {
        match self.try_attach_instance(target, activator, type_, can_share_instance) {
            Ok(frame) => frame,
            Err(error) => panic!("{error}"),
        }
    }

    /// Instances the style on `target` and adds the instance to its value
    /// store. A style with two setters for the same property is an error:
    /// nothing is added to the value store then.
    pub(crate) fn try_attach_instance(
        &self,
        target: &StyledElement,
        activator: Option<Rc<dyn IStyleActivator>>,
        type_: FrameType,
        mut can_share_instance: bool,
    ) -> Result<Rc<dyn ValueFrame>, DuplicateSetterError> {
        let shared = if can_share_instance { self.shared_instance.borrow().clone() } else { None };

        let instance = match shared {
            Some(instance) => instance,
            None => {
                crate::perf_count!(StyleInstancesCreated);
                can_share_instance &= activator.is_none();

                let instance = StyleInstance::new(self.to_ref().downgrade(), activator, type_);

                let setters = self.setters.get().map(FerroList::snapshot).unwrap_or_default();
                for setter in setters.iter() {
                    let setter_instance = setter.clone().instance(&instance, target);
                    can_share_instance &= setter_instance.is_setter;
                    instance.try_add(setter_instance)?;
                }

                let animations = self.animations.get().map(FerroList::snapshot).unwrap_or_default();
                if !animations.is_empty() {
                    instance.add_animations(&animations);
                }

                if can_share_instance {
                    instance.make_shared();
                    *self.shared_instance.borrow_mut() = Some(instance.clone());
                }

                instance
            }
        };

        crate::perf_count!(StyleInstancesAttached);
        target.values().add_frame(target, instance.clone());
        instance.apply_animations(target);
        Ok(instance)
    }

    /// Adds an owner to the style.
    pub fn add_owner(&self, owner: &ResourceHostRef) {
        if self.owner().is_some() {
            panic!("The Style already has a parent.");
        }

        self.set_owner(Some(owner));
        let resources = self.resources.borrow().clone();
        if let Some(resources) = resources {
            resources.add_owner(owner);
        }
    }

    /// Removes the owner of the style.
    pub fn remove_owner(&self, owner: &ResourceHostRef) {
        if self.owner().as_ref() == Some(owner) {
            self.set_owner(None);
            let resources = self.resources.borrow().clone();
            if let Some(resources) = resources {
                resources.remove_owner(owner);
            }
        }
    }
}

/// The style and resource provider interfaces of a style object.
struct StyleBaseHandle(Ref<StyleBase>);

impl IResourceNode for StyleBaseHandle {
    fn reference_id(&self) -> *const () {
        let object: &FerroObject = (*self.0).upcast();
        object as *const FerroObject as *const ()
    }

    fn has_resources(&self) -> bool {
        self.0.resources.borrow().as_ref().is_some_and(|r| r.count() > 0)
    }

    fn try_get_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        self.0.try_get_resource(key, theme)
    }
}

impl IResourceProvider for StyleBaseHandle {
    fn owner(&self) -> Option<ResourceHostRef> {
        self.0.owner()
    }

    fn owner_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.0.subscribe_owner_changed(handler)
    }

    fn add_owner(&self, owner: &ResourceHostRef) {
        self.0.add_owner(owner)
    }

    fn remove_owner(&self, owner: &ResourceHostRef) {
        self.0.remove_owner(owner)
    }

    fn as_object(&self) -> Option<&FerroObject> {
        Some((*self.0).upcast())
    }
}

impl IStyle for StyleBaseHandle {
    fn children(&self) -> Rc<Vec<Rc<dyn IStyle>>> {
        match self.0.children.get() {
            Some(children) => children.snapshot(),
            None => empty_styles(),
        }
    }

    fn as_object(&self) -> Option<&FerroObject> {
        Some((*self.0).upcast())
    }

    fn as_resource_provider(&self) -> Option<&dyn IResourceProvider> {
        Some(self)
    }
}

impl From<Ref<StyleBase>> for Rc<dyn IResourceProvider> {
    fn from(value: Ref<StyleBase>) -> Self {
        Rc::new(StyleBaseHandle(value))
    }
}

impl From<&Ref<StyleBase>> for Rc<dyn IResourceProvider> {
    fn from(value: &Ref<StyleBase>) -> Self {
        Rc::new(StyleBaseHandle(value.clone()))
    }
}

impl<T: ObjectType + Upcast<StyleBase>> From<Ref<T>> for Rc<dyn IStyle> {
    fn from(value: Ref<T>) -> Self {
        Rc::new(StyleBaseHandle(value.upcast()))
    }
}

impl<T: ObjectType + Upcast<StyleBase>> From<&Ref<T>> for Rc<dyn IStyle> {
    fn from(value: &Ref<T>) -> Self {
        Rc::new(StyleBaseHandle(value.clone().upcast()))
    }
}
