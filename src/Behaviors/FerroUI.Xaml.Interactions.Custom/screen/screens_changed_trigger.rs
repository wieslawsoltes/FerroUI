//! Port of `Screen/ScreensChangedTrigger.cs`.

use crate::{AttachedToVisualTreeTriggerBase, AttachedToVisualTreeTriggerBaseImpl, ScreensRef};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::EventArgs;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, FerroPropertyChangedEventArgs, Ref, StaticType, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_controls::TopLevel;
use ferroui_xaml_interactivity::{
    DisposableAction, DisposingTriggerImpl, Interaction, StyledElementBehaviorImpl,
};
use std::cell::RefCell;
use std::rc::Rc;

/// A trigger that executes its actions when the screen configuration changes.
///
/// The actions run in a job of the dispatcher, with [`EventArgs`] as the parameter.
#[repr(C)]
pub struct ScreensChangedTrigger {
    base: AttachedToVisualTreeTriggerBase,
    // The screens the trigger listens to, with the subscription to their change (the reference
    // removes its delegate).
    subscribed: RefCell<Option<(ScreensRef, Rc<dyn IDisposable>)>>,
}

ferro_class!(ScreensChangedTrigger: AttachedToVisualTreeTriggerBase);
ferro_class_info!(ScreensChangedTrigger {
    new: ScreensChangedTrigger::new,
    markup: {
        property_attributes: [
            Screens: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(
    ScreensChangedTrigger: StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingTriggerImpl,
);

ferro_properties! {
    impl ScreensChangedTrigger {
        /// Identifies the `Screens` property.
        pub fn screens_property() -> StyledProperty<Option<ScreensRef>> {
            FerroProperty::register::<ScreensChangedTrigger, _>("Screens", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeTriggerBaseImpl for ScreensChangedTrigger {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            this.subscribe(this.get_screens());

            let trigger = this.to_ref().downgrade();
            DisposableAction::create(move || {
                if let Some(trigger) = trigger.upgrade() {
                    trigger.subscribe(None);
                }
            })
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for ScreensChangedTrigger {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            if change.property() == Self::screens_property().as_property() && this.associated_object().is_some() {
                this.subscribe(change.get_new_value::<Option<ScreensRef>>());
            }
        }
    }
}

impl ScreensChangedTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: AttachedToVisualTreeTriggerBase::construct(<Visual as StaticType>::TYPE),
            subscribed: RefCell::new(None),
        }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the screens that are observed. This is a property of the object model. If
    /// not set, the screens of the top level of the associated object are used.
    pub fn screens(&self) -> Option<ScreensRef> {
        self.get_value(Self::screens_property())
    }

    pub fn set_screens(&self, value: Option<ScreensRef>) {
        self.set_value(Self::screens_property(), value)
    }

    fn subscribe(&self, screens: Option<ScreensRef>) {
        let subscribed = self.subscribed.borrow().as_ref().map(|(subscribed, _)| subscribed.clone());
        if subscribed == screens {
            return;
        }

        let previous = self.subscribed.borrow_mut().take();
        if let Some((_, subscription)) = previous {
            subscription.dispose();
        }

        if let Some(screens) = screens {
            let trigger = self.to_ref().downgrade();
            let subscription = screens.changed(move || {
                if let Some(trigger) = trigger.upgrade() {
                    trigger.on_screens_changed();
                }
            });
            *self.subscribed.borrow_mut() = Some((screens, subscription));
        }
    }

    fn get_screens(&self) -> Option<ScreensRef> {
        self.screens().or_else(|| {
            self.associated_object_as::<Visual>()
                .and_then(|visual| TopLevel::get_top_level(Some(&*visual)))
                .and_then(|top_level| top_level.screens())
                .map(ScreensRef::new)
        })
    }

    fn on_screens_changed(&self) {
        if !self.is_enabled() {
            return;
        }

        let trigger: Ref<ScreensChangedTrigger> = self.to_ref();
        Dispatcher::ui_thread().post_local(
            move || {
                let sender = trigger.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
                let e: BoxedValue = Rc::new(EventArgs::EMPTY);
                Interaction::execute_actions(sender.as_ref(), trigger.actions().as_ref(), Some(&e));
            },
            DispatcherPriority::DEFAULT,
        );
    }
}
