use crate::source_observation::SourceObservation;
use crate::TranslationBindingSource;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::BindingPriority;
use ferroui_base::media::FlowDirection;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_markup_type, ferro_properties, ferro_static_type, AttachedProperty, BoxedValue,
    FerroObject, FerroProperty, FerroPropertyChangedEventArgs, Ref, StyledPropertyOptions, Visual,
};
use ferroui_controls::primitives::HeaderedContentControl;
use ferroui_controls::{ContentControl, TextBlock, TextBox};
use ferroui_protranslate_core::{Culture, TextFlowDirection, TranslationService};
use std::cell::Cell;
use std::rc::Rc;

/// Native attached localization properties. Culture and direction inherit
/// through FerroUI's inheritance tree. Each target owns its subscription and
/// the callback holds only a weak target reference.
pub struct Translation;
ferro_static_type!(Translation);

ferro_properties! {
    impl Translation {
        pub fn key_property() -> AttachedProperty<Option<String>> {
            let property = FerroProperty::register_attached::<Translation, FerroObject, _>("Key", None);
            let _ = property.changed().subscribe(Self::changed);
            property
        }
        pub fn fallback_value_property() -> AttachedProperty<Option<BoxedValue>> {
            let property = FerroProperty::register_attached::<Translation, FerroObject, _>("FallbackValue", None);
            let _ = property.changed().subscribe(Self::changed);
            property
        }
        pub fn string_format_property() -> AttachedProperty<Option<String>> {
            let property = FerroProperty::register_attached::<Translation, FerroObject, _>("StringFormat", None);
            let _ = property.changed().subscribe(Self::changed);
            property
        }
        pub fn culture_property() -> AttachedProperty<Option<String>> {
            let property = FerroProperty::register_attached_with::<Translation, Visual, _>(
                "Culture", StyledPropertyOptions::new(None).inherits(true).validate(|value: &Option<String>| {
                    value.as_ref().is_none_or(|name| Culture::parse(name).is_ok())
                }),
            );
            let _ = property.changed().subscribe(Self::culture_changed);
            property
        }
        pub fn auto_flow_direction_property() -> AttachedProperty<bool> {
            let property = FerroProperty::register_attached_with::<Translation, Visual, _>(
                "AutoFlowDirection", StyledPropertyOptions::new(false).inherits(true),
            );
            let _ = property.changed().subscribe(Self::changed);
            property
        }
        pub fn source_property() -> AttachedProperty<Option<TranslationBindingSource>> {
            let property = FerroProperty::register_attached_with::<Translation, FerroObject, _>(
                "Source", StyledPropertyOptions::new(None).inherits(true),
            );
            let _ = property.changed().subscribe(Self::changed);
            property
        }
        /// Last failure of attached-property evaluation, cleared after a successful refresh.
        pub fn error_property() -> AttachedProperty<Option<String>> {
            FerroProperty::register_attached::<Translation, FerroObject, _>("Error", None)
        }
    }
}

struct AttachedState;
ferro_static_type!(AttachedState);
ferro_properties! {
    impl AttachedState {
        fn subscription_property() -> AttachedProperty<Option<AttachedSubscription>> {
            FerroProperty::register_attached::<AttachedState, FerroObject, _>("TranslationSubscription", None)
        }
        fn flow_override_property() -> AttachedProperty<Option<FlowOverride>> {
            FerroProperty::register_attached::<AttachedState, FerroObject, _>("TranslationFlowOverride", None)
        }
    }
}

#[derive(Clone)]
struct AttachedSubscription(Rc<SourceObservation>);
impl PartialEq for AttachedSubscription {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone)]
struct FlowOverride(Rc<FlowOverrideInner>);
struct FlowOverrideInner {
    value: FlowDirection,
    entry: Rc<dyn IDisposable>,
    disposed: Cell<bool>,
}
impl PartialEq for FlowOverride {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
impl FlowOverrideInner {
    fn dispose(&self) {
        if !self.disposed.replace(true) {
            self.entry.dispose();
        }
    }
}
impl Drop for FlowOverrideInner {
    fn drop(&mut self) {
        self.dispose();
    }
}

impl Translation {
    /// Configures the default source without invalidating already-created bindings.
    pub fn set_service(service: TranslationService) {
        crate::TranslationService::use_service(service);
    }

