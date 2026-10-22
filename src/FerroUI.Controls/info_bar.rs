// Copyright (c) Microsoft Corporation. Licensed under the MIT License. See NOTICE.md.
// Adapted from the pinned InfoBar.idl, InfoBar.cpp and InfoBar.xaml contracts.

use super::{IconSource, InfoBarPanel, Symbol, SymbolIcon};
use crate::automation::peers::{
    AutomationPeer, InfoBarAutomationPeer,
};
use crate::automation::{
    AccessibilityView, AutomationLiveSetting, AutomationProperties,
};
use crate::presenters::ContentPresenter;
use crate::primitives::{
    InfoBarTemplateSettings,
    TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
};
use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
use crate::{
    Border, Button, ContentControl, ContentControlImpl, Control, ControlImpl, Grid,
    TextBlock, ToolTip, Viewbox,
};
use ferroui_base::controls::ResourceKey;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::TemplateBinding;
use ferroui_base::input::{ICommand, InputElementImpl};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::media::{FontFamily, FontWeight, IBrush, SolidColorBrush, TextWrapping};
use ferroui_base::metadata::{into_markup_value, MarkupDelegate, MarkupType, MarkupTyped};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::styling::{ControlTheme, ThemeVariant};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_enum, ferro_markup_type,
    ferro_properties, instantiate, BoxedValue, CornerRadius, DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref,
    StyledElementImpl, StyledProperty, Thickness, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InfoBarSeverity {
    #[default]
    Informational = 0,
    Success = 1,
    Warning = 2,
    Error = 3,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InfoBarCloseReason {
    CloseButton = 0,
    #[default]
    Programmatic = 1,
}

ferro_markup_enum!(InfoBarSeverity { Informational, Success, Warning, Error }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(InfoBarCloseReason { CloseButton, Programmatic }, { namespace: "FerroUI.Controls" });

/// Cancellation is shared between event-argument clones delivered to markup handlers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InfoBarClosingEventArgs {
    reason: InfoBarCloseReason,
    cancel: Rc<Cell<bool>>,
}
impl InfoBarClosingEventArgs {
    pub fn reason(&self) -> InfoBarCloseReason {
        self.reason
    }
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }
    pub fn set_cancel(&self, value: bool) {
        self.cancel.set(value);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InfoBarClosedEventArgs {
    reason: InfoBarCloseReason,
}
impl InfoBarClosedEventArgs {
    pub fn reason(&self) -> InfoBarCloseReason {
        self.reason
    }
}

/// The pinned upstream Opened event is a preview contract.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InfoBarOpenedEventArgs;

ferro_markup_type!(class InfoBarClosingEventArgs {
    namespace: "FerroUI.Controls", handles: [InfoBarClosingEventArgs],
    properties: [
        Reason: InfoBarCloseReason { get: |args: &InfoBarClosingEventArgs| args.reason() },
        Cancel: bool { get: |args: &InfoBarClosingEventArgs| args.cancel(), set: |args: &InfoBarClosingEventArgs, value: bool| args.set_cancel(value) },
    ],
});
ferro_markup_type!(class InfoBarClosedEventArgs {
    namespace: "FerroUI.Controls", handles: [InfoBarClosedEventArgs],
    properties: [Reason: InfoBarCloseReason { get: |args: &InfoBarClosedEventArgs| args.reason() }],
});
ferro_markup_type!(class InfoBarOpenedEventArgs {
    namespace: "FerroUI.Controls", handles: [InfoBarOpenedEventArgs], properties: [],
});

#[derive(Clone, Default)]
struct Parts {
    root: Option<Ref<Border>>,
    background_layer: Option<Ref<Border>>,
    standard_area: Option<Ref<Grid>>,
    standard_icon: Option<Ref<TextBlock>>,
    icon_background: Option<Ref<TextBlock>>,
    user_icon: Option<Ref<Viewbox>>,
    title: Option<Ref<TextBlock>>,
    message: Option<Ref<TextBlock>>,
    banner: Option<Ref<InfoBarPanel>>,
    action: Option<Ref<ContentPresenter>>,
    content: Option<Ref<ContentPresenter>>,
    close: Option<Ref<Button>>,
    close_subscription: Option<(WeakRef<Button>, RoutedEventHandlerToken)>,
}
impl Parts {
    fn detach(&mut self) {
        if let Some((button, token)) = self.close_subscription.take() {
            if let Some(button) = button.upgrade() {
                button.remove_handler(Button::click_event(), token);
            }
        }
    }
}

/// An inline message with severity, live native content and cancellable closing.
///
/// The existing notification controls remain independent. CloseButtonStyle uses
/// a native ControlTheme; translating arbitrary WinUI Style setters is a separate
/// markup operation. Windows notification announcements are not emulated: the
/// peer exposes native status-bar, name and live-region metadata.
#[repr(C)]
pub struct InfoBar {
    base: ContentControl,
    last_close_reason: Cell<InfoBarCloseReason>,
    settings: Ref<InfoBarTemplateSettings>,
    parts: RefCell<Parts>,
    subscriptions: RefCell<Vec<Rc<dyn IDisposable>>>,
    close_button_click: HandlerList<dyn Fn()>,
    closing: HandlerList<dyn Fn(&InfoBarClosingEventArgs)>,
    closed: HandlerList<dyn Fn(&InfoBarClosedEventArgs)>,
    opened: HandlerList<dyn Fn(&InfoBarOpenedEventArgs)>,
}
ferro_class!(InfoBar: ContentControl);
ferro_class_info!(InfoBar {
    new: InfoBar::new,
    markup: {
        namespace: "FerroUI.Controls", content: Content,
        events: [
            CloseButtonClick(Option<BoxedValue>, Option<BoxedValue>) => |this: &Ref<InfoBar>, handler: MarkupDelegate| {
                let weak = this.downgrade();
                this.close_button_click(move || {
                    if let Some(sender) = weak.upgrade() { handler.invoke(&[into_markup_value(Some(sender)), into_markup_value(None::<BoxedValue>)]); }
                })
            },
            Closing(Option<BoxedValue>, InfoBarClosingEventArgs) => |this: &Ref<InfoBar>, handler: MarkupDelegate| {
                let weak = this.downgrade();
                this.closing(move |args| {
                    if let Some(sender) = weak.upgrade() { handler.invoke(&[into_markup_value(Some(sender)), into_markup_value(args.clone())]); }
                })
            },
            Closed(Option<BoxedValue>, InfoBarClosedEventArgs) => |this: &Ref<InfoBar>, handler: MarkupDelegate| {
                let weak = this.downgrade();
                this.closed(move |args| {
                    if let Some(sender) = weak.upgrade() { handler.invoke(&[into_markup_value(Some(sender)), into_markup_value(args.clone())]); }
                })
            },
            Opened(Option<BoxedValue>, InfoBarOpenedEventArgs) => |this: &Ref<InfoBar>, handler: MarkupDelegate| {
                let weak = this.downgrade();
                this.opened(move |args| {
                    if let Some(sender) = weak.upgrade() { handler.invoke(&[into_markup_value(Some(sender)), into_markup_value(args.clone())]); }
                })
            },
        ],
    },
});
ferro_impl_classes!(InfoBar: StyledElementImpl, VisualImpl, LayoutableImpl, InteractiveImpl, InputElementImpl, ContentControlImpl);

impl FerroObjectImpl for InfoBar {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let weak = this.to_ref().downgrade();
        let resources = this.resources_changed(move |_| {
            if let Some(owner) = weak.upgrade() {
                owner.update_parts();
            }
        });
        let weak = this.to_ref().downgrade();
        let theme = this.actual_theme_variant_changed(move || {
            if let Some(owner) = weak.upgrade() {
                owner.update_parts();
            }
        });
        let weak = this.to_ref().downgrade();
        let settings = this.settings.property_changed(move |change| {
            if change.property() == InfoBarTemplateSettings::icon_element_property().as_property() {
                if let Some(owner) = weak.upgrade() {
                    owner.update_parts();
                }
            }
        });
        this.subscriptions
            .borrow_mut()
            .extend([resources, theme, settings]);
        this.update_visibility();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if !change.is_effective_value_change() {
            return;
        }
        let property = change.property();
        if property == Self::is_open_property().as_property() {
            this.open_state_changed();
        } else if property == Self::icon_source_property().as_property() {
            this.settings.set_icon_element(
                this.icon_source()
                    .map(|source| source.create_icon_element()),
            );
            this.update_parts();
        } else if [
            Self::title_property().as_property(),
            Self::message_property().as_property(),
            Self::severity_property().as_property(),
            Self::is_icon_visible_property().as_property(),
            Self::is_closable_property().as_property(),
            Self::action_button_property().as_property(),
            Self::close_button_style_property().as_property(),
            Self::close_button_command_property().as_property(),
            Self::close_button_command_parameter_property().as_property(),
            ContentControl::content_property().as_property(),
            ContentControl::content_template_property().as_property(),
            TemplatedControl::background_property().as_property(),
            TemplatedControl::foreground_property().as_property(),
            TemplatedControl::border_brush_property().as_property(),
            TemplatedControl::border_thickness_property().as_property(),
            TemplatedControl::corner_radius_property().as_property(),
            TemplatedControl::padding_property().as_property(),
        ]
        .contains(&property)
        {
            this.update_parts();
        }
    }
}

impl ControlImpl for InfoBar {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        InfoBarAutomationPeer::new(this).upcast()
    }
}

