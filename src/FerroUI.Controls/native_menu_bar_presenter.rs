use crate::generators::RecycleKey;
use crate::i_native_menu_item_exporter_events_impl_bridge::INativeMenuItemExporterEventsImplBridge;
use crate::items_source::{unbox_item, ItemsSource};
use crate::primitives::{
    HeaderedSelectingItemsControl, HeaderedSelectingItemsControlImpl, SelectingItemsControlImpl, TemplatedControlImpl,
};
use crate::{
    Control, ControlImpl, Image, ItemsControlImpl, ItemsControlImplExt, Menu, MenuBaseImpl, MenuItem, MenuItemImpl, NativeMenuItem,
    NativeMenuItemBase, NativeMenuItemSeparator, Separator, ToolTip,
};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::{InputElement, InputElementImpl};
use ferroui_base::interactivity::{Interactive, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::IImage;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectExtensions,
    FerroObjectImpl, Ref, StyledElementImpl, TypeInfo, Visual, VisualImpl,
};
use std::rc::Rc;

/// The menu that shows the items of a native menu inside a window.
#[repr(C)]
pub(crate) struct NativeMenuBarPresenter {
    base: Menu,
}

ferro_class!(NativeMenuBarPresenter: Menu);
ferro_class_info!(NativeMenuBarPresenter { new: NativeMenuBarPresenter::new });

ferro_impl_classes!(
    NativeMenuBarPresenter: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    SelectingItemsControlImpl,
    MenuBaseImpl
);

impl StyledElementImpl for NativeMenuBarPresenter {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        Menu::TYPE
    }
}

impl ItemsControlImpl for NativeMenuBarPresenter {
    fn create_container_for_item_override(
        this: &Self,
        item: &Option<BoxedValue>,
        index: i32,
        recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        Self::create_container_for_native_item(item, index, recycle_key)
            .unwrap_or_else(|| Self::parent_create_container_for_item_override(this, item, index, recycle_key))
    }
}

impl NativeMenuBarPresenter {
    pub(crate) fn new() -> Ref<Self> {
        instantiate(Self { base: Menu::construct() })
    }

    pub(crate) fn create_container_for_native_item(
        item: &Option<BoxedValue>,
        _index: i32,
        _recycle_key: Option<RecycleKey>,
    ) -> Option<Ref<Control>> {
        let item = unbox_item::<Ref<NativeMenuItemBase>>(item)?;

        if item.is::<NativeMenuItemSeparator>() {
            return Some(Separator::new().upcast());
        }

        let native_item = item.cast::<NativeMenuItem>()?;
        let new_item = NativeMenuItemPresenter::new();

        new_item.set_items_source(native_item.menu().map(|menu| ItemsSource::from(Rc::new(menu.items()))));
        new_item.bind(
            HeaderedSelectingItemsControl::header_property(),
            native_item.get_observable_with(NativeMenuItem::header_property(), |header| {
                header.map(|header| -> BoxedValue { Rc::new(header) })
            }),
            BindingPriority::LocalValue,
        );
        new_item.bind(
            MenuItem::icon_property(),
            native_item.get_observable_with(NativeMenuItem::icon_property(), |icon| {
                icon.map(|bitmap| {
                    let image = Image::new();
                    let source: Rc<dyn IImage> = bitmap;
                    image.set_source(Some(source));
                    Control::boxed(image)
                })
            }),
            BindingPriority::LocalValue,
        );
        new_item.bind_indexer(
            &!MenuItem::is_checked_property().as_property().bind(),
            &native_item.indexer(&!NativeMenuItem::is_checked_property().as_property().bind()),
        );
        new_item.bind(
            InputElement::is_enabled_property(),
            native_item.get_observable(NativeMenuItem::is_enabled_property()),
            BindingPriority::LocalValue,
        );
        new_item.bind(
            Visual::is_visible_property(),
            native_item.get_observable(NativeMenuItem::is_visible_property()),
            BindingPriority::LocalValue,
        );
        new_item.bind(
            MenuItem::command_property(),
            native_item.get_observable(NativeMenuItem::command_property()),
            BindingPriority::LocalValue,
        );
        new_item.bind(
            MenuItem::command_parameter_property(),
            native_item.get_observable(NativeMenuItem::command_parameter_property()),
            BindingPriority::LocalValue,
        );
        new_item.bind(
            MenuItem::input_gesture_property(),
            native_item.get_observable(NativeMenuItem::gesture_property()),
            BindingPriority::LocalValue,
        );
        new_item.bind(
            MenuItem::toggle_type_property(),
            native_item.get_observable(NativeMenuItem::toggle_type_property()),
            BindingPriority::LocalValue,
        );
        new_item.bind(
            ToolTip::tip_property(),
            native_item.get_observable_with(NativeMenuItem::tool_tip_property(), |tip| {
                tip.map(|tip| -> BoxedValue { Rc::new(tip) })
            }),
            BindingPriority::LocalValue,
        );

        new_item.click(Self::menu_item_on_click);

        Some(new_item.upcast())
    }

    fn menu_item_on_click(sender: &Interactive, _e: &RoutedEventArgs) {
        let sender = sender.downcast_ref::<MenuItem>().expect("the sender is a menu item");
        let item = unbox_item::<Ref<NativeMenuItemBase>>(&sender.data_context())
            .and_then(|item| item.cast::<NativeMenuItem>());

        if let Some(item) = item {
            if item.has_click_handlers() {
                let bridge: Rc<dyn INativeMenuItemExporterEventsImplBridge> = item.to_exporter_events_bridge();
                bridge.raise_clicked();
            }
        }
    }
}

/// The menu item that shows a native menu item (and the items of its
/// submenu) inside a window.
#[repr(C)]
struct NativeMenuItemPresenter {
    base: MenuItem,
}

ferro_class!(NativeMenuItemPresenter: MenuItem);
ferro_class_info!(NativeMenuItemPresenter { new: NativeMenuItemPresenter::new });

ferro_impl_classes!(
    NativeMenuItemPresenter: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    SelectingItemsControlImpl,
    HeaderedSelectingItemsControlImpl,
    MenuItemImpl
);

impl StyledElementImpl for NativeMenuItemPresenter {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        MenuItem::TYPE
    }
}

impl ItemsControlImpl for NativeMenuItemPresenter {
    fn create_container_for_item_override(
        this: &Self,
        item: &Option<BoxedValue>,
        index: i32,
        recycle_key: Option<RecycleKey>,
    ) -> Ref<Control> {
        NativeMenuBarPresenter::create_container_for_native_item(item, index, recycle_key)
            .unwrap_or_else(|| Self::parent_create_container_for_item_override(this, item, index, recycle_key))
    }
}

impl NativeMenuItemPresenter {
    fn new() -> Ref<Self> {
        instantiate(Self { base: MenuItem::construct() })
    }
}
