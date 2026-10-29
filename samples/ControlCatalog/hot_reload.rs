//! Manifest-backed catalog sessions. Cached-reference/handler pages require explicit refresh adapters.
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{FerroLocator, Ref, TypeInfo, WeakRef};
use ferroui_controls::ContentPage;
use ferroui_hot_reload::{
    ComponentReloadTarget, FileWatcher, HotReloadSession, ReloadManifest, ReloadTarget,
};
use ferroui_markup_xaml::RuntimeXamlLoaderConfiguration;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    rc::{Rc, Weak},
    time::Duration,
};

struct CatalogSession {
    session: HotReloadSession,
    watcher: FileWatcher,
    targets: Vec<(Rc<ComponentReloadTarget>, WeakRef<ContentPage>)>,
    timer: Option<Rc<DispatcherTimer>>,
    bound: BTreeSet<String>,
}
ferroui_base::ferro_thread_local! {
    static SESSION: RefCell<Weak<RefCell<CatalogSession>>> = const { RefCell::new(Weak::new()) };
    // Set when the session cannot start; the catalog then runs without reload.
    static DISABLED: Cell<bool> = const { Cell::new(false) };
}
impl Drop for CatalogSession {
    fn drop(&mut self) {
        if let Some(timer) = self.timer.take() {
            timer.stop();
        }
    }
}

/// Join the reader before Windows enters TLS destruction under the loader lock.
pub(crate) fn shutdown() {
    let context = SESSION.with(|slot| slot.replace(Weak::new()).upgrade());
    FerroLocator::current_mutable()
        .bind::<RefCell<CatalogSession>>()
        .to_func(|| None);
    drop(context);
}

#[cfg(test)]
pub(crate) fn session_release_check() -> impl Fn() -> bool {
    let session = SESSION.with(|slot| slot.borrow().clone());
    assert!(session.upgrade().is_some(), "a catalog watcher must be running");
    move || session.upgrade().is_none()
}

