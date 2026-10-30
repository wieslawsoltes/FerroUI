//! Accessibility adapters: reuse the framework providers and add inking-specific patterns.
use super::ink_toolbar_buttons::{strings, toolbar_is_inking};
use super::{
    InkToolbar, InkToolbarFlyoutItem, InkToolbarFlyoutItemKind, InkToolbarMenuButton,
    InkToolbarStencilButton, InkToolbarStencilKind, InkToolbarStrokeWidthSlider, InkToolbarTool,
    InkToolbarToolButton,
};
use ferroui_controls::automation::peers::{
    AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt,
    ControlAutomationPeer, ControlAutomationPeerImpl, RadioButtonAutomationPeer,
    RadioButtonAutomationPeerImpl, RangeBaseAutomationPeerImpl, RangeBaseAutomationPeerImplExt,
    SliderAutomationPeer, ToggleButtonAutomationPeer,
};
use ferroui_controls::automation::provider::{
    IExpandCollapseProvider, IInvokeProvider, ISelectionItemProvider, ISelectionProvider,
    IToggleProvider, IValueProvider, ProviderError,
};
use ferroui_controls::automation::{
    AutomationProperties, ElementNotEnabledException, ExpandCollapsePatternIdentifiers,
    ExpandCollapseState, ValuePatternIdentifiers,
};
use ferroui_controls::primitives::{RangeBase, ToggleButton};
use ferroui_controls::{Button, Panel};
use ferroui_base::data::core::ValueType;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl,
    FerroObjectImplExt, FerroPropertyChangedEventArgs, ObjectType, Ref,
};
use std::rc::Rc;

/// The handle through which a peer of this crate implements a provider contract, as
/// `ferroui_controls::automation::provider::Provider` does in the controls crate (whose
/// implementations cannot be written outside it): the peer class lists the conversion of this
/// type in the `interfaces` of its class information.
struct Provider<T: ObjectType>(Ref<T>);

macro_rules! conversions {
    ($($name:ident: $provider:ident),* $(,)?) => {
        impl<T: ObjectType> Provider<T> {
            $(
                fn $name(peer: Ref<T>) -> Rc<dyn $provider>
                where
                    Self: $provider,
                {
                    Rc::new(Self(peer))
                }
            )*
        }
    };
}

conversions! {
    as_expand_collapse_provider: IExpandCollapseProvider,
    as_invoke_provider: IInvokeProvider,
    as_selection_item_provider: ISelectionItemProvider,
    as_value_provider: IValueProvider,
}

