//! Independent consumer: the same compiled/runtime declared controls feed the
//! public Rust customization dialog; no internal constructors or runtime facade.
use ferroribbon::controls::{Ribbon, RibbonCustomizeDialog, RibbonCustomizePage};
use ferroribbon_core::RibbonQuickAccessPosition;
use ferroui_base::data::core::ValueTypes;
use ferroui_controls::{Control, Window};
use std::{cell::Cell, rc::Rc};
#[test]
fn independent_customization_uses_source_markup_native_properties_and_public_rust_api() {
    crate::tests::run_standalone(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("Customization.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/Customization.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let ribbon = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<Ribbon>()
                .unwrap();
            assert!(!ribbon.is_simplified_mode_available());
            assert_eq!(
                ribbon
                    .tabs()
                    .get(0)
                    .unwrap()
                    .groups()
                    .get(0)
                    .unwrap()
                    .items()
                    .count(),
                1
            );
            let window = Window::new();
            window.set_width(1000.);
            window.set_height(800.);
            window.set_content(Some(Control::boxed(ribbon.clone())));
            window.show();
            window.update_layout();
            let dialog =
                RibbonCustomizeDialog::new(ribbon.clone(), RibbonCustomizePage::QuickAccessToolbar)
                    .unwrap();
            let state = dialog.working_state();
            assert!(Rc::ptr_eq(&state, &dialog.working_state()));
            state.set_quick_access_position(RibbonQuickAccessPosition::BelowRibbon);
            let closed = Rc::new(Cell::new(None));
            let seen = closed.clone();
            let subscription = dialog.closed(move |primary| seen.set(Some(primary)));
            dialog.show(window.clone().upcast()).unwrap();
            assert!(dialog.is_open());
            dialog.close(true).unwrap();
            assert_eq!(closed.get(), Some(true));
            assert_eq!(
                ribbon.quick_access_position(),
                RibbonQuickAccessPosition::BelowRibbon
            );
            subscription.dispose();
            let calls = Rc::new(Cell::new(0));
            let seen = calls.clone();
            let subscription = ribbon.customize_requested(move |args| {
                seen.set(seen.get() + 1);
                args.handled.set(true);
            });
            assert!(
                ribbon
                    .show_customize_dialog(RibbonCustomizePage::Ribbon)
                    .unwrap()
                    .is_none()
            );
            assert_eq!(calls.get(), 1);
            subscription.dispose();
            window.close();
            window.set_content(None);
        }
    });
}
