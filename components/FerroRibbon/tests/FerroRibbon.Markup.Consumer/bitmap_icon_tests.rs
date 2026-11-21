use ferroribbon::{
    compat::{BitmapIcon, BitmapIconSource, MenuFlyoutItem},
    primitives::RibbonIconPresenter,
};
use ferroui_base::{
    FerroObjectImpl, StyledElementImpl, VisualImpl, data::core::ValueTypes, ferro_class,
    ferro_impl_classes, input::InputElementImpl, instantiate, interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{ControlImpl, StackPanel, Viewbox, primitives::TemplatedControlImpl};
#[repr(C)]
struct DerivedBitmap {
    base: BitmapIcon,
}
ferro_class!(DerivedBitmap:BitmapIcon);
ferro_impl_classes!(DerivedBitmap:FerroObjectImpl,StyledElementImpl,VisualImpl,InputElementImpl,InteractiveImpl,LayoutableImpl,ControlImpl,TemplatedControlImpl);
#[test]
fn independent_bitmap_uri_flags_parented_copies_and_resources_compile_and_load_at_runtime() {
    crate::tests::run(|| {
        let derived = instantiate(DerivedBitmap {
            base: BitmapIcon::construct(),
        });
        derived.set_show_as_monochrome(false);
        assert!(!derived.show_as_monochrome());
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("BitmapIcons.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/BitmapIcons.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let root = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let original = root.children().get(0).cast::<BitmapIcon>().unwrap();
            assert!(!original.show_as_monochrome());
            assert!(
                original.load_error().is_some(),
                "missing asset stays an explicit native load error"
            );
            let p = root
                .children()
                .get(1)
                .cast::<RibbonIconPresenter>()
                .unwrap();
            let cloned = p
                .children()
                .get(0)
                .cast::<Viewbox>()
                .unwrap()
                .child()
                .unwrap()
                .cast::<BitmapIcon>()
                .unwrap();
            assert!(!cloned.ptr_eq(&original));
            assert_eq!(cloned.uri_source(), original.uri_source());
            assert_eq!(cloned.width(), 24.);
            assert!(!cloned.show_as_monochrome());
            assert_eq!(cloned.foreground(), original.foreground());
            original.set_show_as_monochrome(true);
            assert!(!cloned.show_as_monochrome());
            p.set_icon_size(32.);
            assert!(
                p.children()
                    .get(0)
                    .cast::<Viewbox>()
                    .unwrap()
                    .child()
                    .unwrap()
                    .cast::<BitmapIcon>()
                    .unwrap()
                    .show_as_monochrome()
            );
            let p = root
                .children()
                .get(2)
                .cast::<RibbonIconPresenter>()
                .unwrap();
            let source = ValueTypes::as_object(&*p.icon().unwrap())
                .unwrap()
                .cast::<BitmapIconSource>()
                .unwrap();
            let generated = p.children().get(0).cast::<BitmapIcon>().unwrap();
            assert_eq!(generated.width(), 23.5);
            assert_eq!(generated.uri_source(), source.uri_source());
            assert!(!generated.show_as_monochrome());
            let menu = root.children().get(3).cast::<MenuFlyoutItem>().unwrap();
            let menu_icon = ValueTypes::as_object(&*menu.icon().unwrap())
                .unwrap()
                .cast::<BitmapIcon>()
                .unwrap();
            assert_eq!(menu_icon.uri_source(), source.uri_source());
            source.set_show_as_monochrome(true);
            assert!(!menu_icon.show_as_monochrome());
            assert!(!generated.show_as_monochrome());
            assert!(root.children().get(0).ptr_eq(&original));
        }
    });
}

#[test]
fn independent_bitmap_compiled_runtime_two_way_uri_flags_and_model_replacement() {
    crate::tests::run(|| {
        use ferroui_base::{
            media::SolidColorBrush,
            utilities::{Uri, UriKind},
        };
        use std::rc::Rc;
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("BitmapBindings.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/BitmapBindings.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let icon = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<BitmapIcon>()
                .unwrap();
            let source = BitmapIconSource::new();
            let source_weak = source.downgrade();
            source.set_uri_source(Some(
                Uri::new("ferres://bitmap_bindings/first.png", UriKind::Absolute).unwrap(),
            ));
            source.set_show_as_monochrome(false);
            source.set_foreground(Some(SolidColorBrush::from_uint32(0xffff0000).into()));
            icon.set_data_context(Some(Rc::new(source.clone())));
            assert_eq!(icon.uri_source(), source.uri_source());
            assert!(!icon.show_as_monochrome());
            assert_eq!(icon.foreground(), source.foreground());
            icon.set_show_as_monochrome(true);
            assert!(source.show_as_monochrome());
            icon.set_uri_source(None);
            assert!(source.uri_source().is_none());
            source.set_uri_source(Some(
                Uri::new("ferres://bitmap_bindings/second.png", UriKind::Absolute).unwrap(),
            ));
            assert_eq!(icon.uri_source(), source.uri_source());
            let replacement = BitmapIconSource::new();
            replacement.set_show_as_monochrome(false);
            icon.set_data_context(Some(Rc::new(replacement.clone())));
            drop(source);
            assert!(
                source_weak.upgrade().is_none(),
                "replaced source subscriptions release the model"
            );
            assert!(icon.uri_source().is_none());
            assert!(!icon.show_as_monochrome());
            replacement.set_show_as_monochrome(true);
            assert!(icon.show_as_monochrome());
            icon.set_data_context(None);
        }
    });
}