fn button_state(owner: &Button) -> ExpandCollapseState {
    match owner.flyout() {
        None => ExpandCollapseState::LeafNode,
        Some(f) if f.is_open() => ExpandCollapseState::Expanded,
        _ => ExpandCollapseState::Collapsed,
    }
}
fn append_text(base: Option<String>, suffix: String) -> Option<String> {
    Some(match base.filter(|value| !value.is_empty()) {
        Some(base) if !suffix.is_empty() => format!("{base}, {suffix}"),
        Some(base) => base,
        None => suffix,
    })
}
fn is_builtin_pen(owner: &Button) -> bool {
    owner
        .to_ref()
        .cast::<InkToolbarToolButton>()
        .is_some_and(|owner| {
            matches!(
                owner.tool_kind(),
                InkToolbarTool::BallpointPen | InkToolbarTool::Pencil | InkToolbarTool::Highlighter
            )
        })
}
fn menu_has_dropdown(owner: &Button) -> bool {
    owner.flyout().is_some()
        && owner
            .to_ref()
            .cast::<InkToolbarStencilButton>()
            .is_none_or(|owner| owner.is_ruler_item_visible() && owner.is_protractor_item_visible())
}
fn is_radio_item(owner: &InkToolbarFlyoutItem) -> bool {
    matches!(
        owner.kind(),
        InkToolbarFlyoutItemKind::Radio | InkToolbarFlyoutItemKind::RadioCheck
    )
}
// The existing framework selection-provider contract has a single error type.
// Preserve rejection without panicking; native UIA_E_INVALIDOPERATION remains
// distinguishable by the message, not by a separate HRESULT category.
fn invalid_selection() -> ElementNotEnabledException {
    ElementNotEnabledException::with_message("The requested selection operation is not supported.")
}
fn expand(owner: &Button) -> Result<(), ElementNotEnabledException> {
    if !owner.is_effectively_enabled() {
        return Err(ElementNotEnabledException::new());
    }
    // Native peers route through OpenL3/ExecuteToolAction(false), which selects
    // the owner before considering configuration availability. Reuse the same
    // guards as pointer activation, including FerroUI's real inking state (the
    // pinned WinUI lift leaves that state unwired).
    if let Some(tool) = owner.to_ref().cast::<InkToolbarToolButton>() {
        tool.set_current_value(ToggleButton::is_checked_property(), Some(true));
        if !tool.can_open_configuration() {
            return Ok(());
        }
    } else if let Some(menu) = owner.to_ref().cast::<InkToolbarMenuButton>() {
        menu.set_current_value(ToggleButton::is_checked_property(), Some(true));
        if toolbar_is_inking(&menu) {
            return Ok(());
        }
    }
    if let Some(flyout) = owner.flyout() {
        flyout.show_at(owner);
    }
    Ok(())
}
fn collapse(owner: &Button) -> Result<(), ElementNotEnabledException> {
    if !owner.is_effectively_enabled() {
        return Err(ElementNotEnabledException::new());
    }
    if let Some(flyout) = owner.flyout() {
        flyout.hide();
    }
    Ok(())
}
fn observe_flyout(peer: &Ref<AutomationPeer>, owner: &Button) {
    if let Some(flyout) = owner.flyout() {
        let weak_peer = peer.downgrade();
        let weak_owner = owner.to_ref().downgrade();
        let weak_flyout = flyout.downgrade();
        flyout.opened(move || {
            if let (Some(peer), Some(owner), Some(flyout)) = (
                weak_peer.upgrade(),
                weak_owner.upgrade(),
                weak_flyout.upgrade(),
            ) {
                if owner.flyout().as_ref() == Some(&flyout) {
                    peer.raise_property_changed_event(
                        ExpandCollapsePatternIdentifiers::expand_collapse_state_property(),
                        Some(Rc::new(ExpandCollapseState::Collapsed)),
                        Some(Rc::new(ExpandCollapseState::Expanded)),
                    );
                }
            }
        });
        let weak_peer = peer.downgrade();
        let weak_owner = owner.to_ref().downgrade();
        let weak_flyout = flyout.downgrade();
        flyout.closed(move || {
            if let (Some(peer), Some(owner), Some(flyout)) = (
                weak_peer.upgrade(),
                weak_owner.upgrade(),
                weak_flyout.upgrade(),
            ) {
                if owner.flyout().as_ref() == Some(&flyout) {
                    peer.raise_property_changed_event(
                        ExpandCollapsePatternIdentifiers::expand_collapse_state_property(),
                        Some(Rc::new(ExpandCollapseState::Expanded)),
                        Some(Rc::new(ExpandCollapseState::Collapsed)),
                    );
                }
            }
        });
    }
}
fn subscribe_flyout(peer: &Ref<AutomationPeer>, owner: &Button) {
    observe_flyout(peer, owner);
    let weak_peer = peer.downgrade();
    let weak_owner = owner.to_ref().downgrade();
    owner.property_changed(move |change| {
        if change.property() == Button::flyout_property().as_property() {
            if let (Some(peer), Some(owner)) = (weak_peer.upgrade(), weak_owner.upgrade()) {
                observe_flyout(&peer, &owner);
            }
        }
    });
}

