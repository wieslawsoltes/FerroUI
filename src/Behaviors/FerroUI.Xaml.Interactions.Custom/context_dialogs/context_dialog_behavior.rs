//! Port of `ContextDialogs/ContextDialogBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::{EventArgs, HandlerList};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref, StaticType, StyledElementImpl,
    StyledProperty,
};
use ferroui_controls::primitives::Popup;
use ferroui_controls::{Control, PlacementMode};
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, StyledElementBehaviorImpl};
use std::cell::RefCell;
use std::rc::Rc;

/// The handler of the plain events of the behavior: the sender (the behavior) and the args
/// (`EventHandler` of the reference).
type DialogEventHandler = dyn Fn(Option<&BoxedValue>, &EventArgs);

/// Behavior that manages a context dialog implemented using a [`Popup`].
///
/// Deviation (DEVIATIONS.md, Behaviors): the events of the behavior are Rust handler lists
/// ([`opened`](Self::opened) and [`closed`](Self::closed), which return the subscription),
/// declared to markup as plain events, so that the event triggers find them by name.
#[repr(C)]
pub struct ContextDialogBehavior {
    base: AttachedToVisualTreeBehavior,
    popup: RefCell<Option<Ref<Popup>>>,
    popup_closed: RefCell<Option<Rc<dyn IDisposable>>>,
    opened: HandlerList<DialogEventHandler>,
    closed: HandlerList<DialogEventHandler>,
}

ferro_class!(ContextDialogBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(ContextDialogBehavior {
    new: ContextDialogBehavior::new,
    markup: {
        content: DialogContent,
        events: [
            Opened(Option<BoxedValue>, EventArgs) => |this: &Ref<ContextDialogBehavior>, handler: MarkupDelegate| {
                this.opened(move |sender: Option<&BoxedValue>, e: &EventArgs| {
                    handler.invoke(&[sender.cloned(), into_markup_value(*e)]);
                })
            },
            Closed(Option<BoxedValue>, EventArgs) => |this: &Ref<ContextDialogBehavior>, handler: MarkupDelegate| {
                this.closed(move |sender: Option<&BoxedValue>, e: &EventArgs| {
                    handler.invoke(&[sender.cloned(), into_markup_value(*e)]);
                })
            },
        ],
    },
});
ferroui_base::ferro_impl_classes!(
    ContextDialogBehavior: StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl ContextDialogBehavior {
        /// Identifies the `DialogContent` property.
        pub fn dialog_content_property() -> StyledProperty<Option<Ref<Control>>> {
            FerroProperty::register::<ContextDialogBehavior, _>("DialogContent", None)
        }

        /// Identifies the `IsOpen` property.
        pub fn is_open_property() -> StyledProperty<bool> {
            FerroProperty::register::<ContextDialogBehavior, _>("IsOpen", false)
        }

        /// Identifies the `Placement` property.
        pub fn placement_property() -> StyledProperty<PlacementMode> {
            FerroProperty::register::<ContextDialogBehavior, _>("Placement", PlacementMode::Pointer)
        }

        /// Identifies the `IsLightDismissEnabled` property.
        pub fn is_light_dismiss_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<ContextDialogBehavior, _>("IsLightDismissEnabled", true)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for ContextDialogBehavior {
        fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
            Self::parent_on_property_changed(this, change);

            let Some(popup) = this.popup.borrow().clone() else {
                return;
            };

            if change.property() == Self::dialog_content_property().as_property() {
                popup.set_child(this.dialog_content());
            } else if change.property() == Self::is_open_property().as_property() {
                this.update_popup();
            } else if change.property() == Self::placement_property().as_property() {
                popup.set_placement(this.placement());
            }
        }
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for ContextDialogBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let popup = Popup::new();
            popup.set_placement(this.placement());
            popup.set_placement_target(this.associated_object_as::<Control>());
            popup.set_is_light_dismiss_enabled(this.is_light_dismiss_enabled());
            popup.set_child(this.dialog_content());
            *this.popup.borrow_mut() = Some(popup.clone());

            this.update_popup();

            let behavior = this.to_ref().downgrade();
            let popup_closed = popup.closed(move || {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.popup_on_closed();
                }
            });
            *this.popup_closed.borrow_mut() = Some(popup_closed);

            let behavior = this.to_ref().downgrade();
            DisposableAction::create(move || {
                let Some(behavior) = behavior.upgrade() else {
                    return;
                };

                let popup = behavior.popup.borrow().clone();
                if let Some(popup) = popup {
                    let popup_closed = behavior.popup_closed.borrow_mut().take();
                    if let Some(popup_closed) = popup_closed {
                        popup_closed.dispose();
                    }
                    popup.close();
                    behavior.raise(&behavior.closed);
                    *behavior.popup.borrow_mut() = None;
                }
            })
        }
    }
}

