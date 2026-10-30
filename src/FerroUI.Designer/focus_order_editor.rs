//! Reusable native keyboard-navigation editor. Its draft owns no runtime objects.
use crate::focus_order::{FocusOrderCatalog, FocusOrderEdit, FocusOrderItem, navigation_mode_name};
use ferroui_base::input::{InputElement, Key, KeyboardNavigationMode};
use ferroui_base::layout::{HorizontalAlignment, Orientation};
use ferroui_base::media::{SolidColorBrush, TextWrapping};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{
    Button, CheckBox, ComboBox, Control, ListBox, ScrollViewer, StackPanel, TextBlock, ToolTip,
};
use ferroui_designer_core::NodeId;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

type ApplyHandler = Rc<dyn Fn(Vec<FocusOrderEdit>) -> Result<(), String>>;
type CloseHandler = Rc<dyn Fn()>;
const MODES: [KeyboardNavigationMode; 6] = [
    KeyboardNavigationMode::Continue,
    KeyboardNavigationMode::Local,
    KeyboardNavigationMode::Cycle,
    KeyboardNavigationMode::Contained,
    KeyboardNavigationMode::Once,
    KeyboardNavigationMode::None,
];

/// A staged editor for group-local order, tab eligibility, modes and Label.Target.
/// Every command is submitted together on Apply; Cancel discards the complete draft.
pub struct FocusOrderEditor {
    root: Ref<StackPanel>,
    scroll: Ref<ScrollViewer>,
    scope: Ref<ComboBox>,
    description: Ref<TextBlock>,
    entries: Ref<ListBox>,
    up: Ref<Button>,
    down: Ref<Button>,
    reset: Ref<Button>,
    tab_stop: Ref<CheckBox>,
    focusable: Ref<CheckBox>,
    mode: Ref<ComboBox>,
    label: Ref<ComboBox>,
    target: Ref<ComboBox>,
    apply: Ref<Button>,
    error: Ref<TextBlock>,
    catalog: RefCell<FocusOrderCatalog>,
    orders: RefCell<BTreeMap<u64, Vec<FocusOrderItem>>>,
    reset_orders: RefCell<BTreeSet<u64>>,
    tab_stops: RefCell<BTreeMap<NodeId, bool>>,
    focusables: RefCell<BTreeMap<NodeId, bool>>,
    modes: RefCell<BTreeMap<NodeId, KeyboardNavigationMode>>,
    targets: RefCell<BTreeMap<NodeId, Option<NodeId>>>,
    writable: Cell<bool>,
    active: Cell<bool>,
    guard: Cell<bool>,
    handler: RefCell<Option<ApplyHandler>>,
    close: RefCell<Option<CloseHandler>>,
}
impl FocusOrderEditor {
    pub fn new() -> Rc<Self> {
        let root = StackPanel::new();
        root.set_width(440.0);
        root.set_spacing(8.0);
        root.classes().add("designer-chrome");
        root.children()
            .add(text("Keyboard navigation", 16.0).upcast::<Control>());
        root.children().add(
            text(
                "Edit order within a native group. Changes are staged until Apply.",
                11.0,
            )
            .upcast::<Control>(),
        );
        let scope = choices("FocusOrderScope", "Navigation group");
        root.children().add(scope.clone().upcast::<Control>());
        let description = text("", 11.0);
        description.set_text_wrapping(TextWrapping::Wrap);
        root.children().add(description.clone().upcast::<Control>());
        let entries = ListBox::new();
        entries.set_name(Some("FocusOrderEntries".into()));
        entries.set_height(164.0);
        entries.set_horizontal_alignment(HorizontalAlignment::Stretch);
        AutomationProperties::set_name(&entries, Some("Controls in native tab-index order"));
        root.children().add(entries.clone().upcast::<Control>());
        let commands = StackPanel::new();
        commands.set_orientation(Orientation::Horizontal);
        commands.set_spacing(6.0);
        let up = action(
            "FocusOrderUp",
            "Move up",
            "Move the selected input earlier in its navigation group",
        );
        let down = action(
            "FocusOrderDown",
            "Move down",
            "Move the selected input later in its navigation group",
        );
        let reset = action(
            "FocusOrderReset",
            "Reset indices",
            "Remove authored TabIndex values in this group",
        );
        for button in [&up, &down, &reset] {
            commands.children().add(button.clone().upcast::<Control>());
        }
        root.children().add(commands.upcast::<Control>());
        let eligibility = StackPanel::new();
        eligibility.set_orientation(Orientation::Horizontal);
        eligibility.set_spacing(18.0);
        let tab_stop = check("FocusOrderTabStop", "Tab stop");
        let focusable = check("FocusOrderFocusable", "Focusable");
        eligibility
            .children()
            .add(tab_stop.clone().upcast::<Control>());
        eligibility
            .children()
            .add(focusable.clone().upcast::<Control>());
        root.children().add(eligibility.upcast::<Control>());
        root.children()
            .add(text("Group navigation", 11.0).upcast::<Control>());
        let mode = choices(
            "FocusOrderMode",
            "KeyboardNavigation.TabNavigation for the selected group",
        );
        for mode_value in MODES {
            mode.items()
                .add(Some(Rc::new(navigation_mode_name(mode_value).to_owned())));
        }
        root.children().add(mode.clone().upcast::<Control>());
        root.children()
            .add(text("Label focus target", 12.0).upcast::<Control>());
        let label = choices(
            "FocusOrderLabel",
            "Label whose access key or click transfers focus",
        );
        let target = choices(
            "FocusOrderTarget",
            "Input target in the same XAML namescope",
        );
        root.children().add(label.clone().upcast::<Control>());
        root.children().add(target.clone().upcast::<Control>());
        let error = text("", 11.0);
        error.set_text_wrapping(TextWrapping::Wrap);
        error.set_foreground(Some(SolidColorBrush::from_uint32(0xffd45d5d).into()));
        error.set_is_visible(false);
        root.children().add(error.clone().upcast::<Control>());
        let footer = StackPanel::new();
        footer.set_orientation(Orientation::Horizontal);
        footer.set_horizontal_alignment(HorizontalAlignment::Right);
        footer.set_spacing(8.0);
        footer.set_margin(Thickness::new(0.0, 4.0, 0.0, 0.0));
        let cancel = action(
            "FocusOrderCancel",
            "Cancel",
            "Discard all navigation changes",
        );
        let apply = action(
            "FocusOrderApply",
            "Apply",
            "Apply all navigation changes in one undo step",
        );
        apply.classes().add("accent");
        footer.children().add(cancel.clone().upcast::<Control>());
        footer.children().add(apply.clone().upcast::<Control>());
        root.children().add(footer.upcast::<Control>());
        let scroll = ScrollViewer::new();
        scroll.set_max_height(680.0);
        scroll.set_horizontal_scroll_bar_visibility(
            ferroui_controls::primitives::ScrollBarVisibility::Disabled,
        );
        scroll.set_content(Some(Control::boxed(&root)));
        let this = Rc::new(Self {
            root,
            scroll,
            scope,
            description,
            entries,
            up,
            down,
            reset,
            tab_stop,
            focusable,
            mode,
            label,
            target,
            apply,
            error,
            catalog: RefCell::new(FocusOrderCatalog {
                scopes: Vec::new(),
                labels: Vec::new(),
            }),
            orders: RefCell::new(BTreeMap::new()),
            reset_orders: RefCell::new(BTreeSet::new()),
            tab_stops: RefCell::new(BTreeMap::new()),
            focusables: RefCell::new(BTreeMap::new()),
            modes: RefCell::new(BTreeMap::new()),
            targets: RefCell::new(BTreeMap::new()),
            writable: Cell::new(false),
            active: Cell::new(false),
            guard: Cell::new(false),
            handler: RefCell::new(None),
            close: RefCell::new(None),
        });
        let weak = Rc::downgrade(&this);
        this.scope.selection_changed(move |_, _| {
            if let Some(this) = weak.upgrade().filter(|this| !this.guard.get()) {
                this.refresh_scope();
            }
        });
        let weak = Rc::downgrade(&this);
        this.entries.selection_changed(move |_, _| {
            if let Some(this) = weak.upgrade().filter(|this| !this.guard.get()) {
                this.refresh_selection();
            }
        });
        for (button, delta) in [(&this.up, -1), (&this.down, 1)] {
            let weak = Rc::downgrade(&this);
            button.click(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.move_selected(delta);
                }
            });
        }
        let weak = Rc::downgrade(&this);
        this.reset.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.reset_selected_scope();
            }
        });
        let weak = Rc::downgrade(&this);
        this.tab_stop.is_checked_changed(move |_, _| {
            if let Some(this) = weak
                .upgrade()
                .filter(|this| !this.guard.get() && this.writable.get())
            {
                if let Some(node) = this.selected_item().and_then(|item| item.node) {
                    this.tab_stops
                        .borrow_mut()
                        .insert(node, this.tab_stop.is_checked() == Some(true));
                }
            }
        });
        let weak = Rc::downgrade(&this);
        this.focusable.is_checked_changed(move |_, _| {
            if let Some(this) = weak
                .upgrade()
                .filter(|this| !this.guard.get() && this.writable.get())
            {
                if let Some(node) = this.selected_item().and_then(|item| item.node) {
                    this.focusables
                        .borrow_mut()
                        .insert(node, this.focusable.is_checked() == Some(true));
                }
            }
        });
        let weak = Rc::downgrade(&this);
        this.mode.selection_changed(move |_, _| {
            if let Some(this) = weak
                .upgrade()
                .filter(|this| !this.guard.get() && this.writable.get())
            {
                if let Some(node) = this.selected_scope().and_then(|scope| scope.node) {
                    if let Some(mode) = usize::try_from(this.mode.selected_index())
                        .ok()
                        .and_then(|index| MODES.get(index))
                    {
                        this.modes.borrow_mut().insert(node, *mode);
                    }
                }
            }
        });
        let weak = Rc::downgrade(&this);
        this.label.selection_changed(move |_, _| {
            if let Some(this) = weak.upgrade().filter(|this| !this.guard.get()) {
                this.refresh_label();
            }
        });
        let weak = Rc::downgrade(&this);
        this.target.selection_changed(move |_, _| {
            if let Some(this) = weak
                .upgrade()
                .filter(|this| !this.guard.get() && this.writable.get())
            {
                let catalog = this.catalog.borrow();
                let Some(label) = usize::try_from(this.label.selected_index())
                    .ok()
                    .and_then(|index| catalog.labels.get(index))
                else {
                    return;
                };
                match this.target.selected_index() {
                    0 => {
                        this.targets.borrow_mut().remove(&label.node);
                    }
                    1 => {
                        this.targets.borrow_mut().insert(label.node, None);
                    }
                    index if index >= 2 => {
                        if let Some(candidate) = label.candidates.get(index as usize - 2) {
                            this.targets
                                .borrow_mut()
                                .insert(label.node, Some(candidate.node));
                        }
                    }
                    _ => {}
                }
            }
        });
        let weak = Rc::downgrade(&this);
        this.apply.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.apply_draft();
            }
        });
        let weak = Rc::downgrade(&this);
        cancel.click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.close();
            }
        });
        let weak = Rc::downgrade(&this);
        this.root
            .add_handler(InputElement::key_down_event(), move |_, event| {
                if event.key == Key::Escape {
                    if let Some(this) = weak.upgrade() {
                        this.close();
                        event.set_handled(true);
                    }
                }
            });
        this
    }
    pub fn control(&self) -> Ref<Control> {
        self.scroll.clone().upcast()
    }
    pub fn on_apply(&self, handler: impl Fn(Vec<FocusOrderEdit>) -> Result<(), String> + 'static) {
        self.handler.replace(Some(Rc::new(handler)));
    }
    pub fn on_close(&self, handler: impl Fn() + 'static) {
        self.close.replace(Some(Rc::new(handler)));
    }
    pub fn configure(&self, catalog: FocusOrderCatalog, writable: bool) {
        self.guard.set(true);
        self.active.set(true);
        self.writable.set(writable);
        self.orders.borrow_mut().clear();
        self.reset_orders.borrow_mut().clear();
        self.tab_stops.borrow_mut().clear();
        self.focusables.borrow_mut().clear();
        self.modes.borrow_mut().clear();
        self.targets.borrow_mut().clear();
        self.scope.items().clear();
        for (index, scope) in catalog.scopes.iter().enumerate() {
            self.scope.items().add(Some(Rc::new(format!(
                "{}. {} · {}",
                index + 1,
                scope.label,
                navigation_mode_name(scope.mode)
            ))));
        }
        self.label.items().clear();
        for label in &catalog.labels {
            self.label.items().add(Some(Rc::new(label.label.clone())));
        }
        self.scope
            .set_selected_index(if catalog.scopes.is_empty() { -1 } else { 0 });
        self.label
            .set_selected_index(if catalog.labels.is_empty() { -1 } else { 0 });
        self.label.set_is_enabled(!catalog.labels.is_empty());
        self.catalog.replace(catalog);
        self.apply.set_is_enabled(writable);
        self.error.set_is_visible(false);
        self.guard.set(false);
        self.refresh_scope();
        self.refresh_label();
    }
    pub fn show_error(&self, message: &str) {
        self.error.set_text(Some(message));
        self.error.set_is_visible(true);
    }
    /// Retire callbacks when a host light-dismisses or replaces this editor.
    pub fn retire(&self) {
        self.active.set(false);
        self.apply.set_is_enabled(false);
    }
    pub fn draft(&self) -> Vec<FocusOrderEdit> {
        let catalog = self.catalog.borrow();
        let mut commands = Vec::new();
        for scope in &catalog.scopes {
            if self.reset_orders.borrow().contains(&scope.id) {
                commands.push(FocusOrderEdit::ResetOrder { scope: scope.id });
            } else if let Some(items) = self.orders.borrow().get(&scope.id) {
                if items
                    .iter()
                    .map(|item| item.node)
                    .ne(scope.items.iter().map(|item| item.node))
                {
                    commands.push(FocusOrderEdit::SetOrder {
                        scope: scope.id,
                        order: items.iter().filter_map(|item| item.node).collect(),
                    });
                }
            }
        }
        for (node, value) in self.tab_stops.borrow().iter() {
            if catalog
                .scopes
                .iter()
                .flat_map(|scope| &scope.items)
                .any(|item| item.node == Some(*node) && item.is_tab_stop != *value)
            {
                commands.push(FocusOrderEdit::SetTabStop {
                    node: *node,
                    value: *value,
                });
            }
        }
        for (node, value) in self.focusables.borrow().iter() {
            if catalog
                .scopes
                .iter()
                .flat_map(|scope| &scope.items)
                .any(|item| item.node == Some(*node) && item.focusable != *value)
            {
                commands.push(FocusOrderEdit::SetFocusable {
                    node: *node,
                    value: *value,
                });
            }
        }
        for (node, mode) in self.modes.borrow().iter() {
            if catalog
                .scopes
                .iter()
                .any(|scope| scope.node == Some(*node) && scope.mode != *mode)
            {
                commands.push(FocusOrderEdit::SetNavigation {
                    node: *node,
                    mode: *mode,
                });
            }
        }
        commands.extend(self.targets.borrow().iter().map(|(label, target)| {
            FocusOrderEdit::SetLabelTarget {
                label: *label,
                target: *target,
            }
        }));
        commands
    }
    fn selected_scope(&self) -> Option<crate::focus_order::FocusOrderScope> {
        usize::try_from(self.scope.selected_index())
            .ok()
            .and_then(|index| self.catalog.borrow().scopes.get(index).cloned())
    }
    fn scope_items(&self) -> Vec<FocusOrderItem> {
        self.selected_scope().map_or_else(Vec::new, |scope| {
            self.orders
                .borrow()
                .get(&scope.id)
                .cloned()
                .unwrap_or(scope.items)
        })
    }
    fn selected_item(&self) -> Option<FocusOrderItem> {
        usize::try_from(self.entries.selected_index())
            .ok()
            .and_then(|index| self.scope_items().get(index).cloned())
    }
    fn refresh_scope(&self) {
        self.guard.set(true);
        self.entries.items().clear();
        let scope = self.selected_scope();
        let items = self.scope_items();
        for (index, item) in items.iter().enumerate() {
            let state = if !item.visible {
                " · hidden"
            } else if !item.enabled {
                " · disabled"
            } else if !item.focusable && item.group.is_none() {
                " · not focusable"
            } else if !item.is_tab_stop && item.group.is_none() {
                " · excluded"
            } else {
                ""
            };
            let value = if item.tab_index == i32::MAX {
                "default".into()
            } else {
                item.tab_index.to_string()
            };
            self.entries.items().add(Some(Rc::new(format!(
                "{}. {} · index {value}{state}",
                index + 1,
                item.label
            ))));
        }
        self.entries
            .set_selected_index(if items.is_empty() { -1 } else { 0 });
        if let Some(scope) = scope {
            let extra = if scope.blocked_by_ancestor {
                " An ancestor excludes this group from Tab."
            } else if scope.items.iter().any(|item| item.node.is_none()) {
                " Generated entries are read-only; open the control definition to change their order."
            } else {
                " Equal indices follow the native child order."
            };
            let reset = if self.reset_orders.borrow().contains(&scope.id) {
                " Authored indices will be reset; inherited/default order is evaluated on Apply."
            } else {
                ""
            };
            self.description
                .set_text(Some(&format!("{}{extra}{reset}", scope.mode_description())));
            let mode = scope
                .node
                .and_then(|node| self.modes.borrow().get(&node).copied())
                .unwrap_or(scope.mode);
            self.mode.set_selected_index(
                MODES.iter().position(|value| *value == mode).unwrap_or(0) as i32,
            );
            self.mode
                .set_is_enabled(self.writable.get() && scope.node.is_some());
            self.reset.set_is_enabled(
                self.writable.get() && scope.items.iter().any(|item| item.node.is_some()),
            );
        } else {
            self.description
                .set_text(Some("No navigation groups are available in this preview."));
            self.mode.set_is_enabled(false);
            self.reset.set_is_enabled(false);
        }
        self.guard.set(false);
        self.refresh_selection();
    }
    fn refresh_selection(&self) {
        self.guard.set(true);
        let selected = self.selected_item();
        let editable =
            self.writable.get() && selected.as_ref().is_some_and(|item| item.node.is_some());
        self.tab_stop.set_is_enabled(editable);
        self.focusable.set_is_enabled(editable);
        self.tab_stop
            .set_is_checked(Some(selected.as_ref().is_some_and(|item| {
                item.node
                    .and_then(|node| self.tab_stops.borrow().get(&node).copied())
                    .unwrap_or(item.is_tab_stop)
            })));
        self.focusable
            .set_is_checked(Some(selected.as_ref().is_some_and(|item| {
                item.node
                    .and_then(|node| self.focusables.borrow().get(&node).copied())
                    .unwrap_or(item.focusable)
            })));
        let can_reorder = self.writable.get()
            && self
                .selected_scope()
                .is_some_and(|scope| scope.can_reorder());
        let index = self.entries.selected_index();
        self.up.set_is_enabled(can_reorder && index > 0);
        self.down.set_is_enabled(
            can_reorder && index >= 0 && (index as usize + 1) < self.scope_items().len(),
        );
        self.guard.set(false);
    }
    fn refresh_label(&self) {
        self.guard.set(true);
        self.target.items().clear();
        let catalog = self.catalog.borrow();
        let label = usize::try_from(self.label.selected_index())
            .ok()
            .and_then(|index| catalog.labels.get(index));
        if let Some(label) = label {
            self.target
                .items()
                .add(Some(Rc::new("Keep current target".to_owned())));
            self.target
                .items()
                .add(Some(Rc::new("Reset authored target".to_owned())));
            for candidate in &label.candidates {
                self.target
                    .items()
                    .add(Some(Rc::new(candidate.label.clone())));
            }
            let selection = match self.targets.borrow().get(&label.node) {
                Some(None) => 1,
                Some(Some(node)) => label
                    .candidates
                    .iter()
                    .position(|candidate| candidate.node == *node)
                    .map_or(0, |index| index as i32 + 2),
                None => 0,
            };
            self.target.set_selected_index(selection);
        }
        self.target
            .set_is_enabled(self.writable.get() && label.is_some());
        self.guard.set(false);
    }
    fn move_selected(&self, delta: i32) {
        if !self.writable.get() {
            return;
        }
        let Some(scope) = self.selected_scope().filter(|scope| scope.can_reorder()) else {
            return;
        };
        let selected = self.entries.selected_index();
        let next = selected + delta;
        let mut items = self.scope_items();
        if selected < 0
            || next < 0
            || selected as usize >= items.len()
            || next as usize >= items.len()
        {
            return;
        }
        items.swap(selected as usize, next as usize);
        self.orders.borrow_mut().insert(scope.id, items);
        self.reset_orders.borrow_mut().remove(&scope.id);
        self.refresh_scope();
        self.entries.set_selected_index(next);
    }
    fn reset_selected_scope(&self) {
        if !self.writable.get() {
            return;
        }
        let Some(scope) = self.selected_scope() else {
            return;
        };
        self.orders.borrow_mut().remove(&scope.id);
        self.reset_orders.borrow_mut().insert(scope.id);
        self.refresh_scope();
    }
    fn apply_draft(&self) {
        if !self.writable.get() || !self.active.get() {
            return;
        }
        let Some(handler) = self.handler.borrow().clone() else {
            return;
        };
        match handler(self.draft()) {
            Ok(()) => self.close(),
            Err(error) => self.show_error(&error),
        }
    }
    fn close(&self) {
        self.retire();
        if let Some(close) = self.close.borrow().clone() {
            close();
        }
    }
}

