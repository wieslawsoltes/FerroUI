use super::{Inline, InlineImpl, Span, TextElementImpl};
use crate::{Control, TopLevel};
use ferroui_base::input::Key;
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::{HandlerList, Uri};
use ferroui_base::{
    ferro_class, instantiate, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty,
};
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum UnderlineStyle {
    None = 0,
    #[default]
    Single = 1,
}

ferroui_base::ferro_markup_enum!(UnderlineStyle { None, Single }, { namespace: "FerroUI.Controls.Documents" });

/// Event data shared by every listener of one hyperlink activation.
#[derive(Clone, Debug, PartialEq)]
pub struct HyperlinkClickEventArgs {
    pub navigate_uri: Option<String>,
    pub target_name: Option<String>,
    handled: Rc<Cell<bool>>,
}

impl HyperlinkClickEventArgs {
    pub fn handled(&self) -> bool {
        self.handled.get()
    }
    pub fn set_handled(&self, value: bool) {
        self.handled.set(value);
    }
}

ferroui_base::ferro_markup_type!(class HyperlinkClickEventArgs {
    namespace: "FerroUI.Controls.Documents",
    handles: [HyperlinkClickEventArgs],
    properties: [
        Handled: bool { get: HyperlinkClickEventArgs::handled, set: HyperlinkClickEventArgs::set_handled },
        NavigateUri: Option<String> { get: |args: &HyperlinkClickEventArgs| args.navigate_uri.clone() },
        TargetName: Option<String> { get: |args: &HyperlinkClickEventArgs| args.target_name.clone() },
    ],
});

/// Captured event-list identity for a structured hyperlink snapshot. It keeps
/// the listeners alive without retaining the old parented content tree. Event
/// lists are runtime behavior, so they do not participate in value equality of
/// document content or create otherwise empty undo units.
#[derive(Clone, Default)]
pub struct HyperlinkBehavior {
    handlers: Option<(
        Rc<HandlerList<dyn Fn(&HyperlinkClickEventArgs)>>,
        Rc<HandlerList<dyn Fn(&HyperlinkClickEventArgs)>>,
    )>,
}
impl HyperlinkBehavior {
    pub(crate) fn shares_handlers(&self, other: &Self) -> bool {
        match (&self.handlers, &other.handlers) {
            (Some((left_click, left_navigation)), Some((right_click, right_navigation))) => {
                Rc::ptr_eq(left_click, right_click) && Rc::ptr_eq(left_navigation, right_navigation)
            }
            _ => false,
        }
    }
}
impl std::fmt::Debug for HyperlinkBehavior {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("HyperlinkBehavior")
    }
}
impl PartialEq for HyperlinkBehavior {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

/// An interactive span. Its host supplies hit testing/focus and forwards pointer
/// and keyboard gestures; invocation and URI launch use the framework services.
#[repr(C)]
pub struct Hyperlink {
    base: Span,
    click_handlers: Rc<HandlerList<dyn Fn(&HyperlinkClickEventArgs)>>,
    navigation_handlers: Rc<HandlerList<dyn Fn(&HyperlinkClickEventArgs)>>,
    is_pointer_over: Cell<bool>,
    is_pressed: Cell<bool>,
    is_focused: Cell<bool>,
    field_id: Cell<Option<u64>>,
}

ferro_class!(Hyperlink: Span);
ferroui_base::ferro_class_info!(Hyperlink {
    new: Hyperlink::new,
    markup: {
        events: [
            Click(HyperlinkClickEventArgs) => |this: &Ref<Hyperlink>, handler: MarkupDelegate| {
                this.click(move |args| { handler.invoke(&[into_markup_value(args.clone())]); })
            },
            NavigationRequested(HyperlinkClickEventArgs) => |this: &Ref<Hyperlink>, handler: MarkupDelegate| {
                this.navigation_requested(move |args| { handler.invoke(&[into_markup_value(args.clone())]); })
            },
        ],
    },
});

ferroui_base::ferro_impl_classes!(Hyperlink: StyledElementImpl, TextElementImpl, InlineImpl);

impl FerroObjectImpl for Hyperlink {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        // As with Underline, the intrinsic format overrides inherited parent
        // values without becoming a local assignment in document snapshots.
        // Application styles and local values can still override it.
        this.set_current_value(
            Inline::text_decorations_property(),
            Some(ferroui_base::media::TextDecorations::underline()),
        );
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if matches!(
            change.property().name(),
            "NavigateUri" | "TargetName" | "IsEnabled" | "IsTabStop" | "TabIndex"
        ) {
            if !this.is_enabled() {
                this.is_pressed.set(false);
            }
            if let Some(host) = this.inline_host() {
                host.invalidate();
            }
        } else if change.property().name() == "IsVisited" {
            this.invalidate_host();
        } else if change.property() == Self::underline_style_property().as_property() {
            this.set_text_decorations(if this.should_draw_underline() {
                Some(ferroui_base::media::TextDecorations::underline())
            } else {
                None
            });
        }
    }
}

