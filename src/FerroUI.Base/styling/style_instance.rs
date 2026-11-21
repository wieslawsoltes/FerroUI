use super::activators::{IStyleActivator, IStyleActivatorSink};
use super::{ISetterInstance, IStyleInstance, SetterInstance, SetterInstanceKind, StyleBase};
use crate::animation::IAnimation;
use crate::data::BindingPriority;
use crate::property_store::{FrameType, ValueFrame, ValueFrameBase};
use crate::reactive::{IObservable, IObserver, LightweightSubject, Subscription};
use crate::utilities::OptionCell;
use crate::{Ref, StyledElement, WeakRef};
use std::any::Any;
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// A style has two setters for the same property: the invalid operation the
/// managed original raises when the style is instanced on a control.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateSetterError {
    /// The property, as it is displayed.
    pub property: String,
    /// The style (or control theme), as it is displayed.
    pub style: String,
}

impl std::fmt::Display for DuplicateSetterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Duplicate setter encountered for property '{}' in '{}'.", self.property, self.style)
    }
}

impl std::error::Error for DuplicateSetterError {}

/// Stores state for a style that has been instanced on a control.
///
/// A style instance is created when a style is applied to a control. It is
/// responsible for tracking the setter instances of the style and its
/// activator, and is the value frame that holds the style's values in the
/// control's value store. A style without an activator whose setters are all
/// plain values has a single instance, shared by every control it applies to.
pub struct StyleInstance {
    base: ValueFrameBase,
    this: Weak<StyleInstance>,
    activator: Option<Rc<dyn IStyleActivator>>,
    is_active: Cell<bool>,
    /// What few instances have, made for the first of them (desktop-performance.md, section 9.64).
    extras: OptionCell<Box<StyleInstanceExtras>>,
    source: WeakRef<StyleBase>,
}

/// The setter instances of a style instance that are no value entries, and the animations of its style with
/// what applying them made. A style of a theme has neither, and its instances are most of what a control is
/// styled with, so they are kept apart from the instance.
#[derive(Default)]
struct StyleInstanceExtras {
    setters: Vec<Rc<dyn ISetterInstance>>,
    animations: Option<Vec<Rc<dyn IAnimation>>>,
    animation_apply_disposables: Vec<Subscription>,
    animation_trigger: Option<LightweightSubject<bool>>,
}

impl StyleInstance {
    pub(crate) fn new(
        style: WeakRef<StyleBase>,
        activator: Option<Rc<dyn IStyleActivator>>,
        type_: FrameType,
    ) -> Rc<Self> {
        let priority = Self::get_priority(activator.is_some());
        Rc::new_cyclic(|this| Self {
            base: ValueFrameBase::new(priority, type_),
            this: this.clone(),
            activator,
            is_active: Cell::new(false),
            extras: OptionCell::new(),
            source: style,
        })
    }

    /// The priority of the values of the style instance.
    pub fn priority(&self) -> BindingPriority {
        self.base.priority()
    }

    /// Whether the instance was created by `style`.
    pub(crate) fn is_source(&self, style: &Ref<StyleBase>) -> bool {
        self.source.points_to(style)
    }

    pub(crate) fn source_display(&self) -> String {
        match self.source.upgrade() {
            Some(source) => source.to_display_string(),
            None => "Style".to_string(),
        }
    }

    /// Makes room for the instances of `setters` setters.
    pub(crate) fn reserve(&self, setters: usize) {
        self.base.reserve(setters);
    }

    /// Adds the instance of a setter. A second setter for a property the
    /// style instance already has a value for is an error (the invalid
    /// operation of the managed original); nothing is added then.
    pub(crate) fn try_add(&self, instance: SetterInstance) -> Result<(), DuplicateSetterError> {
        match instance.kind {
            SetterInstanceKind::Entry(value_entry) => {
                if self.base.contains(value_entry.property()) {
                    return Err(DuplicateSetterError {
                        property: value_entry.property().to_string(),
                        style: self.source_display(),
                    });
                }
                self.base.add(value_entry);
            }
            SetterInstanceKind::Other(instance) => self.update_extras(|extras| extras.setters.push(instance)),
        }
        Ok(())
    }

