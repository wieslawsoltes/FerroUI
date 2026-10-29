//! WinUI's `InfoBar` (`controls/dev/InfoBar/InfoBar.cpp`, `InfoBar.h`, `InfoBar.idl` and the
//! closing, closed and opened arguments of microsoft-ui-xaml, commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).

use crate::automation::peers::{AutomationPeer, InfoBarAutomationPeer};
use crate::automation::{AccessibilityView, AutomationProperties};
use crate::primitives::{
    InfoBarTemplateSettings, TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
};
use crate::{Button, ContentControl, ContentControlImpl, Control, ControlImpl, IconSource, ToolTip};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::{ICommand, InputElementImpl};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::metadata::{into_markup_value, MarkupDelegate};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::styling::ControlTheme;
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_enum, ferro_markup_type, ferro_properties,
    ferro_property, instantiate, BoxedValue, DirectProperty, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, StyledProperty, VisualImpl, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The severity of an [`InfoBar`] (WinUI's `InfoBarSeverity`, with its values).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InfoBarSeverity {
    /// An informational message (the default).
    #[default]
    Informational = 0,
    /// A message of success.
    Success = 1,
    /// A warning.
    Warning = 2,
    /// An error.
    Error = 3,
}

/// Why an [`InfoBar`] closes (WinUI's `InfoBarCloseReason`, with its values).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum InfoBarCloseReason {
    /// The close button was clicked.
    CloseButton = 0,
    /// `IsOpen` was set to `false` (the default).
    #[default]
    Programmatic = 1,
}

ferro_markup_enum!(InfoBarSeverity { Informational, Success, Warning, Error }, { namespace: "FerroUI.Controls" });
ferro_markup_enum!(InfoBarCloseReason { CloseButton, Programmatic }, { namespace: "FerroUI.Controls" });

/// The arguments of [`InfoBar::closing`] (WinUI's `InfoBarClosingEventArgs`). A handler cancels the
/// closing with [`set_cancel`](Self::set_cancel); the clones of the arguments given to the
/// handlers of markup share the cancellation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InfoBarClosingEventArgs {
    reason: InfoBarCloseReason,
    cancel: Rc<Cell<bool>>,
}

impl InfoBarClosingEventArgs {
    /// Why the bar closes.
    pub fn reason(&self) -> InfoBarCloseReason {
        self.reason
    }

    /// Whether a handler cancelled the closing.
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }

    /// Cancels the closing (`true`) or lets it happen (`false`).
    pub fn set_cancel(&self, value: bool) {
        self.cancel.set(value);
    }
}

/// The arguments of [`InfoBar::closed`] (WinUI's `InfoBarClosedEventArgs`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InfoBarClosedEventArgs {
    reason: InfoBarCloseReason,
}

impl InfoBarClosedEventArgs {
    /// Why the bar closed.
    pub fn reason(&self) -> InfoBarCloseReason {
        self.reason
    }
}

/// The arguments of [`InfoBar::opened`] (WinUI's `InfoBarOpenedEventArgs`, a preview contract at
/// the pin): no members.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InfoBarOpenedEventArgs;

ferro_markup_type!(class InfoBarClosingEventArgs {
    namespace: "FerroUI.Controls",
    handles: [InfoBarClosingEventArgs],
    properties: [
        Reason: InfoBarCloseReason { get: |args: &InfoBarClosingEventArgs| args.reason() },
        Cancel: bool {
            get: |args: &InfoBarClosingEventArgs| args.cancel(),
            set: |args: &InfoBarClosingEventArgs, value: bool| args.set_cancel(value)
        },
    ],
});

ferro_markup_type!(class InfoBarClosedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [InfoBarClosedEventArgs],
    properties: [
        Reason: InfoBarCloseReason { get: |args: &InfoBarClosedEventArgs| args.reason() },
    ],
});

ferro_markup_type!(class InfoBarOpenedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [InfoBarOpenedEventArgs],
});

