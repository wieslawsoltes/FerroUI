// Copyright (c) 2026 Wiesław Šoltés. MIT. Native CadPage control.
use crate::cad::{
    CadColorOption, CadLayer, CadLinetype, CadLineweight,
    cad_view_model::{CadContextAction, CadInvocation, CadKey, CadLog, CadViewModel},
};
use ferroribbon::controls::{
    Ribbon, RibbonApplicationMenu, RibbonApplicationMenuItem, RibbonComboBox, RibbonGroup,
    RibbonStatusBar, RibbonTitleBar,
};
use ferroui_base::{
    FerroLocator, FerroObjectImpl, LocatorExtensions, Ref, StyledElementImpl, VisualImpl,
    ferro_class, ferro_class_info, ferro_impl_classes, input::InputElementImpl, instantiate,
    interactivity::InteractiveImpl, layout::LayoutableImpl, platform::IPlatformSettings,
    styling::ThemeVariant,
};
use ferroui_base::{
    input::Key,
    metadata::{MarkupDelegate, from_markup_value},
};
use ferroui_controls::{
    Border, ContentControlImpl, Control, ControlImpl, TextBox, UserControl,
    primitives::TemplatedControlImpl,
};
use std::{
    cell::{Cell, OnceCell, RefCell},
    rc::Rc,
};
#[repr(C)]
pub struct CadPage {
    base: UserControl,
    model: OnceCell<Rc<CadViewModel>>,
    page_theme: RefCell<Option<ThemeVariant>>,
    pub(super) menu_icons_dark: Rc<Cell<bool>>,
    pub(super) subscriptions: RefCell<Vec<Rc<dyn ferroui_base::reactive::IDisposable>>>,
    pub(super) initialized: Cell<bool>,
    pub(super) document_tabs:
        OnceCell<Vec<(Ref<Border>, crate::cad::cad_view_model::CadDocumentTab)>>,
    pub(super) tip_popup: RefCell<Option<Ref<ferroui_controls::primitives::Popup>>>,
}
ferro_class!(CadPage:UserControl);
ferro_class_info!(CadPage {new:CadPage::new,markup:{namespace:"FerroRibbon.Sample.Pages",properties:[
CadRibbon:Ref<Ribbon>{get:CadPage::cad_ribbon},DrawPanel:Ref<RibbonGroup>{get:CadPage::draw_panel},LayersPanel:Ref<RibbonGroup>{get:CadPage::layers_panel},LayerDropDown:Ref<RibbonComboBox>{get:CadPage::layer_drop_down},ApplicationMenu:Ref<RibbonApplicationMenu>{get:CadPage::application_menu},SaveAsMenuItem:Ref<RibbonApplicationMenuItem>{get:CadPage::save_as_menu_item}
],methods:[fn RunCommand(String)=>|page:&CadPage,text:String|page.run_command(&text),fn ApplyWorkspace(Option<String>)=>CadPage::apply_workspace,fn ShowTextEditor()=>CadPage::show_text_editor,fn ShowHatchCreation()=>CadPage::show_hatch_creation,fn SetLightTheme(bool)=>CadPage::set_light_theme,fn FloatLayersPanel()=>CadPage::float_layers_panel,fn ShowApplicationMenu()=>CadPage::show_application_menu,try fn ShowToolTip(Option<String>)->bool=>|page:&CadPage,id:Option<String>|page.show_tool_tip(id.as_deref())]}});
ferro_impl_classes!(CadPage:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
impl CadPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            model: OnceCell::new(),
            page_theme: RefCell::new(None),
            menu_icons_dark: Rc::new(Cell::new(true)),
            subscriptions: RefCell::new(Vec::new()),
            initialized: Cell::new(false),
            document_tabs: OnceCell::new(),
            tip_popup: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        Self::new_with_settings(crate::demo_settings::DemoSettings::current())
    }
    pub fn new_with_settings(settings: Rc<crate::demo_settings::DemoSettings>) -> Ref<Self> {
        let this = instantiate(Self::construct());
        // Preserve WinUI's adaptive group width at fractional Windows scaling.
        #[cfg(windows)]
        this.set_use_winui_measure_rounding(true);
        let root = Control::boxed(this.clone());
        if let Some(populate) = ferroui_markup_xaml::FerroXamlLoader::populate_override(Self::TYPE)
        {
            populate(None, &root).expect("CAD page populate override");
        } else {
            crate::generated::compiled_cad_page::populate(None, &this)
                .expect("compiled original CAD page");
        }
        let ribbon = this.cad_ribbon();
        #[cfg(windows)]
        {
            use ferroui_base::media::FontFamily;
            use ferroui_controls::primitives::TemplatedControl;

            let family = TemplatedControl::font_family_property().as_property();
            if !ribbon.is_set(family) && !ribbon.has_local_value_binding(family) {
                // WinUI measures collapsed captions with the optical size for
                // their text size, which can change the selected group states.
                ribbon.set_font_family(
                    FontFamily::new("Segoe UI Variable, Segoe UI, $Default")
                        .with_automatic_optical_sizing(),
                );
            }
        }
        ferroribbon::controls::RibbonScreenTipService::set_extended_delay(
            ferroui_base::animation::SignedDuration::from_ticks(12_000_000),
        );
        this.named::<RibbonTitleBar>("TitleBar")
            .set_ribbon(Some(ribbon.clone()));
        this.install_cad_menu_converter();
        this.named::<RibbonStatusBar>("StatusBar")
            .set_ribbon(Some(ribbon.clone()));
        this.application_menu().set_ribbon(Some(ribbon));
        this.connect_model(settings);
        this.connect_page_actions();
        this.build_workspace_menu();
        this.build_minimize_behavior_menu();
        this.connect_document_tabs();
        this.connect_system_contrast();
        let weak = this.downgrade();
        this.loaded(move |_, _| {
            if let Some(page) = weak.upgrade() {
                page.on_cad_loaded();
            }
        });
        let weak = this.downgrade();
        this.actual_theme_variant_changed(move || {
            let Some(page) = weak.upgrade() else { return };
            page.apply_chrome_brushes();
            // Uno 6.7 queues ActualThemeChanged after its theme walk. FerroUI's
            // notification is synchronous; post the source page callback so the
            // canvas has updated its cached color before UpdateTabVisuals reads it.
            let weak = weak.clone();
            ferroui_base::threading::Dispatcher::ui_thread().post_local(
                move || {
                    if let Some(page) = weak.upgrade() {
                        page.apply_menu_icons()
                            .expect("CAD theme menu icon conversion");
                        page.update_document_tab_visuals();
                    }
                },
                ferroui_base::threading::DispatcherPriority::NORMAL,
            );
        })
        .detach();
        this
    }
    pub fn view_model(&self) -> Rc<CadViewModel> {
        self.model
            .get()
            .expect("CAD presentation initialized")
            .clone()
    }
    pub fn run_command(&self, text: &str) {
        self.view_model()
            .run_action()
            .execute(Some(&text.to_owned()));
    }
    pub fn apply_workspace(&self, workspace: Option<String>) {
        self.view_model().apply_workspace(workspace);
    }
    pub fn show_text_editor(&self) {
        self.view_model()
            .contextual_action()
            .execute(Some(&CadContextAction::ShowTextEditor));
    }
    pub fn show_hatch_creation(&self) {
        self.view_model()
            .contextual_action()
            .execute(Some(&CadContextAction::ShowHatchCreation));
    }
    pub fn set_light_theme(&self, light: bool) {
        self.view_model()
            .contextual_action()
            .execute(Some(&CadContextAction::SetLightTheme(light)));
    }
    pub(super) fn apply_light_theme(&self, light: bool) {
        self.page_theme.replace(Some(if light {
            ThemeVariant::light()
        } else {
            ThemeVariant::dark()
        }));
        let high_contrast = FerroLocator::current()
            .get_service::<dyn IPlatformSettings>()
            .is_some_and(|settings| settings.get_color_values().is_high_contrast());
        self.update_theme_variant(high_contrast);
        self.named::<ferroribbon::controls::RibbonToggleButton>("LightThemeToggle")
            .set_is_checked(Some(light));
    }
    fn connect_system_contrast(&self) {
        self.page_theme
            .replace(self.get_value(ThemeVariant::requested_theme_variant_property()));
        let Some(settings) = FerroLocator::current().get_service::<dyn IPlatformSettings>() else {
            return;
        };
        let previous = Cell::new(settings.get_color_values());
        if previous.get().is_high_contrast() {
            self.update_theme_variant(true);
        }
        let weak = self.to_ref().downgrade();
        let subscription = settings.color_values_changed(Rc::new(move |values| {
            let before = previous.replace(*values);
            if let Some(page) = weak.upgrade() {
                if before.is_high_contrast() != values.is_high_contrast() {
                    if values.is_high_contrast() {
                        page.page_theme.replace(
                            page.get_value(ThemeVariant::requested_theme_variant_property()),
                        );
                    }
                    page.update_theme_variant(values.is_high_contrast());
                } else if values.is_high_contrast()
                    && before.system_colors() != values.system_colors()
                {
                    page.apply_menu_icons()
                        .expect("CAD high-contrast menu icons");
                }
            }
        }));
        self.subscriptions.borrow_mut().push(subscription);
    }
    fn update_theme_variant(&self, high_contrast: bool) {
        let variant = high_contrast
            .then(ThemeVariant::high_contrast_variant)
            .flatten()
            .or_else(|| self.page_theme.borrow().clone());
        self.set_value(ThemeVariant::requested_theme_variant_property(), variant);
    }
    fn connect_model(&self, log: Rc<dyn CadLog>) {
        let model = CadViewModel::new(
            Rc::new(super::cad_session::NativeCadSession::new(&self.to_ref())),
            log,
        );
        assert!(self.model.set(model.clone()).is_ok());
        self.set_data_context(Some(model.clone()));
        let weak = Rc::downgrade(&model);
        self.cad_ribbon()
            .add_item_invoked(Some(MarkupDelegate::new(move |args| {
                let event = from_markup_value::<
                    Rc<ferroribbon::controls::RibbonItemInvokedEventArgs>,
                >(&args[1])
                .expect("native ItemInvoked args");
                if let Some(model) = weak.upgrade() {
                    let split = event
                        .item
                        .cast::<ferroribbon::controls::RibbonSplitButton>();
                    let choice = split
                        .as_ref()
                        .and_then(|s| s.last_choice())
                        .and_then(|c| c.text())
                        .or_else(|| {
                            event
                                .parameter
                                .as_ref()
                                .and_then(|p| p.downcast_ref::<String>().cloned())
                        });
                    let item = CadInvocation {
                        item_id: event
                            .item_id
                            .as_ref()
                            .map(|s| s.to_utf8().expect("CAD source IDs are valid Unicode")),
                        command_id: event
                            .command_id
                            .as_ref()
                            .map(|s| s.to_utf8().expect("CAD source IDs are valid Unicode")),
                        choice,
                        split: split.is_some(),
                        label: super::cad_session::ribbon_item(&event.item)
                            .and_then(|i| i.label())
                            .map(|s| s.to_utf8().expect("CAD source labels are valid Unicode")),
                    };
                    model.invoked_action().execute(Some(&item));
                }
                None
            })));
        let weak = Rc::downgrade(&model);
        self.named::<TextBox>("CommandInput")
            .key_down(move |_, event| {
                if let Some(model) = weak.upgrade() {
                    let key = match event.key {
                        Key::Enter => CadKey::Enter,
                        Key::Space => CadKey::Space,
                        Key::Escape => CadKey::Escape,
                        _ => CadKey::Other,
                    };
                    // Source marks Enter/Space before invoking; Escape after clearing/echoing.
                    if matches!(key, CadKey::Enter | CadKey::Space)
                        && model.input().is_some_and(|s| !s.trim().is_empty())
                    {
                        event.set_handled(true);
                    }
                    if model.handle_key(key) {
                        event.set_handled(true);
                    }
                }
            });
        let weak = Rc::downgrade(&model);
        self.named::<RibbonComboBox>("WorkspaceCombo")
            .add_committed(Some(MarkupDelegate::new(move |args| {
                let event = from_markup_value::<
                    Rc<ferroribbon::controls::RibbonComboBoxCommittedEventArgs>,
                >(&args[1])
                .expect("native combo commit args");
                if let Some(model) = weak.upgrade() {
                    let workspace = event.text().unwrap_or_default();
                    model.workspace_action().execute(Some(&workspace));
                }
                None
            })));
        let weak = Rc::downgrade(&model);
        let toggle = self.named::<ferroribbon::controls::RibbonToggleButton>("LightThemeToggle");
        let weak_toggle = toggle.downgrade();
        toggle.click(move |_, _| {
            if let (Some(model), Some(toggle)) = (weak.upgrade(), weak_toggle.upgrade()) {
                model
                    .contextual_action()
                    .execute(Some(&CadContextAction::SetLightTheme(
                        toggle.is_checked() == Some(true),
                    )));
            }
        });
        let combo = self.named::<RibbonComboBox>("WorkspaceCombo");
        let weak_combo = combo.downgrade();
        let weak_model = Rc::downgrade(&model);
        let token = combo.add_property_changed_handler(Rc::new(move |args| {
            if args.property() == RibbonComboBox::text_property().as_property()
                && let (Some(combo), Some(model)) = (weak_combo.upgrade(), weak_model.upgrade())
            {
                model.set_workspace_text(combo.text());
            }
        }));
        let weak_combo = combo.downgrade();
        self.subscriptions
            .borrow_mut()
            .push(ferroui_base::reactive::Disposable::create(move || {
                if let Some(combo) = weak_combo.upgrade() {
                    combo.remove_property_changed_handler(token);
                }
            }));
        self.initialize_property_combos();
        for (name, action) in [
            ("ShowTextEditorButton", CadContextAction::ShowTextEditor),
            ("ShowHatchButton", CadContextAction::ShowHatchCreation),
            ("CloseTextEditor", CadContextAction::CloseTextEditor),
            ("CloseHatch", CadContextAction::CloseHatch),
        ] {
            let weak = Rc::downgrade(&model);
            self.named::<ferroribbon::controls::RibbonButton>(name)
                .click(move |_, _| {
                    if let Some(model) = weak.upgrade() {
                        model.contextual_action().execute(Some(&action));
                    }
                });
        }
    }
    fn initialize_property_combos(&self) {
        macro_rules! setup {
            ($name:literal,$type:ty,$index:expr) => {{
                let combo = self.named::<RibbonComboBox>($name);
                combo.set_selected_item(Some(<$type>::samples()[$index].clone()));
                combo.set_item_text_selector(Some(MarkupDelegate::new(|args| {
                    let name = from_markup_value::<Option<Rc<$type>>>(&args[0])
                        .flatten()
                        .map(|v| v.name().to_owned())
                        .unwrap_or_default();
                    Some(Rc::new(name))
                })));
            }};
        }
        setup!("LayerCombo", CadLayer, 1);
        setup!("ColorCombo", CadColorOption, 0);
        setup!("TextColorCombo", CadColorOption, 0);
        setup!("HatchColorCombo", CadColorOption, 0);
        setup!("LinetypeCombo", CadLinetype, 0);
        setup!("LineweightCombo", CadLineweight, 0);
    }
    pub(crate) fn named<T: ferroui_base::ObjectType>(&self, name: &str) -> Ref<T> {
        ferroui_base::controls::NameScopeExtensions::find::<T>(self, name)
            .unwrap_or_else(|| panic!("Original CAD page named element missing: {name}"))
    }
    pub fn cad_ribbon(&self) -> Ref<Ribbon> {
        self.named("Ribbon")
    }
    pub fn draw_panel(&self) -> Ref<RibbonGroup> {
        self.named("DrawGroup")
    }
    pub fn layers_panel(&self) -> Ref<RibbonGroup> {
        self.named("LayersGroup")
    }
    pub fn layer_drop_down(&self) -> Ref<RibbonComboBox> {
        self.named("LayerCombo")
    }
    pub fn application_menu(&self) -> Ref<RibbonApplicationMenu> {
        self.named("AppMenu")
    }
    pub fn save_as_menu_item(&self) -> Ref<RibbonApplicationMenuItem> {
        self.named("SaveAsItem")
    }
}
impl Drop for CadPage {
    fn drop(&mut self) {
        if let Some(popup) = self.tip_popup.get_mut().take() {
            popup.set_is_open(false);
        }
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}
