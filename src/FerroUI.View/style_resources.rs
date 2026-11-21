//! Per-control resource subscriptions for the declarative style builder.
use ferroui_base::controls::{ResourceConverter, ResourceDictionary, ResourceHostRef, ResourceKey};
use ferroui_base::data::core::{UntypedObservableBindingExpression, ValueType, ValueTypes};
use ferroui_base::data::{BindingBase, BindingExpressionBase, BindingPriority};
use ferroui_base::media::{Color, IBrush, immutable::ImmutableSolidColorBrush};
use ferroui_base::styling::{Style, ThemeVariant};
use ferroui_base::{BoxedValue, FerroObject, FerroProperty, Ref, StyledElement, UnsetValueType};
use std::rc::Rc;

#[derive(Default)]
pub(super) struct StyleResources(Vec<ResourceEntry>);

struct ResourceEntry {
    theme: Option<ThemeVariant>,
    key: ResourceKey,
    value: BoxedValue,
}

impl StyleResources {
    pub(super) fn add(&mut self, theme: Option<ThemeVariant>, key: ResourceKey, value: BoxedValue) {
        self.0.push(ResourceEntry { theme, key, value });
    }

    pub(super) fn apply(self, style: &Style) {
        if self.0.is_empty() {
            return;
        }
        let resources = style.resources();
        let mut variants = Vec::<(ThemeVariant, Ref<ResourceDictionary>)>::new();
        for ResourceEntry { theme, key, value } in self.0 {
            let dictionary = match theme {
                None => resources.clone(),
                Some(theme) => variants
                    .iter()
                    .find(|(variant, _)| *variant == theme)
                    .map(|(_, dictionary)| dictionary.clone())
                    .unwrap_or_else(|| {
                        let dictionary = ResourceDictionary::new();
                        dictionary.set_key(Some(theme.clone()));
                        resources.add_theme_dictionary(theme.clone(), &dictionary);
                        variants.push((theme, dictionary.clone()));
                        dictionary
                    }),
            };
            dictionary.set(key, Some(value));
        }
    }
}

pub(super) struct StyleResource(pub ResourceKey);

impl BindingBase for StyleResource {
    fn create_instance(
        &self,
        target: &FerroObject,
        property: Option<&'static FerroProperty>,
        _anchor: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        let element = target
            .to_ref()
            .cast::<StyledElement>()
            .expect("Styles attach to styled elements");
        let converter = property.map(|property| {
            let target = ValueType::new(property.property_type(), property.property_type_name());
            Rc::new(move |value| convert(value, target)) as ResourceConverter
        });
        let observable = ResourceHostRef::from(element).resource_observable(&self.0, converter);
        UntypedObservableBindingExpression::new(observable, BindingPriority::LocalValue)
    }
}

fn convert(value: Option<BoxedValue>, target: ValueType) -> Option<BoxedValue> {
    if value
        .as_ref()
        .is_some_and(|value| value.is::<UnsetValueType>())
    {
        return value;
    }
    if target.is::<Rc<dyn IBrush>>() || target.is::<Option<Rc<dyn IBrush>>>() {
        if let Some(brush) = value.as_ref().and_then(brush_value) {
            // The View API works without markup type tables. Box the exact
            // nullable property type even when its markup cast is unregistered.
            return Some(if target.is::<Option<Rc<dyn IBrush>>>() {
                Rc::new(Some(brush))
            } else {
                Rc::new(brush)
            });
        }
    }
    if let Some(cast) = value
        .as_ref()
        .and_then(|value| ValueTypes::try_cast(value, target))
    {
        return Some(cast);
    }
    if target.is::<String>() || target.is::<Option<String>>() {
        return value;
    }
    ValueTypes::try_convert(value.as_ref(), target).unwrap_or(value)
}

fn brush_value(value: &BoxedValue) -> Option<Rc<dyn IBrush>> {
    if let Some(color) = value.downcast_ref::<Color>() {
        return Some(Rc::new(ImmutableSolidColorBrush::new(*color)));
    }
    value.downcast_ref::<Rc<dyn IBrush>>().cloned()
}
