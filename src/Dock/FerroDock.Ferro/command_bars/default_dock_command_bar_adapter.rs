// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::IDockCommandBarAdapter;
use ferrodock_controls_recycling::from_recycling_value;
use ferrodock_model::{
    collections::ReadOnlyListHandle,
    command_bars::{DockCommandBarDefinition, DockCommandBarItem, DockCommandBarKind},
    BoxedValue as ModelValue,
};
use ferroui_base::{layout::Orientation, BoxedValue, Ref};
use ferroui_controls::{
    Button, ContentControl, Control, ItemsSource, Menu, MenuItem, Separator, StackPanel,
};
use ferroui_reactiveui_controls::{boxed_parameter, ReactiveCommandAdapter};
use std::rc::Rc;
/// Original default adapter rendering menus, toolbars and ribbon content.
pub struct DefaultDockCommandBarAdapter;
impl PartialEq for DefaultDockCommandBarAdapter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl DefaultDockCommandBarAdapter {
    /// Creates the original stateless adapter.
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
    fn build_menu(items: &ReadOnlyListHandle<Rc<DockCommandBarItem>>) -> Ref<Control> {
        let menu = Menu::new();
        menu.set_items_source(Some(Self::build_menu_items(items)));
        menu.upcast()
    }
    fn build_menu_items(items: &ReadOnlyListHandle<Rc<DockCommandBarItem>>) -> ItemsSource {
        let mut result = Vec::new();
        for item in ordered_items(items) {
            if item.is_separator() {
                result.push(Control::boxed(Separator::new()));
                continue;
            }
            let menu = MenuItem::new();
            menu.set_header(native(item.header()));
            menu.set_icon(native(item.icon()));
            menu.set_command(item.command().map(ReactiveCommandAdapter::command_of));
            menu.set_command_parameter(boxed_parameter(&item.command_parameter()));
            if let Some(children) = item.items().filter(|i| i.count() > 0) {
                menu.set_items_source(Some(Self::build_menu_items(&children)));
            }
            result.push(Control::boxed(menu));
        }
        ItemsSource::from_items(result.into_iter().map(Some))
    }
    fn build_tool_bar(items: &ReadOnlyListHandle<Rc<DockCommandBarItem>>) -> Ref<Control> {
        let panel = StackPanel::new();
        panel.set_orientation(Orientation::Horizontal);
        panel.set_spacing(4.0);
        for item in ordered_items(items) {
            if item.is_separator() {
                panel.children().add(Separator::new().upcast::<Control>());
                continue;
            }
            let content = Self::build_button_content(&item);
            let button = Button::new();
            button.set_content(content);
            button.set_command(item.command().map(ReactiveCommandAdapter::command_of));
            button.set_command_parameter(boxed_parameter(&item.command_parameter()));
            panel.children().add(button.upcast::<Control>());
        }
        panel.upcast()
    }
    fn build_ribbon(items: &ReadOnlyListHandle<Rc<DockCommandBarItem>>) -> Ref<Control> {
        let panel = StackPanel::new();
        panel.set_orientation(Orientation::Horizontal);
        panel.set_spacing(6.0);
        for item in ordered_items(items) {
            if item.is_separator() {
                panel.children().add(Separator::new().upcast::<Control>());
                continue;
            }
            let content = ContentControl::new();
            content.set_content(native(item.header()));
            panel.children().add(content.upcast::<Control>());
        }
        panel.upcast()
    }
    fn build_button_content(item: &DockCommandBarItem) -> Option<BoxedValue> {
        if item.icon().is_none() {
            return native(item.header());
        }
        if item.header().is_none() {
            return native(item.icon());
        }
        let panel = StackPanel::new();
        panel.set_orientation(Orientation::Horizontal);
        panel.set_spacing(4.0);
        let icon = ContentControl::new();
        icon.set_content(native(item.icon()));
        panel.children().add(icon.upcast::<Control>());
        let header = ContentControl::new();
        header.set_content(native(item.header()));
        panel.children().add(header.upcast::<Control>());
        Some(Control::boxed(panel))
    }
}
fn native(value: Option<ModelValue>) -> Option<BoxedValue> {
    value.as_ref().map(from_recycling_value)
}
fn ordered_items(
    items: &ReadOnlyListHandle<Rc<DockCommandBarItem>>,
) -> Vec<Rc<DockCommandBarItem>> {
    let mut items: Vec<_> = items.iter().collect();
    items.sort_by_key(|item| item.order());
    items
}
impl IDockCommandBarAdapter for DefaultDockCommandBarAdapter {
    fn build_bars(
        &self,
        kind: DockCommandBarKind,
        definitions: &ReadOnlyListHandle<Rc<DockCommandBarDefinition>>,
    ) -> ReadOnlyListHandle<Ref<Control>> {
        let mut definitions: Vec<_> = definitions.iter().collect();
        definitions.sort_by_key(|definition| definition.order());
        let mut bars = Vec::new();
        for definition in definitions {
            if let Some(control) = native(definition.content())
                .as_ref()
                .and_then(Control::from_boxed)
            {
                bars.push(control);
                continue;
            }
            if definition.items().is_none_or(|i| i.count() == 0) {
                if definition.content().is_some() {
                    let content = ContentControl::new();
                    content.set_content(native(definition.content()));
                    bars.push(content.upcast::<Control>());
                }
                continue;
            }
            let items = definition.items().expect("source items checked");
            bars.push(match kind {
                DockCommandBarKind::Menu => Self::build_menu(&items),
                DockCommandBarKind::ToolBar => Self::build_tool_bar(&items),
                DockCommandBarKind::Ribbon => Self::build_ribbon(&items),
            });
        }
        ReadOnlyListHandle::from_items(bars)
    }
}
ferroui_base::ferro_markup_type!(class DefaultDockCommandBarAdapter {
    namespace:"FerroDock.Ferro.CommandBars",this:Rc<DefaultDockCommandBarAdapter>,handles:[Rc<DefaultDockCommandBarAdapter>,Option<Rc<DefaultDockCommandBarAdapter>>],
    interfaces:[Rc<dyn IDockCommandBarAdapter>],constructors:[()=>DefaultDockCommandBarAdapter::new],
    methods:[fn BuildBars(DockCommandBarKind,ReadOnlyListHandle<Rc<DockCommandBarDefinition>>) -> ReadOnlyListHandle<Ref<Control>> => (|this:&Rc<DefaultDockCommandBarAdapter>,kind,definitions|this.build_bars(kind,&definitions))],
});
