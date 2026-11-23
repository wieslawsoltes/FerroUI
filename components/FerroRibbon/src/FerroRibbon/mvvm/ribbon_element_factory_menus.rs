//! RibbonElementFactory.cs menu/flyout methods. Copyright Wiesław Šoltés, MIT.
//! Native host adapters use existing controls; unavailable picker factories fail explicitly.
use super::ribbon_element_factory::{editable, native_payload, source, text};
use super::{
    RibbonAction, RibbonElementFactory, RibbonModelBindings, model_values::box_command_parameter,
};
use crate::{compat::*, controls::*, primitives::RibbonMetrics, theming::RibbonTheme};
use ferroribbon_core::{
    RibbonItemSize, RibbonString,
    collections::{RibbonReadOnlyList, RibbonReadOnlyListImpl},
    model::*,
};
use ferroui_base::{
    BoxedValue, Ref, Thickness, data::core::ValueTypes, layout::HorizontalAlignment,
    media::FontWeight,
};
use ferroui_controls::{
    Border, ContentControl, Control, Flyout, MenuFlyout, Panel, PlacementMode, StackPanel,
    TextBlock, ToolTip,
    automation::AutomationProperties,
    presenters::ContentPresenter,
    primitives::{FlyoutBase, ToggleButton},
    shapes::{Rectangle, Shape},
};
use ferroribbon_core::collections::ObservableCollection;
use std::{cell::RefCell, rc::Rc};

