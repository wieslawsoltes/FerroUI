// Copyright (c) 2026 Wiesław Šoltés. MIT. Native view of the complete source WordPage.
use crate::{
    demo_settings::{DemoSettings, DemoSubscription},
    word_view_model::{WordAction, WordServices, WordViewModel},
};
use ferroribbon::controls::{
    Ribbon, RibbonColorPicker, RibbonComboBoxCommittedEventArgs, RibbonDropDownButton,
    RibbonFontComboBox, RibbonFontSizeComboBox, RibbonGallery, RibbonGalleryItemEventArgs,
    RibbonGridPicker, RibbonItemInvokedEventArgs, RibbonTabChangedEventArgs, RibbonTitleBar,
    RibbonZoomControl,
};
use ferroribbon_core::{
    commands::{RibbonCommandCatalog, RibbonCommandMetadata},
    model::RibbonGridSize,
};
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
    data::core::ValueTypes,
    ferro_class, ferro_class_info, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    media::Color,
    metadata::{MarkupDelegate, from_markup_value},
    platform::PlatformThemeVariant,
    styling::ThemeVariant,
};
use ferroui_controls::{
    ContentControlImpl, Control, ControlImpl, TopLevel, UserControl,
    primitives::{PopupRoot, TemplatedControlImpl},
};
use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
};
#[repr(C)]
pub struct WordPage {
    base: UserControl,
    model: OnceCell<Rc<WordViewModel>>,
    settings: OnceCell<Rc<DemoSettings>>,
    cleanup: RefCell<Vec<Box<dyn FnOnce()>>>,
    settings_subscription: RefCell<Option<DemoSubscription>>,
}
ferro_class!(WordPage:UserControl);
ferro_class_info!(WordPage{new:WordPage::new,markup:{namespace:"FerroRibbon.Sample.Pages"}});
ferro_impl_classes!(WordPage:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
fn argb(c: Color) -> u32 {
    ((c.a as u32) << 24) | ((c.r as u32) << 16) | ((c.g as u32) << 8) | (c.b as u32)
}
fn text(s: Option<ferroribbon_core::RibbonString>) -> Option<String> {
    s.map(|s| s.to_utf8().expect("Word text requires a native string"))
}
fn style_value(event: &RibbonGalleryItemEventArgs) -> Option<String> {
    event
        .value()
        .and_then(|v| v.downcast_ref::<String>().cloned())
}
impl WordPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            model: OnceCell::new(),
            settings: OnceCell::new(),
            cleanup: RefCell::new(Vec::new()),
            settings_subscription: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        Self::new_with_settings(DemoSettings::current())
    }
    pub fn new_with_settings(settings: Rc<DemoSettings>) -> Ref<Self> {
        let this = instantiate(Self::construct());
        if let Some(populate) = ferroui_markup_xaml::FerroXamlLoader::populate_override(Self::TYPE)
        {
            populate(None, &Control::boxed(this.clone())).expect("Word populate override");
        } else {
            crate::generated::compiled_word_page::populate(None, &this)
                .expect("compiled original WordPage");
        }
        let vm = WordViewModel::new(Rc::new(NativeWordServices {
            page: this.downgrade(),
            settings: Rc::downgrade(&settings),
        }));
        assert!(this.model.set(vm.clone()).is_ok());
        this.set_data_context(Some(vm.clone()));
        assert!(this.settings.set(settings.clone()).is_ok());
        let ribbon = this.ribbon();
        ribbon.set_font_family(ribbon.font_family().with_automatic_optical_sizing());
        settings.set_word_ribbon(Some(ribbon.clone()));
        let catalog = RibbonCommandCatalog::new();
        ribbon.set_command_catalog(Some(catalog.clone()));
        let weak = Rc::downgrade(&vm);
        catalog.register_action(
            Some("save".into()),
            Some("Save".into()),
            Rc::new(move |_| {
                if let Some(vm) = weak.upgrade() {
                    vm.action().execute(Some(&Some(WordAction::Save)));
                }
            }),
            None,
            RibbonCommandMetadata {
                shortcut: Some("Ctrl+S".into()),
                description: Some("Save the document".into()),
                category: Some("File".into()),
                ..Default::default()
            },
        );
        for id in ["cut", "copy", "paste", "bold", "newComment"] {
            ribbon.quick_access_candidate_ids().add(Some(id.into()));
        }
        macro_rules! hook {
            ($part:expr,$add:ident,$remove:ident,$make:expr) => {{
                let part = ($part).clone();
                let weak = this.downgrade();
                let handler = MarkupDelegate::new(move |args| {
                    if let Some(page) = weak.upgrade() {
                        let action = ($make)(&page, args);
                        page.view_model().action().execute(Some(&Some(action)));
                    }
                    None
                });
                part.$add(Some(handler.clone()));
                let weak = part.downgrade();
                this.cleanup.borrow_mut().push(Box::new(move || {
                    if let Some(part) = weak.upgrade() {
                        part.$remove(Some(handler));
                    }
                }));
            }};
        }
        hook!(
            ribbon,
            add_item_invoked,
            remove_item_invoked,
            |_: &WordPage, args: &[ferroui_base::metadata::MarkupValue]| {
                let e = from_markup_value::<Rc<RibbonItemInvokedEventArgs>>(&args[1])
                    .expect("Word ItemInvoked");
                WordAction::Invoked {
                    item_id: text(e.item_id.clone()),
                    command_id: text(e.command_id.clone()),
                    parameter: e.parameter.as_ref().map(|p| {
                        let value = p.clone();
                        Rc::new(move || word_parameter_text(&value))
                            as crate::word_view_model::WordParameterText
                    }),
                }
            }
        );
        hook!(
            ribbon,
            add_selected_tab_changed,
            remove_selected_tab_changed,
            |_: &WordPage, args: &[ferroui_base::metadata::MarkupValue]| {
                let e = from_markup_value::<Rc<RibbonTabChangedEventArgs>>(&args[1])
                    .expect("Word tab change");
                WordAction::TabChanged(e.new_tab.as_ref().and_then(|tab| tab.header()))
            }
        );
        hook!(
            this.named::<RibbonFontComboBox>("FontFamilyBox"),
            add_committed,
            remove_committed,
            |_: &WordPage, args: &[ferroui_base::metadata::MarkupValue]| {
                WordAction::FontFamilyCommitted(
                    from_markup_value::<Rc<RibbonComboBoxCommittedEventArgs>>(&args[1])
                        .expect("font commit")
                        .text(),
                )
            }
        );
        hook!(
            this.named::<RibbonFontSizeComboBox>("FontSizeBox"),
            add_committed,
            remove_committed,
            |_: &WordPage, args: &[ferroui_base::metadata::MarkupValue]| {
                WordAction::FontSizeCommitted(
                    from_markup_value::<Rc<RibbonComboBoxCommittedEventArgs>>(&args[1])
                        .expect("size commit")
                        .text(),
                )
            }
        );
        hook!(
            this.named::<RibbonColorPicker>("FontColor"),
            add_color_selected,
            remove_color_selected,
            |_: &WordPage, args: &[ferroui_base::metadata::MarkupValue]| WordAction::FontColor(
                from_markup_value::<Option<Color>>(&args[1])
                    .expect("font color")
                    .map(argb)
            )
        );
        hook!(
            this.named::<RibbonColorPicker>("FontColor"),
            add_click,
            remove_click,
            |page: &WordPage, _: &[ferroui_base::metadata::MarkupValue]| WordAction::FontColor(
                page.named::<RibbonColorPicker>("FontColor")
                    .selected_color()
                    .map(argb)
            )
        );
        hook!(
            this.named::<RibbonColorPicker>("PageColor"),
            add_color_selected,
            remove_color_selected,
            |_: &WordPage, args: &[ferroui_base::metadata::MarkupValue]| WordAction::PageColor(
                from_markup_value::<Option<Color>>(&args[1])
                    .expect("page color")
                    .map(argb)
            )
        );
        hook!(
            this.named::<RibbonGallery>("Styles"),
            add_item_preview,
            remove_item_preview,
            |_: &WordPage, args: &[ferroui_base::metadata::MarkupValue]| WordAction::StylePreview(
                style_value(
                    &from_markup_value::<Rc<RibbonGalleryItemEventArgs>>(&args[1])
                        .expect("style preview")
                )
            )
        );
        hook!(
            this.named::<RibbonGallery>("Styles"),
            add_item_click,
            remove_item_click,
            |_: &WordPage, args: &[ferroui_base::metadata::MarkupValue]| WordAction::StyleClick(
                style_value(
                    &from_markup_value::<Rc<RibbonGalleryItemEventArgs>>(&args[1])
                        .expect("style click")
                )
            )
        );
        hook!(
            this.named::<RibbonGridPicker>("TablePicker"),
            add_size_picked,
            remove_size_picked,
            |_: &WordPage, args: &[ferroui_base::metadata::MarkupValue]| {
                let size = from_markup_value::<RibbonGridSize>(&args[1]).expect("table size");
                WordAction::TablePicked {
                    rows: size.rows,
                    columns: size.columns,
                }
            }
        );
        let weak = this.downgrade();
        let subscription =
            this.named::<RibbonZoomControl>("Zoom")
                .value_changed(move |_, value| {
                    if let Some(page) = weak.upgrade() {
                        page.view_model()
                            .action()
                            .execute(Some(&Some(WordAction::Zoom(value))));
                    }
                });
        this.cleanup
            .borrow_mut()
            .push(Box::new(move || subscription.dispose()));
        let weak = this.downgrade();
        let shared = Rc::downgrade(&settings);
        this.settings_subscription
            .replace(Some(settings.changed(move || {
                if let Some(page) = weak.upgrade()
                    && let Some(shared) = shared.upgrade()
                {
                    page.apply_settings(&shared);
                }
            })));
        let weak = this.downgrade();
        this.loaded(move |_, _| {
            if let Some(page) = weak.upgrade() {
                page.named::<RibbonTitleBar>("TitleBar")
                    .set_ribbon(Some(page.ribbon()));
            }
        });
        this.apply_settings(&settings);
        this
    }
    pub fn ribbon(&self) -> Ref<Ribbon> {
        self.named::<Ribbon>("Ribbon")
    }
    pub fn view_model(&self) -> Rc<WordViewModel> {
        self.model.get().expect("Word initialized").clone()
    }
    fn apply_settings(&self, settings: &DemoSettings) {
        let ribbon = self.ribbon();
        ribbon.set_density(settings.density());
        if let Some(mode) = settings.display_mode() {
            ribbon.set_display_mode(mode);
        }
    }
    pub(crate) fn named<T: ferroui_base::ObjectType>(&self, name: &str) -> Ref<T> {
        ferroui_base::controls::NameScopeExtensions::find::<T>(self, name)
            .unwrap_or_else(|| panic!("Missing source Word element {name}"))
    }
}
impl Drop for WordPage {
    fn drop(&mut self) {
        self.settings_subscription.get_mut().take();
        for cleanup in self.cleanup.get_mut().drain(..) {
            cleanup();
        }
    }
}
struct NativeWordServices {
    page: ferroui_base::WeakRef<WordPage>,
    settings: std::rc::Weak<DemoSettings>,
}
impl WordServices for NativeWordServices {
    fn write(&self, message: &str) -> Result<(), String> {
        self.settings
            .upgrade()
            .ok_or("Word settings owner is no longer available")?
            .write(Some(message))
            .map_err(|e| e.to_string())
    }
    fn hide_table_flyout(&self) {
        if let Some(page) = self.page.upgrade()
            && let Some(flyout) = page.named::<RibbonDropDownButton>("InsertTable").flyout()
        {
            flyout.hide();
        }
    }
    fn set_table_context(&self, visible: bool) {
        if let Some(page) = self.page.upgrade() {
            page.ribbon().set_active_contextual_groups(if visible {
                &[Some("table")]
            } else {
                &[]
            });
        }
    }
    fn toggle_dark_theme(&self) {
        if let Some(page) = self.page.upgrade()
            && let Some(mut top) = TopLevel::get_top_level(Some(&page))
        {
            while let Some(popup) = top.cast::<PopupRoot>() {
                top = popup.parent_top_level().expect("Word popup root owner");
            }
            let current = top
                .requested_theme_variant()
                .as_ref()
                .and_then(|variant| ThemeVariant::to_platform_theme_variant(Some(variant)))
                .or_else(|| ThemeVariant::frame_theme_variant(top.actual_theme_variant().as_ref()));
            let theme = if current == Some(PlatformThemeVariant::Dark) {
                ThemeVariant::light()
            } else {
                ThemeVariant::dark()
            };
            top.set_requested_theme_variant(Some(theme));
        }
    }
}

// Only the two concrete Uno values used by the original command log need
// display adaptation. All scalar/culture formatting stays in the native SDK.
pub(crate) fn word_parameter_text(value: &ferroui_base::BoxedValue) -> Result<String, String> {
    if Rc::ptr_eq(value, &RibbonColorPicker::no_color_parameter()) {
        return Ok("System.Object".into());
    }
    if let Some(color) = value.downcast_ref::<Color>() {
        return Ok(format!("#{:08X}", argb(*color)));
    }
    if ValueTypes::try_to_string(&**value).is_none() {
        return Err("Unregistered Word command parameter display".into());
    }
    ferroui_base::data::converters::composite_format::format_values("{0}", &[Some(value.clone())])
        .map_err(|e| e.to_string())
}
