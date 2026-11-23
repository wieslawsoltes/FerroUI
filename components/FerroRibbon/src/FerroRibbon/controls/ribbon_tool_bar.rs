//! Shared RibbonToolBar.cs, RibbonSpace 9755e281. Copyright Wiesław Šoltés, MIT.
use super::ribbon_quick_access_tool_bar::as_item;
use super::{
    IRibbonItemOwner, IRibbonLayoutHost, Ribbon, RibbonButton, RibbonGroup, RibbonInputBase,
    RibbonItemHelper, RibbonItemInvokedEventArgs, RibbonItemLayout, RibbonSeparator,
    RibbonToolBarAutomationPeer,
};
use crate::{
    compat::Orientation,
    metadata::control_items::RibbonControlItems,
    primitives::{RibbonMetrics, RibbonToolBarOverflowItems, RibbonToolBarPanel},
    theming::RibbonTheme,
};
use ferroribbon_core::{
    RibbonDensity, RibbonItemSize, RibbonString, commands::RibbonCommandCatalog,
    localization::RibbonStrings,
};
use ferroui_base::{
    AttachedProperty, BoxedValue, ElementRef, FerroObject, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, Nullable, Ref, StyledElementImpl, StyledProperty, Thickness, VisualImpl,
    data::core::ValueTypes,
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{
        FocusManager, InputElementImpl, Key, KeyEventArgs, KeyModifiers, KeyboardNavigation,
        NavigationMethod,
    },
    instantiate,
    interactivity::InteractiveImpl,
    layout::{HorizontalAlignment, LayoutableImpl},
    media::TextTrimming,
    metadata::{MarkupDelegate, into_markup_value},
    styling::{ControlTheme, ThemeVariant},
    threading::{Dispatcher, DispatcherPriority},
};
use ferroui_controls::{
    Application, Border, Button, Control, ControlImpl, Flyout, Grid, MenuFlyout, Panel,
    PlacementMode, ScrollViewer, StackPanel, TextBlock, TextBox, ToolTip, TopLevel,
    automation::{AccessibilityView, AutomationProperties},
    primitives::{
        TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
    },
    shapes::{Rectangle, Shape},
};
use ferroribbon_core::collections::ObservableCollection;
use ferroui_system::{
    IDisposable,
    collections::{specialized::INotifyCollectionChanged},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

// The SDK flyout's popup callbacks are weak. Keep a bounded lease until Closed;
// dropping the owner also closes and unlinks it. No flyout -> toolbar strong edge.
struct OverflowLease {
    flyout: Ref<Flyout>,
    cleanup: Rc<OverflowCleanup>,
}
struct OverflowCleanup {
    panel: Ref<StackPanel>,
    completed: Cell<bool>,
}
impl OverflowCleanup {
    fn run(&self) {
        if !self.completed.replace(true) {
            RibbonItemHelper::unlink_tree(Some(self.panel.clone().upcast()));
        }
    }
}
impl Drop for OverflowLease {
    fn drop(&mut self) {
        self.flyout.hide();
        self.cleanup.run();
    }
}

#[repr(C)]
pub struct RibbonToolBar {
    base: TemplatedControl,
    items: RibbonControlItems,
    panel: Ref<RibbonToolBarPanel>,
    overflow_button: Ref<RibbonButton>,
    hidden_by_overflow: RefCell<Vec<(Ref<Control>, bool)>>,
    host: RefCell<Option<Ref<Border>>>,
    item_invoked: RefCell<Option<MarkupDelegate>>,
    collection_token: Cell<Option<u64>>,
    layout_subscription: RefCell<Option<ferroui_base::reactive::Subscription>>,
    open_flyouts: RefCell<Vec<OverflowLease>>,
}
ferro_class!(RibbonToolBar:TemplatedControl);
ferro_impl_classes!(RibbonToolBar:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl);
ferro_class_info!(RibbonToolBar {new:RibbonToolBar::new,markup:{namespace:"FerroRibbon.Controls",content:Items,
 properties:[Items:RibbonControlItems{get:RibbonToolBar::items},Metrics:Arc<RibbonMetrics>{get:RibbonToolBar::metrics},
 Ribbon:Option<Ref<Ribbon>>{get:RibbonToolBar::ribbon,set:|this:&RibbonToolBar,v:Option<Ref<Ribbon>>|this.set_ribbon(v)},
 OverflowItems:RibbonToolBarOverflowItems{get:RibbonToolBar::overflow_items}],
 methods:[fn OnItemInvoked(Ref<Control>,Option<String>,Option<BoxedValue>)=>|this:&RibbonToolBar,item,id:Option<String>,p|this.on_item_invoked(item,id.map(Into::into),p),
 fn InvalidateItemsLayout()=>RibbonToolBar::invalidate_items_layout,fn ShowOverflow()=>RibbonToolBar::show_overflow],
 events:[ItemInvoked(Option<BoxedValue>,Rc<RibbonItemInvokedEventArgs>){add:RibbonToolBar::add_item_invoked,remove:RibbonToolBar::remove_item_invoked}]
 },interfaces:[Rc<dyn IRibbonItemOwner> =>|v|Rc::new(ToolBarOwner(v)) as Rc<dyn IRibbonItemOwner>,
 Rc<dyn IRibbonLayoutHost> =>|v|Rc::new(ToolBarHost(v)) as Rc<dyn IRibbonLayoutHost>]});
ferro_properties! {impl RibbonToolBar {
 pub fn orientation_property()->StyledProperty<Orientation>{FerroProperty::register::<Self,_>("Orientation",Orientation::Horizontal)}
 pub fn columns_property()->StyledProperty<i32>{FerroProperty::register::<Self,_>("Columns",1)}
 pub fn show_labels_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("ShowLabels",false)}
 pub fn is_overflow_enabled_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsOverflowEnabled",true)}
 pub fn density_property()->StyledProperty<RibbonDensity>{FerroProperty::register::<Self,_>("Density",RibbonDensity::Comfortable)}
 pub fn command_catalog_property()->StyledProperty<Option<Rc<RibbonCommandCatalog>>>{FerroProperty::register::<Self,_>("CommandCatalog",None)}
 pub fn ribbon_property()->StyledProperty<Option<ElementRef<Ribbon>>>{
    ValueTypes::register_element_ref::<Ribbon>();
    FerroProperty::register::<Self,_>("Ribbon",None)
 }
}}
impl RibbonToolBar {
    fn static_constructor() {
        Self::orientation_property()
            .changed()
            .add_class_handler::<Self>(|t, _| t.apply_layouts())
            .detach();
        Self::columns_property()
            .changed()
            .add_class_handler::<Self>(|t, _| t.apply_layouts())
            .detach();
        Self::show_labels_property()
            .changed()
            .add_class_handler::<Self>(|t, _| t.apply_layouts())
            .detach();
        Self::is_overflow_enabled_property()
            .changed()
            .add_class_handler::<Self>(|t, _| t.apply_layouts())
            .detach();
        Self::density_property()
            .changed()
            .add_class_handler::<Self>(|t, _| t.apply_layouts())
            .detach();
        Self::command_catalog_property()
            .changed()
            .add_class_handler::<Self>(|t, _| t.resolve_commands())
            .detach();
        Self::ribbon_property()
            .changed()
            .add_class_handler::<Self>(|t, _| t.resolve_commands())
            .detach();
    }
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            items: RibbonControlItems::new_managed(),
            panel: RibbonToolBarPanel::new(),
            overflow_button: RibbonButton::new(),
            hidden_by_overflow: RefCell::default(),
            host: RefCell::default(),
            item_invoked: RefCell::default(),
            collection_token: Cell::new(None),
            layout_subscription: RefCell::default(),
            open_flyouts: RefCell::default(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn items(&self) -> RibbonControlItems {
        self.items.clone()
    }
    pub fn orientation(&self) -> Orientation {
        self.get_value(Self::orientation_property())
    }
    pub fn set_orientation(&self, v: Orientation) {
        self.set_value(Self::orientation_property(), v)
    }
    pub fn columns(&self) -> i32 {
        self.get_value(Self::columns_property())
    }
    pub fn set_columns(&self, v: i32) {
        self.set_value(Self::columns_property(), v)
    }
    pub fn show_labels(&self) -> bool {
        self.get_value(Self::show_labels_property())
    }
    pub fn set_show_labels(&self, v: bool) {
        self.set_value(Self::show_labels_property(), v)
    }
    pub fn is_overflow_enabled(&self) -> bool {
        self.get_value(Self::is_overflow_enabled_property())
    }
    pub fn set_is_overflow_enabled(&self, v: bool) {
        self.set_value(Self::is_overflow_enabled_property(), v)
    }
    pub fn density(&self) -> RibbonDensity {
        self.get_value(Self::density_property())
    }
    pub fn set_density(&self, v: RibbonDensity) {
        self.set_value(Self::density_property(), v)
    }
    pub fn command_catalog(&self) -> Option<Rc<RibbonCommandCatalog>> {
        self.get_value(Self::command_catalog_property())
    }
    pub fn set_command_catalog(&self, v: Option<Rc<RibbonCommandCatalog>>) {
        self.set_value(Self::command_catalog_property(), v)
    }
    pub fn ribbon(&self) -> Option<Ref<Ribbon>> {
        ElementRef::resolve(&self.get_value(Self::ribbon_property()))
    }
    pub fn set_ribbon(&self, v: impl Into<Nullable<Ribbon>>) {
        self.set_value(
            Self::ribbon_property(),
            ElementRef::from_nullable(v.into().0),
        )
    }
    pub fn metrics(&self) -> Arc<RibbonMetrics> {
        self.ribbon()
            .map(|r| r.metrics())
            .unwrap_or_else(|| RibbonMetrics::for_density(self.density()))
    }
    pub fn overflow_items(&self) -> RibbonToolBarOverflowItems {
        self.panel.overflow_items()
    }
    fn owner(&self) -> Rc<dyn IRibbonItemOwner> {
        Rc::new(ToolBarOwner(self.to_ref()))
    }
    pub fn add_item_invoked(&self, h: Option<MarkupDelegate>) {
        let v = MarkupDelegate::combine(self.item_invoked.borrow().as_ref(), h.as_ref());
        self.item_invoked.replace(v);
    }
    pub fn remove_item_invoked(&self, h: Option<MarkupDelegate>) {
        let v = MarkupDelegate::remove(self.item_invoked.borrow().as_ref(), h.as_ref());
        self.item_invoked.replace(v);
    }
    pub fn on_item_invoked(
        &self,
        item: Ref<Control>,
        command_id: Option<RibbonString>,
        parameter: Option<BoxedValue>,
    ) {
        let handler = self.item_invoked.borrow().clone();
        if let Some(handler) = handler {
            let item_id = as_item(&item).and_then(|i| i.id()).or_else(|| {
                item.cast::<RibbonGroup>()
                    .and_then(|g| g.id().map(Into::into))
            });
            handler
                .try_invoke(&[
                    Some(Control::boxed(self.to_ref())),
                    Some(Rc::new(Rc::new(RibbonItemInvokedEventArgs {
                        item: item.clone(),
                        command_id: command_id.clone(),
                        item_id,
                        parameter: parameter.clone(),
                    }))),
                ])
                .expect("ItemInvoked handler");
        }
        // Re-read after the toolbar event: user code can replace Ribbon.
        if let Some(ribbon) = self.ribbon() {
            ribbon.on_item_invoked(item, command_id, parameter);
        }
    }
    fn sync(&self) {
        let overflow = self.panel.overflow_button();
        self.panel.set_overflow_button(None);
        self.panel.children().clear();
        for item in self.items.iter() {
            let item = item.expect("NullReference: toolbar item");
            detach_panel_parent(&item);
            RibbonItemHelper::set_owner(&item, Some(self.owner())).expect("toolbar item owner");
            self.panel.children().add(item);
        }
        let stale: Vec<_> = self
            .hidden_by_overflow
            .borrow()
            .iter()
            .filter(|(i, _)| !self.items.contains(&Some(i.clone())))
            .map(|(i, _)| i.clone())
            .collect();
        for item in stale {
            self.restore_accessibility(&item);
        }
        self.panel.set_overflow_button(overflow);
        self.resolve_commands();
        self.apply_layouts();
    }
    fn resolve_commands(&self) {
        for item in self.items.iter() {
            for item in RibbonGroup::flatten(item).iter() {
                RibbonItemHelper::resolve_command(&item).expect("toolbar command resolution");
            }
        }
    }
    fn update_overflow_accessibility(&self) {
        let overflow = self.panel.overflow_items();
        for item in self.items.iter() {
            let item = item.expect("NullReference: toolbar item");
            let hidden = overflow.iter().any(|i| i.as_ref() == Some(&item));
            let known = self
                .hidden_by_overflow
                .borrow()
                .iter()
                .any(|(i, _)| i == &item);
            if hidden && !known {
                let source_control = item.cast::<TemplatedControl>();
                self.hidden_by_overflow.borrow_mut().push((
                    item.clone(),
                    source_control
                        .as_ref()
                        .is_some_and(|c| KeyboardNavigation::get_is_tab_stop(c)),
                ));
                if let Some(control) = source_control {
                    KeyboardNavigation::set_is_tab_stop(&control, false);
                }
                AutomationProperties::set_accessibility_view(&item, AccessibilityView::Raw);
            } else if !hidden && known {
                self.restore_accessibility(&item);
            }
        }
        let show_button = overflow.count() > 0;
        if KeyboardNavigation::get_is_tab_stop(&self.overflow_button) != show_button {
            KeyboardNavigation::set_is_tab_stop(&self.overflow_button, show_button);
            AutomationProperties::set_accessibility_view(
                &self.overflow_button,
                if show_button {
                    AccessibilityView::Content
                } else {
                    AccessibilityView::Raw
                },
            );
        }
    }
    fn restore_accessibility(&self, item: &Ref<Control>) {
        let saved = {
            let mut hidden = self.hidden_by_overflow.borrow_mut();
            hidden
                .iter()
                .position(|(i, _)| i == item)
                .map(|i| hidden.remove(i).1)
        };
        if let Some(was_tab_stop) = saved {
            if was_tab_stop {
                KeyboardNavigation::set_is_tab_stop(item, true);
            }
            item.clear_local_value(AutomationProperties::accessibility_view_property());
        }
    }
    fn apply_layouts(&self) {
        let metrics = self.metrics();
        self.panel.set_orientation(self.orientation());
        self.panel.set_columns(self.columns().max(1));
        self.panel
            .set_is_overflow_enabled(self.is_overflow_enabled());
        for item in self.items.iter() {
            if let Some(separator) = item.as_ref().and_then(|i| i.cast::<RibbonSeparator>()) {
                separator.set_orientation(if self.orientation() == Orientation::Horizontal {
                    ferroui_base::layout::Orientation::Vertical
                } else {
                    ferroui_base::layout::Orientation::Horizontal
                });
            }
            if let Some(item) = item.as_ref().and_then(|i| as_item(i)) {
                item.apply_layout(RibbonItemLayout {
                    size: if self.show_labels() {
                        RibbonItemSize::Medium
                    } else {
                        RibbonItemSize::Small
                    },
                    metrics: Some(metrics.clone()),
                    is_simplified: true,
                    show_label: if self.show_labels() {
                        None
                    } else {
                        Some(false)
                    },
                    ..Default::default()
                });
            }
        }
        self.overflow_button.apply_layout(RibbonItemLayout {
            size: RibbonItemSize::Small,
            metrics: Some(metrics),
            is_simplified: true,
            show_label: Some(false),
            ..Default::default()
        });
        self.panel.invalidate_measure();
    }
    pub fn invalidate_items_layout(&self) {
        self.panel.invalidate_measure();
    }
    pub fn show_overflow(&self) {
        let panel = StackPanel::new();
        panel.set_spacing(1.);
        panel.set_value(
            ThemeVariant::requested_theme_variant_property(),
            self.actual_theme_variant(),
        );
        for item in self.panel.overflow_items().iter() {
            let item = item.expect("NullReference: overflow item");
            if item.cast::<RibbonSeparator>().is_some() {
                let line = Rectangle::new();
                line.set_height(1.);
                line.set_margin(Thickness::uniform(4.));
                RibbonTheme::set_theme_brush(
                    Some(&line),
                    Some(Shape::fill_property().as_property()),
                    Some("RibbonSeparatorBrush"),
                )
                .expect("separator theme brush");
                panel.children().add(line);
                continue;
            }
            if let Some(copy) = as_item(&item).and_then(|i| i.create_linked_copy()) {
                RibbonItemHelper::set_owner(&copy, Some(self.owner())).expect("overflow owner");
                if let Some(item) = as_item(&copy) {
                    item.apply_layout(RibbonItemLayout {
                        size: RibbonItemSize::Medium,
                        metrics: Some(self.metrics()),
                        is_simplified: false,
                        show_label: Some(true),
                        ..Default::default()
                    });
                }
                copy.set_horizontal_alignment(HorizontalAlignment::Stretch);
                panel.children().add(copy);
            } else if item.is_visible()
                && let Some(placeholder) = Self::create_overflow_placeholder(&item)
            {
                panel.children().add(placeholder);
            }
        }
        // Native StackPanel has no Padding. A Border owns the same panel; no
        // item is reconstructed or moved out of this linked-copy container.
        let padded = Border::new();
        padded.set_min_width(200.);
        padded.set_padding(Thickness::uniform(2.));
        padded.set_child(panel.clone());
        let viewer = ScrollViewer::new();
        viewer.set_content(Some(Control::boxed(padded)));
        viewer.set_max_height(520.);
        let flyout = Flyout::new();
        flyout.set_content(Some(Control::boxed(viewer)));
        flyout.set_placement(if self.orientation() == Orientation::Horizontal {
            PlacementMode::BottomEdgeAlignedRight
        } else {
            PlacementMode::RightEdgeAlignedBottom
        });
        if let Some(style) = application_theme("RibbonFlyoutPresenterStyle") {
            flyout.set_flyout_presenter_theme(style);
        }
        let cleanup = Rc::new(OverflowCleanup {
            panel,
            completed: Cell::new(false),
        });
        let weak = self.to_ref().downgrade();
        let weak_flyout = flyout.downgrade();
        let on_closed = cleanup.clone();
        flyout
            .closed(move || {
                on_closed.run();
                if let (Some(owner), Some(flyout)) = (weak.upgrade(), weak_flyout.upgrade()) {
                    let lease = {
                        let mut leases = owner.open_flyouts.borrow_mut();
                        leases
                            .iter()
                            .position(|l| l.flyout == flyout)
                            .map(|i| leases.remove(i))
                    };
                    drop(lease);
                }
            })
            .detach();
        self.open_flyouts.borrow_mut().push(OverflowLease {
            flyout: flyout.clone(),
            cleanup,
        });
        flyout.show_at(&self.overflow_button);
    }
    fn create_overflow_placeholder(item: &Control) -> Option<Ref<TextBlock>> {
        let mut name = AutomationProperties::get_name(item).unwrap_or_default();
        if name.is_empty() {
            name = item.name().unwrap_or_default();
        }
        if name.is_empty() {
            return None;
        }
        let text = TextBlock::new();
        text.set_text(Some(&name));
        text.set_margin(Thickness::new(8., 4., 8., 4.));
        text.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
        RibbonTheme::set_theme_brush(
            Some(&text),
            Some(TextBlock::foreground_property().as_property()),
            Some("RibbonDisabledForegroundBrush"),
        )
        .expect("placeholder theme brush");
        Some(text)
    }
    fn on_key_down(&self, e: &KeyEventArgs) {
        let (forward, back) = if self.orientation() == Orientation::Horizontal {
            (Key::Right, Key::Left)
        } else {
            (Key::Down, Key::Up)
        };
        if (e.key != forward && e.key != back) || TopLevel::get_top_level(Some(self)).is_none() {
            return;
        }
        let focusables: Vec<_> = self
            .panel
            .children()
            .to_vec()
            .into_iter()
            .filter(|c| {
                c.cast::<TemplatedControl>().is_some()
                    && c.is_visible()
                    && KeyboardNavigation::get_is_tab_stop(c)
                    && c.is_enabled()
                    && c.bounds().width > 0.
            })
            .collect();
        let current = FocusManager::get_focus_manager(self)
            .and_then(|f| f.get_focused_element())
            .and_then(|v| v.cast::<Control>());
        let Some(current) = current else { return };
        if current.cast::<TextBox>().is_some()
            || current.cast::<RibbonInputBase>().is_some()
            || e.source() != Some(current.clone().upcast())
        {
            return;
        }
        let Some(index) = focusables.iter().position(|c| c == &current) else {
            return;
        };
        let next = (index as isize + if e.key == forward { 1 } else { -1 })
            .clamp(0, focusables.len() as isize - 1) as usize;
        if next != index {
            focusables[next].focus_with(NavigationMethod::Directional, KeyModifiers::NONE);
        }
        e.set_handled(true);
    }
}
impl FerroObjectImpl for RibbonToolBar {
    fn constructed(t: &Self) {
        Self::parent_constructed(t);
        RibbonTheme::ensure_resources();
        KeyboardNavigation::set_is_tab_stop(t, false);
        let weak = t.to_ref().downgrade();
        let token = t.items.collection_changed().add(Rc::new(move |_| {
            if let Some(t) = weak.upgrade() {
                t.sync();
            }
        }));
        t.collection_token.set(Some(token));
        let label = RibbonStrings::current()
            .more_options()
            .map(|s| s.to_utf8().expect("native overflow label"));
        let b = &t.overflow_button;
        b.set_icon(Some(Rc::new("\u{E712}".to_string())));
        b.set_label(label.clone());
        b.set_show_label(false);
        b.set_can_add_to_quick_access(false);
        b.set_id(Some("toolbar.overflow".into()));
        ToolTip::set_tip(b, label.map(|s| Rc::new(s) as BoxedValue));
        b.set_is_chrome_button(true);
        let weak = t.to_ref().downgrade();
        b.click(move |_, _| {
            if let Some(t) = weak.upgrade() {
                Dispatcher::ui_thread()
                    .post_local(move || t.show_overflow(), DispatcherPriority::NORMAL);
            }
        });
        RibbonItemHelper::set_owner(b, Some(t.owner())).expect("overflow button owner");
        t.panel.set_overflow_button(Some(b.clone().upcast()));
        AutomationProperties::set_name(t, Some("Toolbar"));
        let weak = t.to_ref().downgrade();
        t.key_down(move |_, e| {
            if let Some(t) = weak.upgrade() {
                t.on_key_down(e);
            }
        });
        let weak = t.to_ref().downgrade();
        t.layout_subscription
            .replace(Some(t.layout_updated(move || {
                if let Some(t) = weak.upgrade() {
                    t.update_overflow_accessibility();
                }
            })));
    }
}
impl Drop for RibbonToolBar {
    fn drop(&mut self) {
        if let Some(token) = self.collection_token.take() {
            self.items.collection_changed().remove(token);
        }
        if let Some(token) = self.layout_subscription.get_mut().take() {
            token.dispose();
        }
        // Draining releases the borrow before hide/Closed/user callbacks.
        let leases = std::mem::take(self.open_flyouts.get_mut());
        drop(leases);
    }
}
impl StyledElementImpl for RibbonToolBar {
    fn style_key_override(_: &Self) -> &'static ferroui_base::TypeInfo {
        Self::TYPE
    }
}
impl ControlImpl for RibbonToolBar {
    fn on_create_automation_peer(
        t: &Self,
    ) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        RibbonToolBarAutomationPeer::new(t).upcast()
    }
}
impl TemplatedControlImpl for RibbonToolBar {
    fn on_apply_template(t: &Self, e: &TemplateAppliedEventArgs) {
        let old_host = t.host.borrow().clone();
        if let Some(host) = old_host {
            host.set_child(None::<Ref<Control>>);
        }
        Self::parent_on_apply_template(t, e);
        let host = e.name_scope().find_as::<Border>("PART_ItemsHost");
        t.host.replace(host.clone());
        if let Some(host) = host {
            host.set_child(t.panel.clone());
        }
        t.sync();
    }
}
struct ToolBarOwner(Ref<RibbonToolBar>);
impl IRibbonItemOwner for ToolBarOwner {
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
    fn metrics(&self) -> Option<Arc<RibbonMetrics>> {
        Some(self.0.metrics())
    }
    fn owner_ribbon(&self) -> Option<Ref<Ribbon>> {
        self.0.ribbon()
    }
    fn command_catalog(&self) -> Option<Rc<RibbonCommandCatalog>> {
        self.0.command_catalog()
    }
    fn on_item_invoked(&self, item: Ref<Control>, id: Option<RibbonString>, p: Option<BoxedValue>) {
        self.0.on_item_invoked(item, id, p)
    }
}
struct ToolBarHost(Ref<RibbonToolBar>);
impl IRibbonLayoutHost for ToolBarHost {
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
    fn invalidate_items_layout(&self) {
        self.0.invalidate_items_layout()
    }
}
fn detach_panel_parent(item: &Control) {
    if let Some(parent) = item.visual_parent().and_then(|p| p.cast::<Panel>()) {
        parent.children().remove(item.to_ref());
    }
}
fn application_theme(key: &str) -> Option<Ref<ControlTheme>> {
    let app = Application::current().expect("NullReference: Application.Current");
    super::ribbon_combo_box::source_dictionary_resource(&app.resources(), &key.into(), None)
        .flatten()
        .map(|v| {
            ValueTypes::as_object(&*v)
                .and_then(|o| o.cast::<ControlTheme>())
                .expect("InvalidCast: source Style resource")
        })
}

