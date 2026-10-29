//! Tracks includes in explicitly registered containers without retaining live owners.
use crate::{ReloadError, ReloadTarget, ResourceReloadTarget, StyleReloadTarget};
use ferroui_base::{controls::ResourceDictionary, styling::Styles, Ref, WeakRef};
use ferroui_markup_xaml::{source_overlay::Sources, RuntimeXamlLoaderConfiguration};
use std::rc::Rc;

struct ResourceOwner {
    owner: WeakRef<ResourceDictionary>,
    configuration: RuntimeXamlLoaderConfiguration,
    targets: Vec<Rc<ResourceReloadTarget>>,
}
struct StyleOwner {
    owner: WeakRef<Styles>,
    configuration: RuntimeXamlLoaderConfiguration,
    targets: Vec<Rc<StyleReloadTarget>>,
}
#[derive(Default)]
pub(crate) struct IncludeTracking {
    resources: Vec<ResourceOwner>,
    styles: Vec<StyleOwner>,
}
impl IncludeTracking {
    pub(crate) fn resources(
        &mut self,
        owner: &Ref<ResourceDictionary>,
        configuration: RuntimeXamlLoaderConfiguration,
    ) {
        if !self.resources.iter().any(|entry| {
            entry
                .owner
                .upgrade()
                .is_some_and(|existing| existing.ptr_eq(owner))
        }) {
            self.resources.push(ResourceOwner {
                owner: owner.downgrade(),
                configuration,
                targets: Vec::new(),
            });
        }
    }
    pub(crate) fn styles(
        &mut self,
        owner: &Ref<Styles>,
        configuration: RuntimeXamlLoaderConfiguration,
    ) {
        if !self.styles.iter().any(|entry| {
            entry
                .owner
                .upgrade()
                .is_some_and(|existing| existing.ptr_eq(owner))
        }) {
            self.styles.push(StyleOwner {
                owner: owner.downgrade(),
                configuration,
                targets: Vec::new(),
            });
        }
    }
    /// Reuse accepted adapters: replacement values need not carry the original
    /// include wrapper's provenance. New contributions must have registered sources.
    pub(crate) fn refresh(
        &mut self,
        sources: &Sources,
    ) -> Result<(Vec<Rc<dyn ReloadTarget>>, bool), ReloadError> {
        let mut changed = false;
        let mut targets: Vec<Rc<dyn ReloadTarget>> = Vec::new();
        let previous = self.resources.len() + self.styles.len();
        self.resources
            .retain(|entry| entry.owner.upgrade().is_some());
        self.styles.retain(|entry| entry.owner.upgrade().is_some());
        changed |= previous != self.resources.len() + self.styles.len();
        for entry in &mut self.resources {
            let Some(owner) = entry.owner.upgrade() else {
                continue;
            };
            let previous = entry.targets.len();
            entry.targets.retain(|target| target.is_attached());
            changed |= previous != entry.targets.len();
            for (index, provider) in owner.merged_dictionaries().to_vec().iter().enumerate() {
                if entry.targets.iter().any(|target| target.tracks(provider)) {
                    continue;
                }
                let Some(uri) = provider.development_source() else {
                    continue;
                };
                if !sources.contains_key(&crate::document_key(&uri)) {
                    continue;
                }
                entry.targets.push(Rc::new(ResourceReloadTarget::merged(
                    &owner,
                    index,
                    uri,
                    entry.configuration.clone(),
                )?));
                changed = true;
            }
            targets.extend(
                entry
                    .targets
                    .iter()
                    .cloned()
                    .map(|target| target as Rc<dyn ReloadTarget>),
            );
        }
        for entry in &mut self.styles {
            let Some(owner) = entry.owner.upgrade() else {
                continue;
            };
            let previous = entry.targets.len();
            entry.targets.retain(|target| target.is_attached());
            changed |= previous != entry.targets.len();
            for (index, style) in owner.snapshot().iter().enumerate() {
                if entry.targets.iter().any(|target| target.tracks(style)) {
                    continue;
                }
                let Some(uri) = style
                    .as_resource_provider()
                    .and_then(|provider| provider.development_source())
                else {
                    continue;
                };
                if !sources.contains_key(&crate::document_key(&uri)) {
                    continue;
                }
                entry.targets.push(Rc::new(StyleReloadTarget::contribution(
                    &owner,
                    index,
                    uri,
                    entry.configuration.clone(),
                )?));
                changed = true;
            }
            targets.extend(
                entry
                    .targets
                    .iter()
                    .cloned()
                    .map(|target| target as Rc<dyn ReloadTarget>),
            );
        }
        Ok((targets, changed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContentReloadTarget, HotReloadSession, ReloadStatus};
    use ferroui_base::utilities::{Uri, UriKind};
    use ferroui_controls::{
        testing::{TestServices, UnitTestApplication},
        Application, Border, ContentControl,
    };
    use ferroui_markup_xaml::FromXamlObject;

    fn dynamic_include_lifecycle(style: bool) {
        let _application = UnitTestApplication::start(TestServices::styled_window());
        ferroui_controls::register_types();
        ferroui_markup_xaml::register_types();
        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::register();
        let values = |value| {
            let content = format!(r#"<x:Double x:Key="width">{value}</x:Double>"#);
            let root = if style {
                "Styles"
            } else {
                "ResourceDictionary"
            };
            let content = if style {
                format!("<Styles.Resources>{content}</Styles.Resources>")
            } else {
                content
            };
            format!(
                r#"<{root} xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">{content}</{root}>"#
            )
        };
        let resource_uri = "ferres://tracked/Values.xaml";
        let view_uri = "ferres://tracked/View.xaml";
        let view = r#"<Border xmlns="https://github.com/ferroui" Width="{StaticResource width}"/>"#;
        let mut session = HotReloadSession::new();
        session
            .register_document(resource_uri.into(), values(7))
            .unwrap();
        session
            .register_document(view_uri.into(), view.into())
            .unwrap();
        let application = Application::current().unwrap();
        let resources = application.resources();
        let styles = application.styles();
        let index = if style {
            styles.count()
        } else {
            resources.merged_dictionaries().count()
        };
        // Registration is idempotent and may precede all include contributions.
        for _ in 0..2 {
            if style {
                session.track_style_includes(&styles, RuntimeXamlLoaderConfiguration::default());
            } else {
                session
                    .track_resource_includes(&resources, RuntimeXamlLoaderConfiguration::default());
            }
        }
        let attach = || {
            let base = Uri::try_create(view_uri, UriKind::Absolute).unwrap();
            let source = Uri::try_create("Values.xaml", UriKind::RelativeOrAbsolute);
            if style {
                let include = ferroui_markup_xaml::styling::StyleInclude::new(Some(base));
                include.set_source(source);
                styles.add(include.as_style());
            } else {
                let include = ferroui_markup_xaml::styling::ResourceInclude::new(Some(base));
                include.set_source(source);
                resources.add_merged_dictionary(include.as_resource_provider());
            }
        };
        let host = ContentControl::new();
        let consumer: Rc<dyn ReloadTarget> = Rc::new(ContentReloadTarget::new(
            &host,
            view_uri.into(),
            vec![],
            RuntimeXamlLoaderConfiguration::default(),
        ));
        session.register_target(&consumer);
        session.submit(resource_uri.into(), values(8));
        assert_eq!(
            session.apply_pending().unwrap().status,
            ReloadStatus::RestartRequired
        );
        assert!(
            session.apply_pending().is_none(),
            "a failed draft must not spin"
        );
        attach();
        let applied = session
            .apply_pending()
            .expect("a new include must retry the pending draft");
        assert_eq!(applied.status, ReloadStatus::Applied, "{applied:?}");
        let width = || {
            Ref::<Border>::from_xaml_object(&host.content().unwrap())
                .unwrap()
                .width()
        };
        assert_eq!(width(), 8.0, "consumer has no explicit resource dependency");
        for value in 9..=11 {
            let before = host.content();
            let retired = {
                let provider: Rc<dyn ferroui_base::controls::IResourceNode> = if style {
                    styles.snapshot()[index].clone()
                } else {
                    resources.merged_dictionaries().to_vec()[index].clone()
                };
                Rc::downgrade(&provider)
            };
            session.submit(resource_uri.into(), values(value));
            let failed = session
                .apply_pending_with(|| Err(ReloadError::Failed("rollback".into())))
                .unwrap();
            assert_eq!(failed.status, ReloadStatus::Failed, "{failed:?}");
            assert_eq!(host.content(), before);
            assert_eq!(width(), (value - 1) as f64);
            session.submit(resource_uri.into(), values(value));
            let applied = session.apply_pending().unwrap();
            assert_eq!(applied.status, ReloadStatus::Applied, "{applied:?}");
            assert_eq!(
                width(),
                value as f64,
                "accepted replacements must stay tracked"
            );
            assert!(
                retired.upgrade().is_none(),
                "discovery retained a retired contribution"
            );
        }
        let before = host.content();
        session.submit(resource_uri.into(), values(12).replace(">12<", ">invalid<"));
        assert_eq!(
            session.apply_pending().unwrap().status,
            ReloadStatus::Failed
        );
        assert_eq!(host.content(), before);
        if style {
            styles.remove_at(index);
        } else {
            resources.merged_dictionaries().remove_at(index);
        }
        session.submit(resource_uri.into(), values(12));
        assert_eq!(
            session.apply_pending().unwrap().status,
            ReloadStatus::RestartRequired
        );
        assert_eq!(
            host.content(),
            before,
            "removed contribution must not be silently reclaimed"
        );
        attach();
        let applied = session.apply_pending().unwrap();
        assert_eq!(applied.status, ReloadStatus::Applied, "{applied:?}");
        assert_eq!(width(), 12.0);
    }
    #[test]
    fn resource_includes_added_and_removed_after_startup_are_tracked() {
        dynamic_include_lifecycle(false);
    }
    #[test]
    fn style_includes_added_and_removed_after_startup_are_tracked() {
        dynamic_include_lifecycle(true);
    }
    #[test]
    fn tracked_containers_and_their_contributions_are_weakly_owned() {
        use ferroui_base::{threading::Dispatcher, FerroObject};
        use ferroui_markup_xaml::RuntimeXamlLoaderDocument;
        let _scope = Dispatcher::unit_test_scope();
        ferroui_controls::register_types();
        ferroui_markup_xaml::register_types();
        for style in [false, true] {
            let root = if style {
                "Styles"
            } else {
                "ResourceDictionary"
            };
            let source = format!(r#"<{root} xmlns="https://github.com/ferroui"/>"#);
            let uri = "ferres://weak-includes/Value.xaml";
            let mut session = HotReloadSession::new();
            session
                .register_document(uri.into(), source.clone())
                .unwrap();
            let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
                RuntimeXamlLoaderDocument::with_base_uri(
                    Uri::try_create(uri, UriKind::Absolute),
                    &source,
                ),
                None,
            )
            .unwrap();
            let owner: Ref<FerroObject> = if style {
                let owner = Styles::new();
                owner.add(Ref::<Styles>::from_xaml_object(&value).unwrap());
                session.track_style_includes(&owner, RuntimeXamlLoaderConfiguration::default());
                owner.upcast()
            } else {
                let owner = ResourceDictionary::new();
                owner.add_merged_dictionary(
                    Ref::<ResourceDictionary>::from_xaml_object(&value).unwrap(),
                );
                session.track_resource_includes(&owner, RuntimeXamlLoaderConfiguration::default());
                owner.upcast()
            };
            let weak = owner.downgrade();
            drop(value);
            session.submit(uri.into(), source.clone());
            let applied = session.apply_pending().unwrap();
            assert_eq!(applied.status, ReloadStatus::Applied, "{applied:?}");
            drop(owner);
            assert!(
                weak.upgrade().is_none(),
                "session retained a registered owner"
            );
            session.submit(uri.into(), source);
            let orphaned = session.apply_pending().unwrap();
            assert_eq!(
                orphaned.status,
                ReloadStatus::RestartRequired,
                "{orphaned:?}"
            );
            assert!(session.apply_pending().is_none());
        }
    }
}
