//! Ownership of inline styles on a preserved component or panel.
use crate::{PreparedReload, ReloadError};
use ferroui_base::{
    controls::{ResourceDictionary, ResourceDictionaryOwnership, ResourceKey, ResourceValue},
    styling::{style_ptr_eq, IStyle, Styles, ThemeVariant},
    Ref, StyledElement, WeakRef,
};
use std::{cell::RefCell, rc::{Rc, Weak}};

/// Admit style bindings before validation applies styling to a detached root.
/// Known pull/two-way bindings can be inspected without starting subscriptions;
/// source-only, delayed and custom bindings need an explicit transaction adapter.
pub(crate) fn validate_bindings(root: &Ref<StyledElement>, live: &Ref<StyledElement>) -> Result<(), ReloadError> {
    use ferroui_base::{data::{CompiledBinding, ReflectionBinding}, styling::{Style, StyleBase, Setter, SetterValue, SelectorMatchResult}};
    use ferroui_markup_xaml::markup_extensions::{CompiledBindingExtension, ReflectionBindingExtension, DynamicResourceExtension};
    let mut pending = root.styles().snapshot().to_vec();
    if let Some(theme) = root.get_effective_theme() {
        let style: Rc<dyn IStyle> = theme.into();
        pending.push(style);
    }
    let mut visited = std::collections::HashSet::new();
    while let Some(style) = pending.pop() {
        if !visited.insert(style.reference_id() as usize) { continue; }
        pending.extend(style.children().iter().cloned());
        if let Some(parent) = style.as_object()
            .and_then(|object| object.downcast_ref::<ferroui_base::styling::ControlTheme>())
            .and_then(|theme| theme.based_on()) {
            let parent: Rc<dyn IStyle> = parent.into();
            pending.push(parent);
        }
        let Some(base) = style.as_object().and_then(|object| object.downcast_ref::<StyleBase>()) else { continue };
        let can_target_root = base.downcast_ref::<Style>().is_none_or(|style| {
            style.selector().is_none_or(|selector| {
                selector.match_(live, style.parent().as_deref(), false).result() != SelectorMatchResult::NeverThisType
            })
        });
        for setter in base.setters().to_vec() {
            let setter = setter.as_any().and_then(|setter| setter.downcast_ref::<Setter>())
                .ok_or_else(|| ReloadError::RestartRequired("Custom style setters require an adapter".into()))?;
            let Some(property) = setter.property() else { continue };
            if can_target_root && (property.name() == "DataContext" || property.is_direct()) {
                return Err(ReloadError::RestartRequired("A root DataContext or direct-property style setter requires an adapter".into()));
            }
            match setter.value() {
                Some(SetterValue::BindingBase(binding)) => {
                    let kind = binding.as_any();
                    if kind.is_some_and(|kind| kind.is::<DynamicResourceExtension>()) { continue; }
                    let supported = kind.is_some_and(|kind| {
                        kind.is::<ReflectionBinding>() || kind.is::<CompiledBinding>()
                            || kind.is::<ReflectionBindingExtension>() || kind.is::<CompiledBindingExtension>()
                    });
                    if !supported || property.is_direct() {
                        return Err(ReloadError::RestartRequired("Custom/direct style bindings require an adapter".into()));
                    }
                    let target = root.clone().upcast();
                    let expression = binding.create_instance(&target, Some(property), Some(&target));
                    if !expression.can_pause_for_reload() {
                        return Err(ReloadError::RestartRequired("Source-only or delayed style bindings require an adapter".into()));
                    }
                }
                Some(SetterValue::Binding(_)) => {
                    return Err(ReloadError::RestartRequired("Observable style setters require an adapter".into()));
                }
                _ => {}
            }
        }
    }
    Ok(())
}
/// A style setter may select a different theme. Validate that theme's setters
/// before applying it, and settle the candidate before inspecting its template.
pub(crate) fn prepare_styling(root: &Ref<StyledElement>, live: &Ref<StyledElement>) -> Result<(), ReloadError> {
    for _ in 0..8 {
        validate_bindings(root, live)?;
        let previous = root.get_effective_theme();
        root.try_apply_styling().map_err(|error| ReloadError::Failed(error.to_string()))?;
        if root.get_effective_theme() == previous {
            return Ok(());
        }
    }
    Err(ReloadError::Failed("Candidate theme selection did not stabilize".into()))
}

