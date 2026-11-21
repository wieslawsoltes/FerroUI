// Copyright (c) 2026 Wiesław Šoltés. MIT. Original ShellPage native view adapter.
use crate::{
    demo_settings::{DemoSettings, DemoSubscription},
    pages::*,
    shell_view_model::{ShellError, ShellSurface, ShellViewModel},
};
use ferroribbon::{
    controls::{Ribbon, RibbonToggleButton, RibbonToolBar},
    theming::RibbonTheme,
};
use ferroribbon_core::theming::{RibbonChromeStyle, RibbonThemePalette, RibbonThemeStyle};
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, Visual, VisualImpl, WeakRef, ferro_class,
    ferro_class_info, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    metadata::{MarkupDelegate, from_markup_value},
    reactive::Subscription,
    threading::{Dispatcher, DispatcherPriority},
};
use ferroui_controls::{
    ContentControlImpl, Control, ControlImpl, Grid, UserControl, primitives::TemplatedControlImpl,
};
use std::{
    cell::{Cell, OnceCell, RefCell},
    rc::Rc,
};
type CadThemeSubscription = (WeakRef<CadPage>, Subscription);

#[repr(C)]
pub struct ShellPage {
    base: UserControl,
    model: OnceCell<Rc<ShellViewModel<NativeShellSurface>>>,
    settings: OnceCell<Rc<DemoSettings>>,
    cleanup: RefCell<Vec<Box<dyn FnOnce()>>>,
    style_subscription: RefCell<Option<DemoSubscription>>,
    cad_theme_subscriptions: RefCell<Vec<CadThemeSubscription>>,
}
ferro_class!(ShellPage:UserControl);
ferro_class_info!(ShellPage {new:ShellPage::new,markup:{namespace:"FerroRibbon.Sample",properties:[CurrentPage:Option<String>{get:ShellPage::current_page},CurrentContent:Option<Ref<Control>>{get:|p:&ShellPage|p.current_content().expect("Shell CurrentContent") }],methods:[try fn Show(Option<String>)=>|p:&ShellPage,key:Option<String>|p.show_optional(key.as_deref())]}});
ferro_impl_classes!(ShellPage:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
impl ShellPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            model: OnceCell::new(),
            settings: OnceCell::new(),
            cleanup: RefCell::new(Vec::new()),
            style_subscription: RefCell::new(None),
            cad_theme_subscriptions: RefCell::new(Vec::new()),
        }
    }
    pub fn new() -> Ref<Self> {
        Self::new_with_settings(DemoSettings::current())
    }
    pub fn new_with_settings(settings: Rc<DemoSettings>) -> Ref<Self> {
        let this = instantiate(Self::construct());
        if let Some(populate) = ferroui_markup_xaml::FerroXamlLoader::populate_override(Self::TYPE)
        {
            populate(None, &Control::boxed(this.clone())).expect("Shell populate override");
        } else {
            crate::generated::compiled_shell_page::populate(None, &this)
                .expect("compiled original ShellPage");
        }
        assert!(this.settings.set(settings.clone()).is_ok());
        assert!(
            this.model
                .set(ShellViewModel::new(NativeShellSurface {
                    page: this.downgrade(),
                    settings: settings.clone()
                }))
                .is_ok()
        );
        this.named::<RibbonToggleButton>("CadRail")
            .set_icon(crate::cad::CadIcon::get("AppLogo").map(|i| i as ferroui_base::BoxedValue));
        for toggle in this
            .rail()
            .items()
            .iter()
            .flatten()
            .filter_map(|item| item.cast::<RibbonToggleButton>())
        {
            let weak = this.downgrade();
            let checked = MarkupDelegate::new(move |args| {
                if let Some(page) = weak.upgrade() {
                    let sender = from_markup_value::<Ref<RibbonToggleButton>>(&args[0])
                        .expect("source rail Checked sender");
                    let key = tag(&sender).expect("source rail Tag cast");
                    page.view_model().navigation().execute(Some(&key));
                }
                None
            });
            toggle.add_checked(Some(checked.clone()));
            let weak = toggle.downgrade();
            this.cleanup.borrow_mut().push(Box::new(move || {
                if let Some(toggle) = weak.upgrade() {
                    toggle.remove_checked(Some(checked));
                }
            }));
        }
        let weak = this.downgrade();
        this.loaded(move |_, _| {
            if let Some(page) = weak.upgrade() {
                page.show("word").expect("source Shell Loaded");
            }
        });
        let weak = this.downgrade();
        this.style_subscription
            .replace(Some(settings.surface_style_changed(move || {
                if let Some(page) = weak.upgrade() {
                    page.view_model().invalidate_office_pages();
                }
            })));
        this
    }
    pub(crate) fn settings(&self) -> Rc<DemoSettings> {
        self.settings
            .get()
            .expect("Shell settings initialized")
            .clone()
    }
    pub fn view_model(&self) -> Rc<ShellViewModel<NativeShellSurface>> {
        self.model.get().expect("Shell initialized").clone()
    }
    pub fn current_page(&self) -> Option<String> {
        self.view_model().current_page()
    }
    pub fn current_content(&self) -> Result<Option<Ref<Control>>, ShellError> {
        self.view_model().current_content()
    }
    pub fn show(&self, key: &str) -> Result<(), ShellError> {
        self.view_model().show(key)
    }
    pub fn show_optional(&self, key: Option<&str>) -> Result<(), ShellError> {
        self.view_model().show_optional(key)
    }
    pub fn rail(&self) -> Ref<RibbonToolBar> {
        self.named("Rail")
    }
    pub fn page_host(&self) -> Ref<Grid> {
        self.named("PageHost")
    }
    pub(crate) fn named<T: ferroui_base::ObjectType>(&self, name: &str) -> Ref<T> {
        ferroui_base::controls::NameScopeExtensions::find::<T>(self, name)
            .unwrap_or_else(|| panic!("Missing shell element {name}"))
    }
}
impl Drop for ShellPage {
    fn drop(&mut self) {
        for cleanup in self.cleanup.get_mut().drain(..) {
            cleanup();
        }
        self.style_subscription.get_mut().take();
        for (_, subscription) in self.cad_theme_subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}
fn tag(toggle: &RibbonToggleButton) -> Result<Option<String>, ShellError> {
    toggle
        .tag()
        .map(|value| {
            if let Some(text) = value.downcast_ref::<String>() {
                Ok(text.clone())
            } else if let Some(text) = value.downcast_ref::<ferroribbon_core::RibbonString>() {
                text.to_utf8()
                    .map_err(|e| ShellError::Native(format!("Rail UTF16 boundary: {e:?}")))
            } else {
                Err(ShellError::InvalidTag)
            }
        })
        .transpose()
}
pub struct NativeShellSurface {
    page: WeakRef<ShellPage>,
    settings: Rc<DemoSettings>,
}
impl NativeShellSurface {
    fn page(&self) -> Ref<ShellPage> {
        self.page
            .upgrade()
            .expect("Shell native surface owner is gone")
    }
}
// Source traversal is depth-first and reads child count again after each child.
// Do not replace it with a flattened snapshot or a named-page Ribbon shortcut.
pub(crate) fn find_ribbon(root: &Visual) -> Option<Ref<Ribbon>> {
    if let Some(ribbon) = root.to_ref().cast::<Ribbon>() {
        return Some(ribbon);
    }
    let mut index = 0;
    while index < root.visual_children().count() {
        if let Some(ribbon) = find_ribbon(&root.visual_children().get(index)) {
            return Some(ribbon);
        }
        index += 1;
    }
    None
}
impl ShellSurface for NativeShellSurface {
    type Page = Ref<Control>;
    fn surface_style(&self) -> RibbonThemeStyle {
        self.settings.surface_style()
    }
    fn theme_style(&self) -> RibbonThemeStyle {
        RibbonTheme::style()
    }
    fn chrome(&self) -> RibbonChromeStyle {
        RibbonTheme::chrome_style()
    }
    fn palette(&self) -> Rc<RibbonThemePalette> {
        RibbonTheme::palette()
    }
    fn palette_override(&self) -> Option<Rc<RibbonThemePalette>> {
        self.settings.palette_override()
    }
    fn apply_theme(
        &self,
        palette: Rc<RibbonThemePalette>,
        chrome: RibbonChromeStyle,
        style: RibbonThemeStyle,
    ) -> Result<(), ShellError> {
        RibbonTheme::apply_with_style(Some(palette), chrome, style)
            .map_err(|e| ShellError::Native(e.to_string()))
    }
    fn apply_palette(&self, palette: Rc<RibbonThemePalette>) -> Result<(), ShellError> {
        RibbonTheme::apply_palette(Some(palette)).map_err(|e| ShellError::Native(e.to_string()))
    }
    fn create_page(&self, key: &str) -> Result<Self::Page, ShellError> {
        let settings = self.settings.clone();
        Ok(match key {
            "word" => WordPage::new_with_settings(settings).upcast(),
            "excel" => ExcelPage::new_with_settings(settings).upcast(),
            "powerpoint" => PowerPointPage::new_with_settings(settings).upcast(),
            "tools" => ToolsPage::new_with_settings(settings).upcast(),
            "cad" => CadPage::new_with_settings(settings).upcast(),
            "settings" => SettingsPage::new_with_settings(settings).upcast(),
            _ => return Err(ShellError::UnknownPage(key.to_owned())),
        })
    }
    fn add_page(&self, page: &Self::Page) {
        self.page().page_host().children().add(page.clone());
    }
    fn remove_page(&self, page: &Self::Page) {
        self.page().page_host().children().remove(page.clone());
    }
    fn show_only(&self, page: &Self::Page) -> Result<(), ShellError> {
        // Pinned Uno uses List<View>.Enumerator. Native FerroList deliberately
        // snapshots instead, so observe mutation for this source traversal only.
        let children = self.page().page_host().children();
        let changed = Rc::new(Cell::new(false));
        let flag = changed.clone();
        let token = children.add_collection_changed(Rc::new(move |_| flag.set(true)));
        let _subscription = ChildTraversalSubscription {
            children: children.clone(),
            token,
        };
        let mut index = 0;
        loop {
            if changed.get() {
                return Err(ShellError::ChildrenChanged);
            }
            if index == children.count() {
                break;
            }
            let child = children.get(index);
            child.set_is_visible(child.ptr_eq(page));
            index += 1;
        }
        Ok(())
    }
    fn check_rail(&self, key: Option<&str>) -> Result<(), ShellError> {
        for toggle in self
            .page()
            .rail()
            .items()
            .iter()
            .flatten()
            .filter_map(|item| item.cast::<RibbonToggleButton>())
        {
            toggle.set_is_checked(Some(tag(&toggle)?.as_deref() == key));
        }
        Ok(())
    }
    fn suspend(&self, page: &Self::Page) {
        if let Some(ribbon) = find_ribbon(page) {
            ribbon.suspend_popups();
        }
    }
    fn resume(&self, page: &Self::Page) {
        if let Some(ribbon) = find_ribbon(page) {
            ribbon.resume_popups();
        }
    }
    fn is_cad(&self, page: &Self::Page) -> bool {
        page.cast::<CadPage>().is_some()
    }
    fn follow_cad_theme(&self, page: &Self::Page) {
        let shell = self.page();
        let cad = page.cast::<CadPage>().expect("CAD surface");
        shell.rail().set_value(
            ferroui_base::styling::ThemeVariant::requested_theme_variant_property(),
            cad.actual_theme_variant(),
        );
        let previous = {
            let mut subscriptions = shell.cad_theme_subscriptions.borrow_mut();
            subscriptions
                .iter()
                .position(|(p, _)| p.upgrade().is_some_and(|p| p == cad))
                .map(|i| subscriptions.remove(i))
        };
        if let Some((_, subscription)) = previous {
            subscription.dispose();
        }
        let weak = shell.downgrade();
        let weak_cad = cad.downgrade();
        let subscription = cad.actual_theme_variant_changed(move || {
            let weak = weak.clone();
            let weak_cad = weak_cad.clone();
            Dispatcher::ui_thread().post_local(
                move || {
                    if let (Some(shell), Some(cad)) = (weak.upgrade(), weak_cad.upgrade())
                        && shell.current_page().as_deref() == Some("cad")
                    {
                        shell.rail().set_value(
                            ferroui_base::styling::ThemeVariant::requested_theme_variant_property(),
                            cad.actual_theme_variant(),
                        );
                    }
                },
                DispatcherPriority::NORMAL,
            );
        });
        shell
            .cad_theme_subscriptions
            .borrow_mut()
            .push((cad.downgrade(), subscription));
    }
    fn reset_rail_theme(&self) {
        self.page().rail().set_value(
            ferroui_base::styling::ThemeVariant::requested_theme_variant_property(),
            None,
        );
    }
}

struct ChildTraversalSubscription {
    children: ferroui_controls::Controls,
    token: u64,
}
impl Drop for ChildTraversalSubscription {
    fn drop(&mut self) {
        self.children.remove_collection_changed(self.token);
    }
}
