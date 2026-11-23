use ferroui_base::{
    animation::{animators::CornerRadiusAnimator, Animation, InterpolatingAnimator},
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    metadata::{from_markup_value, MarkupAssembly},
    platform::storage::file_io::StorageProviderHelpers,
    CornerRadius, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref,
    StyledElementImpl, StyledProperty, TypeInfo, VisualImpl,
};
use ferroui_controls::{AppBuilder, Application, ControlImpl, TextBlock, TextBlockImpl, Window};
use ferroui_designer_support::remote::RemoteDesignerEntryPoint;
use ferroui_designer_support::{PreviewFixture, register_preview_fixture};
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use ferroui_markup_xaml::styling::StyleInclude;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use ferroui_skia::SkiaApplicationExtensions;
use std::{fs::File, path::Path};
use std::rc::Rc;

mod animation_codec;
mod behaviors;
mod motion;
mod review_states;
mod shader;
mod template_review;
mod template_rows;
mod tree_templates;

static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "todo-workbench",
    crate_name: "todo_workbench",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[
        (
            "FerroUI.Designer.Rsx.v1:TodoWorkbench.Design.PreviewBadge",
            r#"{"path":"crate::app_preview::PreviewBadge","inherit":true}"#,
        ),
        (
            "FerroUI.Designer.RsxAnimation.v1:crate::app_preview::motion::review_corner_property",
            "::ferroui_controls::Border::corner_radius_property",
        ),
    ],
};

pub(crate) fn start_if_requested() {
    RemoteDesignerEntryPoint::run_if_requested(|| {
        AppBuilder::configure::<Application>()
            .use_skia()
            .use_harfbuzz()
            .after_setup(|builder| {
                register_markup();
                register_fixtures();
                ferroui_designer_support::register_preview_animation_codec(Box::new(animation_codec::ReviewShadowCodec));
                if let Some(app) = builder.instance() {
                    app.styles()
                        .add(ferroui_themes_winui::WinUITheme::new().as_style());
                    app.styles().add(grid_styles().as_style());
                }
            })
    });
}

fn grid_styles() -> Rc<StyleInclude> {
    let include = StyleInclude::new(None);
    include.set_source(Some(
        "ferres://FerroUI.Controls.TreeDataGrid/Themes/Generic.xaml"
            .parse().expect("built-in TreeDataGrid style URI"),
    ));
    include
}

pub(crate) fn template_demo_window() -> Ref<Window> {
    register_markup();
    let window = Window::new();
    window.set_title(Some("FerroUI Account hierarchy".into()));
    window.set_width(660.0);
    window.set_height(350.0);
    window.styles().add(grid_styles().as_style());
    window.set_content(Some(ferroui_controls::Control::boxed(tree_templates::view())));
    window
}

fn register_fixtures() {
    register_preview_fixture("/app_preview/motion.rs", "Compiled motion", || {
        Ok(PreviewFixture::RustView(motion::view()))
    }).expect("valid Todo Workbench motion fixture");
    register_preview_fixture("/app_preview/behaviors.rs", "Compiled approval", || {
        Ok(PreviewFixture::RustView(behaviors::view().mount()))
    }).expect("valid Todo Workbench behavior fixture");
    register_preview_fixture("/app_preview/review_states.rs", "Interactive review", || {
        Ok(PreviewFixture::RustView(review_states::view()))
    }).expect("valid Todo Workbench state fixture");
    register_preview_fixture("/app_preview/template_review.rs", "Review queue", || {
        Ok(PreviewFixture::RustView(template_review::view()))
    }).expect("valid Todo Workbench template fixture");
    register_preview_fixture("/app_preview/tree_templates.rs", "Account hierarchy", || {
        Ok(PreviewFixture::RustView(tree_templates::view()))
    }).expect("valid Todo Workbench grid fixture");
    register_preview_fixture("/HostedTreeTemplates.xaml", "Account hierarchy", || {
        Ok(PreviewFixture::DataContext(Rc::new(tree_templates::customers())))
    }).expect("valid Todo Workbench XAML grid fixture");
    for (name, text) in [("Loading", "Loading tasks…"), ("Ready", "Ready to review")] {
        register_preview_fixture("/HostedPreview.xaml", name, move || {
            Ok(PreviewFixture::DataContext(Rc::new(text.to_owned())))
        }).expect("valid Todo Workbench XAML fixture");
    }
    register_preview_fixture("/HostedStateValues.xaml", "Review metrics", || {
        Ok(PreviewFixture::DataContext(Rc::new(0.8_f64)))
    }).expect("valid Todo Workbench state values fixture");
    for (name, text) in [("Empty queue", "No tasks yet"), ("One task", "Review the project")] {
        register_preview_fixture("/app_preview.rs", name, move || {
            use ferroui_view::prelude::*;
            Ok(PreviewFixture::RustView(rsx! {
                StackPanel {
                    width: 360.0,
                    height: 180.0,
                    spacing: 12.0,
                    TextBlock { text: "Todo Workbench", font_size: 24.0 }
                    TextBlock { text: text }
                }
            }.mount()))
        }).expect("valid Todo Workbench RSX fixture");
    }
}