    pub fn get_source(target: &FerroObject) -> TranslationBindingSource {
        target
            .get_value(Self::source_property())
            .unwrap_or_else(TranslationBindingSource::default_source)
    }
    pub fn set_source(target: &FerroObject, source: Option<TranslationBindingSource>) {
        target.set_value(Self::source_property(), source);
    }
    pub fn get_key(target: &FerroObject) -> Option<String> {
        target.get_value(Self::key_property())
    }
    pub fn set_key(target: &FerroObject, key: Option<String>) {
        target.set_value(Self::key_property(), key);
    }
    pub fn get_fallback_value(target: &FerroObject) -> Option<BoxedValue> {
        target.get_value(Self::fallback_value_property())
    }
    pub fn set_fallback_value(target: &FerroObject, value: Option<BoxedValue>) {
        target.set_value(Self::fallback_value_property(), value);
    }
    /// Sets a text fallback while the native property can hold any content object.
    pub fn set_fallback_text(target: &FerroObject, value: Option<String>) {
        Self::set_fallback_value(target, value.map(|value| Rc::new(value) as BoxedValue));
    }
    pub fn get_string_format(target: &FerroObject) -> Option<String> {
        target.get_value(Self::string_format_property())
    }
    pub fn set_string_format(target: &FerroObject, value: Option<String>) {
        target.set_value(Self::string_format_property(), value);
    }
    pub fn get_culture(target: &FerroObject) -> Option<String> {
        target.get_value(Self::culture_property())
    }
    pub fn set_culture(target: &FerroObject, value: Option<String>) {
        target.set_value(Self::culture_property(), value);
    }
    pub fn get_auto_flow_direction(target: &FerroObject) -> bool {
        target.get_value(Self::auto_flow_direction_property())
    }
    pub fn set_auto_flow_direction(target: &FerroObject, value: bool) {
        target.set_value(Self::auto_flow_direction_property(), value);
    }

    pub fn to_flow_direction(culture: &Culture) -> FlowDirection {
        match culture.flow_direction() {
            TextFlowDirection::LeftToRight => FlowDirection::LeftToRight,
            TextFlowDirection::RightToLeft => FlowDirection::RightToLeft,
        }
    }

    fn culture_changed(change: &FerroPropertyChangedEventArgs<'_>) {
        let target = change.sender();
        if let Some(name) = target.get_value(Self::culture_property()) {
            if let Ok(culture) = Culture::parse(&name) {
                Self::get_source(target).set_culture(culture);
            }
        }
        Self::changed(change);
    }

    fn changed(change: &FerroPropertyChangedEventArgs<'_>) {
        let target = change.sender();
        let previous = target.get_value(AttachedState::subscription_property());
        let requires = Self::get_key(target).is_some_and(|key| !key.trim().is_empty())
            || Self::get_auto_flow_direction(target);
        match (requires, previous) {
            (true, Some(previous)) => previous.0.refresh(),
            (true, None) => {
                let weak = target.to_weak();
                let update_target = weak.clone();
                let subscription = SourceObservation::start(
                    weak,
                    None,
                    Rc::new(move || {
                        if let Some(target) = update_target.upgrade() {
                            Self::refresh(&target);
                        }
                    }),
                );
                target.set_value(
                    AttachedState::subscription_property(),
                    Some(AttachedSubscription(subscription.clone())),
                );
                subscription.refresh();
            }
            (false, previous) => {
                if let Some(previous) = previous {
                    previous.0.dispose();
                }
                target.set_value(AttachedState::subscription_property(), None);
                target.set_value(Self::error_property(), None);
                Self::clear_flow_override(target);
            }
        }
    }

