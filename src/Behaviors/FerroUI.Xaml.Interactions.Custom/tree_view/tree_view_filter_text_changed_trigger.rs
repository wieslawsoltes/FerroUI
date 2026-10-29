//! Port of `TreeView/TreeViewFilterTextChangedTrigger.cs`.

use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventHandlerToken};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, ElementRef, FerroObjectImpl,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty, WeakRef,
};
use ferroui_controls::{TextBox, TextChangedEventArgs};
use ferroui_xaml_interactivity::{InteractiveTriggerBase, StyledElementBehaviorImpl};
use std::cell::RefCell;
use std::rc::Rc;

/// Executes actions when the search box text changes, with the args of the change as the
/// parameter.
#[repr(C)]
pub struct TreeViewFilterTextChangedTrigger {
    base: InteractiveTriggerBase,
    // The search box the handler is attached to.
    text_changed: RefCell<Option<(WeakRef<TextBox>, RoutedEventHandlerToken)>>,
}

ferro_class!(TreeViewFilterTextChangedTrigger: InteractiveTriggerBase);
ferro_class_info!(TreeViewFilterTextChangedTrigger {
    new: TreeViewFilterTextChangedTrigger::new,
    markup: {
        property_attributes: [
            SearchBox: [ResolveByName],
        ],
    },
});
ferroui_base::ferro_impl_classes!(TreeViewFilterTextChangedTrigger: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl TreeViewFilterTextChangedTrigger {
        /// Identifies the `SearchBox` property.
        ///
        /// Deviation (DEVIATIONS.md, Behaviors): the trigger does not own the search box, so the
        /// value is an element reference.
        pub fn search_box_property() -> StyledProperty<Option<ElementRef<TextBox>>> {
            ValueTypes::register_element_ref::<TextBox>();
            FerroProperty::register::<TreeViewFilterTextChangedTrigger, _>("SearchBox", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl StyledElementBehaviorImpl for TreeViewFilterTextChangedTrigger {
        fn on_attached_to_visual_tree(this: &Self) {
            this.remove_text_changed_handler();

            if let Some(search_box) = this.search_box() {
                let trigger = this.to_ref().downgrade();
                let token = search_box.add_handler_with(
                    TextBox::text_changed_event(),
                    move |_sender: &Interactive, e: &TextChangedEventArgs| {
                        if let Some(trigger) = trigger.upgrade() {
                            trigger.on_text_changed(e);
                        }
                    },
                    this.routing_strategies(),
                    false,
                );
                *this.text_changed.borrow_mut() = Some((search_box.downgrade(), token));
            }
        }

        fn on_detached_from_visual_tree(this: &Self) {
            this.remove_text_changed_handler();
        }
    }
}

impl TreeViewFilterTextChangedTrigger {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InteractiveTriggerBase::construct(), text_changed: RefCell::new(None) }
    }

    /// Creates a trigger.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the search box control. Markup resolves a name given for it to the named
    /// element.
    pub fn search_box(&self) -> Option<Ref<TextBox>> {
        ElementRef::resolve(&self.get_value(Self::search_box_property()))
    }

    pub fn set_search_box(&self, value: impl Into<Nullable<TextBox>>) {
        self.set_value(Self::search_box_property(), ElementRef::from_nullable(value.into().0))
    }

    /// Removes the handler from the search box it was added to (the reference removes it from
    /// the current search box).
    // Deviation (DEVIATIONS.md, Behaviors): a handler is removed by its token from the element it
    // was added to, which the reference does through the current search box.
    fn remove_text_changed_handler(&self) {
        let text_changed = self.text_changed.borrow_mut().take();
        if let Some((search_box, token)) = text_changed {
            if let Some(search_box) = search_box.upgrade() {
                search_box.remove_handler(TextBox::text_changed_event(), token);
            }
        }
    }

    fn on_text_changed(&self, e: &TextChangedEventArgs) {
        let parameter: BoxedValue = Rc::new(IRoutedEventArgs::share(e));
        self.execute(Some(&parameter));
    }
}