impl ContextDialogBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: AttachedToVisualTreeBehavior::construct(<Control as StaticType>::TYPE),
            popup: RefCell::new(None),
            popup_closed: RefCell::new(None),
            opened: HandlerList::new(),
            closed: HandlerList::new(),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the dialog content. This is a property of the object model, and the content
    /// of the behavior in markup.
    pub fn dialog_content(&self) -> Option<Ref<Control>> {
        self.get_value(Self::dialog_content_property())
    }

    pub fn set_dialog_content(&self, value: Option<Ref<Control>>) {
        self.set_value(Self::dialog_content_property(), value)
    }

    /// Gets or sets a value indicating whether the dialog is open. This is a property of the
    /// object model.
    pub fn is_open(&self) -> bool {
        self.get_value(Self::is_open_property())
    }

    pub fn set_is_open(&self, value: bool) {
        self.set_value(Self::is_open_property(), value)
    }

    /// Gets or sets the popup placement mode. This is a property of the object model.
    pub fn placement(&self) -> PlacementMode {
        self.get_value(Self::placement_property())
    }

    pub fn set_placement(&self, value: PlacementMode) {
        self.set_value(Self::placement_property(), value)
    }

    /// Gets or sets a value that determines how the dialog can be dismissed. This is a property of
    /// the object model.
    pub fn is_light_dismiss_enabled(&self) -> bool {
        self.get_value(Self::is_light_dismiss_enabled_property())
    }

    pub fn set_is_light_dismiss_enabled(&self, value: bool) {
        self.set_value(Self::is_light_dismiss_enabled_property(), value)
    }

    /// Occurs when the dialog is opened.
    pub fn opened(&self, handler: impl Fn(Option<&BoxedValue>, &EventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe_event(|behavior| &behavior.opened, Rc::new(handler))
    }

    /// Occurs when the dialog is closed.
    pub fn closed(&self, handler: impl Fn(Option<&BoxedValue>, &EventArgs) + 'static) -> Rc<dyn IDisposable> {
        self.subscribe_event(|behavior| &behavior.closed, Rc::new(handler))
    }

    fn subscribe_event(
        &self,
        list: fn(&ContextDialogBehavior) -> &HandlerList<DialogEventHandler>,
        handler: Rc<DialogEventHandler>,
    ) -> Rc<dyn IDisposable> {
        let token = list(self).add(handler);
        let behavior = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(behavior) = behavior.upgrade() {
                list(&behavior).remove(token);
            }
        })
    }

    fn raise(&self, list: &HandlerList<DialogEventHandler>) {
        let object: Ref<FerroObject> = self.to_ref().upcast();
        let sender: BoxedValue = Rc::new(object);
        for (_, handler) in list.snapshot().iter() {
            handler(Some(&sender), &EventArgs::EMPTY);
        }
    }

    fn popup_on_closed(&self) {
        if self.popup.borrow().is_some() {
            self.set_current_value(Self::is_open_property(), false);
            self.raise(&self.closed);
        }
    }

    fn update_popup(&self) {
        let Some(popup) = self.popup.borrow().clone() else {
            return;
        };

        if self.is_open() {
            popup.open();
            self.raise(&self.opened);
        } else {
            popup.close();
            self.raise(&self.closed);
        }
    }
}
