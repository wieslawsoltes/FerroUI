use crate::{PreparedReload, ReloadError};
use ferroui_base::{
    controls::{
        IResourceHost, IResourceNode, ResourceDictionary, ResourceDictionaryChange,
        ResourceDictionaryOwnership, ResourceHostRef, ResourceKey, ResourceValue,
        ResourcesChangedEventArgs,
    },
    metadata::{service, IServiceProvider},
    reactive::{Disposable, IDisposable},
    styling::ThemeVariant,
    BoxedValue, Ref, StyledElement, WeakRef,
};
use ferroui_markup_xaml::xaml_il::runtime::{
    IFerroXamlIlParentStackProvider, XamlIlRuntimeHelpers,
};
use std::{
    any::{Any, TypeId},
    rc::Rc,
};

#[derive(Clone)]
pub(crate) struct OwnedResources {
    root: WeakRef<StyledElement>,
    ownership: ResourceDictionaryOwnership,
}
impl OwnedResources {
    pub(crate) fn capture(root: &Ref<StyledElement>) -> Self {
        Self {
            root: root.downgrade(),
            ownership: ResourceDictionaryOwnership::capture(&root.resources()),
        }
    }
    pub(crate) fn services(&self) -> Result<Rc<dyn IServiceProvider>, ReloadError> {
        let root = self
            .root
            .upgrade()
            .ok_or_else(|| ReloadError::Failed("Component closed".into()))?;
        if !self.ownership.owns_container(&root.resources()) {
            return Err(ReloadError::RestartRequired(
                "Application replaced the component resource dictionary".into(),
            ));
        }
        Ok(services(&root, Some(self), None))
    }
    pub(crate) fn application_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        self.ownership.application_resource(key, theme)
    }
    pub(crate) fn preview_resource(&self, candidate: &ResourceDictionary, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        self.ownership.preview_resource(candidate, key, theme)
            .unwrap_or_else(|error| std::panic::panic_any(ReloadError::RestartRequired(error)))
    }
    pub(crate) fn prepare(
        &self,
        candidate: &Ref<StyledElement>,
    ) -> Result<Box<dyn PreparedReload>, ReloadError> {
        self.services()?;
        let resources = candidate.resources();
        let change = self
            .ownership
            .prepare_with_preview(&resources)
            .map_err(ReloadError::RestartRequired)?;
        // Detach only candidate ownership. Existing resources remain live until commit.
        candidate.set_resources(ResourceDictionary::new());
        Ok(Box::new(Change {
            root: self.root.clone(),
            ownership: self.ownership.clone(),
            change,
        }))
    }
}
struct Change {
    root: WeakRef<StyledElement>,
    ownership: ResourceDictionaryOwnership,
    change: ResourceDictionaryChange,
}
impl PreparedReload for Change {
    fn commit(&mut self) -> Result<(), ReloadError> {
        if !self
            .root
            .upgrade()
            .is_some_and(|root| self.ownership.owns_container(&root.resources()))
        {
            return Err(ReloadError::RestartRequired(
                "Component resources changed after preparation".into(),
            ));
        }
        self.change.commit().map_err(ReloadError::RestartRequired)
    }
    fn rollback(&mut self) {
        self.change.rollback();
    }
    fn finish(&mut self) {
        self.change.finish();
    }
}
#[derive(Clone)]
struct Ambient {
    root: WeakRef<StyledElement>,
    resources: Option<OwnedResources>,
    styles: Option<crate::root_styles::OwnedStyles>,
}
pub(crate) fn services(
    root: &Ref<StyledElement>, resources: Option<&OwnedResources>,
    styles: Option<&crate::root_styles::OwnedStyles>,
) -> Rc<dyn IServiceProvider> {
    Rc::new(Services {
        resources: Ambient {
            root: root.downgrade(), resources: resources.cloned(), styles: styles.cloned(),
        },
        fallback: XamlIlRuntimeHelpers::create_root_service_provider_v2(),
    })
}
struct Services {
    resources: Ambient,
    fallback: Rc<dyn IServiceProvider>,
}
impl IServiceProvider for Services {
    fn get_service(&self, type_: TypeId) -> Option<Rc<dyn Any>> {
        service(type_, || {
            Rc::new(Parents {
                resources: self.resources.clone(),
                fallback: self.fallback.get_service_of(),
            }) as Rc<dyn IFerroXamlIlParentStackProvider>
        })
        .or_else(|| self.fallback.get_service(type_))
    }
}
struct Parents {
    resources: Ambient,
    fallback: Option<Rc<dyn IFerroXamlIlParentStackProvider>>,
}
impl IFerroXamlIlParentStackProvider for Parents {
    fn parents(&self) -> Vec<BoxedValue> {
        let host: Rc<dyn IResourceHost> = Rc::new(self.resources.clone());
        let mut result: Vec<BoxedValue> = vec![Rc::new(ResourceHostRef::Other(host))];
        let mut parent = self.resources.root.upgrade().and_then(|root| root.parent());
        while let Some(current) = parent {
            parent = current.parent();
            result.push(Rc::new(current));
        }
        if let Some(fallback) = &self.fallback {
            result.extend(fallback.parents());
        }
        result
    }
}
impl IResourceNode for Ambient {
    fn has_resources(&self) -> bool {
        true
    }
    fn try_get_resource(
        &self,
        key: &ResourceKey,
        theme: Option<&ThemeVariant>,
    ) -> Option<ResourceValue> {
        let root = self.root.upgrade()?;
        let resources = match &self.resources {
            Some(resources) => resources.application_resource(key, theme),
            None => root.resources().try_get_resource(key, theme),
        };
        resources.or_else(|| match &self.styles {
            Some(styles) => styles.application_resource(key, theme),
            None => root.styles().try_get_resource(key, theme),
        })
    }
}
impl IResourceHost for Ambient {
    fn resources_changed(&self, _: Rc<dyn Fn(&ResourcesChangedEventArgs)>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
    fn notify_hosted_resources_changed(&self, _: ResourcesChangedEventArgs) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ComponentReloadTarget, HotReloadSession, ReloadStatus, ReloadTarget};
    use ferroui_controls::{Border, ContentControl, TextBlock};
    use ferroui_markup_xaml::{
        FromXamlObject, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument,
    };
    fn source(width: &str, text: &str) -> String {
        format!(
            r#"<ContentControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
<ContentControl.Resources><x:Double x:Key="width">{width}</x:Double><x:String x:Key="removable">old</x:String></ContentControl.Resources>
<Border Name="child" Width="{{StaticResource width}}"><TextBlock Text="{text}"/></Border></ContentControl>"#
        )
    }
    fn load(source: &str) -> Ref<ContentControl> {
        ferroui_controls::register_types();
        ferroui_markup_xaml::register_types();
        let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
            RuntimeXamlLoaderDocument::new(source),
            None,
        )
        .unwrap();
        Ref::<ContentControl>::from_xaml_object(&value).unwrap()
    }
    #[test]
    fn dictionary_change_preserves_deferred_factories_and_realized_candidate_identity() {
        use std::cell::Cell;
        let old_count = Rc::new(Cell::new(0));
        let count = old_count.clone();
        let dictionary = ResourceDictionary::new();
        dictionary.add_deferred_fn("lazy", move |_| {
            count.set(count.get() + 1);
            Some(Rc::new("old".to_string()))
        });
        let ownership = ResourceDictionaryOwnership::capture(&dictionary);
        dictionary.add_value("application", "keep".to_string());
        let app_value = dictionary.get(&"application".into()).unwrap();
        let new_count = Rc::new(Cell::new(0));
        let count = new_count.clone();
        let candidate = ResourceDictionary::new();
        candidate.add_deferred_fn("lazy", move |_| {
            count.set(count.get() + 1);
            Some(Rc::new("new".to_string()))
        });
        let mut change = ownership.prepare(&candidate).unwrap();
        assert_eq!(old_count.get(), 0);
        assert_eq!(new_count.get(), 0);
        let resolved_after_prepare = candidate.get(&"lazy".into()).unwrap();
        change.commit().unwrap();
        assert!(Rc::ptr_eq(
            &resolved_after_prepare,
            &dictionary.get(&"lazy".into()).unwrap()
        ));
        assert_eq!(new_count.get(), 1);
        change.rollback();
        assert!(dictionary.contains_deferred_key(&"lazy".into()));
        assert_eq!(old_count.get(), 0);
        assert!(Rc::ptr_eq(
            &app_value,
            &dictionary.get(&"application".into()).unwrap()
        ));
        let original = dictionary.get(&"lazy".into()).unwrap();
        assert_eq!(original.downcast_ref::<String>().unwrap(), "old");
        // Realization leaves ownership valid. A subsequent change must preserve
        // the realized old value on rollback rather than rerun its factory.
        let mut second = ownership.prepare(&candidate).unwrap();
        second.commit().unwrap();
        second.rollback();
        assert!(Rc::ptr_eq(
            &original,
            &dictionary.get(&"lazy".into()).unwrap()
        ));
        assert_eq!(old_count.get(), 1);
        dictionary.set("application", Some(app_value));
        assert!(
            second.commit().is_err(),
            "application writes after preparation were ignored"
        );
        let mut third = ownership.prepare(&candidate).unwrap();
        candidate.set(
            "lazy",
            Some(Rc::new("changed after preparation".to_string())),
        );
        assert!(
            third.commit().is_err(),
            "candidate writes after preparation were ignored"
        );
        assert!(Rc::ptr_eq(
            &original,
            &dictionary.get(&"lazy".into()).unwrap()
        ));
    }
    #[test]
    fn merged_and_theme_providers_transfer_owners_and_release_retired_graphs() {
        use ferroui_base::controls::{IResourceProvider, IThemeVariantProvider};
        let root = ContentControl::new();
        let dictionary = root.resources();
        let old = ResourceDictionary::new();
        old.add_value("merged", 10.0f64);
        dictionary
            .merged_dictionaries()
            .add(Rc::<dyn IResourceProvider>::from(old.clone()));
        let old_theme = ResourceDictionary::new();
        old_theme.add_value("theme", 11.0f64);
        dictionary.add_theme_dictionary(
            ThemeVariant::dark(),
            Rc::<dyn IThemeVariantProvider>::from(old_theme.clone()),
        );
        let ownership = ResourceDictionaryOwnership::capture(&dictionary);
        let candidate = ResourceDictionary::new();
        let new = ResourceDictionary::new();
        new.add_value("merged", 20.0f64);
        candidate
            .merged_dictionaries()
            .add(Rc::<dyn IResourceProvider>::from(new.clone()));
        let new_theme = ResourceDictionary::new();
        new_theme.add_value("theme", 21.0f64);
        candidate.add_theme_dictionary(
            ThemeVariant::dark(),
            Rc::<dyn IThemeVariantProvider>::from(new_theme.clone()),
        );
        let mut change = ownership.prepare(&candidate).unwrap();
        change.commit().unwrap();
        assert!(old.owner().is_none());
        assert!(old_theme.owner().is_none());
        assert!(new.owner() == Some(root.clone().upcast::<StyledElement>().into()));
        assert!(new_theme.owner() == new.owner());
        change.rollback();
        assert!(old.owner() == Some(root.clone().upcast::<StyledElement>().into()));
        assert!(old_theme.owner() == old.owner());
        assert!(new.owner().is_none());
        assert!(new_theme.owner().is_none());
        change.commit().unwrap();
        change.finish();
        let retired = old.downgrade();
        let retired_theme = old_theme.downgrade();
        drop(change);
        drop(old);
        drop(old_theme);
        assert!(retired.upgrade().is_none());
        assert!(retired_theme.upgrade().is_none());
        let root_weak = root.downgrade();
        drop(root);
        assert!(
            root_weak.upgrade().is_none(),
            "ownership tracker retained the component"
        );
    }
    #[test]
    fn inline_resources_preserve_app_keys_and_last_good_graph_across_replacement_and_rollback() {
        let _scope = ferroui_base::threading::Dispatcher::unit_test_scope();
        let baseline = source("10", "initial");
        let root = load(&baseline);
        let dictionary = root.resources();
        let uri = "ferres://inline-resources/View.xaml";
        let target: Rc<dyn ReloadTarget> = Rc::new(
            ComponentReloadTarget::content_control(
                &root,
                uri.into(),
                baseline.clone(),
                RuntimeXamlLoaderConfiguration::default(),
            )
            .with_owned_resources(),
        );
        dictionary.add_value("application", "application value".to_string());
        let application_value = dictionary.get(&"application".into()).unwrap();
        let mut session = HotReloadSession::new();
        session.register_document(uri.into(), baseline).unwrap();
        session.register_target(&target);
        for width in [20, 30, 40] {
            let old = Ref::<Border>::from_xaml_object(&root.content().unwrap()).unwrap();
            let retired = old.downgrade();
            let original = dictionary.get(&"width".into()).unwrap();
            let update = source(&width.to_string(), "{StaticResource application}")
                .replace(r#"<x:String x:Key="removable">old</x:String>"#, "");
            session.submit(uri.into(), update.clone());
            let failed = session
                .apply_pending_with(|| Err(ReloadError::Failed("rollback".into())))
                .unwrap();
            assert_eq!(failed.status, ReloadStatus::Failed, "{failed:?}");
            assert!(root.resources().ptr_eq(&dictionary));
            assert!(Rc::ptr_eq(
                &original,
                &dictionary.get(&"width".into()).unwrap()
            ));
            assert!(Ref::<Border>::from_xaml_object(&root.content().unwrap())
                .unwrap()
                .ptr_eq(&old));
            session.submit(uri.into(), update);
            let result = session.apply_pending().unwrap();
            assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
            ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
            let child = Ref::<Border>::from_xaml_object(&root.content().unwrap()).unwrap();
            assert_eq!(child.width(), width as f64);
            let text = child.child().unwrap().cast::<TextBlock>().unwrap();
            assert_eq!(text.text().as_deref(), Some("application value"));
            assert!(!dictionary.contains_key(&"removable".into()));
            assert!(Rc::ptr_eq(
                &application_value,
                &dictionary.get(&"application".into()).unwrap()
            ));
            drop(old);
            assert!(retired.upgrade().is_none(), "retired child is retained");
        }
        let good = root.content().unwrap();
        session.submit(
            uri.into(),
            source("50", "new").replace(r#"x:Key="width""#, r#"x:Key="missing""#),
        );
        let result = session.apply_pending().unwrap();
        assert_eq!(result.status, ReloadStatus::Failed, "{result:?}");
        assert!(Rc::ptr_eq(&good, &root.content().unwrap()));
        assert_eq!(
            *dictionary
                .get(&"width".into())
                .unwrap()
                .downcast_ref::<f64>()
                .unwrap(),
            40.0
        );
    }
    #[test]
    fn mixed_root_dictionaries_prepare_with_final_precedence_and_rollback() {
        use ferroui_base::controls::IResourceProvider;
        use ferroui_base::media::SolidColorBrush;
        let _scope = ferroui_base::threading::Dispatcher::unit_test_scope();
        let source = |width| format!(r#"<ContentControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
<ContentControl.Resources><ResourceDictionary>
<ResourceDictionary.MergedDictionaries><ResourceDictionary>
<x:Double x:Key="width">{width}</x:Double><Color x:Key="accent">Red</Color>
</ResourceDictionary></ResourceDictionary.MergedDictionaries>
<SolidColorBrush x:Key="brush" Color="{{StaticResource accent}}"/>
</ResourceDictionary></ContentControl.Resources>
<Border Width="{{StaticResource width}}" Background="{{StaticResource brush}}"/>
</ContentControl>"#);
        let baseline = source(10);
        let root = load(&baseline);
        let dictionary = root.resources();
        let uri = "ferres://mixed-resources/View.xaml";
        let target: Rc<dyn ReloadTarget> = Rc::new(ComponentReloadTarget::content_control(
            &root, uri.into(), baseline.clone(), RuntimeXamlLoaderConfiguration::default(),
        ).with_owned_resources());
        let application = ResourceDictionary::new();
        application.add_value("width", 91.0f64);
        dictionary.merged_dictionaries().add(Rc::<dyn IResourceProvider>::from(application.clone()));
        dictionary.add_value("accent", ferroui_base::media::Color::parse("Blue").unwrap());
        let mut session = HotReloadSession::new();
        session.register_document(uri.into(), baseline).unwrap();
        session.register_target(&target);
        for width in [20, 30] {
            let old = root.content().unwrap();
            let reached = std::cell::Cell::new(false);
            session.submit(uri.into(), source(width));
            let failed = session.apply_pending_with(|| {
                reached.set(true);
                let child = Ref::<Border>::from_xaml_object(&root.content().unwrap()).unwrap();
                assert_eq!(child.width(), 91.0);
                Err(ReloadError::Failed("rollback".into()))
            }).unwrap();
            assert!(reached.get(), "{failed:?}");
            assert_eq!(failed.status, ReloadStatus::Failed);
            assert!(Rc::ptr_eq(&old, &root.content().unwrap()));
            assert!(dictionary.merged_dictionaries().get(1).as_ref() == Rc::<dyn IResourceProvider>::from(application.clone()).as_ref());
            session.submit(uri.into(), source(width));
            let outcome = session.apply_pending().unwrap();
            assert_eq!(outcome.status, ReloadStatus::Applied, "{outcome:?}");
            let child = Ref::<Border>::from_xaml_object(&root.content().unwrap()).unwrap();
            assert_eq!(child.width(), 91.0);
            let brush = Ref::<SolidColorBrush>::from_xaml_object(&dictionary.get(&"brush".into()).unwrap()).unwrap();
            assert_eq!(brush.color(), ferroui_base::media::Color::parse("Blue").unwrap());
            assert!(root.resources().ptr_eq(&dictionary));
        }
        // Removing the application override exposes the latest XAML provider,
        // confirming it was committed even while its value was shadowed.
        dictionary.merged_dictionaries().remove(&Rc::<dyn IResourceProvider>::from(application.clone()));
        assert_eq!(*dictionary.try_get_resource(&"width".into(), None).unwrap().unwrap().downcast_ref::<f64>().unwrap(), 30.0);
    }
    #[test]
    fn composed_dictionary_preview_matches_commit_for_themes_nulls_and_provider_order() {
        use ferroui_base::controls::{IResourceProvider, IThemeVariantProvider};
        let provider = |key: &str, value: i32| {
            let dictionary = ResourceDictionary::new();
            dictionary.add_value(key, value);
            dictionary
        };
        let root = ContentControl::new();
        let live = root.resources();
        let old = provider("retired", 1);
        live.merged_dictionaries().add(Rc::<dyn IResourceProvider>::from(old.clone()));
        live.add_theme_dictionary(ThemeVariant::light(), Rc::<dyn IThemeVariantProvider>::from(provider("removed-theme", 2)));
        let ownership = ResourceDictionaryOwnership::capture(&live);
        let before = provider("order", 3);
        let after = provider("order", 4);
        live.merged_dictionaries().insert(0, Rc::<dyn IResourceProvider>::from(before.clone()));
        live.merged_dictionaries().add(Rc::<dyn IResourceProvider>::from(after.clone()));
        live.add_value("direct", 5);
        let dark = provider("theme", 6);
        dark.add_value("inherited", 7);
        live.add_theme_dictionary(ThemeVariant::dark(), Rc::<dyn IThemeVariantProvider>::from(dark.clone()));
        let candidate = ResourceDictionary::new();
        let merged = provider("order", 20);
        merged.add_value("direct", 21);
        merged.add_value("theme", 22);
        merged.add_value("new", 23);
        candidate.merged_dictionaries().add(Rc::<dyn IResourceProvider>::from(merged.clone()));
        candidate.add_value("own", 24);
        candidate.add("null", None);
        let default = provider("theme", 25);
        candidate.add_theme_dictionary(ThemeVariant::default(), Rc::<dyn IThemeVariantProvider>::from(default));
        let custom = ThemeVariant::new("Custom", Some(ThemeVariant::dark()));
        let variants = [None, Some(ThemeVariant::light()), Some(ThemeVariant::dark()), Some(custom)];
        let keys = ["direct", "order", "theme", "inherited", "new", "own", "null", "retired", "removed-theme"];
        let mut expected = Vec::new();
        for theme in &variants {
            for key in &keys {
                expected.push(ownership.preview_resource(&candidate, &(*key).into(), theme.as_ref()).unwrap());
            }
        }
        assert!(ownership.prepare(&candidate).is_err(), "uncomposed callers must still reject this combination");
        let mut change = ownership.prepare_with_preview(&candidate).unwrap();
        for _ in 0..2 {
            change.commit().unwrap();
            let mut actual = Vec::new();
            for theme in &variants {
                for key in &keys { actual.push(live.try_get_resource(&(*key).into(), theme.as_ref())); }
            }
            assert_eq!(actual, expected);
            let number = |key: &str, theme: Option<&ThemeVariant>| *live.try_get_resource(&key.into(), theme).unwrap().unwrap().downcast_ref::<i32>().unwrap();
            assert_eq!(number("direct", None), 5);
            assert_eq!(number("order", None), 4);
            assert_eq!(number("theme", Some(&ThemeVariant::dark())), 6);
            assert_eq!(number("theme", None), 25);
            assert_eq!(live.try_get_resource(&"null".into(), None), Some(None));
            assert!(old.owner().is_none());
            assert!(before.owner().is_some() && after.owner().is_some() && dark.owner().is_some());
            change.rollback();
            assert!(old.owner().is_some());
            assert!(merged.owner().is_none());
            assert!(live.try_get_resource(&"retired".into(), None).is_some());
        }
        change.commit().unwrap();
        change.finish();
        let retired = old.downgrade();
        drop(change); drop(old);
        assert!(retired.upgrade().is_none());
        let empty = ResourceDictionary::new();
        let mut removal = ownership.prepare_with_preview(&empty).unwrap();
        removal.commit().unwrap();
        removal.finish();
        assert_eq!(live.merged_dictionaries().count(), 2);
        assert!(live.theme_dictionary(&ThemeVariant::dark()).is_some());
        assert!(live.theme_dictionary(&ThemeVariant::default()).is_none());
    }
    #[test]
    fn external_owned_key_writes_and_new_key_collisions_require_restart() {
        let _scope = ferroui_base::threading::Dispatcher::unit_test_scope();
        let baseline = source("10", "initial");
        for overwrite in [true, false] {
            let root = load(&baseline);
            let target: Rc<dyn ReloadTarget> = Rc::new(
                ComponentReloadTarget::content_control(
                    &root,
                    "ferres://resource-conflict/View.xaml".into(),
                    baseline.clone(),
                    RuntimeXamlLoaderConfiguration::default(),
                )
                .with_owned_resources(),
            );
            let mut session = HotReloadSession::new();
            let uri = "ferres://resource-conflict/View.xaml";
            session
                .register_document(uri.into(), baseline.clone())
                .unwrap();
            session.register_target(&target);
            let original = root.content().unwrap();
            if overwrite {
                let same = root.resources().get(&"width".into());
                root.resources().set("width", same);
                session.submit(uri.into(), source("20", "new"));
            } else {
                root.resources().add_value("application", 30.0f64);
                session.submit(
                    uri.into(),
                    source("20", "new").replace(
                        "</ContentControl.Resources>",
                        r#"<x:Double x:Key="application">40</x:Double></ContentControl.Resources>"#,
                    ),
                );
            }
            let result = session.apply_pending().unwrap();
            assert_eq!(result.status, ReloadStatus::RestartRequired, "{result:?}");
            assert!(Rc::ptr_eq(&original, &root.content().unwrap()));
        }
    }
    #[test]
    fn panel_resource_replacement_preserves_application_children() {
        use ferroui_controls::Panel;
        let _scope = ferroui_base::threading::Dispatcher::unit_test_scope();
        ferroui_controls::register_types();
        ferroui_markup_xaml::register_types();
        let source = |width| {
            format!(
                r#"<StackPanel xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><StackPanel.Resources><x:Double x:Key="width">{width}</x:Double></StackPanel.Resources><Border Width="{{StaticResource width}}"/></StackPanel>"#
            )
        };
        let baseline = source(10);
        let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
            RuntimeXamlLoaderDocument::new(&baseline),
            None,
        )
        .unwrap();
        let root = Ref::<Panel>::from_xaml_object(&value).unwrap();
        let uri = "ferres://panel-resources/View.xaml";
        let target: Rc<dyn ReloadTarget> = Rc::new(
            crate::PanelReloadTarget::new(
                &root,
                0..1,
                uri.into(),
                baseline.clone(),
                RuntimeXamlLoaderConfiguration::default(),
            )
            .unwrap()
            .with_owned_resources(),
        );
        let application_child = Border::new();
        application_child.set_width(91.0);
        root.children().add(
            application_child
                .clone()
                .upcast::<ferroui_controls::Control>(),
        );
        let dictionary = root.resources();
        let mut session = HotReloadSession::new();
        session.register_document(uri.into(), baseline).unwrap();
        session.register_target(&target);
        for width in [20, 30, 40] {
            let old = root.children().to_vec()[0].clone();
            session.submit(uri.into(), source(width));
            let failed = session
                .apply_pending_with(|| Err(ReloadError::Failed("rollback".into())))
                .unwrap();
            assert_eq!(failed.status, ReloadStatus::Failed, "{failed:?}");
            assert!(root.children().to_vec()[0].ptr_eq(&old));
            session.submit(uri.into(), source(width));
            let result = session.apply_pending().unwrap();
            assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
            ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(root.children().to_vec()[0].width(), width as f64);
            assert!(root.children().to_vec()[1]
                .cast::<Border>()
                .unwrap()
                .ptr_eq(&application_child));
            assert_eq!(application_child.width(), 91.0);
            assert!(root.resources().ptr_eq(&dictionary));
        }
    }
    #[test]
    fn inline_theme_templates_publish_with_resources_and_restore_the_old_template_on_failure() {
        use ferroui_base::threading::Dispatcher;
        use ferroui_controls::{
            testing::{TestServices, UnitTestApplication},
            Control, Window,
        };
        let _application = UnitTestApplication::start(TestServices::styled_window());
        let source = |value| {
            format!(
                r#"<ContentControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml" Theme="{{StaticResource RootTheme}}"><ContentControl.Resources><x:Double x:Key="width">{value}</x:Double><ControlTheme x:Key="RootTheme" TargetType="ContentControl"><Setter Property="Opacity" Value="{value}"/><Setter Property="Template"><ControlTemplate><Border Width="{{StaticResource width}}"/></ControlTemplate></Setter></ControlTheme></ContentControl.Resources><Border/></ContentControl>"#
            )
        };
        let baseline = source(0.2);
        let root = load(&baseline);
        let window = Window::new();
        window.set_content(Some(Control::boxed(&root.clone().upcast::<Control>())));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let target: Rc<dyn ReloadTarget> = Rc::new(
            ComponentReloadTarget::content_control(
                &root,
                "ferres://inline-theme/View.xaml".into(),
                baseline.clone(),
                RuntimeXamlLoaderConfiguration::default(),
            )
            .with_owned_resources()
            .with_owned_root_static_resource_property(StyledElement::theme_property()),
        );
        let mut session = HotReloadSession::new();
        let uri = "ferres://inline-theme/View.xaml";
        session.register_document(uri.into(), baseline).unwrap();
        session.register_target(&target);
        let old = root.visual_children().get(0);
        let old_theme = root.theme().unwrap();
        let retired = old.downgrade();
        let retired_theme = old_theme.downgrade();
        session.submit(uri.into(), source(0.4));
        let committed = std::cell::Cell::new(false);
        let failed = session
            .apply_pending_with(|| {
                committed.set(true);
                assert!(!root.theme().unwrap().ptr_eq(&old_theme));
                Err(ReloadError::Failed("rollback".into()))
            })
            .unwrap();
        assert_eq!(failed.status, ReloadStatus::Failed, "{failed:?}");
        assert!(
            committed.get(),
            "Candidate did not reach commit: {failed:?}"
        );
        Dispatcher::ui_thread().run_jobs(None);
        assert!(root.visual_children().get(0).ptr_eq(&old));
        assert!(root.theme().unwrap().ptr_eq(&old_theme));
        assert_eq!(root.opacity(), 0.2);
        for invalid in [
            source(0.5).replace("Width=\"{StaticResource width}\"", "Width=\"{StaticResource absent}\""),
            source(0.5).replace("<x:Double x:Key=\"width\">0.5</x:Double>",
                "<x:String x:Key=\"width\">invalid width</x:String>"),
        ] {
            session.submit(uri.into(), invalid);
            let result = session.apply_pending().unwrap();
            assert_eq!(result.status, ReloadStatus::Failed, "{result:?}");
            Dispatcher::ui_thread().run_jobs(None);
            assert!(root.visual_children().get(0).ptr_eq(&old));
            assert!(root.theme().unwrap().ptr_eq(&old_theme));
        }
        session.submit(uri.into(), source(0.6));
        let result = session.apply_pending().unwrap();
        assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(root.opacity(), 0.6);
        assert_eq!(
            root.visual_children()
                .get(0)
                .cast::<Border>()
                .unwrap()
                .width(),
            0.6
        );
        let published = Ref::<ferroui_base::styling::ControlTheme>::from_xaml_object(
            &root.resources().get(&"RootTheme".into()).unwrap(),
        )
        .unwrap();
        assert!(published.ptr_eq(&root.theme().unwrap()));
        drop(old);
        drop(old_theme);
        assert!(retired.upgrade().is_none());
        assert!(retired_theme.upgrade().is_none());
        window.close();
    }

    #[test]
    fn realized_inline_brush_keeps_dynamic_resource_subscription_after_reload() {
        use ferroui_base::media::{Color, SolidColorBrush};
        let _scope = ferroui_base::threading::Dispatcher::unit_test_scope();
        let source = |color| format!(r#"<ContentControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><ContentControl.Resources><Color x:Key="accent">{color}</Color><SolidColorBrush x:Key="brush" Color="{{DynamicResource accent}}"/></ContentControl.Resources><Border Background="{{StaticResource brush}}"/></ContentControl>"#);
        let baseline = source("Red");
        let root = load(&baseline);
        let uri = "ferres://inline-dynamic/View.xaml";
        let target: Rc<dyn ReloadTarget> = Rc::new(ComponentReloadTarget::content_control(
            &root, uri.into(), baseline.clone(), RuntimeXamlLoaderConfiguration::default(),
        ).with_owned_resources());
        let mut session = HotReloadSession::new();
        session.register_document(uri.into(), baseline).unwrap();
        session.register_target(&target);
        for color in ["Blue", "Green", "Blue"] {
            let old = Ref::<SolidColorBrush>::from_xaml_object(&root.resources().get(&"brush".into()).unwrap()).unwrap();
            let old_color = old.color();
            session.submit(uri.into(), source(color));
            let reached_commit = std::cell::Cell::new(false);
            let result = session.apply_pending_with(|| {
                reached_commit.set(true);
                Err(ReloadError::Failed("rollback".into()))
            }).unwrap();
            assert_eq!(result.status, ReloadStatus::Failed, "{result:?}");
            assert!(reached_commit.get(), "{result:?}");
            assert_eq!(old.color(), old_color);
            let restored = Ref::<SolidColorBrush>::from_xaml_object(&root.resources().get(&"brush".into()).unwrap()).unwrap();
            assert!(restored.ptr_eq(&old));
            drop(restored);
            session.submit(uri.into(), source(color));
            let result = session.apply_pending().unwrap();
            assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
            let brush = Ref::<SolidColorBrush>::from_xaml_object(&root.resources().get(&"brush".into()).unwrap()).unwrap();
            assert_eq!(brush.color(), Color::parse(color).unwrap());
            let retired = old.downgrade();
            drop(old);
            assert!(retired.upgrade().is_none());
        }
        let brush = Ref::<SolidColorBrush>::from_xaml_object(&root.resources().get(&"brush".into()).unwrap()).unwrap();
        root.resources().set("accent", Some(Rc::new(Color::parse("Green").unwrap())));
        assert_eq!(brush.color(), Color::parse("Green").unwrap());
        let weak_root = root.downgrade();
        drop(root);
        assert!(weak_root.upgrade().is_none());
    }
}