// The texts WinUI takes from its localized resources (`controls/dev/InfoBar/Strings/en-us/
// Resources.resw`), in English: FerroUI has no localized resources of its framework.
const CLOSE_BUTTON_NAME: &str = "Close";
const CLOSE_BUTTON_TOOLTIP: &str = "Close";

const PC_INFORMATIONAL: &str = ":informational";
const PC_SUCCESS: &str = ":success";
const PC_WARNING: &str = ":warning";
const PC_ERROR: &str = ":error";
const PC_STANDARD_ICON_VISIBLE: &str = ":standard-icon-visible";
const PC_USER_ICON_VISIBLE: &str = ":user-icon-visible";
const PC_CLOSE_BUTTON_COLLAPSED: &str = ":close-button-collapsed";
const PC_OPEN: &str = ":open";
const PC_FOREGROUND_SET: &str = ":foreground-set";
const PC_NO_BANNER_CONTENT: &str = ":no-banner-content";

/// An inline message of the application (WinUI's `InfoBar`): a title, a message, an action button
/// and content, with the icon and the colours of its [`severity`](Self::severity), shown while
/// [`is_open`](Self::is_open) and closed by its close button or by the application.
///
/// The control sets the states of WinUI as pseudo-classes, which the control theme of the WinUI
/// theme (`src/FerroUI.Themes.WinUI/Controls/InfoBar.xaml`) styles: `:informational`,
/// `:success`, `:warning`, `:error` (SeverityLevels), `:standard-icon-visible`,
/// `:user-icon-visible` (IconStates; neither is NoIconVisible), `:close-button-collapsed`,
/// `:open` (InfoBarVisible; its absence is InfoBarCollapsed), `:foreground-set` (a `Foreground`
/// set on the bar itself) and `:no-banner-content` (no title, message or action button). The
/// template parts it looks up are the button `CloseButton` and the severity icon `StandardIcon`;
/// the content is presented by `PART_ContentPresenter` (WinUI's `ContentArea`). The Fluent and
/// Simple themes have no template for it (docs/porting/winui-controls.md, section 5.1).
///
/// Differences from WinUI (docs/porting/winui-controls.md, section 10.2): the bar is a
/// [`ContentControl`]; UI Automation notifications are not raised (the peer reports the title and
/// the message as the name and a live setting of the severity instead); the texts of the close
/// button and the severity icons are WinUI's English texts.
#[repr(C)]
pub struct InfoBar {
    base: ContentControl,
    apply_template_called: Cell<bool>,
    is_visible: Cell<bool>,
    last_close_reason: Cell<InfoBarCloseReason>,
    foreground_is_local: Cell<bool>,
    settings: Ref<InfoBarTemplateSettings>,
    close_button: RefCell<Option<(WeakRef<Button>, RoutedEventHandlerToken)>>,
    standard_icon: RefCell<Option<WeakRef<Control>>>,
    close_button_click: HandlerList<dyn Fn()>,
    closing: HandlerList<dyn Fn(&InfoBarClosingEventArgs)>,
    closed: HandlerList<dyn Fn(&InfoBarClosedEventArgs)>,
    opened: HandlerList<dyn Fn(&InfoBarOpenedEventArgs)>,
}