impl TemplatedControlImpl for InfoBar {
    fn on_apply_template(this: &Self, args: &TemplateAppliedEventArgs) {
        let mut old = this.parts.replace(Parts::default());
        old.detach();
        Self::parent_on_apply_template(this, args);
        let scope = args.name_scope();
        let close = scope.find_as::<Button>("CloseButton");
        let close_subscription = close.as_ref().map(|button| {
            let weak = this.to_ref().downgrade();
            let token = button.click(move |_, _| {
                if let Some(owner) = weak.upgrade() {
                    owner.close_clicked();
                }
            });
            (button.downgrade(), token)
        });
        *this.parts.borrow_mut() = Parts {
            root: scope.find_as::<Border>("ContentRoot"),
            background_layer: scope.find_as::<Border>("BackgroundLayer"),
            standard_area: scope.find_as::<Grid>("StandardIconArea"),
            standard_icon: scope.find_as::<TextBlock>("StandardIcon"),
            icon_background: scope.find_as::<TextBlock>("IconBackground"),
            user_icon: scope.find_as::<Viewbox>("UserIconBox"),
            title: scope.find_as::<TextBlock>("Title"),
            message: scope.find_as::<TextBlock>("Message"),
            banner: scope.find_as::<InfoBarPanel>("BannerPanel"),
            action: scope.find_as::<ContentPresenter>("ActionButtonPresenter"),
            content: scope
                .find_as::<ContentPresenter>("PART_ContentPresenter")
                .or_else(|| scope.find_as::<ContentPresenter>("ContentArea")),
            close,
            close_subscription,
        };
        this.update_parts();
        this.update_visibility();
    }
}