#[repr(C)]
pub struct RibbonContextualToolBar {
    base: TemplatedControl,
    items: RibbonControlItems,
    host: RefCell<Option<Ref<Grid>>>,
    context_changed: RefCell<Option<MarkupDelegate>>,
    collection_token: Cell<Option<u64>>,
}
ferro_class!(RibbonContextualToolBar:TemplatedControl);
ferro_impl_classes!(RibbonContextualToolBar:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_class_info!(RibbonContextualToolBar{new:RibbonContextualToolBar::new,markup:{namespace:"FerroRibbon.Controls",content:Items,
 properties:[Items:RibbonControlItems{get:RibbonContextualToolBar::items}],
 methods:[static fn GetContext(Ref<FerroObject>)->Option<String> =>|o:Ref<FerroObject>|RibbonContextualToolBar::get_context(&o),static fn SetContext(Ref<FerroObject>,Option<String>)=>|o:Ref<FerroObject>,v|RibbonContextualToolBar::set_context(&o,v)],
 events:[ContextChanged(Option<BoxedValue>,Option<String>){add:RibbonContextualToolBar::add_context_changed,remove:RibbonContextualToolBar::remove_context_changed}]
}});
ferro_properties! {impl RibbonContextualToolBar{
 pub fn active_context_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("ActiveContext",None)}
 pub fn context_property()->AttachedProperty<Option<String>>{FerroProperty::register_attached::<Self,FerroObject,_>("Context",None)}
}}
impl RibbonContextualToolBar {
    fn static_constructor() {
        Self::active_context_property()
            .changed()
            .add_class_handler::<Self>(|t, _| t.update())
            .detach();
    }
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            items: RibbonControlItems::new_managed(),
            host: RefCell::default(),
            context_changed: RefCell::default(),
            collection_token: Cell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn items(&self) -> RibbonControlItems {
        self.items.clone()
    }
    pub fn active_context(&self) -> Option<String> {
        self.get_value(Self::active_context_property())
    }
    pub fn set_active_context(&self, v: Option<String>) {
        self.set_value(Self::active_context_property(), v)
    }
    pub fn get_context(o: &FerroObject) -> Option<String> {
        o.get_value(Self::context_property())
    }
    pub fn set_context(o: &FerroObject, v: Option<String>) {
        o.set_value(Self::context_property(), v)
    }
    pub fn add_context_changed(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(self.context_changed.borrow().as_ref(), h.as_ref());
        self.context_changed.replace(next);
    }
    pub fn remove_context_changed(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(self.context_changed.borrow().as_ref(), h.as_ref());
        self.context_changed.replace(next);
    }
    fn sync(&self) {
        let Some(host) = self.host.borrow().clone() else {
            return;
        };
        host.children().clear();
        for item in self.items.iter() {
            let item = item.expect("NullReference: contextual toolbar item");
            detach_panel_parent(&item);
            let current_host = self
                .host
                .borrow()
                .clone()
                .expect("NullReference: contextual toolbar host");
            current_host.children().add(item);
        }
        self.update();
    }
    fn update(&self) {
        for item in self.items.iter() {
            let item = item.expect("NullReference: contextual toolbar item");
            item.set_is_visible(Self::get_context(&item) == self.active_context());
        }
        let handler = self.context_changed.borrow().clone();
        if let Some(handler) = handler {
            handler
                .try_invoke(&[
                    Some(Control::boxed(self.to_ref())),
                    into_markup_value(self.active_context()),
                ])
                .expect("ContextChanged handler");
        }
    }
}
impl FerroObjectImpl for RibbonContextualToolBar {
    fn constructed(t: &Self) {
        Self::parent_constructed(t);
        KeyboardNavigation::set_is_tab_stop(t, false);
        let weak = t.to_ref().downgrade();
        let token = t.items.collection_changed().add(Rc::new(move |_| {
            if let Some(t) = weak.upgrade() {
                t.sync();
            }
        }));
        t.collection_token.set(Some(token));
    }
}
impl Drop for RibbonContextualToolBar {
    fn drop(&mut self) {
        if let Some(token) = self.collection_token.take() {
            self.items.collection_changed().remove(token);
        }
    }
}
impl StyledElementImpl for RibbonContextualToolBar {
    fn style_key_override(_: &Self) -> &'static ferroui_base::TypeInfo {
        Self::TYPE
    }
}
impl TemplatedControlImpl for RibbonContextualToolBar {
    fn on_apply_template(t: &Self, e: &TemplateAppliedEventArgs) {
        let old_host = t.host.borrow().clone();
        if let Some(host) = old_host {
            host.children().clear();
        }
        Self::parent_on_apply_template(t, e);
        t.host.replace(e.name_scope().find_as::<Grid>("PART_Host"));
        t.sync();
    }
}

