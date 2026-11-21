//! Shared source input layout/focus state, using native inherited properties and pseudo classes.
use super::{
    RibbonControlBase, RibbonControlBaseImpl, RibbonItemHelper, RibbonItemLayout,
    RibbonOverflowMenuItems,
};
use crate::compat::Visibility;
use ferroribbon_core::{RibbonItemSize, model::RibbonItemsSource};
use ferroui_base::{
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, StyledElementImpl, StyledProperty,
    VisualImpl, ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{
        FocusChangedEventArgs, InputElement, InputElementImpl, InputElementImplExt,
        PointerEventArgs,
    },
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{
    ControlImpl,
    primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt},
};
use std::cell::Cell;
#[repr(C)]
pub struct RibbonInputBase {
    base: RibbonControlBase,
    is_pointer_over: Cell<bool>,
    has_focus: Cell<bool>,
}
ferro_class!(RibbonInputBase:RibbonControlBase,virtuals RibbonInputBaseImpl:RibbonControlBaseImpl{
 fn shows_label_by_default(this)->bool;
});
// Abstract source type: no fake concrete constructor or default template.
ferro_class_info!(RibbonInputBase{markup:{namespace:"FerroRibbon.Controls",properties:[InputHeight:f64{get:RibbonInputBase::input_height},LabelVisibility:Visibility{get:RibbonInputBase::label_visibility},IconVisibility:Visibility{get:RibbonInputBase::icon_visibility}]}});
ferro_impl_classes!(RibbonInputBase:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,ControlImpl);
ferro_properties! {impl RibbonInputBase{
 pub fn input_width_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("InputWidth",120.)}
 pub fn placeholder_text_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("PlaceholderText",None)}
 pub fn input_height_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("InputHeight",22.)}
 pub fn label_visibility_property()->StyledProperty<Visibility>{FerroProperty::register::<Self,_>("LabelVisibility",Visibility::Collapsed)}
 pub fn icon_visibility_property()->StyledProperty<Visibility>{FerroProperty::register::<Self,_>("IconVisibility",Visibility::Collapsed)}
}}
impl RibbonInputBase {
    /// Source protected TryParseNumber: current-culture Float, then invariant;
    /// failed parsing always replaces the out value with zero.
    pub fn try_parse_number(text: Option<&str>, value: &mut f64) -> bool {
        use ferroui_base::utilities::{
            CultureInfo, NumberFormatInfo, number_format::try_parse_double_float,
        };
        let parsed =
            try_parse_double_float(text, &CultureInfo::current_formatting().number_format())
                .or_else(|| try_parse_double_float(text, &NumberFormatInfo::invariant_info()));
        *value = parsed.unwrap_or(0.);
        parsed.is_some()
    }
    fn static_constructor() {
        Self::input_width_property()
            .changed()
            .add_class_handler::<Self>(|this, _| RibbonItemHelper::invalidate_host_layout(this))
            .detach();
    }
    pub fn construct() -> Self {
        Self {
            base: RibbonControlBase::construct(),
            is_pointer_over: Cell::new(false),
            has_focus: Cell::new(false),
        }
    }
    pub fn input_width(&self) -> f64 {
        self.get_value(Self::input_width_property())
    }
    pub fn set_input_width(&self, v: f64) {
        self.set_value(Self::input_width_property(), v)
    }
    pub fn placeholder_text(&self) -> Option<String> {
        self.get_value(Self::placeholder_text_property())
    }
    pub fn set_placeholder_text(&self, v: Option<String>) {
        self.set_value(Self::placeholder_text_property(), v)
    }
    pub fn input_height(&self) -> f64 {
        self.get_value(Self::input_height_property())
    }
    pub fn label_visibility(&self) -> Visibility {
        self.get_value(Self::label_visibility_property())
    }
    pub fn icon_visibility(&self) -> Visibility {
        self.get_value(Self::icon_visibility_property())
    }
    fn update_input_layout(&self) {
        let height = if self.is_simplified() {
            let metrics = self.metrics().expect("NullReference: Metrics");
            let value = metrics.simplified_item_height() - 4.;
            if value.is_nan() {
                value
            } else {
                24_f64.max(value)
            }
        } else {
            self.metrics().expect("NullReference: Metrics").row_height()
        };
        self.set_value(Self::input_height_property(), height);
        let label = !self.label().as_deref().unwrap_or("").is_empty()
            && self.shows_label_by_default()
            && (self.is_simplified()
                || (self.actual_show_label() && self.current_size() != RibbonItemSize::Small));
        self.set_value(
            Self::label_visibility_property(),
            if label {
                Visibility::Visible
            } else {
                Visibility::Collapsed
            },
        );
        let icon = self.icon().is_some()
            && (self.current_size() != RibbonItemSize::Small
                || self.label_visibility() == Visibility::Collapsed);
        self.set_value(
            Self::icon_visibility_property(),
            if icon {
                Visibility::Visible
            } else {
                Visibility::Collapsed
            },
        );
    }
    pub fn update_input_states(&self, _use_transitions: bool) {
        let enabled = self.is_effectively_enabled();
        let focused = self.has_focus.get();
        let hover = self.is_pointer_over.get();
        self.pseudo_classes().set(":input-disabled", !enabled);
        self.pseudo_classes()
            .set(":input-focused", enabled && focused);
        self.pseudo_classes()
            .set(":input-pointerover", enabled && !focused && hover);
        self.pseudo_classes()
            .set(":input-normal", enabled && !focused && !hover);
    }
}
impl FerroObjectImpl for RibbonInputBase {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_size(RibbonItemSize::Medium);
        this.set_is_tab_stop(false);
        this.set_can_add_to_quick_access(true);
        this.add_property_changed_handler(std::rc::Rc::new(|e| {
            if e.property() == InputElement::is_effectively_enabled_property().as_property()
                && e.is_effective_value_change()
            {
                e.sender()
                    .downcast_ref::<RibbonInputBase>()
                    .unwrap()
                    .update_input_states(true);
            }
        }));
    }
}
impl RibbonInputBaseImpl for RibbonInputBase {
    fn shows_label_by_default(_this: &Self) -> bool {
        true
    }
}
ferroui_base::ferro_thread_local! {static EMPTY_OVERFLOW:RibbonOverflowMenuItems=RibbonItemsSource::from_values(vec![]);}
impl RibbonControlBaseImpl for RibbonInputBase {
    fn uses_command_can_execute(_this: &Self) -> bool {
        false
    }
    fn on_layout_applied_core(this: &Self, _layout: RibbonItemLayout) {
        this.update_input_layout();
    }
    fn on_ribbon_property_changed(
        this: &Self,
        _e: &ferroui_base::FerroPropertyChangedEventArgs<'_>,
    ) {
        this.update_input_layout();
    }
    fn create_overflow_menu_items_core(_this: &Self) -> RibbonOverflowMenuItems {
        EMPTY_OVERFLOW.with(Clone::clone)
    }
}
impl TemplatedControlImpl for RibbonInputBase {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        this.update_input_layout();
        this.update_input_states(false);
    }
}
impl InputElementImpl for RibbonInputBase {
    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);
        this.has_focus.set(true);
        this.update_input_states(true);
    }
    fn on_lost_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_lost_focus(this, e);
        this.has_focus.set(false);
        this.update_input_states(true);
    }
    fn on_pointer_entered(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_entered(this, e);
        this.is_pointer_over.set(true);
        this.update_input_states(true);
    }
    fn on_pointer_exited(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_exited(this, e);
        this.is_pointer_over.set(false);
        this.update_input_states(true);
    }
}
