//! The control themes of the pull-to-refresh controls of the test theme:
//! the themes of the reference simple theme built in code, with the same
//! templates, part names, bindings and setters.
//!
//! Left out: the background and foreground setters of the refresh
//! visualizer, whose values are theme brushes.

use crate::presenters::ContentPresenter;
use crate::primitives::TemplatedControl;
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions, IControlTemplate};
use crate::{ContentControl, Control, Grid, Panel, PathIcon, RefreshContainer, RefreshVisualizer};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::InputElement;
use ferroui_base::layout::Layoutable;
use ferroui_base::media::Geometry;
use ferroui_base::styling::{ControlTheme, ITemplate, Setter, Styles};
use ferroui_base::{BoxedValue, FerroObject, FerroProperty, Ref};
use std::rc::Rc;

/// The path of the refresh glyph of the reference theme.
const REFRESH_GLYPH: &str = "M18.6195264,3.31842271 C19.0080059,3.31842271 19.3290603,3.60710385 19.3798716,3.9816481 L19.3868766,4.08577298 L19.3868766,6.97963208 C19.3868766,7.36811161 19.0981955,7.68916605 18.7236513,7.73997735 L18.6195264,7.74698235 L15.7256673,7.74698235 C15.3018714,7.74698235 14.958317,7.40342793 14.958317,6.97963208 C14.958317,6.59115255 15.2469981,6.27009811 15.6215424,6.21928681 L15.7256673,6.21228181 L16.7044011,6.21182461 C13.7917384,3.87107476 9.52212532,4.05209336 6.81933829,6.75488039 C3.92253872,9.65167996 3.92253872,14.34832 6.81933829,17.2451196 C9.71613786,20.1419192 14.4127779,20.1419192 17.3095775,17.2451196 C19.0725398,15.4821573 19.8106555,12.9925923 19.3476248,10.58925 C19.2674502,10.173107 19.5398064,9.77076216 19.9559494,9.69058758 C20.3720923,9.610413 20.7744372,9.88276918 20.8546118,10.2989121 C21.4129973,13.1971899 20.5217103,16.2033812 18.3947747,18.3303168 C14.8986373,21.8264542 9.23027854,21.8264542 5.73414113,18.3303168 C2.23800371,14.8341794 2.23800371,9.16582064 5.73414113,5.66968323 C9.05475132,2.34907304 14.3349409,2.18235834 17.8523166,5.16953912 L17.8521761,4.08577298 C17.8521761,3.66197713 18.1957305,3.31842271 18.6195264,3.31842271 Z";

fn bind_template(target: &FerroObject, target_property: &'static FerroProperty, source: &'static FerroProperty) {
    target.bind_binding(target_property, &TemplateBinding::new(source));
}

/// The control template of the refresh container.
pub fn refresh_container_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        let grid = Grid::new();

        let presenter = ContentPresenter::new();
        presenter.set_name(Some("PART_ContentPresenter".to_string()));
        let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
            bind_template(&presenter, target, source);
        };
        bind(ContentPresenter::background_property().as_property(), TemplatedControl::background_property().as_property());
        bind(
            ContentPresenter::border_brush_property().as_property(),
            TemplatedControl::border_brush_property().as_property(),
        );
        bind(
            ContentPresenter::border_thickness_property().as_property(),
            TemplatedControl::border_thickness_property().as_property(),
        );
        bind(
            ContentPresenter::corner_radius_property().as_property(),
            TemplatedControl::corner_radius_property().as_property(),
        );
        bind(
            ContentPresenter::content_template_property().as_property(),
            ContentControl::content_template_property().as_property(),
        );
        bind(ContentPresenter::content_property().as_property(), ContentControl::content_property().as_property());
        bind(ContentPresenter::padding_property().as_property(), TemplatedControl::padding_property().as_property());
        bind(
            ContentPresenter::vertical_content_alignment_property().as_property(),
            ContentControl::vertical_content_alignment_property().as_property(),
        );
        bind(
            ContentPresenter::horizontal_content_alignment_property().as_property(),
            ContentControl::horizontal_content_alignment_property().as_property(),
        );
        grid.children().add(presenter.register_in_name_scope(&**ns));

        let visualizer_presenter = Grid::new();
        visualizer_presenter.set_name(Some("PART_RefreshVisualizerPresenter".to_string()));
        grid.children().add(visualizer_presenter.register_in_name_scope(&**ns));

        grid.upcast::<Control>()
    })
}

/// The control template of the refresh visualizer.
pub fn refresh_visualizer_template() -> Rc<dyn IControlTemplate> {
    FuncControlTemplate::new(|_, ns| {
        let root = Grid::new();
        root.set_name(Some("PART_Root".to_string()));
        root.set_min_height(80.0);
        bind_template(
            &root,
            Panel::background_property().as_property(),
            TemplatedControl::background_property().as_property(),
        );
        root.register_in_name_scope(&**ns).upcast::<Control>()
    })
}

/// The content of the refresh visualizer of the reference theme: built anew
/// for each visualizer the theme is applied to.
struct RefreshVisualizerContentTemplate;

impl ITemplate for RefreshVisualizerContentTemplate {
    fn build(&self) -> BoxedValue {
        let icon = PathIcon::new();
        icon.set_name(Some("PART_Icon".to_string()));
        icon.set_data(Geometry::parse(REFRESH_GLYPH).ok());
        icon.set_width(24.0);
        icon.set_height(24.0);
        let content: Option<BoxedValue> = Some(Control::boxed(icon));
        Rc::new(content)
    }
}

/// The control theme of the refresh container.
pub fn refresh_container_theme() -> Ref<ControlTheme> {
    ControlTheme::with_setters(
        RefreshContainer::TYPE,
        [Setter::new(TemplatedControl::template_property(), Some(refresh_container_template()))],
    )
}

/// The control theme of the refresh visualizer.
pub fn refresh_visualizer_theme() -> Ref<ControlTheme> {
    ControlTheme::with_setters(
        RefreshVisualizer::TYPE,
        [
            Setter::new(InputElement::is_tab_stop_property(), false),
            Setter::new(InputElement::is_hit_test_visible_property(), false),
            Setter::new(Layoutable::height_property(), 100.0),
            Setter::new_template(
                ContentControl::content_property().as_property(),
                Rc::new(RefreshVisualizerContentTemplate),
            ),
            Setter::new(TemplatedControl::template_property(), Some(refresh_visualizer_template())),
        ],
    )
}

/// Adds the control themes of the pull-to-refresh controls.
pub fn add_pull_to_refresh_themes(styles: &Ref<Styles>) {
    styles.resources().add_value(ResourceKey::Type(RefreshContainer::TYPE), refresh_container_theme());
    styles.resources().add_value(ResourceKey::Type(RefreshVisualizer::TYPE), refresh_visualizer_theme());
}
