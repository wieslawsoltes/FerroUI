//! Port of `Control/DragControlBehavior.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{InputElement, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventHandlerToken};
use ferroui_base::media::{ITransform, TranslateTransform};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, ElementRef, FerroObjectImpl, FerroProperty, Nullable,
    Point, Ref, StaticType, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Drags the target control (or else the associated control) inside its parent with the
/// pointer, through a translate transform.
#[repr(C)]
pub struct DragControlBehavior {
    base: StyledElementBehaviorOfT,
    parent: RefCell<Option<Ref<Control>>>,
    previous: Cell<Point>,
    // The handler added to the associated control and the handlers added to the parent (the
    // reference removes its delegates).
    pointer_pressed_handler: Cell<Option<RoutedEventHandlerToken>>,
    parent_handlers: Cell<Option<(RoutedEventHandlerToken, RoutedEventHandlerToken)>>,
}

ferro_class!(DragControlBehavior: StyledElementBehaviorOfT);
ferro_class_info!(DragControlBehavior {
    new: DragControlBehavior::new,
    markup: {
        property_attributes: [
            TargetControl: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(DragControlBehavior: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl DragControlBehavior {
        /// Identifies the `TargetControl` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the behavior does not own the control, so the
        /// value is an element reference.
        pub fn target_control_property() -> StyledProperty<Option<ElementRef<Control>>> {
            ValueTypes::register_element_ref::<Control>();
            FerroProperty::register::<DragControlBehavior, _>("TargetControl", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for DragControlBehavior {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(source) = this.associated_object_as::<Control>() else {
                return;
            };

            let behavior = this.to_ref().downgrade();
            let handler = source.add_handler(
                InputElement::pointer_pressed_event(),
                move |_sender: &Interactive, e: &PointerPressedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.source_pointer_pressed(e);
                    }
                },
            );
            this.pointer_pressed_handler.set(Some(handler));
        }

        fn on_detached_from_visual_tree(this: &Self) {
            if let Some(source) = this.associated_object_as::<Control>() {
                if let Some(handler) = this.pointer_pressed_handler.take() {
                    source.remove_handler(InputElement::pointer_pressed_event(), handler);
                }
            }

            // Deviation (DEVIATIONS.md, Behaviors): the handlers added to the parent are removed
            // too; the reference forgets the parent and leaves them.
            this.detach_parent();
        }
    }
}

impl DragControlBehavior {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE),
            parent: RefCell::new(None),
            previous: Cell::new(Point::default()),
            pointer_pressed_handler: Cell::new(None),
            parent_handlers: Cell::new(None),
        }
    }

    /// Creates a behavior.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the control that is dragged; when it is null, the associated control. Markup
    /// resolves a name given for it to the named element. This is a property of the object model.
    pub fn target_control(&self) -> Option<Ref<Control>> {
        ElementRef::resolve(&self.get_value(Self::target_control_property()))
    }

    pub fn set_target_control(&self, value: impl Into<Nullable<Control>>) {
        self.set_value(Self::target_control_property(), ElementRef::from_nullable(value.into().0))
    }

    fn target(&self) -> Option<Ref<Control>> {
        self.target_control().or_else(|| self.associated_object_as::<Control>())
    }

    fn parent_visual(parent: &Option<Ref<Control>>) -> Option<&Visual> {
        parent.as_ref().map(|parent| {
            let visual: &Visual = parent;
            visual
        })
    }

    fn translate_transform_of(target: &Control) -> Option<Ref<TranslateTransform>> {
        target
            .render_transform()
            .and_then(|transform| transform.as_object().and_then(|object| object.to_ref().cast::<TranslateTransform>()))
    }

    fn source_pointer_pressed(&self, e: &PointerPressedEventArgs) {
        let Some(target) = self.target() else {
            return;
        };

        // Deviation (DEVIATIONS.md, Behaviors): a press while a drag is in progress moves the
        // handlers to the new parent; the reference adds them a second time.
        self.detach_parent();

        let parent = target.parent().and_then(|parent| parent.cast::<Control>());

        if Self::translate_transform_of(&target).is_none() {
            let transform = TranslateTransform::new();
            let render_transform: Rc<dyn ITransform> = (&transform).into();
            target.set_render_transform(Some(render_transform));
        }

        self.previous.set(e.get_position(Self::parent_visual(&parent)));
        if let Some(parent) = &parent {
            let behavior = self.to_ref().downgrade();
            let moved = parent.add_handler(
                InputElement::pointer_moved_event(),
                move |_sender: &Interactive, e: &PointerEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.parent_pointer_moved(e);
                    }
                },
            );
            let behavior = self.to_ref().downgrade();
            let released = parent.add_handler(
                InputElement::pointer_released_event(),
                move |_sender: &Interactive, e: &PointerReleasedEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.parent_pointer_released(e);
                    }
                },
            );
            self.parent_handlers.set(Some((moved, released)));
        }
        *self.parent.borrow_mut() = parent;
    }

    fn parent_pointer_moved(&self, args: &PointerEventArgs) {
        let Some(target) = self.target() else {
            return;
        };

        let parent = self.parent.borrow().clone();
        let position = args.get_position(Self::parent_visual(&parent));
        if let Some(transform) = Self::translate_transform_of(&target) {
            let previous = self.previous.get();
            transform.set_x(transform.x() + position.x - previous.x);
            transform.set_y(transform.y() + position.y - previous.y);
        }

        self.previous.set(position);
    }

    fn parent_pointer_released(&self, _e: &PointerReleasedEventArgs) {
        self.detach_parent();
    }

    fn detach_parent(&self) {
        let parent = self.parent.borrow_mut().take();
        let handlers = self.parent_handlers.take();
        if let (Some(parent), Some((moved, released))) = (parent, handlers) {
            parent.remove_handler(InputElement::pointer_moved_event(), moved);
            parent.remove_handler(InputElement::pointer_released_event(), released);
        }
    }
}