fn configuration() -> RuntimeXamlLoaderConfiguration {
    let mut configuration = RuntimeXamlLoaderConfiguration::default();
    configuration.local_assembly = Some(&crate::register_types::ASSEMBLY);
    configuration.use_compiled_bindings_by_default = true;
    configuration
}
pub(crate) fn track(root: Ref<ContentPage>, class: &'static TypeInfo, path: &str) {
    if ![
        "/Pages/AdornerLayerPage.xaml",
        "/Pages/BorderPage.xaml",
        "/Pages/SliderPage.xaml",
        "/Pages/ProgressBarPage.xaml",
        "/Pages/CheckBoxPage.xaml",
        "/Pages/CanvasPage.xaml",
        "/Pages/RadioButtonPage.xaml",
        "/Pages/TextBlockPage.xaml",
        "/Pages/ViewboxPage.xaml",
        "/Pages/ExpanderPage.xaml",
        "/Pages/ListBoxPage.xaml",
    ]
    .contains(&path)
    {
        return;
    }
    if DISABLED.with(Cell::get) {
        return;
    }
    SESSION.with(|slot| {
        let mut slot = slot.borrow_mut();
        let uri = crate::markup::document_uri(path);
        let Some(context) = slot
            .upgrade()
            .or_else(|| {
                let mut manifest =
                    ReloadManifest::parse(include_str!(env!("FERROUI_HOT_RELOAD_MANIFEST")))
                        .expect("catalog development manifest");
                let overlay_root = std::env::var_os("FERROUI_HOT_RELOAD_ROOT");
                if let Some(directory) = &overlay_root {
                    if let Err(message) = overlay_sources(
                        &mut manifest.files,
                        Path::new(env!("CARGO_MANIFEST_DIR")),
                        Path::new(directory),
                    ) {
                        eprintln!("Catalog reload disabled: {message}");
                        DISABLED.with(|disabled| disabled.set(true));
                        return None;
                    }
                }
                let watcher = if overlay_root.is_some() {
                    // An explicit partial source overlay must not collide with
                    // discovery of the original file under the same document URI.
                    FileWatcher::with_baseline(
                        manifest.files.clone(),
                        Duration::from_millis(60),
                        manifest.sources.clone(),
                    )
                } else {
                    manifest.watcher(Duration::from_millis(60))
                };
                for source in manifest.sources.values_mut() {
                    *source = normalize(source.clone()).expect("valid embedded geometry");
                }
                let mut session = manifest
                    .session()
                    .expect("register catalog reload documents");
                if let Some(application) = ferroui_controls::Application::current() {
                    session.track_resource_includes(
                        &application.resources(),
                        configuration(),
                    );
                    session.track_style_includes(&application.styles(), configuration());
                }
                let context = Rc::new(RefCell::new(CatalogSession {
                    session,
                    watcher,
                    targets: Vec::new(),
                    timer: None,
                    bound: BTreeSet::new(),
                }));
                let weak = Rc::downgrade(&context);
                let timer = DispatcherTimer::with_callback(
                    Duration::from_millis(60),
                    DispatcherPriority::BACKGROUND,
                    move |timer| {
                        let Some(context) = weak.upgrade() else {
                            timer.stop();
                            return;
                        };
                        let mut context = context.borrow_mut();
                        context.targets.retain(|(target, _)| target.is_alive());
                        let latest = context.watcher.changes.try_iter().last();
                        if let Some(mut batch) = latest {
                            for file in &mut batch.files {
                                file.contents =
                                    std::mem::replace(&mut file.contents, Err(String::new()))
                                        .and_then(normalize);
                            }
                            context.session.submit_files(batch);
                        }
                        if let Some(outcome) = context.session.apply_pending() {
                            if std::env::var("FERROUI_RELOAD_TARGET_STATE").as_deref() == Ok("1") {
                                use ferroui_markup_xaml::FromXamlObject;
                                for (_, root) in &context.targets {
                                    let Some(root) = root.upgrade() else { continue };
                                    let Some(content) = root.content().as_ref().and_then(Ref::<ferroui_controls::Control>::from_xaml_object) else { continue };
                                    let theme = root.theme().as_ref().map(|theme| &**theme as *const _ as *const ()).unwrap_or(std::ptr::null());
                                    let width = content.width();
                                    let width = if width.is_finite() { width.to_string() } else { "null".into() };
                                    let root_width = root.width();
                                    let root_width = if root_width.is_finite() { root_width.to_string() } else { "null".into() };
                                    let root_height = root.height();
                                    let root_height = if root_height.is_finite() { root_height.to_string() } else { "null".into() };
                                    let model = root.data_context().as_ref().map(|value| Rc::as_ptr(value) as *const ()).unwrap_or(std::ptr::null());
                                    let window = ferroui_controls::TopLevel::get_top_level(Some(&root));
                                    let window = window.as_ref().map(|window| &**window as *const _ as *const ()).unwrap_or(std::ptr::null());
                                    eprintln!("FERROUI_RELOAD_TARGET_STATE={{\"revision\":{},\"status\":\"{:?}\",\"root\":\"{:p}\",\"window\":\"{:p}\",\"model\":\"{:p}\",\"content\":\"{:p}\",\"content_width\":{},\"root_width\":{},\"root_height\":{},\"theme\":\"{:p}\"}}", outcome.revision, outcome.status, &*root, window, model, &*content, width, root_width, root_height, theme);
                                }
                            }

                            if let Some(top_level) = context.targets.iter().find_map(|(_, root)| {
                                root.upgrade().and_then(|root| {
                                    ferroui_controls::TopLevel::get_top_level(Some(&root))
                                })
                            }) {
                                ferroui_hot_reload::record_frame_timing(&top_level, &outcome);
                            }
                            eprintln!("Catalog reload: {outcome:?}");
                        }
                    },
                );
                context.borrow_mut().timer = Some(timer);
                // The application/test service scope ends before thread teardown.
                // Joining a watcher from a Windows TLS destructor deadlocks with
                // the worker's own loader-lock-protected thread exit.
                FerroLocator::current_mutable().bind_to_self(context.clone());
                *slot = Rc::downgrade(&context);
                Some(context)
            })
        else {
            return;
        };
        let mut context = context.borrow_mut();
        let canonical = ferroui_base::utilities::Uri::try_create(
            &uri,
            ferroui_base::utilities::UriKind::Absolute,
        )
        .unwrap();
        if context.bound.insert(canonical.absolute_uri().to_owned()) {
            context
                .session
                .bind_class(class, uri.clone(), configuration())
                .expect("bind catalog class");
        }
        let baseline = context.session.sources()[canonical.absolute_uri()].clone();
        let target = component_target(&root, uri, baseline, Vec::new());
        let target = Rc::new(target);
        let erased: Rc<dyn ReloadTarget> = target.clone();
        context.session.register_target(&erased);
        context.targets.push((target, root.downgrade()));
        eprintln!("Catalog reload tracking: {path}");
    });
}

