//! Generated-common source maps here; registered property IDs live in the parent single
//! ferro_properties block required by FerroUI.
use super::super::{IRibbonItem, IRibbonItemEditable, RibbonItemHelper};
use super::*;
use ferroribbon_core::RibbonString;
use ferroui_base::FerroObject;

impl RibbonControlBase {
    pub fn id(&self) -> Option<String> {
        self.get_value(Self::id_property())
    }
    pub fn set_id(&self, value: Option<String>) {
        self.set_value(Self::id_property(), value)
    }
    pub fn label(&self) -> Option<String> {
        self.get_value(Self::label_property())
    }
    pub fn set_label(&self, value: Option<String>) {
        self.set_value(Self::label_property(), value)
    }
    pub fn icon(&self) -> Option<BoxedValue> {
        self.get_value(Self::icon_property())
    }
    pub fn set_icon(&self, value: Option<BoxedValue>) {
        self.set_value(Self::icon_property(), value)
    }
    pub fn large_icon(&self) -> Option<BoxedValue> {
        self.get_value(Self::large_icon_property())
    }
    pub fn set_large_icon(&self, value: Option<BoxedValue>) {
        self.set_value(Self::large_icon_property(), value)
    }
    pub fn size(&self) -> RibbonItemSize {
        self.get_value(Self::size_property())
    }
    pub fn set_size(&self, value: RibbonItemSize) {
        self.set_value(Self::size_property(), value)
    }
    pub fn size_definition(&self) -> Option<String> {
        self.get_value(Self::size_definition_property())
    }
    pub fn set_size_definition(&self, value: Option<String>) {
        self.set_value(Self::size_definition_property(), value)
    }
    pub fn current_size(&self) -> RibbonItemSize {
        self.get_value(Self::current_size_property())
    }
    pub fn set_current_size(&self, value: RibbonItemSize) {
        self.set_value(Self::current_size_property(), value)
    }
    pub fn is_simplified(&self) -> bool {
        self.get_value(Self::is_simplified_property())
    }
    pub fn set_is_simplified(&self, value: bool) {
        self.set_value(Self::is_simplified_property(), value)
    }
    pub fn actual_show_label(&self) -> bool {
        self.get_value(Self::actual_show_label_property())
    }
    pub fn set_actual_show_label(&self, value: bool) {
        self.set_value(Self::actual_show_label_property(), value)
    }
    pub fn key_tip(&self) -> Option<String> {
        self.get_value(Self::key_tip_property())
    }
    pub fn set_key_tip(&self, value: Option<String>) {
        self.set_value(Self::key_tip_property(), value)
    }
    pub fn screen_tip(&self) -> Option<BoxedValue> {
        self.get_value(Self::screen_tip_property())
    }
    pub fn set_screen_tip(&self, value: Option<BoxedValue>) {
        self.set_value(Self::screen_tip_property(), value)
    }
    pub fn command_id(&self) -> Option<String> {
        self.get_value(Self::command_id_property())
    }
    pub fn set_command_id(&self, value: Option<String>) {
        self.set_value(Self::command_id_property(), value)
    }
    pub fn shortcut(&self) -> Option<String> {
        self.get_value(Self::shortcut_property())
    }
    pub fn set_shortcut(&self, value: Option<String>) {
        self.set_value(Self::shortcut_property(), value)
    }
    pub fn simplified_visibility(&self) -> RibbonSimplifiedVisibility {
        self.get_value(Self::simplified_visibility_property())
    }
    pub fn set_simplified_visibility(&self, value: RibbonSimplifiedVisibility) {
        self.set_value(Self::simplified_visibility_property(), value)
    }
    pub fn simplified_label(&self) -> RibbonSimplifiedLabel {
        self.get_value(Self::simplified_label_property())
    }
    pub fn set_simplified_label(&self, value: RibbonSimplifiedLabel) {
        self.set_value(Self::simplified_label_property(), value)
    }
    pub fn show_label(&self) -> bool {
        self.get_value(Self::show_label_property())
    }
    pub fn set_show_label(&self, value: bool) {
        self.set_value(Self::show_label_property(), value)
    }
    pub fn can_add_to_quick_access(&self) -> bool {
        self.get_value(Self::can_add_to_quick_access_property())
    }
    pub fn set_can_add_to_quick_access(&self, value: bool) {
        self.set_value(Self::can_add_to_quick_access_property(), value)
    }
    pub fn metrics(&self) -> Option<Arc<RibbonMetrics>> {
        self.get_value(Self::metrics_property())
    }
    pub fn set_metrics(&self, value: Option<Arc<RibbonMetrics>>) {
        self.set_value(Self::metrics_property(), value)
    }