#[derive(Default)]
struct Entries {
    index: usize,
    styles: Vec<Weak<dyn IStyle>>,
}
#[derive(Clone)]
pub(crate) struct OwnedStyles {
    root: WeakRef<StyledElement>,
    container: WeakRef<Styles>,
    entries: Rc<RefCell<Entries>>,
    resources: ResourceDictionaryOwnership,
}
impl OwnedStyles {
    pub(crate) fn capture(root: &Ref<StyledElement>) -> Self {
        let container = root.styles();
        Self {
            root: root.downgrade(),
            container: container.downgrade(),
            entries: Rc::new(RefCell::new(Entries {
                index: 0,
                styles: container.snapshot().iter().map(Rc::downgrade).collect(),
            })),
            resources: ResourceDictionaryOwnership::capture(&container.resources()),
        }
    }
    fn current(&self) -> Result<(Ref<Styles>, usize, Vec<Rc<dyn IStyle>>), ReloadError> {
        let root = self.root.upgrade().ok_or_else(|| ReloadError::Failed("Component closed".into()))?;
        let container = self.container.upgrade().ok_or_else(|| ReloadError::Failed("Style owner closed".into()))?;
        if !root.styles().ptr_eq(&container) || !self.resources.owns_container(&container.resources()) {
            return Err(ReloadError::RestartRequired("Application replaced owned style resources".into()));
        }
        let owned = self.entries.borrow();
        let styles = owned.styles.iter().map(|weak| weak.upgrade().ok_or_else(|| {
            ReloadError::RestartRequired("Owned inline style was removed".into())
        })).collect::<Result<Vec<_>, _>>()?;
        let entries = container.snapshot();
        for style in &styles {
            if entries.iter().filter(|entry| style_ptr_eq(entry, style)).count() != 1 {
                return Err(ReloadError::RestartRequired("Ambiguous inline style ownership".into()));
            }
        }
        let index = styles.first().and_then(|first| entries.iter().position(|entry| style_ptr_eq(entry, first)))
            .unwrap_or(owned.index);
        if entries.get(index..index + styles.len()).is_none_or(|range| {
            range.iter().zip(&styles).any(|(a, b)| !style_ptr_eq(a, b))
        }) {
            return Err(ReloadError::RestartRequired("Owned inline styles are no longer contiguous".into()));
        }
        Ok((container, index, styles))
    }
    pub(crate) fn application_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        let container = self.container.upgrade()?;
        self.resources.application_resource(key, theme).or_else(|| {
            let owned = self.entries.borrow();
            container.snapshot().iter().rev().filter(|style| {
                !owned.styles.iter().filter_map(Weak::upgrade).any(|old| style_ptr_eq(&old, style))
            }).find_map(|style| ferroui_base::controls::resource_preview::lookup(&**style, key, theme))
        })
    }
    /// The resource lookup order after commit, without changing any live owner.
    pub(crate) fn preview_dictionary(&self, candidate: &ResourceDictionary, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        self.resources.preview_resource(candidate, key, theme)
            .unwrap_or_else(|error| std::panic::panic_any(ReloadError::RestartRequired(error)))
    }
    pub(crate) fn preview_resource(&self, candidate: &Styles, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        self.preview_dictionary(&candidate.resources(), key, theme)
            .or_else(|| {
                let (container, index, old) = self.current().unwrap_or_else(|error| std::panic::panic_any(error));
                let current = container.snapshot();
                let new = candidate.snapshot();
                current[index + old.len()..].iter().rev()
                    .chain(new.iter().rev())
                    .chain(current[..index].iter().rev())
                    .find_map(|style| ferroui_base::controls::resource_preview::lookup(&**style, key, theme))
            })
    }
    pub(crate) fn prepare(&self, candidate: &Ref<StyledElement>) -> Result<Box<dyn PreparedReload>, ReloadError> {
        let (container, index, old) = self.current()?;
        let candidates = candidate.styles();
        let new = candidates.snapshot().to_vec();
        let resources = self.resources.prepare_with_preview(&candidates.resources()).map_err(ReloadError::RestartRequired)?;
        // Remove ownership without attaching any candidate style to the live root.
        candidates.clear();
        candidates.set_resources(ResourceDictionary::new());
        Ok(Box::new(Change {
            owner: self.clone(), original: container.snapshot().to_vec(),
            container, index, old, new, resources, started: false,
        }))
    }
}
struct Change {
    owner: OwnedStyles,
    container: Ref<Styles>,
    original: Vec<Rc<dyn IStyle>>,
    index: usize,
    old: Vec<Rc<dyn IStyle>>,
    new: Vec<Rc<dyn IStyle>>,
    resources: ferroui_base::controls::ResourceDictionaryChange,
    started: bool,
}
impl PreparedReload for Change {
    fn commit(&mut self) -> Result<(), ReloadError> {
        self.owner.current()?;
        if *self.container.snapshot() != self.original {
            return Err(ReloadError::RestartRequired("Inline styles changed after preparation".into()));
        }
        self.resources.commit().map_err(ReloadError::RestartRequired)?;
        self.started = true;
        self.container.remove_range(self.index, self.old.len());
        self.container.insert_range(self.index, self.new.clone());
        Ok(())
    }
    fn rollback(&mut self) {
        if self.started {
            for style in &self.new {
                self.container.remove(style.clone());
            }
            for (offset, style) in self.old.iter().enumerate() {
                if !self.container.contains(style.clone()) {
                    self.container.insert(self.index + offset, style.clone());
                }
            }
            self.started = false;
        }
        self.resources.rollback();
    }
    fn finish(&mut self) {
        self.resources.finish();
        *self.owner.entries.borrow_mut() = Entries {
            index: self.index, styles: self.new.iter().map(Rc::downgrade).collect(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ComponentReloadTarget, HotReloadSession, ReloadStatus, ReloadTarget};
    use ferroui_base::threading::Dispatcher;
    use ferroui_controls::{testing::{TestServices, UnitTestApplication}, Border, ContentControl, Control, Window};
    use ferroui_markup_xaml::{FromXamlObject, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument};
    fn load(source: &str) -> ferroui_base::BoxedValue {
        ferroui_controls::register_types();
        ferroui_markup_xaml::register_types();
        ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
            RuntimeXamlLoaderDocument::new(source), None,
        ).unwrap()
    }
    fn source(width: u32) -> String {
        format!(r#"<ContentControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
<ContentControl.Styles><Styles>
  <Styles.Resources><x:Double x:Key="width">{width}</x:Double></Styles.Resources>
  <Style Selector="ContentControl">
    <Setter Property="Template"><ControlTemplate><Border Width="{{StaticResource width}}"/></ControlTemplate></Setter>
  </Style>
</Styles></ContentControl.Styles><Border/></ContentControl>"#)
    }
    #[test]
    fn inline_styles_preserve_application_entries_and_restore_templates_on_failure() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let baseline = source(10);
        let root = Ref::<ContentControl>::from_xaml_object(&load(&baseline)).unwrap();
        let model: ferroui_base::BoxedValue = Rc::new("stable model".to_owned());
        root.set_data_context(Some(model.clone()));
        let window = Window::new();
        window.set_content(Some(Control::boxed(&root.clone().upcast::<Control>())));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let uri = "ferres://inline-styles/View.xaml";
        let target: Rc<dyn ReloadTarget> = Rc::new(ComponentReloadTarget::content_control(
            &root, uri.into(), baseline.clone(), RuntimeXamlLoaderConfiguration::default(),
        ).with_owned_styles());
        let container = root.styles();
        let resources = container.resources();
        resources.add_value("application", 17.0f64);
        let application = Rc::<dyn IStyle>::from_xaml_object(&load(
            r#"<Style xmlns="https://github.com/ferroui" Selector="Border"><Setter Property="Height" Value="44"/></Style>"#,
        )).unwrap();
        container.add(application.clone());
        let mut session = HotReloadSession::new();
        session.register_document(uri.into(), baseline).unwrap();
        session.register_target(&target);
        for width in [20, 30, 40] {
            let previous = root.visual_children().get(0);
            let old_style = container.get(0);
            let weak_style = Rc::downgrade(&old_style);
            let weak_template = previous.downgrade();
            session.submit(uri.into(), source(width));
            let committed = std::cell::Cell::new(false);
            let failed = session.apply_pending_with(|| {
                committed.set(true);
                Err(ReloadError::Failed("rollback".into()))
            }).unwrap();
            assert!(committed.get(), "{failed:?}");
            assert_eq!(failed.status, ReloadStatus::Failed, "{failed:?}");
            Dispatcher::ui_thread().run_jobs(None);
            assert!(root.visual_children().get(0).ptr_eq(&previous));
            assert!(style_ptr_eq(&container.get(0), &old_style));
            session.submit(uri.into(), source(width));
            let result = session.apply_pending().unwrap();
            assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
            Dispatcher::ui_thread().run_jobs(None);
            let border = root.visual_children().get(0).cast::<Border>().unwrap();
            assert_eq!(border.width(), width as f64);
            assert_eq!(border.height(), 44.0);
            assert!(root.styles().ptr_eq(&container));
            assert!(container.resources().ptr_eq(&resources));
            assert!(style_ptr_eq(&container.get(1), &application));
            assert!(Rc::ptr_eq(&model, &root.data_context().unwrap()));
            drop(previous);
            drop(old_style);
            assert!(weak_template.upgrade().is_none());
            assert!(weak_style.upgrade().is_none());
        }
        let previous = root.visual_children().get(0);
        session.submit(uri.into(), source(50).replace("<x:Double x:Key=\"width\">50</x:Double>", ""));
        let failed = session.apply_pending().unwrap();
        assert_eq!(failed.status, ReloadStatus::Failed, "{failed:?}");
        assert!(root.visual_children().get(0).ptr_eq(&previous));
        let removed = r#"<ContentControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><Border/></ContentControl>"#;
        session.submit(uri.into(), removed.into());
        let result = session.apply_pending().unwrap();
        assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
        assert_eq!(container.count(), 1);
        assert!(style_ptr_eq(&container.get(0), &application));
        assert!(container.try_get_resource(&"width".into(), None).is_none());
        assert!(resources.contains_key(&"application".into()));
        window.close();
    }
    #[test]
    fn candidate_static_resources_use_application_style_order_and_root_precedence() {
        let _scope = Dispatcher::unit_test_scope();
        let source = |width| format!(r#"<ContentControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><ContentControl.Styles><Styles><Styles.Resources><x:Double x:Key="width">{width}</x:Double></Styles.Resources></Styles></ContentControl.Styles><Border Width="{{StaticResource width}}"/></ContentControl>"#);
        let baseline = source(10);
        let root = Ref::<ContentControl>::from_xaml_object(&load(&baseline)).unwrap();
        let uri = "ferres://style-order/View.xaml";
        let target: Rc<dyn ReloadTarget> = Rc::new(ComponentReloadTarget::content_control(
            &root, uri.into(), baseline.clone(), RuntimeXamlLoaderConfiguration::default(),
        ).with_owned_styles());
        let application = Rc::<dyn IStyle>::from_xaml_object(&load(
            r#"<Styles xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><Styles.Resources><x:Double x:Key="width">88</x:Double></Styles.Resources></Styles>"#,
        )).unwrap();
        root.styles().add(application.clone());
        let mut session = HotReloadSession::new();
        session.register_document(uri.into(), baseline).unwrap();
        session.register_target(&target);
        for (value, root_resource, expected) in [(20, None, 88.0), (30, Some(99.0), 99.0), (40, None, 88.0)] {
            if let Some(value) = root_resource {
                root.resources().set("width", Some(Rc::new(value)));
            } else {
                root.resources().remove(&"width".into());
            }
            session.submit(uri.into(), source(value));
            let result = session.apply_pending().unwrap();
            assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
            let child = Ref::<Border>::from_xaml_object(&root.content().unwrap()).unwrap();
            assert_eq!(child.width(), expected);
            assert!(style_ptr_eq(&root.styles().get(1), &application));
        }
    }
    #[test]
    fn panel_style_updates_preserve_application_children_and_reject_external_removal() {
        use ferroui_controls::Panel;
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let source = |height| format!(r#"<StackPanel xmlns="https://github.com/ferroui"><StackPanel.Styles><Style Selector="Border"><Setter Property="Height" Value="{height}"/></Style></StackPanel.Styles><Border/></StackPanel>"#);
        let baseline = source(10);
        let root = Ref::<Panel>::from_xaml_object(&load(&baseline)).unwrap();
        let uri = "ferres://panel-styles/View.xaml";
        let target: Rc<dyn ReloadTarget> = Rc::new(crate::PanelReloadTarget::new(
            &root, 0..1, uri.into(), baseline.clone(), RuntimeXamlLoaderConfiguration::default(),
        ).unwrap().with_owned_styles());
        let sibling = Border::new();
        root.children().add(sibling.clone().upcast::<Control>());
        let mut session = HotReloadSession::new();
        session.register_document(uri.into(), baseline).unwrap();
        session.register_target(&target);
        for height in [20, 30] {
            session.submit(uri.into(), source(height));
            let result = session.apply_pending().unwrap();
            assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
            root.children().get(0).apply_styling();
            sibling.apply_styling();
            assert_eq!(root.children().get(0).height(), height as f64);
            assert_eq!(sibling.height(), height as f64);
            assert!(root.children().get(1).ptr_eq(&sibling));
        }
        let child = root.children().get(0);
        root.styles().clear();
        session.submit(uri.into(), source(40));
        let result = session.apply_pending().unwrap();
        assert_eq!(result.status, ReloadStatus::RestartRequired, "{result:?}");
        assert!(root.children().get(0).ptr_eq(&child));
    }
    #[test]
    fn style_admission_rejects_source_writes_before_touching_the_model() {
        let _scope = Dispatcher::unit_test_scope();
        let baseline = r#"<ContentControl xmlns="https://github.com/ferroui"><ContentControl.Styles><Style Selector="ContentControl"><Setter Property="Width" Value="10"/></Style></ContentControl.Styles><Border/></ContentControl>"#;
        let root = Ref::<ContentControl>::from_xaml_object(&load(baseline)).unwrap();
        let model = Border::new();
        model.set_height(17.0);
        root.resources().add("model", Some(Rc::new(model.clone())));
        let uri = "ferres://style-binding/View.xaml";
        let target: Rc<dyn ReloadTarget> = Rc::new(ComponentReloadTarget::content_control(
            &root, uri.into(), baseline.into(), RuntimeXamlLoaderConfiguration::default(),
        ).with_owned_styles());
        let original = root.content().unwrap();
        let property = ferroui_base::layout::Layoutable::height_property();
        let version = model.local_assignment_version(property);
        let mut session = HotReloadSession::new();
        session.register_document(uri.into(), baseline.into()).unwrap();
        session.register_target(&target);
        let source = baseline.replace(r#"Value="10""#, r#"Value="{Binding Height, Source={StaticResource model}, Mode=OneWayToSource}""#);
        session.submit(uri.into(), source.clone());
        let result = session.apply_pending().unwrap();
        assert_eq!(result.status, ReloadStatus::RestartRequired, "{result:?}");
        assert_eq!(model.height(), 17.0);
        assert_eq!(model.local_assignment_version(property), version);
        assert!(Rc::ptr_eq(&original, &root.content().unwrap()));
        session.submit(uri.into(), source.replace("OneWayToSource", "TwoWay"));
        let result = session.apply_pending().unwrap();
        assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
        root.apply_styling();
        assert_eq!(root.width(), 17.0);
        assert_eq!(model.height(), 17.0);
        assert_eq!(model.local_assignment_version(property), version);
        model.set_width(23.0);
        let width_property = ferroui_base::layout::Layoutable::width_property();
        let width_version = model.local_assignment_version(width_property);
        let next = source.replace("OneWayToSource", "TwoWay").replace("Binding Height", "Binding Width");
        session.submit(uri.into(), next.clone());
        let result = session.apply_pending_with(|| {
            root.apply_styling();
            assert_eq!(root.width(), 23.0);
            Err(ReloadError::Failed("rollback".into()))
        }).unwrap();
        assert_eq!(result.status, ReloadStatus::Failed, "{result:?}");
        root.apply_styling();
        assert_eq!(root.width(), 17.0);
        assert_eq!(model.local_assignment_version(property), version);
        assert_eq!(model.local_assignment_version(width_property), width_version);
        session.submit(uri.into(), next);
        let result = session.apply_pending().unwrap();
        assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
        root.apply_styling();
        assert_eq!(root.width(), 23.0);
        assert_eq!(model.local_assignment_version(property), version);
        assert_eq!(model.local_assignment_version(width_property), width_version);
        model.set_height(27.0);
        assert_eq!(root.width(), 23.0, "Retired style binding remained subscribed");
        session.submit(uri.into(), baseline.replace(r#"Property="Width" Value="10""#, r#"Property="DataContext" Value="changed""#));
        let result = session.apply_pending().unwrap();
        assert_eq!(result.status, ReloadStatus::RestartRequired, "{result:?}");
    }
    #[test]
    fn theme_selected_by_inline_style_is_validated_before_commit() {
        let _app = UnitTestApplication::start(TestServices::styled_window());
        let source = |key: &str| format!(r#"<ContentControl xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
<ContentControl.Styles><Styles><Styles.Resources>
<x:Double x:Key="width">41</x:Double>
<ControlTheme x:Key="theme" TargetType="ContentControl"><Setter Property="Template"><ControlTemplate><Border Width="{{StaticResource {key}}}"/></ControlTemplate></Setter></ControlTheme>
</Styles.Resources><Style Selector="ContentControl"><Setter Property="Theme" Value="{{StaticResource theme}}"/></Style></Styles></ContentControl.Styles><Border/></ContentControl>"#);
        let baseline = source("width");
        let root = Ref::<ContentControl>::from_xaml_object(&load(&baseline)).unwrap();
        let window = Window::new();
        window.set_content(Some(Control::boxed(&root.clone().upcast::<Control>())));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let original = root.visual_children().get(0);
        let uri = "ferres://inline-style-theme/View.xaml";
        let target: Rc<dyn ReloadTarget> = Rc::new(ComponentReloadTarget::content_control(
            &root, uri.into(), baseline.clone(), RuntimeXamlLoaderConfiguration::default(),
        ).with_owned_styles());
        let mut session = HotReloadSession::new();
        session.register_document(uri.into(), baseline).unwrap();
        session.register_target(&target);
        session.submit(uri.into(), source("missing"));
        let committed = std::cell::Cell::new(false);
        let result = session.apply_pending_with(|| { committed.set(true); Ok(()) }).unwrap();
        assert_eq!(result.status, ReloadStatus::Failed, "{result:?}");
        assert!(!committed.get());
        assert!(root.visual_children().get(0).ptr_eq(&original));
        session.submit(uri.into(), source("width").replace(">41<", ">42<"));
        let result = session.apply_pending().unwrap();
        assert_eq!(result.status, ReloadStatus::Applied, "{result:?}");
        Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(root.visual_children().get(0).cast::<Border>().unwrap().width(), 42.0);
        window.close();
    }
}