ferro_class!(InfoBar: ContentControl);
ferro_class_info!(InfoBar {
    new: InfoBar::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: Content,
        events: [
            CloseButtonClick(Option<BoxedValue>, Option<BoxedValue>) => |this: &Ref<InfoBar>, handler: MarkupDelegate| {
                let sender = this.downgrade();
                this.close_button_click(move || {
                    if let Some(sender) = sender.upgrade() {
                        handler.invoke(&[into_markup_value(Some(sender)), into_markup_value(None::<BoxedValue>)]);
                    }
                })
            },
            Closing(Option<BoxedValue>, InfoBarClosingEventArgs) => |this: &Ref<InfoBar>, handler: MarkupDelegate| {
                let sender = this.downgrade();
                this.closing(move |args: &InfoBarClosingEventArgs| {
                    if let Some(sender) = sender.upgrade() {
                        handler.invoke(&[into_markup_value(Some(sender)), into_markup_value(args.clone())]);
                    }
                })
            },
            Closed(Option<BoxedValue>, InfoBarClosedEventArgs) => |this: &Ref<InfoBar>, handler: MarkupDelegate| {
                let sender = this.downgrade();
                this.closed(move |args: &InfoBarClosedEventArgs| {
                    if let Some(sender) = sender.upgrade() {
                        handler.invoke(&[into_markup_value(Some(sender)), into_markup_value(args.clone())]);
                    }
                })
            },
            Opened(Option<BoxedValue>, InfoBarOpenedEventArgs) => |this: &Ref<InfoBar>, handler: MarkupDelegate| {
                let sender = this.downgrade();
                this.opened(move |args: &InfoBarOpenedEventArgs| {
                    if let Some(sender) = sender.upgrade() {
                        handler.invoke(&[into_markup_value(Some(sender)), into_markup_value(args.clone())]);
                    }
                })
            },
        ],
        attributes: [
            TemplatePart("CloseButton", type(Ref<Button>)),
            TemplatePart("StandardIcon", type(Ref<Control>)),
            PseudoClasses(
                ":informational",
                ":success",
                ":warning",
                ":error",
                ":standard-icon-visible",
                ":user-icon-visible",
                ":close-button-collapsed",
                ":open",
                ":foreground-set",
                ":no-banner-content"
            ),
        ],
    },
});
ferro_impl_classes!(
    InfoBar: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ContentControlImpl
);

impl FerroObjectImpl for InfoBar {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        // The states that do not wait for the template (InfoBar::OnApplyTemplate sets them all
        // again).
        this.update_severity();
        this.update_icon_visibility();
        this.update_close_button();
        this.update_foreground();
        this.update_content_position();
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        let property = change.property();
        if property == TemplatedControl::foreground_property().as_property() {
            // WinUI asks whether the bar has a local Foreground (ReadLocalValue); the port follows
            // the priority of the changes of the property (docs/porting/winui-controls.md, 10.2).
            this.track_local_foreground(change);
            this.update_foreground();
            return;
        }
        if !change.is_effective_value_change() {
            return;
        }
        if property == Self::is_open_property().as_property() {
            this.on_is_open_property_changed();
        } else if property == Self::title_property().as_property()
            || property == Self::message_property().as_property()
            || property == Self::action_button_property().as_property()
        {
            this.update_content_position();
        } else if property == Self::severity_property().as_property() {
            this.update_severity();
        } else if property == Self::icon_source_property().as_property() {
            this.update_icon();
            this.update_icon_visibility();
        } else if property == Self::is_icon_visible_property().as_property() {
            this.update_icon_visibility();
        } else if property == Self::is_closable_property().as_property() {
            this.update_close_button();
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
        this.detach_close_button();
        Self::parent_on_apply_template(this, args);
        this.apply_template_called.set(true);
        let scope = args.name_scope();

        if let Some(close_button) = scope.find_as::<Button>("CloseButton") {
            let weak = this.to_ref().downgrade();
            let token = close_button.click(move |_, _| {
                if let Some(owner) = weak.upgrade() {
                    owner.on_close_button_click();
                }
            });
            *this.close_button.borrow_mut() = Some((close_button.downgrade(), token));

            // The name and the tool tip of the close button.
            if AutomationProperties::get_name(&close_button).is_none_or(|name| name.is_empty()) {
                AutomationProperties::set_name(&close_button, Some(CLOSE_BUTTON_NAME));
            }
            ToolTip::set_tip(&close_button, Some(Rc::new(CLOSE_BUTTON_TOOLTIP.to_string()) as BoxedValue));
        }

        let standard_icon = scope.find_as::<Control>("StandardIcon");
        if let Some(icon) = &standard_icon {
            AutomationProperties::set_name(icon, Some(Self::icon_severity_level_name(this.severity())));
        }
        *this.standard_icon.borrow_mut() = standard_icon.map(|icon| icon.downgrade());

        this.update_visibility(true);
        this.update_severity();
        this.update_icon();
        this.update_icon_visibility();
        this.update_close_button();
        this.update_foreground();
        this.update_content_position();
    }
}