ferro_properties! { impl InfoBar {
    pub fn is_open_property() -> StyledProperty<bool> { FerroProperty::register::<InfoBar, _>("IsOpen", false) }
    pub fn title_property() -> StyledProperty<String> { FerroProperty::register::<InfoBar, _>("Title", String::new()) }
    pub fn message_property() -> StyledProperty<String> { FerroProperty::register::<InfoBar, _>("Message", String::new()) }
    pub fn severity_property() -> StyledProperty<InfoBarSeverity> { FerroProperty::register::<InfoBar, _>("Severity", InfoBarSeverity::Informational) }
    pub fn icon_source_property() -> StyledProperty<Option<Ref<IconSource>>> { FerroProperty::register::<InfoBar, _>("IconSource", None) }
    pub fn is_icon_visible_property() -> StyledProperty<bool> { FerroProperty::register::<InfoBar, _>("IsIconVisible", true) }
    pub fn is_closable_property() -> StyledProperty<bool> { FerroProperty::register::<InfoBar, _>("IsClosable", true) }
    /// Native theme adaptation; arbitrary WinUI Style setter translation is not implied.
    pub fn close_button_style_property() -> StyledProperty<Option<Ref<ControlTheme>>> { FerroProperty::register::<InfoBar, _>("CloseButtonStyle", None) }
    pub fn close_button_command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> { FerroProperty::register::<InfoBar, _>("CloseButtonCommand", None) }
    pub fn close_button_command_parameter_property() -> StyledProperty<Option<BoxedValue>> { FerroProperty::register::<InfoBar, _>("CloseButtonCommandParameter", None) }
    pub fn action_button_property() -> StyledProperty<Option<Ref<Button>>> { FerroProperty::register::<InfoBar, _>("ActionButton", None) }
    pub fn template_settings_property() -> DirectProperty<InfoBar, Ref<InfoBarTemplateSettings>> {
        FerroProperty::register_direct::<InfoBar, _>("TemplateSettings", |owner| owner.template_settings(), None, InfoBarTemplateSettings::new())
    }
} }

