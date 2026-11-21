//! Native tab, tab header and contextual group from RibbonTab.cs. Linked-control simplified
//! overflow is in ribbon_tab_overflow.rs.
use super::{Ribbon, RibbonButton, RibbonItemLayout, RibbonPanelPresentation};
use crate::{
    metadata::ribbon_collections::RibbonGroups,
    primitives::{RibbonGroupsPanel, RibbonIconPresenter, RibbonMetrics, RibbonScrollPanel},
    theming::RibbonTheme,
};
use ferroribbon_core::{
    RibbonContextualActivation, RibbonItemSize, layout::RibbonReductionStrategy,
    localization::RibbonStrings,
};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    BoxedValue, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref,
    StyledElementImpl, StyledProperty, VisualImpl, WeakRef,
    controls::NameScopeRef,
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    media::{Color, IBrush},
};
use ferroui_controls::{
    Button, ButtonImpl, ContentControlImpl, Control, ControlImpl, Panel, TextBlock, ToolTip,
    automation::{
        AutomationProperties, SelectionItemPatternIdentifiers, peers::ControlAutomationPeer,
    },
    presenters::ContentPresenter,
    primitives::{
        TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
    },
    shapes::Rectangle,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
macro_rules! property {
    ($get:ident,$set:ident,$p:ident,$ty:ty) => {
        pub fn $get(&self) -> $ty {
            self.get_value(Self::$p())
        }
        pub fn $set(&self, value: $ty) {
            self.set_value(Self::$p(), value)
        }
    };
}
#[repr(C)]
pub struct RibbonTab {
    base: TemplatedControl,
    groups: RibbonGroups,
    pub(super) display_group_view: Rc<super::RibbonDisplayGroups>,
    groups_panel: Ref<RibbonGroupsPanel>,
    overflow_button: Ref<RibbonButton>,
    scroll_host: RefCell<Option<Ref<RibbonScrollPanel>>>,
    ribbon: RefCell<Option<WeakRef<Ribbon>>>,
    // The ribbon owns generated headers; a header owns its tab. This cache is
    // weak to keep these ownership links acyclic.
    header_element: RefCell<Option<WeakRef<RibbonTabHeader>>>,
    hidden_by_customization: Cell<bool>,
    pub(crate) original_header: RefCell<Option<String>>,
    pub(crate) is_custom: Cell<bool>,
    pub(super) group_order: RefCell<
        Option<
            ferroribbon_core::collections::RibbonReadOnlyList<
                Option<ferroribbon_core::RibbonString>,
            >,
        >,
    >,
    pub(super) open_overflows: RefCell<Vec<Ref<ferroui_controls::Flyout>>>,
}
ferro_class!(RibbonTab:TemplatedControl);
ferro_class_info!(RibbonTab {new:RibbonTab::new,markup:{namespace:"FerroRibbon.Controls",content:Groups,properties:[
 Groups:RibbonGroups{get:RibbonTab::groups},GroupsPanel:Ref<RibbonGroupsPanel>{get:RibbonTab::groups_panel},OverflowButton:Ref<RibbonButton>{get:RibbonTab::overflow_button},
 Ribbon:Option<Ref<Ribbon>>{get:RibbonTab::ribbon},HeaderElement:Option<Ref<RibbonTabHeader>>{get:RibbonTab::header_element},
 DisplayGroups:Rc<super::RibbonDisplayGroups>{get:RibbonTab::display_groups},IsContextual:bool{get:RibbonTab::is_contextual},IsAvailable:bool{get:RibbonTab::is_available}],methods:[fn InvalidateGroupsLayout() => RibbonTab::invalidate_groups_layout,fn ShowOverflow()=>RibbonTab::show_overflow]}});
ferro_impl_classes!(RibbonTab:VisualImpl,InteractiveImpl,InputElementImpl,LayoutableImpl);
ferro_properties! {impl RibbonTab {
 pub fn header_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Header",None)}
 pub fn id_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Id",None)}
 pub fn key_tip_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("KeyTip",None)}
 pub fn is_tab_visible_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsTabVisible",true)}
 pub fn contextual_group_id_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("ContextualGroupId",None)}
 pub fn is_selected_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsSelected",false)}
 pub fn screen_tip_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("ScreenTip",None)}
 pub fn icon_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("Icon",None)}
}}
impl RibbonTab {
    fn static_constructor() {
        macro_rules! tabs {($($p:ident),*) => {$(Self::$p().changed().add_class_handler::<Self>(|this,_|{if let Some(r)=this.ribbon(){r.on_tabs_changed()}}).detach();)*};}
        tabs!(
            header_property,
            is_tab_visible_property,
            contextual_group_id_property
        );
        macro_rules! text {($($p:ident),*) => {$(Self::$p().changed().add_class_handler::<Self>(|this,_|{if let Some(h)=this.header_element(){h.update_text()}}).detach();)*};}
        text!(screen_tip_property, icon_property);
        Self::id_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.update_header_automation_id())
            .detach();
        ferroui_base::StyledElement::name_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.update_header_automation_id())
            .detach();
        ferroui_base::input::InputElement::is_enabled_property()
            .changed()
            .add_class_handler::<Self>(|this, _| {
                if let Some(r) = this.ribbon() {
                    r.on_tabs_changed()
                }
            })
            .detach();
    }
    pub fn construct() -> Self {
        let groups = RibbonGroups::new_managed();
        let display_group_view = super::RibbonDisplayGroups::from_groups(groups.clone());
        let overflow = RibbonButton::new();
        overflow.set_icon(Some(Rc::new("\u{e712}".to_owned())));
        overflow.set_label(
            RibbonStrings::current()
                .more_options()
                .map(|s| s.to_utf8().expect("UTF-16 native label")),
        );
        overflow.set_show_label(false);
        overflow.set_can_add_to_quick_access(false);
        overflow.set_id(Some("ribbon.overflow".into()));
        overflow.set_is_chrome_button(true);
        ToolTip::set_tip(
            &overflow,
            overflow.label().map(|v| Rc::new(v) as BoxedValue),
        );
        let panel = RibbonGroupsPanel::new();
        panel.set_overflow_button(Some(overflow.clone().upcast()));
        Self {
            base: TemplatedControl::construct(),
            groups,
            display_group_view,
            groups_panel: panel,
            overflow_button: overflow,
            scroll_host: RefCell::default(),
            ribbon: RefCell::default(),
            header_element: RefCell::default(),
            hidden_by_customization: Cell::new(false),
            original_header: RefCell::default(),
            is_custom: Cell::new(false),
            group_order: RefCell::default(),
            open_overflows: RefCell::default(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn groups(&self) -> RibbonGroups {
        self.groups.clone()
    }
    pub fn groups_panel(&self) -> Ref<RibbonGroupsPanel> {
        self.groups_panel.clone()
    }
    pub fn overflow_button(&self) -> Ref<RibbonButton> {
        self.overflow_button.clone()
    }
    pub(crate) fn on_strings_changed(&self) {
        self.overflow_button.set_label(
            RibbonStrings::current()
                .more_options()
                .map(|s| s.to_utf8().expect("native overflow label")),
        );
        ToolTip::set_tip(
            &self.overflow_button,
            RibbonStrings::current()
                .more_options()
                .map(|s| Rc::new(s.to_utf8().expect("native overflow tooltip")) as BoxedValue),
        );
        if let Some(header) = self.header_element() {
            header.update_text();
        }
        for group in self.groups.iter() {
            group
                .expect("null RibbonGroup during strings refresh")
                .on_strings_changed();
        }
    }
    pub fn ribbon(&self) -> Option<Ref<Ribbon>> {
        self.ribbon.borrow().as_ref().and_then(WeakRef::upgrade)
    }
    /// The live generated header. The ribbon retains it while this tab belongs
    /// to `Tabs`, including contextual hiding. Keep the returned handle to retain
    /// a detached presentation; the tab's lookup cache itself is weak.
    pub fn header_element(&self) -> Option<Ref<RibbonTabHeader>> {
        self.header_element
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade)
    }
    pub(crate) fn set_header_element(&self, h: &Ref<RibbonTabHeader>) {
        self.header_element.replace(Some(h.downgrade()));
    }
    fn update_header_automation_id(&self) {
        if let Some(header) = self.header_element() {
            header.update_automation_id();
        }
    }
    pub(crate) fn attach_to_ribbon(&self, ribbon: &Ribbon) {
        self.ribbon.replace(Some(ribbon.to_ref().downgrade()));
        super::RibbonItemHelper::set_owner(self, Some(ribbon.item_owner())).expect("tab owner");
        super::RibbonItemHelper::set_owner(&self.overflow_button, Some(ribbon.item_owner()))
            .expect("overflow owner");
        for group in self.groups.iter() {
            let group = group.expect("null RibbonGroup");
            group.set_tab(self.to_ref());
            group.attach_to_ribbon(ribbon);
        }
    }
    property!(header, set_header, header_property, Option<String>);
    property!(id, set_id, id_property, Option<String>);
    property!(key_tip, set_key_tip, key_tip_property, Option<String>);
    property!(
        is_tab_visible,
        set_is_tab_visible,
        is_tab_visible_property,
        bool
    );
    property!(
        contextual_group_id,
        set_contextual_group_id,
        contextual_group_id_property,
        Option<String>
    );
    property!(is_selected, set_is_selected, is_selected_property, bool);
    property!(
        screen_tip,
        set_screen_tip,
        screen_tip_property,
        Option<BoxedValue>
    );
    property!(icon, set_icon, icon_property, Option<BoxedValue>);
    pub(crate) fn effective_id(&self) -> Option<String> {
        self.id()
            .filter(|s| !s.is_empty())
            .or_else(|| self.name().map(|s| s.to_string()).filter(|s| !s.is_empty()))
            .or_else(|| self.header())
    }
    pub fn is_contextual(&self) -> bool {
        self.contextual_group_id().is_some_and(|s| !s.is_empty())
    }
    pub fn contextual_group(&self) -> Option<Ref<RibbonContextualTabGroup>> {
        let id = self.contextual_group_id()?;
        self.ribbon()?.find_contextual_group(&id)
    }
    pub(crate) fn hidden_by_customization(&self) -> bool {
        self.hidden_by_customization.get()
    }
    pub(crate) fn set_hidden_by_customization(&self, value: bool) {
        self.hidden_by_customization.set(value);
        if let Some(r) = self.ribbon() {
            r.on_tabs_changed();
        }
    }
    pub fn is_available(&self) -> bool {
        self.is_tab_visible()
            && !self.hidden_by_customization.get()
            && (!self.is_contextual() || self.contextual_group().is_some_and(|g| g.is_visible()))
    }
    pub(super) fn sync_groups(&self) {
        let overflow = self.groups_panel.overflow_button();
        self.groups_panel.set_overflow_button(None);
        self.groups_panel.children().clear();
        for group in self.display_groups().iter() {
            let group = group.expect("null RibbonGroup");
            if let Some(parent) = group.visual_parent().and_then(|p| p.cast::<Panel>()) {
                parent.children().remove(group.clone());
            }
            group.set_tab(self.to_ref());
            if let Some(ribbon) = self.ribbon() {
                group.attach_to_ribbon(&ribbon);
            }
            self.groups_panel.children().add(group.clone());
            group.apply_template();
        }
        self.groups_panel.set_overflow_button(overflow);
        self.groups_panel.invalidate_cache();
        if let Some(ribbon) = self.ribbon() {
            ribbon.on_groups_changed(self);
        }
    }
    pub fn invalidate_groups_layout(&self) {
        self.groups_panel.invalidate_measure();
        if let Some(host) = self.scroll_host.borrow().clone() {
            host.invalidate_measure()
        }
        self.invalidate_measure();
    }
    pub(crate) fn apply_presentation(
        &self,
        metrics: Arc<RibbonMetrics>,
        simplified: bool,
        adaptive: bool,
        presentation: RibbonPanelPresentation,
        show_captions: bool,
        strategy: RibbonReductionStrategy,
    ) {
        let p = &self.groups_panel;
        let changed = p
            .metrics()
            .as_ref()
            .is_none_or(|m| !Arc::ptr_eq(m, &metrics))
            || p.is_simplified() != simplified
            || p.is_adaptive() != adaptive
            || p.panel_presentation() != presentation
            || p.show_captions() != show_captions
            || p.reduction_strategy() != strategy;
        p.set_reduction_strategy(strategy);
        p.set_metrics(Some(metrics.clone()));
        p.set_is_simplified(simplified);
        p.set_is_adaptive(adaptive);
        p.set_panel_presentation(presentation);
        p.set_show_captions(show_captions);
        self.overflow_button.apply_layout(RibbonItemLayout {
            size: RibbonItemSize::Small,
            metrics: Some(metrics),
            is_simplified: true,
            show_label: Some(false),
            ..Default::default()
        });
        if changed {
            for group in self.groups.iter() {
                group.expect("null RibbonGroup").reset_presentation_state();
            }
            p.invalidate_cache();
            self.invalidate_groups_layout();
        }
    }
}
impl FerroObjectImpl for RibbonTab {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        RibbonTheme::ensure_resources();
        ferroui_base::input::KeyboardNavigation::set_is_tab_stop(this, false);
        let weak = this.to_ref().downgrade();
        this.groups.add_collection_changed(Rc::new(move |_| {
            if let Some(tab) = weak.upgrade() {
                tab.sync_groups()
            }
        }));
        let weak = this.to_ref().downgrade();
        this.overflow_button.click(move |_, _| {
            if let Some(tab) = weak.upgrade() {
                ferroui_base::threading::Dispatcher::ui_thread().post_local(
                    move || tab.show_overflow(),
                    ferroui_base::threading::DispatcherPriority::NORMAL,
                );
            }
        });
    }
}
impl StyledElementImpl for RibbonTab {
    fn style_key_override(_this: &Self) -> &'static ferroui_base::TypeInfo {
        <Self as ferroui_base::StaticType>::TYPE
    }
}
impl TemplatedControlImpl for RibbonTab {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        if let Some(host) = this.scroll_host.borrow().clone() {
            host.set_content(None::<Ref<Control>>);
        }
        Self::parent_on_apply_template(this, e);
        let host = e
            .name_scope()
            .find("PART_ScrollHost")
            .and_then(|v| v.cast::<RibbonScrollPanel>());
        this.scroll_host.replace(host.clone());
        if let Some(host) = host {
            host.set_content(this.groups_panel.clone());
        }
        this.sync_groups();
    }
}

