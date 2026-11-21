//! Staged source RibbonSearchBox state machine. Copyright Wiesław Šoltés, MIT. Rust ownership
//! uses weak control references and explicit native subscription disposal.
use super::{
    RibbonCommandPalette, RibbonSearchQueryEventArgs,
    ribbon_search_results::RibbonSearchResultsList,
};
use crate::{controls::Ribbon, primitives::RibbonPopupPlacement, theming::RibbonTheme};
use ferroribbon_core::{RibbonString, localization::RibbonStrings, search::RibbonSearchEntry};
use ferroui_base::{
    BoxedValue, CornerRadius, ElementRef, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    Nullable, Ref, StaticType, StyledElementImpl, StyledProperty, Thickness, TypeInfo, Visual,
    VisualImpl, WeakRef,
    data::core::ValueTypes,
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{
        FocusChangingEventArgs, InputElement, InputElementImpl, Key, KeyEventArgs, KeyModifiers,
        KeyboardNavigation, NavigationMethod,
    },
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
    metadata::MarkupDelegate,
    reactive::{Disposable, IDisposable},
    styling::ThemeVariant,
    utilities::math_utilities::max,
};
use ferroui_controls::{
    Border, Control, ControlImpl, TextBox, TopLevel,
    automation::AutomationProperties,
    primitives::{
        Popup, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl,
        TemplatedControlImplExt,
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[repr(C)]
pub struct RibbonSearchBox {
    base: TemplatedControl,
    results: Rc<RibbonSearchResultsList>,
    query_entries: RefCell<Vec<Option<Rc<RibbonSearchEntry>>>>,
    text_box: RefCell<Option<Ref<TextBox>>>,
    popup: RefCell<Option<Ref<Popup>>>,
    shown_query: RefCell<Option<String>>,
    ignored_text: RefCell<Option<String>>,
    return_focus: RefCell<Option<WeakRef<InputElement>>>,
    part_subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    search_handler: RefCell<Option<MarkupDelegate>>,
    query_submitting: RefCell<Option<MarkupDelegate>>,
    result_executed: RefCell<Option<MarkupDelegate>>,
    is_executing: Cell<bool>,
}
ferro_class!(RibbonSearchBox:TemplatedControl);
ferro_impl_classes!(RibbonSearchBox:VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
ferro_class_info!(RibbonSearchBox{new:RibbonSearchBox::new,markup:{namespace:"FerroRibbon.Controls",properties:[Ribbon:Option<Ref<Ribbon>>{get:RibbonSearchBox::ribbon,set:|v:&RibbonSearchBox,r:Option<Ref<Ribbon>>|v.set_ribbon(r)},IsResultsOpen:bool{get:RibbonSearchBox::is_results_open}],methods:[fn FocusSearch()=>RibbonSearchBox::focus_search,fn Close()=>RibbonSearchBox::close],events:[QuerySubmitting(Option<BoxedValue>,Rc<RibbonSearchQueryEventArgs>){add:RibbonSearchBox::add_query_submitting,remove:RibbonSearchBox::remove_query_submitting},ResultExecuted(Option<BoxedValue>,Rc<RibbonSearchEntry>){add:RibbonSearchBox::add_result_executed,remove:RibbonSearchBox::remove_result_executed}]}});
ferro_properties! {impl RibbonSearchBox{
 pub fn ribbon_property()->StyledProperty<Option<ElementRef<Ribbon>>>{ValueTypes::register_element_ref::<Ribbon>();FerroProperty::register::<Self,_>("Ribbon",None)}
 pub fn placeholder_text_property()->StyledProperty<Option<String>>{FerroProperty::register::<Self,_>("PlaceholderText",None)}
 pub fn max_results_property()->StyledProperty<i32>{FerroProperty::register::<Self,_>("MaxResults",10)}
}}
// Native popup closure can transiently restore query focus while dispatching
// Backstage. Keep that framework focus transition from reopening command search.
// Explicit focus_search calls remain allowed, including command reentrancy.
struct SearchInvocation<'a> {
    active: &'a Cell<bool>,
    previous: bool,
}
impl Drop for SearchInvocation<'_> {
    fn drop(&mut self) {
        self.active.set(self.previous);
    }
}
impl RibbonSearchBox {
    fn static_constructor() {
        Self::ribbon_property()
            .changed()
            .add_class_handler::<Self>(|this, e| {
                let old = e.get_old_value::<Option<ElementRef<Ribbon>>>().flatten();
                let new = e.get_new_value::<Option<ElementRef<Ribbon>>>();
                this.on_ribbon_changed(ElementRef::resolve(&old), ElementRef::resolve(&new));
            })
            .detach();
        Self::placeholder_text_property()
            .changed()
            .add_class_handler::<Self>(|this, _| this.update_placeholder())
            .detach();
    }
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
            results: RibbonSearchResultsList::new(),
            query_entries: RefCell::default(),
            text_box: RefCell::default(),
            popup: RefCell::default(),
            shown_query: RefCell::default(),
            ignored_text: RefCell::default(),
            return_focus: RefCell::default(),
            part_subscriptions: RefCell::default(),
            search_handler: RefCell::default(),
            query_submitting: RefCell::default(),
            result_executed: RefCell::default(),
            is_executing: Cell::new(false),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn ribbon(&self) -> Option<Ref<Ribbon>> {
        ElementRef::resolve(&self.get_value(Self::ribbon_property()))
    }
    pub fn set_ribbon(&self, ribbon: impl Into<Nullable<Ribbon>>) {
        self.set_value(
            Self::ribbon_property(),
            ElementRef::from_nullable(ribbon.into().0),
        );
    }
    pub fn placeholder_text(&self) -> Option<String> {
        self.get_value(Self::placeholder_text_property())
    }
    pub fn set_placeholder_text(&self, value: Option<String>) {
        self.set_value(Self::placeholder_text_property(), value);
    }
    pub fn max_results(&self) -> i32 {
        self.get_value(Self::max_results_property())
    }
    pub fn set_max_results(&self, value: i32) {
        self.set_value(Self::max_results_property(), value);
    }
    pub fn is_results_open(&self) -> bool {
        self.popup.borrow().as_ref().is_some_and(|p| p.is_open())
    }
    pub fn add_query_submitting(&self, value: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(self.query_submitting.borrow().as_ref(), value.as_ref());
        self.query_submitting.replace(next);
    }
    pub fn remove_query_submitting(&self, value: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(self.query_submitting.borrow().as_ref(), value.as_ref());
        self.query_submitting.replace(next);
    }
    pub fn add_result_executed(&self, value: Option<MarkupDelegate>) {
        let next = MarkupDelegate::combine(self.result_executed.borrow().as_ref(), value.as_ref());
        self.result_executed.replace(next);
    }
    pub fn remove_result_executed(&self, value: Option<MarkupDelegate>) {
        let next = MarkupDelegate::remove(self.result_executed.borrow().as_ref(), value.as_ref());
        self.result_executed.replace(next);
    }
    /// Installs a typed Rust handler. Dispose its native token to detach early.
    pub fn query_submitting(
        &self,
        handler: impl Fn(&RibbonSearchBox, &RibbonSearchQueryEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let weak = self.to_ref().downgrade();
        let callback = MarkupDelegate::new(move |values| {
            if let Some(this) = weak.upgrade() {
                let args = values[1]
                    .as_ref()
                    .unwrap()
                    .downcast_ref::<RibbonSearchQueryEventArgs>()
                    .expect("query event data");
                handler(&this, args);
            }
            None
        });
        self.add_query_submitting(Some(callback.clone()));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.remove_query_submitting(Some(callback));
            }
        })
    }
    /// Installs a typed Rust result handler with a native removal token.
    pub fn result_executed(
        &self,
        handler: impl Fn(&RibbonSearchBox, &RibbonSearchEntry) + 'static,
    ) -> Rc<dyn IDisposable> {
        let weak = self.to_ref().downgrade();
        let callback = MarkupDelegate::new(move |values| {
            if let Some(this) = weak.upgrade() {
                let entry = values[1]
                    .as_ref()
                    .unwrap()
                    .downcast_ref::<RibbonSearchEntry>()
                    .expect("result event data");
                handler(&this, entry);
            }
            None
        });
        self.add_result_executed(Some(callback.clone()));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.remove_result_executed(Some(callback));
            }
        })
    }
    fn update_placeholder(&self) {
        let part = self.text_box.borrow().clone();
        if let Some(part) = part {
            let placeholder = self.placeholder_text().or_else(|| {
                RibbonStrings::current()
                    .search_placeholder()
                    .map(|s| s.to_utf8().expect("native placeholder UTF16"))
            });
            part.set_placeholder_text(placeholder.as_deref());
        }
    }
    fn on_ribbon_changed(&self, old: Option<Ref<Ribbon>>, new: Option<Ref<Ribbon>>) {
        self.shown_query.replace(None);
        let handler = self.search_handler.borrow().clone();
        if let Some(old) = old {
            old.remove_search_requested(handler.clone());
            self.remove_query_entries(&old);
        }
        if let Some(new) = new {
            new.add_search_requested(handler);
        }
    }
    fn on_search_requested(&self) {
        let part_exists = self.text_box.borrow().is_some();
        if !self.is_visible() || !self.is_loaded() || self.bounds().width <= 0. || !part_exists {
            if let Some(ribbon) = self.ribbon()
                && TopLevel::get_top_level(Some(&ribbon)).is_some()
            {
                RibbonCommandPalette::show(&ribbon);
            }
            return;
        }
        self.focus_search();
    }
    pub fn focus_search(&self) {
        let part = self.text_box.borrow().clone();
        let Some(part) = part else {
            return;
        };
        part.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
        let part = self
            .text_box
            .borrow()
            .clone()
            .expect("NullReference: _textBox.SelectAll");
        part.select_all();
        self.show_results();
    }
    fn on_getting_focus(&self, args: &FocusChangingEventArgs) {
        if self.is_executing.get() {
            return;
        }
        if let Some(old) = args.old_focused_element.as_ref()
            && !self.is_inside(old)
        {
            self.return_focus.replace(Some(old.downgrade()));
        }
    }
    fn is_inside(&self, element: &InputElement) -> bool {
        let mut current: Option<Ref<Visual>> = Some(element.to_ref().upcast());
        while let Some(node) = current {
            if node.ptr_eq(&self.to_ref()) {
                return true;
            }
            current = node.visual_parent();
        }
        false
    }
    fn on_text_changed(&self) {
        let part = self.text_box.borrow().clone();
        let Some(part) = part else {
            return;
        };
        let current = part.text().unwrap_or_default();
        if self
            .ignored_text
            .borrow()
            .as_ref()
            .is_some_and(|ignored| *ignored == current)
        {
            // Several queued events can read the same reset text after Enter.
            return;
        }
        self.ignored_text.replace(None);
        let open = self.is_results_open();
        if open && self.shown_query.borrow().as_deref() == Some(current.as_str()) {
            return;
        }
        if part.is_focused() || open {
            self.show_results();
        }
    }
    fn results_match_editor(&self) -> bool {
        self.text_box.borrow().as_ref().is_some_and(|part| {
            self.shown_query.borrow().as_deref() == Some(part.text().as_deref().unwrap_or_default())
        })
    }
    fn on_key_down(&self, e: &KeyEventArgs) {
        if matches!(e.key, Key::Down | Key::Up | Key::Enter)
            && self.is_results_open()
            && !self.results_match_editor()
        {
            // Input can arrive before the TextBox publishes its queued TextChanged.
            self.show_results();
            if !self.results_match_editor() {
                // QuerySubmitting can change the editor before results are shown.
                e.set_handled(true);
                return;
            }
        }
        match e.key {
            Key::Down if !self.is_results_open() => {
                self.show_results();
                e.set_handled(true);
            }
            Key::Down => {
                self.results.move_highlight(1);
                e.set_handled(true);
            }
            Key::Up if self.is_results_open() => {
                self.results.move_highlight(-1);
                e.set_handled(true);
            }
            Key::Enter if self.is_results_open() => {
                if let Some(entry) = self.results.highlighted() {
                    self.execute(entry);
                }
                e.set_handled(true);
            }
            Key::Escape => {
                self.close();
                self.restore_focus();
                e.set_handled(true);
            }
            _ => {}
        }
    }
    fn restore_focus(&self) {
        let target = self
            .return_focus
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade);
        if let Some(target) = target
            && target.visual_root().is_some()
            && !self.is_inside(&target)
        {
            target.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
        } else if let Some(header) = self
            .ribbon()
            .and_then(|r| r.selected_tab())
            .and_then(|t| t.header_element())
        {
            header.focus_with(NavigationMethod::Tab, KeyModifiers::NONE);
        }
        self.return_focus.replace(None);
    }
    fn show_results(&self) {
        let Some(ribbon) = self.ribbon() else {
            return;
        };
        if TopLevel::get_top_level(Some(self)).is_none() {
            return;
        }
        let part = self.text_box.borrow().clone();
        let Some(part) = part else {
            return;
        };
        let text = part.text();
        let shown_query = text.clone().unwrap_or_default();
        let query = RibbonString::from(shown_query.as_str());
        let args = RibbonSearchQueryEventArgs::new(text);
        let callback = self.query_submitting.borrow().clone();
        if let Some(callback) = callback {
            callback
                .try_invoke(&[Some(Control::boxed(self.to_ref())), Some(args.clone())])
                .expect("QuerySubmitting handler");
        }
        self.remove_query_entries(&ribbon);
        let contributions = args.additional_entries().borrow().clone();
        for extra in contributions {
            let represented = ribbon.additional_search_entries().iter().any(|entry| {
                entry.expect("NullReference: e.Id").id()
                    == extra.as_ref().expect("NullReference: extra.Id").id()
            });
            if !represented {
                ribbon.additional_search_entries().add(extra.clone());
                self.query_entries.borrow_mut().push(extra);
            }
        }
        self.results.show(
            &ribbon.search(Some(&query), self.max_results()),
            Some(&query),
        );
        self.shown_query.replace(Some(shown_query));
        let existing = self.popup.borrow().clone();
        let popup = if let Some(popup) = existing {
            popup
        } else {
            let chrome = Border::new();
            chrome.set_child(self.results.root());
            chrome.set_border_thickness(Thickness::uniform(1.));
            chrome.set_corner_radius(CornerRadius::uniform(8.));
            RibbonTheme::set_theme_brush(
                Some(&chrome),
                Some(Border::background_property().as_property()),
                Some("RibbonPopupBackgroundBrush"),
            )
            .expect("search popup background");
            RibbonTheme::set_theme_brush(
                Some(&chrome),
                Some(Border::border_brush_property().as_property()),
                Some("RibbonPopupBorderBrush"),
            )
            .expect("search popup border");
            let popup = Popup::new();
            popup.set_takes_focus_from_native_control(false);
            // Source Popup belongs to XamlRoot; use the same-window native layer.
            popup.set_should_use_overlay_layer(true);
            popup.set_child(chrome);
            popup.set_is_light_dismiss_enabled(true);
            popup.set_popup_parent(Some(&self.to_ref().upcast::<Control>()));
            let weak = self.to_ref().downgrade();
            popup
                .closed(move || {
                    if let Some(this) = weak.upgrade()
                        && let Some(current) = this.ribbon()
                    {
                        this.remove_query_entries(&current);
                    }
                })
                .detach();
            self.popup.replace(Some(popup.clone()));
            popup
        };
        if let Some(child) = popup.child() {
            child.set_value(
                ThemeVariant::requested_theme_variant_property(),
                self.actual_theme_variant(),
            );
            child.set_width(max(self.bounds().width, 360.));
        }
        RibbonPopupPlacement::place_below(&popup, self);
        popup.set_is_open(true);
    }
    fn remove_query_entries(&self, ribbon: &Ribbon) {
        let entries = self.query_entries.borrow().clone();
        for entry in entries {
            ribbon.additional_search_entries().remove(&entry);
        }
        self.query_entries.borrow_mut().clear();
    }
    fn execute(&self, entry: Rc<RibbonSearchEntry>) {
        if !entry.is_enabled() {
            return;
        }
        let invocation = SearchInvocation {
            active: &self.is_executing,
            previous: self.is_executing.replace(true),
        };
        self.close();
        let part = self.text_box.borrow().clone();
        let search_box = part.as_ref().map(|part| part.downgrade());
        if let Some(part) = part
            && !part.text().unwrap_or_default().is_empty()
        {
            self.ignored_text.replace(Some(String::new()));
            let part = self
                .text_box
                .borrow()
                .clone()
                .expect("NullReference: _textBox.Text");
            part.set_text(Some(""));
        }
        self.return_focus.replace(None);
        if let Some(ribbon) = self.ribbon() {
            ribbon.execute_search_entry_from(Some(entry.clone()), search_box);
        }
        drop(invocation);
        let callback = self.result_executed.borrow().clone();
        if let Some(callback) = callback {
            callback
                .try_invoke(&[Some(Control::boxed(self.to_ref())), Some(entry)])
                .expect("ResultExecuted handler");
        }
    }
    pub fn close(&self) {
        let popup = self.popup.borrow().clone();
        if let Some(popup) = popup
            && popup.is_open()
        {
            popup.set_is_open(false);
        }
    }
}
// The view does not own its ribbon. Releasing the view removes native
// subscriptions and its temporary contributions instead of relying on GC.
impl Drop for RibbonSearchBox {
    fn drop(&mut self) {
        for subscription in self.part_subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
        if let Some(ribbon) = self.ribbon() {
            ribbon.remove_search_requested(self.search_handler.get_mut().take());
            for entry in self.query_entries.get_mut().drain(..) {
                ribbon.additional_search_entries().remove(&entry);
            }
        }
        if let Some(popup) = self.popup.get_mut().take() {
            popup.set_is_open(false);
        }
    }
}
impl FerroObjectImpl for RibbonSearchBox {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        RibbonTheme::ensure_resources();
        KeyboardNavigation::set_is_tab_stop(this, false);
        let weak = this.to_ref().downgrade();
        this.search_handler
            .replace(Some(MarkupDelegate::new(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.on_search_requested();
                }
                None
            })));
        let weak = this.to_ref().downgrade();
        this.results.set_invoked(Some(Rc::new(move |entry| {
            if let Some(this) = weak.upgrade() {
                this.execute(entry);
            }
        })));
    }
}
impl StyledElementImpl for RibbonSearchBox {
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <Self as StaticType>::TYPE
    }
}
impl TemplatedControlImpl for RibbonSearchBox {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        let old = std::mem::take(&mut *this.part_subscriptions.borrow_mut());
        for token in old {
            token.dispose();
        }
        Self::parent_on_apply_template(this, e);
        let part = e.name_scope().find_as::<TextBox>("PART_TextBox");
        this.shown_query.replace(None);
        this.ignored_text.replace(None);
        this.text_box.replace(part.clone());
        if let Some(part) = part {
            this.update_placeholder();
            let weak = this.to_ref().downgrade();
            let text = part.add_disposable_handler(TextBox::text_changed_event(), move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.on_text_changed();
                }
            });
            let weak = this.to_ref().downgrade();
            let key = part.add_disposable_handler(InputElement::key_down_event(), move |_, e| {
                if let Some(this) = weak.upgrade() {
                    this.on_key_down(e);
                }
            });
            let weak = this.to_ref().downgrade();
            let got = part.add_disposable_handler(InputElement::got_focus_event(), move |_, _| {
                if let Some(this) = weak.upgrade()
                    && !this.is_executing.get()
                {
                    this.show_results();
                }
            });
            let weak = this.to_ref().downgrade();
            let getting =
                part.add_disposable_handler(InputElement::getting_focus_event(), move |_, e| {
                    if let Some(this) = weak.upgrade() {
                        this.on_getting_focus(e);
                    }
                });
            this.part_subscriptions.borrow_mut().extend([
                text.detach(),
                key.detach(),
                got.detach(),
                getting.detach(),
            ]);
            AutomationProperties::set_name(
                &part,
                RibbonStrings::current()
                    .search()
                    .map(|s| s.to_utf8().expect("native search name UTF16"))
                    .as_deref(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::{
        controls::ResourceDictionary,
        interactivity::RoutedEventArgs,
        styling::ControlTheme,
        threading::{CancellationToken, Dispatcher},
    };
    use ferroui_controls::{
        Application, Button, StackPanel, Window, templates::FuncControlTemplate,
    };
    use ferroui_headless::FerroHeadlessPlatformExtensions;
    use std::cell::Cell;

    fn run(test: impl FnOnce() + Send + 'static) {
        ferroui_headless::HeadlessUnitTestSession::start_new(|| {
            ferroui_controls::AppBuilder::configure::<Application>()
                .use_headless(Default::default())
        })
        .dispatch(
            move || {
                crate::register_types();
                let app = Application::current().unwrap();
                let theme = ferroui_themes_simple::SimpleTheme::new().as_style();
                app.styles().add(theme.clone());
                test();
                app.styles().remove(theme);
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
    }
    fn compiled_theme() -> Ref<ControlTheme> {
        let value =
            crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/Search.xaml")
                .unwrap()
                .unwrap();
        let dictionary = ValueTypes::as_object(&*value)
            .unwrap()
            .cast::<ResourceDictionary>()
            .unwrap();
        let value = dictionary
            .try_get_resource(&<RibbonSearchBox as StaticType>::TYPE.into(), None)
            .flatten()
            .unwrap();
        ValueTypes::as_object(&*value).unwrap().cast().unwrap()
    }
    fn host() -> (Ref<Window>, Ref<Ribbon>, Ref<RibbonSearchBox>, Ref<Button>) {
        let ribbon = Ribbon::new();
        let search = RibbonSearchBox::new();
        search.set_ribbon(ribbon.clone());
        search.set_theme(Some(compiled_theme()));
        let document = Button::new();
        document.set_content(Some(Rc::new("Document".to_string())));
        let panel = StackPanel::new();
        panel.children().add(document.clone());
        panel.children().add(search.clone());
        panel.children().add(ribbon.clone());
        let window = Window::new();
        window.set_width(800.);
        window.set_height(600.);
        window.set_content(Some(Control::boxed(panel)));
        window.show();
        window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        (window, ribbon, search, document)
    }
    fn key(part: &TextBox, key: Key) -> bool {
        let mut args = KeyEventArgs::new();
        args.key = key;
        args.set_routed_event(Some(InputElement::key_down_event()));
        part.raise_event(&args);
        args.handled()
    }
    fn ids(ribbon: &Ribbon) -> Vec<String> {
        ribbon
            .additional_search_entries()
            .iter()
            .map(|entry| entry.unwrap().id().unwrap().to_utf8().unwrap())
            .collect()
    }
    #[test]
    fn compiled_source_template_loads_real_part_resources_and_defaults() {
        run(|| {
            let (window, _, search, _) = host();
            let part = search
                .text_box
                .borrow()
                .clone()
                .expect("compiled PART_TextBox");
            assert_eq!(search.width(), 320.);
            assert_eq!(search.height(), 30.);
            assert_eq!(search.max_results(), 10);
            assert_eq!(search.part_subscriptions.borrow().len(), 4);
            assert_eq!(part.placeholder_text().as_deref(), Some("Search (Alt+Q)"));
            assert_eq!(
                AutomationProperties::get_name(&part).as_deref(),
                Some("Search")
            );
            assert!(
                part.resources()
                    .try_get_resource(&"TextControlBackgroundFocused".into(), None)
                    .is_some()
            );
            search.set_placeholder_text(Some("Find a command".into()));
            assert_eq!(part.placeholder_text().as_deref(), Some("Find a command"));
            window.close();
        });
    }
    #[test]
    fn query_contributions_cleanup_after_callback_and_execute_resets_before_result_event() {
        run(|| {
            let (window, ribbon, search, _) = host();
            ribbon
                .additional_search_entries()
                .add(Some(RibbonSearchEntry::new(
                    Some("stale".into()),
                    Some("Stale".into()),
                )));
            let entry = RibbonSearchEntry::new(Some("extra".into()), Some("Extra".into()));
            let seen = Rc::new(RefCell::new(Vec::new()));
            let observed = seen.clone();
            let weak = ribbon.downgrade();
            search.add_query_submitting(Some(MarkupDelegate::new(move |values| {
                observed.borrow_mut().push(ids(&weak.upgrade().unwrap()));
                let args = values[1]
                    .as_ref()
                    .unwrap()
                    .downcast_ref::<RibbonSearchQueryEventArgs>()
                    .unwrap();
                args.additional_entries()
                    .borrow_mut()
                    .extend([Some(entry.clone()), Some(entry.clone())]);
                None
            })));
            search.show_results();
            assert!(search.is_results_open());
            assert_eq!(ids(&ribbon), ["stale", "extra"]);
            let part = search.text_box.borrow().clone().unwrap();
            part.set_text(Some("Extra"));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(ids(&ribbon), ["stale", "extra"]);
            assert_eq!(seen.borrow().first().unwrap(), &["stale"]);
            assert_eq!(seen.borrow().last().unwrap(), &["stale", "extra"]);
            let results = Rc::new(RefCell::new(Vec::new()));
            let observed = results.clone();
            let weak = search.downgrade();
            search.add_result_executed(Some(MarkupDelegate::new(move |_| {
                let search = weak.upgrade().unwrap();
                observed.borrow_mut().push((
                    search.is_results_open(),
                    search
                        .text_box
                        .borrow()
                        .as_ref()
                        .unwrap()
                        .text()
                        .unwrap_or_default(),
                    ids(&search.ribbon().unwrap()),
                ));
                None
            })));
            assert!(key(&part, Key::Enter));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(
                &*results.borrow(),
                &[(false, String::new(), vec!["stale".to_string()])]
            );
            assert!(
                !search.is_results_open(),
                "queued reset must not reopen popup"
            );
            assert_eq!(window.opened_popups().len(), 0);
            window.close();
        });
    }
    #[test]
    fn search_box_enter_uses_current_query_before_async_text_changed() {
        run(|| {
            let (window, ribbon, search, _) = host();
            let invoked = Rc::new(RefCell::new(Vec::new()));
            let catalog = ferroribbon_core::commands::RibbonCommandCatalog::new();
            for name in ["Alpha", "Beta"] {
                let observed = invoked.clone();
                catalog.register_action(
                    Some(name.into()),
                    Some(name.into()),
                    Rc::new(move |_| observed.borrow_mut().push(name)),
                    None,
                    Default::default(),
                );
            }
            ribbon.set_command_catalog(Some(catalog));
            search.focus_search();
            let part = search.text_box.borrow().clone().unwrap();
            part.set_text(Some("Alpha"));
            Dispatcher::ui_thread().run_jobs(None);

            let queries = Rc::new(RefCell::new(Vec::new()));
            let observed = queries.clone();
            let _submitting = search.query_submitting(move |_, args| {
                observed.borrow_mut().push(args.query().map(str::to_owned));
            });
            part.set_text(Some("Beta"));
            assert!(key(&part, Key::Enter));
            assert_eq!(&*invoked.borrow(), &["Beta"]);
            assert_eq!(part.text().as_deref(), Some(""));
            Dispatcher::ui_thread().run_jobs(None);
            assert!(
                !search.is_results_open(),
                "queued updates must keep results closed"
            );
            assert!(part.is_focused());
            assert_eq!(&*queries.borrow(), &[Some("Beta".to_owned())]);
            window.close();
        });
    }

    #[test]
    fn retemplating_disposes_old_part_handlers() {
        run(|| {
            let (window, _, search, _) = host();
            let old = search.text_box.borrow().clone().unwrap();
            let calls = Rc::new(Cell::new(0));
            let observed = calls.clone();
            search.add_query_submitting(Some(MarkupDelegate::new(move |_| {
                observed.set(observed.get() + 1);
                None
            })));
            search.set_template(Some(FuncControlTemplate::new(|_, scope| {
                let part = TextBox::new();
                scope.register("PART_TextBox", part.clone().upcast());
                part.upcast()
            })));
            search.apply_template();
            window.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            let current = search.text_box.borrow().clone().unwrap();
            assert!(!old.ptr_eq(&current));
            assert_eq!(search.part_subscriptions.borrow().len(), 4);
            calls.set(0);
            old.raise_event(&RoutedEventArgs::with_event(TextBox::text_changed_event()));
            assert!(!key(&old, Key::Down));
            assert_eq!(calls.get(), 0);
            assert!(key(&current, Key::Down));
            assert_eq!(calls.get(), 1);
            search.close();
            window.close();
        });
    }
    #[test]
    fn native_handler_tokens_and_view_drop_release_contributions_and_popup() {
        run(|| {
            let (window, ribbon, search, _) = host();
            let calls = Rc::new(Cell::new(0));
            let observed = calls.clone();
            let token = search.query_submitting(move |_, args| {
                observed.set(observed.get() + 1);
                args.additional_entries()
                    .borrow_mut()
                    .push(Some(RibbonSearchEntry::new(
                        Some("extra".into()),
                        Some("Extra".into()),
                    )));
            });
            search.show_results();
            assert_eq!(ids(&ribbon), ["extra"]);
            assert_eq!(calls.get(), 1);
            token.dispose();
            token.dispose();
            search.show_results();
            assert_eq!(calls.get(), 1);
            assert!(ids(&ribbon).is_empty());
            let _retained_token = search.query_submitting(|_, args| {
                args.additional_entries()
                    .borrow_mut()
                    .push(Some(RibbonSearchEntry::new(
                        Some("owned".into()),
                        Some("Owned".into()),
                    )));
            });
            search.show_results();
            assert_eq!(ids(&ribbon), ["owned"]);
            let weak = search.downgrade();
            window.set_content(None);
            window.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            assert!(
                search.visual_root().is_none(),
                "view must be actually detached before testing release"
            );
            drop(search);
            assert!(
                weak.upgrade().is_none(),
                "the subscription token must not retain the view"
            );
            assert!(ids(&ribbon).is_empty());
            assert!(window.opened_popups().is_empty());
            window.close();
        });
    }
    #[test]
    fn escape_closes_popup_and_restores_external_keyboard_focus() {
        run(|| {
            let (window, _, search, document) = host();
            assert!(document.focus());
            search.focus_search();
            let part = search.text_box.borrow().clone().unwrap();
            assert!(part.is_focused());
            assert!(search.is_results_open());
            assert!(key(&part, Key::Escape));
            assert!(!search.is_results_open());
            assert!(document.is_focused());
            window.close();
        });
    }
}
