//! Port of `Pages/SettingsPage.xaml.cs`: the class of the document
//! `Pages/SettingsPage.xaml`.

use crate::app::App;
use crate::markup::xaml_class;
use crate::models::CatalogTheme;
use crate::transparent_styles::TransparentStyles;
use crate::view_models::SettingsViewModel;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Colors, FlowDirection, IBrush};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::{styles_as_style, Styles, ThemeVariant};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
    VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Application, ContentPage, ControlImpl, PageImpl, SelectionChangedEventArgs, TopLevel, Window, WindowDecorations,
    WindowTransparencyLevel, WindowTransparencyLevelCollection,
};
use std::cell::OnceCell;
use std::rc::Rc;

#[repr(C)]
pub struct SettingsPage {
    base: ContentPage,
    /// `_transparentStyles`. Created when first used: the field initialiser
    /// of the managed original runs with the constructor.
    transparent_styles: OnceCell<Ref<TransparentStyles>>,
}

ferro_class!(SettingsPage: ContentPage);
ferro_impl_classes!(
    SettingsPage: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    PageImpl
);

/// The first added item of a selection change, as a `T`.
fn first_added<T: Clone + 'static>(e: &Rc<dyn IRoutedEventArgs>) -> Option<T> {
    let e = e.downcast_ref::<SelectionChangedEventArgs>()?;
    from_markup_value::<T>(e.added_items().first()?)
}

ferro_class_info!(SettingsPage {
    new: SettingsPage::new,
    markup: {
        methods: [
            fn Decorations_SelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<SettingsPage>, _sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.decorations_selection_changed(first_added::<WindowDecorations>(&e))
                },
            fn Themes_SelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<SettingsPage>, _sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.themes_selection_changed(first_added::<CatalogTheme>(&e))
                },
            fn ThemeVariants_SelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<SettingsPage>, _sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.theme_variants_selection_changed(first_added::<ThemeVariant>(&e))
                },
            fn FlowDirection_SelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<SettingsPage>, _sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.flow_direction_selection_changed(first_added::<FlowDirection>(&e))
                },
            fn TransparencyLevels_SelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<SettingsPage>, _sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.transparency_levels_selection_changed(first_added::<WindowTransparencyLevel>(&e))
                },
        ],
    },
});
xaml_class!(SettingsPage, "/Pages/SettingsPage.xaml");

impl VisualImpl for SettingsPage {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        if let Some(view_model) = from_markup_value::<Rc<SettingsViewModel>>(&this.data_context()) {
            let top_level = TopLevel::get_top_level(Some(this)).expect("the page is attached to a top level");
            if let Some(window) = top_level.cast::<Window>() {
                view_model.set_selected_decoration_index(window.window_decorations() as i32);
            }
        }
    }
}

impl SettingsPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct(), transparent_styles: OnceCell::new() }
    }

    /// `new SettingsPage(settingsViewModel)`.
    pub fn with_settings_view_model(settings_view_model: Rc<SettingsViewModel>) -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        this.set_data_context(Some(settings_view_model as BoxedValue));
        this
    }

    /// `new SettingsPage()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(SettingsViewModel::new() as BoxedValue));
        this
    }

    fn transparent_styles(&self) -> Ref<TransparentStyles> {
        self.transparent_styles.get_or_init(TransparentStyles::new).clone()
    }

    fn top_level(&self) -> Option<Ref<TopLevel>> {
        TopLevel::get_top_level(Some(self))
    }

    fn decorations_selection_changed(&self, added: Option<WindowDecorations>) {
        if let (Some(window), Some(system_decorations)) = (self.top_level().and_then(|t| t.cast::<Window>()), added) {
            window.set_window_decorations(system_decorations);
        }
    }

    fn themes_selection_changed(&self, added: Option<CatalogTheme>) {
        if let Some(theme) = added {
            App::set_catalog_themes(theme);
        }
    }

    fn theme_variants_selection_changed(&self, added: Option<ThemeVariant>) {
        if let (Some(app), Some(theme_variant)) = (Application::current(), added) {
            app.set_requested_theme_variant(Some(theme_variant));
        }
    }

    fn flow_direction_selection_changed(&self, added: Option<FlowDirection>) {
        if let (Some(top_level), Some(flow_direction)) = (self.top_level(), added) {
            top_level.set_flow_direction(flow_direction);
        }
    }

    fn transparency_levels_selection_changed(&self, added: Option<WindowTransparencyLevel>) {
        if let (Some(top_level), Some(transparency_level)) = (self.top_level(), added) {
            top_level.set_transparency_level_hint(WindowTransparencyLevelCollection::new(vec![transparency_level.clone()]));

            if top_level.actual_transparency_level() != WindowTransparencyLevel::none()
                && transparency_level != WindowTransparencyLevel::none()
            {
                let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::with_opacity(Colors::GRAY, 0.2));
                top_level.set_background(Some(brush));
                let transparent_styles = styles_as_style(&self.transparent_styles().upcast::<Styles>());
                if !top_level.styles().contains(transparent_styles.clone()) {
                    top_level.styles().add(transparent_styles);
                }
            } else {
                top_level.set_background(None);
                top_level.styles().remove(styles_as_style(&self.transparent_styles().upcast::<Styles>()));
            }
        }
    }
}