    fn refresh(target: &FerroObject) {
        let source = Self::get_source(target);
        if Self::get_auto_flow_direction(target) {
            if let Some(visual) = target.downcast_ref::<Visual>() {
                let value = Self::to_flow_direction(&source.culture());
                let previous = target.get_value(AttachedState::flow_override_property());
                if previous
                    .as_ref()
                    .is_none_or(|previous| previous.0.value != value)
                {
                    // A removable style entry respects explicit local direction and
                    // restores inheritance when automatic direction is disabled.
                    let entry = visual
                        .set_value_with_priority(
                            Visual::flow_direction_property(),
                            value,
                            BindingPriority::StyleTrigger,
                        )
                        .expect("a styled direction has a removable value entry");
                    target.set_value(
                        AttachedState::flow_override_property(),
                        Some(FlowOverride(Rc::new(FlowOverrideInner {
                            value,
                            entry,
                            disposed: Cell::new(false),
                        }))),
                    );
                    if let Some(previous) = previous {
                        previous.0.dispose();
                    }
                }
            }
        } else {
            Self::clear_flow_override(target);
        }
        let Some(key) = Self::get_key(target).filter(|key| !key.trim().is_empty()) else {
            target.set_value(Self::error_property(), None);
            return;
        };
        match source.render_value(
            &key,
            &[],
            None,
            Self::get_fallback_value(target).as_ref(),
            Self::get_string_format(target).as_deref(),
        ) {
            Ok(value) => {
                target.set_value(Self::error_property(), None);
                if let Some(text_block) = target.downcast_ref::<TextBlock>() {
                    text_block.set_current_value(
                        TextBlock::text_property(),
                        Some(ValueTypes::to_display_string(Some(&value))),
                    );
                } else if let Some(text_box) = target.downcast_ref::<TextBox>() {
                    text_box.set_current_value(
                        TextBox::text_property(),
                        Some(ValueTypes::to_display_string(Some(&value))),
                    );
                } else if let Some(headered) = target.downcast_ref::<HeaderedContentControl>() {
                    headered
                        .set_current_value(HeaderedContentControl::header_property(), Some(value));
                } else if let Some(content) = target.downcast_ref::<ContentControl>() {
                    content.set_current_value(ContentControl::content_property(), Some(value));
                }
            }
            Err(error) => {
                target.set_value(Self::error_property(), Some(error.to_string()));
            }
        }
    }

    fn clear_flow_override(target: &FerroObject) {
        let previous = target.get_value(AttachedState::flow_override_property());
        target.clear_value(AttachedState::flow_override_property());
        // Remove the native frame after the attached-property write has released
        // the property store, so the resulting flow notification may be reentrant.
        if let Some(previous) = previous {
            previous.0.dispose();
        }
    }
}

ferro_markup_type!(static Translation {
    type_info: Translation,
    methods: [
        static fn GetKey(Ref<FerroObject>) -> Option<String> => |target: Ref<FerroObject>| Translation::get_key(&target),
        static fn SetKey(Ref<FerroObject>, Option<String>) => |target: Ref<FerroObject>, value: Option<String>| Translation::set_key(&target, value),
        static fn GetFallbackValue(Ref<FerroObject>) -> Option<BoxedValue> => |target: Ref<FerroObject>| Translation::get_fallback_value(&target),
        static fn SetFallbackValue(Ref<FerroObject>, Option<BoxedValue>) => |target: Ref<FerroObject>, value: Option<BoxedValue>| Translation::set_fallback_value(&target, value),
        static fn GetStringFormat(Ref<FerroObject>) -> Option<String> => |target: Ref<FerroObject>| Translation::get_string_format(&target),
        static fn SetStringFormat(Ref<FerroObject>, Option<String>) => |target: Ref<FerroObject>, value: Option<String>| Translation::set_string_format(&target, value),
        static fn GetCulture(Ref<FerroObject>) -> Option<String> => |target: Ref<FerroObject>| Translation::get_culture(&target),
        static fn SetCulture(Ref<FerroObject>, Option<String>) => |target: Ref<FerroObject>, value: Option<String>| Translation::set_culture(&target, value),
        static fn GetAutoFlowDirection(Ref<FerroObject>) -> bool => |target: Ref<FerroObject>| Translation::get_auto_flow_direction(&target),
        static fn SetAutoFlowDirection(Ref<FerroObject>, bool) => |target: Ref<FerroObject>, value: bool| Translation::set_auto_flow_direction(&target, value),
    ],
});
