//! Native Ribbon composition from Ribbon.cs.
use super::{
    IRibbonItemOwner, RibbonContextualTabGroup, RibbonPanelPresentation, RibbonTab, RibbonTabHeader,
};
use crate::{
    metadata::ribbon_collections::{RibbonContextualGroups, RibbonTabs},
    primitives::{RibbonMetrics, RibbonScrollPanel},
    theming::RibbonTheme,
};
use ferroribbon_core::{
    RibbonContextualActivation, RibbonDensity, RibbonDisplayMode, RibbonString,
    RibbonVisibilityMode, commands::RibbonCommandCatalog, layout::RibbonReductionStrategy,
};
use ferroui_base::{
    BoxedValue, FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Ref,
    StyledElementImpl, StyledProperty, VisualImpl, WeakRef, ferro_class, ferro_class_info,
    ferro_impl_classes, ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::{LayoutableImpl, Orientation},
    media::SolidColorBrush,
    metadata::MarkupDelegate,
};
use ferroui_controls::{
    Button, Control, ControlImpl, Grid, Panel, StackPanel,
    automation::{AutomationProperties, peers::ControlAutomationPeer},
    primitives::{
        TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
    },
};
#[path = "ribbon_backstage_popup.rs"]
mod backstage_popup;
#[path = "ribbon_commands.rs"]
mod commands;
#[path = "ribbon_context_menu.rs"]
mod context_menu;
#[path = "ribbon_customization.rs"]
mod customization;
pub use context_menu::{RibbonContextMenuError, RibbonContextMenuEventArgs};
#[path = "ribbon_display_options.rs"]
mod display_options;
#[path = "ribbon_localization.rs"]
mod localization;
pub use customization::RibbonCustomizeRequestedEventArgs;
#[path = "ribbon_keyboard.rs"]
mod keyboard;
#[path = "ribbon_model.rs"]
mod model;
#[path = "ribbon_quick_access.rs"]
mod quick_access;
#[path = "ribbon_shortcuts.rs"]
mod shortcuts;
#[path = "ribbon_state.rs"]
mod state;
#[path = "ribbon_visibility.rs"]
mod visibility;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
#[derive(Clone, Copy, PartialEq, Eq)]
enum PartClick {
    Application,
    Options,
    Reveal,
    Minimize,
    Behavior,
}
#[derive(Default)]
struct Parts {
    quick_access_above: Option<Ref<ferroui_controls::Border>>,
    quick_access_below: Option<Ref<ferroui_controls::Border>>,
    command_bar: Option<Ref<ferroui_controls::Border>>,
    reveal_bar: Option<Ref<Control>>,
    reveal: Option<Ref<Button>>,
    minimize_buttons: Option<Ref<Control>>,
    minimize: Option<Ref<Button>>,
    minimize_behavior: Option<Ref<Button>>,
    minimize_glyph: Option<Ref<crate::compat::FontIcon>>,
    click_handlers: Vec<(
        PartClick,
        Ref<Button>,
        ferroui_base::interactivity::RoutedEventHandlerToken,
    )>,
    content: Option<Ref<Grid>>,
    scroll: Option<Ref<RibbonScrollPanel>>,
    application: Option<Ref<Button>>,
    options: Option<Ref<Button>>,
    tab_row: Option<Ref<Control>>,
    strip_items_host: Option<Ref<Panel>>,
}
#[repr(C)]
pub struct Ribbon {
    base: TemplatedControl,
    visibility: visibility::VisibilityState,
    shortcuts: shortcuts::ShortcutState,
    keyboard: keyboard::KeyboardState,
    localization: localization::LocalizationState,
    quick_access: quick_access::QuickAccessState,
    state: state::RootState,
    customization_ui: customization::CustomizationState,
    context_menu_state: context_menu::ContextMenuState,
    model: model::ModelState,
    backstage_state: backstage_popup::BackstageState,
    command_states: RefCell<
        std::collections::HashMap<
            ferroribbon_core::text::RibbonOrdinalIgnoreCaseKey,
            commands::CommandState,
        >,
    >,
    catalog_subscription: RefCell<Option<commands::CatalogSubscription>>,
    tab_strip_items: crate::metadata::control_items::RibbonControlItems,
    tabs: RibbonTabs,
    contextual_groups: RibbonContextualGroups,
    strip: Ref<StackPanel>,
    headers: RefCell<Vec<Ref<RibbonTabHeader>>>,
    // Retain generated presentations for every tab in Tabs, including hidden
    // contextual tabs. Tab's back-cache is weak; Header.Tab is strong.
    tab_header_cache: RefCell<Vec<Ref<RibbonTabHeader>>>,
    parts: RefCell<Parts>,
    last_regular: RefCell<Option<WeakRef<RibbonTab>>>,
    updating_selection: Cell<bool>,
    contextual_subscriptions: RefCell<Vec<(Ref<RibbonContextualTabGroup>, u64)>>,
    selected_tab_changed: RefCell<Option<MarkupDelegate>>,
    item_invoked: RefCell<Option<MarkupDelegate>>,
    state_changed: RefCell<Option<MarkupDelegate>>,
    group_floating_changed: RefCell<Option<MarkupDelegate>>,
    search_engine: Rc<ferroribbon_core::search::RibbonSearchEngine>,
    pub(super) additional_search_entries: super::ribbon_search::RibbonAdditionalSearchEntries,
    compact_tab_strip_width: Cell<f64>,
    compact_tab_strip: Cell<bool>,
}
ferro_class!(Ribbon:TemplatedControl);
ferro_class_info!(Ribbon{new:Ribbon::new,markup:{namespace:"FerroRibbon.Controls",content:Tabs,properties:[ItemFactory:Ref<crate::mvvm::RibbonElementFactory>{get:Ribbon::item_factory,set:|r:&Ribbon,f:Ref<crate::mvvm::RibbonElementFactory>|r.set_item_factory(Some(f))},Customization:Rc<ferroribbon_core::state::RibbonCustomization>{get:Ribbon::customization},FloatingGroups:crate::metadata::ribbon_collections::RibbonFloatingGroups{get:Ribbon::floating_groups},QuickAccessCandidateIds:crate::metadata::quick_access_collections::RibbonQuickAccessCandidates{get:Ribbon::quick_access_candidate_ids},DefaultQuickAccessItemIds:Option<crate::metadata::quick_access_collections::RibbonQuickAccessItemIds>{get:Ribbon::default_quick_access_item_ids},DefaultQuickAccessPosition:ferroribbon_core::RibbonQuickAccessPosition{get:Ribbon::default_quick_access_position},IsKeyTipMode:bool{get:Ribbon::is_key_tip_mode},ActiveKeyTips:crate::metadata::key_tip_collections::RibbonActiveKeyTips{get:Ribbon::active_key_tips},CurrentKeyTips:crate::metadata::key_tip_collections::RibbonCurrentKeyTips{get:Ribbon::current_key_tips},TabStripItems:crate::metadata::control_items::RibbonControlItems{get:Ribbon::tab_strip_items},SearchEngine:Rc<ferroribbon_core::search::RibbonSearchEngine>{get:Ribbon::search_engine},AdditionalSearchEntries:crate::controls::RibbonAdditionalSearchEntries{get:Ribbon::additional_search_entries},IsMinimizedPopupOpen:bool{get:Ribbon::is_minimized_popup_open},CompactTabStripWidth:f64{get:Ribbon::compact_tab_strip_width,set:Ribbon::set_compact_tab_strip_width},Tabs:RibbonTabs{get:Ribbon::tabs},ContextualGroups:RibbonContextualGroups{get:Ribbon::contextual_groups},Metrics:Arc<RibbonMetrics>{get:Ribbon::metrics},ApplicationButton:Option<Ref<Button>>{get:Ribbon::application_button},DisplayOptionsButton:Option<Ref<Button>>{get:Ribbon::display_options_button}],methods:[try fn ShowItemContextMenu(Ref<Control>,ferroui_base::Point)->bool =>Ribbon::show_item_context_menu,try fn BuildShowTabsMenu()->Ref<crate::compat::MenuFlyoutSubItem> =>Ribbon::build_show_tabs_menu,try fn BuildShowPanelsMenu(Ref<RibbonTab>)->Ref<crate::compat::MenuFlyoutSubItem> =>Ribbon::build_show_panels_menu,try fn ShowCustomizeDialog(super::RibbonCustomizePage)->Option<Ref<super::RibbonCustomizeDialog>> =>Ribbon::show_customize_dialog,fn BuildDisplayOptionsMenu()->Ref<ferroui_controls::MenuFlyout> =>Ribbon::build_display_options_menu,fn BuildSearchEntries()->crate::controls::RibbonSearchEntries =>Ribbon::build_search_entries,fn Search(Option<String>)->ferroribbon_core::search::RibbonSearchResults =>|r:&Ribbon,q:Option<String>|r.search_default(q.map(ferroribbon_core::RibbonString::from).as_ref()),fn Search(Option<String>,i32)->ferroribbon_core::search::RibbonSearchResults =>|r:&Ribbon,q:Option<String>,n|r.search(q.map(ferroribbon_core::RibbonString::from).as_ref(),n),fn ExecuteSearchEntry(Option<Rc<ferroribbon_core::search::RibbonSearchEntry>>)->bool =>Ribbon::execute_search_entry,static fn GetModel(Ref<ferroui_base::FerroObject>)->Option<Rc<dyn ferroribbon_core::model::RibbonNodeModelImpl>> =>|element:Ref<ferroui_base::FerroObject>|Ribbon::get_model(&element),fn InvokeApplicationButton()=>Ribbon::invoke_application_button,fn InvokeApplicationButton(Option<Ref<Control>>)=>Ribbon::invoke_application_button_at,try fn GetState()->Rc<ferroribbon_core::state::RibbonState> =>Ribbon::get_state,
 try fn ApplyState(Option<Rc<ferroribbon_core::state::RibbonState>>)=>Ribbon::apply_saved_state,
 try fn ApplyCustomization(Option<Rc<ferroribbon_core::state::RibbonCustomization>>)=>Ribbon::apply_customization,
 try fn ResetCustomization()=>Ribbon::reset_customization,
 try fn ResetQuickAccess()=>Ribbon::reset_quick_access,
 try fn SaveStateToJson()->String=>Ribbon::save_state_json_markup,
 try fn LoadStateFromJson(Option<String>)->bool=>|r:&Ribbon,s:Option<String>|r.load_state_from_json(s.map(Into::into)),
 fn ReturnAllPanelsToRibbon()=>Ribbon::return_all_panels_to_ribbon,fn FindQuickAccessCopy(Option<Ref<Control>>)->Option<Ref<Control>> =>Ribbon::find_quick_access_copy,fn IsInQuickAccess(Ref<Control>)->bool=>|r:&Ribbon,c:Ref<Control>|r.is_in_quick_access(&c),fn AddToQuickAccess(Ref<Control>)->bool=>|r:&Ribbon,c:Ref<Control>|r.add_to_quick_access(&c),fn AddToQuickAccess(Option<String>)->bool=>|r:&Ribbon,id:Option<String>|r.add_to_quick_access_id(id.map(Into::into)),fn RemoveFromQuickAccess(Ref<Control>)->bool=>|r:&Ribbon,c:Ref<Control>|r.remove_from_quick_access(&c),fn GetQuickAccessItemIds()->crate::metadata::quick_access_collections::RibbonQuickAccessItemIds=>Ribbon::get_quick_access_item_ids,fn BuildQuickAccessCustomizeMenu()->Ref<ferroui_controls::MenuFlyout> =>Ribbon::build_quick_access_customize_menu,fn ToggleQuickAccessPosition()=>Ribbon::toggle_quick_access_position,fn ShowKeyTips()=>Ribbon::show_key_tips,fn HideKeyTips()=>Ribbon::hide_key_tips,fn CancelKeyTips()=>Ribbon::cancel_key_tips,fn PopKeyTipLevel()->bool=>Ribbon::pop_key_tip_level,try fn ProcessKeyTipInput(char)->bool=>Ribbon::process_key_tip_char,fn ProcessKeyTipInputUtf16(u16)->bool=>Ribbon::process_key_tip_input,fn SuspendPopups() => Ribbon::suspend_popups,fn ToggleMinimized()=>Ribbon::toggle_minimized,fn NextMinimizeState()->ferroribbon_core::RibbonVisibilityMode =>Ribbon::next_minimize_state,fn OpenMinimizedPopup()=>Ribbon::open_minimized_popup,fn CloseMinimizedPopup()=>Ribbon::close_minimized_popup,fn BuildMinimizeBehaviorMenu()->Ref<ferroui_controls::MenuFlyout> =>Ribbon::build_minimize_behavior_menu,fn ResumePopups() => Ribbon::resume_popups,fn GetAllItems()->ferroribbon_core::model::RibbonItemsSource<Ref<Control>> => Ribbon::get_all_items,
 fn FindItem(Option<String>)->Option<Ref<Control>> => |this:&Ref<Ribbon>,id:Option<String>|this.find_item(id.map(Into::into)),
 fn FindItemsByCommand(Option<String>)->ferroribbon_core::model::RibbonItemsSource<Ref<Control>> => |this:&Ref<Ribbon>,id:Option<String>|this.find_items_by_command(id.map(Into::into)),
 fn SetCommandEnabled(Option<String>,bool) => |this:&Ref<Ribbon>,id:Option<String>,value|this.set_command_enabled(id.map(Into::into),value),
 fn SetCommandChecked(Option<String>,bool) => |this:&Ref<Ribbon>,id:Option<String>,value|this.set_command_checked(id.map(Into::into),value),
 fn OnItemInvoked(Ref<Control>,Option<String>,Option<BoxedValue>) => |this:&Ref<Ribbon>,item,id:Option<String>,parameter|this.on_item_invoked(item,id.map(Into::into),parameter),
 fn FindTab(Option<String>)->Option<Ref<RibbonTab>> => |this:&Ref<Ribbon>,id:Option<String>|this.find_tab_by_id_or_header(id.as_deref()),
 fn FindContextualGroup(Option<String>)->Option<Ref<RibbonContextualTabGroup>> => |this:&Ref<Ribbon>,id:Option<String>|this.find_contextual_group_by_id(id.as_deref()),
 fn SetActiveContextualGroups(Option<Vec<Option<String>>>) => |this:&Ref<Ribbon>,ids:Option<Vec<Option<String>>>|this.set_active_contextual_groups(&ids.as_deref().unwrap_or_default().iter().map(Option::as_deref).collect::<Vec<_>>()),
 fn SelectTab(Option<String>)->bool => |this:&Ref<Ribbon>,id:Option<String>|this.select_tab_by_id_or_header(id.as_deref()),fn SelectTab(Ref<RibbonTab>)->bool => Ribbon::select_tab,fn InvalidateShortcuts() => Ribbon::invalidate_shortcuts,fn InvalidateLayout() => Ribbon::invalidate_layout],events:[ContextMenuOpening(Option<BoxedValue>,Rc<RibbonContextMenuEventArgs>){add:Ribbon::add_context_menu_opening,remove:Ribbon::remove_context_menu_opening},CustomizeRequested(Option<BoxedValue>,Rc<RibbonCustomizeRequestedEventArgs>){add:Ribbon::add_customize_requested,remove:Ribbon::remove_customize_requested},ApplicationButtonClick(Option<BoxedValue>,Rc<super::RibbonHandledEventArgs>){add:Ribbon::add_application_button_click,remove:Ribbon::remove_application_button_click},BackstageOpened(Option<BoxedValue>,ferroui_base::utilities::EventArgs){add:Ribbon::add_backstage_opened,remove:Ribbon::remove_backstage_opened},BackstageClosed(Option<BoxedValue>,ferroui_base::utilities::EventArgs){add:Ribbon::add_backstage_closed,remove:Ribbon::remove_backstage_closed},GroupFloatingChanged(Option<BoxedValue>,Ref<super::RibbonGroup>){add:Ribbon::add_group_floating_changed,remove:Ribbon::remove_group_floating_changed},QuickAccessChanged(Option<BoxedValue>,ferroui_base::utilities::EventArgs){add:Ribbon::add_quick_access_changed,remove:Ribbon::remove_quick_access_changed},KeyTipModeChanged(Option<BoxedValue>,bool){add:Ribbon::add_key_tip_mode_changed,remove:Ribbon::remove_key_tip_mode_changed},SearchRequested(Option<BoxedValue>,ferroui_base::utilities::EventArgs){add:Ribbon::add_search_requested,remove:Ribbon::remove_search_requested},StateChanged(Option<BoxedValue>,ferroui_base::utilities::EventArgs){add:Ribbon::add_state_changed,remove:Ribbon::remove_state_changed},SelectedTabChanged(Option<BoxedValue>,Rc<RibbonTabChangedEventArgs>){add:Ribbon::add_selected_tab_changed,remove:Ribbon::remove_selected_tab_changed},ItemInvoked(Option<BoxedValue>,Rc<RibbonItemInvokedEventArgs>){add:Ribbon::add_item_invoked,remove:Ribbon::remove_item_invoked}]},interfaces:[Rc<dyn IRibbonItemOwner> => |value|Rc::new(RibbonOwner(value)) as Rc<dyn IRibbonItemOwner>]});