#[repr(C)]
pub struct InkToolbarAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(InkToolbarAutomationPeer:ControlAutomationPeer);
ferro_class_info!(InkToolbarAutomationPeer {});
ferro_impl_classes!(InkToolbarAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for InkToolbarAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "InkToolbar".to_owned()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        // As WinUI (`InkToolBarAutomationPeer.cpp`, which matches the peer of WinUI 2).
        AutomationControlType::Pane
    }
}
impl InkToolbarAutomationPeer {
    pub fn construct(owner: &InkToolbar) -> Self {
        Self {
            base: ControlAutomationPeer::construct(owner),
        }
    }
    pub fn new(owner: &InkToolbar) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
}

#[repr(C)]
pub struct InkToolbarToolButtonAutomationPeer {
    base: RadioButtonAutomationPeer,
}
ferro_class!(InkToolbarToolButtonAutomationPeer:RadioButtonAutomationPeer);
ferro_class_info!(InkToolbarToolButtonAutomationPeer{interfaces:[Rc<dyn IExpandCollapseProvider> => Provider::as_expand_collapse_provider,Rc<dyn ISelectionItemProvider> => Provider::as_selection_item_provider]});
ferro_impl_classes!(InkToolbarToolButtonAutomationPeer:ControlAutomationPeerImpl,RadioButtonAutomationPeerImpl);
impl FerroObjectImpl for InkToolbarToolButtonAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        subscribe_flyout(&this.to_ref().upcast(), &this.owner_button());
    }
}
impl AutomationPeerImpl for InkToolbarToolButtonAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "InkToolbarToolButton".to_owned()
    }
    fn get_automation_control_type_core(this: &Self) -> AutomationControlType {
        if is_builtin_pen(&this.owner_button()) {
            AutomationControlType::Button
        } else {
            AutomationControlType::Custom
        }
    }
    fn get_localized_control_type_core(this: &Self) -> String {
        let owner = this.owner_button();
        let key = if is_builtin_pen(&owner) && owner.flyout().is_some() {
            "InkToolbarDropDownButtonControlTypeName"
        } else {
            "InkToolbarToolButtonControlTypeName"
        };
        strings::text(&owner, key)
    }
    fn get_help_text_core(this: &Self) -> Option<String> {
        let owner = this.owner_button();
        if is_builtin_pen(&owner) {
            // Tooltip fallback must not replace an explicitly cleared color
            // description: the native peer then reports only the palette hint.
            append_text(
                AutomationProperties::get_help_text(&owner),
                strings::text(&owner, "InkToolbarColorPaletteHelpText"),
            )
        } else {
            Self::parent_get_help_text_core(this)
        }
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let owner = this.owner_button();
        let name = Self::parent_get_name_core(this);
        if owner
            .cast::<InkToolbarToolButton>()
            .is_some_and(|owner| owner.is_checked() == Some(true))
        {
            append_text(
                name,
                strings::text(&owner, "InkToolbarToolButtonSelectedStateName"),
            )
        } else {
            name
        }
    }
}
impl InkToolbarToolButtonAutomationPeer {
    pub fn construct(owner: &InkToolbarToolButton) -> Self {
        Self {
            base: RadioButtonAutomationPeer::construct(owner),
        }
    }
    pub fn new(owner: &InkToolbarToolButton) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
    fn owner_button(&self) -> Ref<Button> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("tool button peer owner")
    }
}
impl IExpandCollapseProvider for Provider<InkToolbarToolButtonAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn expand_collapse_state(&self) -> ExpandCollapseState {
        // The upstream tool provider retains Collapsed even without L3 content.
        match button_state(&self.0.owner_button()) {
            ExpandCollapseState::LeafNode => ExpandCollapseState::Collapsed,
            state => state,
        }
    }
    fn shows_menu(&self) -> bool {
        true
    }
    fn expand(&self) -> Result<(), ElementNotEnabledException> {
        expand(&self.0.owner_button())
    }
    fn collapse(&self) -> Result<(), ElementNotEnabledException> {
        collapse(&self.0.owner_button())
    }
}
impl ISelectionItemProvider for Provider<InkToolbarToolButtonAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_selected(&self) -> bool {
        self.0.is_selected()
    }
    fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        self.0.selection_container()
    }
    fn add_to_selection(&self) -> Result<(), ElementNotEnabledException> {
        let owner = self.0.owner_button();
        if !owner.is_effectively_enabled() {
            return Err(ElementNotEnabledException::new());
        }
        if self.is_selected() {
            Ok(())
        } else {
            Err(invalid_selection())
        }
    }
    fn remove_from_selection(&self) -> Result<(), ElementNotEnabledException> {
        let owner = self.0.owner_button();
        if !owner.is_effectively_enabled() {
            return Err(ElementNotEnabledException::new());
        }
        if self.is_selected() {
            Err(invalid_selection())
        } else {
            Ok(())
        }
    }
    fn select(&self) -> Result<(), ElementNotEnabledException> {
        let owner = self.0.owner_button();
        if !owner.is_effectively_enabled() {
            return Err(ElementNotEnabledException::new());
        }
        owner
            .cast::<InkToolbarToolButton>()
            .expect("tool peer owner")
            .set_is_checked(Some(true));
        Ok(())
    }
}