#[repr(C)]
pub struct RibbonTabHeader {
    base: Button,
    tab: Ref<RibbonTab>,
    scope: RefCell<Option<NameScopeRef>>,
    generated_automation_id: RefCell<Option<String>>,
}
ferro_class!(RibbonTabHeader:Button);
ferro_class_info!(RibbonTabHeader {markup:{namespace:"FerroRibbon.Controls",constructors:[(Ref<RibbonTab>)=>RibbonTabHeader::new],properties:[Tab:Ref<RibbonTab>{get:RibbonTabHeader::tab}],methods:[fn Select() => RibbonTabHeader::select]}});
ferro_impl_classes!(RibbonTabHeader:VisualImpl,InteractiveImpl,InputElementImpl,LayoutableImpl,ContentControlImpl,ButtonImpl);
ferro_properties! {impl RibbonTabHeader {
 pub fn is_selected_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsSelected",false)}
 pub fn contextual_brush_property()->StyledProperty<Option<Rc<dyn IBrush>>>{FerroProperty::register::<Self,_>("ContextualBrush",None)}
 pub fn is_contextual_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsContextual",false)}
 pub fn contextual_header_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("ContextualHeader",None)}
}}
impl RibbonTabHeader {
    fn static_constructor() {
        macro_rules! states{($($p:ident),*) => {$(Self::$p().changed().add_class_handler::<Self>(|this,_|this.update_states()).detach();)*};}
        states!(contextual_brush_property, is_contextual_property);
        Self::is_selected_property()
            .changed()
            .add_class_handler::<Self>(|this, change| {
                this.update_states();
                let (old, new) = change.get_old_and_new_value::<bool>();
                if new == this.is_selected()
                    && let Some(peer) = ControlAutomationPeer::from_element(this)
                {
                    peer.raise_property_changed_event(
                        SelectionItemPatternIdentifiers::is_selected_property(),
                        Some(Rc::new(old)),
                        Some(Rc::new(new)),
                    );
                }
            })
            .detach();
    }
    pub fn construct(tab: Ref<RibbonTab>) -> Self {
        Self {
            base: Button::construct(),
            tab,
            scope: RefCell::default(),
            generated_automation_id: RefCell::default(),
        }
    }
    pub fn new(tab: Ref<RibbonTab>) -> Ref<Self> {
        instantiate(Self::construct(tab))
    }
    pub fn tab(&self) -> Ref<RibbonTab> {
        self.tab.clone()
    }
    pub fn select(&self) {
        if let Some(r) = self.tab().ribbon() {
            r.select_tab(self.tab());
        }
    }
    property!(is_selected, set_is_selected, is_selected_property, bool);
    property!(
        contextual_brush,
        set_contextual_brush,
        contextual_brush_property,
        Option<Rc<dyn IBrush>>
    );
    property!(
        is_contextual,
        set_is_contextual,
        is_contextual_property,
        bool
    );
    property!(
        contextual_header,
        set_contextual_header,
        contextual_header_property,
        Option<String>
    );
    fn part<T: ferroui_base::ObjectType>(&self, name: &str) -> Option<Ref<T>> {
        self.scope
            .borrow()
            .as_ref()
            .and_then(|s| s.find(name).and_then(|v| v.cast::<T>()))
    }
    fn update_states(&self) {
        let contextual = self.is_contextual() && self.contextual_brush().is_some();
        self.pseudo_classes()
            .set(":ribbon-selected", self.is_selected());
        self.pseudo_classes()
            .set(":ribbon-contextual", self.is_contextual());
        self.pseudo_classes()
            .set(":ribbon-contextual-brush", contextual);
        if contextual && let Some(indicator) = self.part::<Rectangle>("PART_Indicator") {
            indicator.set_fill(self.contextual_brush());
        }
        if let Some(brush) = self.contextual_brush()
            && let Some(band) = self.part::<Rectangle>("PART_ContextualBand")
        {
            band.set_fill(Some(brush));
        }
        if let Some(p) = self.part::<ContentPresenter>("PART_Text") {
            if contextual {
                p.set_foreground(self.contextual_brush());
            } else {
                p.clear_value(ContentPresenter::foreground_property());
            }
        } else if let Some(p) = self.part::<TextBlock>("PART_Text") {
            if contextual {
                p.set_foreground(self.contextual_brush());
            } else {
                p.clear_value(TextBlock::foreground_property());
            }
        }
        self.update_text();
    }
    pub(crate) fn update_text(&self) {
        let header = self.tab().header();
        let same = self
            .content()
            .and_then(|v| v.downcast_ref::<String>().cloned())
            == header;
        if !same {
            self.set_content(header.clone().map(|v| Rc::new(v) as BoxedValue));
        }
        AutomationProperties::set_name(self, header.as_deref().or(Some("")));
        self.update_automation_id();
        if let Some(icon) = self.part::<RibbonIconPresenter>("PART_Icon") {
            icon.set_icon(self.tab().icon());
            icon.set_is_visible(self.tab().icon().is_some());
        }
        let tip = if self.is_contextual() && self.contextual_header().is_some_and(|s| !s.is_empty())
        {
            Some(Rc::new(format!(
                "{} › {}",
                self.contextual_header().unwrap(),
                self.tab().header().unwrap_or_default()
            )) as BoxedValue)
        } else {
            self.tab().screen_tip()
        };
        ToolTip::set_tip(self, tip);
    }
    fn update_automation_id(&self) {
        // Compiled XAML can add the tab before setting its Id and Header.
        // Revise the generated value without taking ownership of an application ID.
        let current = AutomationProperties::get_automation_id(self);
        let next = self.tab.effective_id().map(|id| format!("RibbonTab_{id}"));
        if current != next && current == *self.generated_automation_id.borrow() {
            self.generated_automation_id.replace(next.clone());
            AutomationProperties::set_automation_id(self, next.as_deref());
        }
    }
}
impl FerroObjectImpl for RibbonTabHeader {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_content(this.tab().header().map(|s| Rc::new(s) as BoxedValue));
        AutomationProperties::set_name(this, this.tab().header().as_deref().or(Some("")));
        this.update_automation_id();
    }
}
impl StyledElementImpl for RibbonTabHeader {
    fn style_key_override(_this: &Self) -> &'static ferroui_base::TypeInfo {
        <Self as ferroui_base::StaticType>::TYPE
    }
}
impl TemplatedControlImpl for RibbonTabHeader {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        this.scope.replace(Some(e.name_scope().clone()));
        this.update_states();
    }
}

