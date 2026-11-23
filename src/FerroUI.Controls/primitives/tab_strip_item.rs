use super::TemplatedControlImpl;
use crate::{ContentControlImpl, ControlImpl, ListBoxItem};
use ferroui_base::input::{FocusChangedEventArgs, InputElementImpl, InputElementImplExt};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use crate::templates::IDataTemplate;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref, StyledElementImpl,
    StyledProperty, VisualImpl,
};
use std::rc::Rc;

/// Represents a tab in a `TabStrip`.
#[repr(C)]
pub struct TabStripItem {
    base: ListBoxItem,
}

ferro_class!(TabStripItem: ListBoxItem);
ferroui_base::ferro_class_info!(TabStripItem { new: TabStripItem::new });
ferro_impl_classes!(
    TabStripItem: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl InputElementImpl for TabStripItem {
    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);
        this.update_selection_from_event(e);
    }
}

ferroui_base::ferro_properties! { impl TabStripItem {
    ferro_property!(
        /// Defines the `Icon` property.
        pub fn icon_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<TabStripItem, _>("Icon", None)
        }
    );

    ferro_property!(
        /// Defines the `IconTemplate` property.
        pub fn icon_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<TabStripItem, _>("IconTemplate", None)
        }
    );
} }

impl TabStripItem {
    /// The icon displayed alongside the content.
    pub fn icon(&self) -> Option<BoxedValue> {
        self.get_value(Self::icon_property())
    }

    pub fn set_icon(&self, value: Option<BoxedValue>) {
        self.set_value(Self::icon_property(), value)
    }

    /// The data template used to display the icon.
    pub fn icon_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::icon_template_property())
    }

    pub fn set_icon_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::icon_template_property(), value)
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ListBoxItem::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