#[repr(C)]
pub struct InkToolbarMenuButtonAutomationPeer {
    base: ToggleButtonAutomationPeer,
}
ferro_class!(InkToolbarMenuButtonAutomationPeer:ToggleButtonAutomationPeer);
ferro_class_info!(InkToolbarMenuButtonAutomationPeer{interfaces:[Rc<dyn IExpandCollapseProvider> => Provider::as_expand_collapse_provider]});
ferro_impl_classes!(InkToolbarMenuButtonAutomationPeer:ControlAutomationPeerImpl);
impl FerroObjectImpl for InkToolbarMenuButtonAutomationPeer {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        subscribe_flyout(&this.to_ref().upcast(), &this.owner_button());
    }
}
impl AutomationPeerImpl for InkToolbarMenuButtonAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "InkToolbarMenuButton".to_owned()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::Button
    }
    fn get_localized_control_type_core(this: &Self) -> String {
        let owner = this.owner_button();
        strings::text(
            &owner,
            if menu_has_dropdown(&owner) {
                "InkToolbarStencilDropDownControlTypeName"
            } else {
                "InkToolbarToolButtonControlTypeName"
            },
        )
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let owner = this.owner_button();
        let persistent = strings::text(&owner, "InkToolbarStencilButtonName");
        let mut name = Self::parent_get_name_core(this);
        if let Some(stencil) = owner.cast::<InkToolbarStencilButton>() {
            if name
                .as_deref()
                .is_none_or(|value| value.is_empty() || value == persistent)
            {
                name = Some(strings::text(
                    &owner,
                    match stencil.selected_stencil() {
                        InkToolbarStencilKind::Ruler => "InkToolbarStencilRulerName",
                        InkToolbarStencilKind::Protractor => "InkToolbarStencilProtractorName",
                    },
                ));
            }
        }
        append_text(Some(persistent), name.unwrap_or_default())
    }
    fn get_provider_core(this: &Self, provider_type: ValueType) -> Option<BoxedValue> {
        let dropdown = menu_has_dropdown(&this.owner_button());
        if (provider_type == ValueType::of::<Rc<dyn IExpandCollapseProvider>>() && !dropdown)
            || (provider_type == ValueType::of::<Rc<dyn IToggleProvider>>() && dropdown)
        {
            return None;
        }
        Self::parent_get_provider_core(this, provider_type)
    }
}
impl InkToolbarMenuButtonAutomationPeer {
    pub fn construct(owner: &InkToolbarMenuButton) -> Self {
        Self {
            base: ToggleButtonAutomationPeer::construct(owner),
        }
    }
    pub fn new(owner: &InkToolbarMenuButton) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
    fn owner_button(&self) -> Ref<Button> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("menu button peer owner")
    }
}
impl IExpandCollapseProvider for Provider<InkToolbarMenuButtonAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn expand_collapse_state(&self) -> ExpandCollapseState {
        let owner = self.0.owner_button();
        if menu_has_dropdown(&owner) {
            button_state(&owner)
        } else {
            ExpandCollapseState::LeafNode
        }
    }
    fn shows_menu(&self) -> bool {
        true
    }
    fn expand(&self) -> Result<(), ElementNotEnabledException> {
        let owner = self.0.owner_button();
        if menu_has_dropdown(&owner) {
            expand(&owner)
        } else {
            Ok(())
        }
    }
    fn collapse(&self) -> Result<(), ElementNotEnabledException> {
        collapse(&self.0.owner_button())
    }
}