ferro_impl_classes!(Ribbon:VisualImpl,InteractiveImpl,InputElementImpl,LayoutableImpl);
ferro_properties! {impl Ribbon {
 pub fn model_property()->StyledProperty<Option<Rc<dyn ferroribbon_core::model::RibbonModelImpl>>>{FerroProperty::register::<Self,_>("Model",None)}
 pub fn backstage_property()->StyledProperty<Option<Ref<super::RibbonBackstage>>>{FerroProperty::register::<Self,_>("Backstage",None)}
 pub fn is_backstage_open_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsBackstageOpen",false)}
 pub fn application_menu_property()->StyledProperty<Option<Ref<ferroui_controls::primitives::FlyoutBase>>>{FerroProperty::register::<Self,_>("ApplicationMenu",None)}
    pub fn can_float_groups_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("CanFloatGroups",false)}
 pub fn quick_access_tool_bar_property()->StyledProperty<Option<Ref<super::RibbonQuickAccessToolBar>>>{FerroProperty::register::<Self,_>("QuickAccessToolBar",None)}
 pub fn quick_access_position_property()->StyledProperty<ferroribbon_core::RibbonQuickAccessPosition>{FerroProperty::register::<Self,_>("QuickAccessPosition",ferroribbon_core::RibbonQuickAccessPosition::AboveRibbon)}
 pub fn is_quick_access_visible_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsQuickAccessVisible",true)}
 pub fn show_quick_access_labels_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("ShowQuickAccessLabels",false)}
 pub fn is_quick_access_hosted_externally_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsQuickAccessHostedExternally",false)}

 pub fn application_button_key_tip_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("ApplicationButtonKeyTip",Some("F".into()))}
 pub fn is_key_tips_enabled_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsKeyTipsEnabled",true)}
 pub fn is_shortcut_routing_enabled_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsShortcutRoutingEnabled",false)}
 pub fn visibility_mode_property()->StyledProperty<ferroribbon_core::RibbonVisibilityMode>{FerroProperty::register::<Self,_>("VisibilityMode",ferroribbon_core::RibbonVisibilityMode::AlwaysShow)}
 pub fn is_minimized_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsMinimized",false)}
 pub fn is_collapsible_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsCollapsible",true)}
 pub fn is_full_screen_revealed_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsFullScreenRevealed",false)}
 pub fn minimize_behavior_property()->StyledProperty<ferroribbon_core::RibbonMinimizeBehavior>{FerroProperty::register::<Self,_>("MinimizeBehavior",ferroribbon_core::RibbonMinimizeBehavior::Tabs)}
 pub fn is_minimize_button_visible_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsMinimizeButtonVisible",false)}
 pub fn command_catalog_property()->StyledProperty<Option<Rc<RibbonCommandCatalog>>>{FerroProperty::register::<Self,_>("CommandCatalog",None)}
 pub fn selected_tab_property()->StyledProperty<Option<Ref<RibbonTab>>>{FerroProperty::register::<Self,_>("SelectedTab",None)}
 pub fn selected_tab_id_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("SelectedTabId",None)}
 pub fn display_mode_property()->StyledProperty<RibbonDisplayMode>{FerroProperty::register::<Self,_>("DisplayMode",RibbonDisplayMode::Classic)}
 pub fn density_property()->StyledProperty<RibbonDensity>{FerroProperty::register::<Self,_>("Density",RibbonDensity::Comfortable)}
 pub fn custom_metrics_property()->StyledProperty<Option<Arc<RibbonMetrics>>>{FerroProperty::register::<Self,_>("CustomMetrics",None)}
 pub fn is_adaptive_layout_enabled_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsAdaptiveLayoutEnabled",true)}
 pub fn reduction_strategy_property()->StyledProperty<RibbonReductionStrategy>{FerroProperty::register::<Self,_>("ReductionStrategy",RibbonReductionStrategy::Stepwise)}
 pub fn show_group_captions_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("ShowGroupCaptions",true)}
 pub fn contextual_activation_property()->StyledProperty<RibbonContextualActivation>{FerroProperty::register::<Self,_>("ContextualActivation",RibbonContextualActivation::None)}
 pub fn is_application_button_visible_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsApplicationButtonVisible",true)}
 pub fn is_context_menu_enabled_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsContextMenuEnabled",true)}
 pub fn can_customize_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("CanCustomize",true)}
 pub fn is_visibility_menu_enabled_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsVisibilityMenuEnabled",false)}
 pub fn is_simplified_mode_available_property()->StyledProperty<bool>{FerroProperty::register::<Self, _>("IsSimplifiedModeAvailable",true)}
 pub fn is_display_options_button_visible_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsDisplayOptionsButtonVisible",true)}
 pub fn application_button_label_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("ApplicationButtonLabel",None)}
 pub fn application_button_content_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("ApplicationButtonContent",None)}
 pub fn tab_strip_start_content_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("TabStripStartContent",None)}
 pub fn tab_strip_end_content_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("TabStripEndContent",None)}
}}
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
impl Ribbon {
    fn static_constructor() {
        Self::initialize_model();
        Self::initialize_backstage();
        Self::quick_access_tool_bar_property()
            .changed()
            .add_class_handler::<Self>(|this, e| {
                let (old, new) =
                    e.get_old_and_new_value::<Option<Ref<super::RibbonQuickAccessToolBar>>>();
                this.on_quick_access_tool_bar_changed(old, new);
            })
            .detach();
        Self::quick_access_position_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.on_quick_access_options_changed())
            .detach();
        Self::is_quick_access_visible_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.on_quick_access_options_changed())
            .detach();
        Self::show_quick_access_labels_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.on_quick_access_options_changed())
            .detach();
        Self::is_quick_access_hosted_externally_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.update_quick_access_placement())
            .detach();

        Self::visibility_mode_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.on_visibility_mode_changed())
            .detach();
        Self::is_minimized_property()
            .changed()
            .add_class_handler::<Self>(|this, e| {
                let (_, new) = e.get_old_and_new_value::<bool>();
                this.on_is_minimized_changed(new);
            })
            .detach();
        Self::is_full_screen_revealed_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.update_chrome())
            .detach();
        Self::minimize_behavior_property()
            .changed()
            .add_class_handler::<Self>(|this, _| {
                this.update_minimize_button();
                this.raise_state_changed();
                this.on_model_panel_options_changed();
            })
            .detach();
        Self::is_minimize_button_visible_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.update_minimize_button())
            .detach();
        Self::command_catalog_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.on_command_catalog_changed())
            .detach();
        Self::selected_tab_property()
            .changed()
            .add_class_handler::<Self>(|this, e| {
                let (old, new) = e.get_old_and_new_value::<Option<Ref<RibbonTab>>>();
                this.on_selected_tab_changed(old, new);
            })
            .detach();
        Self::selected_tab_id_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.on_selected_tab_id_changed())
            .detach();
        macro_rules! presentation{($($p:ident),*) => {$(Self::$p().changed().add_class_handler::<Self>(|this,_|this.apply_presentation()).detach();)*};}
        presentation!(
            is_adaptive_layout_enabled_property,
            reduction_strategy_property
        );
        Self::density_property()
            .changed()
            .add_class_handler::<Self>(|this, _| {
                this.on_model_display_mode_changed();
                this.apply_presentation();
                this.apply_quick_access_layout();
                this.raise_state_changed();
            })
            .detach();
        Self::custom_metrics_property()
            .changed()
            .add_class_handler::<Self>(|this, _| {
                this.on_model_display_mode_changed();
                this.apply_presentation();
                this.apply_quick_access_layout();
                this.raise_state_changed();
            })
            .detach();
        Self::show_group_captions_property()
            .changed()
            .add_class_handler::<Self>(|this, _| {
                this.apply_presentation();
                this.raise_state_changed();
                this.on_model_panel_options_changed();
            })
            .detach();
        Self::display_mode_property()
            .changed()
            .add_class_handler::<Self>(|this, _| {
                this.close_minimized_popup();
                this.apply_presentation();
                this.update_chrome();
                this.raise_state_changed();
                this.on_model_display_mode_changed();
            })
            .detach();
        ferroui_base::Visual::is_visible_property()
            .changed()
            .add_class_handler::<Self>(|this, _| {
                if this.is_visible() {
                    this.resume_popups();
                } else {
                    this.suspend_popups();
                }
            })
            .detach();
        macro_rules! chrome{($($p:ident),*) => {$(Self::$p().changed().add_class_handler::<Self>(|this,_|this.update_chrome()).detach();)*};}
        chrome!(
            is_application_button_visible_property,
            is_display_options_button_visible_property,
            application_button_label_property,
            application_button_content_property
        );
    }
    pub fn construct() -> Self {
        let strip = StackPanel::new();
        strip.set_orientation(Orientation::Horizontal);
        strip.set_spacing(0.);
        Self {
            base: TemplatedControl::construct(),
            visibility: visibility::VisibilityState::default(),
            shortcuts: shortcuts::ShortcutState::default(),
            keyboard: keyboard::KeyboardState::default(),
            localization: localization::LocalizationState::default(),
            quick_access: quick_access::QuickAccessState::default(),
            state: state::RootState::default(),
            customization_ui: customization::CustomizationState::default(),
            context_menu_state: context_menu::ContextMenuState::default(),
            model: model::ModelState::default(),
            backstage_state: backstage_popup::BackstageState::default(),
            command_states: RefCell::default(),
            catalog_subscription: RefCell::default(),
            tab_strip_items: crate::metadata::control_items::RibbonControlItems::new_managed(),
            tabs: RibbonTabs::new_managed(),
            contextual_groups: RibbonContextualGroups::new_managed(),
            strip,
            headers: RefCell::default(),
            tab_header_cache: RefCell::default(),
            parts: RefCell::default(),
            last_regular: RefCell::default(),
            updating_selection: Cell::new(false),
            contextual_subscriptions: RefCell::default(),
            selected_tab_changed: RefCell::default(),
            item_invoked: RefCell::default(),
            state_changed: RefCell::default(),
            group_floating_changed: RefCell::default(),
            search_engine: ferroribbon_core::search::RibbonSearchEngine::new(),
            additional_search_entries:
                super::ribbon_search::RibbonAdditionalSearchEntries::new_managed(),
            compact_tab_strip_width: Cell::new(900.),
            compact_tab_strip: Cell::new(false),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn search_engine(&self) -> Rc<ferroribbon_core::search::RibbonSearchEngine> {
        self.search_engine.clone()
    }
    pub fn compact_tab_strip_width(&self) -> f64 {
        self.compact_tab_strip_width.get()
    }
    pub fn set_compact_tab_strip_width(&self, value: f64) {
        self.compact_tab_strip_width.set(value);
    }
    fn sync_tab_strip_items(&self) {
        let Some(host) = self.parts.borrow().strip_items_host.clone() else {
            return;
        };
        host.children().clear();
        for item in self.tab_strip_items.iter() {
            let item = item.expect("ArgumentNull: tab strip item");
            if let Some(parent) = item.visual_parent().and_then(|p| p.cast::<Panel>()) {
                parent.children().remove(item.clone());
            }
            super::RibbonItemHelper::set_owner(&item, Some(self.item_owner()))
                .expect("tab strip owner");
            host.children().add(item.clone());
            if let Some(item) = ferroui_base::data::core::ValueTypes::try_cast(
                &Control::boxed(item),
                ferroui_base::data::core::ValueType::of::<Rc<dyn super::IRibbonItem>>(),
            )
            .and_then(|v| v.downcast_ref::<Rc<dyn super::IRibbonItem>>().cloned())
            {
                item.apply_layout(super::RibbonItemLayout {
                    size: ferroribbon_core::RibbonItemSize::Medium,
                    metrics: Some(self.metrics()),
                    is_simplified: true,
                    ..Default::default()
                });
            }
        }
    }
    pub fn command_catalog(&self) -> Option<Rc<RibbonCommandCatalog>> {
        self.get_value(Self::command_catalog_property())
    }
    pub fn set_command_catalog(&self, value: Option<Rc<RibbonCommandCatalog>>) {
        self.set_value(Self::command_catalog_property(), value);
    }
    pub(crate) fn item_owner(&self) -> Rc<dyn IRibbonItemOwner> {
        Rc::new(RibbonOwner(self.to_ref()))
    }
    pub fn tab_strip_items(&self) -> crate::metadata::control_items::RibbonControlItems {
        self.tab_strip_items.clone()
    }
    pub fn tabs(&self) -> RibbonTabs {
        self.tabs.clone()
    }
    pub fn contextual_groups(&self) -> RibbonContextualGroups {
        self.contextual_groups.clone()
    }
    property!(
        selected_tab,
        set_selected_tab,
        selected_tab_property,
        Option<Ref<RibbonTab>>
    );
    property!(
        selected_tab_id,
        set_selected_tab_id,
        selected_tab_id_property,
        Option<String>
    );
    property!(
        display_mode,
        set_display_mode,
        display_mode_property,
        RibbonDisplayMode
    );
    property!(density, set_density, density_property, RibbonDensity);
    property!(
        custom_metrics,
        set_custom_metrics,
        custom_metrics_property,
        Option<Arc<RibbonMetrics>>
    );
    property!(
        is_adaptive_layout_enabled,
        set_is_adaptive_layout_enabled,
        is_adaptive_layout_enabled_property,
        bool
    );
    property!(
        reduction_strategy,
        set_reduction_strategy,
        reduction_strategy_property,
        RibbonReductionStrategy
    );
    property!(
        show_group_captions,
        set_show_group_captions,
        show_group_captions_property,
        bool
    );
    property!(
        contextual_activation,
        set_contextual_activation,
        contextual_activation_property,
        RibbonContextualActivation
    );
    property!(
        is_application_button_visible,
        set_is_application_button_visible,
        is_application_button_visible_property,
        bool
    );
    property!(
        is_display_options_button_visible,
        set_is_display_options_button_visible,
        is_display_options_button_visible_property,
        bool
    );
    property!(
        application_button_label,
        set_application_button_label,
        application_button_label_property,
        Option<String>
    );
    property!(
        application_button_content,
        set_application_button_content,
        application_button_content_property,
        Option<BoxedValue>
    );
    property!(
        tab_strip_start_content,
        set_tab_strip_start_content,
        tab_strip_start_content_property,
        Option<BoxedValue>
    );
    property!(
        tab_strip_end_content,
        set_tab_strip_end_content,
        tab_strip_end_content_property,
        Option<BoxedValue>
    );
    pub fn metrics(&self) -> Arc<RibbonMetrics> {
        self.custom_metrics()
            .unwrap_or_else(|| RibbonMetrics::for_density(self.density()))
    }
    pub fn application_button(&self) -> Option<Ref<Button>> {
        self.parts.borrow().application.clone()
    }
    pub fn display_options_button(&self) -> Option<Ref<Button>> {
        self.parts.borrow().options.clone()
    }
    pub fn tab_headers(&self) -> Vec<Ref<RibbonTabHeader>> {
        self.headers.borrow().clone()
    }
    pub fn visible_tabs(&self) -> Vec<Ref<RibbonTab>> {
        self.headers.borrow().iter().map(|h| h.tab()).collect()
    }
    pub fn find_tab(&self, id: &str) -> Option<Ref<RibbonTab>> {
        self.find_tab_by_id_or_header(Some(id))
    }
    fn find_tab_by_id_or_header(&self, id: Option<&str>) -> Option<Ref<RibbonTab>> {
        self.tabs
            .iter()
            .map(|t| t.expect("null RibbonTab"))
            .find(|t| t.effective_id().as_deref() == id || t.header().as_deref() == id)
    }
    pub fn select_tab_id(&self, id: &str) -> bool {
        self.select_tab_by_id_or_header(Some(id))
    }
    fn select_tab_by_id_or_header(&self, id: Option<&str>) -> bool {
        let Some(tab) = self
            .find_tab_by_id_or_header(id)
            .filter(|t| t.is_available())
        else {
            return false;
        };
        self.set_selected_tab(Some(tab));
        true
    }
    pub fn select_tab(&self, tab: Ref<RibbonTab>) -> bool {
        if !self.tabs.contains(&Some(tab.clone())) || !tab.is_available() {
            return false;
        }
        self.set_selected_tab(Some(tab));
        true
    }
    pub fn find_contextual_group(&self, id: &str) -> Option<Ref<RibbonContextualTabGroup>> {
        self.find_contextual_group_by_id(Some(id))
    }
    fn find_contextual_group_by_id(
        &self,
        id: Option<&str>,
    ) -> Option<Ref<RibbonContextualTabGroup>> {
        self.contextual_groups
            .iter()
            .map(|g| g.expect("null contextual group"))
            .find(|g| g.id().as_deref() == id)
    }
    /// Shows exactly the contextual groups whose IDs occur in `ids`, in collection order.
    ///
    /// IDs use ordinal, case-sensitive equality. `None` matches an unset group ID;
    /// an empty slice hides every group. Each change uses the ordinary property
    /// path so activation, selection restoration, and notifications remain native.
    pub fn set_active_contextual_groups(&self, ids: &[Option<&str>]) {
        for group in self.contextual_groups.iter() {
            let group = group.expect("null contextual group");
            group.set_is_visible(ids.contains(&group.id().as_deref()));
        }
    }
    pub(crate) fn on_groups_changed(&self, tab: &RibbonTab) {
        self.invalidate_shortcuts();
        self.apply_customization_to_tab(tab)
            .expect("ApplyCustomizationToTab");
        for item in tab
            .groups()
            .iter()
            .flat_map(|group| group.expect("null group").get_all_items().iter())
        {
            self.apply_command_state(&item);
        }
    }
    fn tabs_changed(&self) {
        for tab in self.tabs.iter() {
            tab.expect("null RibbonTab").attach_to_ribbon(self);
        }
        self.sync_tab_content();
        self.on_tabs_changed();
    }
    fn contextual_groups_changed(&self) {
        let stale = self
            .contextual_subscriptions
            .borrow()
            .iter()
            .filter(|(g, _)| !self.contextual_groups.contains(&Some(g.clone())))
            .cloned()
            .collect::<Vec<_>>();
        for (g, token) in stale {
            g.remove_property_changed_handler(token);
            self.contextual_subscriptions
                .borrow_mut()
                .retain(|(v, _)| !v.ptr_eq(&g));
        }
        for g in self.contextual_groups.iter() {
            let g = g.expect("null contextual group");
            if self
                .contextual_subscriptions
                .borrow()
                .iter()
                .any(|(v, _)| v.ptr_eq(&g))
            {
                continue;
            }
            let weak = self.to_ref().downgrade();
            let weak_group = g.downgrade();
            let token = g.add_property_changed_handler(Rc::new(move |e| {
                if e.property() != RibbonContextualTabGroup::color_property().as_property()
                    && e.property() != RibbonContextualTabGroup::is_visible_property().as_property()
                    && e.property() != RibbonContextualTabGroup::is_enabled_property().as_property()
                {
                    return;
                }
                if let (Some(r), Some(g)) = (weak.upgrade(), weak_group.upgrade()) {
                    r.contextual_group_changed(&g)
                }
            }));
            self.contextual_subscriptions.borrow_mut().push((g, token));
        }
        self.on_tabs_changed();
    }
    fn contextual_group_changed(&self, g: &RibbonContextualTabGroup) {
        let was_visible = self
            .headers
            .borrow()
            .iter()
            .any(|h| h.tab().contextual_group_id() == g.id());
        self.on_tabs_changed();
        let activation = if g.activation() == RibbonContextualActivation::None {
            self.contextual_activation()
        } else {
            g.activation()
        };
        if g.is_visible() && !was_visible && activation == RibbonContextualActivation::SelectOnShow
        {
            let first = self
                .headers
                .borrow()
                .iter()
                .find(|h| h.tab().contextual_group_id() == g.id())
                .map(|h| h.tab());
            if let Some(first) = first {
                self.set_selected_tab(Some(first));
            }
        }
    }
    fn sync_tab_content(&self) {
        let Some(host) = self.parts.borrow().content.clone() else {
            return;
        };
        let stale = host
            .children()
            .snapshot()
            .iter()
            .filter_map(|c| c.cast::<RibbonTab>())
            .filter(|t| !self.tabs.contains(&Some(t.clone())))
            .collect::<Vec<_>>();
        for stale in stale {
            host.children().remove(stale);
        }
        for tab in self.tabs.iter() {
            let tab = tab.expect("null RibbonTab");
            tab.set_is_visible(self.selected_tab().as_ref() == Some(&tab));
            if tab.visual_parent().as_ref() == Some(&host.clone().upcast()) {
                continue;
            }
            if let Some(parent) = tab.visual_parent().and_then(|p| p.cast::<Panel>()) {
                parent.children().remove(tab.clone());
            }
            host.children().add(tab.clone());
            tab.apply_template();
        }
        self.apply_presentation();
    }
    pub(crate) fn on_tabs_changed(&self) {
        if self.state.applying.get() {
            return;
        }
        let mut ordered = self
            .tabs
            .iter()
            .map(|t| t.expect("null RibbonTab"))
            .filter(|t| t.is_available() && !t.is_contextual())
            .collect::<Vec<_>>();
        for g in self.contextual_groups.iter() {
            let g = g.expect("null contextual group");
            if g.is_visible() {
                ordered.extend(
                    self.tabs
                        .iter()
                        .map(|t| t.expect("null RibbonTab"))
                        .filter(|t| t.contextual_group_id() == g.id() && t.is_available()),
                );
            }
        }
        let ordered = self.apply_tab_order(ordered);
        let tab_headers_changed = self
            .headers
            .borrow()
            .iter()
            .map(|header| header.tab())
            .ne(ordered.iter().cloned());
        self.strip.children().clear();
        self.headers.borrow_mut().clear();
        for tab in &ordered {
            let header = tab.header_element().unwrap_or_else(|| {
                let h = RibbonTabHeader::new(tab.clone());
                tab.set_header_element(&h);
                let weak = self.to_ref().downgrade();
                let weak_tab = tab.downgrade();
                h.click(move |_, _| {
                    if let (Some(r), Some(tab)) = (weak.upgrade(), weak_tab.upgrade()) {
                        r.on_header_clicked(tab);
                    }
                });
                let weak = self.to_ref().downgrade();
                let weak_header = h.downgrade();
                h.key_down(move |_, e| {
                    if let (Some(r), Some(h)) = (weak.upgrade(), weak_header.upgrade()) {
                        e.set_handled(r.on_header_key_down(&h, e.key));
                    }
                });
                let weak = self.to_ref().downgrade();
                h.add_handler(
                    ferroui_base::input::InputElement::double_tapped_event(),
                    move |_, e| {
                        if let Some(r) = weak.upgrade()
                            && r.is_collapsible()
                        {
                            r.toggle_minimized();
                            e.set_handled(true);
                        }
                    },
                );
                let weak = self.to_ref().downgrade();
                h.add_handler(
                    ferroui_base::input::InputElement::right_tapped_event(),
                    move |sender, e| {
                        if let Some(root) = weak.upgrade() {
                            let header = sender
                                .to_ref()
                                .cast::<Control>()
                                .expect("tab header context target");
                            root.show_item_context_menu(
                                header.clone(),
                                e.get_position(Some(&header)),
                            )
                            .expect("tab header context menu");
                            e.set_handled(true);
                        }
                    },
                );
                h
            });
            let cached = self.tab_header_cache.borrow().contains(&header);
            if !cached {
                self.tab_header_cache.borrow_mut().push(header.clone());
            }
            header.update_text();
            let group = tab.contextual_group();
            header.set_is_contextual(group.is_some());
            header.set_contextual_header(group.as_ref().and_then(|g| g.header()));
            header.set_contextual_brush(group.as_ref().map(|g| {
                SolidColorBrush::with_color(
                    if self.actual_theme_variant()
                        == Some(ferroui_base::styling::ThemeVariant::dark())
                    {
                        crate::theming::to_color(
                            crate::theming::to_ribbon_color(g.color()).lighten(0.35),
                        )
                    } else {
                        g.color()
                    },
                )
                .into()
            }));
            header.set_is_selected(self.selected_tab().as_ref() == Some(tab));
            header
                .set_is_enabled(tab.is_enabled() && group.as_ref().is_none_or(|g| g.is_enabled()));
            ferroui_base::input::KeyboardNavigation::set_is_tab_stop(&header, header.is_selected());
            self.headers.borrow_mut().push(header.clone());
            self.strip.children().add(header);
        }
        if self
            .selected_tab()
            .as_ref()
            .is_some_and(|t| !ordered.contains(t))
        {
            let last = self
                .last_regular
                .borrow()
                .as_ref()
                .and_then(WeakRef::upgrade)
                .filter(|t| t.is_available() && ordered.contains(t));
            self.set_selected_tab(last.or_else(|| ordered.first().cloned()));
        } else if self.selected_tab().is_none() && self.is_loaded() {
            self.select_first_tab();
        }
        // Publish the retained cache before releasing removed controls: their
        // subscription cleanup can call application code reentrantly.
        let (retained, removed): (Vec<_>, Vec<_>) = self
            .tab_header_cache
            .take()
            .into_iter()
            .partition(|h| self.tabs.contains(&Some(h.tab())));
        let replaced = self.tab_header_cache.replace(retained);
        drop(replaced);
        drop(removed);
        if tab_headers_changed
            && let Some(peer) = ControlAutomationPeer::from_element(self)
                .and_then(|peer| peer.cast::<ControlAutomationPeer>())
        {
            // AT-SPI exposes these headers as children of the selection container.
            // The nested strip's visual updates do not refresh that child list.
            peer.invalidate_children();
        }
    }
    fn on_header_key_down(&self, header: &RibbonTabHeader, key: ferroui_base::input::Key) -> bool {
        use ferroui_base::{
            input::{FocusManager, Key, KeyModifiers, NavigationMethod},
            media::FlowDirection,
        };
        let headers = self.headers.borrow().clone();
        let Some(index) = headers.iter().position(|h| h.ptr_eq(&header.to_ref())) else {
            return false;
        };
        let rtl = self.flow_direction() == FlowDirection::RightToLeft;
        let target = match key {
            Key::Left => Some(if rtl {
                (index + 1) % headers.len()
            } else {
                (index + headers.len() - 1) % headers.len()
            }),
            Key::Right => Some(if rtl {
                (index + headers.len() - 1) % headers.len()
            } else {
                (index + 1) % headers.len()
            }),
            Key::Home => Some(0),
            Key::End => Some(headers.len() - 1),
            _ => None,
        };
        let Some(target) = target else {
            if key == Key::Down
                && let Some(tab) = self.selected_tab()
            {
                if self.visibility_mode() == ferroribbon_core::RibbonVisibilityMode::TabsOnly {
                    self.open_minimized_popup();
                }
                return FocusManager::find_first_focusable_element_in(&tab.upcast())
                    .and_then(|e| e.cast::<Control>())
                    .is_some_and(|c| c.focus_with(NavigationMethod::Tab, KeyModifiers::NONE));
            }
            return false;
        };
        let next = headers[target].clone();
        self.set_selected_tab(Some(next.tab()));
        next.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
        true
    }
    fn select_first_tab(&self) {
        if let Some(tab) = self
            .selected_tab_id()
            .and_then(|id| self.find_tab(&id))
            .filter(|t| t.is_available())
        {
            self.set_selected_tab(Some(tab));
            return;
        }
        let first = self.headers.borrow().first().map(|h| h.tab()).or_else(|| {
            self.tabs
                .iter()
                .map(|t| t.expect("null RibbonTab"))
                .find(|t| t.is_available())
        });
        self.set_selected_tab(first);
    }
    fn on_selected_tab_id_changed(&self) {
        if self.updating_selection.get() {
            return;
        }
        let Some(id) = self.selected_tab_id() else {
            return;
        };
        if let Some(tab) = self.find_tab(&id).filter(|t| t.is_available())
            && self.selected_tab().as_ref() != Some(&tab)
        {
            self.set_selected_tab(Some(tab));
        } else if let Some(current) = self.selected_tab()
            && current.effective_id().as_deref() != Some(&id)
        {
            self.updating_selection.set(true);
            self.set_selected_tab_id(current.effective_id());
            self.updating_selection.set(false);
        }
    }
    fn on_selected_tab_changed(&self, old: Option<Ref<RibbonTab>>, new: Option<Ref<RibbonTab>>) {
        if let Some(tab) = &old {
            tab.set_is_selected(false);
            tab.set_is_visible(false);
            for g in tab.groups().iter() {
                let group = g.expect("null RibbonGroup");
                group.close_popup();
                group.close_slide_out();
            }
        }
        if let Some(tab) = &new {
            tab.set_is_selected(true);
            tab.set_is_visible(true);
            if !tab.is_contextual() {
                self.last_regular.replace(Some(tab.downgrade()));
            }
            self.present_tab(tab);
            for group in tab.groups().iter() {
                let group = group.expect("null group");
                if group.is_slide_out_pinned() {
                    ferroui_base::threading::Dispatcher::ui_thread().post_local(
                        move || group.open_slide_out(),
                        ferroui_base::threading::DispatcherPriority::NORMAL,
                    );
                }
            }
        }
        let headers = self.headers.borrow().clone();
        for (i, h) in headers.iter().enumerate() {
            h.set_is_selected(new.as_ref() == Some(&h.tab()));
            ferroui_base::input::KeyboardNavigation::set_is_tab_stop(
                h,
                h.is_selected() || (new.is_none() && i == 0),
            );
        }
        self.updating_selection.set(true);
        self.set_selected_tab_id(new.as_ref().and_then(|t| t.effective_id()));
        self.updating_selection.set(false);
        if let Some(element) = new.as_ref().and_then(|t| t.header_element())
            && let Some(scroll) = self.parts.borrow().scroll.clone()
        {
            scroll.bring_into_view(&element.upcast());
        }
        let event = self.selected_tab_changed.borrow().clone();
        if let Some(event) = event {
            event
                .try_invoke(&[
                    Some(Control::boxed(self.to_ref())),
                    Some(Rc::new(Rc::new(RibbonTabChangedEventArgs {
                        old_tab: old,
                        new_tab: new.clone(),
                    }))),
                ])
                .expect("SelectedTabChanged handler");
        }
        self.on_model_selected_tab_changed(new);
    }
    fn present_tab(&self, tab: &RibbonTab) {
        tab.apply_presentation(
            self.metrics(),
            self.display_mode() == RibbonDisplayMode::Simplified,
            self.is_adaptive_layout_enabled(),
            self.current_panel_presentation(),
            self.show_group_captions(),
            self.reduction_strategy(),
        );
    }
    fn apply_presentation(&self) {
        for tab in self.tabs.iter() {
            self.present_tab(&tab.expect("null RibbonTab"));
        }
        for item in self.tab_strip_items.iter().flatten() {
            if let Some(item) = ferroui_base::data::core::ValueTypes::try_cast(
                &Control::boxed(item),
                ferroui_base::data::core::ValueType::of::<Rc<dyn super::IRibbonItem>>(),
            )
            .and_then(|v| v.downcast_ref::<Rc<dyn super::IRibbonItem>>().cloned())
            {
                item.apply_layout(super::RibbonItemLayout {
                    size: if self.compact_tab_strip.get() {
                        ferroribbon_core::RibbonItemSize::Small
                    } else {
                        ferroribbon_core::RibbonItemSize::Medium
                    },
                    metrics: Some(self.metrics()),
                    is_simplified: true,
                    show_label: if self.compact_tab_strip.get() {
                        Some(false)
                    } else {
                        None
                    },
                    ..Default::default()
                });
            }
        }
        if let Some(row) = self.parts.borrow().tab_row.clone() {
            row.set_min_height(self.metrics().tab_height());
        }
        self.pseudo_classes().set(
            ":simplified",
            self.display_mode() == RibbonDisplayMode::Simplified,
        );
    }
    fn update_chrome(&self) {
        if let Some(button) = self.application_button() {
            button.set_is_visible(self.is_application_button_visible());
            let target = self
                .application_button()
                .expect("application button changed during visibility update");
            let content = self
                .application_button_content()
                .or_else(|| {
                    self.application_button_label()
                        .map(|s| Rc::new(s) as BoxedValue)
                })
                .or_else(|| {
                    ferroribbon_core::localization::RibbonStrings::current()
                        .file()
                        .map(|s| Rc::new(s.to_utf8().expect("native UTF-16")) as BoxedValue)
                });
            target.set_content(content);
            let target = self
                .application_button()
                .expect("application button changed during content update");
            let label = self.application_button_label().or_else(|| {
                ferroribbon_core::localization::RibbonStrings::current()
                    .file()
                    .map(|s| s.to_utf8().expect("native UTF-16"))
            });
            AutomationProperties::set_name(&target, label.as_deref());
        }
        if let Some(b) = self.display_options_button() {
            b.set_is_visible(self.is_display_options_button_visible());
        }
        self.update_visibility_chrome();
    }
    pub fn invalidate_layout(&self) {
        for tab in self.tabs.iter() {
            let tab = tab.expect("null RibbonTab");
            for g in tab.groups().iter() {
                g.expect("null RibbonGroup").invalidate_items_layout();
            }
            tab.groups_panel().invalidate_cache();
        }
    }
    pub fn add_selected_tab_changed(&self, handler: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(
            self.selected_tab_changed.borrow().as_ref(),
            handler.as_ref(),
        );
        self.selected_tab_changed.replace(next);
    }
    pub fn remove_selected_tab_changed(&self, handler: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(
            self.selected_tab_changed.borrow().as_ref(),
            handler.as_ref(),
        );
        self.selected_tab_changed.replace(next);
    }
    pub fn add_item_invoked(&self, handler: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(self.item_invoked.borrow().as_ref(), handler.as_ref());
        self.item_invoked.replace(next);
    }
    pub fn remove_item_invoked(&self, handler: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(self.item_invoked.borrow().as_ref(), handler.as_ref());
        self.item_invoked.replace(next);
    }
}
impl StyledElementImpl for Ribbon {
    fn style_key_override(_this: &Self) -> &'static ferroui_base::TypeInfo {
        <Self as ferroui_base::StaticType>::TYPE
    }
}
impl FerroObjectImpl for Ribbon {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        RibbonTheme::ensure_resources();
        ferroui_base::input::KeyboardNavigation::set_is_tab_stop(this, false);
        AutomationProperties::set_name(this, Some("Ribbon"));
        let weak = this.to_ref().downgrade();
        this.actual_theme_variant_changed(move || {
            if let Some(r) = weak.upgrade() {
                r.on_tabs_changed();
                r.update_chrome();
                r.refresh_backstage_popup_theme();
            }
        })
        .detach();
        let weak = this.to_ref().downgrade();
        this.tab_strip_items
            .add_collection_changed(Rc::new(move |_| {
                if let Some(r) = weak.upgrade() {
                    r.sync_tab_strip_items();
                }
            }));
        let weak = this.to_ref().downgrade();
        this.size_changed(move |_, e| {
            if let Some(r) = weak.upgrade() {
                r.close_minimized_popup();
                let compact = e.new_size().width < r.compact_tab_strip_width();
                if compact != r.compact_tab_strip.get() {
                    r.compact_tab_strip.set(compact);
                    r.apply_presentation();
                }
            }
        });
        let weak = this.to_ref().downgrade();
        this.tabs.add_collection_changed(Rc::new(move |_| {
            if let Some(r) = weak.upgrade() {
                r.tabs_changed();
            }
        }));
        let weak = this.to_ref().downgrade();
        this.contextual_groups
            .add_collection_changed(Rc::new(move |_| {
                if let Some(r) = weak.upgrade() {
                    r.contextual_groups_changed();
                }
            }));
        this.set_quick_access_tool_bar(Some(super::RibbonQuickAccessToolBar::new()));
    }
}
impl ControlImpl for Ribbon {
    fn on_create_automation_peer(
        this: &Self,
    ) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        super::RibbonAutomationPeer::new(this).upcast()
    }
    fn on_unloaded(this: &Self, e: &ferroui_base::interactivity::RoutedEventArgs) {
        this.detach_visibility_input();
        this.hide_key_tips();
        this.close_minimized_popup();
        this.detach_strings();
        this.detach_command_catalog();
        this.on_model_unloaded();
        if this.is_backstage_open() {
            this.set_is_backstage_open(false);
        }
        for tab in this.tabs.iter() {
            for group in tab.expect("null tab").groups().iter() {
                let group = group.expect("null group");
                group.close_popup();
                group.close_slide_out();
            }
        }
        this.suspend_floating_groups();
        // Control's base override raises the public event. Upstream installs
        // its own Unloaded handler in the constructor, before application
        // handlers: cleanup must finish before those handlers run.
        <Self as ferroui_controls::ControlImplExt>::parent_on_unloaded(this, e);
    }
    fn on_loaded(this: &Self, e: &ferroui_base::interactivity::RoutedEventArgs) {
        this.attach_visibility_input();
        this.attach_strings();
        this.attach_command_catalog();
        this.on_model_loaded();
        this.capture_quick_access_defaults();
        if this.selected_tab().is_none() {
            this.select_first_tab();
        }
        if this.is_backstage_open() {
            this.open_backstage_core();
        }
        this.show_floating_groups();
        // Match the constructor-installed source handler's ordering, including
        // a Current change made by an application's Loaded handler.
        <Self as ferroui_controls::ControlImplExt>::parent_on_loaded(this, e);
    }
}
impl TemplatedControlImpl for Ribbon {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        this.remove_part_handlers(PartClick::Application);
        this.remove_part_handlers(PartClick::Options);
        this.remove_part_handlers(PartClick::Reveal);
        let scroll = this.parts.borrow().scroll.clone();
        if let Some(scroll) = scroll {
            scroll.set_content(None::<Ref<Control>>);
        }
        let host = this.parts.borrow().content.clone();
        if let Some(host) = host {
            host.children().clear();
        }
        Self::parent_on_apply_template(this, e);
        // Match source field publication order; minimize-label callbacks can
        // reenter before the new command/reveal bars are published.
        macro_rules! part {
            ($field:ident,$name:literal) => {{
                let value = e.name_scope().find($name).and_then(|v| v.cast());
                let old = std::mem::replace(&mut this.parts.borrow_mut().$field, value);
                drop(old);
            }};
        }
        part!(content, "PART_TabContentHost");
        part!(scroll, "PART_TabStripScroll");
        part!(application, "PART_ApplicationButton");
        part!(options, "PART_DisplayOptionsButton");
        this.remove_part_handlers(PartClick::Minimize);
        this.remove_part_handlers(PartClick::Behavior);
        part!(minimize_buttons, "PART_MinimizeButtons");
        part!(minimize, "PART_MinimizeButton");
        part!(minimize_behavior, "PART_MinimizeBehaviorButton");
        part!(minimize_glyph, "PART_MinimizeGlyph");
        this.attach_minimize_parts();
        this.update_minimize_button();
        part!(reveal, "PART_RevealButton");
        part!(tab_row, "PART_TabRow");
        part!(command_bar, "PART_CommandBar");
        part!(reveal_bar, "PART_RevealBar");
        part!(quick_access_above, "PART_QuickAccessAboveHost");
        part!(quick_access_below, "PART_QuickAccessBelowHost");
        part!(strip_items_host, "PART_TabStripItemsHost");
        let scroll = this.parts.borrow().scroll.clone();
        if let Some(scroll) = scroll {
            scroll.set_content(this.strip.clone());
        }
        if let Some(button) = this.application_button() {
            let weak = this.to_ref().downgrade();
            let token = button.click(move |_, _| {
                if let Some(root) = weak.upgrade() {
                    root.invoke_application_button();
                }
            });
            this.parts
                .borrow_mut()
                .click_handlers
                .push((PartClick::Application, button, token));
        }
        if let Some(button) = this.display_options_button() {
            let weak = this.to_ref().downgrade();
            let token = button.click(move |sender, _| {
                if let Some(ribbon) = weak.upgrade() {
                    ribbon.show_display_options_menu(
                        &sender
                            .to_ref()
                            .cast::<Control>()
                            .expect("display-options anchor"),
                    );
                }
            });
            this.parts
                .borrow_mut()
                .click_handlers
                .push((PartClick::Options, button, token));
            this.refresh_display_options_strings();
        }
        this.attach_reveal_part();
        this.sync_tab_content();
        this.sync_tab_strip_items();
        this.on_tabs_changed();
        this.update_quick_access_placement();
        this.update_chrome();
        this.apply_presentation();
    }
}
impl Drop for Ribbon {
    fn drop(&mut self) {
        self.detach_visibility_input();
        self.close_minimized_popup();
        for (g, token) in self.contextual_subscriptions.get_mut().drain(..) {
            g.remove_property_changed_handler(token);
        }
    }
}
impl Ribbon {
    pub fn can_float_groups(&self) -> bool {
        self.get_value(Self::can_float_groups_property())
    }
    pub fn set_can_float_groups(&self, value: bool) {
        self.set_value(Self::can_float_groups_property(), value);
    }
    pub fn floating_groups(&self) -> crate::metadata::ribbon_collections::RibbonFloatingGroups {
        crate::metadata::ribbon_collections::floating_groups_snapshot(
            self.tabs
                .iter()
                .flat_map(|t| t.expect("null tab").groups().iter())
                .filter(|g| g.as_ref().expect("null group").is_floating())
                .collect(),
        )
    }
    pub fn return_all_panels_to_ribbon(&self) {
        for group in self.floating_groups().iter() {
            group.expect("null floating group").return_to_ribbon();
        }
    }
    fn show_floating_groups(&self) {
        for group in self.floating_groups().iter() {
            group.expect("null floating group").show_float();
        }
    }
    fn suspend_floating_groups(&self) {
        for group in self.floating_groups().iter() {
            group.expect("null floating group").suspend_float();
        }
    }
    pub(crate) fn on_group_floating_changed(&self, group: Ref<super::RibbonGroup>) {
        self.invalidate_shortcuts();
        let h = self.group_floating_changed.borrow().clone();
        if let Some(h) = h {
            h.try_invoke(&[
                Some(Control::boxed(self.to_ref())),
                Some(Control::boxed(group)),
            ])
            .expect("GroupFloatingChanged handler");
        }
        self.raise_state_changed();
    }
    pub fn add_group_floating_changed(&self, h: Option<MarkupDelegate>) {
        let next =
            MarkupDelegate::combine(self.group_floating_changed.borrow().as_ref(), h.as_ref());
        self.group_floating_changed.replace(next);
    }
    pub fn remove_group_floating_changed(&self, h: Option<MarkupDelegate>) {
        let next =
            MarkupDelegate::remove(self.group_floating_changed.borrow().as_ref(), h.as_ref());
        self.group_floating_changed.replace(next);
    }
    pub fn suspend_popups(&self) {
        self.cancel_key_tips();
        self.close_minimized_popup();
        if self.is_backstage_open() {
            self.set_is_backstage_open(false);
        }
        for tab in self.tabs.iter() {
            for group in tab.expect("null tab").groups().iter() {
                let group = group.expect("null group");
                group.close_popup();
                group.close_slide_out();
            }
        }
        self.suspend_floating_groups();
    }
    pub fn resume_popups(&self) {
        if !self.is_loaded() || !self.is_visible() {
            return;
        }
        self.show_floating_groups();
        if let Some(tab) = self.selected_tab() {
            for group in tab.groups().iter() {
                let group = group.expect("null group");
                if group.is_slide_out_pinned() {
                    ferroui_base::threading::Dispatcher::ui_thread().post_local(
                        move || group.open_slide_out(),
                        ferroui_base::threading::DispatcherPriority::NORMAL,
                    );
                }
            }
        }
    }
    pub fn on_item_invoked(
        &self,
        item: Ref<Control>,
        command_id: Option<RibbonString>,
        parameter: Option<BoxedValue>,
    ) {
        self.on_visibility_item_invoked(&item);
        if let Some(tab) = self.selected_tab() {
            for group in tab.groups().iter() {
                let group = group.expect("null RibbonGroup");
                if group.is_popup_open() && !group.to_ref().upcast::<Control>().ptr_eq(&item) {
                    group.close_popup();
                }
            }
        }
        if self.visibility_mode() == RibbonVisibilityMode::FullScreen {
            self.set_is_full_screen_revealed(false);
        }
        if let Some(item) = ferroui_base::data::core::ValueTypes::try_cast(
            &Control::boxed(item.clone()),
            ferroui_base::data::core::ValueType::of::<Rc<dyn super::IRibbonItem>>(),
        )
        .and_then(|v| v.downcast_ref::<Rc<dyn super::IRibbonItem>>().cloned())
            && let Some(id) = item.id()
        {
            self.search_engine.mark_used(Some(id));
        }
        let handler = self.item_invoked.borrow().clone();
        if let Some(handler) = handler {
            let item_id = ferroui_base::data::core::ValueTypes::try_cast(
                &Control::boxed(item.clone()),
                ferroui_base::data::core::ValueType::of::<Rc<dyn super::IRibbonItem>>(),
            )
            .and_then(|v| v.downcast_ref::<Rc<dyn super::IRibbonItem>>().cloned())
            .and_then(|i| i.id())
            .or_else(|| {
                item.cast::<super::RibbonGroup>()
                    .and_then(|g| g.id().map(Into::into))
            });
            handler
                .try_invoke(&[
                    Some(Control::boxed(self.to_ref())),
                    Some(Rc::new(Rc::new(RibbonItemInvokedEventArgs {
                        item,
                        command_id,
                        item_id,
                        parameter,
                    }))),
                ])
                .expect("ItemInvoked handler");
        }
    }
    pub fn add_state_changed(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(self.state_changed.borrow().as_ref(), h.as_ref());
        self.state_changed.replace(next);
    }
    pub fn remove_state_changed(&self, h: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(self.state_changed.borrow().as_ref(), h.as_ref());
        self.state_changed.replace(next);
    }
    pub fn raise_state_changed(&self) {
        if self.is_loaded() {
            let h = self.state_changed.borrow().clone();
            if let Some(h) = h {
                h.try_invoke(&[
                    Some(Control::boxed(self.to_ref())),
                    Some(Rc::new(ferroui_base::utilities::EventArgs::EMPTY)),
                ])
                .expect("StateChanged handler");
            }
        }
    }
}
struct RibbonOwner(Ref<Ribbon>);
impl IRibbonItemOwner for RibbonOwner {
    fn owner_ribbon(&self) -> Option<Ref<Ribbon>> {
        Some(self.0.clone())
    }
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
    fn metrics(&self) -> Option<Arc<RibbonMetrics>> {
        Some(self.0.metrics())
    }
    fn command_catalog(&self) -> Option<Rc<RibbonCommandCatalog>> {
        self.0.command_catalog()
    }
    fn on_item_invoked(
        &self,
        item: Ref<Control>,
        command_id: Option<RibbonString>,
        parameter: Option<BoxedValue>,
    ) {
        self.0.on_item_invoked(item, command_id, parameter);
    }
}
pub struct RibbonTabChangedEventArgs {
    pub old_tab: Option<Ref<RibbonTab>>,
    pub new_tab: Option<Ref<RibbonTab>>,
}
impl PartialEq for RibbonTabChangedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class RibbonTabChangedEventArgs {namespace:"FerroRibbon.Controls",handles:[Rc<RibbonTabChangedEventArgs>],this:Rc<RibbonTabChangedEventArgs>,properties:[OldTab:Option<Ref<RibbonTab>>{get:|this:&Rc<RibbonTabChangedEventArgs>|this.old_tab.clone()},NewTab:Option<Ref<RibbonTab>>{get:|this:&Rc<RibbonTabChangedEventArgs>|this.new_tab.clone()}]});
pub struct RibbonItemInvokedEventArgs {
    pub item: Ref<Control>,
    pub command_id: Option<RibbonString>,
    pub item_id: Option<RibbonString>,
    pub parameter: Option<BoxedValue>,
}
impl PartialEq for RibbonItemInvokedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class RibbonItemInvokedEventArgs {namespace:"FerroRibbon.Controls",handles:[Rc<RibbonItemInvokedEventArgs>],this:Rc<RibbonItemInvokedEventArgs>,properties:[CommandId:Option<String>{try_get:|this:&Rc<RibbonItemInvokedEventArgs>|this.command_id.as_ref().map(|s|s.to_utf8()).transpose()},ItemId:Option<String>{try_get:|this:&Rc<RibbonItemInvokedEventArgs>|this.item_id.as_ref().map(|s|s.to_utf8()).transpose()},Item:Ref<Control>{get:|this:&Rc<RibbonItemInvokedEventArgs>|this.item.clone()},Parameter:Option<BoxedValue>{get:|this:&Rc<RibbonItemInvokedEventArgs>|this.parameter.clone()}]});
