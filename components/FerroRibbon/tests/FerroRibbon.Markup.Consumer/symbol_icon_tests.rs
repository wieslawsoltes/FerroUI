use ferroui_controls::{Symbol, SymbolIcon, SymbolIconSource};
use ferroribbon::{
    compat::MenuFlyoutItem,
    primitives::RibbonIconPresenter,
};
use ferroui_base::data::core::ValueTypes;
use ferroui_controls::{StackPanel, Viewbox};
#[test]
fn independent_symbol_enum_control_and_source_markup_keeps_snapshot_and_clone_rules() {
    super::tests::run(|| {
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("SymbolIcons.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/SymbolIcons.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let root = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let original = root.children().get(0).cast::<SymbolIcon>().unwrap();
            assert_eq!(original.symbol(), Symbol::Home);
            assert_eq!(original.symbol(), Symbol::Home);
            let p = root
                .children()
                .get(1)
                .cast::<RibbonIconPresenter>()
                .unwrap();
            let clone = p
                .children()
                .get(0)
                .cast::<Viewbox>()
                .unwrap()
                .child()
                .unwrap()
                .cast::<SymbolIcon>()
                .unwrap();
            assert!(!clone.ptr_eq(&original));
            assert_eq!(clone.symbol(), Symbol::Home);
            assert_eq!(clone.width(), 24.);
            assert_eq!(clone.foreground(), original.foreground());
            original.set_symbol(Symbol::Add);
            assert_eq!(clone.symbol(), Symbol::Home);
            p.set_icon_size(32.);
            assert_eq!(
                p.children()
                    .get(0)
                    .cast::<Viewbox>()
                    .unwrap()
                    .child()
                    .unwrap()
                    .cast::<SymbolIcon>()
                    .unwrap()
                    .symbol(),
                Symbol::Add
            );
            let p = root
                .children()
                .get(2)
                .cast::<RibbonIconPresenter>()
                .unwrap();
            let source = ValueTypes::as_object(&*p.icon().unwrap())
                .unwrap()
                .cast::<SymbolIconSource>()
                .unwrap();
            let generated = p
                .children()
                .get(0)
                .cast::<Viewbox>()
                .unwrap()
                .child()
                .unwrap()
                .cast::<SymbolIcon>()
                .unwrap();
            assert_eq!(generated.symbol(), Symbol::Save);
            assert_eq!(generated.foreground(), source.foreground());
            source.set_symbol(Symbol::Accept);
            assert_eq!(generated.symbol(), Symbol::Save);
            let menu = root.children().get(3).cast::<MenuFlyoutItem>().unwrap();
            let menu_icon = ValueTypes::as_object(&*menu.icon().unwrap())
                .unwrap()
                .cast::<SymbolIcon>()
                .unwrap();
            assert_eq!(menu_icon.symbol(), Symbol::Save);
            p.set_icon_size(32.);
            assert_eq!(
                p.children()
                    .get(0)
                    .cast::<Viewbox>()
                    .unwrap()
                    .child()
                    .unwrap()
                    .cast::<SymbolIcon>()
                    .unwrap()
                    .symbol(),
                Symbol::Accept
            );
            assert!(original.parent().unwrap().ptr_eq(&root));
        }
    });
}
