//! Original compiler-yield menu clone: type-ordered shallow snapshots, recursive submenus and
//! the original asymmetric click state hooks.
use super::{RibbonItemHelper, RibbonMenu, RibbonOverflowMenuItems};
use crate::compat::{
    MenuFlyoutItem, MenuFlyoutSeparator, MenuFlyoutSubItem, RadioMenuFlyoutItem,
    ToggleMenuFlyoutItem,
};
use ferroribbon_core::model::RibbonItemsSource;
use ferroui_base::{Ref, ferro_static_type};
use ferroui_controls::Control;
pub struct RibbonMenuCloner;
ferro_static_type!(RibbonMenuCloner);
impl RibbonMenuCloner {
    pub fn clone(items: Option<RibbonOverflowMenuItems>) -> RibbonOverflowMenuItems {
        RibbonItemsSource::deferred(move || {
            Box::new(
                items
                    .as_ref()
                    .expect("NullReference: menu items")
                    .iter()
                    .filter_map(|item| clone_item(item.as_deref()).map(Some)),
            )
        })
    }
}
fn clone_item(item: Option<&Control>) -> Option<Ref<Control>> {
    let item = item?;
    if item.is::<MenuFlyoutSeparator>() {
        return Some(MenuFlyoutSeparator::new().upcast());
    }
    if let Some(sub) = item.downcast_ref::<MenuFlyoutSubItem>() {
        let copy = MenuFlyoutSubItem::new();
        copy.set_text(sub.text());
        copy.set_icon(
            RibbonItemHelper::create_menu_icon(sub.icon())
                .expect("submenu icon conversion")
                .map(Control::boxed),
        );
        for child in
            RibbonMenuCloner::clone(Some(RibbonItemsSource::from_collection(sub.items()))).iter()
        {
            copy.items().add(child);
        }
        return Some(copy.upcast());
    }
    if let Some(radio) = item.downcast_ref::<RadioMenuFlyoutItem>() {
        let copy = RadioMenuFlyoutItem::new();
        copy.set_text(radio.text());
        copy.set_group_name(radio.group_name());
        copy.set_is_checked(radio.is_checked());
        copy.set_command(radio.command());
        copy.set_command_parameter(radio.command_parameter());
        copy.set_icon(
            RibbonItemHelper::create_menu_icon(radio.icon())
                .expect("radio icon conversion")
                .map(Control::boxed),
        );
        let source = radio.to_ref();
        copy.add_click_action(move |_, _| source.set_is_checked(true));
        return Some(copy.upcast());
    }
    if let Some(toggle) = item.downcast_ref::<ToggleMenuFlyoutItem>() {
        let copy = ToggleMenuFlyoutItem::new();
        copy.set_text(toggle.text());
        copy.set_is_checked(toggle.is_checked());
        copy.set_command(toggle.command());
        copy.set_command_parameter(toggle.command_parameter());
        copy.set_icon(
            RibbonItemHelper::create_menu_icon(toggle.icon())
                .expect("toggle icon conversion")
                .map(Control::boxed),
        );
        copy.set_keyboard_accelerator_text_override(Some(
            toggle.keyboard_accelerator_text_override(),
        ));
        let source = toggle.to_ref();
        copy.add_click_action(move |target, _| source.set_is_checked(target.is_checked()));
        return Some(copy.upcast());
    }
    if let Some(menu) = item.downcast_ref::<MenuFlyoutItem>() {
        let copy = MenuFlyoutItem::new();
        copy.set_text(menu.text());
        copy.set_command(menu.command());
        copy.set_command_parameter(menu.command_parameter());
        copy.set_icon(
            RibbonItemHelper::create_menu_icon(menu.icon())
                .expect("menu icon conversion")
                .map(Control::boxed),
        );
        copy.set_keyboard_accelerator_text_override(Some(
            menu.keyboard_accelerator_text_override(),
        ));
        copy.set_is_enabled(menu.is_enabled());
        copy.set_tag(menu.tag());
        if let Some(invoke) = RibbonMenu::get_invoke(menu) {
            let action = invoke.clone();
            copy.add_click_action(move |_, _| {
                action.try_invoke(&[]).expect("cloned menu action");
            });
            RibbonMenu::set_invoke(&copy, Some(invoke));
        }
        return Some(copy.upcast());
    }
    None
}
ferroui_base::ferro_markup_type!(static RibbonMenuCloner{namespace:"FerroRibbon.Controls",type_info:RibbonMenuCloner,methods:[static fn Clone(Option<RibbonOverflowMenuItems>)->RibbonOverflowMenuItems => RibbonMenuCloner::clone]});
// Source MenuFlyout.Items is a notifying vector. This native view retains its
// identity and checks its mutation epoch instead of snapshotting a Vec of items.
struct NativeMenuItems {
    items: ferroui_controls::ItemCollection,
    version: std::rc::Rc<std::cell::Cell<u32>>,
    token: u64,
}
impl Drop for NativeMenuItems {
    fn drop(&mut self) {
        self.items.view().remove_collection_changed(self.token);
    }
}
struct NativeCursor {
    source: std::rc::Rc<NativeMenuItems>,
    version: u32,
    index: usize,
}
impl Iterator for NativeCursor {
    type Item = Option<Ref<Control>>;
    fn next(&mut self) -> Option<Self::Item> {
        assert_eq!(
            self.version,
            self.source.version.get(),
            "InvalidOperation: collection was modified"
        );
        if self.index >= self.source.items.view().count() {
            return None;
        }
        let value = self.source.items.view().get_at(self.index);
        self.index += 1;
        Some(value.and_then(|v| Control::from_boxed(&v)))
    }
}
pub(crate) fn native_menu_items(
    items: ferroui_controls::ItemCollection,
) -> RibbonOverflowMenuItems {
    let version = std::rc::Rc::new(std::cell::Cell::new(0_u32));
    let epoch = version.clone();
    let token = items
        .view()
        .add_collection_changed(std::rc::Rc::new(move |_| {
            epoch.set(epoch.get().wrapping_add(1))
        }));
    let source = std::rc::Rc::new(NativeMenuItems {
        items,
        version,
        token,
    });
    RibbonItemsSource::new(move || {
        Box::new(NativeCursor {
            source: source.clone(),
            version: source.version.get(),
            index: 0,
        })
    })
}
