// Copyright (c) 2026 Wiesław Šoltés. MIT. Passive native view of the original SettingsPage.
use super::settings_services::NativeSettingsServices;
use crate::{
    demo_settings::{DemoSettings, DemoSubscription},
    settings_view_model::{SettingsSelection, SettingsViewModel},
};
use ferroui_base::{
    FerroObjectImpl, Ref, StyledElementImpl, VisualImpl, ferro_class, ferro_class_info,
    ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::{InteractiveImpl, RoutedEventHandlerToken},
    layout::LayoutableImpl,
};
use ferroui_controls::{
    ComboBox, ContentControlImpl, Control, ControlImpl, UserControl,
    primitives::{SelectingItemsControl, TemplatedControlImpl},
};
use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
};
#[repr(C)]
pub struct SettingsPage {
    base: UserControl,
    model: OnceCell<Rc<SettingsViewModel>>,
    subscriptions: RefCell<Vec<(Ref<ComboBox>, RoutedEventHandlerToken)>>,
    log_subscription: RefCell<Option<DemoSubscription>>,
}
ferro_class!(SettingsPage:UserControl);
ferro_class_info!(SettingsPage{new:SettingsPage::new,markup:{namespace:"FerroRibbon.Sample.Pages"}});
ferro_impl_classes!(SettingsPage:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
impl SettingsPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            model: OnceCell::new(),
            subscriptions: RefCell::new(Vec::new()),
            log_subscription: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        Self::new_with_settings(DemoSettings::current())
    }
    pub fn new_with_settings(settings: Rc<DemoSettings>) -> Ref<Self> {
        let this = instantiate(Self::construct());
        if let Some(populate) = ferroui_markup_xaml::FerroXamlLoader::populate_override(Self::TYPE)
        {
            populate(None, &Control::boxed(this.clone())).expect("SettingsPage populate override");
        } else {
            crate::generated::compiled_settings_page::populate(None, &this)
                .expect("compiled source SettingsPage");
        }
        let palette = this.named::<ComboBox>("Palette");
        palette
            .items()
            .add(Some(Rc::new("Per application (default)".to_owned())));
        for preset in ferroribbon_core::theming::RibbonThemePalette::presets().iter() {
            palette.items().add(preset.name.as_ref().map(|name| {
                Rc::new(name.to_utf8().expect("Palette name requires native text"))
                    as ferroui_base::BoxedValue
            }));
        }
        // Source sets the index before installing callbacks. Opening Settings must
        // not reset the active app palette or eagerly apply all picker defaults.
        palette.set_selected_index(0);
        let service = Rc::new(NativeSettingsServices {
            page: this.downgrade(),
            settings: settings.clone(),
        });
        let model = SettingsViewModel::new(service);
        assert!(this.model.set(model.clone()).is_ok());
        this.set_data_context(Some(model.clone()));
        for name in [
            "Theme",
            "Palette",
            "Chrome",
            "Surface",
            "Density",
            "Layout",
            "LanguageBox",
        ] {
            let combo = this.named::<ComboBox>(name);
            let weak = this.downgrade();
            let token = combo.selection_changed(move |_, _| {
                if let Some(page) = weak.upgrade() {
                    let selected = page.named::<ComboBox>(name);
                    let index = selected.selected_index();
                    let selection = match name {
                        "Theme" => SettingsSelection::Theme(index),
                        "Palette" => SettingsSelection::Palette(index),
                        "Chrome" => SettingsSelection::Chrome(index),
                        "Surface" => SettingsSelection::Surface(index),
                        "Density" | "Layout" => SettingsSelection::Layout {
                            density_index: page.named::<ComboBox>("Density").selected_index(),
                            layout_index: page.named::<ComboBox>("Layout").selected_index(),
                        },
                        "LanguageBox" => {
                            SettingsSelection::Language(selected.selected_item().map(|value| {
                                value
                                    .downcast_ref::<String>()
                                    .expect("Source language items are strings")
                                    .clone()
                            }))
                        }
                        _ => unreachable!(),
                    };
                    page.view_model()
                        .selection_action()
                        .execute(Some(&Some(selection)));
                }
            });
            this.subscriptions.borrow_mut().push((combo, token));
        }
        let weak = Rc::downgrade(&model);
        this.log_subscription
            .replace(Some(settings.logged(move |_| {
                if let Some(model) = weak.upgrade() {
                    model.refresh_action().execute(None);
                }
            })));
        let weak = Rc::downgrade(&model);
        this.loaded(move |_, _| {
            if let Some(model) = weak.upgrade() {
                model.refresh_action().execute(None);
            }
        });
        this
    }
    pub fn view_model(&self) -> Rc<SettingsViewModel> {
        self.model.get().expect("Settings initialized").clone()
    }
    pub(crate) fn named<T: ferroui_base::ObjectType>(&self, name: &str) -> Ref<T> {
        ferroui_base::controls::NameScopeExtensions::find::<T>(self, name)
            .unwrap_or_else(|| panic!("Missing source Settings element {name}"))
    }
}
impl Drop for SettingsPage {
    fn drop(&mut self) {
        self.log_subscription.get_mut().take();
        for (combo, token) in self.subscriptions.get_mut().drain(..) {
            combo.remove_handler(SelectingItemsControl::selection_changed_event(), token);
        }
    }
}
