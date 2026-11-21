//! Classic group composition from shared RibbonGroup.cs.
use super::{
    IRibbonItem, IRibbonLayoutHost, RibbonButton, RibbonItemLayout, RibbonScreenTip,
    RibbonScreenTipService,
};
use crate::{
    metadata::control_items::RibbonControlItems,
    primitives::{RibbonGroupItemsPanel, RibbonMetrics},
    theming::RibbonTheme,
};
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::{
    RibbonGroupItemsLayout, RibbonGroupState, RibbonItemSize, RibbonSimplifiedVisibility,
    collections::RibbonReadOnlyList,
};
use ferroui_base::{
    BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref, StaticType,
    StyledElementImpl, StyledProperty, Thickness, TypeInfo, Visual, VisualImpl,
    controls::NameScopeRef,
    data::core::{ValueType, ValueTypes},
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{ICommand, InputElementImpl, KeyboardNavigation},
    instantiate,
    interactivity::{InteractiveImpl, RoutedEventHandlerToken},
    layout::{LayoutableImpl, LayoutableImplExt},
};
use ferroui_controls::{
    Border, Button, Control, ControlImpl, Panel, ToolTip,
    automation::AutomationProperties,
    primitives::{
        TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
#[path = "ribbon_group_floating.rs"]
mod floating;
#[path = "ribbon_group_popup.rs"]
mod popup;
use popup::GroupPopup;
#[path = "ribbon_group_slide_out.rs"]
mod slide_out;
#[path = "ribbon_group_traversal.rs"]
mod traversal;

#[derive(Clone)]
struct AppliedState {
    state: RibbonGroupState,
    simplified: bool,
    metrics: Arc<RibbonMetrics>,
    mask: Option<RibbonReadOnlyList<bool>>,
}
#[derive(Default)]
struct Parts {
    presenter: Option<Ref<Border>>,
    slide_out_button: Option<Ref<Button>>,
    slide_out_handler: Option<RoutedEventHandlerToken>,
    caption: Option<Ref<Control>>,
    caption_handlers: Vec<RoutedEventHandlerToken>,
    launcher: Option<Ref<Button>>,
    collapsed: Option<Ref<RibbonButton>>,
    launcher_handler: Option<RoutedEventHandlerToken>,
    collapsed_handler: Option<RoutedEventHandlerToken>,
}
#[repr(C)]
pub struct RibbonGroup {
    base: TemplatedControl,
    items: RibbonControlItems,
    slide_out_items: RibbonControlItems,
    slide_out_panel: Ref<ferroui_controls::StackPanel>,
    items_panel: Ref<RibbonGroupItemsPanel>,
    metrics: RefCell<Arc<RibbonMetrics>>,
    applied_state: RefCell<Option<AppliedState>>,
    layout_version: Cell<i32>,
    visibility_tokens: RefCell<Vec<(Ref<Control>, u64)>>,
    parts: RefCell<Parts>,
    scope: RefCell<Option<NameScopeRef>>,
    hidden_by_customization: Cell<bool>,
    pub(crate) original_header: RefCell<Option<String>>,
    is_custom: Cell<bool>,
    popup: RefCell<Option<Rc<GroupPopup>>>,
    slide_out: RefCell<Option<Rc<slide_out::SlideOut>>>,
    floating: floating::FloatingState,
    dialog_launcher_click: RefCell<Option<ferroui_base::metadata::MarkupDelegate>>,
    panel_presentation: Cell<super::RibbonPanelPresentation>,
    tab: RefCell<Option<ferroui_base::WeakRef<super::RibbonTab>>>,
}
ferro_class!(RibbonGroup:TemplatedControl);
ferro_class_info!(RibbonGroup {
    new:RibbonGroup::new,
    markup:{namespace:"FerroRibbon.Controls",content:Items,properties:[
        Tab:Option<Ref<super::RibbonTab>>{get:RibbonGroup::tab},
        Items:RibbonControlItems{get:RibbonGroup::items},
        SlideOutItems:RibbonControlItems{get:RibbonGroup::slide_out_items},
        Ribbon:Option<Ref<super::Ribbon>>{get:RibbonGroup::ribbon},
        ItemsPanel:Ref<RibbonGroupItemsPanel>{get:RibbonGroup::items_panel},
        Metrics:Arc<RibbonMetrics>{get:RibbonGroup::metrics},
        LayoutVersion:i32{get:RibbonGroup::layout_version},
        PanelPresentation:super::RibbonPanelPresentation{get:RibbonGroup::panel_presentation},
        IsPopupOpen:bool{get:RibbonGroup::is_popup_open},
        FloatingPosition:ferroui_base::Point{get:RibbonGroup::floating_position,set:RibbonGroup::set_floating_position},
        IsFloatingPanelOpen:bool{get:RibbonGroup::is_floating_panel_open},
        IsSlideOutOpen:bool{get:RibbonGroup::is_slide_out_open},
        SlideOutButton:Option<Ref<Button>>{get:RibbonGroup::slide_out_button},
        CollapsedButton:Option<Ref<RibbonButton>>{get:RibbonGroup::collapsed_button},
        DialogLauncher:Option<Ref<Button>>{get:RibbonGroup::dialog_launcher},
    ],methods:[fn Float()=>|g:&RibbonGroup|g.float(None),fn Float(Option<ferroui_base::Point>)=>RibbonGroup::float,fn ReturnToRibbon()=>RibbonGroup::return_to_ribbon,fn GetAllItems()->ferroribbon_core::model::RibbonItemsSource<Ref<Control>> => RibbonGroup::get_all_items,fn InvalidateItemsLayout()=>RibbonGroup::invalidate_items_layout,
        fn OpenSlideOut()=>RibbonGroup::open_slide_out,fn CloseSlideOut()=>RibbonGroup::close_slide_out,
        fn OpenPopup()=>RibbonGroup::open_popup,fn ClosePopup()=>RibbonGroup::close_popup,
        try fn OpenDialogLauncher()=>RibbonGroup::open_dialog_launcher],
        events:[FloatingChanged(Option<BoxedValue>,ferroui_base::utilities::EventArgs){add:RibbonGroup::add_floating_changed,remove:RibbonGroup::remove_floating_changed},DialogLauncherClick(Option<BoxedValue>,ferroui_base::interactivity::RoutedEventArgs){add:RibbonGroup::add_dialog_launcher_click,remove:RibbonGroup::remove_dialog_launcher_click}],
    },
    interfaces:[Rc<dyn IRibbonLayoutHost> => |value| Rc::new(GroupHost(value)) as Rc<dyn IRibbonLayoutHost>],
});
ferro_impl_classes!(RibbonGroup:VisualImpl,InteractiveImpl,InputElementImpl);
ferro_properties! {impl RibbonGroup {
    pub fn is_floating_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsFloating",false)}
    pub fn is_slide_out_pinned_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsSlideOutPinned",false)}
    pub fn header_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Header",None)}
    pub fn icon_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("Icon",None)}
    pub fn id_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Id",None)}
    pub fn key_tip_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("KeyTip",None)}
    pub fn screen_tip_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("ScreenTip",None)}
    pub fn dialog_launcher_command_property()->StyledProperty<Option<Rc<dyn ICommand>>>{FerroProperty::register::<Self,_>("DialogLauncherCommand",None)}
    pub fn dialog_launcher_command_parameter_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("DialogLauncherCommandParameter",None)}
    pub fn is_dialog_launcher_visible_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsDialogLauncherVisible",false)}
    pub fn dialog_launcher_screen_tip_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("DialogLauncherScreenTip",None)}
    pub fn reduction_order_property()->StyledProperty<i32>{FerroProperty::register::<Self,_>("ReductionOrder",0)}
    pub fn can_collapse_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("CanCollapse",true)}
    pub fn items_layout_property()->StyledProperty<RibbonGroupItemsLayout>{FerroProperty::register::<Self,_>("ItemsLayout",RibbonGroupItemsLayout::Columns)}
    pub fn row_count_property()->StyledProperty<i32>{FerroProperty::register::<Self,_>("RowCount",3)}
    pub fn state_property()->StyledProperty<RibbonGroupState>{FerroProperty::register::<Self,_>("State",RibbonGroupState::Large)}
    pub fn is_simplified_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsSimplified",false)}
    pub fn simplified_visibility_property()->StyledProperty<RibbonSimplifiedVisibility>{FerroProperty::register::<Self,_>("SimplifiedVisibility",RibbonSimplifiedVisibility::Auto)}
    pub fn is_group_visible_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsGroupVisible",true)}
}}
macro_rules! property {
    ($get:ident,$set:ident,$id:ident,$ty:ty) => {
        pub fn $get(&self) -> $ty {
            self.get_value(Self::$id())
        }
        pub fn $set(&self, value: $ty) {
            self.set_value(Self::$id(), value)
        }
    };
}
impl RibbonGroup {
    fn static_constructor() {
        Self::is_floating_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.on_is_floating_changed())
            .detach();
        Self::is_slide_out_pinned_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.on_slide_out_pinned_changed())
            .detach();
        Self::header_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.on_header_changed())
            .detach();
        Self::screen_tip_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.on_header_changed())
            .detach();
        Self::dialog_launcher_command_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.update_launcher())
            .detach();
        Self::is_dialog_launcher_visible_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.update_launcher())
            .detach();
        Self::dialog_launcher_screen_tip_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.update_launcher())
            .detach();
        macro_rules! invalidate {($($p:ident),*)=>{$(Self::$p().changed().add_class_handler::<Self>(|this,_|this.invalidate_items_layout()).detach();)*};}
        invalidate!(
            reduction_order_property,
            can_collapse_property,
            items_layout_property,
            row_count_property,
            simplified_visibility_property
        );
        Self::is_group_visible_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.update_visibility())
            .detach();
    }
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            items: RibbonControlItems::new_managed(),
            slide_out_items: RibbonControlItems::new_managed(),
            slide_out_panel: {
                let p = ferroui_controls::StackPanel::new();
                p.set_spacing(1.);
                p
            },
            items_panel: RibbonGroupItemsPanel::new(),
            metrics: RefCell::new(RibbonMetrics::comfortable()),
            applied_state: RefCell::new(None),
            layout_version: Cell::new(0),
            visibility_tokens: RefCell::default(),
            parts: RefCell::default(),
            scope: RefCell::default(),
            hidden_by_customization: Cell::new(false),
            original_header: RefCell::default(),
            is_custom: Cell::new(false),
            popup: RefCell::default(),
            slide_out: RefCell::default(),
            floating: floating::FloatingState::default(),
            dialog_launcher_click: RefCell::default(),
            panel_presentation: Cell::new(super::RibbonPanelPresentation::Full),
            tab: RefCell::default(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn tab(&self) -> Option<Ref<super::RibbonTab>> {
        self.tab
            .borrow()
            .as_ref()
            .and_then(ferroui_base::WeakRef::upgrade)
    }
    pub(crate) fn set_tab(&self, tab: Ref<super::RibbonTab>) {
        self.tab.replace(Some(tab.downgrade()));
    }
    pub fn ribbon(&self) -> Option<Ref<super::Ribbon>> {
        self.tab().and_then(|t| t.ribbon())
    }
    pub(crate) fn attach_to_ribbon(&self, ribbon: &super::Ribbon) {
        for item in self.items.iter() {
            let item = item.expect("null group item owner target");
            super::RibbonItemHelper::set_owner(&item, Some(ribbon.item_owner()))
                .expect("group item owner");
        }
        super::RibbonItemHelper::set_owner(self, Some(ribbon.item_owner())).expect("group owner");
    }
    pub fn slide_out_items(&self) -> RibbonControlItems {
        self.slide_out_items.clone()
    }
    pub fn items(&self) -> RibbonControlItems {
        self.items.clone()
    }
    pub fn items_panel(&self) -> Ref<RibbonGroupItemsPanel> {
        self.items_panel.clone()
    }
    pub fn metrics(&self) -> Arc<RibbonMetrics> {
        self.metrics.borrow().clone()
    }
    pub fn layout_version(&self) -> i32 {
        self.layout_version.get()
    }
    property!(header, set_header, header_property, Option<String>);
    property!(icon, set_icon, icon_property, Option<BoxedValue>);
    property!(id, set_id, id_property, Option<String>);
    property!(key_tip, set_key_tip, key_tip_property, Option<String>);
    property!(
        screen_tip,
        set_screen_tip,
        screen_tip_property,
        Option<BoxedValue>
    );
    property!(
        dialog_launcher_command,
        set_dialog_launcher_command,
        dialog_launcher_command_property,
        Option<Rc<dyn ICommand>>
    );
    property!(
        dialog_launcher_command_parameter,
        set_dialog_launcher_command_parameter,
        dialog_launcher_command_parameter_property,
        Option<BoxedValue>
    );
    property!(
        is_dialog_launcher_visible,
        set_is_dialog_launcher_visible,
        is_dialog_launcher_visible_property,
        bool
    );
    property!(
        dialog_launcher_screen_tip,
        set_dialog_launcher_screen_tip,
        dialog_launcher_screen_tip_property,
        Option<BoxedValue>
    );
    property!(
        reduction_order,
        set_reduction_order,
        reduction_order_property,
        i32
    );
    property!(can_collapse, set_can_collapse, can_collapse_property, bool);
    property!(
        items_layout,
        set_items_layout,
        items_layout_property,
        RibbonGroupItemsLayout
    );
    property!(row_count, set_row_count, row_count_property, i32);
    property!(
        simplified_visibility,
        set_simplified_visibility,
        simplified_visibility_property,
        RibbonSimplifiedVisibility
    );
    property!(
        is_group_visible,
        set_is_group_visible,
        is_group_visible_property,
        bool
    );
    pub fn state(&self) -> RibbonGroupState {
        self.get_value(Self::state_property())
    }
    pub fn is_simplified(&self) -> bool {
        self.get_value(Self::is_simplified_property())
    }
    pub fn dialog_launcher(&self) -> Option<Ref<Button>> {
        self.parts.borrow().launcher.clone()
    }
    pub fn collapsed_button(&self) -> Option<Ref<RibbonButton>> {
        self.parts.borrow().collapsed.clone()
    }
    /// Deferred indexed traversal from the source yield method. Reads the live
    /// panel Count/mask/visibility on each step, without materializing its children.
    pub fn get_overflow_items(&self) -> ferroribbon_core::model::RibbonItemsSource<Ref<Control>> {
        let group = self.to_ref();
        ferroribbon_core::model::RibbonItemsSource::deferred(move || {
            let group = group.clone();
            let mut index = 0;
            Box::new(std::iter::from_fn(move || {
                while index < group.items_panel.children().count() {
                    let current = index;
                    let child = group.items_panel.children().get(index);
                    index += 1;
                    if child.is_visible()
                        && (!group.items_panel.is_in_line(current as i32)
                            || Self::as_item(&child).is_some_and(|item| {
                                item.simplified_visibility() == RibbonSimplifiedVisibility::Overflow
                            }))
                    {
                        return Some(child);
                    }
                }
                None
            }))
        })
    }
    #[doc(hidden)]
    pub fn measure_width(
        &self,
        state: RibbonGroupState,
        metrics: Arc<RibbonMetrics>,
        height: f64,
    ) -> f64 {
        self.apply_state(state, false, metrics, None);
        Self::prepare_synchronous_measure(self);
        self.measure(ferroui_base::Size::new(f64::INFINITY, height));
        self.desired_size().width
    }
    #[doc(hidden)]
    pub fn measure_simplified_item_widths(&self, metrics: Arc<RibbonMetrics>) -> Vec<f64> {
        self.apply_state(RibbonGroupState::Large, true, metrics, None);
        let mut widths = vec![0.; self.items_panel.children().count()];
        for (index, width) in widths.iter_mut().enumerate() {
            let child = self.items_panel.children().get(index);
            if !child.is_visible() {
                continue;
            }
            Self::prepare_synchronous_measure(&child);
            child.measure(ferroui_base::Size::INFINITY);
            *width = child.desired_size().width + self.items_panel.simplified_spacing();
        }
        widths
    }
    // WinUI's synchronous state probes see changes in nested template parts.
    // FerroUI queues a dirty child and invalidates its parent only after the
    // child's desired size changes during a layout pass. A direct Measure on
    // the group otherwise reuses valid intermediate template containers.
    // Reconcile only dirty descendant paths, without running the dispatcher or
    // changing the source layout-version/cache/reduction algorithm.
    fn prepare_synchronous_measure(visual: &ferroui_base::Visual) -> bool {
        let mut dirty_child = false;
        if let Some(children) = visual.visual_children_snapshot() {
            for child in children.iter() {
                dirty_child |= Self::prepare_synchronous_measure(child);
            }
        }
        if let Some(layout) = visual.downcast_ref::<ferroui_base::layout::Layoutable>() {
            if dirty_child {
                layout.invalidate_measure();
            }
            !layout.is_measure_valid()
        } else {
            dirty_child
        }
    }
    #[doc(hidden)]
    pub fn is_custom(&self) -> bool {
        self.is_custom.get()
    }
    #[doc(hidden)]
    pub fn set_is_custom(&self, value: bool) {
        self.is_custom.set(value);
    }
    pub(crate) fn effective_id(&self) -> Option<String> {
        self.id()
            .filter(|s| !s.is_empty())
            .or_else(|| self.name().map(|s| s.to_string()).filter(|s| !s.is_empty()))
            .or_else(|| self.header())
    }
    pub(crate) fn is_shown(&self) -> bool {
        self.is_visible()
    }
    #[cfg(test)]
    pub(crate) fn hidden_by_customization(&self) -> bool {
        self.hidden_by_customization.get()
    }
    pub(crate) fn set_hidden_by_customization(&self, value: bool) {
        self.hidden_by_customization.set(value);
        self.update_visibility();
    }
    fn update_visibility(&self) {
        self.set_is_visible(
            self.is_group_visible() && !self.hidden_by_customization.get() && !self.is_floating(),
        );
        if let Some(tab) = self.tab() {
            tab.invalidate_groups_layout();
        }
    }
    fn as_item(child: &Control) -> Option<Rc<dyn IRibbonItem>> {
        let value = Control::boxed(child.to_ref());
        ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IRibbonItem>>())
            .and_then(|value| value.downcast_ref::<Rc<dyn IRibbonItem>>().cloned())
    }
    fn on_items_changed(&self) {
        self.sync_items();
        self.on_header_changed()
    }
    fn sync_items(&self) {
        let removed = self
            .visibility_tokens
            .borrow()
            .iter()
            .filter(|(item, _)| !self.items.contains(&Some(item.clone())))
            .cloned()
            .collect::<Vec<_>>();
        for (item, token) in removed {
            item.remove_property_changed_handler(token);
            self.visibility_tokens
                .borrow_mut()
                .retain(|(current, _)| !current.ptr_eq(&item));
        }
        let stale = self
            .items_panel
            .children()
            .snapshot()
            .iter()
            .filter(|child| !self.items.contains(&Some((*child).clone())))
            .cloned()
            .collect::<Vec<_>>();
        for child in stale {
            self.items_panel.children().remove(child);
        }
        let mut i = 0;
        // Source uses an indexed live Count loop, not an ObservableCollection enumerator.
        while i < self.items.count() {
            let item = self
                .items
                .get(i)
                .expect("ArgumentNull: RibbonGroup.Items child");
            let index = self.items_panel.children().index_of(&item);
            if index != Some(i) {
                if let Some(index) = index {
                    self.items_panel.children().remove_at(index);
                } else if let Some(parent) = item.visual_parent().and_then(|p| p.cast::<Panel>()) {
                    parent.children().remove(item.clone());
                }
                self.items_panel
                    .children()
                    .insert(i.min(self.items_panel.children().count()), item.clone());
            }
            if let Some(ribbon) = self.ribbon() {
                super::RibbonItemHelper::set_owner(&item, Some(ribbon.item_owner()))
                    .expect("group item owner");
            }
            if !self
                .visibility_tokens
                .borrow()
                .iter()
                .any(|(child, _)| child.ptr_eq(&item))
            {
                let weak = self.to_ref().downgrade();
                let token = item.add_property_changed_handler(Rc::new(move |e| {
                    if e.property() == Visual::is_visible_property().as_property()
                        && let Some(group) = weak.upgrade()
                    {
                        group.invalidate_items_layout();
                    }
                }));
                self.visibility_tokens.borrow_mut().push((item, token));
            }
            i += 1;
        }
        drop(self.applied_state.replace(None));
        if let Some(ribbon) = self.ribbon() {
            ribbon.invalidate_shortcuts();
        }
        self.invalidate_items_layout();
    }
    pub fn invalidate_items_layout(&self) {
        self.layout_version
            .set(self.layout_version.get().wrapping_add(1));
        self.invalidate_measure();
        self.items_panel.invalidate_measure();
        if let Some(tab) = self.tab() {
            tab.invalidate_groups_layout();
        }
    }
    fn on_header_changed(&self) {
        if let Some(button) = self.collapsed_button() {
            button.set_label(self.header());
            let button = self.collapsed_button();
            let icon = if self.panel_presentation() == super::RibbonPanelPresentation::Titles {
                None
            } else {
                self.icon()
                    .or_else(|| {
                        self.items
                            .iter()
                            .flatten()
                            .filter_map(|child| Self::as_item(&child))
                            .find(|item| item.icon().is_some())
                            .and_then(|item| item.icon())
                    })
                    .or_else(|| Some(Rc::new("\u{e712}".to_string())))
            };
            button
                .expect("CollapsedButton became null during Label notification")
                .set_icon(icon);
            let button = self.collapsed_button();
            let tip = self.screen_tip();
            button
                .expect("CollapsedButton became null during Icon notification")
                .set_screen_tip(tip);
        }
        AutomationProperties::set_name(self, Some(self.header().as_deref().unwrap_or("")));
        let help = self.screen_tip().and_then(|tip| {
            if let Some(text) = tip.downcast_ref::<String>() {
                return Some(text.clone());
            }
            if let Some(control) =
                ValueTypes::as_object(&*tip).and_then(|o| o.cast::<RibbonScreenTip>())
            {
                return control.description().or_else(|| control.title());
            }
            if let Some(model) = tip.downcast_ref::<Rc<ferroribbon_core::model::RibbonScreenTip>>()
            {
                return model.description().or_else(|| model.title()).map(|s| {
                    String::from_utf16(s.as_utf16())
                        .expect("ScreenTip contains unpaired UTF-16 surrogates")
                });
            }
            None
        });
        if let Some(help) = help.filter(|help| !help.is_empty()) {
            AutomationProperties::set_help_text(self, Some(&help));
        } else {
            self.clear_value(AutomationProperties::help_text_property());
        }
        self.update_launcher();
        self.invalidate_items_layout();
    }
    fn launcher_title(&self) -> String {
        use ferroribbon_core::localization::{RibbonFormatArgument, RibbonStrings};
        RibbonStrings::current()
            .format(
                Some(&"DialogLauncher".into()),
                Some(RibbonFixedItems::new(vec![RibbonFormatArgument::from(
                    self.header().as_deref().unwrap_or(""),
                )])),
            )
            .map(|s| {
                String::from_utf16(s.as_utf16())
                    .expect("DialogLauncher contains unpaired UTF-16 surrogates")
            })
            .expect("DialogLauncher localization format")
    }
    fn create_launcher_tool_tip(&self) -> Option<Ref<ToolTip>> {
        let title = self.launcher_title();
        let tip = self
            .dialog_launcher_screen_tip()
            .unwrap_or_else(|| Rc::new(title.clone()));
        let tip = if tip.downcast_ref::<String>().is_some() {
            None
        } else {
            Some(tip)
        };
        RibbonScreenTipService::create_tool_tip(
            RibbonScreenTipService::create(Some(title), tip, None)
                .expect("launcher ScreenTip conversion"),
        )
    }
    pub(crate) fn on_strings_changed(&self) {
        self.update_launcher();
    }
    fn update_launcher(&self) {
        let Some(launcher) = self.dialog_launcher() else {
            return;
        };
        launcher.set_is_visible(
            self.is_dialog_launcher_visible() || self.dialog_launcher_command().is_some(),
        );
        let launcher = self.dialog_launcher();
        let tip = self.create_launcher_tool_tip().map(Control::boxed);
        ToolTip::set_tip(
            &launcher.expect("DialogLauncher became null before SetToolTip"),
            tip,
        );
        let launcher = self.dialog_launcher();
        let title = self.launcher_title();
        AutomationProperties::set_name(
            &launcher.expect("DialogLauncher became null before SetName"),
            Some(&title),
        );
    }
    /// Source internal layout entry point, exposed to native Rust hosts.
    #[doc(hidden)]
    pub fn apply_state(
        &self,
        state: RibbonGroupState,
        simplified: bool,
        metrics: Arc<RibbonMetrics>,
        mask: Option<RibbonReadOnlyList<bool>>,
    ) {
        let current = self.applied_state.borrow().clone();
        if current.as_ref().is_some_and(|current| {
            current.state == state
                && current.simplified == simplified
                && Arc::ptr_eq(&current.metrics, &metrics)
                && mask_equals(current.mask.as_ref(), mask.as_ref())
        }) {
            return;
        }
        drop(self.applied_state.replace(Some(AppliedState {
            state,
            simplified,
            metrics: metrics.clone(),
            mask: mask.clone(),
        })));
        drop(self.metrics.replace(metrics.clone()));
        self.set_value(Self::state_property(), state);
        self.set_value(Self::is_simplified_property(), simplified);
        self.items_panel.set_metrics(Some(metrics.clone()));
        self.items_panel.set_is_simplified(simplified);
        self.items_panel.set_items_layout(self.items_layout());
        self.items_panel.set_row_count(self.row_count());
        self.items_panel.set_in_line_mask(mask);
        let item_state = if state == RibbonGroupState::Collapsed {
            RibbonGroupState::Large
        } else {
            state
        };
        if !self.is_popup_open() {
            self.apply_item_layouts(item_state, simplified, metrics);
        }
        self.apply_collapsed_button_layout();
        self.apply_presentation();
        self.items_panel.invalidate_measure();
        self.invalidate_measure();
        Self::prepare_synchronous_measure(self);
    }
    fn apply_item_layouts(
        &self,
        state: RibbonGroupState,
        simplified: bool,
        metrics: Arc<RibbonMetrics>,
    ) {
        for item in self
            .items
            .iter()
            .flatten()
            .filter_map(|child| Self::as_item(&child))
        {
            let size = if simplified {
                RibbonItemSize::Small
            } else {
                item.effective_size_definition().get_size(state)
            };
            item.apply_layout(RibbonItemLayout {
                size,
                metrics: Some(metrics.clone()),
                is_simplified: simplified,
                show_label: None,
                group_state: state,
            });
        }
    }
    fn apply_collapsed_button_layout(&self) {
        if let Some(button) = self.collapsed_button() {
            button.apply_layout(
                if self.panel_presentation() == super::RibbonPanelPresentation::Titles {
                    RibbonItemLayout {
                        show_label: Some(true),
                        ..RibbonItemLayout::new(RibbonItemSize::Medium, Some(self.metrics()))
                    }
                } else {
                    RibbonItemLayout::new(RibbonItemSize::Large, Some(self.metrics()))
                },
            );
        }
    }
    pub fn panel_presentation(&self) -> super::RibbonPanelPresentation {
        self.panel_presentation.get()
    }
    /// Native Rust entry point corresponding to the source internal setter.
    #[doc(hidden)]
    pub fn set_panel_presentation(&self, value: super::RibbonPanelPresentation) {
        if self.panel_presentation() == value {
            return;
        }
        self.panel_presentation.set(value);
        drop(self.applied_state.replace(None));
        self.on_header_changed();
        self.apply_collapsed_button_layout();
    }
    #[doc(hidden)]
    pub fn reset_presentation_state(&self) {
        drop(self.applied_state.replace(None));
    }
    fn apply_presentation(&self) {
        let collapsed = self.state() == RibbonGroupState::Collapsed && !self.is_simplified();
        let (presenter, button, caption) = {
            let parts = self.parts.borrow();
            (
                parts.presenter.clone(),
                parts.collapsed.clone(),
                parts.caption.clone(),
            )
        };
        if let Some(presenter) = presenter {
            presenter.set_is_visible(!collapsed);
            presenter.set_margin(if self.is_simplified() {
                Thickness::new(2., 0., 2., 0.)
            } else {
                Thickness::new(4., 2., 4., 0.)
            });
        }
        if let Some(button) = button {
            button.set_is_visible(collapsed);
        }
        if let Some(caption) = caption {
            caption.set_is_visible(
                !self.is_simplified()
                    && !collapsed
                    && self
                        .tab()
                        .and_then(|t| t.ribbon())
                        .is_none_or(|r| r.show_group_captions()),
            );
            caption.set_height(self.metrics().group_caption_height());
        }
        self.update_slide_out_button();
        self.pseudo_classes()
            .set(":simplified", self.is_simplified());
        self.pseudo_classes().set(":collapsed", collapsed);
        self.pseudo_classes()
            .set(":expanded", !self.is_simplified() && !collapsed);
    }
    fn detach_parts(&self) {
        let previous = std::mem::take(&mut *self.parts.borrow_mut());
        Self::detach_caption_drag(previous.caption.as_deref(), &previous.caption_handlers);
        if let (Some(launcher), Some(token)) = (&previous.launcher, previous.launcher_handler) {
            launcher.remove_handler(Button::click_event(), token);
        }
        if let (Some(button), Some(token)) = (&previous.collapsed, previous.collapsed_handler) {
            button.remove_handler(Button::click_event(), token);
        }
        if let (Some(button), Some(token)) =
            (&previous.slide_out_button, previous.slide_out_handler)
        {
            button.remove_handler(Button::click_event(), token);
        }
        if let Some(presenter) = previous.presenter
            && presenter
                .child()
                .is_some_and(|child| child.ptr_eq(&self.items_panel))
        {
            presenter.set_child(None);
        }
    }
}
fn mask_equals(a: Option<&RibbonReadOnlyList<bool>>, b: Option<&RibbonReadOnlyList<bool>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => Rc::ptr_eq(a, b) || a.iter().eq(b.iter()),
        _ => false,
    }
}
impl FerroObjectImpl for RibbonGroup {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        RibbonTheme::ensure_resources();
        this.add_handler(
            ferroui_base::input::InputElement::right_tapped_event(),
            |sender, e| {
                if e.handled() {
                    return;
                }
                let group = sender
                    .to_ref()
                    .cast::<RibbonGroup>()
                    .expect("group context target");
                if let Some(root) = group.ribbon() {
                    e.set_handled(
                        root.show_item_context_menu(
                            group.clone().upcast(),
                            e.get_position(Some(&group)),
                        )
                        .expect("group context menu"),
                    );
                }
            },
        );
        let weak = this.to_ref().downgrade();
        this.items.add_collection_changed(Rc::new(move |_| {
            if let Some(group) = weak.upgrade() {
                group.on_items_changed()
            }
        }));
        let weak = this.to_ref().downgrade();
        this.slide_out_items
            .add_collection_changed(Rc::new(move |_| {
                if let Some(group) = weak.upgrade() {
                    group.on_slide_out_items_changed();
                }
            }));
        KeyboardNavigation::set_is_tab_stop(this, false);
        // Source Custom landmark has no equivalent in this SDK enum.
    }
}
impl StyledElementImpl for RibbonGroup {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <Self as StaticType>::TYPE
    }
}
impl LayoutableImpl for RibbonGroup {
    fn apply_template(this: &Self) {
        Self::parent_apply_template(this);
        if this.visual_children().count() == 0 {
            this.detach_parts();
            this.scope.replace(None);
        }
    }
}
impl TemplatedControlImpl for RibbonGroup {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        this.detach_parts();
        Self::parent_on_apply_template(this, e);
        this.scope.replace(Some(e.name_scope().clone()));
        let find = |name| e.name_scope().find(name).and_then(|p| p.cast::<Control>());
        let mut parts = Parts {
            slide_out_button: find("PART_SlideOutButton").and_then(|p| p.cast()),
            slide_out_handler: None,
            presenter: find("PART_ItemsPresenter").and_then(|p| p.cast()),
            caption: find("PART_CaptionRow"),
            caption_handlers: Vec::new(),
            launcher: find("PART_DialogLauncher").and_then(|p| p.cast()),
            collapsed: find("PART_CollapsedButton").and_then(|p| p.cast()),
            launcher_handler: None,
            collapsed_handler: None,
        };
        if let Some(presenter) = &parts.presenter
            && !this.panel_in_popup()
            && !this.is_floating()
        {
            presenter.set_child(this.items_panel.clone());
        }
        if let Some(launcher) = &parts.launcher {
            let weak = this.to_ref().downgrade();
            parts.launcher_handler = Some(launcher.click(move |_, _| {
                if let Some(group) = weak.upgrade() {
                    group
                        .open_dialog_launcher()
                        .expect("DialogLauncherClick handler");
                }
            }));
        }
        if let Some(button) = &parts.collapsed {
            let weak = this.to_ref().downgrade();
            parts.collapsed_handler = Some(button.click(move |_, _| {
                let weak = weak.clone();
                ferroui_base::threading::Dispatcher::ui_thread().post_local(
                    move || {
                        if let Some(group) = weak.upgrade() {
                            group.open_popup()
                        }
                    },
                    ferroui_base::threading::DispatcherPriority::NORMAL,
                );
            }));
        }
        if let Some(button) = &parts.slide_out_button {
            let weak = this.to_ref().downgrade();
            parts.slide_out_handler = Some(button.click(move |_, _| {
                if let Some(group) = weak.upgrade() {
                    if group.is_slide_out_open() {
                        group.set_is_slide_out_pinned(false);
                        group.close_slide_out();
                    } else {
                        ferroui_base::threading::Dispatcher::ui_thread().post_local(
                            move || group.open_slide_out(),
                            ferroui_base::threading::DispatcherPriority::NORMAL,
                        );
                    }
                }
            }));
        }
        if let Some(caption) = &parts.caption {
            parts.caption_handlers = this.attach_caption_drag(caption);
        }
        this.parts.replace(parts);
        if let Some(button) = this.collapsed_button() {
            button.set_is_chrome_button(true);
            this.apply_collapsed_button_layout();
        }
        this.sync_items();
        this.update_launcher();
        this.on_header_changed();
        this.apply_presentation();
    }
}
struct GroupHost(Ref<RibbonGroup>);
impl IRibbonLayoutHost for GroupHost {
    fn as_object(&self) -> Ref<ferroui_base::FerroObject> {
        self.0.clone().upcast()
    }
    fn invalidate_items_layout(&self) {
        self.0.invalidate_items_layout()
    }
}

impl ControlImpl for RibbonGroup {
    fn on_create_automation_peer(
        this: &Self,
    ) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        super::RibbonGroupAutomationPeer::new(this).upcast()
    }
}

#[cfg(test)]
#[path = "../key_tip_target_tests.rs"]
mod key_tip_target_tests;