type Nodes = ObservableCollection<RibbonNodeReference>;
type AssignFlyout = Rc<dyn Fn(Option<Ref<FlyoutBase>>)>;
// A live adapter, never a collection snapshot. No native metadata/object-identity
// claim is made for the adapter itself; elements retain their actual allocations.
struct Entries(Nodes);
impl RibbonReadOnlyListImpl<RibbonNodeReference> for Entries {
    fn count(&self) -> i32 {
        i32::try_from(self.0.count()).expect("Int32 Count")
    }
    fn get(&self, index: i32) -> RibbonNodeReference {
        self.0
            .get(usize::try_from(index).expect("ArgumentOutOfRange: index"))
    }
    fn iter_owned(self: Rc<Self>) -> RibbonIterator<RibbonNodeReference> {
        Box::new(self.0.iter())
    }
}
fn entries(items: Nodes) -> RibbonReadOnlyList<RibbonNodeReference> {
    Rc::new(Entries(items)) as RibbonReadOnlyList<_>
}
fn menu(entry: RibbonNodeReference) -> Option<Rc<dyn RibbonMenuItemModelImpl>> {
    try_cast_node(entry).expect("declared model hierarchy")
}
#[derive(Clone)]
enum SignaturePart {
    Content(Option<RibbonObject>),
    Node(RibbonNodeReference),
    Check(bool),
    Group(Option<RibbonString>),
    Count(usize),
}
impl PartialEq for SignaturePart {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Content(a), Self::Content(b)) => a == b,
            (Self::Node(Some(a)), Self::Node(Some(b))) => a.equals_node(b.as_ref()),
            (Self::Node(None), Self::Node(None)) => true,
            (Self::Check(a), Self::Check(b)) => a == b,
            (Self::Group(a), Self::Group(b)) => a == b,
            (Self::Count(a), Self::Count(b)) => a == b,
            _ => false,
        }
    }
}
fn append_signature(out: &mut Vec<SignaturePart>, items: Nodes) {
    for entry in items.iter() {
        out.push(SignaturePart::Node(entry.clone()));
        if let Some(item) = menu(entry) {
            out.push(SignaturePart::Check(item.is_checkable()));
            out.push(SignaturePart::Group(item.group_name()));
            out.push(SignaturePart::Count(item.items().count()));
            append_signature(out, item.items());
        }
    }
}
fn watch_structure(items: Nodes, scope: &Rc<RibbonModelBindings>, rebuild: RibbonAction) {
    for item in items.iter().filter_map(menu) {
        scope.watch(
            Some(item.clone()),
            Some(rebuild.clone()),
            Some(vec![Some("IsCheckable".into()), Some("GroupName".into())]),
        );
        scope.watch_collection(Some(Rc::new(item.items())), Some(rebuild.clone()));
        watch_structure(item.items(), scope, rebuild.clone());
    }
}
impl RibbonElementFactory {
    /// Source protected nonvirtual BindFlyout. Child callbacks hold a weak action:
    /// the binding owner owns rebuilding, without model -> callback -> scope cycles.
    pub fn bind_flyout(
        &self,
        model: Rc<dyn RibbonDropDownButtonModelImpl>,
        bindings: Rc<RibbonModelBindings>,
        assign: AssignFlyout,
    ) {
        #[derive(Default)]
        struct State {
            scope: Option<Rc<RibbonModelBindings>>,
            signature: Option<Vec<SignaturePart>>,
        }
        let state = Rc::new(RefCell::new(State::default()));
        let parent = Rc::downgrade(&bindings);
        let factory = self.to_ref();
        let m = model.clone();
        let callback = Rc::new(RefCell::new(None::<std::rc::Weak<dyn Fn()>>));
        let weak = callback.clone();
        let rebuild: RibbonAction = Rc::new(move || {
            let mut next = vec![SignaturePart::Content(m.drop_down_content())];
            append_signature(&mut next, m.menu_items());
            let old = state.borrow().signature.clone();
            if old.as_ref().is_some_and(|old| next == *old) {
                return;
            }
            state.borrow_mut().signature = Some(next);
            let old_scope = state.borrow().scope.clone();
            if let Some(scope) = old_scope {
                scope.dispose();
            }
            let Some(parent) = parent.upgrade() else {
                return;
            };
            let child = parent.child();
            state.borrow_mut().scope = Some(child.clone());
            assign(factory.create_flyout(
                Some(entries(m.menu_items())),
                m.drop_down_content(),
                Some(child),
            ));
            let current = state.borrow().scope.clone().expect("assigned child scope");
            let action = weak.clone();
            watch_structure(
                m.menu_items(),
                &current,
                Rc::new(move || {
                    let action = action.borrow().as_ref().and_then(std::rc::Weak::upgrade);
                    if let Some(action) = action {
                        action();
                    }
                }),
            );
        });
        callback.replace(Some(Rc::downgrade(&rebuild)));
        bindings.watch_collection(Some(Rc::new(model.menu_items())), Some(rebuild.clone()));
        bindings.one_way(
            Some(model),
            Some(rebuild),
            Some(vec![Some("DropDownContent".into())]),
        );
    }
    pub(super) fn build_flyout(
        &self,
        items: Option<RibbonReadOnlyList<RibbonNodeReference>>,
        content: Option<RibbonObject>,
        bindings: Option<Rc<RibbonModelBindings>>,
    ) -> Option<Ref<FlyoutBase>> {
        let items = items.expect("ArgumentNullException: entries");
        let bindings = bindings.expect("ArgumentNullException: bindings");
        if items.count() == 0 && content.is_none() {
            return None;
        }
        let simple = content.is_none()
            && items.iter().all(|e| {
                e.is_some_and(|e| {
                    e.as_ref().type_id() == std::any::TypeId::of::<RibbonMenuSeparatorModel>()
                        || e.as_ref().type_id() == std::any::TypeId::of::<RibbonMenuHeaderModel>()
                        || menu(Some(e)).is_some()
                })
            });
        if simple {
            let flyout = MenuFlyout::new();
            flyout.set_placement(PlacementMode::BottomEdgeAlignedLeft);
            let list = items.clone();
            for item in self
                .create_menu_items(
                    Some(RibbonItemsSource::new(move || list.iter())),
                    Some(bindings),
                )
                .iter()
            {
                flyout.items().add(Some(Control::boxed(item)));
            }
            return Some(flyout.upcast());
        }
        let panel = StackPanel::new();
        panel.set_spacing(1.);
        panel.set_min_width(180.);
        let flyout = Flyout::new();
        flyout.set_content(Some(Control::boxed(panel.clone())));
        flyout.set_placement(PlacementMode::BottomEdgeAlignedLeft);
        Self::apply_flyout_style(&flyout);
        if let Some(content) = content {
            let native = native_payload(content);
            if let Some(element) = ValueTypes::as_object(&*native).and_then(|o| o.cast::<Control>())
            {
                Self::detach_from_parent(element.clone());
                panel.children().add(element.clone());
                let p = panel.clone();
                bindings.add(Some(Rc::new(move || {
                    if p.children().contains(&element) {
                        p.children().remove(element.clone());
                    }
                })));
            } else {
                let presenter = ContentPresenter::new();
                presenter.set_content(Some(native.clone()));
                presenter.set_content_template(
                    self.content_template_selector()
                        .and_then(|s| s.select_template(Some(native))),
                );
                panel.children().add(presenter);
            }
        }
        for entry in items.iter() {
            let Some(node) = entry.clone() else { continue };
            if node.as_ref().type_id() == std::any::TypeId::of::<RibbonMenuSeparatorModel>() {
                panel.children().add(Self::menu_separator(node, &bindings));
            } else if node.as_ref().type_id() == std::any::TypeId::of::<RibbonMenuHeaderModel>() {
                let title = TextBlock::new();
                title.set_font_weight(FontWeight::SemiBold);
                title.set_margin(Thickness::new(8., 4., 8., 2.));
                let e = title.clone();
                bindings.one_way(
                    Some(node.clone()),
                    Some(Rc::new(move || {
                        e.set_text(Some(text(node.label()).as_deref().unwrap_or("")));
                        e.set_is_visible(node.is_visible());
                    })),
                    Some(vec![Some("Label".into()), Some("IsVisible".into())]),
                );
                panel.children().add(title);
            } else if let Some(m) = menu(entry.clone()) {
                if let Some(element) = self.create_menu_element(entry, Some(bindings.clone())) {
                    if m.items().count() == 0
                        && let Some(button) = element.cast::<ferroui_controls::Button>()
                    {
                        let weak = flyout.downgrade();
                        button.click(move |_, _| {
                            if let Some(f) = weak.upgrade() {
                                f.hide();
                            }
                        });
                    }
                    panel.children().add(element);
                }
            } else if let Some(item) = try_cast_node::<dyn RibbonItemModelImpl>(Some(node))
                .expect("declared model hierarchy")
            {
                if let Some(grid) =
                    try_cast_node::<dyn RibbonGridPickerModelImpl>(Some(item.clone()))
                        .expect("model hierarchy")
                {
                    let picker = RibbonGridPicker::new();
                    let p = picker.clone();
                    let m = grid.clone();
                    bindings.one_way(
                        Some(grid.clone()),
                        Some(Rc::new(move || {
                            p.set_rows(m.rows());
                            p.set_columns(m.columns());
                            p.set_is_visible(m.is_visible());
                            p.set_is_enabled(m.is_enabled());
                        })),
                        Some(
                            ["Rows", "Columns", "IsVisible", "IsEnabled"]
                                .into_iter()
                                .map(|s| Some(s.into()))
                                .collect(),
                        ),
                    );
                    let p = picker.clone();
                    let m = grid.clone();
                    let factory = self.to_ref();
                    self.bind_commands(
                        &bindings,
                        grid,
                        Rc::new(move || {
                            p.set_command(
                                m.command()
                                    .or_else(|| factory.resolve_command(m.command_id()))
                                    .map(crate::commands::adapt_core_command),
                            );
                        }),
                        vec![Some("Command".into()), Some("CommandId".into())],
                    );
                    let weak = flyout.downgrade();
                    let handler = ferroui_base::metadata::MarkupDelegate::new(move |_| {
                        if let Some(flyout) = weak.upgrade() {
                            flyout.hide();
                        }
                        None
                    });
                    picker.add_size_picked(Some(handler.clone()));
                    // Source keeps this local hide handler after binding disposal.
                    // A weak flyout capture preserves that behavior without a cycle.
                    panel.children().add(picker);
                    continue;
                }
                if let Some(colors) =
                    try_cast_node::<dyn RibbonColorPickerModelImpl>(Some(item.clone()))
                        .expect("model hierarchy")
                {
                    panel
                        .children()
                        .add(self.create_flyout_palette(colors, &flyout, &bindings));
                    continue;
                }
                if let Some(element) = self.create_item(Some(item), bindings.clone()) {
                    self.menu_layout(&element);
                    panel.children().add(element);
                }
            }
        }
        Some(flyout.upcast())
    }
    pub(super) fn build_menu_items(
        &self,
        items: Option<RibbonItemsSource<RibbonNodeReference>>,
        bindings: Option<Rc<RibbonModelBindings>>,
    ) -> RibbonItemsSource<Ref<Control>> {
        // C# iterator argument validation and source acquisition occur on MoveNext.
        let f = self.to_ref();
        RibbonItemsSource::deferred(move || {
            let items = items.clone().expect("ArgumentNullException: entries");
            let b = bindings.clone().expect("ArgumentNullException: bindings");
            let f = f.clone();
            Box::new(items.iter().filter_map(move |entry| {
                let node = entry.clone()?;
                if node.as_ref().type_id() == std::any::TypeId::of::<RibbonMenuSeparatorModel>() {
                    let line = MenuFlyoutSeparator::new();
                    let e = line.clone();
                    b.one_way(
                        Some(node.clone()),
                        Some(Rc::new(move || e.set_is_visible(node.is_visible()))),
                        Some(vec![Some("IsVisible".into())]),
                    );
                    return Some(line.upcast());
                }
                if node.as_ref().type_id() == std::any::TypeId::of::<RibbonMenuHeaderModel>() {
                    let title = MenuFlyoutItem::new();
                    title.set_is_enabled(false);
                    title.set_font_weight(FontWeight::SemiBold);
                    let e = title.clone();
                    b.one_way(
                        Some(node.clone()),
                        Some(Rc::new(move || {
                            e.set_text(Some(text(node.label()).unwrap_or_default()));
                            e.set_is_visible(node.is_visible());
                        })),
                        Some(vec![Some("Label".into()), Some("IsVisible".into())]),
                    );
                    return Some(title.upcast());
                }
                let item = menu(entry)?;
                if item.items().count() > 0 {
                    let sub = MenuFlyoutSubItem::new();
                    Self::set_model(&sub, item.clone());
                    let e = sub.clone();
                    let m = item.clone();
                    b.one_way(
                        Some(item.clone()),
                        Some(Rc::new(move || {
                            e.set_text(Some(text(m.label()).unwrap_or_default()));
                            e.set_icon(Self::menu_icon(&m));
                            e.set_is_enabled(m.is_enabled());
                            e.set_is_visible(m.is_visible());
                            Self::menu_decorations(&e, m.as_ref());
                        })),
                        Some(vec![]),
                    );
                    let e = sub.clone();
                    let m = item.clone();
                    Self::watch_screen_tip(
                        &b,
                        item.clone(),
                        Rc::new(move || Self::menu_decorations(&e, m.as_ref())),
                    );
                    for child in f
                        .create_menu_items(
                            Some(RibbonItemsSource::from_collection(item.items())),
                            Some(b.clone()),
                        )
                        .iter()
                    {
                        sub.items().add(Some(child));
                    }
                    Some(sub.upcast())
                } else {
                    Some(f.create_menu_item(Some(item), Some(b.clone())).upcast())
                }
            }))
        })
    }
    fn menu_icon(item: &Rc<dyn RibbonMenuItemModelImpl>) -> Option<BoxedValue> {
        RibbonItemHelper::create_menu_icon(item.icon().map(|i| Rc::new(i) as BoxedValue))
            .expect("factory menu icon")
            .map(Control::boxed)
    }
    fn menu_decorations(element: &Control, item: &dyn RibbonMenuItemModelImpl) {
        RibbonKeyTip::set_key_tip(element, text(item.key_tip()));
        let tip = if item.screen_tip().is_some() {
            RibbonScreenTipService::create(
                text(item.label()),
                item.screen_tip().map(|tip| Rc::new(tip) as BoxedValue),
                text(item.shortcut()),
            )
            .expect("factory menu ScreenTip")
        } else {
            None
        };
        ToolTip::set_tip(element, tip);
        let help = item
            .screen_tip()
            .and_then(|s| s.description())
            .or_else(|| item.description());
        if help.as_ref().is_none_or(|s| s.is_empty()) {
            element.clear_value(AutomationProperties::help_text_property());
        } else {
            AutomationProperties::set_help_text(element, text(help).as_deref());
        }
    }
    pub(super) fn build_menu_item(
        &self,
        item: Option<Rc<dyn RibbonMenuItemModelImpl>>,
        bindings: Option<Rc<RibbonModelBindings>>,
    ) -> Ref<MenuFlyoutItem> {
        let item = item.expect("ArgumentNullException: item");
        let b = bindings.expect("ArgumentNullException: bindings");
        let element: Ref<MenuFlyoutItem> = if item.is_checkable() {
            if item.group_name().is_some() {
                let radio = RadioMenuFlyoutItem::new();
                radio.set_group_name(text(item.group_name()));
                radio.upcast()
            } else {
                ToggleMenuFlyoutItem::new().upcast()
            }
        } else {
            MenuFlyoutItem::new()
        };
        Self::set_model(&element, item.clone());
        let m = item.clone();
        let e = element.clone();
        b.one_way(
            Some(item.clone()),
            Some(Rc::new(move || {
                e.set_text(Some(text(m.label()).unwrap_or_default()));
                e.set_icon(Self::menu_icon(&m));
                e.set_is_enabled(m.is_enabled());
                e.set_is_visible(m.is_visible());
                e.set_keyboard_accelerator_text_override(Some(
                    text(m.shortcut()).unwrap_or_default(),
                ));
                Self::menu_decorations(&e, m.as_ref());
                // Source tests Toggle before Radio (the Uno radio derives from Toggle).
                if let Some(toggle) = e.cast::<ToggleMenuFlyoutItem>() {
                    toggle.set_is_checked(m.is_checked());
                } else if let Some(radio) = e.cast::<RadioMenuFlyoutItem>() {
                    radio.set_is_checked(m.is_checked());
                }
            })),
            Some(vec![]),
        );
        let m = item.clone();
        let e = element.clone();
        Self::watch_screen_tip(
            &b,
            item.clone(),
            Rc::new(move || Self::menu_decorations(&e, m.as_ref())),
        );
        let f = self.to_ref();
        let e = element.clone();
        let m = item.clone();
        self.bind_commands(
            &b,
            item.clone(),
            Rc::new(move || {
                e.set_command(
                    m.command()
                        .or_else(|| f.resolve_command(m.command_id()))
                        .map(crate::commands::adapt_core_command),
                );
                e.set_command_parameter(m.command_parameter().map(box_command_parameter));
            }),
            vec![
                Some("Command".into()),
                Some("CommandId".into()),
                Some("CommandParameter".into()),
            ],
        );
        let m = item.clone();
        let f = self.to_ref();
        element.add_click_action(move |e, _| {
            if let Some(toggle) = e.downcast_ref::<ToggleMenuFlyoutItem>() {
                m.set_is_checked(toggle.is_checked());
            } else if let Some(radio) = e.downcast_ref::<RadioMenuFlyoutItem>() {
                m.set_is_checked(radio.is_checked());
            }
            if let Some(r) = f.ribbon() {
                r.on_item_invoked(
                    e.to_ref().upcast(),
                    m.command_id(),
                    m.command_parameter().map(box_command_parameter),
                );
            }
        });
        let weak = element.downgrade();
        let f = self.to_ref();
        RibbonMenu::set_invoke(
            &element,
            Some(crate::controls::action_delegate(move || {
                let command = item
                    .command()
                    .or_else(|| f.resolve_command(item.command_id()));
                let parameter = item.command_parameter().map(|p| p.payload());
                Self::execute(command.as_deref(), parameter.as_deref());
                if item.is_checkable() {
                    item.set_is_checked(item.group_name().is_some() || !item.is_checked());
                }
                if let Some(r) = f.ribbon()
                    && let Some(e) = weak.upgrade()
                {
                    r.on_item_invoked(
                        e.upcast(),
                        item.command_id(),
                        item.command_parameter().map(box_command_parameter),
                    );
                }
            })),
        );
        element
    }
    fn menu_layout(&self, element: &Control) {
        if let Some(item) = ValueTypes::try_cast(
            &Control::boxed(element.to_ref()),
            ferroui_base::data::core::ValueType::of::<Rc<dyn IRibbonItem>>(),
        )
        .and_then(|v| v.downcast_ref::<Rc<dyn IRibbonItem>>().cloned())
        {
            let mut layout = RibbonItemLayout::new(
                RibbonItemSize::Medium,
                Some(
                    self.ribbon()
                        .map(|r| r.metrics())
                        .unwrap_or_else(RibbonMetrics::comfortable),
                ),
            );
            layout.show_label = Some(true);
            item.apply_layout(layout);
        }
    }
    pub(super) fn build_menu_element(
        &self,
        entry: RibbonNodeReference,
        bindings: Option<Rc<RibbonModelBindings>>,
    ) -> Option<Ref<Control>> {
        let b = bindings.expect("ArgumentNullException: bindings");
        let Some(item) = menu(entry.clone()) else {
            return entry
                .filter(|e| {
                    e.as_ref().type_id() == std::any::TypeId::of::<RibbonMenuSeparatorModel>()
                })
                .map(|node| Self::menu_separator(node, &b));
        };
        let weak = Rc::downgrade(&b);
        let recreate: RibbonAction = Rc::new(move || {
            if let Some(b) = weak.upgrade() {
                b.request_recreate();
            }
        });
        b.watch(
            Some(item.clone()),
            Some(recreate.clone()),
            Some(vec![Some("IsCheckable".into()), Some("GroupName".into())]),
        );
        b.watch_collection(Some(Rc::new(item.items())), Some(recreate.clone()));
        watch_structure(item.items(), &b, recreate);
        let element: Ref<Control> = if item.is_checkable() {
            let toggle = RibbonToggleButton::new();
            toggle.set_horizontal_alignment(HorizontalAlignment::Stretch);
            toggle.set_can_add_to_quick_access(false);
            Self::set_model(&toggle, item.clone());
            self.bind_menu_element(toggle.clone().upcast(), item.clone(), b.clone());
            let m = item.clone();
            let e = toggle.clone();
            b.one_way(
                Some(item.clone()),
                Some(Rc::new(move || e.set_group_name(text(m.group_name())))),
                Some(vec![Some("GroupName".into())]),
            );
            let m = item.clone();
            let e = toggle.clone();
            let mm = item.clone();
            let ee = toggle.clone();
            b.two_way(
                Some(item),
                Some("IsChecked".into()),
                Some(Rc::new(move || e.set_is_checked(Some(m.is_checked())))),
                Some(toggle.clone().upcast()),
                Some(ToggleButton::is_checked_property().as_property()),
                Some(Rc::new(move || {
                    mm.set_is_checked(ee.is_checked() == Some(true))
                })),
            );
            toggle.upcast()
        } else if item.items().count() > 0 {
            let button = RibbonDropDownButton::new();
            button.set_horizontal_alignment(HorizontalAlignment::Stretch);
            button.set_can_add_to_quick_access(false);
            Self::set_model(&button, item.clone());
            self.bind_menu_element(button.clone().upcast(), item.clone(), b.clone());
            let flyout = MenuFlyout::new();
            flyout.set_placement(PlacementMode::RightEdgeAlignedTop);
            for child in self
                .create_menu_items(
                    Some(RibbonItemsSource::from_collection(item.items())),
                    Some(b),
                )
                .iter()
            {
                flyout.items().add(Some(Control::boxed(child)));
            }
            button.set_flyout(Some(flyout.upcast()));
            button.upcast()
        } else {
            return self.create_menu_button(entry, Some(b));
        };
        self.menu_layout(&element);
        Some(element)
    }
    pub(super) fn build_menu_button(
        &self,
        entry: RibbonNodeReference,
        bindings: Option<Rc<RibbonModelBindings>>,
    ) -> Option<Ref<Control>> {
        let item = menu(entry)?;
        let b = bindings.expect("ArgumentNullException: bindings");
        let button = RibbonButton::new();
        button.set_horizontal_alignment(HorizontalAlignment::Stretch);
        button.set_can_add_to_quick_access(false);
        Self::set_model(&button, item.clone());
        self.bind_menu_element(button.clone().upcast(), item, b);
        self.menu_layout(&button);
        Some(button.upcast())
    }
    fn menu_separator(
        node: Rc<dyn RibbonNodeModelImpl>,
        bindings: &Rc<RibbonModelBindings>,
    ) -> Ref<Control> {
        let line = Rectangle::new();
        line.set_height(1.);
        line.set_margin(Thickness::uniform(4.));
        RibbonTheme::set_theme_brush(
            Some(&line),
            Some(Shape::fill_property().as_property()),
            Some("RibbonSeparatorBrush"),
        )
        .expect("menu separator brush");
        let e = line.clone();
        bindings.one_way(
            Some(node.clone()),
            Some(Rc::new(move || e.set_is_visible(node.is_visible()))),
            Some(vec![Some("IsVisible".into())]),
        );
        line.upcast()
    }
    fn bind_menu_element(
        &self,
        element: Ref<Control>,
        item: Rc<dyn RibbonMenuItemModelImpl>,
        bindings: Rc<RibbonModelBindings>,
    ) {
        let e = element.clone();
        let m = item.clone();
        bindings.one_way(
            Some(item.clone()),
            Some(Rc::new(move || {
                if let Some(edit) = editable(&e) {
                    edit.set_id(m.id());
                    edit.set_label(m.label());
                    edit.set_icon(m.icon().map(|v| Rc::new(v) as BoxedValue));
                    edit.set_large_icon(m.large_icon().map(|v| Rc::new(v) as BoxedValue));
                    edit.set_shortcut(m.shortcut());
                    edit.set_key_tip(m.key_tip());
                    edit.set_screen_tip(
                        m.screen_tip()
                            .map(|v| Rc::new(v) as BoxedValue)
                            .or_else(|| text(m.description()).map(|v| Rc::new(v) as BoxedValue)),
                    );
                    edit.set_can_add_to_quick_access(false);
                }
                e.set_is_visible(m.is_visible());
                e.set_is_enabled(m.is_enabled());
            })),
            Some(vec![]),
        );
        if source(&element).is_some() && item.items().count() == 0 {
            let f = self.to_ref();
            let m = item.clone();
            let e = element.clone();
            self.bind_commands(
                &bindings,
                item.clone(),
                Rc::new(move || {
                    let source = source(&e).expect("native command source");
                    source.set_command(
                        m.command()
                            .or_else(|| f.resolve_command(m.command_id()))
                            .map(crate::commands::adapt_core_command),
                    );
                    source.set_command_parameter(m.command_parameter().map(box_command_parameter));
                }),
                vec![
                    Some("Command".into()),
                    Some("CommandId".into()),
                    Some("CommandParameter".into()),
                ],
            );
        }
        Self::watch_screen_tip(
            &bindings,
            item,
            Rc::new(move || {
                RibbonItemHelper::update_tool_tip(element.clone()).expect("factory menu tooltip")
            }),
        );
    }
    /// Source DetachFromParent removes only from the four supported parent kinds.
    pub fn detach_from_parent(element: Ref<Control>) {
        let parent = if let Some(visual) = element.visual_parent() {
            visual.cast::<Control>()
        } else {
            element
                .get_logical_parent()
                .and_then(|p| p.cast::<Control>())
        };
        let Some(parent) = parent else { return };
        // Uno's untemplated base ContentControl directly owns its UI content.
        // FerroUI supplies a default PART_ContentPresenter instead. Map that
        // framework-only wrapper back to its actual content owner; an explicit
        // downstream template keeps the original visual-parent behavior.
        if let Some(presenter) = parent.cast::<ContentPresenter>()
            && presenter.name().as_deref() == Some("PART_ContentPresenter")
            && let Some(owner) = presenter
                .templated_parent()
                .and_then(|p| p.cast::<ContentControl>())
            && owner.get_type() == ContentControl::TYPE
            && !owner.is_set(
                ferroui_controls::primitives::TemplatedControl::template_property().as_property(),
            )
            && owner
                .content()
                .as_ref()
                .and_then(Control::from_boxed)
                .is_some_and(|content| content.ptr_eq(&element))
        {
            owner.set_content(None);
            return;
        }
        if let Some(panel) = parent.cast::<Panel>() {
            panel.children().remove(element);
        } else if let Some(border) = parent.cast::<Border>() {
            if border.child().as_ref() == Some(&element) {
                border.set_child(None);
            }
        } else if let Some(presenter) = parent.cast::<ContentPresenter>() {
            if presenter
                .content()
                .as_ref()
                .and_then(Control::from_boxed)
                .as_ref()
                == Some(&element)
            {
                presenter.set_content(None);
            }
        } else if let Some(control) = parent.cast::<ContentControl>()
            && control
                .content()
                .as_ref()
                .and_then(Control::from_boxed)
                .as_ref()
                == Some(&element)
        {
            control.set_content(None);
        }
    }
}