    pub fn effective_size_definition(&self) -> RibbonSizeDefinition {
        RibbonSizeDefinition::try_parse(self.size_definition().as_deref())
            .unwrap_or_else(|| RibbonSizeDefinition::for_preferred_size(self.size()))
    }
    pub fn show_label_in_simplified_mode(&self) -> bool {
        match self.simplified_label() {
            RibbonSimplifiedLabel::Show => true,
            RibbonSimplifiedLabel::Hide => false,
            _ => self.size() == RibbonItemSize::Large,
        }
    }
    pub fn apply_layout(&self, layout: RibbonItemLayout) {
        self.layout_applied.set(true);
        self.set_metrics(layout.metrics.clone());
        self.set_is_simplified(layout.is_simplified);
        self.set_current_size(layout.size);
        self.set_actual_show_label(layout.show_label.unwrap_or_else(|| {
            if layout.is_simplified {
                self.show_label_in_simplified_mode()
            } else {
                self.show_label()
            }
        }));
        self.on_layout_applied_core(layout);
    }
    pub(super) fn register_common_callbacks() {
        Self::id_property()
            .changed()
            .add_class_handler::<Self>(|this, e| {
                required(RibbonItemHelper::update_automation(this.to_ref()));
                this.on_ribbon_property_changed(e)
            })
            .detach();
        Self::label_property()
            .changed()
            .add_class_handler::<Self>(|this, e| {
                RibbonItemHelper::invalidate_host_layout(this);
                required(RibbonItemHelper::update_tool_tip(this.to_ref()));
                this.on_ribbon_property_changed(e)
            })
            .detach();
        for property in [
            Self::icon_property().as_property(),
            Self::large_icon_property().as_property(),
            Self::size_definition_property().as_property(),
            Self::simplified_visibility_property().as_property(),
            Self::simplified_label_property().as_property(),
            Self::show_label_property().as_property(),
        ] {
            property
                .changed()
                .add_class_handler::<Self>(|this, e| {
                    RibbonItemHelper::invalidate_host_layout(this);
                    this.on_ribbon_property_changed(e)
                })
                .detach();
        }
        Self::size_property()
            .changed()
            .add_class_handler::<Self>(|this, e| {
                if !this.layout_applied.get() {
                    this.set_current_size(this.size())
                }
                RibbonItemHelper::invalidate_host_layout(this);
                this.on_ribbon_property_changed(e)
            })
            .detach();
        for property in [
            Self::key_tip_property().as_property(),
            Self::screen_tip_property().as_property(),
        ] {
            property
                .changed()
                .add_class_handler::<Self>(|this, e| {
                    required(RibbonItemHelper::update_tool_tip(this.to_ref()));
                    this.on_ribbon_property_changed(e)
                })
                .detach();
        }
        Self::command_id_property()
            .changed()
            .add_class_handler::<Self>(|this, e| {
                this.resolve_common_command();
                this.on_ribbon_property_changed(e)
            })
            .detach();
        Self::shortcut_property()
            .changed()
            .add_class_handler::<Self>(|this, e| {
                required(RibbonItemHelper::update_tool_tip(this.to_ref()));
                RibbonItemHelper::increment_items_version();
                this.on_ribbon_property_changed(e)
            })
            .detach();
    }
    fn resolve_common_command(&self) {
        required(RibbonItemHelper::resolve_command(self));
    }
    pub(super) fn initialize_common_item(&self) {
        crate::theming::RibbonTheme::ensure_resources();
        self.loaded(|sender, _| {
            if let Some(this) = sender.downcast_ref::<RibbonControlBase>() {
                required(RibbonItemHelper::on_item_loaded(this));
            }
        });
        self.unloaded(|sender, _| {
            if let Some(this) = sender.downcast_ref::<RibbonControlBase>() {
                RibbonItemHelper::on_item_unloaded(this);
            }
        });
        // WinUI's IsEnabledChanged also fires when an ancestor is disabled.
        self.add_property_changed_handler(Rc::new(|e| {
            if e.property() == InputElement::is_effectively_enabled_property().as_property()
                && e.is_effective_value_change()
                && let Some(this) = e.sender().downcast_ref::<RibbonControlBase>()
            {
                required(RibbonItemHelper::update_tool_tip(this.to_ref()));
            }
        }));
        required(RibbonItemHelper::update_automation(self.to_ref()));
        RibbonItemHelper::attach_context_menu(&self.to_ref().upcast::<Control>());
    }
    pub fn execute_command(
        &self,
        parameter: Option<BoxedValue>,
    ) -> Result<bool, RibbonItemHelperError> {
        let value = parameter.or_else(|| self.command_parameter());
        let executed = RibbonItemHelper::execute(&CommandSourceView(self.to_ref()), value.as_ref());
        if RibbonItemHelper::get_source_item(self).is_none() {
            RibbonItemHelper::notify_invoked(self, value);
        }
        Ok(executed)
    }
}
fn required(result: Result<(), RibbonItemHelperError>) {
    if let Err(error) = result {
        panic!("{error}")
    }
}
fn text(value: Option<String>) -> Option<RibbonString> {
    value.map(Into::into)
}
fn native_text(value: Option<RibbonString>) -> Option<String> {
    value.map(|v| {
        v.to_utf8()
            .expect("native common item text cannot represent unpaired UTF-16")
    })
}
pub(super) struct ItemView(Ref<RibbonControlBase>);
pub(super) fn as_item(value: Ref<RibbonControlBase>) -> Rc<dyn IRibbonItem> {
    Rc::new(ItemView(value))
}
pub(super) fn as_editable(value: Ref<RibbonControlBase>) -> Rc<dyn IRibbonItemEditable> {
    Rc::new(ItemView(value))
}
pub(super) fn as_internal(value: Ref<RibbonControlBase>) -> Rc<dyn IRibbonItemInternal> {
    Rc::new(ItemView(value))
}
impl IRibbonItem for ItemView {
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
    fn id(&self) -> Option<RibbonString> {
        RibbonItemHelper::get_id(&self.0, text(self.0.id()), text(self.0.command_id()))
    }
    fn label(&self) -> Option<RibbonString> {
        text(self.0.label())
    }
    fn icon(&self) -> Option<BoxedValue> {
        self.0.icon()
    }
    fn key_tip(&self) -> Option<RibbonString> {
        text(self.0.key_tip())
    }
    fn command_id(&self) -> Option<RibbonString> {
        text(self.0.command_id())
    }
    fn size(&self) -> RibbonItemSize {
        self.0.size()
    }
    fn effective_size_definition(&self) -> RibbonSizeDefinition {
        self.0.effective_size_definition()
    }
    fn current_size(&self) -> RibbonItemSize {
        self.0.current_size()
    }
    fn simplified_visibility(&self) -> RibbonSimplifiedVisibility {
        self.0.simplified_visibility()
    }
    fn show_label_in_simplified_mode(&self) -> bool {
        self.0.show_label_in_simplified_mode()
    }
    fn can_add_to_quick_access(&self) -> bool {
        self.0.can_add_to_quick_access()
    }
    fn apply_layout(&self, layout: RibbonItemLayout) {
        self.0.apply_layout(layout)
    }
    fn create_linked_copy(&self) -> Option<Ref<Control>> {
        self.0.create_linked_copy_core()
    }
    fn create_overflow_menu_items(&self) -> RibbonOverflowMenuItems {
        self.0.create_overflow_menu_items_core()
    }
    fn on_key_tip(&self) -> Rc<RibbonKeyTipResult> {
        self.0.on_key_tip_core()
    }
    fn invoke(&self) -> bool {
        self.0.invoke_core()
    }
}
impl IRibbonItemEditable for ItemView {
    fn id(&self) -> Option<RibbonString> {
        text(self.0.id())
    }
    fn set_id(&self, value: Option<RibbonString>) {
        self.0.set_id(native_text(value))
    }
    fn label(&self) -> Option<RibbonString> {
        text(self.0.label())
    }
    fn set_label(&self, value: Option<RibbonString>) {
        self.0.set_label(native_text(value))
    }
    fn icon(&self) -> Option<BoxedValue> {
        self.0.icon()
    }
    fn set_icon(&self, value: Option<BoxedValue>) {
        self.0.set_icon(value)
    }
    fn size(&self) -> RibbonItemSize {
        self.0.size()
    }
    fn set_size(&self, value: RibbonItemSize) {
        self.0.set_size(value)
    }
    fn key_tip(&self) -> Option<RibbonString> {
        text(self.0.key_tip())
    }
    fn set_key_tip(&self, value: Option<RibbonString>) {
        self.0.set_key_tip(native_text(value))
    }
    fn command_id(&self) -> Option<RibbonString> {
        text(self.0.command_id())
    }
    fn set_command_id(&self, value: Option<RibbonString>) {
        self.0.set_command_id(native_text(value))
    }
    fn simplified_visibility(&self) -> RibbonSimplifiedVisibility {
        self.0.simplified_visibility()
    }
    fn set_simplified_visibility(&self, value: RibbonSimplifiedVisibility) {
        self.0.set_simplified_visibility(value)
    }
    fn can_add_to_quick_access(&self) -> bool {
        self.0.can_add_to_quick_access()
    }
    fn set_can_add_to_quick_access(&self, value: bool) {
        self.0.set_can_add_to_quick_access(value)
    }
    fn large_icon(&self) -> Option<BoxedValue> {
        self.0.large_icon()
    }
    fn set_large_icon(&self, value: Option<BoxedValue>) {
        self.0.set_large_icon(value)
    }
    fn size_definition(&self) -> Option<RibbonString> {
        text(self.0.size_definition())
    }
    fn set_size_definition(&self, value: Option<RibbonString>) {
        self.0.set_size_definition(native_text(value))
    }
    fn screen_tip(&self) -> Option<BoxedValue> {
        self.0.screen_tip()
    }
    fn set_screen_tip(&self, value: Option<BoxedValue>) {
        self.0.set_screen_tip(value)
    }
    fn shortcut(&self) -> Option<RibbonString> {
        text(self.0.shortcut())
    }
    fn set_shortcut(&self, value: Option<RibbonString>) {
        self.0.set_shortcut(native_text(value))
    }
    fn simplified_label(&self) -> RibbonSimplifiedLabel {
        self.0.simplified_label()
    }
    fn set_simplified_label(&self, value: RibbonSimplifiedLabel) {
        self.0.set_simplified_label(value)
    }
    fn show_label(&self) -> bool {
        self.0.show_label()
    }
    fn set_show_label(&self, value: bool) {
        self.0.set_show_label(value)
    }
}
impl IRibbonItemInternal for ItemView {
    fn shortcut(&self) -> Option<RibbonString> {
        text(self.0.shortcut())
    }
    fn screen_tip(&self) -> Option<BoxedValue> {
        self.0.screen_tip()
    }
    fn linked_properties(&self) -> Vec<&'static FerroProperty> {
        vec![
            RibbonControlBase::label_property(),
            RibbonControlBase::icon_property(),
            RibbonControlBase::large_icon_property(),
            RibbonControlBase::screen_tip_property(),
            RibbonControlBase::shortcut_property(),
            RibbonControlBase::key_tip_property(),
            RibbonControlBase::size_property(),
            RibbonControlBase::size_definition_property(),
        ]
    }
    fn on_unlinked(&self) {
        self.0.on_unlinked_core()
    }
    fn owned_values(&self) -> &OwnedItemValues {
        &self.0.owned_values
    }
}