#[repr(C)]
pub struct InkToolbarFlyoutItemAutomationPeer {
    base: ToggleButtonAutomationPeer,
}
ferro_class!(InkToolbarFlyoutItemAutomationPeer:ToggleButtonAutomationPeer);
ferro_class_info!(InkToolbarFlyoutItemAutomationPeer{interfaces:[Rc<dyn ISelectionItemProvider> => Provider::as_selection_item_provider,Rc<dyn IInvokeProvider> => Provider::as_invoke_provider]});
ferro_impl_classes!(InkToolbarFlyoutItemAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl AutomationPeerImpl for InkToolbarFlyoutItemAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "InkToolbarFlyoutItem".to_owned()
    }
    fn get_automation_control_type_core(_this: &Self) -> AutomationControlType {
        AutomationControlType::MenuItem
    }
    fn get_provider_core(this: &Self, provider_type: ValueType) -> Option<BoxedValue> {
        if provider_type == ValueType::of::<Rc<dyn IToggleProvider>>() {
            // The WinUI item derives from ButtonBase, exposing invocation and
            // (for radio kinds) selection rather than an additional Toggle.
            return None;
        }
        if provider_type == ValueType::of::<Rc<dyn ISelectionItemProvider>>()
            && !is_radio_item(&this.owner_item())
        {
            return None;
        }
        Self::parent_get_provider_core(this, provider_type)
    }
}
impl InkToolbarFlyoutItemAutomationPeer {
    pub fn construct(owner: &InkToolbarFlyoutItem) -> Self {
        Self {
            base: ToggleButtonAutomationPeer::construct(owner),
        }
    }
    pub fn new(owner: &InkToolbarFlyoutItem) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
    fn owner_item(&self) -> Ref<InkToolbarFlyoutItem> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("flyout item peer owner")
    }
    fn ensure_selection_enabled(
        &self,
    ) -> Result<Ref<InkToolbarFlyoutItem>, ElementNotEnabledException> {
        let owner = self.owner_item();
        if !owner.is_effectively_enabled() {
            return Err(ElementNotEnabledException::new());
        }
        if !is_radio_item(&owner) {
            return Err(invalid_selection());
        }
        Ok(owner)
    }
}
impl IInvokeProvider for Provider<InkToolbarFlyoutItemAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn invoke(&self) -> Result<(), ElementNotEnabledException> {
        let owner = self.0.owner_item();
        if !owner.is_effectively_enabled() {
            return Err(ElementNotEnabledException::new());
        }
        owner.on_click();
        Ok(())
    }
}
impl ISelectionItemProvider for Provider<InkToolbarFlyoutItemAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_selected(&self) -> bool {
        let owner = self.0.owner_item();
        is_radio_item(&owner) && owner.is_checked() == Some(true)
    }
    fn selection_container(&self) -> Option<Rc<dyn ISelectionProvider>> {
        None
    }
    fn add_to_selection(&self) -> Result<(), ElementNotEnabledException> {
        let owner = self.0.ensure_selection_enabled()?;
        if owner.is_checked() == Some(true) {
            return Ok(());
        }
        if owner
            .visual_parent()
            .and_then(|parent| parent.cast::<Panel>())
            .is_some_and(|panel| {
                panel
                    .children()
                    .snapshot()
                    .iter()
                    .filter_map(|child| child.cast::<InkToolbarFlyoutItem>())
                    .any(|other| {
                        other != owner && is_radio_item(&other) && other.is_checked() == Some(true)
                    })
            })
        {
            return Err(invalid_selection());
        }
        owner.on_click();
        Ok(())
    }
    fn remove_from_selection(&self) -> Result<(), ElementNotEnabledException> {
        let owner = self.0.ensure_selection_enabled()?;
        if owner.is_checked() == Some(true) {
            if owner.kind() == InkToolbarFlyoutItemKind::Radio {
                return Err(invalid_selection());
            }
            owner.on_click();
        }
        Ok(())
    }
    fn select(&self) -> Result<(), ElementNotEnabledException> {
        let owner = self.0.ensure_selection_enabled()?;
        if owner.is_checked() != Some(true) {
            owner.on_click();
        }
        Ok(())
    }
}

