//! Source RibbonElementFactory.CreateCustom/FindTemplate (9755e281).
//! Copyright Wiesław Šoltés, MIT. Native controls/templates preserve identity;
//! binding scopes own subscriptions and models, with weak control backlinks.
use super::custom_template_resources::lookup_with_theme;
use super::custom_template_theme::TemplateTheme;
use super::ribbon_element_factory::{native_payload, text};
use super::{RibbonElementFactory, RibbonModelBindings};
use crate::compat::DataTemplateSelectorExt;
use ferroribbon_core::{RibbonString, model::*};
use ferroui_base::{
    BoxedValue, FerroLocator, LocatorExtensions, Ref,
    controls::{IResourceDictionary, ResourceKey, ResourceValue},
    data::core::{ValueType, ValueTypes},
    platform::{ColorContrastPreference, IPlatformSettings},
    styling::ThemeVariant,
};
use ferroui_controls::{
    Application, Control, presenters::ContentPresenter, templates::IDataTemplate,
};
use std::rc::Rc;
fn control(value: Option<RibbonObject>) -> Option<Ref<Control>> {
    value
        .map(native_payload)
        .as_ref()
        .and_then(Control::from_boxed)
}
fn data_template(value: Option<BoxedValue>) -> Option<Rc<dyn IDataTemplate>> {
    let value = value?;
    ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IDataTemplate>>())
        .and_then(|v| v.downcast_ref::<Rc<dyn IDataTemplate>>().cloned())
}
impl RibbonElementFactory {
    pub(super) fn build_custom(
        &self,
        model: Rc<dyn RibbonCustomItemModelImpl>,
        bindings: Rc<RibbonModelBindings>,
    ) -> Ref<Control> {
        if let Some(element) = control(model.content()) {
            Self::detach_from_parent(element.clone());
            let e = element.clone();
            let m = model.clone();
            let owner = Rc::downgrade(&bindings);
            bindings.watch(
                Some(model),
                Some(Rc::new(move || {
                    if !control(m.content()).is_some_and(|current| current.ptr_eq(&e))
                        && let Some(owner) = owner.upgrade()
                    {
                        owner.request_recreate();
                    }
                })),
                Some(vec![Some("Content".into())]),
            );
            return element;
        }
        let presenter = ContentPresenter::new();
        let p = presenter.clone();
        let owner = Rc::downgrade(&bindings);
        let f = self.to_ref();
        bindings.one_way(
            Some(model.clone()),
            Some(Rc::new(move || {
                if control(model.content()).is_some() {
                    if let Some(owner) = owner.upgrade() {
                        owner.request_recreate();
                    }
                    return;
                }
                p.set_content(model.content().map(native_payload));
                // Reread after the content setter, which can synchronously reenter.
                p.set_content_template(
                    f.find_template(model.template_key(), model.content().map(native_payload)),
                );
            })),
            Some(vec![Some("Content".into()), Some("TemplateKey".into())]),
        );
        presenter.upcast()
    }
    /// Source protected FindTemplate: local ribbon, application, then selector.
    /// Native IDataTemplate is the reusable FerroUI template contract.
    pub fn find_template(
        &self,
        key: Option<RibbonString>,
        content: Option<BoxedValue>,
    ) -> Option<Rc<dyn IDataTemplate>> {
        if let Some(key) = text(key) {
            let key: ResourceKey = key.into();
            let theme = || self.template_theme().0;
            if let Some(ribbon) = self.ribbon() {
                let dictionary: Rc<dyn IResourceDictionary> = ribbon.resources().into();
                if let Some(template) = data_template(
                    lookup_with_theme(dictionary.as_ref(), &key, &theme, || {
                        self.system_template_resource(&key)
                    })
                    .flatten(),
                ) {
                    return Some(template);
                }
            }
            // Reread Current after local/lazy lookup, which can synchronously reenter.
            let application =
                Application::current().expect("FindTemplate requires Application for keyed lookup");
            let dictionary: Rc<dyn IResourceDictionary> = application.resources().into();
            if let Some(template) = data_template(
                lookup_with_theme(dictionary.as_ref(), &key, &theme, || {
                    self.system_template_resource(&key)
                })
                .flatten(),
            ) {
                return Some(template);
            }
        }
        self.content_template_selector()
            .and_then(|selector| selector.select_template(content))
    }
    fn template_theme(&self) -> (TemplateTheme, ThemeVariant) {
        let actual = self
            .ribbon()
            .and_then(|ribbon| ribbon.actual_theme_variant())
            .or_else(|| Application::current().and_then(|app| app.actual_theme_variant()))
            .unwrap_or_else(ThemeVariant::light);
        let explicit_high_contrast = actual == ThemeVariant::new("HighContrast", None);
        let high_contrast = explicit_high_contrast
            || FerroLocator::current()
                .get_service::<dyn IPlatformSettings>()
                .is_some_and(|settings| {
                    settings.get_color_values().contrast_preference()
                        == ColorContrastPreference::High
                });
        let base = if explicit_high_contrast {
            actual.inherit_variant().unwrap_or_else(ThemeVariant::light)
        } else {
            actual.clone()
        };
        (TemplateTheme::new(base, high_contrast), actual)
    }
    fn system_template_resource(&self, key: &ResourceKey) -> Option<ResourceValue> {
        // Native migration boundary: platform/theme resources are provided by
        // Application styles. Do not search Application.Resources here: source
        // TryGetValue reaches that scope separately after the local/system step.
        let actual = self.template_theme().1;
        Application::current().and_then(|app| app.styles().try_get_resource(key, Some(&actual)))
    }
}
