//! Native list used by RibbonCustomizeDialog.cs (ListRow / SelectableList).
//! Copyright Wiesław Šoltés, MIT. The source builds these row visuals in code.
//! Outline-level accessibility and physical screen-reader behavior remain unaudited.
use crate::{primitives::RibbonIconPresenter, theming::RibbonTheme};
use ferroribbon_core::RibbonString;
use ferroui_base::{
    BoxedValue, CornerRadius, Ref,
    data::core::ValueTypes,
    input::{InputElement, Key, KeyEventArgs, KeyModifiers, NavigationMethod},
    interactivity::RoutedEventHandlerToken,
    layout::{HorizontalAlignment, VerticalAlignment},
    media::{Colors, FontWeight, IBrush, SolidColorBrush},
    reactive::{Disposable, IDisposable},
    styling::ControlTheme,
    utilities::HandlerList,
};
use ferroui_controls::{
    Application, Border, Button, CheckBox, Control, ScrollViewer, StackPanel, TextBlock,
    automation::{
        AutomationProperties, SelectionItemPatternIdentifiers, peers::ControlAutomationPeer,
    },
    primitives::ScrollBarVisibility,
};
use std::{cell::RefCell, rc::Rc};

/// Shared row identity matters independently of its ID (IDs may be duplicates).
#[derive(Clone)]
pub(crate) struct RibbonCustomizationListRow {
    pub id: Option<RibbonString>,
    pub label: Option<String>,
    pub detail: Option<String>,
    pub icon: Option<BoxedValue>,
    pub level: i32,
    pub is_checked: Option<bool>,
}
type ToggleChanged = dyn Fn(Option<RibbonString>, bool);
struct RowControl {
    row: Rc<RibbonCustomizationListRow>,
    button: Ref<Button>,
}
/// A private product component: native controls and closures, no enumerable or
/// delegate facade. The editor/dialog owns this Rc; control handlers use Weak.
pub(crate) struct RibbonCustomizationList {
    root: Ref<Border>,
    panel: Ref<StackPanel>,
    items: RefCell<Vec<RowControl>>,
    selected: RefCell<Option<Rc<RibbonCustomizationListRow>>>,
    toggle_changed: HandlerList<ToggleChanged>,
    theme_subscription: RefCell<Option<ferroui_base::reactive::Subscription>>,
    key_subscription: RefCell<Option<RoutedEventHandlerToken>>,
}
impl RibbonCustomizationList {
    pub fn new() -> Rc<Self> {
        let panel = StackPanel::new();
        let scroll = ScrollViewer::new();
        scroll.set_content(Some(Control::boxed(panel.clone())));
        scroll.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        let root = Border::new();
        root.set_height(380.);
        root.set_border_thickness(ferroui_base::Thickness::uniform(1.));
        root.set_corner_radius(CornerRadius::uniform(4.));
        root.set_child(scroll);
        RibbonTheme::set_theme_brush(
            Some(&root),
            Some(Border::border_brush_property().as_property()),
            Some("RibbonInputBorderBrush"),
        )
        .expect("customization list border brush");
        let this = Rc::new(Self {
            root: root.clone(),
            panel,
            items: RefCell::default(),
            selected: RefCell::default(),
            toggle_changed: HandlerList::new(),
            theme_subscription: RefCell::default(),
            key_subscription: RefCell::default(),
        });
        let weak = Rc::downgrade(&this);
        this.theme_subscription
            .replace(Some(root.actual_theme_variant_changed(move || {
                if let Some(list) = weak.upgrade() {
                    list.update_selection_visuals();
                }
            })));
        let weak = Rc::downgrade(&this);
        this.key_subscription
            .replace(Some(root.key_down(move |_, e| {
                if let Some(list) = weak.upgrade() {
                    list.on_key_down(e);
                }
            })));
        this
    }
    pub fn root(&self) -> Ref<Border> {
        self.root.clone()
    }
    pub fn selected(&self) -> Option<Rc<RibbonCustomizationListRow>> {
        self.selected.borrow().clone()
    }
    pub fn set_name(&self, value: Option<&str>) {
        AutomationProperties::set_name(&self.panel, Some(value.unwrap_or("")));
    }
    pub fn toggle_changed(
        self: &Rc<Self>,
        handler: impl Fn(Option<RibbonString>, bool) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.toggle_changed.add(Rc::new(handler));
        let weak = Rc::downgrade(self);
        Disposable::create(move || {
            if let Some(list) = weak.upgrade() {
                list.toggle_changed.remove(token);
            }
        })
    }
    pub fn fill(self: &Rc<Self>, rows: impl IntoIterator<Item = Rc<RibbonCustomizationListRow>>) {
        // Source clears before enumerating the incoming rows, including selection.
        self.panel.children().clear();
        self.items.borrow_mut().clear();
        self.selected.replace(None);
        let rows: Vec<_> = rows.into_iter().collect();
        let count =
            i32::try_from(rows.len()).expect("customization row count exceeds source i32 limit");
        for (index, row) in rows.into_iter().enumerate() {
            let content = StackPanel::new();
            content.set_orientation(ferroui_base::layout::Orientation::Horizontal);
            content.set_spacing(6.);
            content.set_margin(ferroui_base::Thickness::new(
                f64::from(row.level.wrapping_mul(18)),
                0.,
                0.,
                0.,
            ));
            if let Some(checked) = row.is_checked {
                let check = CheckBox::new();
                check.set_is_checked(Some(checked));
                check.set_min_width(0.);
                check.set_padding(ferroui_base::Thickness::uniform(0.));
                AutomationProperties::set_name(&check, row.label.as_deref());
                let weak = Rc::downgrade(self);
                let id = row.id.clone();
                let weak_check = check.downgrade();
                check.is_checked_changed(move |_, _| {
                    if let (Some(list), Some(check)) = (weak.upgrade(), weak_check.upgrade())
                        && let Some(visible) = check.is_checked()
                    {
                        for (_, callback) in list.toggle_changed.snapshot().iter() {
                            callback(id.clone(), visible);
                        }
                    }
                });
                content.children().add(check);
            }
            if let Some(icon) = &row.icon {
                let presenter = RibbonIconPresenter::new();
                presenter.set_icon(Some(icon.clone()));
                presenter.set_icon_size(16.);
                RibbonTheme::set_theme_brush(
                    Some(&presenter),
                    Some(RibbonIconPresenter::foreground_property().as_property()),
                    Some("RibbonIconBrush"),
                )
                .expect("customization row icon brush");
                content.children().add(presenter);
            }
            let label = TextBlock::new();
            label.set_text(row.label.as_deref());
            label.set_vertical_alignment(VerticalAlignment::Center);
            label.set_font_weight(if row.level == 0 && row.is_checked.is_some() {
                FontWeight::SemiBold
            } else {
                FontWeight::Normal
            });
            content.children().add(label);
            if let Some(detail) = row.detail.as_deref().filter(|v| !v.is_empty()) {
                let label = TextBlock::new();
                label.set_text(Some(detail));
                label.set_opacity(0.6);
                label.set_font_size(11.);
                label.set_vertical_alignment(VerticalAlignment::Center);
                content.children().add(label);
            }
            let button = Button::new();
            button.set_content(Some(Control::boxed(content)));
            button.set_horizontal_alignment(HorizontalAlignment::Stretch);
            button.set_horizontal_content_alignment(HorizontalAlignment::Left);
            button.set_padding(ferroui_base::Thickness::new(6., 3., 6., 3.));
            if let Some(style) = Application::current()
                .expect("customization list application")
                .resources()
                .try_get_resource(&"RibbonMenuItemButtonStyle".into(), None)
            {
                button.set_theme(style.map(|v| {
                    ValueTypes::as_object(&*v)
                        .and_then(|o| o.cast::<ControlTheme>())
                        .expect("RibbonMenuItemButtonStyle must be ControlTheme")
                }));
            }
            AutomationProperties::set_name(&button, row.label.as_deref());
            AutomationProperties::set_position_in_set(&button, index as i32 + 1);
            AutomationProperties::set_size_of_set(&button, count);
            // Source SetLevel is outline depth, not heading level. Do not label these command
            // buttons as headings.
            if let Some(detail) = row.detail.as_deref().filter(|v| !v.is_empty()) {
                AutomationProperties::set_help_text(&button, Some(detail));
            }
            let weak = Rc::downgrade(self);
            let id = row.id.clone();
            button.click(move |_, _| {
                if let Some(list) = weak.upgrade() {
                    list.select(id.as_ref());
                }
            });
            self.items.borrow_mut().push(RowControl {
                row,
                button: button.clone(),
            });
            self.panel.children().add(button);
        }
    }
    pub fn select(&self, id: Option<&RibbonString>) {
        let selected = self
            .items
            .borrow()
            .iter()
            .find(|item| item.row.id.as_ref() == id)
            .map(|item| item.row.clone());
        self.selected.replace(selected);
        self.update_selection_visuals();
    }
    fn update_selection_visuals(&self) {
        let brush = RibbonTheme::get_brush(Some(&self.root), Some("RibbonAccentSubtleBrush"))
            .expect("customization selection brush");
        // Snapshot native handles before assigning properties, which can reenter.
        let items: Vec<_> = self
            .items
            .borrow()
            .iter()
            .map(|item| (item.row.clone(), item.button.clone()))
            .collect();
        for (row, button) in items {
            let selected = self
                .selected()
                .is_some_and(|selected| Rc::ptr_eq(&row, &selected));
            let background: Rc<dyn IBrush> = if selected && let Some(brush) = &brush {
                brush.clone().into()
            } else {
                SolidColorBrush::with_color(Colors::TRANSPARENT).into()
            };
            button.set_background(Some(background));
        }
    }
    fn on_key_down(&self, e: &KeyEventArgs) {
        if self.items.borrow().is_empty()
            || e.source()
                .is_some_and(|source| source.cast::<CheckBox>().is_some())
        {
            return;
        }
        let selected = self.selected();
        let index = selected.and_then(|selected| {
            self.items
                .borrow()
                .iter()
                .position(|item| Rc::ptr_eq(&item.row, &selected))
        });
        let count = self.items.borrow().len();
        let target = match e.key {
            Key::Down => Some(index.map_or(0, |i| (i + 1).min(count - 1))),
            Key::Up => Some(index.unwrap_or(0).saturating_sub(1)),
            Key::Home => Some(0),
            Key::End => Some(count - 1),
            _ => None,
        };
        let Some(target) = target else { return };
        e.set_handled(true);
        if Some(target) == index {
            return;
        }
        let (row, button) = {
            let items = self.items.borrow();
            (items[target].row.clone(), items[target].button.clone())
        };
        self.select(row.id.as_ref());
        button.focus_with(NavigationMethod::Directional, KeyModifiers::NONE);
        button.bring_into_view();
        // Use the pinned native peer event service only if a peer already exists,
        // as source FromElement does. OS announcement parity needs native checks.
        if let Some(peer) = ControlAutomationPeer::from_element(&button) {
            peer.raise_property_changed_event(
                SelectionItemPatternIdentifiers::is_selected_property(),
                Some(Rc::new(false)),
                Some(Rc::new(true)),
            );
        }
    }
}
impl Drop for RibbonCustomizationList {
    fn drop(&mut self) {
        if let Some(subscription) = self.theme_subscription.get_mut().take() {
            subscription.dispose();
        }
        if let Some(token) = self.key_subscription.get_mut().take() {
            self.root
                .remove_handler(InputElement::key_down_event(), token);
        }
    }
}
#[cfg(test)]
#[path = "ribbon_customization_list_tests.rs"]
mod tests;
