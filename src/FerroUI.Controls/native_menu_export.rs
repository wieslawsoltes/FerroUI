use crate::platform::{as_native_menu_exporter_provider, ITopLevelNativeMenuExporter};
use crate::{NativeMenu, TopLevel};
use ferroui_base::platform::IOptionalFeatureProvider;
use ferroui_base::{ferro_properties, AttachedProperty, FerroObject, FerroProperty, Ref};
use std::cell::Cell;
use std::rc::Rc;

/// What the native menu support keeps for a top-level.
struct NativeMenuInfo {
    changing_is_exported: Cell<bool>,
    exporter: Option<Rc<dyn ITopLevelNativeMenuExporter>>,
}

impl NativeMenuInfo {
    fn new(target: &TopLevel) -> Rc<Self> {
        let exporter = target.platform_impl().and_then(|platform_impl| {
            let provider: &dyn IOptionalFeatureProvider = &*platform_impl;
            provider.try_get::<dyn ITopLevelNativeMenuExporter>()
        });
        if let Some(exporter) = &exporter {
            // Both are held weakly: the exporter belongs to the platform
            // implementation of the top-level.
            let target = target.to_ref().downgrade();
            let weak_exporter = Rc::downgrade(exporter);
            let _ = exporter.on_is_native_menu_exported_changed(Rc::new(move || {
                if let (Some(target), Some(exporter)) = (target.upgrade(), weak_exporter.upgrade()) {
                    NativeMenu::set_is_native_menu_exported(&target, exporter.is_native_menu_exported());
                }
            }));
        }
        Rc::new(Self { changing_is_exported: Cell::new(false), exporter })
    }
}

/// The value of the private attached property that holds a
/// [`NativeMenuInfo`]; compares by reference.
#[derive(Clone)]
struct NativeMenuInfoRef(Rc<NativeMenuInfo>);

impl PartialEq for NativeMenuInfoRef {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

ferro_properties! {
    impl NativeMenu, fn register_export_properties {
        /// Defines the `IsNativeMenuExported` attached property.
        pub fn is_native_menu_exported_property() -> AttachedProperty<bool> {
            FerroProperty::register_attached::<NativeMenu, TopLevel, _>("IsNativeMenuExported", false)
        }

        fn native_menu_info_property() -> AttachedProperty<Option<NativeMenuInfoRef>> {
            FerroProperty::register_attached::<NativeMenu, TopLevel, _>("___NativeMenuInfo", None)
        }

        /// Defines the `Menu` attached property.
        pub fn menu_property() -> AttachedProperty<Option<Ref<NativeMenu>>> {
            FerroProperty::register_attached::<NativeMenu, FerroObject, _>("Menu", None)
        }
    }
}

impl NativeMenu {
    /// Whether the native menu of the top-level is exported to the
    /// platform.
    pub fn get_is_native_menu_exported(tl: &TopLevel) -> bool {
        tl.get_value(Self::is_native_menu_exported_property())
    }

    fn get_info(target: &TopLevel) -> Rc<NativeMenuInfo> {
        match target.get_value(Self::native_menu_info_property()) {
            Some(rv) => rv.0,
            None => {
                let rv = NativeMenuInfo::new(target);
                target.set_value(Self::native_menu_info_property(), Some(NativeMenuInfoRef(rv.clone())));
                Self::set_is_native_menu_exported(
                    target,
                    rv.exporter.as_ref().is_some_and(|exporter| exporter.is_native_menu_exported()),
                );
                rv
            }
        }
    }

    fn set_is_native_menu_exported(tl: &TopLevel, value: bool) {
        Self::get_info(tl).changing_is_exported.set(true);
        tl.set_value(Self::is_native_menu_exported_property(), value);
    }

    /// Sets the native menu of an object: a top-level, the application or
    /// an object that provides a native menu exporter.
    pub fn set_menu(o: &FerroObject, menu: Option<Ref<NativeMenu>>) {
        o.set_value(Self::menu_property(), menu)
    }

    /// Gets the native menu of an object.
    pub fn get_menu(o: &FerroObject) -> Option<Ref<NativeMenu>> {
        o.get_value(Self::menu_property())
    }

    /// The part of the static constructor that belongs to this file.
    pub(crate) fn static_constructor_export() {
        // This is needed because of the lack of attached direct properties
        Self::is_native_menu_exported_property().changed().subscribe(|args| {
            let sender = args.sender().downcast_ref::<TopLevel>().expect("the sender is a top-level");
            let info = Self::get_info(sender);
            if !info.changing_is_exported.get() {
                panic!("IsNativeMenuExported property is read-only");
            }
            info.changing_is_exported.set(false);
        });
        Self::menu_property().changed().subscribe(|args| {
            let new_value = args.get_new_value::<Option<Ref<NativeMenu>>>();
            if let Some(tl) = args.sender().downcast_ref::<TopLevel>() {
                if let Some(exporter) = &Self::get_info(tl).exporter {
                    exporter.set_native_menu(new_value);
                }
            } else if let Some(provider) = as_native_menu_exporter_provider(args.sender()) {
                if let Some(exporter) = provider.native_menu_exporter() {
                    exporter.set_native_menu(new_value);
                }
            }
        });
    }
}
