//! Port of `ScrollViewer/ViewportBehavior.cs`.

use crate::{AttachedToVisualTreeBehavior, AttachedToVisualTreeBehaviorImpl};
use ferroui_base::interactivity::Interactive;
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::{EventArgs, HandlerList};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObject, FerroObjectImpl,
    FerroPropertyChangedEventArgs, FerroProperty, Point, Rect, Ref, StaticType, StyledElementImpl, StyledProperty,
    Visual, WeakRef,
};
use ferroui_controls::{ScrollChangedEventArgs, ScrollViewer};
use ferroui_xaml_interactivity::{DisposableAction, DisposingBehaviorImpl, Interaction, StyledElementBehaviorImpl};
use std::cell::RefCell;
use std::rc::Rc;

/// The handler of the plain events of the behavior: the sender (the associated element) and the
/// args (`EventHandler` of the reference).
type ViewportEventHandler = dyn Fn(Option<&BoxedValue>, &EventArgs);

/// Listens for the associated element entering or exiting the parent [`ScrollViewer`] viewport.
///
/// Deviation (DEVIATIONS.md, Behaviors): the events of the behavior are Rust handler lists
/// ([`entered_viewport`](Self::entered_viewport) and the others, which return the subscription),
/// declared to markup as plain events.
#[repr(C)]
pub struct ViewportBehavior {
    base: AttachedToVisualTreeBehavior,
    entered_viewport: HandlerList<ViewportEventHandler>,
    exited_viewport: HandlerList<ViewportEventHandler>,
    entering_viewport: HandlerList<ViewportEventHandler>,
    exiting_viewport: HandlerList<ViewportEventHandler>,
    host_scroll_viewer: RefCell<Option<WeakRef<ScrollViewer>>>,
}

ferro_class!(ViewportBehavior: AttachedToVisualTreeBehavior);
ferro_class_info!(ViewportBehavior {
    new: ViewportBehavior::new,
    markup: {
        events: [
            EnteredViewport(Option<BoxedValue>, EventArgs) => |this: &Ref<ViewportBehavior>, handler: MarkupDelegate| {
                this.entered_viewport(move |sender: Option<&BoxedValue>, e: &EventArgs| {
                    handler.invoke(&[sender.cloned(), into_markup_value(*e)]);
                })
            },
            ExitedViewport(Option<BoxedValue>, EventArgs) => |this: &Ref<ViewportBehavior>, handler: MarkupDelegate| {
                this.exited_viewport(move |sender: Option<&BoxedValue>, e: &EventArgs| {
                    handler.invoke(&[sender.cloned(), into_markup_value(*e)]);
                })
            },
            EnteringViewport(Option<BoxedValue>, EventArgs) => |this: &Ref<ViewportBehavior>, handler: MarkupDelegate| {
                this.entering_viewport(move |sender: Option<&BoxedValue>, e: &EventArgs| {
                    handler.invoke(&[sender.cloned(), into_markup_value(*e)]);
                })
            },
            ExitingViewport(Option<BoxedValue>, EventArgs) => |this: &Ref<ViewportBehavior>, handler: MarkupDelegate| {
                this.exiting_viewport(move |sender: Option<&BoxedValue>, e: &EventArgs| {
                    handler.invoke(&[sender.cloned(), into_markup_value(*e)]);
                })
            },
        ],
    },
});
ferroui_base::ferro_impl_classes!(
    ViewportBehavior: FerroObjectImpl,
    StyledElementImpl,
    StyledElementBehaviorImpl,
    DisposingBehaviorImpl,
);

