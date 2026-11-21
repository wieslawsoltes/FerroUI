// Copyright (c) 2026 Wiesław Šoltés. MIT. Native view of the original PowerPointPage.
use crate::{
    demo_settings::{DemoSettings, DemoSubscription},
    office_view_models::{PowerPointAction, PowerPointViewModel, ThemeApplication},
};
use ferroribbon::controls::{
    Ribbon, RibbonGallery, RibbonGalleryItem, RibbonGalleryItemEventArgs,
    RibbonItemInvokedEventArgs, RibbonTitleBar,
};
use ferroui_base::{
    FerroObject, FerroObjectExtensions, FerroObjectImpl, FerroProperty, Ref, StyledElementImpl,
    StyledProperty, VisualImpl,
    data::core::{ValueType, ValueTypes},
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    media::{Color, IBrush, SolidColorBrush},
    metadata::{MarkupDelegate, from_markup_value},
    reactive::AnonymousObserver,
};
use ferroui_controls::{
    Application, ContentControlImpl, Control, ControlImpl, TextBlock, UserControl,
    primitives::TemplatedControlImpl,
};
use std::{
    cell::{OnceCell, RefCell},
    rc::Rc,
};
#[repr(C)]
pub struct PowerPointPage {
    base: UserControl,
    model: OnceCell<Rc<PowerPointViewModel>>,
    settings: OnceCell<Rc<DemoSettings>>,
    cleanup: RefCell<Vec<Box<dyn FnOnce()>>>,
    settings_subscription: RefCell<Option<DemoSubscription>>,
}
ferro_class!(PowerPointPage:UserControl);
ferro_impl_classes!(PowerPointPage:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl);
ferro_class_info!(PowerPointPage{new:PowerPointPage::new,markup:{namespace:"FerroRibbon.Sample.Pages"}});
ferro_properties! {impl PowerPointPage{
 pub fn slide_paint_property()->StyledProperty<Option<Rc<dyn IBrush>>>{FerroProperty::register::<Self,_>("SlidePaint",None)}
 pub fn title_paint_property()->StyledProperty<Option<Rc<dyn IBrush>>>{FerroProperty::register::<Self,_>("TitlePaint",None)}
 pub fn subtitle_paint_property()->StyledProperty<Option<Rc<dyn IBrush>>>{FerroProperty::register::<Self,_>("SubtitlePaint",None)}
 pub fn thumbnail_accent_property()->StyledProperty<Option<Rc<dyn IBrush>>>{FerroProperty::register::<Self,_>("ThumbnailAccent",None)}
}}
fn brush(argb: u32) -> Rc<dyn IBrush> {
    SolidColorBrush::with_color(Color::from_argb(
        (argb >> 24) as u8,
        (argb >> 16) as u8,
        (argb >> 8) as u8,
        argb as u8,
    ))
    .into()
}
impl PowerPointPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            model: OnceCell::new(),
            settings: OnceCell::new(),
            cleanup: RefCell::new(Vec::new()),
            settings_subscription: RefCell::new(None),
        }
    }
    pub fn new() -> Ref<Self> {
        Self::new_with_settings(DemoSettings::current())
    }
    pub fn new_with_settings(settings: Rc<DemoSettings>) -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.set_value(Self::slide_paint_property(), Some(brush(0xffffffff)));
        this.set_value(Self::title_paint_property(), Some(brush(0xff262626)));
        this.set_value(Self::subtitle_paint_property(), Some(brush(0xff595959)));
        if let Some(populate) = ferroui_markup_xaml::FerroXamlLoader::populate_override(Self::TYPE)
        {
            populate(None, &Control::boxed(this.clone())).expect("PowerPoint populate override");
        } else {
            crate::generated::compiled_powerpoint_page::populate(None, &this)
                .expect("compiled source PowerPointPage");
        }
        this.named::<RibbonTitleBar>("TitleBar")
            .set_ribbon(Some(this.ribbon()));
        let accent = Application::current()
            .expect("PowerPoint Application")
            .resources()
            .try_get_resource(&"RibbonAccentBrush".into(), None)
            .flatten()
            .expect("source Application RibbonAccentBrush");
        let accent = ValueTypes::try_cast(&accent, ValueType::of::<Rc<dyn IBrush>>())
            .and_then(|v| v.downcast_ref::<Rc<dyn IBrush>>().cloned())
            .expect("source RibbonAccentBrush must be a brush");
        this.set_value(Self::thumbnail_accent_property(), Some(accent));
        let weak_settings = Rc::downgrade(&settings);
        let weak = this.downgrade();
        let vm = PowerPointViewModel::new(
            Rc::new(move |line| {
                weak_settings
                    .upgrade()
                    .ok_or("PowerPoint settings owner is gone")?
                    .write(Some(line))
                    .map_err(|e| e.to_string())
            }),
            Rc::new(move || {
                let page = weak.upgrade().ok_or("PowerPoint view is gone")?;
                page.apply_theme(None);
                Ok(())
            }),
        );
        assert!(this.model.set(vm.clone()).is_ok());
        assert!(this.settings.set(settings.clone()).is_ok());
        this.set_data_context(Some(vm));
        // Source logs Status.Text after assigning it. A nested native text setter
        // can run while the binding suppresses its own target-to-source feedback.
        // Observe that concrete control value synchronously so the model's late
        // read retains the source contract, including nested setter changes.
        let weak = Rc::downgrade(&this.view_model());
        let status = this.named::<TextBlock>("Status");
        let subscription = FerroObjectExtensions::get_observable(
            &*status.upcast::<FerroObject>(),
            TextBlock::text_property(),
        )
        .subscribe(Rc::new(AnonymousObserver::new(
            move |text: Option<String>| {
                if let Some(model) = weak.upgrade() {
                    model.set_status(text);
                }
            },
        )));
        this.cleanup
            .borrow_mut()
            .push(Box::new(move || subscription.dispose()));
        macro_rules! hook {
            ($part:expr,$add:ident,$remove:ident,$make:expr) => {{
                let part = ($part).clone();
                let weak = this.downgrade();
                let handler = MarkupDelegate::new(move |args| {
                    if let Some(page) = weak.upgrade() {
                        let action = ($make)(&page, args);
                        page.view_model().action().execute(Some(&Some(action)));
                    }
                    None
                });
                part.$add(Some(handler.clone()));
                let weak = part.downgrade();
                this.cleanup.borrow_mut().push(Box::new(move || {
                    if let Some(part) = weak.upgrade() {
                        part.$remove(Some(handler));
                    }
                }));
            }};
        }
        hook!(
            this.named::<RibbonGallery>("Themes"),
            add_item_preview,
            remove_item_preview,
            |page: &PowerPointPage, args: &[ferroui_base::metadata::MarkupValue]| {
                let e = from_markup_value::<Rc<RibbonGalleryItemEventArgs>>(&args[1])
                    .expect("theme preview");
                PowerPointAction::Preview(page.theme_application(e.item()))
            }
        );
        hook!(
            this.named::<RibbonGallery>("Themes"),
            add_item_click,
            remove_item_click,
            |page: &PowerPointPage, args: &[ferroui_base::metadata::MarkupValue]| {
                let e = from_markup_value::<Rc<RibbonGalleryItemEventArgs>>(&args[1])
                    .expect("theme click");
                PowerPointAction::Pick(page.theme_application(e.item()))
            }
        );
        hook!(
            this.ribbon(),
            add_item_invoked,
            remove_item_invoked,
            |_: &PowerPointPage, args: &[ferroui_base::metadata::MarkupValue]| {
                let e = from_markup_value::<Rc<RibbonItemInvokedEventArgs>>(&args[1])
                    .expect("PowerPoint invocation");
                PowerPointAction::Invoked {
                    item_id: e
                        .item_id
                        .as_ref()
                        .map(|s| s.to_utf8().expect("native PowerPoint ID")),
                    parameter: e.parameter.as_ref().map(|value| {
                        let value = value.clone();
                        Rc::new(move || powerpoint_parameter_text(&value))
                            as crate::office_view_models::ParameterText
                    }),
                }
            }
        );
        let weak = this.downgrade();
        let shared = Rc::downgrade(&settings);
        this.settings_subscription
            .replace(Some(settings.changed(move || {
                if let Some(page) = weak.upgrade()
                    && let Some(settings) = shared.upgrade()
                {
                    let ribbon = page.ribbon();
                    ribbon.set_density(settings.density());
                    if let Some(mode) = settings.display_mode() {
                        ribbon.set_display_mode(mode);
                    }
                }
            })));
        // Unlike Word, source PowerPoint subscribes without eagerly applying shared settings.
        this
    }
    pub(super) fn theme_application(
        &self,
        item: Option<ferroui_base::BoxedValue>,
    ) -> Option<ThemeApplication> {
        let item = item
            .and_then(|v| ValueTypes::as_object(&*v))
            .and_then(|o| o.cast::<RibbonGalleryItem>())?;
        let weak = self.to_ref().downgrade();
        Some(Rc::new(move || {
            let page = weak.upgrade().ok_or("PowerPoint view is gone")?;
            page.apply_theme(Some(&item));
            Ok(())
        }))
    }
    fn apply_theme(&self, theme: Option<&RibbonGalleryItem>) {
        self.set_value(
            Self::slide_paint_property(),
            Some(
                theme
                    .and_then(|t| t.preview_background())
                    .unwrap_or_else(|| brush(0xffffffff)),
            ),
        );
        let foreground = theme
            .and_then(|t| t.preview_foreground())
            .unwrap_or_else(|| brush(0xff262626));
        self.set_value(Self::title_paint_property(), Some(foreground.clone()));
        self.set_value(Self::subtitle_paint_property(), Some(foreground));
    }
    pub fn ribbon(&self) -> Ref<Ribbon> {
        self.named::<Ribbon>("Ribbon")
    }
    pub fn view_model(&self) -> Rc<PowerPointViewModel> {
        self.model.get().expect("PowerPoint initialized").clone()
    }
    pub(crate) fn named<T: ferroui_base::ObjectType>(&self, name: &str) -> Ref<T> {
        ferroui_base::controls::NameScopeExtensions::find::<T>(self, name)
            .unwrap_or_else(|| panic!("Missing PowerPoint element {name}"))
    }
}
impl Drop for PowerPointPage {
    fn drop(&mut self) {
        self.settings_subscription.get_mut().take();
        for cleanup in self.cleanup.get_mut().drain(..) {
            cleanup();
        }
    }
}

// Original PowerPoint logs its default gallery item's object display. Preserve
// that concrete product string across the documented namespace migration.
fn powerpoint_parameter_text(value: &ferroui_base::BoxedValue) -> Result<String, String> {
    if ValueTypes::as_object(&**value).is_some_and(|o| o.get_type() == RibbonGalleryItem::TYPE) {
        return Ok("RibbonSpace.Controls.RibbonGalleryItem".into());
    }
    super::word_page::word_parameter_text(value)
}