#[repr(C)]
pub struct RibbonContextualTabGroup {
    base: FerroObject,
}
ferro_class!(RibbonContextualTabGroup:FerroObject);
ferro_class_info!(RibbonContextualTabGroup{new:RibbonContextualTabGroup::new,markup:{namespace:"FerroRibbon.Controls"}});
ferro_impl_classes!(RibbonContextualTabGroup:FerroObjectImpl);
ferro_properties! {impl RibbonContextualTabGroup {
 pub fn id_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Id",None)}
 pub fn header_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Header",None)}
 pub fn color_property()->StyledProperty<Color>{FerroProperty::register::<Self,_>("Color",Color::from_argb(255,15,108,189))}
 pub fn is_visible_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsVisible",false)}
 pub fn activation_property()->StyledProperty<RibbonContextualActivation>{FerroProperty::register::<Self,_>("Activation",RibbonContextualActivation::None)}
 pub fn key_tip_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("KeyTip",None)}
 pub fn is_enabled_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsEnabled",true)}
}}
impl RibbonContextualTabGroup {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: FerroObject::construct(),
        })
    }
    property!(id, set_id, id_property, Option<String>);
    property!(header, set_header, header_property, Option<String>);
    property!(color, set_color, color_property, Color);
    property!(is_visible, set_is_visible, is_visible_property, bool);
    property!(
        activation,
        set_activation,
        activation_property,
        RibbonContextualActivation
    );
    property!(key_tip, set_key_tip, key_tip_property, Option<String>);
    property!(is_enabled, set_is_enabled, is_enabled_property, bool);
}

impl Drop for RibbonTab {
    fn drop(&mut self) {
        for flyout in self.open_overflows.get_mut().drain(..) {
            flyout.hide();
        }
    }
}

impl ControlImpl for RibbonTab {
    fn on_create_automation_peer(
        this: &Self,
    ) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        ferroui_controls::automation::peers::ControlAutomationPeer::new(this).upcast()
    }
}
impl ControlImpl for RibbonTabHeader {
    fn on_create_automation_peer(
        this: &Self,
    ) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        super::RibbonTabHeaderAutomationPeer::new(this).upcast()
    }
}
