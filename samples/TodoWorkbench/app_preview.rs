use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    metadata::{from_markup_value, MarkupAssembly},
    platform::storage::file_io::StorageProviderHelpers,
    FerroObjectImpl, Ref, StyledElementImpl, TypeInfo, VisualImpl,
};
use ferroui_controls::{AppBuilder, Application, ControlImpl, TextBlock, TextBlockImpl, Window};
use ferroui_designer_support::remote::RemoteDesignerEntryPoint;
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use ferroui_skia::SkiaApplicationExtensions;
use std::{fs::File, path::Path};

static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "todo-workbench",
    crate_name: "todo_workbench",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[(
        "FerroUI.Designer.Rsx.v1:TodoWorkbench.Design.PreviewBadge",
        r#"{"path":"crate::app_preview::PreviewBadge","inherit":true}"#,
    )],
};

pub(crate) fn start_if_requested() {
    RemoteDesignerEntryPoint::run_if_requested(|| {
        AppBuilder::configure::<Application>()
            .use_skia()
            .use_harfbuzz()
            .after_setup(|builder| {
                register_markup();
                if let Some(app) = builder.instance() {
                    app.styles()
                        .add(ferroui_themes_winui::WinUITheme::new().as_style());
                }
            })
    });
}

fn register_markup() {
    FerroRuntimeXamlLoader::register();
    ferroui_themes_winui::register_types();
    TypeInfo::register_namespaces(&[(module_path!(), "TodoWorkbench.Design")]);
    TypeInfo::register_all(&[PreviewBadge::TYPE]);
    MarkupAssembly::register(&ASSEMBLY);
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
ferro_impl_classes!(PreviewBadge: FerroObjectImpl, StyledElementImpl, VisualImpl, LayoutableImpl,
    InteractiveImpl, InputElementImpl, ControlImpl, TextBlockImpl);

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