#[repr(C)]
pub struct InkToolbarStrokeWidthSliderAutomationPeer {
    base: SliderAutomationPeer,
}
ferro_class!(InkToolbarStrokeWidthSliderAutomationPeer:SliderAutomationPeer);
ferro_class_info!(InkToolbarStrokeWidthSliderAutomationPeer{interfaces:[Rc<dyn IValueProvider> => Provider::as_value_provider]});
ferro_impl_classes!(InkToolbarStrokeWidthSliderAutomationPeer:FerroObjectImpl,ControlAutomationPeerImpl);
impl RangeBaseAutomationPeerImpl for InkToolbarStrokeWidthSliderAutomationPeer {
    fn owner_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_owner_property_changed(this, change);
        if change.property() == RangeBase::value_property().as_property() {
            this.raise_property_changed_event(
                ValuePatternIdentifiers::value_property(),
                change
                    .get_old_value::<f64>()
                    .map(|value| Rc::new(value.to_string()) as BoxedValue),
                Some(Rc::new(change.get_new_value::<f64>().to_string()) as BoxedValue),
            );
        }
    }
}
impl AutomationPeerImpl for InkToolbarStrokeWidthSliderAutomationPeer {
    fn get_class_name_core(_this: &Self) -> String {
        "InkToolbarStrokeWidthSlider".to_owned()
    }
    fn get_name_core(this: &Self) -> Option<String> {
        let owner = this.owner_slider();
        let range = strings::text(&owner, "InkToolbarStrokeWidthSliderRangeFormat")
            .replace("%1!s!", &owner.minimum().to_string())
            .replace("%2!s!", &owner.maximum().to_string());
        append_text(Self::parent_get_name_core(this), range)
    }
}
impl InkToolbarStrokeWidthSliderAutomationPeer {
    pub fn construct(owner: &InkToolbarStrokeWidthSlider) -> Self {
        Self {
            base: SliderAutomationPeer::construct(owner),
        }
    }
    pub fn new(owner: &InkToolbarStrokeWidthSlider) -> Ref<Self> {
        instantiate(Self::construct(owner))
    }
    fn owner_slider(&self) -> Ref<InkToolbarStrokeWidthSlider> {
        ControlAutomationPeer::owner(self)
            .cast()
            .expect("stroke width peer owner")
    }
}
impl IValueProvider for Provider<InkToolbarStrokeWidthSliderAutomationPeer> {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }
    fn is_read_only(&self) -> bool {
        !self.0.owner_slider().is_effectively_enabled()
    }
    fn value(&self) -> Option<String> {
        Some(self.0.owner_slider().value().to_string())
    }
    fn set_value(&self, value: Option<&str>) -> Result<(), ProviderError> {
        let owner = self.0.owner_slider();
        if !owner.is_effectively_enabled() {
            return Err(Box::new(ElementNotEnabledException::new()));
        }
        let value = value.unwrap_or("").parse::<f64>()?;
        if !value.is_finite() {
            return Err(Box::new(super::InkError::InvalidInput(
                "Stroke width must be finite",
            )));
        }
        owner.set_range_value(value.clamp(owner.minimum(), owner.maximum()));
        Ok(())
    }
}
