use crate::generators::RecycleKey;
use crate::metadata::TemplatePartAttribute;
use crate::primitives::{
    SelectedItemsList, SelectingItemsControl, SelectingItemsControlImpl, TemplateAppliedEventArgs,
    TemplatedControlImpl, TemplatedControlImplExt,
};
use crate::selection::ISelectionModel;
use crate::templates::{FuncTemplate, ITemplateOf};
use crate::{
    Control, ControlImpl, ItemsControl, ItemsControlImpl, ListBoxItem, Panel, SelectionMode, VirtualizingStackPanel,
};
use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, KeyEventArgs, KeyModifiers, KeyboardNavigation,
    KeyboardNavigationMode,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, BoxedValue, DirectProperty, FerroObjectImpl, FerroProperty, Ref, StaticType, StyledElementImpl, StyledProperty, VisualImpl,
};
use std::cell::RefCell;
use std::rc::Rc;

/// An [`ItemsControl`] in which individual items can be selected.
#[repr(C)]
pub struct ListBox {
    base: SelectingItemsControl,
    scroll: RefCell<Option<Ref<InputElement>>>,
}

ferro_class!(ListBox: SelectingItemsControl);
ferroui_base::ferro_class_info!(ListBox { new: ListBox::new });
ferro_impl_classes!(
    ListBox: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    SelectingItemsControlImpl
);

ferroui_base::ferro_impl_classes!(ListBox: FerroObjectImpl);

impl InputElementImpl for ListBox {
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        let hotkeys = this.get_platform_settings().map(|settings| settings.hotkey_configuration());
        let ctrl = hotkeys.as_ref().is_some_and(|hotkeys| e.key_modifiers.contains(hotkeys.command_modifiers));
        let direction = e.key.to_navigation_direction(KeyModifiers::NONE).filter(|direction| direction.is_directional());

        if let (false, Some(direction)) = (ctrl, direction) {
            let moved =
                this.move_selection(direction, this.wrap_selection(), e.key_modifiers.contains(KeyModifiers::SHIFT));
            e.set_handled(e.handled() | moved);
        } else if this.selection_mode().contains(SelectionMode::MULTIPLE)
            && hotkeys.as_ref().is_some_and(|hotkeys| hotkeys.select_all.iter().any(|x| x.matches(Some(e))))
        {
            this.selection().select_all();
            e.set_handled(true);
        }

        Self::parent_on_key_down(this, e);
    }
}

impl TemplatedControlImpl for ListBox {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        let scroll = e
            .name_scope()
            .find("PART_ScrollViewer")
            .and_then(|element| element.cast::<InputElement>())
            .filter(|element| element.as_scrollable().is_some());
        this.set_scroll(scroll);
    }
}

impl ItemsControlImpl for ListBox {
    fn create_container_for_item_override(
        _this: &Self,
        _item: &Option<BoxedValue>,
        _index: i32,
        _recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        ListBoxItem::new().upcast()
    }

    fn needs_container_override(this: &Self, item: &Option<BoxedValue>, _index: i32) -> (bool, Option<RecycleKey>) {
        this.needs_container::<ListBoxItem>(item)
    }
}

impl ControlImpl for ListBox {
    fn on_create_automation_peer(this: &Self) -> Ref<crate::automation::peers::AutomationPeer> {
        crate::automation::peers::ListBoxAutomationPeer::new(this).upcast()
    }
}

impl ListBox {
    /// The named parts expected in the control template: the scrollable
    /// that scrolls the items.
    pub const TEMPLATE_PARTS: &'static [TemplatePartAttribute] =
        &[TemplatePartAttribute::new("PART_ScrollViewer", <InputElement as StaticType>::TYPE)];
}

ferroui_base::ferro_properties! { impl ListBox {
    ferro_property!(
        /// Defines the `Scroll` property.
        pub fn scroll_property() -> DirectProperty<ListBox, Option<Ref<InputElement>>> {
            FerroProperty::register_direct::<ListBox, _>("Scroll", |o| o.scroll(), None, None)
        }
    );
} }

impl ListBox {
    /// Defines the `SelectedItems` property.
    pub fn selected_items_property() -> &'static DirectProperty<SelectingItemsControl, Option<SelectedItemsList>> {
        SelectingItemsControl::selected_items_property()
    }

    /// Defines the `Selection` property.
    pub fn selection_property() -> &'static DirectProperty<SelectingItemsControl, Option<Rc<dyn ISelectionModel>>> {
        SelectingItemsControl::selection_property()
    }

    /// Defines the `SelectionMode` property.
    pub fn selection_mode_property() -> &'static StyledProperty<SelectionMode> {
        SelectingItemsControl::selection_mode_property()
    }

    fn static_constructor() {
        // The default value for the `ItemsPanel` property.
        let default_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> =
            FuncTemplate::new(|| Some(VirtualizingStackPanel::new().upcast::<Panel>()));
        ItemsControl::items_panel_property().override_default_value::<ListBox>(default_panel);
        KeyboardNavigation::tab_navigation_property().override_default_value::<ListBox>(KeyboardNavigationMode::Once);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: SelectingItemsControl::construct(), scroll: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the scroll information for the [`ListBox`]: the element of the
    /// template that implements the scrollable contract (see
    /// `InputElement::as_scrollable`).
    pub fn scroll(&self) -> Option<Ref<InputElement>> {
        self.scroll.borrow().clone()
    }

    fn set_scroll(&self, value: Option<Ref<InputElement>>) {
        self.set_and_raise(Self::scroll_property(), &self.scroll, value);
    }

    /// Selects all items in the [`ListBox`].
    pub fn select_all(&self) {
        self.selection().select_all()
    }

    /// Deselects all items in the [`ListBox`].
    pub fn unselect_all(&self) {
        self.selection().clear()
    }
}
