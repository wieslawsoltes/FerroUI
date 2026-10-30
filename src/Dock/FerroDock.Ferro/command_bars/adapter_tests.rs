// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use super::dock_command_bar_manager::tests::{bar, item, list, run};
use super::*;
use ferrodock_model::{
    command_bars::{
        DockCommandBarDefinition, DockCommandBarItem, DockCommandBarKind as Kind,
        DockCommandBarMergeMode as Mode,
    },
    BoxedValue as ModelValue,
};
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::{Button, ContentControl, Control, Menu, MenuItem, Separator, StackPanel};
use std::{any::Any, cell::Cell, rc::Rc};
struct EmptyDisposable;
impl ferroui_system::IDisposable for EmptyDisposable {
    fn dispose(&self) {}
}
struct Command {
    parameter: Rc<Cell<i32>>,
    calls: Cell<usize>,
}
impl ferroui_reactiveui::ICommand for Command {
    fn can_execute(&self, parameter: Option<&dyn Any>) -> bool {
        parameter
            .and_then(|p| p.downcast_ref::<Cell<i32>>())
            .is_some_and(|p| std::ptr::eq(p, &*self.parameter) && p.get() > 0)
    }
    fn execute(&self, parameter: Option<&dyn Any>) {
        assert!(self.can_execute(parameter));
        self.calls.set(self.calls.get() + 1);
    }
    fn can_execute_changed(&self, _handler: Rc<dyn Fn()>) -> Rc<dyn ferroui_system::IDisposable> {
        Rc::new(EmptyDisposable)
    }
}
fn control(source: Option<BoxedValue>) -> Ref<Control> {
    Control::from_boxed(&source.unwrap()).unwrap()
}
#[test]
fn native_adapter_preserves_content_priority_null_branches_and_requested_kind() {
    run(|| {
        let adapter = DefaultDockCommandBarAdapter::new();
        let direct = Button::new();
        let definition = bar(
            "direct",
            Kind::Menu,
            Mode::Replace,
            2,
            vec![item("ignored", 0, None)],
        );
        definition.set_content(Some(ferrodock_controls_recycling::to_recycling_value(
            Control::boxed(direct.clone()),
        )));
        let text = DockCommandBarDefinition::new("content".into(), Kind::Ribbon);
        text.set_order(1);
        text.set_content(Some(Rc::new("content".to_owned())));
        let none = DockCommandBarDefinition::new("none".into(), Kind::Menu);
        let bars = adapter.build_bars(Kind::ToolBar, &list(vec![definition, text, none]));
        assert_eq!(bars.count(), 2);
        assert_eq!(
            bars.get(0)
                .cast::<ContentControl>()
                .unwrap()
                .content()
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            "content"
        );
        assert_eq!(bars.get(1), direct.upcast::<Control>());
        let rendered = adapter.build_bars(
            Kind::ToolBar,
            &list(vec![bar(
                "menu-definition",
                Kind::Menu,
                Mode::Replace,
                0,
                vec![item("button", 0, None)],
            )]),
        );
        assert!(rendered.get(0).is::<StackPanel>());
    });
}
#[test]
fn menu_builds_nested_sorted_items_and_separators_and_runs_original_parameter() {
    run(|| {
        let parameter = Rc::new(Cell::new(1));
        let command = Rc::new(Command {
            parameter: parameter.clone(),
            calls: Cell::new(0),
        });
        let leaf = item("leaf", 0, None);
        leaf.set_command(Some(command.clone()));
        leaf.set_command_parameter(Some(parameter.clone() as ModelValue));
        leaf.set_icon(Some(Rc::new("icon".to_owned())));
        let parent = item("parent", 2, None);
        parent.set_items(Some(list(vec![leaf])));
        let separator = item("separator", 1, None);
        separator.set_is_separator(true);
        let bars = DefaultDockCommandBarAdapter::new().build_bars(
            Kind::Menu,
            &list(vec![bar(
                "menu",
                Kind::Menu,
                Mode::Replace,
                0,
                vec![parent, separator],
            )]),
        );
        let menu = bars.get(0).cast::<Menu>().unwrap();
        assert_eq!(menu.items().count(), 2);
        assert!(control(menu.items().get_at(0)).is::<Separator>());
        let parent = control(menu.items().get_at(1)).cast::<MenuItem>().unwrap();
        let leaf = control(parent.items().get_at(0))
            .cast::<MenuItem>()
            .unwrap();
        assert_eq!(
            leaf.header().unwrap().downcast_ref::<String>().unwrap(),
            "leaf"
        );
        assert_eq!(
            leaf.icon().unwrap().downcast_ref::<String>().unwrap(),
            "icon"
        );
        let command_native = leaf.command().unwrap();
        let native_parameter = leaf.command_parameter();
        assert!(command_native.can_execute(native_parameter.as_ref()));
        command_native.execute(native_parameter.as_ref());
        assert_eq!(command.calls.get(), 1);
        parameter.set(0);
        assert!(!command_native.can_execute(native_parameter.as_ref()));
    });
}
#[test]
fn toolbar_content_composition_and_ribbon_header_only_match_original() {
    run(|| {
        let both = item("header", 0, None);
        both.set_icon(Some(Rc::new("icon".to_owned())));
        let icon = DockCommandBarItem::new(None);
        icon.set_icon(Some(Rc::new("only icon".to_owned())));
        icon.set_order(1);
        let separator = item("separator", 2, None);
        separator.set_is_separator(true);
        let header = item("only header", 3, None);
        let definition = bar(
            "bar",
            Kind::ToolBar,
            Mode::Replace,
            0,
            vec![header, both, icon, separator],
        );
        let adapter = DefaultDockCommandBarAdapter::new();
        let toolbar = adapter
            .build_bars(Kind::ToolBar, &list(vec![definition.clone()]))
            .get(0)
            .cast::<StackPanel>()
            .unwrap();
        assert_eq!(toolbar.spacing(), 4.0);
        assert_eq!(toolbar.children().count(), 4);
        let button = toolbar.children().get(0).cast::<Button>().unwrap();
        let content = control(button.content()).cast::<StackPanel>().unwrap();
        assert_eq!(content.spacing(), 4.0);
        assert_eq!(
            content
                .children()
                .get(0)
                .cast::<ContentControl>()
                .unwrap()
                .content()
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            "icon"
        );
        assert_eq!(
            content
                .children()
                .get(1)
                .cast::<ContentControl>()
                .unwrap()
                .content()
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            "header"
        );
        assert_eq!(
            toolbar
                .children()
                .get(1)
                .cast::<Button>()
                .unwrap()
                .content()
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            "only icon"
        );
        assert!(toolbar.children().get(2).is::<Separator>());
        assert_eq!(
            toolbar
                .children()
                .get(3)
                .cast::<Button>()
                .unwrap()
                .content()
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            "only header"
        );
        let ribbon = adapter
            .build_bars(Kind::Ribbon, &list(vec![definition]))
            .get(0)
            .cast::<StackPanel>()
            .unwrap();
        assert_eq!(ribbon.spacing(), 6.0);
        assert_eq!(ribbon.children().count(), 4);
        assert_eq!(
            ribbon
                .children()
                .get(0)
                .cast::<ContentControl>()
                .unwrap()
                .content()
                .unwrap()
                .downcast_ref::<String>()
                .unwrap(),
            "header"
        );
        assert!(ribbon
            .children()
            .get(1)
            .cast::<ContentControl>()
            .unwrap()
            .content()
            .is_none());
        assert!(ribbon.children().get(2).is::<Separator>());
    });
}
