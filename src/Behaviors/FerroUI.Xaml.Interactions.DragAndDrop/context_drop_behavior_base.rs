//! Port of `ContextDropBehaviorBase.cs`.

use crate::drag_drop_context_store::DragDropContextStore;
use crate::routed_handlers::RoutedHandlers;
use crate::values::boxed_object;
use ferroui_base::input::{DataFormat, DataFormatOf, DataTransferExtensions, DragDrop, DragEventArgs};
use ferroui_base::interactivity::{Interactive, RoutedEventArgs};
use ferroui_base::{
    ferro_class, ferro_properties, BoxedValue, FerroObjectImpl, FerroProperty, StaticType, StyledElementImpl,
    StyledProperty,
};
use ferroui_controls::Control;
use ferroui_xaml_interactivity::{StyledElementBehaviorImpl, StyledElementBehaviorOfT};

/// Behavior base class that enables dropping context data onto the associated control: the drag
/// events of the control go to the members of [`ContextDropBehaviorBaseImpl`], with the context of
/// the drag source (found by its key in the drag data) and the [`context`](Self::context) of the
/// target, or else the data context of the control.
///
/// The class is abstract.
#[repr(C)]
pub struct ContextDropBehaviorBase {
    base: StyledElementBehaviorOfT,
    handlers: RoutedHandlers,
}

ferro_class! {
    ContextDropBehaviorBase: StyledElementBehaviorOfT, virtuals ContextDropBehaviorBaseImpl: StyledElementBehaviorImpl {
        /// Called when a drag enters the target.
        fn on_enter(this, sender: Option<&BoxedValue>, e: &DragEventArgs, source_context: Option<&BoxedValue>, target_context: Option<&BoxedValue>);
        /// Called when the drag leaves the target.
        fn on_leave(this, sender: Option<&BoxedValue>, e: &RoutedEventArgs);
        /// Called repeatedly as the drag moves over the target.
        fn on_over(this, sender: Option<&BoxedValue>, e: &DragEventArgs, source_context: Option<&BoxedValue>, target_context: Option<&BoxedValue>);
        /// Called when the drag is dropped on the target.
        fn on_drop(this, sender: Option<&BoxedValue>, e: &DragEventArgs, source_context: Option<&BoxedValue>, target_context: Option<&BoxedValue>);
    }
}

ferroui_base::ferro_impl_classes!(ContextDropBehaviorBase: FerroObjectImpl, StyledElementImpl);

impl ContextDropBehaviorBaseImpl for ContextDropBehaviorBase {
    fn on_enter(
        _this: &Self,
        _sender: Option<&BoxedValue>,
        _e: &DragEventArgs,
        _source_context: Option<&BoxedValue>,
        _target_context: Option<&BoxedValue>,
    ) {
        panic!("ContextDropBehaviorBase is abstract: 'on_enter' must be implemented by the deriving class")
    }

    fn on_leave(_this: &Self, _sender: Option<&BoxedValue>, _e: &RoutedEventArgs) {
        panic!("ContextDropBehaviorBase is abstract: 'on_leave' must be implemented by the deriving class")
    }

    fn on_over(
        _this: &Self,
        _sender: Option<&BoxedValue>,
        _e: &DragEventArgs,
        _source_context: Option<&BoxedValue>,
        _target_context: Option<&BoxedValue>,
    ) {
        panic!("ContextDropBehaviorBase is abstract: 'on_over' must be implemented by the deriving class")
    }

    fn on_drop(
        _this: &Self,
        _sender: Option<&BoxedValue>,
        _e: &DragEventArgs,
        _source_context: Option<&BoxedValue>,
        _target_context: Option<&BoxedValue>,
    ) {
        panic!("ContextDropBehaviorBase is abstract: 'on_drop' must be implemented by the deriving class")
    }
}