ferro_properties! { impl InfoBar {
    ferro_property!(
        /// Defines the `IsOpen` property: whether the bar is shown. `false` by default.
        pub fn is_open_property() -> StyledProperty<bool> {
            FerroProperty::register::<InfoBar, _>("IsOpen", false)
        }
    );

    ferro_property!(
        /// Defines the `Title` property: the title of the message.
        pub fn title_property() -> StyledProperty<String> {
            FerroProperty::register::<InfoBar, _>("Title", String::new())
        }
    );

    ferro_property!(
        /// Defines the `Message` property: the text of the message.
        pub fn message_property() -> StyledProperty<String> {
            FerroProperty::register::<InfoBar, _>("Message", String::new())
        }
    );

    ferro_property!(
        /// Defines the `Severity` property: the kind of the message, which selects its icon and
        /// colours. `Informational` by default.
        pub fn severity_property() -> StyledProperty<InfoBarSeverity> {
            FerroProperty::register::<InfoBar, _>("Severity", InfoBarSeverity::Informational)
        }
    );

    ferro_property!(
        /// Defines the `IconSource` property: an icon of the application shown in place of the
        /// icon of the severity.
        pub fn icon_source_property() -> StyledProperty<Option<Ref<IconSource>>> {
            FerroProperty::register::<InfoBar, _>("IconSource", None)
        }
    );

    ferro_property!(
        /// Defines the `IsIconVisible` property: whether an icon is shown. `true` by default.
        pub fn is_icon_visible_property() -> StyledProperty<bool> {
            FerroProperty::register::<InfoBar, _>("IsIconVisible", true)
        }
    );

    ferro_property!(
        /// Defines the `IsClosable` property: whether the close button is shown. `true` by
        /// default.
        pub fn is_closable_property() -> StyledProperty<bool> {
            FerroProperty::register::<InfoBar, _>("IsClosable", true)
        }
    );

    ferro_property!(
        /// Defines the `CloseButtonStyle` property: the control theme of the close button
        /// (WinUI's style; the WinUI theme sets `InfoBarCloseButtonStyle`).
        pub fn close_button_style_property() -> StyledProperty<Option<Ref<ControlTheme>>> {
            FerroProperty::register::<InfoBar, _>("CloseButtonStyle", None)
        }
    );

    ferro_property!(
        /// Defines the `CloseButtonCommand` property: the command of the close button.
        pub fn close_button_command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            FerroProperty::register::<InfoBar, _>("CloseButtonCommand", None)
        }
    );

    ferro_property!(
        /// Defines the `CloseButtonCommandParameter` property: the parameter of the command of the
        /// close button.
        pub fn close_button_command_parameter_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<InfoBar, _>("CloseButtonCommandParameter", None)
        }
    );

    ferro_property!(
        /// Defines the `ActionButton` property: a button shown after the message.
        pub fn action_button_property() -> StyledProperty<Option<Ref<Button>>> {
            FerroProperty::register::<InfoBar, _>("ActionButton", None)
        }
    );

    ferro_property!(
        /// Defines the `TemplateSettings` property: the values the template binds to (read-only).
        pub fn template_settings_property() -> DirectProperty<InfoBar, Ref<InfoBarTemplateSettings>> {
            FerroProperty::register_direct::<InfoBar, _>(
                "TemplateSettings",
                |owner| owner.template_settings(),
                None,
                InfoBarTemplateSettings::new(),
            )
        }
    );
} }