fn text(value: &str, size: f64) -> Ref<TextBlock> {
    let field = TextBlock::new();
    field.set_text(Some(value));
    field.set_font_size(size);
    field
}
fn choices(name: &str, help: &str) -> Ref<ComboBox> {
    let field = ComboBox::new();
    field.set_name(Some(name.to_owned()));
    field.classes().add("designer-property-field");
    field.set_horizontal_alignment(HorizontalAlignment::Stretch);
    field.set_min_height(28.0);
    AutomationProperties::set_name(&field, Some(help));
    ToolTip::set_tip(&field, Some(Rc::new(help.to_owned())));
    field
}
fn action(name: &str, label: &str, help: &str) -> Ref<Button> {
    let button = Button::new();
    button.set_name(Some(name.to_owned()));
    button.set_content(Some(Rc::new(label.to_owned())));
    button.set_min_height(28.0);
    button.set_padding(Thickness::symmetric(9.0, 4.0));
    AutomationProperties::set_name(&button, Some(help));
    ToolTip::set_tip(&button, Some(Rc::new(help.to_owned())));
    button
}
fn check(name: &str, label: &str) -> Ref<CheckBox> {
    let field = CheckBox::new();
    field.set_name(Some(name.to_owned()));
    field.set_content(Some(Rc::new(label.to_owned())));
    AutomationProperties::set_name(&field, Some(label));
    field
}

#[cfg(test)]
#[path = "focus_order_editor_tests.rs"]
mod tests;
