use ferroribbon::controls::RibbonColorPalette;
use ferroui_base::{
    Rect, Size, TypeInfo, data::core::ValueTypes, media::Color, threading::Dispatcher,
};
use ferroui_controls::{Button, Control, StackPanel, TextBlock, Window};
use std::{collections::HashMap, rc::Rc};
#[test]
fn independent_palette_compiled_runtime_collections_events_bindings_styles_and_focus() {
    crate::tests::run_standalone(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        TypeInfo::register_namespaces(&[(
            "ferroribbon_markup_consumer::palette_consumer",
            "FerroRibbon.Consumer",
        )]);
        TypeInfo::register_all(&[
            crate::palette_consumer::ConsumerPalette::TYPE,
            crate::palette_consumer::PaletteView::TYPE,
        ]);
        for runtime in [false, true] {
            let v = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("ColorPalette.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/ColorPalette.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let view = ValueTypes::as_object(&*v)
                .unwrap()
                .cast::<crate::palette_consumer::PaletteView>()
                .unwrap();
            let panel = view.clone().upcast::<StackPanel>();
            let p = panel
                .children()
                .get(0)
                .cast::<crate::palette_consumer::ConsumerPalette>()
                .unwrap();
            let w = Window::new();
            w.set_content(Some(Control::boxed(view.clone())));
            w.show();
            w.measure(Size::new(600., 600.));
            w.arrange(Rect::new(0., 0., 600., 600.));
            Dispatcher::ui_thread().run_jobs(None);
            assert!(
                ferroui_controls::Application::current()
                    .unwrap()
                    .resources()
                    .try_get_resource(&"RibbonSwatchButtonStyle".into(), None)
                    .is_some(),
                "standalone application must own the automatically installed swatch style; a source static theme cache from an earlier application must not leak into this consumer scenario"
            );
            assert_eq!(p.style_key(), RibbonColorPalette::TYPE);
            assert_eq!(p.theme_colors().count(), 2);
            assert_eq!(p.standard_colors().count(), 1);
            assert_eq!(p.selected_color(), Some(Color::from_argb(128, 1, 2, 3)));
            assert!(p.focus_selected());
            let swatches = p
                .get_visual_descendants()
                .filter_map(|c| c.cast::<Button>())
                .filter(|b| {
                    b.tag()
                        .as_ref()
                        .is_some_and(|v| v.downcast_ref::<Color>().is_some())
                })
                .collect::<Vec<_>>();
            assert_eq!(swatches.len(), 13);
            let selected = swatches
                .iter()
                .find(|b| {
                    b.tag().unwrap().downcast_ref::<Color>()
                        == Some(&Color::from_argb(128, 1, 2, 3))
                })
                .unwrap();
            assert!(selected.is_focused());
            let ring = selected
                .get_visual_descendants()
                .find(|c| c.name().as_deref() == Some("SelectedRing"))
                .unwrap();
            assert_eq!(ring.opacity(), 1.);
            p.select(Some(Color::from_argb(128, 9, 2, 3)));
            assert_eq!(
                view.selections(),
                vec![Some(Color::from_argb(128, 9, 2, 3))]
            );
            assert_eq!(
                panel
                    .children()
                    .get(1)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("1")
            );
            let names = p.color_names();
            p.set_color_name(Color::from_argb(128, 1, 2, 3), Some("Custom accent".into()));
            assert!(Rc::ptr_eq(&names, &p.color_names()));
            assert_eq!(names.len(), 1);
            assert_eq!(
                names.get(&Color::from_argb(128, 1, 2, 3)).as_deref(),
                Some("Custom accent")
            );
            let snapshot: HashMap<Color, String> = names.snapshot();
            assert_eq!(snapshot.len(), 1);
            assert_eq!(
                panel
                    .children()
                    .get(2)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("0"),
                "source Dictionary does not notify Count"
            );
            assert_eq!(
                panel
                    .children()
                    .get(3)
                    .cast::<TextBlock>()
                    .unwrap()
                    .text()
                    .as_deref(),
                Some("0"),
                "typed compiled ColorNames.Count binding keeps the same non-notifying contract"
            );
            let more = p
                .get_visual_descendants()
                .filter_map(|c| c.cast::<Button>())
                .filter(|b| b.tag().is_none())
                .last()
                .unwrap();
            more.on_click();
            assert_eq!(view.more_calls(), 1);
            assert!(view.previews().is_empty());
            w.close();
            w.set_content(None);
        }
    });
}
