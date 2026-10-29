//! Detached framework roots and composed resource lookup during population.
use crate::{root_resources::OwnedResources, root_styles::OwnedStyles, ReloadError};
use ferroui_base::{
    controls::{resource_preview::{ResourcePreviewGuard, ResourcePreviewResolver}, IResourceNode, ResourceKey, ResourceValue},
    styling::ThemeVariant, Ref, StyledElement,
};
use std::rc::Rc;

pub(crate) fn create(source: &str) -> Result<Ref<StyledElement>, ReloadError> {
    let xml = roxmltree::Document::parse(source).map_err(|error| ReloadError::Failed(error.to_string()))?;
    use ferroui_controls::*;
    use ferroui_controls::primitives::UniformGrid;
    Ok(match xml.root_element().tag_name().name() {
        "ContentControl" => ContentControl::new().upcast(),
        "ContentPage" => ContentPage::new().upcast(),
        "UserControl" => UserControl::new().upcast(),
        "Panel" => Panel::new().upcast(),
        "StackPanel" => StackPanel::new().upcast(),
        "WrapPanel" => WrapPanel::new().upcast(),
        "DockPanel" => DockPanel::new().upcast(),
        "Canvas" => Canvas::new().upcast(),
        "Grid" => Grid::new().upcast(),
        "UniformGrid" => UniformGrid::new().upcast(),
        _ => return Err(ReloadError::RestartRequired("Custom roots require an adapter".into())),
    })
}