ferro_properties! {
    impl ViewportBehavior {
        /// Identifies the `IsFullyInViewport` property.
        pub fn is_fully_in_viewport_property() -> StyledProperty<bool> {
            FerroProperty::register::<ViewportBehavior, _>("IsFullyInViewport", false)
        }

        /// Identifies the `IsInViewport` property.
        pub fn is_in_viewport_property() -> StyledProperty<bool> {
            FerroProperty::register::<ViewportBehavior, _>("IsInViewport", false)
        }

        /// Identifies the `IsAlwaysOn` property.
        pub fn is_always_on_property() -> StyledProperty<bool> {
            FerroProperty::register::<ViewportBehavior, _>("IsAlwaysOn", false)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl AttachedToVisualTreeBehaviorImpl for ViewportBehavior {
        fn on_attached_to_visual_tree_override(this: &Self) -> Rc<dyn IDisposable> {
            let host_scroll_viewer = this
                .associated_object_as::<Visual>()
                .and_then(|associated_object| associated_object.find_ancestor_of_type::<ScrollViewer>(false));
            let Some(host_scroll_viewer) = host_scroll_viewer else {
                panic!("This behavior can only be attached to an element which has a ScrollViewer as a parent.");
            };

            *this.host_scroll_viewer.borrow_mut() = Some(host_scroll_viewer.downgrade());

            let behavior = this.to_ref().downgrade();
            let token = host_scroll_viewer.scroll_changed(move |_sender: &Interactive, e: &ScrollChangedEventArgs| {
                if let Some(behavior) = behavior.upgrade() {
                    behavior.on_scroll_changed(e);
                }
            });
            this.evaluate_viewport_state();

            let behavior = this.to_ref().downgrade();
            let host = host_scroll_viewer.downgrade();
            DisposableAction::create(move || {
                if let Some(host) = host.upgrade() {
                    host.remove_handler(ScrollViewer::scroll_changed_event(), token);
                }
                if let Some(behavior) = behavior.upgrade() {
                    *behavior.host_scroll_viewer.borrow_mut() = None;
                }
            })
        }
    }
}

impl ViewportBehavior {
    /// The static constructor: the changes of the viewport state raise the events.
    fn static_constructor() {
        Self::is_fully_in_viewport_property()
            .changed()
            .add_class_handler::<ViewportBehavior>(|behavior, e| Self::on_is_fully_in_viewport_changed(behavior, e));
        Self::is_in_viewport_property()
            .changed()
            .add_class_handler::<ViewportBehavior>(|behavior, e| Self::on_is_in_viewport_changed(behavior, e));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: AttachedToVisualTreeBehavior::construct(<Visual as StaticType>::TYPE),
            entered_viewport: HandlerList::new(),
            exited_viewport: HandlerList::new(),
            entering_viewport: HandlerList::new(),
            exiting_viewport: HandlerList::new(),
            host_scroll_viewer: RefCell::new(None),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets a value indicating whether this behavior will remain attached after the
    /// associated element enters the viewport. When false, the behavior will remove itself after
    /// entering.
    pub fn is_always_on(&self) -> bool {
        self.get_value(Self::is_always_on_property())
    }

    pub fn set_is_always_on(&self, value: bool) {
        self.set_value(Self::is_always_on_property(), value)
    }

    /// Gets a value indicating whether the associated element is fully in the [`ScrollViewer`]
    /// viewport.
    pub fn is_fully_in_viewport(&self) -> bool {
        self.get_value(Self::is_fully_in_viewport_property())
    }

    fn set_is_fully_in_viewport(&self, value: bool) {
        self.set_value(Self::is_fully_in_viewport_property(), value)
    }

    /// Gets a value indicating whether the associated element is in the [`ScrollViewer`] viewport.
    pub fn is_in_viewport(&self) -> bool {
        self.get_value(Self::is_in_viewport_property())
    }

    fn set_is_in_viewport(&self, value: bool) {
        self.set_value(Self::is_in_viewport_property(), value)
    }

    /// Occurs when the associated element has fully entered the viewport.
    pub fn entered_viewport(&self, handler: impl Fn(Option<&BoxedValue>, &EventArgs) + 'static) -> Rc<dyn IDisposable> {
        Self::subscribe_event(self, |behavior| &behavior.entered_viewport, Rc::new(handler))
    }

    /// Occurs when the associated element has fully exited the viewport.
    pub fn exited_viewport(&self, handler: impl Fn(Option<&BoxedValue>, &EventArgs) + 'static) -> Rc<dyn IDisposable> {
        Self::subscribe_event(self, |behavior| &behavior.exited_viewport, Rc::new(handler))
    }

    /// Occurs when the associated element starts entering the viewport.
    pub fn entering_viewport(
        &self,
        handler: impl Fn(Option<&BoxedValue>, &EventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        Self::subscribe_event(self, |behavior| &behavior.entering_viewport, Rc::new(handler))
    }

    /// Occurs when the associated element starts exiting the viewport.
    pub fn exiting_viewport(
        &self,
        handler: impl Fn(Option<&BoxedValue>, &EventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        Self::subscribe_event(self, |behavior| &behavior.exiting_viewport, Rc::new(handler))
    }

    fn subscribe_event(
        &self,
        list: fn(&ViewportBehavior) -> &HandlerList<ViewportEventHandler>,
        handler: Rc<ViewportEventHandler>,
    ) -> Rc<dyn IDisposable> {
        let token = list(self).add(handler);
        let behavior = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(behavior) = behavior.upgrade() {
                list(&behavior).remove(token);
            }
        })
    }

    fn raise(&self, list: &HandlerList<ViewportEventHandler>) {
        let sender = self.associated_object().map(|associated_object| Rc::new(associated_object) as BoxedValue);
        for (_, handler) in list.snapshot().iter() {
            handler(sender.as_ref(), &EventArgs::EMPTY);
        }
    }

    fn on_scroll_changed(&self, _e: &ScrollChangedEventArgs) {
        self.evaluate_viewport_state();
    }

    fn evaluate_viewport_state(&self) {
        let Some(associated_object) = self.associated_object_as::<Visual>() else {
            return;
        };
        let Some(host_scroll_viewer) = self.host_scroll_viewer.borrow().as_ref().and_then(WeakRef::upgrade) else {
            return;
        };

        if !associated_object.is_initialized() {
            return;
        }

        let Some(matrix) = associated_object.transform_to_visual(&host_scroll_viewer) else {
            return;
        };

        let associated_element_rect = Rect::new(
            0.0,
            0.0,
            associated_object.bounds().size().width,
            associated_object.bounds().size().height,
        )
        .transform_to_aabb(matrix);

        let bounds = host_scroll_viewer.bounds();
        let host_scroll_viewer_rect = Rect::new(0.0, 0.0, bounds.width, bounds.height);

        let corners = [
            Point::new(associated_element_rect.left(), associated_element_rect.top()),
            Point::new(associated_element_rect.right(), associated_element_rect.top()),
            Point::new(associated_element_rect.right(), associated_element_rect.bottom()),
            Point::new(associated_element_rect.left(), associated_element_rect.bottom()),
        ];

        if corners.iter().any(|corner| host_scroll_viewer_rect.contains(*corner)) {
            self.set_is_in_viewport(true);

            if corners.iter().all(|corner| host_scroll_viewer_rect.contains(*corner)) {
                self.set_is_fully_in_viewport(true);
            } else {
                self.set_is_fully_in_viewport(false);
            }
        } else {
            self.set_is_in_viewport(false);
            self.set_is_fully_in_viewport(false);
        }
    }

    fn on_is_fully_in_viewport_changed(obj: &ViewportBehavior, e: &FerroPropertyChangedEventArgs<'_>) {
        let value = e.get_new_value::<bool>();
        if value {
            obj.raise(&obj.entered_viewport);

            if !obj.is_always_on() {
                if let Some(element) = obj.associated_object() {
                    let behavior: Ref<FerroObject> = obj.to_ref().upcast();
                    Interaction::get_behaviors(&element).remove(&behavior);
                }
            }
        } else {
            obj.raise(&obj.exiting_viewport);
        }
    }

    fn on_is_in_viewport_changed(obj: &ViewportBehavior, e: &FerroPropertyChangedEventArgs<'_>) {
        let value = e.get_new_value::<bool>();
        if value {
            obj.raise(&obj.entering_viewport);
        } else {
            obj.raise(&obj.exited_viewport);
        }
    }
}
