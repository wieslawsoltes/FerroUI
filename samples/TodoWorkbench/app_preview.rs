use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, TypeInfo, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl, metadata::MarkupAssembly,
};
use ferroui_controls::{AppBuilder, Application, ControlImpl, TextBlock, TextBlockImpl};
use ferroui_designer_support::remote::RemoteDesignerEntryPoint;
use ferroui_harfbuzz::HarfBuzzApplicationExtensions;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use ferroui_skia::SkiaApplicationExtensions;

static ASSEMBLY: MarkupAssembly = MarkupAssembly {
    name: "todo-workbench",
    crate_name: "todo_workbench",
    xmlns_definitions: &[],
    xmlns_prefixes: &[],
    metadata: &[],
};

pub(crate) fn start_if_requested() {
    RemoteDesignerEntryPoint::run_if_requested(|| {
        AppBuilder::configure::<Application>()
            .use_skia()
            .use_harfbuzz()
            .after_setup(|builder| {
                FerroRuntimeXamlLoader::register();
                ferroui_themes_winui::register_types();
                TypeInfo::register_namespaces(&[(module_path!(), "TodoWorkbench.Design")]);
                TypeInfo::register_all(&[PreviewBadge::TYPE]);
                MarkupAssembly::register(&ASSEMBLY);
                if let Some(app) = builder.instance() {
                    app.styles()
                        .add(ferroui_themes_winui::WinUITheme::new().as_style());
                }
            })
    });
}

#[repr(C)]
struct PreviewBadge {
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