#[derive(Clone, Copy)]
enum Kind { Root, Resources, Styles, StyleResources }
struct Preview {
    root: Ref<StyledElement>,
    candidate: Ref<StyledElement>,
    resources: Option<OwnedResources>,
    styles: Option<OwnedStyles>,
    kind: Kind,
}
impl Preview {
    fn resources(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        match &self.resources {
            Some(resources) => resources.preview_resource(&self.candidate.resources(), key, theme),
            None => self.candidate.resources().try_get_resource(key, theme)
                .or_else(|| self.root.resources().try_get_resource(key, theme)),
        }
    }
    fn styles(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        match &self.styles {
            Some(styles) => styles.preview_resource(&self.candidate.styles(), key, theme),
            None => self.root.styles().try_get_resource(key, theme),
        }
    }
}
impl IResourceNode for Preview {
    fn has_resources(&self) -> bool { true }
    fn try_get_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        match self.kind {
            Kind::Root => self.resources(key, theme).or_else(|| self.styles(key, theme)),
            Kind::Resources => self.resources(key, theme),
            Kind::Styles => self.styles(key, theme),
            Kind::StyleResources => match &self.styles {
                Some(styles) => styles.preview_dictionary(&self.candidate.styles().resources(), key, theme),
                None => self.root.styles().resources().try_get_resource(key, theme),
            },
        }
    }
}
/// Keeps enclosing include/dependency previews active. The caller must retain
/// these guards through candidate population, validation and extraction.
pub(crate) fn install(
    root: &Ref<StyledElement>, candidate: &Ref<StyledElement>,
    resources: Option<&OwnedResources>, styles: Option<&OwnedStyles>,
) -> Scope {
    let mut guards = Vec::new();
    for kind in [Kind::Root, Kind::Resources, Kind::Styles, Kind::StyleResources] {
        let preview: Rc<dyn IResourceNode> = Rc::new(Preview {
            root: root.clone(), candidate: candidate.clone(),
            resources: resources.cloned(), styles: styles.cloned(), kind,
        });
        let factory = Rc::new(move || preview.clone());
        guards.push(match kind {
            Kind::Root => ResourcePreviewGuard::new(&**candidate, factory),
            Kind::Resources => {
                let provider: Rc<dyn ferroui_base::controls::IResourceProvider> = candidate.resources().into();
                ResourcePreviewGuard::new(&*provider, factory)
            }
            Kind::Styles => {
                let style: Rc<dyn ferroui_base::styling::IStyle> = candidate.styles().into();
                ResourcePreviewGuard::new(&*style, factory)
            }
            Kind::StyleResources => {
                let provider: Rc<dyn ferroui_base::controls::IResourceProvider> = candidate.styles().resources().into();
                ResourcePreviewGuard::new(&*provider, factory)
            }
        });
    }
    // An explicit dictionary is assigned after the loader constructs it. Follow
    // the current containers so lazy values cannot bypass the composed preview
    // by resolving through that newly assigned dictionary on their parent stack.
    let root = root.clone();
    let candidate = candidate.clone();
    let resources = resources.cloned();
    let styles = styles.cloned();
    let resolver = ResourcePreviewResolver::new(Rc::new(move |node| {
        use ferroui_base::controls::IResourceProvider;
        let dictionary: Rc<dyn IResourceProvider> = candidate.resources().into();
        let style_dictionary: Rc<dyn IResourceProvider> = candidate.styles().resources().into();
        let kind = if node.reference_id() == dictionary.reference_id() { Kind::Resources }
            else if node.reference_id() == style_dictionary.reference_id() { Kind::StyleResources }
            else { return None; };
        Some(Rc::new(Preview {
            root: root.clone(), candidate: candidate.clone(),
            resources: resources.clone(), styles: styles.clone(), kind,
        }))
    }));
    Scope { guards, resolver: Some(resolver) }
}
pub(crate) struct Scope {
    guards: Vec<ResourcePreviewGuard>,
    resolver: Option<ResourcePreviewResolver>,
}
impl Drop for Scope {
    fn drop(&mut self) {
        self.resolver.take();
        while self.guards.pop().is_some() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::controls::{resource_preview::{lookup, ResourcePreviewScope}, IResourceProvider, ResourceDictionary};
    fn node(value: i32) -> Rc<dyn IResourceNode> {
        let dictionary = ResourceDictionary::new();
        dictionary.add_value("value", value);
        let provider: Rc<dyn IResourceProvider> = dictionary.into();
        provider
    }
    fn value(node: &Rc<dyn IResourceNode>) -> i32 {
        *lookup(&**node, &"value".into(), None).unwrap().unwrap().downcast_ref::<i32>().unwrap()
    }
    #[test]
    fn reassigned_candidate_dictionaries_use_composed_lookup_until_unwind() {
        let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
        let root: Ref<StyledElement> = ferroui_controls::ContentControl::new().upcast();
        let owned = OwnedResources::capture(&root);
        root.resources().add_value("value", 1i32);
        let candidate: Ref<StyledElement> = ferroui_controls::ContentControl::new().upcast();
        let replacement = ResourceDictionary::new();
        let merged = ResourceDictionary::new();
        merged.add_value("value", 2i32);
        replacement.merged_dictionaries().add(Rc::<dyn IResourceProvider>::from(merged));
        let provider: Rc<dyn IResourceNode> = Rc::<dyn IResourceProvider>::from(replacement.clone());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _preview = install(&root, &candidate, Some(&owned), None);
            candidate.set_resources(replacement.clone());
            assert_eq!(value(&provider), 1, "new dictionary bypassed application direct-key precedence");
            panic!("candidate failed");
        }));
        assert!(result.is_err());
        assert_eq!(value(&provider), 2, "candidate resolver survived unwind");
        let retired = candidate.downgrade();
        drop(candidate);
        assert!(retired.upgrade().is_none());
    }
    #[test]
    fn candidate_overrides_preserve_dependency_previews_and_restore_on_unwind() {
        let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
        let dependency = node(1);
        let candidate = node(10);
        let scope = ResourcePreviewScope::new();
        let replacement = node(2);
        assert!(scope.insert(&*dependency, Rc::new(move || replacement.clone())));
        let preview = node(20);
        let guard = ResourcePreviewGuard::new(&*candidate, Rc::new(move || preview.clone()));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let nested = node(30);
            let _inner = ResourcePreviewGuard::new(&*candidate, Rc::new(move || nested.clone()));
            assert_eq!(value(&candidate), 30);
            assert_eq!(value(&dependency), 2);
            panic!("candidate failed");
        }));
        assert!(result.is_err());
        assert_eq!(value(&candidate), 20);
        assert_eq!(value(&dependency), 2);
        drop(guard);
        assert_eq!(value(&candidate), 10);
        assert_eq!(value(&dependency), 2);
        drop(scope);
        assert_eq!(value(&dependency), 1);
        let preview = node(40);
        let standalone = ResourcePreviewGuard::new(&*candidate, Rc::new(move || preview.clone()));
        assert_eq!(value(&candidate), 40);
        drop(standalone);
        assert_eq!(value(&candidate), 10);
    }
}