/// Source menu entries retain their identities in this collection while the
/// native MenuFlyout displays them. The source MenuFlyoutItemBase inheritance
/// boundary is a documented compatibility gap (native separators derive Control).
pub type RibbonMenuBarItems = ObservableCollection<Option<Ref<RibbonMenuBarItem>>>;
#[repr(C)]
pub struct RibbonMenuBarItem {
    base: FerroObject,
    items: RibbonControlItems,
    menu: RefCell<Option<Ref<MenuFlyout>>>,
    opening: RefCell<Option<MarkupDelegate>>,
    header_changed: ferroui_base::utilities::HandlerList<dyn Fn(&RibbonMenuBarItem)>,
    collection_token: Cell<Option<u64>>,
}
ferro_class!(RibbonMenuBarItem:FerroObject);
ferro_class_info!(RibbonMenuBarItem{new:RibbonMenuBarItem::new,markup:{namespace:"FerroRibbon.Controls",content:Items,
 properties:[Items:RibbonControlItems{get:RibbonMenuBarItem::items}],
 events:[Opening(Option<BoxedValue>,ferroui_base::utilities::EventArgs){add:RibbonMenuBarItem::add_opening,remove:RibbonMenuBarItem::remove_opening}]
}});
ferro_properties! {impl RibbonMenuBarItem{
 pub fn header_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Header",None)}
}}
impl RibbonMenuBarItem {
    fn static_constructor() {
        Self::header_property()
            .changed()
            .add_class_handler::<Self>(|t, _| {
                for (_, handler) in t.header_changed.snapshot().iter() {
                    handler(t);
                }
            })
            .detach();
    }
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            items: RibbonControlItems::new_managed(),
            menu: RefCell::default(),
            opening: RefCell::default(),
            header_changed: Default::default(),
            collection_token: Cell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn items(&self) -> RibbonControlItems {
        self.items.clone()
    }
    pub fn header(&self) -> Option<String> {
        self.get_value(Self::header_property())
    }
    pub fn set_header(&self, v: Option<String>) {
        self.set_value(Self::header_property(), v)
    }
    pub fn add_opening(&self, h: Option<MarkupDelegate>) {
        let v = MarkupDelegate::combine(self.opening.borrow().as_ref(), h.as_ref());
        self.opening.replace(v);
    }
    pub fn remove_opening(&self, h: Option<MarkupDelegate>) {
        let v = MarkupDelegate::remove(self.opening.borrow().as_ref(), h.as_ref());
        self.opening.replace(v);
    }
    fn menu(&self) -> Ref<MenuFlyout> {
        let current = self.menu.borrow().clone();
        if let Some(menu) = current {
            return menu;
        }
        let menu = MenuFlyout::new();
        menu.set_placement(PlacementMode::BottomEdgeAlignedLeft);
        self.menu.replace(Some(menu.clone()));
        let weak = self.to_ref().downgrade();
        menu.opening(move |_| {
            if let Some(t) = weak.upgrade() {
                let handler = t.opening.borrow().clone();
                if let Some(handler) = handler {
                    handler
                        .try_invoke(&[
                            Some(Rc::new(t)),
                            Some(Rc::new(ferroui_base::utilities::EventArgs::EMPTY)),
                        ])
                        .expect("Opening handler");
                }
            }
        })
        .detach();
        self.sync_menu();
        menu
    }
    fn sync_menu(&self) {
        let menu = self.menu.borrow().clone();
        let Some(menu) = menu else { return };
        menu.items().clear();
        for item in self.items.iter() {
            menu.items().add(item.map(Control::boxed));
        }
    }
}
impl FerroObjectImpl for RibbonMenuBarItem {
    fn constructed(t: &Self) {
        Self::parent_constructed(t);
        let weak = t.to_ref().downgrade();
        let token = t.items.collection_changed().add(Rc::new(move |_| {
            if let Some(t) = weak.upgrade() {
                t.sync_menu();
            }
        }));
        t.collection_token.set(Some(token));
    }
}
impl Drop for RibbonMenuBarItem {
    fn drop(&mut self) {
        if let Some(token) = self.collection_token.take() {
            self.items.collection_changed().remove(token);
        }
    }
}