ferroui_base::ferro_properties! {
    impl Hyperlink {
        pub fn navigate_uri_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<Hyperlink, _>("NavigateUri", None)
        }
        pub fn target_name_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<Hyperlink, _>("TargetName", None)
        }
        pub fn is_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<Hyperlink, _>("IsEnabled", true)
        }
        pub fn is_tab_stop_property() -> StyledProperty<bool> {
            FerroProperty::register::<Hyperlink, _>("IsTabStop", true)
        }
        pub fn tab_index_property() -> StyledProperty<i32> {
            FerroProperty::register::<Hyperlink, _>("TabIndex", 0)
        }
        pub fn is_visited_property() -> StyledProperty<bool> {
            FerroProperty::register::<Hyperlink, _>("IsVisited", false)
        }
        pub fn underline_style_property() -> StyledProperty<UnderlineStyle> {
            FerroProperty::register::<Hyperlink, _>("UnderlineStyle", UnderlineStyle::Single)
        }
    }
}

impl Hyperlink {
    fn static_constructor() {
        Inline::text_decorations_property().override_default_value::<Hyperlink>(Some(
            ferroui_base::media::TextDecorations::underline(),
        ));
    }

    pub fn construct() -> Self {
        Self {
            base: Span::construct(),
            click_handlers: Rc::new(HandlerList::new()),
            navigation_handlers: Rc::new(HandlerList::new()),
            is_pointer_over: Cell::new(false),
            is_pressed: Cell::new(false),
            is_focused: Cell::new(false),
            field_id: Cell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub(crate) fn field_id(&self) -> Option<u64> {
        self.field_id.get()
    }

    pub(crate) fn set_field_id(&self, value: Option<u64>) {
        self.field_id.set(value);
    }

    pub(crate) fn snapshot_behavior(&self) -> HyperlinkBehavior {
        HyperlinkBehavior {
            handlers: Some((
                self.click_handlers.clone(),
                self.navigation_handlers.clone(),
            )),
        }
    }

    pub(crate) fn with_snapshot_behavior(behavior: &HyperlinkBehavior) -> Ref<Self> {
        let mut value = Self::construct();
        if let Some((click, navigation)) = &behavior.handlers {
            value.click_handlers = click.clone();
            value.navigation_handlers = navigation.clone();
        }
        instantiate(value)
    }

    pub(crate) fn with_behavior_from(source: &Self) -> Ref<Self> {
        let mut value = Self::construct();
        value.click_handlers = source.click_handlers.clone();
        value.navigation_handlers = source.navigation_handlers.clone();
        let link = instantiate(value);
        link.set_is_tab_stop(source.is_tab_stop());
        link.set_tab_index(source.tab_index());
        link.set_is_visited(source.is_visited());
        link.set_underline_style(source.underline_style());
        link
    }

    pub fn with_text(text: &str, navigate_uri: Option<&str>) -> Ref<Self> {
        let link = Self::new();
        link.add_child_text(text);
        link.set_navigate_uri(navigate_uri.map(str::to_owned));
        link
    }

    pub fn navigate_uri(&self) -> Option<String> {
        self.get_value(Self::navigate_uri_property())
    }
    pub fn set_navigate_uri(&self, value: Option<String>) {
        self.set_value(Self::navigate_uri_property(), value);
    }
    pub fn target_name(&self) -> Option<String> {
        self.get_value(Self::target_name_property())
    }
    pub fn set_target_name(&self, value: Option<String>) {
        self.set_value(Self::target_name_property(), value);
    }
    pub fn is_enabled(&self) -> bool {
        self.get_value(Self::is_enabled_property())
    }
    pub fn set_is_enabled(&self, value: bool) {
        self.set_value(Self::is_enabled_property(), value);
    }
    pub fn is_tab_stop(&self) -> bool {
        self.get_value(Self::is_tab_stop_property())
    }
    pub fn set_is_tab_stop(&self, value: bool) {
        self.set_value(Self::is_tab_stop_property(), value);
    }
    pub fn tab_index(&self) -> i32 {
        self.get_value(Self::tab_index_property())
    }
    pub fn set_tab_index(&self, value: i32) {
        self.set_value(Self::tab_index_property(), value);
    }
    pub fn is_visited(&self) -> bool {
        self.get_value(Self::is_visited_property())
    }
    pub fn set_is_visited(&self, value: bool) {
        self.set_value(Self::is_visited_property(), value);
    }
    pub fn underline_style(&self) -> UnderlineStyle {
        self.get_value(Self::underline_style_property())
    }
    pub fn set_underline_style(&self, value: UnderlineStyle) {
        self.set_value(Self::underline_style_property(), value);
    }
    pub fn should_draw_underline(&self) -> bool {
        self.underline_style() != UnderlineStyle::None
    }
    pub fn content_text(&self) -> String {
        self.inlines().text().unwrap_or_default()
    }
    pub fn is_pointer_over(&self) -> bool {
        self.is_pointer_over.get()
    }
    pub fn is_pressed(&self) -> bool {
        self.is_pressed.get()
    }
    pub fn is_focused(&self) -> bool {
        self.is_focused.get()
    }

    pub fn set_is_pointer_over(&self, value: bool) {
        if self.is_pointer_over.replace(value) != value {
            self.invalidate_host();
        }
    }

    pub fn set_is_focused(&self, value: bool) {
        if self.is_focused.replace(value) != value {
            self.invalidate_host();
        }
    }

    pub fn click(
        &self,
        handler: impl Fn(&HyperlinkClickEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.click_handlers, handler)
    }

    pub fn navigation_requested(
        &self,
        handler: impl Fn(&HyperlinkClickEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        subscribe(&self.navigation_handlers, handler)
    }

    /// Raises Click, then NavigationRequested. A handled event cancels default URI
    /// launch. A relative URI remains available to an application navigation handler.
    pub fn navigate(&self) -> bool {
        if !self.is_enabled() {
            return false;
        }
        let args = HyperlinkClickEventArgs {
            navigate_uri: self.navigate_uri(),
            target_name: self.target_name(),
            handled: Rc::new(Cell::new(false)),
        };
        for (_, handler) in self.click_handlers.snapshot().iter() {
            handler(&args);
        }
        if args.handled() {
            return true;
        }
        if args.navigate_uri.is_none() {
            return true;
        }
        for (_, handler) in self.navigation_handlers.snapshot().iter() {
            handler(&args);
        }
        if !args.handled() {
            self.launch_uri(&args);
        }
        true
    }

    pub fn pointer_pressed(&self) -> bool {
        if !self.is_enabled() {
            return false;
        }
        self.is_pressed.set(true);
        self.invalidate_host();
        true
    }

    pub fn pointer_released(&self, is_over: bool) -> bool {
        let was_pressed = self.is_pressed.replace(false);
        self.invalidate_host();
        was_pressed && is_over && self.navigate()
    }

    pub fn cancel_pointer(&self) {
        self.is_pressed.set(false);
        self.invalidate_host();
    }

    pub fn key_down(&self, key: Key) -> bool {
        if !self.is_enabled() || !Self::is_link_navigation_key(key) {
            return false;
        }
        self.is_pressed.set(true);
        self.invalidate_host();
        true
    }

    pub fn key_up(&self, key: Key) -> bool {
        if !Self::is_link_navigation_key(key) {
            return false;
        }
        let pressed = self.is_pressed.replace(false);
        self.invalidate_host();
        pressed && self.navigate()
    }

    pub fn is_link_navigation_key(key: Key) -> bool {
        matches!(key, Key::Enter | Key::Space)
    }

    fn invalidate_host(&self) {
        if let Some(host) = self.inline_host() {
            host.invalidate_render();
        }
    }

    fn launch_uri(&self, args: &HyperlinkClickEventArgs) {
        let Some(uri) = args
            .navigate_uri
            .as_deref()
            .and_then(|s| Uri::absolute(s).ok())
        else {
            return;
        };
        let mut parent = self.parent();
        while let Some(element) = parent {
            if let Some(control) = element.cast::<Control>() {
                let Some(top_level) = TopLevel::get_top_level(Some(&control)) else {
                    return;
                };
                let weak = self.to_ref().downgrade();
                let _ = Dispatcher::ui_thread().invoke_async_task_local(move || async move {
                    if top_level.launcher().launch_uri_async(&uri).await {
                        if let Some(link) = weak.upgrade() {
                            link.set_is_visited(true);
                        }
                    }
                });
                return;
            }
            parent = element.parent();
        }
    }
}

fn subscribe(
    handlers: &Rc<HandlerList<dyn Fn(&HyperlinkClickEventArgs)>>,
    handler: impl Fn(&HyperlinkClickEventArgs) + 'static,
) -> Rc<dyn IDisposable> {
    let token = handlers.add(Rc::new(handler));
    let handlers = Rc::downgrade(handlers);
    Disposable::create(move || {
        if let Some(handlers) = handlers.upgrade() {
            handlers.remove(token);
        }
    })
}