impl InfoBar {
    fn static_constructor() {
        TemplatedControl::border_thickness_property()
            .override_default_value::<InfoBar>(Thickness::uniform(1.0));
        TemplatedControl::corner_radius_property()
            .override_default_value::<InfoBar>(CornerRadius::uniform(4.0));
        TemplatedControl::padding_property()
            .override_default_value::<InfoBar>(Thickness::new(16.0, 0.0, 0.0, 0.0));
        let template = FuncControlTemplate::for_type::<InfoBar>(|_, scope| {
            let root = Border::new();
            root.set_name(Some("ContentRoot".into()));
            root.set_vertical_alignment(VerticalAlignment::Top);
            root.set_min_height(48.0);
            let background_layer = Border::new();
            background_layer.set_name(Some("BackgroundLayer".into()));
            let grid = Grid::new();
            grid.set_min_height(48.0);
            grid.set_column_definitions("Auto,*,Auto".parse().expect("constant InfoBar columns"));
            grid.set_row_definitions("Auto,Auto".parse().expect("constant InfoBar rows"));
            let standard_area = Grid::new();
            standard_area.set_name(Some("StandardIconArea".into()));
            let make_icon_text = |name: &str, glyph: &str| {
                let text = TextBlock::new();
                text.set_name(Some(name.into()));
                text.set_text(Some(glyph));
                text.set_font_family(FontFamily::new("Segoe Fluent Icons"));
                text.set_font_size(16.0);
                text.set_vertical_alignment(VerticalAlignment::Top);
                text.set_margin(Thickness::new(0.0, 16.0, 14.0, 16.0));
                text.register_in_name_scope(&**scope)
            };
            let icon_background = make_icon_text("IconBackground", "\u{F136}");
            AutomationProperties::set_accessibility_view(&icon_background, AccessibilityView::Raw);
            standard_area
                .children()
                .add(icon_background.upcast::<Control>());
            standard_area
                .children()
                .add(make_icon_text("StandardIcon", "\u{F13F}").upcast::<Control>());
            grid.children().add(
                standard_area
                    .register_in_name_scope(&**scope)
                    .upcast::<Control>(),
            );
            let user_icon = Viewbox::new();
            user_icon.set_name(Some("UserIconBox".into()));
            user_icon.set_max_width(16.0);
            user_icon.set_max_height(16.0);
            user_icon.set_vertical_alignment(VerticalAlignment::Top);
            user_icon.set_margin(Thickness::new(0.0, 16.0, 14.0, 16.0));
            grid.children().add(
                user_icon
                    .register_in_name_scope(&**scope)
                    .upcast::<Control>(),
            );
            let banner = InfoBarPanel::new();
            banner.set_name(Some("BannerPanel".into()));
            banner.set_margin(Thickness::new(0.0, 0.0, 16.0, 0.0));
            banner.set_vertical_orientation_padding(Thickness::new(0.0, 14.0, 0.0, 18.0));
            Grid::set_column(&banner, 1);
            let title = TextBlock::new();
            title.set_name(Some("Title".into()));
            title.set_font_size(14.0);
            title.set_font_weight(FontWeight::SemiBold);
            title.set_text_wrapping(TextWrapping::Wrap);
            title.set_horizontal_alignment(HorizontalAlignment::Left);
            InfoBarPanel::set_horizontal_orientation_margin(
                &title,
                Thickness::new(0.0, 14.0, 0.0, 0.0),
            );
            InfoBarPanel::set_vertical_orientation_margin(
                &title,
                Thickness::new(0.0, 14.0, 0.0, 0.0),
            );
            banner
                .children()
                .add(title.register_in_name_scope(&**scope).upcast::<Control>());
            let message = TextBlock::new();
            message.set_name(Some("Message".into()));
            message.set_font_size(14.0);
            message.set_text_wrapping(TextWrapping::Wrap);
            message.set_horizontal_alignment(HorizontalAlignment::Left);
            InfoBarPanel::set_horizontal_orientation_margin(
                &message,
                Thickness::new(12.0, 14.0, 0.0, 0.0),
            );
            InfoBarPanel::set_vertical_orientation_margin(
                &message,
                Thickness::new(0.0, 4.0, 0.0, 0.0),
            );
            banner
                .children()
                .add(message.register_in_name_scope(&**scope).upcast::<Control>());
            let action = ContentPresenter::new();
            action.set_name(Some("ActionButtonPresenter".into()));
            action.set_vertical_alignment(VerticalAlignment::Top);
            InfoBarPanel::set_horizontal_orientation_margin(
                &action,
                Thickness::new(16.0, 8.0, 0.0, 0.0),
            );
            InfoBarPanel::set_vertical_orientation_margin(
                &action,
                Thickness::new(0.0, 12.0, 0.0, 0.0),
            );
            banner
                .children()
                .add(action.register_in_name_scope(&**scope).upcast::<Control>());
            grid.children()
                .add(banner.register_in_name_scope(&**scope).upcast::<Control>());
            let content = ContentPresenter::new();
            content.set_name(Some("PART_ContentPresenter".into()));
            content.bind_binding(
                ContentPresenter::content_property().as_property(),
                &TemplateBinding::new(ContentControl::content_property().as_property()),
            );
            content.bind_binding(
                ContentPresenter::content_template_property().as_property(),
                &TemplateBinding::new(ContentControl::content_template_property().as_property()),
            );
            Grid::set_column(&content, 1);
            Grid::set_row(&content, 1);
            grid.children()
                .add(content.register_in_name_scope(&**scope).upcast::<Control>());
            let close = Button::new();
            close.set_name(Some("CloseButton".into()));
            close.set_width(38.0);
            close.set_height(38.0);
            close.set_vertical_alignment(VerticalAlignment::Top);
            close.set_margin(Thickness::uniform(5.0));
            let close_glyph = Viewbox::new();
            close_glyph.set_name(Some("CloseGlyph".into()));
            close_glyph.set_width(16.0);
            close_glyph.set_height(16.0);
            close_glyph.set_child(SymbolIcon::with_symbol(Symbol::Cancel));
            close.set_content(Some(Control::boxed(
                close_glyph.register_in_name_scope(&**scope),
            )));
            Grid::set_column(&close, 2);
            grid.children()
                .add(close.register_in_name_scope(&**scope).upcast::<Control>());
            background_layer.set_child(grid);
            root.set_child(background_layer.register_in_name_scope(&**scope));
            root.register_in_name_scope(&**scope).upcast()
        });
        TemplatedControl::template_property().override_default_value::<InfoBar>(Some(template));
    }

    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            last_close_reason: Cell::new(InfoBarCloseReason::Programmatic),
            settings: InfoBarTemplateSettings::new(),
            parts: RefCell::new(Parts::default()),
            subscriptions: RefCell::new(Vec::new()),
            close_button_click: HandlerList::new(),
            closing: HandlerList::new(),
            closed: HandlerList::new(),
            opened: HandlerList::new(),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    pub fn is_open(&self) -> bool {
        self.get_value(Self::is_open_property())
    }
    pub fn set_is_open(&self, value: bool) {
        self.set_value(Self::is_open_property(), value);
    }
    pub fn title(&self) -> String {
        self.get_value(Self::title_property())
    }
    pub fn set_title(&self, value: impl Into<String>) {
        self.set_value(Self::title_property(), value.into());
    }
    pub fn message(&self) -> String {
        self.get_value(Self::message_property())
    }
    pub fn set_message(&self, value: impl Into<String>) {
        self.set_value(Self::message_property(), value.into());
    }
    pub fn severity(&self) -> InfoBarSeverity {
        self.get_value(Self::severity_property())
    }
    pub fn set_severity(&self, value: InfoBarSeverity) {
        self.set_value(Self::severity_property(), value);
    }
    pub fn icon_source(&self) -> Option<Ref<IconSource>> {
        self.get_value(Self::icon_source_property())
    }
    pub fn set_icon_source(&self, value: Option<Ref<IconSource>>) {
        self.set_value(Self::icon_source_property(), value);
    }
    pub fn is_icon_visible(&self) -> bool {
        self.get_value(Self::is_icon_visible_property())
    }
    pub fn set_is_icon_visible(&self, value: bool) {
        self.set_value(Self::is_icon_visible_property(), value);
    }
    pub fn is_closable(&self) -> bool {
        self.get_value(Self::is_closable_property())
    }
    pub fn set_is_closable(&self, value: bool) {
        self.set_value(Self::is_closable_property(), value);
    }
    pub fn close_button_style(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::close_button_style_property())
    }
    pub fn set_close_button_style(&self, value: Option<Ref<ControlTheme>>) {
        self.set_value(Self::close_button_style_property(), value);
    }
    pub fn close_button_command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::close_button_command_property())
    }
    pub fn set_close_button_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::close_button_command_property(), value);
    }
    pub fn close_button_command_parameter(&self) -> Option<BoxedValue> {
        self.get_value(Self::close_button_command_parameter_property())
    }
    pub fn set_close_button_command_parameter(&self, value: Option<BoxedValue>) {
        self.set_value(Self::close_button_command_parameter_property(), value);
    }
    pub fn action_button(&self) -> Option<Ref<Button>> {
        self.get_value(Self::action_button_property())
    }
    pub fn set_action_button(&self, value: Option<Ref<Button>>) {
        self.set_value(Self::action_button_property(), value);
    }
    pub fn template_settings(&self) -> Ref<InfoBarTemplateSettings> {
        self.settings.clone()
    }

    pub fn close_button_click(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.close_button_click.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.close_button_click.remove(token);
            }
        })
    }
    pub fn closing(
        &self,
        handler: impl Fn(&InfoBarClosingEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.closing.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.closing.remove(token);
            }
        })
    }
    pub fn closed(
        &self,
        handler: impl Fn(&InfoBarClosedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.closed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.closed.remove(token);
            }
        })
    }
    pub fn opened(
        &self,
        handler: impl Fn(&InfoBarOpenedEventArgs) + 'static,
    ) -> Rc<dyn IDisposable> {
        let token = self.opened.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.opened.remove(token);
            }
        })
    }

    fn close_clicked(&self) {
        for (_, handler) in self.close_button_click.snapshot().iter() {
            handler();
        }
        self.last_close_reason.set(InfoBarCloseReason::CloseButton);
        self.set_current_value(Self::is_open_property(), false);
    }

    fn open_state_changed(&self) {
        if self.is_open() {
            self.last_close_reason.set(InfoBarCloseReason::Programmatic);
            self.update_visibility();
            for (_, handler) in self.opened.snapshot().iter() {
                handler(&InfoBarOpenedEventArgs);
            }
        } else {
            let args = InfoBarClosingEventArgs {
                reason: self.last_close_reason.get(),
                cancel: Rc::new(Cell::new(false)),
            };
            for (_, handler) in self.closing.snapshot().iter() {
                handler(&args);
            }
            if args.cancel() {
                self.set_current_value(Self::is_open_property(), true);
            } else {
                self.update_visibility();
                let args = InfoBarClosedEventArgs {
                    reason: self.last_close_reason.get(),
                };
                for (_, handler) in self.closed.snapshot().iter() {
                    handler(&args);
                }
            }
        }
    }

    fn update_visibility(&self) {
        let root = self.parts.borrow().root.clone();
        if let Some(root) = root {
            root.set_is_visible(self.is_open());
        }
        AutomationProperties::set_accessibility_view(
            self,
            if self.is_open() {
                AccessibilityView::Control
            } else {
                AccessibilityView::Raw
            },
        );
        self.pseudo_classes().set(":open", self.is_open());
        self.invalidate_measure();
    }

    fn resource<T: Clone + 'static>(&self, key: &str) -> Option<T> {
        self.try_find_resource(
            &ResourceKey::from(key),
            self.actual_theme_variant().as_ref(),
        )
        .flatten()
        .and_then(|value| value.downcast_ref::<T>().cloned())
    }

    fn brush(&self, key: &str) -> Option<Rc<dyn IBrush>> {
        let value = self
            .try_find_resource(
                &ResourceKey::from(key),
                self.actual_theme_variant().as_ref(),
            )
            .flatten()?;
        let value = ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IBrush>>())?;
        value.downcast_ref::<Rc<dyn IBrush>>().cloned()
    }

    fn update_parts(&self) {
        let parts = self.parts.borrow().clone();
        let severity = self.severity();
        let suffix = match severity {
            InfoBarSeverity::Informational => "Informational",
            InfoBarSeverity::Success => "Success",
            InfoBarSeverity::Warning => "Warning",
            InfoBarSeverity::Error => "Error",
        };
        for (name, candidate) in [
            (":informational", InfoBarSeverity::Informational),
            (":success", InfoBarSeverity::Success),
            (":warning", InfoBarSeverity::Warning),
            (":error", InfoBarSeverity::Error),
        ] {
            self.pseudo_classes().set(name, severity == candidate);
        }
        let dark = self.actual_theme_variant().as_ref() == Some(&ThemeVariant::dark());
        let fallback_background = match (severity, dark) {
            (InfoBarSeverity::Informational, false) => 0x80F6F6F6,
            (InfoBarSeverity::Informational, true) => 0x08FFFFFF,
            (InfoBarSeverity::Success, false) => 0xFFDFF6DD,
            (InfoBarSeverity::Success, true) => 0xFF393D1B,
            (InfoBarSeverity::Warning, false) => 0xFFFFF4CE,
            (InfoBarSeverity::Warning, true) => 0xFF433519,
            (InfoBarSeverity::Error, false) => 0xFFFDE7E9,
            (InfoBarSeverity::Error, true) => 0xFF442726,
        };
        if let Some(root) = &parts.root {
            root.set_background(
                self.brush(&format!("InfoBar{suffix}SeverityBackgroundBrush"))
                    .or_else(|| Some(SolidColorBrush::from_uint32(fallback_background).into())),
            );
            root.set_border_brush(
                self.get_base_value(TemplatedControl::border_brush_property())
                    .unwrap_or_else(|| self.brush("InfoBarBorderBrush")),
            );
            root.set_border_thickness(
                self.get_base_value(TemplatedControl::border_thickness_property())
                    .unwrap_or_else(|| {
                        self.resource::<Thickness>("InfoBarBorderThickness")
                            .unwrap_or_else(|| self.border_thickness())
                    }),
            );
            root.set_corner_radius(
                self.get_base_value(TemplatedControl::corner_radius_property())
                    .unwrap_or_else(|| {
                        self.resource::<CornerRadius>("ControlCornerRadius")
                            .unwrap_or_else(|| self.corner_radius())
                    }),
            );
            root.set_min_height(self.resource::<f64>("InfoBarMinHeight").unwrap_or(48.0));
        }
        if let Some(background) = &parts.background_layer {
            // A translucent user background overlays the severity color, just
            // as the pinned template's inner grid overlays its outer border.
            background.set_background(self.background());
            background.set_corner_radius(self.corner_radius());
            background.set_padding(
                self.get_base_value(TemplatedControl::padding_property())
                    .unwrap_or_else(|| {
                        self.resource::<Thickness>("InfoBarContentRootPadding")
                            .unwrap_or_else(|| self.padding())
                    }),
            );
        }
        let title = self.title();
        let message = self.message();
        if let Some(text) = &parts.title {
            text.set_text(Some(&title));
            text.set_is_visible(!title.is_empty());
            text.set_foreground(
                self.get_base_value(TemplatedControl::foreground_property())
                    .unwrap_or_else(|| {
                        self.brush("InfoBarTitleForeground")
                            .or_else(|| self.foreground())
                    }),
            );
            text.set_font_size(self.resource::<f64>("InfoBarTitleFontSize").unwrap_or(14.0));
            text.set_font_weight(
                self.resource::<FontWeight>("InfoBarTitleFontWeight")
                    .unwrap_or(FontWeight::SemiBold),
            );
            InfoBarPanel::set_horizontal_orientation_margin(
                text,
                self.resource::<Thickness>("InfoBarTitleHorizontalOrientationMargin")
                    .unwrap_or(Thickness::new(0.0, 14.0, 0.0, 0.0)),
            );
            InfoBarPanel::set_vertical_orientation_margin(
                text,
                self.resource::<Thickness>("InfoBarTitleVerticalOrientationMargin")
                    .unwrap_or(Thickness::new(0.0, 14.0, 0.0, 0.0)),
            );
        }
        if let Some(text) = &parts.message {
            text.set_text(Some(&message));
            text.set_is_visible(!message.is_empty());
            text.set_foreground(
                self.get_base_value(TemplatedControl::foreground_property())
                    .unwrap_or_else(|| {
                        self.brush("InfoBarMessageForeground")
                            .or_else(|| self.foreground())
                    }),
            );
            text.set_font_size(
                self.resource::<f64>("InfoBarMessageFontSize")
                    .unwrap_or(14.0),
            );
            text.set_font_weight(
                self.resource::<FontWeight>("InfoBarMessageFontWeight")
                    .unwrap_or(FontWeight::Normal),
            );
            InfoBarPanel::set_horizontal_orientation_margin(
                text,
                self.resource::<Thickness>("InfoBarMessageHorizontalOrientationMargin")
                    .unwrap_or(Thickness::new(12.0, 14.0, 0.0, 0.0)),
            );
            InfoBarPanel::set_vertical_orientation_margin(
                text,
                self.resource::<Thickness>("InfoBarMessageVerticalOrientationMargin")
                    .unwrap_or(Thickness::new(0.0, 4.0, 0.0, 0.0)),
            );
        }
        let has_banner = !title.is_empty() || !message.is_empty() || self.action_button().is_some();
        if let Some(banner) = &parts.banner {
            banner.set_is_visible(has_banner);
            banner.set_margin(
                self.resource::<Thickness>("InfoBarPanelMargin")
                    .unwrap_or(Thickness::new(0.0, 0.0, 16.0, 0.0)),
            );
            banner.set_horizontal_orientation_padding(
                self.resource::<Thickness>("InfoBarPanelHorizontalOrientationPadding")
                    .unwrap_or_default(),
            );
            banner.set_vertical_orientation_padding(
                self.resource::<Thickness>("InfoBarPanelVerticalOrientationPadding")
                    .unwrap_or(Thickness::new(0.0, 14.0, 0.0, 18.0)),
            );
        }
        if let Some(action) = &parts.action {
            action.set_content(self.action_button().map(Control::boxed));
            action.set_is_visible(self.action_button().is_some());
            InfoBarPanel::set_horizontal_orientation_margin(
                action,
                self.resource::<Thickness>("InfoBarActionHorizontalOrientationMargin")
                    .unwrap_or(Thickness::new(16.0, 8.0, 0.0, 0.0)),
            );
            InfoBarPanel::set_vertical_orientation_margin(
                action,
                self.resource::<Thickness>("InfoBarActionVerticalOrientationMargin")
                    .unwrap_or(Thickness::new(0.0, 12.0, 0.0, 0.0)),
            );
        }
        if let Some(content) = &parts.content {
            content.set_is_visible(self.content().is_some());
            Grid::set_row(content, if has_banner { 1 } else { 0 });
            content.set_margin(Thickness::new(
                0.0,
                if has_banner { 0.0 } else { 14.0 },
                16.0,
                18.0,
            ));
        }
        let user_icon = self.settings.icon_element();
        let icon_size = self.resource::<f64>("InfoBarIconFontSize").unwrap_or(16.0);
        let icon_margin = self
            .resource::<Thickness>("InfoBarIconMargin")
            .unwrap_or(Thickness::new(0.0, 16.0, 14.0, 16.0));
        if let Some(area) = &parts.standard_area {
            area.set_is_visible(self.is_icon_visible() && user_icon.is_none());
        }
        if let Some(icon) = &parts.user_icon {
            icon.set_child(user_icon.clone().map(|icon| icon.upcast::<Control>()));
            icon.set_is_visible(self.is_icon_visible() && user_icon.is_some());
            icon.set_max_width(icon_size);
            icon.set_max_height(icon_size);
            icon.set_margin(icon_margin);
        }
        if let Some(icon) = &parts.standard_icon {
            let glyph = match severity {
                InfoBarSeverity::Informational => "\u{F13F}",
                InfoBarSeverity::Success => "\u{F13E}",
                InfoBarSeverity::Warning => "\u{F13C}",
                InfoBarSeverity::Error => "\u{F13D}",
            };
            icon.set_text(Some(
                &self
                    .resource::<String>(&format!("InfoBar{suffix}IconGlyph"))
                    .unwrap_or_else(|| glyph.to_owned()),
            ));
            icon.set_font_size(icon_size);
            icon.set_font_family(
                self.resource::<FontFamily>("SymbolThemeFontFamily")
                    .unwrap_or_else(|| FontFamily::new("Segoe Fluent Icons")),
            );
            icon.set_margin(icon_margin);
            icon.set_foreground(
                self.brush(&format!("InfoBar{suffix}SeverityIconForeground"))
                    .or_else(|| {
                        Some(
                            SolidColorBrush::from_uint32(if dark {
                                0xFF000000
                            } else {
                                0xFFFFFFFF
                            })
                            .into(),
                        )
                    }),
            );
            AutomationProperties::set_name(
                icon,
                Some(
                    &self
                        .resource::<String>(&format!("InfoBar{suffix}IconName"))
                        .unwrap_or_else(|| suffix.to_owned()),
                ),
            );
        }
        if let Some(background) = &parts.icon_background {
            background.set_text(Some(
                &self
                    .resource::<String>("InfoBarIconBackgroundGlyph")
                    .unwrap_or_else(|| "\u{F136}".to_owned()),
            ));
            background.set_font_size(icon_size);
            background.set_font_family(
                self.resource::<FontFamily>("SymbolThemeFontFamily")
                    .unwrap_or_else(|| FontFamily::new("Segoe Fluent Icons")),
            );
            background.set_margin(icon_margin);
            let color = match (severity, dark) {
                (InfoBarSeverity::Informational, false) => 0xFF0078D4,
                (InfoBarSeverity::Informational, true) => 0xFF60CDFF,
                (InfoBarSeverity::Success, false) => 0xFF0F7B0F,
                (InfoBarSeverity::Success, true) => 0xFF6CCB5F,
                (InfoBarSeverity::Warning, false) => 0xFF9D5D00,
                (InfoBarSeverity::Warning, true) => 0xFFFCE100,
                (InfoBarSeverity::Error, false) => 0xFFC42B1C,
                (InfoBarSeverity::Error, true) => 0xFFFF99A4,
            };
            background.set_foreground(
                self.brush(&format!("InfoBar{suffix}SeverityIconBackground"))
                    .or_else(|| Some(SolidColorBrush::from_uint32(color).into())),
            );
        }
        if let Some(close) = &parts.close {
            close.set_is_visible(self.is_closable());
            close.set_command(self.close_button_command());
            close.set_command_parameter(self.close_button_command_parameter());
            close.set_theme(
                self.get_base_value(Self::close_button_style_property())
                    .unwrap_or_else(|| {
                        self.resource::<Ref<ControlTheme>>("InfoBarCloseButtonStyle")
                    }),
            );
            let size = self
                .resource::<f64>("InfoBarCloseButtonSize")
                .unwrap_or(38.0);
            close.set_width(size);
            close.set_height(size);
            let close_name = self
                .resource::<String>("InfoBarCloseButtonName")
                .unwrap_or_else(|| "Close".into());
            if AutomationProperties::get_name(close).is_none_or(|name| name.is_empty()) {
                AutomationProperties::set_name(close, Some(&close_name));
            }
            let tooltip = self
                .resource::<String>("InfoBarCloseButtonTooltip")
                .unwrap_or(close_name);
            ToolTip::set_tip(close, Some(Rc::new(tooltip)));
        }
        // This metadata is consumed by native accessibility providers. The
        // Windows RaiseNotificationEvent channel is intentionally not claimed.
        AutomationProperties::set_live_setting(
            self,
            match severity {
                InfoBarSeverity::Warning | InfoBarSeverity::Error => {
                    AutomationLiveSetting::Assertive
                }
                _ => AutomationLiveSetting::Polite,
            },
        );
        self.invalidate_measure();
    }
}

impl Drop for InfoBar {
    fn drop(&mut self) {
        self.parts.get_mut().detach();
        for subscription in self.subscriptions.get_mut().drain(..) {
            subscription.dispose();
        }
    }
}

pub(crate) const TYPES: &[&MarkupType] = &[
    <InfoBarSeverity as MarkupTyped>::MARKUP,
    <InfoBarCloseReason as MarkupTyped>::MARKUP,
    <InfoBarClosingEventArgs as MarkupTyped>::MARKUP,
    <InfoBarClosedEventArgs as MarkupTyped>::MARKUP,
    <InfoBarOpenedEventArgs as MarkupTyped>::MARKUP,
];
pub(crate) fn register_markup() {
    MarkupType::register_all(TYPES);
}