    /// As [`try_add`](Self::try_add); a duplicate setter panics with the
    /// message of the error.
    #[allow(dead_code)]
    pub(crate) fn add(&self, instance: SetterInstance) {
        if let Err(error) = self.try_add(instance) {
            panic!("{error}");
        }
    }

    pub(crate) fn add_animations(&self, animations: &[Rc<dyn IAnimation>]) {
        self.update_extras(|extras| {
            extras.animations.get_or_insert_with(Vec::new).extend(animations.iter().cloned());
        });
    }

    /// Applies the animations of the style to `control`: they run while the
    /// style instance is active.
    pub(crate) fn apply_animations(&self, control: &StyledElement) {
        let animations = self.extras.with(|extras| extras.and_then(|extras| extras.animations.clone()));
        if let Some(animations) = animations {
            let trigger = self
                .update_extras(|extras| extras.animation_trigger.get_or_insert_with(LightweightSubject::new).clone());
            let match_: Rc<dyn IObservable<bool>> = Rc::new(trigger.clone());
            for animation in &animations {
                let disposable = animation.apply(control, None, match_.clone(), None, false);
                self.update_extras(|extras| extras.animation_apply_disposables.push(Subscription::new(disposable)));
            }

            if self.activator.is_none() {
                trigger.on_next(true);
            }
        }
    }

    fn trigger_animations(&self, value: bool) {
        let trigger = self.extras.with(|extras| extras.and_then(|extras| extras.animation_trigger.clone()));
        if let Some(trigger) = trigger {
            trigger.on_next(value);
        }
    }

    pub(crate) fn make_shared(&self) {
        self.base.make_shared();
    }

    /// Changes the extras of the instance, which are made for the first change. Nothing of the instance is
    /// asked for inside `update`: the extras are out of their cell while it runs.
    fn update_extras<R>(&self, update: impl FnOnce(&mut StyleInstanceExtras) -> R) -> R {
        let mut extras = self.extras.take().unwrap_or_default();
        let result = update(&mut extras);
        self.extras.set(Some(extras));
        result
    }

    /// Whether the instance has its extras: setter instances that are no value entries, or animations.
    #[cfg(test)]
    pub(crate) fn has_extras(&self) -> bool {
        self.extras.is_some()
    }

    fn get_priority(has_activator: bool) -> BindingPriority {
        if has_activator {
            BindingPriority::StyleTrigger
        } else {
            BindingPriority::Style
        }
    }
}

impl IStyleInstance for StyleInstance {
    fn source(&self) -> Option<Ref<StyleBase>> {
        self.source.upgrade()
    }

    #[inline]
    fn has_activator(&self) -> bool {
        self.activator.is_some()
    }

    fn is_active(&self) -> bool {
        self.is_active.get()
    }
}

impl IStyleActivatorSink for StyleInstance {
    fn on_next(&self, value: bool) {
        if let Some(owner) = self.base.owner() {
            if let Some(this) = self.this.upgrade() {
                let frame: Rc<dyn ValueFrame> = this;
                owner.values().on_frame_activation_changed(&owner, &frame);
            }
        }
        self.trigger_animations(value);
    }
}

impl ValueFrame for StyleInstance {
    fn base(&self) -> &ValueFrameBase {
        &self.base
    }

    fn get_is_active(&self) -> (bool, bool) {
        let previous = self.is_active.get();

        let is_active = match &self.activator {
            Some(activator) => {
                if !activator.is_subscribed() {
                    let sink: Weak<dyn IStyleActivatorSink> = self.this.clone();
                    activator.subscribe(sink);
                    self.trigger_animations(activator.get_is_active());
                }
                activator.get_is_active()
            }
            None => true,
        };

        self.is_active.set(is_active);
        (is_active, is_active != previous)
    }

    fn dispose(&self) {
        self.base.unsubscribe_all();
        if let Some(activator) = &self.activator {
            activator.dispose();
        }
        if self.extras.is_some() {
            // Out of the instance before they are disposed of: what an animation stops may ask for it.
            let disposables = self.update_extras(|extras| std::mem::take(&mut extras.animation_apply_disposables));
            drop(disposables);
        }
    }

}
