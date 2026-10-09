use super::{
    SelectingItemsControl, SelectingItemsControlImpl, SelectingItemsControlImplExt, TabStripItem, TemplatedControlImpl,
};
use crate::generators::RecycleKey;
use crate::templates::{FuncTemplate, ITemplateOf};
use crate::{Control, ControlImpl, ItemsControl, ItemsControlImpl, Panel, SelectionMode, WrapPanel};
use ferroui_base::input::{
    FocusChangedEventArgs, InputElement, InputElementImpl, NavigationMethod, PointerEventArgs, PointerUpdateKind,
};
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl};
use ferroui_base::layout::{LayoutableImpl, Orientation};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref,
    StyledElementImpl, Visual, VisualImpl,
};
use std::rc::Rc;

/// A strip of tabs: a [`SelectingItemsControl`] that always has a selected
/// item.
#[repr(C)]
pub struct TabStrip {
    base: SelectingItemsControl,
}

ferro_class!(TabStrip: SelectingItemsControl);
ferroui_base::ferro_class_info!(TabStrip { new: TabStrip::new });
ferro_impl_classes!(
    TabStrip: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

ferroui_base::ferro_impl_classes!(TabStrip: FerroObjectImpl);

impl ItemsControlImpl for TabStrip {
    fn create_container_for_item_override(
        _this: &Self,
        _item: &Option<BoxedValue>,
        _index: i32,
        _recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        TabStripItem::new().upcast()
    }

    fn needs_container_override(this: &Self, item: &Option<BoxedValue>, _index: i32) -> (bool, Option<RecycleKey>) {
        this.needs_container::<TabStripItem>(item)
    }
}

impl SelectingItemsControlImpl for TabStrip {
    fn should_trigger_selection(this: &Self, selectable: &Visual, e: &PointerEventArgs) -> bool {
        matches!(
            e.properties().pointer_update_kind,
            PointerUpdateKind::LeftButtonPressed | PointerUpdateKind::LeftButtonReleased
        ) && Self::parent_should_trigger_selection(this, selectable, e)
    }

    fn update_selection_from_event(this: &Self, container: &Ref<Control>, event_args: &dyn IRoutedEventArgs) -> bool {
        if let Some(focus) = event_args.downcast_ref::<FocusChangedEventArgs>() {
            if focus.navigation_method != NavigationMethod::Directional {
                return false;
            }
        }

        Self::parent_update_selection_from_event(this, container, event_args)
    }
}

impl TabStrip {
    fn static_constructor() {
        // The default value for the `ItemsPanel` property.
        let default_panel: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> = FuncTemplate::new(|| {
            let panel = WrapPanel::new();
            panel.set_orientation(Orientation::Horizontal);
            Some(panel.upcast::<Panel>())
        });

        SelectingItemsControl::selection_mode_property()
            .override_default_value::<TabStrip>(SelectionMode::ALWAYS_SELECTED);
        InputElement::focusable_property().override_default_value::<TabStrip>(false);
        ItemsControl::items_panel_property().override_default_value::<TabStrip>(default_panel);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: SelectingItemsControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