fn register_markup() {
    FerroRuntimeXamlLoader::register();
    shader::register();
    ferroui_xaml_interactions::register_types();
    ferroui_xaml_interactions_custom::register_types();
    ferroui_controls_treedatagrid::register_types();
    Animation::register_custom_animator::<ReviewCorners>();
    ferroui_themes_winui::register_types();
    TypeInfo::register_namespaces(&[(module_path!(), "TodoWorkbench.Design")]);
    TypeInfo::register_all(&[PreviewBadge::TYPE]);
    MarkupAssembly::register(&ASSEMBLY);
}

#[derive(Default)]
struct ReviewCorners;

impl InterpolatingAnimator for ReviewCorners {
    type Value = CornerRadius;

    fn interpolate(&self, progress: f64, from: &CornerRadius, to: &CornerRadius) -> CornerRadius {
        CornerRadiusAnimator::interpolate_core(progress * progress, from, to)
    }
}

pub(crate) fn load_window(path: &Path) -> Result<Ref<Window>, Box<dyn std::error::Error>> {
    register_markup();
    let path = path.canonicalize()?;
    let uri = path
        .to_str()
        .and_then(|path| StorageProviderHelpers::try_get_uri_from_file_path(path, false))
        .ok_or("The XAML window path cannot be expressed as a file URI")?;
    let file = File::open(&path)?;
    let loaded = FerroRuntimeXamlLoader::load_stream(
        Box::new(file),
        Some(&ASSEMBLY),
        None,
        Some(uri),
        false,
    )?;
    from_markup_value(&Some(loaded))
        .ok_or_else(|| "The XAML document must contain a Window root".into())
}

#[repr(C)]
pub(crate) struct PreviewBadge {
    base: TextBlock,
}

ferro_class!(PreviewBadge: TextBlock);
ferro_class_info!(PreviewBadge {
    new: PreviewBadge::new,
    markup: { namespace: "TodoWorkbench.Design" },
});
ferro_impl_classes!(PreviewBadge: StyledElementImpl, VisualImpl, LayoutableImpl,
    InteractiveImpl, InputElementImpl, ControlImpl, TextBlockImpl);

ferro_properties! {
    impl PreviewBadge {
        pub fn emphasis_property() -> StyledProperty<f64> {
            FerroProperty::register::<PreviewBadge, _>("Emphasis", 1.0)
        }
    }
}

impl FerroObjectImpl for PreviewBadge {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::emphasis_property().as_property() {
            this.set_font_size(24.0 * this.get_value(Self::emphasis_property()).clamp(0.5, 2.0));
        }
    }
}

impl PreviewBadge {
    fn new() -> Ref<Self> {
        let badge = instantiate(Self {
            base: TextBlock::construct(),
        });
        badge.set_font_size(24.0);
        badge
    }
}

impl ferroui_view::ViewClass for PreviewBadge {
    fn create() -> Ref<Self> {
        Self::new()
    }
}

#[cfg(test)]
#[test]
fn registered_preview_badge_can_be_used_in_rsx() {
    use ferroui_view::prelude::*;
    let _ = rsx! { crate::app_preview::PreviewBadge { text: "Design review" } };
}
