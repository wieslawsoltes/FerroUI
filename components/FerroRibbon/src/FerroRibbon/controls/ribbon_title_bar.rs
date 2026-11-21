//! RibbonTitleBar UI state/ownership port. Copyright Wiesław Šoltés, MIT. Native window
//! attachment.
mod window_chrome;
use super::{Ribbon, RibbonHandledEventArgs, RibbonQuickAccessToolBar, RibbonSearchBox};
use crate::theming::RibbonTheme;
use ferroribbon_core::{
    RibbonQuickAccessPosition,
    model::{RibbonModelExt, RibbonModelImpl},
};
use ferroui_base::{
    BoxedValue, ElementRef, FerroObjectExtensions, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, Ref, StyledElementImpl, StyledProperty, Thickness, TypeInfo, VisualImpl,
    data::{BindingPriority, core::ValueTypes},
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{InputElementImpl, KeyboardNavigation},
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    metadata::MarkupDelegate,
    reactive::{Disposable, IDisposable},
    utilities::{
        HandlerList,
        math_utilities::{max, min},
    },
};
use ferroui_controls::{
    Border, Button, Control, ControlImpl, ControlImplExt, ToolTip,
    automation::AutomationProperties,
    primitives::{
        TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
pub use window_chrome::{RibbonTitleBarWindowError, RibbonTitleBarWindowState};

#[derive(Default)]
struct Parts {
    quick_access: Option<Ref<Border>>,
    search: Option<Ref<RibbonSearchBox>>,
    title: Option<Ref<Control>>,
    app_button: Option<Ref<Button>>,
    app_icon: Option<Ref<Control>>,
    drag: Option<Ref<Control>>,
    start: Option<Ref<Control>>,
    end: Option<Ref<Control>>,
}
type AppClickHandler = dyn Fn(&RibbonTitleBar, &Rc<RibbonHandledEventArgs>);
#[repr(C)]
pub struct RibbonTitleBar {
    base: TemplatedControl,
    parts: RefCell<Parts>,
    hosted_quick_access: RefCell<Option<Ref<RibbonQuickAccessToolBar>>>,
    foreground_binding: RefCell<Option<Rc<dyn IDisposable>>>,
    ribbon_subscription: RefCell<Option<MarkupDelegate>>,
    model_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    watched_model: RefCell<Option<Rc<dyn RibbonModelImpl>>>,
    model_token: Cell<Option<u64>>,
    part_subscription: RefCell<Option<ferroui_base::reactive::Subscription>>,
    invoked: HandlerList<AppClickHandler>,
    markup_invoked: RefCell<Vec<(MarkupDelegate, u64)>>,
    window_attachment: RefCell<Option<window_chrome::WindowAttachment>>,
    window_binding: RefCell<Option<window_chrome::WindowBinding>>,
    window_state: Cell<RibbonTitleBarWindowState>,
    window_pending: Cell<usize>,
}
ferro_class!(RibbonTitleBar:TemplatedControl);
ferro_class_info!(RibbonTitleBar{new:RibbonTitleBar::new,markup:{namespace:"FerroRibbon.Controls",properties:[
    Ribbon:Option<Ref<Ribbon>>{get:RibbonTitleBar::ribbon,set:|c:&RibbonTitleBar,v|c.set_ribbon(v)},
    DisplayTitle:Option<String>{get:RibbonTitleBar::display_title},
    AppButton:Option<Ref<Button>>{get:RibbonTitleBar::app_button},
    SearchBox:Option<Ref<RibbonSearchBox>>{get:RibbonTitleBar::search_box}
],events:[AppIconClick(Option<BoxedValue>,Rc<RibbonHandledEventArgs>){add:RibbonTitleBar::add_app_icon_click,remove:RibbonTitleBar::remove_app_icon_click}]}});
ferro_impl_classes!(RibbonTitleBar:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl);
ferro_properties! {impl RibbonTitleBar{
    pub fn ribbon_property()->StyledProperty<Option<ElementRef<Ribbon>>>{ValueTypes::register_element_ref::<Ribbon>();FerroProperty::register::<Self,_>("Ribbon",None)}
    pub fn title_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Title",None)}
    pub fn display_title_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("DisplayTitle",None)}
    pub fn subtitle_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("Subtitle",None)}
    pub fn is_app_icon_menu_enabled_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsAppIconMenuEnabled",false)}
    pub fn app_icon_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("AppIcon",None)}
    pub fn start_content_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("StartContent",None)}
    pub fn end_content_property()->StyledProperty<Option<BoxedValue>>{FerroProperty::register::<Self,_>("EndContent",None)}
    pub fn is_search_visible_property()->StyledProperty<bool>{FerroProperty::register::<Self,_>("IsSearchVisible",true)}
    pub fn caption_buttons_inset_property()->StyledProperty<Thickness>{FerroProperty::register::<Self,_>("CaptionButtonsInset",Thickness::default())}
    pub fn search_width_property()->StyledProperty<f64>{FerroProperty::register::<Self,_>("SearchWidth",340.)}
}}
impl RibbonTitleBar {
    fn static_constructor() {
        Self::ribbon_property()
            .changed()
            .add_class_handler::<Self>(|c, e| {
                let (old, new) = e.get_old_and_new_value::<Option<ElementRef<Ribbon>>>();
                c.on_ribbon_changed(ElementRef::resolve(&old), ElementRef::resolve(&new));
            })
            .detach();
        Self::title_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.update_display_title())
            .detach();
        Self::is_app_icon_menu_enabled_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.update_app_button())
            .detach();
        Self::is_search_visible_property()
            .changed()
            .add_class_handler::<Self>(|c, _| c.update_adaptive())
            .detach();
    }
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            parts: RefCell::new(Parts::default()),
            hosted_quick_access: RefCell::new(None),
            foreground_binding: RefCell::new(None),
            ribbon_subscription: RefCell::new(None),
            model_subscription: RefCell::new(None),
            watched_model: RefCell::new(None),
            model_token: Cell::new(None),
            part_subscription: RefCell::new(None),
            invoked: HandlerList::new(),
            markup_invoked: RefCell::new(vec![]),
            window_attachment: RefCell::new(None),
            window_binding: RefCell::new(None),
            window_state: Cell::new(RibbonTitleBarWindowState::Detached),
            window_pending: Cell::new(0),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn ribbon(&self) -> Option<Ref<Ribbon>> {
        ElementRef::resolve(&self.get_value(Self::ribbon_property()))
    }
    pub fn set_ribbon(&self, v: Option<Ref<Ribbon>>) {
        self.set_value(Self::ribbon_property(), ElementRef::from_nullable(v));
    }
    pub fn title(&self) -> Option<String> {
        self.get_value(Self::title_property())
    }
    pub fn set_title(&self, v: Option<String>) {
        self.set_value(Self::title_property(), v);
    }
    pub fn display_title(&self) -> Option<String> {
        self.get_value(Self::display_title_property())
    }
    pub fn subtitle(&self) -> Option<String> {
        self.get_value(Self::subtitle_property())
    }
    pub fn set_subtitle(&self, v: Option<String>) {
        self.set_value(Self::subtitle_property(), v);
    }
    pub fn is_app_icon_menu_enabled(&self) -> bool {
        self.get_value(Self::is_app_icon_menu_enabled_property())
    }
    pub fn set_is_app_icon_menu_enabled(&self, v: bool) {
        self.set_value(Self::is_app_icon_menu_enabled_property(), v);
    }
    pub fn app_icon(&self) -> Option<BoxedValue> {
        self.get_value(Self::app_icon_property())
    }
    pub fn set_app_icon(&self, v: Option<BoxedValue>) {
        self.set_value(Self::app_icon_property(), v);
    }
    pub fn start_content(&self) -> Option<BoxedValue> {
        self.get_value(Self::start_content_property())
    }
    pub fn set_start_content(&self, v: Option<BoxedValue>) {
        self.set_value(Self::start_content_property(), v);
    }
    pub fn end_content(&self) -> Option<BoxedValue> {
        self.get_value(Self::end_content_property())
    }
    pub fn set_end_content(&self, v: Option<BoxedValue>) {
        self.set_value(Self::end_content_property(), v);
    }
    pub fn is_search_visible(&self) -> bool {
        self.get_value(Self::is_search_visible_property())
    }
    pub fn set_is_search_visible(&self, v: bool) {
        self.set_value(Self::is_search_visible_property(), v);
    }
    pub fn caption_buttons_inset(&self) -> Thickness {
        self.get_value(Self::caption_buttons_inset_property())
    }
    pub fn set_caption_buttons_inset(&self, v: Thickness) {
        self.set_value(Self::caption_buttons_inset_property(), v);
    }
    pub fn search_width(&self) -> f64 {
        self.get_value(Self::search_width_property())
    }
    pub fn set_search_width(&self, v: f64) {
        self.set_value(Self::search_width_property(), v);
    }
    pub fn app_button(&self) -> Option<Ref<Button>> {
        self.parts.borrow().app_button.clone()
    }
    pub fn search_box(&self) -> Option<Ref<RibbonSearchBox>> {
        self.parts.borrow().search.clone()
    }
    pub fn app_icon_click(
        &self,
        h: impl Fn(&Self, &Rc<RibbonHandledEventArgs>) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.invoked.add(Rc::new(h));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(c) = weak.upgrade() {
                c.invoked.remove(token);
            }
        })
    }
    pub fn add_app_icon_click(&self, h: Option<MarkupDelegate>) {
        let Some(h) = h else {
            return;
        };
        let handler = h.clone();
        let token = self.invoked.add(Rc::new(move |c, args| {
            handler
                .try_invoke(&[
                    Some(Control::boxed(c.to_ref())),
                    Some(Rc::new(args.clone())),
                ])
                .expect("Title bar AppIconClick XAML handler");
        }));
        self.markup_invoked.borrow_mut().push((h, token));
    }
    pub fn remove_app_icon_click(&self, h: Option<MarkupDelegate>) {
        let removed = {
            let mut hs = self.markup_invoked.borrow_mut();
            hs.iter()
                .rposition(|(v, _)| Some(v) == h.as_ref())
                .map(|i| hs.remove(i))
        };
        if let Some((_, token)) = removed {
            self.invoked.remove(token);
        }
    }
    fn update_app_button(&self) {
        if let Some(button) = self.app_button() {
            button.set_is_visible(self.is_app_icon_menu_enabled());
            let name = self
                .ribbon()
                .and_then(|r| r.application_button_label())
                .or_else(|| {
                    ferroribbon_core::localization::RibbonStrings::current()
                        .file()
                        .map(|v| String::from_utf16_lossy(v.as_utf16()))
                });
            AutomationProperties::set_name(&button, name.as_deref());
            ToolTip::set_tip(&button, name.map(|v| Rc::new(v) as BoxedValue));
        }
        let icon = self.parts.borrow().app_icon.clone();
        if let Some(icon) = icon {
            icon.set_is_visible(!self.is_app_icon_menu_enabled());
        }
    }
    fn on_app_button_click(&self, anchor: Ref<Button>) {
        let args = RibbonHandledEventArgs::new();
        for (_, h) in self.invoked.snapshot().iter() {
            h(self, &args);
        }
        if !args.handled()
            && let Some(ribbon) = self.ribbon()
        {
            ribbon.invoke_application_button_at(Some(anchor.upcast()));
        }
    }
    fn on_ribbon_changed(&self, old: Option<Ref<Ribbon>>, new: Option<Ref<Ribbon>>) {
        if let Some(old) = old {
            old.remove_quick_access_changed(self.ribbon_subscription.borrow_mut().take());
            old.set_is_quick_access_hosted_externally(false);
            self.release_quick_access();
        }
        if let Some(new) = new.as_ref() {
            let weak = self.to_ref().downgrade();
            let h = MarkupDelegate::new(move |_| {
                if let Some(c) = weak.upgrade() {
                    c.host_quick_access();
                }
                None
            });
            new.add_quick_access_changed(Some(h.clone()));
            self.ribbon_subscription.replace(Some(h));
            new.set_is_quick_access_hosted_externally(true);
        }
        if let Some(search) = self.search_box() {
            search.set_ribbon(new);
        }
        self.host_quick_access();
        if self.is_loaded() {
            self.watch_model();
        } else {
            self.update_display_title();
        }
    }
    fn host_quick_access(&self) {
        let Some(host) = self.parts.borrow().quick_access.clone() else {
            return;
        };
        let ribbon = self.ribbon();
        let qat = ribbon.as_ref().and_then(|r| r.quick_access_tool_bar());
        let show = ribbon.as_ref().is_some_and(|r| {
            r.is_quick_access_visible()
                && r.quick_access_position() == RibbonQuickAccessPosition::AboveRibbon
        }) && qat.is_some();
        if !show {
            self.release_quick_access();
            if let Some(ribbon) = ribbon {
                ribbon.update_quick_access_placement();
            }
            return;
        }
        let qat = qat.unwrap();
        if self.hosted_quick_access.borrow().as_ref() != Some(&qat) {
            self.release_quick_access();
        }
        if !host.child().is_some_and(|v| v.ptr_eq(&qat)) {
            if let Some(parent) = qat.visual_parent().and_then(|v| v.cast::<Border>()) {
                RibbonQuickAccessToolBar::detach_from_border(&parent);
            }
            host.set_child(qat.clone());
        }
        self.hosted_quick_access.replace(Some(qat.clone()));
        let old = self.foreground_binding.borrow_mut().take();
        if let Some(old) = old {
            old.dispose();
        }
        let source = FerroObjectExtensions::get_observable(
            &*self.to_ref().upcast::<ferroui_base::FerroObject>(),
            TemplatedControl::foreground_property(),
        );
        let binding = qat.bind(
            TemplatedControl::foreground_property(),
            source,
            BindingPriority::LocalValue,
        );
        self.foreground_binding.replace(Some(binding));
    }
    fn release_quick_access(&self) {
        let host = self.parts.borrow().quick_access.clone();
        if let Some(host) = host
            && host
                .child()
                .is_some_and(|v| v.is::<RibbonQuickAccessToolBar>())
        {
            RibbonQuickAccessToolBar::detach_from_border(&host);
        }
        let old = self.foreground_binding.borrow_mut().take();
        if let Some(old) = old {
            old.dispose();
        }
        let qat = self.hosted_quick_access.borrow_mut().take();
        if let Some(qat) = qat {
            qat.clear_value(TemplatedControl::foreground_property());
        }
    }
    fn watch_model(&self) {
        self.unwatch_model();
        if let Some(ribbon) = self.ribbon() {
            let weak = self.to_ref().downgrade();
            let token = ribbon.property_changed(move |e| {
                if e.property() == Ribbon::model_property().as_property()
                    && let Some(c) = weak.upgrade()
                {
                    c.watch_model();
                }
            });
            self.model_subscription.replace(Some(token));
            let model = ribbon.model();
            if let Some(model) = model.as_ref() {
                let weak = self.to_ref().downgrade();
                let token = model.property_changed().add(Rc::new(move |name| {
                    if (name.is_empty() || name == "Title")
                        && let Some(c) = weak.upgrade()
                    {
                        c.update_display_title();
                    }
                }));
                self.model_token.set(Some(token));
            }
            self.watched_model.replace(model);
        }
        self.update_display_title();
    }
    fn unwatch_model(&self) {
        let token = self.model_subscription.borrow_mut().take();
        if let Some(token) = token {
            token.dispose();
        }
        let model = self.watched_model.borrow_mut().take();
        if let Some(model) = model
            && let Some(token) = self.model_token.take()
        {
            model.property_changed().remove(token);
        }
    }
    fn update_display_title(&self) {
        let title = self.title().or_else(|| {
            self.ribbon()
                .and_then(|r| r.model())
                .and_then(|m| m.title())
                // FerroUI renders valid Rust strings. Preserve the model's raw
                // units; replace unpaired surrogates only at this visual boundary.
                .map(|v| String::from_utf16_lossy(v.as_utf16()))
        });
        self.set_value(Self::display_title_property(), title);
    }
    fn update_adaptive(&self) {
        let width = self.bounds().width;
        if let Some(search) = self.search_box() {
            search.set_is_visible(self.is_search_visible() && width > 720.);
            search.set_width(min(self.search_width(), max(180., (width - 520.) / 2.)));
        }
        let title = self.parts.borrow().title.clone();
        if let Some(title) = title {
            title.set_is_visible(width == 0. || width > 480.);
        }
    }
}
impl Drop for RibbonTitleBar {
    fn drop(&mut self) {
        self.release_window_on_drop();
        self.unwatch_model();
        let ribbon = self.ribbon();
        if let Some(ribbon) = ribbon.as_ref() {
            ribbon.remove_quick_access_changed(self.ribbon_subscription.get_mut().take());
        }
        self.release_quick_access();
        if let Some(ribbon) = ribbon {
            ribbon.set_is_quick_access_hosted_externally(false);
            ribbon.update_quick_access_placement();
        }
        if let Some(token) = self.part_subscription.get_mut().take() {
            token.dispose();
        }
    }
}
impl FerroObjectImpl for RibbonTitleBar {
    fn constructed(c: &Self) {
        Self::parent_constructed(c);
        RibbonTheme::ensure_resources();
        KeyboardNavigation::set_is_tab_stop(c, false);
    }
}
impl StyledElementImpl for RibbonTitleBar {
    fn style_key_override(_: &Self) -> &'static TypeInfo {
        Self::TYPE
    }
}
impl ControlImpl for RibbonTitleBar {
    fn on_loaded(c: &Self, e: &ferroui_base::interactivity::RoutedEventArgs) {
        Self::parent_on_loaded(c, e);
        c.watch_model();
    }
    fn on_unloaded(c: &Self, e: &ferroui_base::interactivity::RoutedEventArgs) {
        Self::parent_on_unloaded(c, e);
        c.unwatch_model();
    }
    fn on_size_changed(c: &Self, e: &ferroui_controls::SizeChangedEventArgs) {
        Self::parent_on_size_changed(c, e);
        c.update_adaptive();
        c.refresh_window_chrome();
    }
}
impl TemplatedControlImpl for RibbonTitleBar {
    fn on_apply_template(c: &Self, e: &TemplateAppliedEventArgs) {
        let old_host = c.parts.borrow().quick_access.clone();
        if let Some(host) = old_host
            && host
                .child()
                .is_some_and(|v| v.is::<RibbonQuickAccessToolBar>())
        {
            RibbonQuickAccessToolBar::detach_from_border(&host);
        }
        Self::parent_on_apply_template(c, e);
        if let Some(token) = c.part_subscription.borrow_mut().take() {
            token.dispose();
        }
        let p = Parts {
            quick_access: e.name_scope().find_as::<Border>("PART_QuickAccessHost"),
            search: e.name_scope().find_as::<RibbonSearchBox>("PART_SearchBox"),
            title: e.name_scope().find_as::<Control>("PART_Title"),
            app_button: e.name_scope().find_as::<Button>("PART_AppButton"),
            app_icon: e.name_scope().find_as::<Control>("PART_AppIcon"),
            drag: e.name_scope().find_as::<Control>("PART_DragRegion"),
            start: e.name_scope().find_as::<Control>("PART_StartContent"),
            end: e.name_scope().find_as::<Control>("PART_EndContent"),
        };
        c.restore_window_roles();
        c.parts.replace(p);
        if let Some(button) = c.app_button() {
            let weak = c.to_ref().downgrade();
            let anchor = button.downgrade();
            let token = button.add_disposable_handler(Button::click_event(), move |_, _| {
                if let (Some(c), Some(anchor)) = (weak.upgrade(), anchor.upgrade()) {
                    c.on_app_button_click(anchor);
                }
            });
            c.part_subscription.replace(Some(token));
        }
        c.update_app_button();
        if let Some(search) = c.search_box() {
            search.set_ribbon(c.ribbon());
        }
        c.host_quick_access();
        c.update_adaptive();
        c.update_display_title();
        c.install_window_roles();
        c.refresh_window_chrome();
    }
}

#[cfg(test)]
#[path = "../title_bar_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../title_window_tests.rs"]
mod window_tests;
