//! RibbonQuickAccessToolBar.cs, including its shared toolbar automation peer.
//! Copyright (c) 2026 Wiesław Šoltés, MIT.
use super::{IRibbonItem, IRibbonLayoutHost, Ribbon, RibbonItemHelper, RibbonItemLayout};
use crate::{
    metadata::control_items::RibbonControlItems, primitives::RibbonMetrics, theming::RibbonTheme,
};
use ferroribbon_core::{RibbonItemSize, localization::RibbonStrings};
use ferroui_base::{
    FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref, StyledElementImpl,
    StyledProperty, VisualImpl, WeakRef,
    data::core::{ValueType, ValueTypes},
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{InputElementImpl, KeyboardNavigation},
    instantiate,
    interactivity::{InteractiveImpl, RoutedEventHandlerToken},
    layout::LayoutableImpl,
};
use ferroui_controls::automation::peers::ControlAutomationPeer;
use ferroui_controls::{
    Border, Button, Control, ControlImpl, Panel, ToolTip,
    automation::AutomationProperties,
    primitives::{
        TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
    },
};
use std::{cell::RefCell, rc::Rc, sync::Arc};
#[derive(Default)]
struct Parts {
    host: Option<Ref<Panel>>,
    customize: Option<Ref<Button>>,
    click: Option<RoutedEventHandlerToken>,
}
impl Parts {
    fn detach(&mut self) {
        if let (Some(b), Some(t)) = (&self.customize, self.click.take()) {
            b.remove_handler(Button::click_event(), t);
        }
    }
}
impl Drop for Parts {
    fn drop(&mut self) {
        self.detach();
    }
}
#[repr(C)]
pub struct RibbonQuickAccessToolBar {
    base: TemplatedControl,
    items: RibbonControlItems,
    parts: RefCell<Parts>,
    // Clear/UnionWith in the source leaves entries in first-insertion order.
    // Keeping the ordered distinct references avoids Rust HashSet randomization.
    known: RefCell<Vec<Option<Ref<Control>>>>,
    ribbon: RefCell<Option<WeakRef<Ribbon>>>,
}
ferro_class!(RibbonQuickAccessToolBar:TemplatedControl);
ferro_impl_classes!(RibbonQuickAccessToolBar:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl);
ferro_class_info!(RibbonQuickAccessToolBar{new:RibbonQuickAccessToolBar::new,markup:{namespace:"FerroRibbon.Controls",constructors:[()=>RibbonQuickAccessToolBar::new],content:Items,properties:[Items:RibbonControlItems{get:RibbonQuickAccessToolBar::items},Ribbon:Option<Ref<Ribbon>>{get:RibbonQuickAccessToolBar::ribbon},CustomizeButton:Option<Ref<Button>>{get:RibbonQuickAccessToolBar::customize_button}],methods:[fn InvalidateItemsLayout()=>RibbonQuickAccessToolBar::invalidate_items_layout]},interfaces:[Rc<dyn IRibbonLayoutHost> =>|v|Rc::new(Host(v)) as Rc<dyn IRibbonLayoutHost>]});
ferro_properties! {impl RibbonQuickAccessToolBar{
 pub fn show_customize_button_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("ShowCustomizeButton",true)}
 pub fn show_labels_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("ShowLabels",false)}
 pub fn metrics_property()->StyledProperty<Option<Arc<RibbonMetrics>>>{FerroProperty::register::<Self,_>("Metrics",Some(RibbonMetrics::comfortable()))}
}}
impl RibbonQuickAccessToolBar {
    fn static_constructor() {
        Self::show_customize_button_property()
            .changed()
            .add_class_handler::<Self>(|t, _| t.update_customize_button())
            .detach();
        Self::show_labels_property()
            .changed()
            .add_class_handler::<Self>(|t, _| t.apply_layouts())
            .detach();
        Self::metrics_property()
            .changed()
            .add_class_handler::<Self>(|t, _| t.apply_layouts())
            .detach();
    }
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            items: RibbonControlItems::new_managed(),
            parts: RefCell::default(),
            known: RefCell::default(),
            ribbon: RefCell::default(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn items(&self) -> RibbonControlItems {
        self.items.clone()
    }
    pub fn show_customize_button(&self) -> bool {
        self.get_value(Self::show_customize_button_property())
    }
    pub fn set_show_customize_button(&self, v: bool) {
        self.set_value(Self::show_customize_button_property(), v)
    }
    pub fn show_labels(&self) -> bool {
        self.get_value(Self::show_labels_property())
    }
    pub fn set_show_labels(&self, v: bool) {
        self.set_value(Self::show_labels_property(), v)
    }
    pub fn metrics(&self) -> Option<Arc<RibbonMetrics>> {
        self.get_value(Self::metrics_property())
    }
    pub fn set_metrics(&self, v: Option<Arc<RibbonMetrics>>) {
        self.set_value(Self::metrics_property(), v)
    }
    pub fn ribbon(&self) -> Option<Ref<Ribbon>> {
        self.ribbon.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    pub(crate) fn set_ribbon(&self, v: Option<Ref<Ribbon>>) {
        self.ribbon.replace(v.map(|v| v.downgrade()));
    }
    pub fn customize_button(&self) -> Option<Ref<Button>> {
        self.parts.borrow().customize.clone()
    }
    fn host(&self) -> Option<Ref<Panel>> {
        self.parts.borrow().host.clone()
    }
    fn update_customize_button(&self) {
        if let Some(b) = self.customize_button() {
            b.set_is_visible(self.show_customize_button());
        }
    }
    pub(super) fn detach_from_border(host: &Border) {
        host.set_child(None);
        // Let the previous host release its cached automation child before the
        // next host claims the same toolbar peer.
        if let Some(peer) = ControlAutomationPeer::from_element(host) {
            peer.get_children();
        }
    }
    fn on_customize_click(&self) {
        if let (Some(r), Some(b)) = (self.ribbon(), self.customize_button()) {
            r.build_quick_access_customize_menu().show_at(&b);
        }
    }
    fn on_items_changed(&self) {
        let removed = self
            .known
            .borrow()
            .iter()
            .filter(|i| !self.items.contains(i))
            .cloned()
            .collect::<Vec<_>>();
        for removed in removed {
            let removed = removed.expect("ArgumentNull: GetSourceItem element");
            if RibbonItemHelper::get_source_item(&removed).is_some() {
                RibbonItemHelper::unlink_tree(Some(removed.upcast()));
            }
        }
        self.known.borrow_mut().clear();
        for item in self.items.iter() {
            if !self.known.borrow().contains(&item) {
                self.known.borrow_mut().push(item);
            }
        }
        self.sync_items();
        if let Some(r) = self.ribbon() {
            r.on_quick_access_items_changed();
        }
    }
    fn sync_items(&self) {
        let Some(host) = self.host() else { return };
        host.children().clear();
        for item in self.items.iter() {
            let item = item.expect("ArgumentNull: QAT child");
            if let Some(parent) = item.visual_parent().and_then(|p| p.cast::<Panel>()) {
                parent.children().remove(item.clone());
            }
            if let Some(r) = self.ribbon() {
                RibbonItemHelper::set_owner(&item, Some(r.item_owner())).expect("QAT owner");
            }
            self.host()
                .expect("QAT items host became null during synchronization")
                .children()
                .add(item);
        }
        self.apply_layouts();
    }
    pub(crate) fn apply_layouts(&self) {
        for item in self.items.iter().flatten() {
            if let Some(item) = as_item(&item) {
                item.apply_layout(RibbonItemLayout {
                    is_simplified: true,
                    show_label: Some(self.show_labels()),
                    ..RibbonItemLayout::new(RibbonItemSize::Small, self.metrics())
                });
            }
        }
    }
    pub fn invalidate_items_layout(&self) {
        self.invalidate_measure();
    }
}
pub(crate) fn as_item(item: &Control) -> Option<Rc<dyn IRibbonItem>> {
    ValueTypes::try_cast(
        &Control::boxed(item.to_ref()),
        ValueType::of::<Rc<dyn IRibbonItem>>(),
    )
    .and_then(|v| v.downcast_ref::<Rc<dyn IRibbonItem>>().cloned())
}
impl FerroObjectImpl for RibbonQuickAccessToolBar {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        RibbonTheme::ensure_resources();
        KeyboardNavigation::set_is_tab_stop(this, false);
        let weak = this.to_ref().downgrade();
        this.items.add_collection_changed(Rc::new(move |_| {
            if let Some(t) = weak.upgrade() {
                t.on_items_changed();
            }
        }));
        AutomationProperties::set_name(
            this,
            RibbonStrings::current()
                .quick_access_toolbar()
                .as_ref()
                .map(|s| s.to_utf8().expect("native toolbar automation name"))
                .as_deref(),
        );
    }
}
impl StyledElementImpl for RibbonQuickAccessToolBar {
    fn style_key_override(_: &Self) -> &'static ferroui_base::TypeInfo {
        <Self as ferroui_base::StaticType>::TYPE
    }
}
impl ControlImpl for RibbonQuickAccessToolBar {
    fn on_create_automation_peer(
        this: &Self,
    ) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        RibbonToolBarAutomationPeer::new(this).upcast()
    }
}
impl TemplatedControlImpl for RibbonQuickAccessToolBar {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        this.parts.borrow_mut().detach();
        if let Some(h) = this.host() {
            h.children().clear();
        }
        Self::parent_on_apply_template(this, e);
        let host = e
            .name_scope()
            .find("PART_ItemsHost")
            .and_then(|v| v.cast::<Panel>());
        this.parts.borrow_mut().host = host;
        let b = e
            .name_scope()
            .find("PART_CustomizeButton")
            .and_then(|v| v.cast::<Button>());
        this.parts.borrow_mut().customize = b.clone();
        if let Some(b) = b {
            let weak = this.to_ref().downgrade();
            let token = b.click(move |_, _| {
                if let Some(t) = weak.upgrade() {
                    t.on_customize_click();
                }
            });
            this.parts.borrow_mut().click = Some(token);
            let label = RibbonStrings::current()
                .customize_quick_access_toolbar()
                .map(|s| s.to_utf8().expect("native QAT label"));
            ToolTip::set_tip(
                &b,
                label
                    .as_ref()
                    .map(|s| Rc::new(s.clone()) as ferroui_base::BoxedValue),
            );
            AutomationProperties::set_name(&b, label.as_deref());
        }
        this.sync_items();
        this.update_customize_button();
    }
}
struct Host(Ref<RibbonQuickAccessToolBar>);
impl IRibbonLayoutHost for Host {
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
    fn invalidate_items_layout(&self) {
        self.0.invalidate_items_layout();
    }
}
#[repr(C)]
pub struct RibbonToolBarAutomationPeer {
    base: ferroui_controls::automation::peers::ControlAutomationPeer,
}
ferro_class!(RibbonToolBarAutomationPeer:ControlAutomationPeer);
ferro_class_info!(RibbonToolBarAutomationPeer{markup:{namespace:"FerroRibbon.Controls",constructors:[(Ref<Control>)=>|v:Ref<Control>|RibbonToolBarAutomationPeer::new(&v)]}});
impl FerroObjectImpl for RibbonToolBarAutomationPeer {}
impl ferroui_controls::automation::peers::ControlAutomationPeerImpl
    for RibbonToolBarAutomationPeer
{
}
impl RibbonToolBarAutomationPeer {
    pub fn new(owner: &Control) -> Ref<Self> {
        instantiate(Self {
            base: ferroui_controls::automation::peers::ControlAutomationPeer::construct(owner),
        })
    }
}
impl ferroui_controls::automation::peers::AutomationPeerImpl for RibbonToolBarAutomationPeer {
    fn get_automation_control_type_core(
        _: &Self,
    ) -> ferroui_controls::automation::peers::AutomationControlType {
        ferroui_controls::automation::peers::AutomationControlType::ToolBar
    }
    fn get_class_name_core(this: &Self) -> String {
        ferroui_controls::automation::peers::ControlAutomationPeer::owner(this)
            .get_type()
            .name()
            .to_owned()
    }
}