impl InfoBar {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            apply_template_called: Cell::new(false),
            is_visible: Cell::new(false),
            last_close_reason: Cell::new(InfoBarCloseReason::Programmatic),
            foreground_is_local: Cell::new(false),
            settings: InfoBarTemplateSettings::new(),
            close_button: RefCell::new(None),
            standard_icon: RefCell::new(None),
            close_button_click: HandlerList::new(),
            closing: HandlerList::new(),
            closed: HandlerList::new(),
            opened: HandlerList::new(),
        }
    }

    /// Creates an info bar.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets whether the bar is shown.
    pub fn is_open(&self) -> bool {
        self.get_value(Self::is_open_property())
    }

    /// Sets whether the bar is shown.
    pub fn set_is_open(&self, value: bool) {
        self.set_value(Self::is_open_property(), value);
    }

    /// Gets the title.
    pub fn title(&self) -> String {
        self.get_value(Self::title_property())
    }

    /// Sets the title.
    pub fn set_title(&self, value: impl Into<String>) {
        self.set_value(Self::title_property(), value.into());
    }

    /// Gets the message.
    pub fn message(&self) -> String {
        self.get_value(Self::message_property())
    }

    /// Sets the message.
    pub fn set_message(&self, value: impl Into<String>) {
        self.set_value(Self::message_property(), value.into());
    }

    /// Gets the severity.
    pub fn severity(&self) -> InfoBarSeverity {
        self.get_value(Self::severity_property())
    }

    /// Sets the severity.
    pub fn set_severity(&self, value: InfoBarSeverity) {
        self.set_value(Self::severity_property(), value);
    }

    /// Gets the icon of the application.
    pub fn icon_source(&self) -> Option<Ref<IconSource>> {
        self.get_value(Self::icon_source_property())
    }

    /// Sets the icon of the application.
    pub fn set_icon_source(&self, value: Option<Ref<IconSource>>) {
        self.set_value(Self::icon_source_property(), value);
    }

    /// Gets whether an icon is shown.
    pub fn is_icon_visible(&self) -> bool {
        self.get_value(Self::is_icon_visible_property())
    }

    /// Sets whether an icon is shown.
    pub fn set_is_icon_visible(&self, value: bool) {
        self.set_value(Self::is_icon_visible_property(), value);
    }

    /// Gets whether the close button is shown.
    pub fn is_closable(&self) -> bool {
        self.get_value(Self::is_closable_property())
    }

    /// Sets whether the close button is shown.
    pub fn set_is_closable(&self, value: bool) {
        self.set_value(Self::is_closable_property(), value);
    }

    /// Gets the control theme of the close button.
    pub fn close_button_style(&self) -> Option<Ref<ControlTheme>> {
        self.get_value(Self::close_button_style_property())
    }

    /// Sets the control theme of the close button.
    pub fn set_close_button_style(&self, value: Option<Ref<ControlTheme>>) {
        self.set_value(Self::close_button_style_property(), value);
    }

    /// Gets the command of the close button.
    pub fn close_button_command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::close_button_command_property())
    }

    /// Sets the command of the close button.
    pub fn set_close_button_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::close_button_command_property(), value);
    }

    /// Gets the parameter of the command of the close button.
    pub fn close_button_command_parameter(&self) -> Option<BoxedValue> {
        self.get_value(Self::close_button_command_parameter_property())
    }

    /// Sets the parameter of the command of the close button.
    pub fn set_close_button_command_parameter(&self, value: Option<BoxedValue>) {
        self.set_value(Self::close_button_command_parameter_property(), value);
    }

    /// Gets the action button.
    pub fn action_button(&self) -> Option<Ref<Button>> {
        self.get_value(Self::action_button_property())
    }

    /// Sets the action button.
    pub fn set_action_button(&self, value: Option<Ref<Button>>) {
        self.set_value(Self::action_button_property(), value);
    }

    /// Gets the values the template binds to; each bar has its own.
    pub fn template_settings(&self) -> Ref<InfoBarTemplateSettings> {
        self.settings.clone()
    }

    /// Adds a handler of `CloseButtonClick`, raised when the close button is clicked, before the
    /// bar closes. Disposing the result removes it.
    pub fn close_button_click(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.close_button_click.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.close_button_click.remove(token);
            }
        })
    }

    /// Adds a handler of `Closing`, raised when the bar is about to close; a handler may cancel
    /// it. Disposing the result removes it.
    pub fn closing(&self, handler: impl Fn(&InfoBarClosingEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.closing.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.closing.remove(token);
            }
        })
    }

    /// Adds a handler of `Closed`, raised when the bar has closed. Disposing the result removes
    /// it.
    pub fn closed(&self, handler: impl Fn(&InfoBarClosedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.closed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.closed.remove(token);
            }
        })
    }

    /// Adds a handler of `Opened`, raised when the bar opens. Disposing the result removes it.
    pub fn opened(&self, handler: impl Fn(&InfoBarOpenedEventArgs) + 'static) -> Rc<dyn IDisposable> {
        let token = self.opened.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(owner) = weak.upgrade() {
                owner.opened.remove(token);
            }
        })
    }

    /// InfoBar::OnCloseButtonClick: the event, then the bar closes because of the button. The
    /// current value is set, so that a binding of `IsOpen` stays.
    fn on_close_button_click(&self) {
        for (_, handler) in self.close_button_click.snapshot().iter() {
            handler();
        }
        self.last_close_reason.set(InfoBarCloseReason::CloseButton);
        self.set_current_value(Self::is_open_property(), false);
    }

    /// InfoBar::RaiseClosingEvent: a cancelled closing opens the bar again.
    fn raise_closing_event(&self) {
        let args = InfoBarClosingEventArgs { reason: self.last_close_reason.get(), cancel: Rc::new(Cell::new(false)) };
        for (_, handler) in self.closing.snapshot().iter() {
            handler(&args);
        }
        if !args.cancel() {
            self.update_visibility(false);
            self.raise_closed_event();
        } else {
            // A handler cancelled the closing: the bar opens again.
            self.set_current_value(Self::is_open_property(), true);
        }
    }

    fn raise_closed_event(&self) {
        let args = InfoBarClosedEventArgs { reason: self.last_close_reason.get() };
        for (_, handler) in self.closed.snapshot().iter() {
            handler(&args);
        }
    }

    fn raise_opened_event(&self) {
        for (_, handler) in self.opened.snapshot().iter() {
            handler(&InfoBarOpenedEventArgs);
        }
    }

    /// InfoBar::OnIsOpenPropertyChanged.
    fn on_is_open_property_changed(&self) {
        if self.is_open() {
            // The reason of the next closing is programmatic unless the button closes the bar.
            self.last_close_reason.set(InfoBarCloseReason::Programmatic);
            self.update_visibility(false);
            self.raise_opened_event();
        } else {
            self.raise_closing_event();
        }
    }

    /// InfoBar::UpdateVisibility: the state InfoBarVisible (`:open`) or InfoBarCollapsed and the
    /// accessibility view of the bar, once the template is applied. The notifications WinUI raises
    /// to UI Automation here are not ported.
    fn update_visibility(&self, force: bool) {
        if !self.apply_template_called.get() {
            // OnApplyTemplate updates the visibility.
            return;
        }
        let is_open = self.is_open();
        if force || is_open != self.is_visible.get() {
            self.pseudo_classes().set(PC_OPEN, is_open);
            AutomationProperties::set_accessibility_view(
                self,
                if is_open { AccessibilityView::Control } else { AccessibilityView::Raw },
            );
            self.is_visible.set(is_open);
        }
    }

    /// InfoBar::UpdateSeverity: the state of the severity and the name of its icon.
    fn update_severity(&self) {
        let severity = self.severity();
        let classes = self.pseudo_classes();
        classes.set(PC_INFORMATIONAL, severity == InfoBarSeverity::Informational);
        classes.set(PC_SUCCESS, severity == InfoBarSeverity::Success);
        classes.set(PC_WARNING, severity == InfoBarSeverity::Warning);
        classes.set(PC_ERROR, severity == InfoBarSeverity::Error);
        let icon = self.standard_icon.borrow().as_ref().and_then(WeakRef::upgrade);
        if let Some(icon) = icon {
            AutomationProperties::set_name(&icon, Some(Self::icon_severity_level_name(severity)));
        }
    }

    /// InfoBar::UpdateIcon: the icon element of the icon source, for the template.
    fn update_icon(&self) {
        self.settings.set_icon_element(self.icon_source().and_then(|source| source.create_icon_element()));
    }

    /// InfoBar::UpdateIconVisibility: UserIconVisible, StandardIconVisible or NoIconVisible.
    fn update_icon_visibility(&self) {
        let visible = self.is_icon_visible();
        let user_icon = self.icon_source().is_some();
        self.pseudo_classes().set(PC_USER_ICON_VISIBLE, visible && user_icon);
        self.pseudo_classes().set(PC_STANDARD_ICON_VISIBLE, visible && !user_icon);
    }

    /// InfoBar::UpdateCloseButton: CloseButtonVisible or CloseButtonCollapsed.
    fn update_close_button(&self) {
        self.pseudo_classes().set(PC_CLOSE_BUTTON_COLLAPSED, !self.is_closable());
    }

    /// InfoBar::UpdateForeground: ForegroundSet when the bar has a local `Foreground`, so that the
    /// title and the message take it.
    fn update_foreground(&self) {
        self.pseudo_classes().set(PC_FOREGROUND_SET, self.foreground_is_local.get());
    }

    /// InfoBar::UpdateContentPosition: the content moves up when there is no title, message or
    /// action button (NoBannerContent).
    fn update_content_position(&self) {
        let no_banner = self.title().is_empty() && self.message().is_empty() && self.action_button().is_none();
        self.pseudo_classes().set(PC_NO_BANNER_CONTENT, no_banner);
    }

    /// Whether `Foreground` has a local value, from the priority of its changes: a change at the
    /// local priority (a value or a binding set on the bar) makes it local; a change of the
    /// effective value at a lower priority (the local value was cleared) makes it not local; a
    /// change of an animation leaves it as it is.
    fn track_local_foreground(&self, change: &FerroPropertyChangedEventArgs<'_>) {
        match change.priority() {
            BindingPriority::LocalValue => self.foreground_is_local.set(true),
            BindingPriority::Animation => {}
            _ if change.is_effective_value_change() => self.foreground_is_local.set(false),
            _ => {}
        }
    }

    /// The name of the icon of a severity (InfoBar::GetIconSeverityLevelResourceName, the English
    /// texts of its resources).
    fn icon_severity_level_name(severity: InfoBarSeverity) -> &'static str {
        match severity {
            InfoBarSeverity::Informational => "Informational icon",
            InfoBarSeverity::Success => "Success icon",
            InfoBarSeverity::Warning => "Warning icon",
            InfoBarSeverity::Error => "Error icon",
        }
    }

    fn detach_close_button(&self) {
        if let Some((button, token)) = self.close_button.borrow_mut().take() {
            if let Some(button) = button.upgrade() {
                button.remove_handler(Button::click_event(), token);
            }
        }
    }
}

impl Drop for InfoBar {
    fn drop(&mut self) {
        if let Some((button, token)) = self.close_button.get_mut().take() {
            if let Some(button) = button.upgrade() {
                button.remove_handler(Button::click_event(), token);
            }
        }
    }
}