/// Points the sources of the manifest that `directory` (`FERROUI_HOT_RELOAD_ROOT`) also holds at
/// the copies there. The sources are paths under `original_root`, the source directory of the
/// build; an error says why the overlay cannot be used, for example a binary that runs where that
/// directory does not exist.
pub(crate) fn overlay_sources(
    files: &mut BTreeMap<String, PathBuf>,
    original_root: &Path,
    directory: &Path,
) -> Result<(), String> {
    let original_root = original_root.canonicalize().map_err(|error| {
        format!(
            "the source directory of the build, {}, is not available ({error})",
            original_root.display()
        )
    })?;
    let mut overlaid = files.clone();
    for source in overlaid.values_mut() {
        let relative = source.strip_prefix(&original_root).map_err(|_| {
            format!(
                "the source {} is not under the source directory of the build, {}",
                source.display(),
                original_root.display()
            )
        })?;
        let overlay = directory.join(relative);
        if overlay.exists() {
            *source = overlay;
        }
    }
    *files = overlaid;
    Ok(())
}

pub(crate) fn component_target(
    root: &Ref<ContentPage>,
    uri: String,
    baseline: String,
    dependencies: Vec<String>,
) -> ComponentReloadTarget {
    let mut target = ComponentReloadTarget::content_page(root, uri, baseline, configuration())
        .with_dependencies(dependencies)
        .with_owned_resources()
        .with_owned_styles()
        .with_owned_handlers()
        .with_owned_root_static_resource_property(ferroui_base::StyledElement::theme_property())
        .with_owned_root_binding_property(ferroui_base::layout::Layoutable::width_property())
        .with_owned_root_binding_property(ferroui_base::layout::Layoutable::height_property())
        .with_owned_root_binding_property(ferroui_base::layout::Layoutable::margin_property());
    if let Some(page) = root.cast::<crate::pages::ExpanderPage>() {
        target = target.with_refresh(page.reload_refresh());
    }
    if let Some(page) = root.cast::<crate::pages::AdornerLayerPage>() {
        target = target.with_refresh(page.reload_refresh());
    }
    if root.cast::<crate::pages::CheckBoxPage>().is_some() {
        target = target.with_state_adapter(Rc::new(CheckBoxStateAdapter));
    }
    target
}

// Application opt-in: built-in state transfer does not assume ownership of every
// control property. The named unbound check box demonstrates a custom snapshot.
struct CheckBoxStateAdapter;
struct CheckBoxState(Option<bool>);
impl ferroui_hot_reload::ControlStateAdapter for CheckBoxStateAdapter {
    fn capture(
        &self,
        control: &Ref<ferroui_base::StyledElement>,
    ) -> Option<Box<dyn ferroui_hot_reload::CapturedControlState>> {
        Some(Box::new(CheckBoxState(control.cast::<ferroui_controls::CheckBox>()?.is_checked())))
    }
}
impl ferroui_hot_reload::CapturedControlState for CheckBoxState {
    fn restore(
        &self,
        control: &Ref<ferroui_base::StyledElement>,
        context: &ferroui_hot_reload::ControlStateRestoreContext<'_>,
    ) {
        if context.may_restore_property(ferroui_controls::primitives::ToggleButton::is_checked_property()) {
            if let Some(control) = control.cast::<ferroui_controls::CheckBox>() {
                control.set_is_checked(self.0);
            }
        }
    }
}

#[path = "build/placeholder_geometry.rs"]
mod placeholder_geometry;
include!(concat!(env!("OUT_DIR"), "/hot_reload_geometries.rs"));
fn normalize(mut source: String) -> Result<String, String> {
    if cfg!(feature = "placeholder-branding") {
        for (key, data) in RELOAD_GEOMETRIES {
            placeholder_geometry::replace_geometry(&mut source, key, data)?;
        }
    }
    Ok(source)
}
