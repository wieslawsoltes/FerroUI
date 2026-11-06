//! Native data/event adapters. Layout lives in views/settings/*.xaml; helper semantics are in VMs.
use crate::settings_callback_owner::SettingsCallbackOwner;
use crate::settings_fields::{FieldError, FieldKind, FieldValue, SettingsFieldViewModel};
use ferroui_base::{Ref, controls::NameScope, metadata::into_markup_value};
use ferroui_controls::{ComboBox, Control, ItemsSource, Slider, TextBlock, ToggleSwitch};
use ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers;
use ferroui_system::component_model::INotifyPropertyChanged;
use std::rc::Rc;
pub(crate) struct SettingsFieldView {
    pub root: Ref<Control>,
    pub input: Ref<Control>,
    pub model: Rc<SettingsFieldViewModel>,
    observer: u64,
}
impl Drop for SettingsFieldView {
    fn drop(&mut self) {
        self.model.property_changed().remove(self.observer);
    }
}
impl SettingsFieldView {
    pub fn new(
        model: Rc<SettingsFieldViewModel>,
        owner: Rc<SettingsCallbackOwner>,
    ) -> Result<Rc<Self>, String> {
        let services = || Some(XamlIlRuntimeHelpers::create_root_service_provider_v3(None));
        let root = match model.kind() {
            FieldKind::Boolean(_) => {
                crate::compiled_settings_views::build_views_settings_fieldtoggle_xaml(services())
            }
            FieldKind::Number(_) => {
                crate::compiled_settings_views::build_views_settings_fieldslider_xaml(services())
            }
            FieldKind::Enum(_) | FieldKind::Theme { .. } => {
                crate::compiled_settings_views::build_views_settings_fieldcombo_xaml(services())
            }
        }
        .map_err(|error| error.to_string())?;
        let names = NameScope::get_name_scope(&root).ok_or("missing compiled field name scope")?;
        let caption = names
            .find("Header")
            .and_then(|part| part.cast::<TextBlock>())
            .ok_or("missing field caption")?;
        caption.set_text(Some(model.label()));
        let input = names
            .find("Value")
            .and_then(|part| part.cast::<Control>())
            .ok_or("missing field input")?;
        let observer = match model.kind() {
            FieldKind::Boolean(_) => {
                let control = input
                    .clone()
                    .cast::<ToggleSwitch>()
                    .ok_or("invalid toggle field input")?;
                let weak = control.downgrade();
                let captured = model.clone();
                control.is_checked_changed(move |_, _| {
                    let Some(_owner) = owner.retain() else {
                        return;
                    };
                    if let Some(control) = weak.upgrade()
                        && let Some(value) = control.is_checked()
                    {
                        captured
                            .dispatch(FieldValue::Bool(value))
                            .expect("source toggle callback");
                    }
                });
                let weak = control.downgrade();
                let observed = Rc::downgrade(&model);
                model.property_changed().add(Rc::new(move |name| {
                    if name == "Value"
                        && let (Some(control), Some(model)) = (weak.upgrade(), observed.upgrade())
                        && let FieldValue::Bool(value) = model.value()
                    {
                        control.set_is_checked(Some(value));
                    }
                }))
            }
            FieldKind::Number(number) => {
                let control = input
                    .clone()
                    .cast::<Slider>()
                    .ok_or("invalid slider field input")?;
                let (min, max, step) = number.range();
                control.set_minimum(min);
                control.set_maximum(max);
                control.set_small_change(step);
                control.set_tick_frequency(step);
                control.set_is_snap_to_tick_enabled(true);
                // Establish the native range default before wiring the source callback. Otherwise
                // RangeBase defers coercing its default until visual attachment and emits an
                // initialization ValueChanged that would overwrite the still-unread source value.
                control.set_range_value(min);
                let captured = model.clone();
                control.value_changed(move |_, e| {
                    let Some(_owner) = owner.retain() else {
                        return;
                    };
                    captured
                        .dispatch(FieldValue::Number(e.new_value()))
                        .expect("source slider callback");
                });
                let weak = control.downgrade();
                let observed = Rc::downgrade(&model);
                model.property_changed().add(Rc::new(move |name| {
                    if name == "Value"
                        && let (Some(control), Some(model)) = (weak.upgrade(), observed.upgrade())
                        && let FieldValue::Number(value) = model.value()
                    {
                        control.set_value(
                            ferroui_controls::primitives::RangeBase::value_property(),
                            value,
                        );
                    }
                }))
            }
            FieldKind::Enum(_) | FieldKind::Theme { .. } => {
                let control = input
                    .clone()
                    .cast::<ComboBox>()
                    .ok_or("invalid combo field input")?;
                let labels = model
                    .labels()
                    .iter()
                    .map(|t| {
                        t.as_ref()
                            .map(ferroboard_core::text::Utf16Text::to_utf8)
                            .transpose()
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|e| e.to_string())?;
                control.set_items_source(Some(ItemsSource::from_items(
                    labels.into_iter().map(into_markup_value),
                )));
                let weak = control.downgrade();
                let captured = model.clone();
                control.selection_changed(move |_, _| {
                    let Some(_owner) = owner.retain() else {
                        return;
                    };
                    if let Some(control) = weak.upgrade() {
                        captured
                            .dispatch(FieldValue::Index(control.selected_index()))
                            .expect("source combo callback");
                    }
                });
                let weak = control.downgrade();
                let observed = Rc::downgrade(&model);
                model.property_changed().add(Rc::new(move |name| {
                    if name == "Value"
                        && let (Some(control), Some(model)) = (weak.upgrade(), observed.upgrade())
                        && let FieldValue::Index(value) = model.value()
                    {
                        control.set_selected_index(value);
                    }
                }))
            }
        };
        Ok(Rc::new(Self {
            root: root.upcast(),
            input,
            model,
            observer,
        }))
    }
    pub fn refresh(&self) -> Result<(), FieldError> {
        self.model.refresh()
    }
}