ferro_properties! {
    impl ContextDropBehaviorBase {
        /// Identifies the `Context` property.
        pub fn context_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ContextDropBehaviorBase, _>("Context", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for ContextDropBehaviorBase {
        fn on_attached_to_visual_tree(this: &Self) {
            let Some(associated_object) = this.associated_object_as::<Control>() else {
                return;
            };

            DragDrop::set_allow_drop(&associated_object, true);
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                DragDrop::drag_enter_event(),
                move |sender: &Interactive, e: &DragEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.drag_enter(&boxed_object(&sender.to_ref()), e);
                    }
                },
                Interactive::DEFAULT_ROUTES,
                false,
            );
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                DragDrop::drag_leave_event(),
                move |sender: &Interactive, e: &DragEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.drag_leave(&boxed_object(&sender.to_ref()), e);
                    }
                },
                Interactive::DEFAULT_ROUTES,
                false,
            );
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                DragDrop::drag_over_event(),
                move |sender: &Interactive, e: &DragEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.drag_over(&boxed_object(&sender.to_ref()), e);
                    }
                },
                Interactive::DEFAULT_ROUTES,
                false,
            );
            let behavior = this.to_ref().downgrade();
            this.handlers.add(
                &associated_object,
                DragDrop::drop_event(),
                move |sender: &Interactive, e: &DragEventArgs| {
                    if let Some(behavior) = behavior.upgrade() {
                        behavior.drop_event_handler(&boxed_object(&sender.to_ref()), e);
                    }
                },
                Interactive::DEFAULT_ROUTES,
                false,
            );
        }

        fn on_detached_from_visual_tree(this: &Self) {
            if let Some(associated_object) = this.associated_object_as::<Control>() {
                DragDrop::set_allow_drop(&associated_object, false);
            }
            this.handlers.clear();
        }
    }
}

impl ContextDropBehaviorBase {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementBehaviorOfT::construct(<Control as StaticType>::TYPE), handlers: RoutedHandlers::new() }
    }

    /// Identifies the application data format used to store context identifiers.
    ///
    /// Deviation (DEVIATIONS.md, Behaviors): the identifier of the format names the namespace of
    /// the port.
    pub fn context_data_transfer_format() -> DataFormatOf<String> {
        DataFormat::create_string_application_format("FerroUI.Xaml.Interactions.DragAndDrop.Context")
    }

    /// Gets or sets context data provided to the drop handler.
    pub fn context(&self) -> Option<BoxedValue> {
        self.get_value(Self::context_property())
    }

    pub fn set_context(&self, value: Option<BoxedValue>) {
        self.set_value(Self::context_property(), value)
    }

    /// The context of the drag source (`DragDropContextStore.Get(e.DataTransfer.TryGetValue(..))`).
    fn source_context(e: &DragEventArgs) -> Option<BoxedValue> {
        let source_context_key = e.data_transfer().try_get_value(&Self::context_data_transfer_format());
        DragDropContextStore::get(source_context_key.as_deref())
    }

    /// The context of the target: the context of the behavior, or the data context of its control.
    fn target_context(&self) -> Option<BoxedValue> {
        self.context()
            .or_else(|| self.associated_object_as::<Control>().and_then(|associated_object| associated_object.data_context()))
    }

    fn drag_enter(&self, sender: &BoxedValue, e: &DragEventArgs) {
        let source_context = Self::source_context(e);
        let target_context = self.target_context();
        self.on_enter(Some(sender), e, source_context.as_ref(), target_context.as_ref());
    }

    fn drag_leave(&self, sender: &BoxedValue, e: &RoutedEventArgs) {
        self.on_leave(Some(sender), e);
    }

    fn drag_over(&self, sender: &BoxedValue, e: &DragEventArgs) {
        let source_context = Self::source_context(e);
        let target_context = self.target_context();
        self.on_over(Some(sender), e, source_context.as_ref(), target_context.as_ref());
    }

    // `Drop` of the reference: a method named `drop` would be taken for the destructor.
    fn drop_event_handler(&self, sender: &BoxedValue, e: &DragEventArgs) {
        let source_context = Self::source_context(e);
        let target_context = self.target_context();
        self.on_drop(Some(sender), e, source_context.as_ref(), target_context.as_ref());
    }
}