struct MenuHeader {
    header: Ref<Button>,
    item: Ref<RibbonMenuBarItem>,
    header_token: u64,
    opened: Rc<dyn IDisposable>,
    closed: Rc<dyn IDisposable>,
}
impl Drop for MenuHeader {
    fn drop(&mut self) {
        self.item.header_changed.remove(self.header_token);
        self.opened.dispose();
        self.closed.dispose();
    }
}
#[repr(C)]
pub struct RibbonMenuBar {
    base: TemplatedControl,
    items: RibbonMenuBarItems,
    menus: RefCell<Vec<MenuHeader>>,
    host: RefCell<Option<Ref<StackPanel>>>,
    open: RefCell<Option<Ref<MenuFlyout>>>,
    collection_token: Cell<Option<u64>>,
}
ferro_class!(RibbonMenuBar:TemplatedControl);
ferro_impl_classes!(RibbonMenuBar:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_class_info!(RibbonMenuBar{new:RibbonMenuBar::new,markup:{namespace:"FerroRibbon.Controls",content:Items,
 properties:[Items:RibbonMenuBarItems{get:RibbonMenuBar::items},IsMenuOpen:bool{get:RibbonMenuBar::is_menu_open}],
 methods:[fn FocusFirst()=>RibbonMenuBar::focus_first,fn OpenMenu(Option<String>)->bool =>|t:&RibbonMenuBar,h:Option<String>|t.open_menu(h.as_deref())]
}});
impl RibbonMenuBar {
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            items: RibbonMenuBarItems::new_managed(),
            menus: RefCell::default(),
            host: RefCell::default(),
            open: RefCell::default(),
            collection_token: Cell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn items(&self) -> RibbonMenuBarItems {
        self.items.clone()
    }
    pub fn is_menu_open(&self) -> bool {
        self.open.borrow().is_some()
    }
    fn build(&self) {
        let old = std::mem::take(&mut *self.menus.borrow_mut());
        drop(old);
        let old_host = self.host.borrow().clone();
        if let Some(host) = old_host {
            host.children().clear();
        }
        if self.host.borrow().is_none() {
            return;
        }
        for item in self.items.iter() {
            let item = item.expect("NullReference: menu bar item");
            let header = Button::new();
            header.set_content(item.header().map(|s| Rc::new(s) as BoxedValue));
            header.set_padding(Thickness::new(8., 2., 8., 2.));
            if let Some(theme) = application_theme("RibbonChromeButtonStyle") {
                header.set_theme(theme);
            }
            AutomationProperties::set_name(&header, Some(item.header().as_deref().unwrap_or("")));
            let menu = item.menu();
            menu.set_overlay_input_pass_through_element(self.to_ref());
            let weak = self.to_ref().downgrade();
            let weak_menu = menu.downgrade();
            let opened = menu.opened(move || {
                if let Some(t) = weak.upgrade() {
                    let old = t.open.replace(weak_menu.upgrade());
                    drop(old);
                }
            });
            let weak = self.to_ref().downgrade();
            let weak_menu = menu.downgrade();
            let closed = menu.closed(move || {
                let owner = weak
                    .upgrade()
                    .filter(|t| t.open.borrow().as_ref() == weak_menu.upgrade().as_ref());
                if let Some(t) = owner {
                    let old = t.open.replace(None);
                    drop(old);
                }
            });
            let weak = self.to_ref().downgrade();
            let header_token = item.header_changed.add(Rc::new(move |item| {
                if let Some(t) = weak.upgrade() {
                    t.on_item_header_changed(item);
                }
            }));
            let weak = self.to_ref().downgrade();
            let weak_header = header.downgrade();
            let clicked_menu = menu.clone();
            header.click(move |_, _| {
                if let (Some(t), Some(header)) = (weak.upgrade(), weak_header.upgrade()) {
                    let is_open = t.open.borrow().as_ref() == Some(&clicked_menu);
                    if is_open {
                        clicked_menu.hide();
                    } else {
                        t.open(&header, clicked_menu.clone());
                    }
                }
            });
            let weak = self.to_ref().downgrade();
            let weak_header = header.downgrade();
            let hovered_menu = menu.clone();
            header.pointer_entered(move |_, _| {
                if let (Some(t), Some(header)) = (weak.upgrade(), weak_header.upgrade()) {
                    let switch = t.open.borrow().as_ref().is_some_and(|m| m != &hovered_menu);
                    if switch {
                        t.open(&header, hovered_menu.clone());
                    }
                }
            });
            let weak = self.to_ref().downgrade();
            let weak_header = header.downgrade();
            header.key_down(move |_, e| {
                if let (Some(t), Some(header)) = (weak.upgrade(), weak_header.upgrade()) {
                    t.on_header_key(&header, e);
                }
            });
            self.menus.borrow_mut().push(MenuHeader {
                header: header.clone(),
                item,
                header_token,
                opened: opened.detach(),
                closed: closed.detach(),
            });
            // Source reads its current host again after callbacks in construction.
            self.host
                .borrow()
                .clone()
                .expect("NullReference: menu bar host")
                .children()
                .add(header);
        }
    }
    fn on_item_header_changed(&self, item: &RibbonMenuBarItem) {
        // Snapshot references only, never hold a RefCell borrow across setters.
        let headers: Vec<_> = self
            .menus
            .borrow()
            .iter()
            .filter(|m| m.item.ptr_eq(&item.to_ref()))
            .map(|m| m.header.clone())
            .collect();
        for header in headers {
            header.set_content(item.header().map(|s| Rc::new(s) as BoxedValue));
            AutomationProperties::set_name(&header, Some(item.header().as_deref().unwrap_or("")));
        }
    }
    fn open(&self, header: &Button, menu: Ref<MenuFlyout>) {
        let current = self.open.borrow().clone();
        if let Some(current) = current
            && current != menu
        {
            current.hide();
        }
        let old = self.open.replace(Some(menu.clone()));
        drop(old);
        menu.show_at(header);
    }
    fn on_header_key(&self, header: &Button, e: &KeyEventArgs) {
        let next = {
            let menus = self.menus.borrow();
            let Some(index) = menus.iter().position(|m| m.header.ptr_eq(&header.to_ref())) else {
                return;
            };
            match e.key {
                Key::Right => Some((menus[(index + 1) % menus.len()].header.clone(), None)),
                Key::Left => Some((
                    menus[(index + menus.len() - 1) % menus.len()]
                        .header
                        .clone(),
                    None,
                )),
                Key::Down => Some((header.to_ref(), Some(menus[index].item.clone()))),
                _ => None,
            }
        };
        if let Some((header, item)) = next {
            if let Some(item) = item {
                self.open(&header, item.menu());
            } else {
                header.focus_with(NavigationMethod::Directional, KeyModifiers::NONE);
            }
            e.set_handled(true);
        }
    }
    pub fn focus_first(&self) {
        let header = self.menus.borrow().first().map(|m| m.header.clone());
        if let Some(header) = header {
            header.focus_with(NavigationMethod::Directional, KeyModifiers::NONE);
        }
    }
    pub fn open_menu(&self, header: Option<&str>) -> bool {
        use ferroribbon_core::text::RibbonOrdinalIgnoreCaseKey as OrdinalKey;
        let found = self
            .menus
            .borrow()
            .iter()
            .find(|m| match (m.item.header(), header) {
                (None, None) => true,
                (Some(a), Some(b)) => OrdinalKey(a.into()) == OrdinalKey(b.into()),
                _ => false,
            })
            .map(|m| (m.header.clone(), m.item.clone()));
        let Some((header, item)) = found else {
            return false;
        };
        self.open(&header, item.menu());
        true
    }
}
impl FerroObjectImpl for RibbonMenuBar {
    fn constructed(t: &Self) {
        Self::parent_constructed(t);
        RibbonTheme::ensure_resources();
        KeyboardNavigation::set_is_tab_stop(t, false);
        let weak = t.to_ref().downgrade();
        let token = t.items.collection_changed().add(Rc::new(move |_| {
            if let Some(t) = weak.upgrade() {
                t.build();
            }
        }));
        t.collection_token.set(Some(token));
    }
}
impl Drop for RibbonMenuBar {
    fn drop(&mut self) {
        if let Some(token) = self.collection_token.take() {
            self.items.collection_changed().remove(token);
        }
        let old = std::mem::take(self.menus.get_mut());
        drop(old);
    }
}
impl StyledElementImpl for RibbonMenuBar {
    fn style_key_override(_: &Self) -> &'static ferroui_base::TypeInfo {
        Self::TYPE
    }
}
impl TemplatedControlImpl for RibbonMenuBar {
    fn on_apply_template(t: &Self, e: &TemplateAppliedEventArgs) {
        let old_host = t.host.borrow().clone();
        if let Some(host) = old_host {
            host.children().clear();
        }
        Self::parent_on_apply_template(t, e);
        t.host
            .replace(e.name_scope().find_as::<StackPanel>("PART_ItemsHost"));
        t.build();
    }
}

#[cfg(test)]
#[path = "../toolbar_tests.rs"]
mod tests;
