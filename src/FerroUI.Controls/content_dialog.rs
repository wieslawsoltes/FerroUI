//! WinUI's `ContentDialog` (`dxaml/xcp/dxaml/lib/ContentDialog_Partial.cpp` and
//! `controls/dev/CommonStyles/ContentDialog_themeresources.xaml` of microsoft-ui-xaml commit
//! `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md).
//!
//! The dialog is hosted in the [`OverlayLayer`] of the window of the element it is shown for, over
//! a smoke layer that takes the pointer, with the keyboard focus kept inside it. The visual states
//! of the template are pseudo-classes: `:showing`, `:full-size`, one of the button states
//! ([`BUTTON_STATES`]) and one of `:primary-default`, `:secondary-default` and `:close-default`.

use crate::primitives::{OverlayLayer, TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt};
use crate::templates::IDataTemplate;
use crate::{Button, ContentControl, ContentControlImpl, Control, ControlImpl, TopLevel};
use ferroui_base::animation::easings::{Easing, SplineEasing};
use ferroui_base::animation::{
    Animation, Cue, FillMode, IAnimationSetter, IGlobalClock, KeyFrame, SignedDuration,
};
use ferroui_base::input::{
    FocusManager, ICommand, InputElement, InputElementImpl, InputElementImplExt, Key, KeyEventArgs,
};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::ScaleTransform;
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::styling::Setter;
use ferroui_base::threading::{CancellationToken, Dispatcher};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, BoxedValue, FerroLocator,
    FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, LocatorExtensions, Ref, Size,
    StyledElementImpl, StyledProperty, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

/// `ControlNormalAnimationDuration`: the scale of the dialog when it opens.
const SHOW_SCALE_MS: f64 = 250.0;
/// `ControlFastAnimationDuration`: the scale of the dialog when it closes.
const HIDE_SCALE_MS: f64 = 167.0;
/// `ControlFasterAnimationDuration`: the fade of the dialog.
const FADE_MS: f64 = 83.0;
/// The scale the dialog opens from and closes to.
const OUTER_SCALE: f64 = 1.05;

/// The pseudo-classes of WinUI's `ButtonsVisibilityStates`.
pub(crate) const BUTTON_STATES: [&str; 8] = [
    ":all-visible",
    ":none-visible",
    ":primary-visible",
    ":secondary-visible",
    ":close-visible",
    ":primary-secondary-visible",
    ":primary-close-visible",
    ":secondary-close-visible",
];

/// The button state of the buttons that have a text.
pub(crate) fn button_state(primary: bool, secondary: bool, close: bool) -> &'static str {
    match (primary, secondary, close) {
        (true, true, true) => ":all-visible",
        (false, false, false) => ":none-visible",
        (true, false, false) => ":primary-visible",
        (false, true, false) => ":secondary-visible",
        (false, false, true) => ":close-visible",
        (true, true, false) => ":primary-secondary-visible",
        (true, false, true) => ":primary-close-visible",
        (false, true, true) => ":secondary-close-visible",
    }
}

/// The button of a dialog that Enter invokes (WinUI's `ContentDialogButton`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ContentDialogButton {
    /// No button is the default.
    #[default]
    None = 0,
    /// The primary button.
    Primary = 1,
    /// The secondary button.
    Secondary = 2,
    /// The close button.
    Close = 3,
}

/// How a dialog was closed (WinUI's `ContentDialogResult`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ContentDialogResult {
    /// Closed by the close button, Escape or [`ContentDialog::hide`].
    #[default]
    None = 0,
    /// Closed by the primary button.
    Primary = 1,
    /// Closed by the secondary button.
    Secondary = 2,
}

/// Why a dialog could not be shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContentDialogError {
    /// This dialog is already open.
    AlreadyOpen,
    /// Another dialog is open in the same window: a window shows one at a time.
    AnotherDialogOpen,
    /// The element the dialog is shown for is not in a window.
    NoHost,
}

impl fmt::Display for ContentDialogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::AlreadyOpen => "the dialog is already open",
            Self::AnotherDialogOpen => "another dialog is open in this window",
            Self::NoHost => "the element the dialog is shown for is not in a window",
        })
    }
}

impl std::error::Error for ContentDialogError {}

/// The deferrals of one event: the continuation runs when the last one completes.
#[derive(Default)]
struct DeferralState {
    outstanding: Cell<usize>,
    continuation: RefCell<Option<Box<dyn FnOnce()>>>,
}

impl DeferralState {
    fn complete_one(&self) {
        let outstanding = self.outstanding.get().saturating_sub(1);
        self.outstanding.set(outstanding);
        if outstanding == 0 {
            let continuation = self.continuation.borrow_mut().take();
            if let Some(continuation) = continuation {
                continuation();
            }
        }
    }

    fn run_when_complete(&self, continuation: impl FnOnce() + 'static) {
        if self.outstanding.get() == 0 {
            continuation();
        } else {
            *self.continuation.borrow_mut() = Some(Box::new(continuation));
        }
    }
}

/// Holds an event of a dialog open until it completes. Dropping it completes it, so that a handler
/// that fails cannot leave the dialog waiting forever.
pub struct ContentDialogDeferral {
    state: Option<Rc<DeferralState>>,
}

impl ContentDialogDeferral {
    fn new(state: &Rc<DeferralState>) -> Self {
        state.outstanding.set(state.outstanding.get() + 1);
        Self { state: Some(state.clone()) }
    }

    /// Lets the event continue.
    pub fn complete(mut self) {
        self.finish();
    }

    fn finish(&mut self) {
        if let Some(state) = self.state.take() {
            state.complete_one();
        }
    }
}

impl Drop for ContentDialogDeferral {
    fn drop(&mut self) {
        self.finish();
    }
}

/// The arguments of the button click events of a dialog.
#[derive(Clone)]
pub struct ContentDialogButtonClickEventArgs {
    cancel: Rc<Cell<bool>>,
    deferral: Rc<DeferralState>,
}

impl ContentDialogButtonClickEventArgs {
    /// Whether a handler keeps the dialog open.
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }

    /// Keeps the dialog open (`true`).
    pub fn set_cancel(&self, value: bool) {
        self.cancel.set(value);
    }

    /// Holds the click until the deferral completes.
    pub fn get_deferral(&self) -> ContentDialogDeferral {
        ContentDialogDeferral::new(&self.deferral)
    }
}

/// The arguments of [`ContentDialog::closing`].
#[derive(Clone)]
pub struct ContentDialogClosingEventArgs {
    result: ContentDialogResult,
    cancel: Rc<Cell<bool>>,
    deferral: Rc<DeferralState>,
}

impl ContentDialogClosingEventArgs {
    /// The result the dialog closes with.
    pub fn result(&self) -> ContentDialogResult {
        self.result
    }

    /// Whether a handler keeps the dialog open.
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }

    /// Keeps the dialog open (`true`).
    pub fn set_cancel(&self, value: bool) {
        self.cancel.set(value);
    }

    /// Holds the closing until the deferral completes.
    pub fn get_deferral(&self) -> ContentDialogDeferral {
        ContentDialogDeferral::new(&self.deferral)
    }
}

/// The arguments of [`ContentDialog::closed`].
#[derive(Clone, Copy, Debug)]
pub struct ContentDialogClosedEventArgs {
    result: ContentDialogResult,
}

impl ContentDialogClosedEventArgs {
    /// The result the dialog closed with.
    pub fn result(&self) -> ContentDialogResult {
        self.result
    }
}

#[derive(Default)]
struct OperationState {
    result: Cell<Option<ContentDialogResult>>,
    waker: RefCell<Option<Waker>>,
}

impl OperationState {
    fn complete(&self, result: ContentDialogResult) {
        self.result.set(Some(result));
        let waker = self.waker.borrow_mut().take();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

/// The open dialog, as a future of its result. Dropping it leaves the dialog open.
#[derive(Clone)]
pub struct ContentDialogOperation {
    state: Rc<OperationState>,
}

impl ContentDialogOperation {
    /// The result, once the dialog has closed.
    pub fn result(&self) -> Option<ContentDialogResult> {
        self.state.result.get()
    }
}

impl Future for ContentDialogOperation {
    type Output = ContentDialogResult;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.state.result.get() {
            Some(result) => Poll::Ready(result),
            None => {
                *self.state.waker.borrow_mut() = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// The named parts of the template.
struct Parts {
    background: Ref<Control>,
    title: Option<Ref<Control>>,
    content: Option<Ref<Control>>,
    command_space: Option<Ref<Control>>,
    primary: Option<Ref<Button>>,
    secondary: Option<Ref<Button>>,
    close: Option<Ref<Button>>,
    tokens: Vec<(Ref<Button>, RoutedEventHandlerToken)>,
}

type Handlers<T> = Rc<HandlerList<dyn Fn(&T)>>;

/// A modal dialog over the content of its window (WinUI's `ContentDialog`).
#[repr(C)]
pub struct ContentDialog {
    base: ContentControl,
    parts: RefCell<Option<Parts>>,
    operation: RefCell<Option<Rc<OperationState>>>,
    hide_in_progress: Cell<bool>,
    click_in_progress: Cell<bool>,
    previous_focus: RefCell<Option<WeakRef<InputElement>>>,
    opened: Rc<HandlerList<dyn Fn()>>,
    closing: Handlers<ContentDialogClosingEventArgs>,
    closed: Handlers<ContentDialogClosedEventArgs>,
    primary_click: Handlers<ContentDialogButtonClickEventArgs>,
    secondary_click: Handlers<ContentDialogButtonClickEventArgs>,
    close_click: Handlers<ContentDialogButtonClickEventArgs>,
}

ferro_class!(ContentDialog: ContentControl);
ferro_class_info!(ContentDialog {
    new: ContentDialog::new,
    markup: { namespace: "FerroUI.Controls", content: Content },
});
ferro_impl_classes!(ContentDialog: StyledElementImpl, InteractiveImpl, ControlImpl, ContentControlImpl);

ferro_properties! {
    impl ContentDialog {
        /// Defines the `Title` property.
        pub fn title_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ContentDialog, _>("Title", None)
        }

        /// Defines the `TitleTemplate` property.
        pub fn title_template_property() -> StyledProperty<Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register::<ContentDialog, _>("TitleTemplate", None)
        }

        /// Defines the `PrimaryButtonText` property.
        pub fn primary_button_text_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<ContentDialog, _>("PrimaryButtonText", None)
        }

        /// Defines the `SecondaryButtonText` property.
        pub fn secondary_button_text_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<ContentDialog, _>("SecondaryButtonText", None)
        }

        /// Defines the `CloseButtonText` property.
        pub fn close_button_text_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<ContentDialog, _>("CloseButtonText", None)
        }

        /// Defines the `IsPrimaryButtonEnabled` property.
        pub fn is_primary_button_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<ContentDialog, _>("IsPrimaryButtonEnabled", true)
        }

        /// Defines the `IsSecondaryButtonEnabled` property.
        pub fn is_secondary_button_enabled_property() -> StyledProperty<bool> {
            FerroProperty::register::<ContentDialog, _>("IsSecondaryButtonEnabled", true)
        }

        /// Defines the `DefaultButton` property.
        pub fn default_button_property() -> StyledProperty<ContentDialogButton> {
            FerroProperty::register::<ContentDialog, _>("DefaultButton", ContentDialogButton::None)
        }

        /// Defines the `FullSizeDesired` property.
        pub fn full_size_desired_property() -> StyledProperty<bool> {
            FerroProperty::register::<ContentDialog, _>("FullSizeDesired", false)
        }

        /// Defines the `PrimaryButtonCommand` property.
        pub fn primary_button_command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            FerroProperty::register::<ContentDialog, _>("PrimaryButtonCommand", None)
        }

        /// Defines the `PrimaryButtonCommandParameter` property.
        pub fn primary_button_command_parameter_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ContentDialog, _>("PrimaryButtonCommandParameter", None)
        }

        /// Defines the `SecondaryButtonCommand` property.
        pub fn secondary_button_command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            FerroProperty::register::<ContentDialog, _>("SecondaryButtonCommand", None)
        }

        /// Defines the `SecondaryButtonCommandParameter` property.
        pub fn secondary_button_command_parameter_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ContentDialog, _>("SecondaryButtonCommandParameter", None)
        }

        /// Defines the `CloseButtonCommand` property.
        pub fn close_button_command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
            FerroProperty::register::<ContentDialog, _>("CloseButtonCommand", None)
        }

        /// Defines the `CloseButtonCommandParameter` property.
        pub fn close_button_command_parameter_property() -> StyledProperty<Option<BoxedValue>> {
            FerroProperty::register::<ContentDialog, _>("CloseButtonCommandParameter", None)
        }
    }
}

impl FerroObjectImpl for ContentDialog {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let property = change.property();
        if property == Self::primary_button_text_property().as_property()
            || property == Self::secondary_button_text_property().as_property()
            || property == Self::close_button_text_property().as_property()
            || property == Self::default_button_property().as_property()
            || property == Self::full_size_desired_property().as_property()
            || property == Self::title_property().as_property()
            || property == Self::title_template_property().as_property()
        {
            this.update_visual_state();
        }
    }
}

impl VisualImpl for ContentDialog {
    /// A dialog taken out of its window (the window closed) ends with no result, so that nothing
    /// waits for it forever.
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        if this.operation.borrow().is_some() {
            this.finish_closing(ContentDialogResult::None);
        }
    }
}

impl LayoutableImpl for ContentDialog {
    /// The dialog covers the overlay layer that measures it.
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let desired = Self::parent_measure_override(this, available_size);
        if this.is_open() && available_size.width.is_finite() && available_size.height.is_finite() {
            available_size
        } else {
            desired
        }
    }
}

impl InputElementImpl for ContentDialog {
    /// `ProcessLayoutRootKey`: Escape closes the dialog and Enter invokes its default button.
    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        Self::parent_on_key_down(this, e);
        if e.handled() || !this.is_open() {
            return;
        }
        match e.key {
            Key::Escape => {
                this.execute_close_action();
                e.set_handled(true);
            }
            Key::Enter => {
                let button = this.default_button();
                if button != ContentDialogButton::None && this.is_button_available(button) {
                    this.on_command_button(button);
                    e.set_handled(true);
                }
            }
            _ => {}
        }
    }
}

impl TemplatedControlImpl for ContentDialog {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        if let Some(old) = this.parts.borrow_mut().take() {
            for (button, token) in old.tokens {
                button.remove_handler(Button::click_event(), token);
            }
        }

        let scope = e.name_scope();
        let Some(background) = scope.find_as::<Control>("BackgroundElement") else {
            return;
        };
        let primary = scope.find_as::<Button>("PrimaryButton");
        let secondary = scope.find_as::<Button>("SecondaryButton");
        let close = scope.find_as::<Button>("CloseButton");
        let mut tokens = Vec::new();
        for (button, kind) in [
            (&primary, ContentDialogButton::Primary),
            (&secondary, ContentDialogButton::Secondary),
            (&close, ContentDialogButton::Close),
        ] {
            if let Some(button) = button {
                let weak = this.to_ref().downgrade();
                let token = button.click(move |_, e| {
                    if let Some(this) = weak.upgrade() {
                        this.on_command_button(kind);
                    }
                    e.set_handled(true);
                });
                tokens.push((button.clone(), token));
            }
        }
        *this.parts.borrow_mut() = Some(Parts {
            background,
            title: scope.find_as::<Control>("Title"),
            content: scope.find_as::<Control>("Content"),
            command_space: scope.find_as::<Control>("CommandSpace"),
            primary,
            secondary,
            close,
            tokens,
        });
        this.update_visual_state();
    }
}

impl ContentDialog {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            parts: RefCell::new(None),
            operation: RefCell::new(None),
            hide_in_progress: Cell::new(false),
            click_in_progress: Cell::new(false),
            previous_focus: RefCell::new(None),
            opened: Rc::new(HandlerList::new()),
            closing: Rc::new(HandlerList::new()),
            closed: Rc::new(HandlerList::new()),
            primary_click: Rc::new(HandlerList::new()),
            secondary_click: Rc::new(HandlerList::new()),
            close_click: Rc::new(HandlerList::new()),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The title of the dialog.
    pub fn title(&self) -> Option<BoxedValue> {
        self.get_value(Self::title_property())
    }

    pub fn set_title(&self, value: Option<BoxedValue>) {
        self.set_value(Self::title_property(), value);
    }

    /// The template of the title.
    pub fn title_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.get_value(Self::title_template_property())
    }

    pub fn set_title_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_value(Self::title_template_property(), value);
    }

    /// The text of the primary button; the button is hidden without one.
    pub fn primary_button_text(&self) -> Option<String> {
        self.get_value(Self::primary_button_text_property())
    }

    pub fn set_primary_button_text(&self, value: Option<&str>) {
        self.set_value(Self::primary_button_text_property(), value.map(str::to_owned));
    }

    /// The text of the secondary button; the button is hidden without one.
    pub fn secondary_button_text(&self) -> Option<String> {
        self.get_value(Self::secondary_button_text_property())
    }

    pub fn set_secondary_button_text(&self, value: Option<&str>) {
        self.set_value(Self::secondary_button_text_property(), value.map(str::to_owned));
    }

    /// The text of the close button; the button is hidden without one.
    pub fn close_button_text(&self) -> Option<String> {
        self.get_value(Self::close_button_text_property())
    }

    pub fn set_close_button_text(&self, value: Option<&str>) {
        self.set_value(Self::close_button_text_property(), value.map(str::to_owned));
    }

    /// Whether the primary button is enabled. The default is `true`.
    pub fn is_primary_button_enabled(&self) -> bool {
        self.get_value(Self::is_primary_button_enabled_property())
    }

    pub fn set_is_primary_button_enabled(&self, value: bool) {
        self.set_value(Self::is_primary_button_enabled_property(), value);
    }

    /// Whether the secondary button is enabled. The default is `true`.
    pub fn is_secondary_button_enabled(&self) -> bool {
        self.get_value(Self::is_secondary_button_enabled_property())
    }

    pub fn set_is_secondary_button_enabled(&self, value: bool) {
        self.set_value(Self::is_secondary_button_enabled_property(), value);
    }

    /// The button Enter invokes.
    pub fn default_button(&self) -> ContentDialogButton {
        self.get_value(Self::default_button_property())
    }

    pub fn set_default_button(&self, value: ContentDialogButton) {
        self.set_value(Self::default_button_property(), value);
    }

    /// Whether the dialog takes the full height of its window.
    pub fn full_size_desired(&self) -> bool {
        self.get_value(Self::full_size_desired_property())
    }

    pub fn set_full_size_desired(&self, value: bool) {
        self.set_value(Self::full_size_desired_property(), value);
    }

    /// The command the primary button executes.
    pub fn primary_button_command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::primary_button_command_property())
    }

    pub fn set_primary_button_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::primary_button_command_property(), value);
    }

    /// The parameter of the command of the primary button.
    pub fn primary_button_command_parameter(&self) -> Option<BoxedValue> {
        self.get_value(Self::primary_button_command_parameter_property())
    }

    pub fn set_primary_button_command_parameter(&self, value: Option<BoxedValue>) {
        self.set_value(Self::primary_button_command_parameter_property(), value);
    }

    /// The command the secondary button executes.
    pub fn secondary_button_command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::secondary_button_command_property())
    }

    pub fn set_secondary_button_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::secondary_button_command_property(), value);
    }

    /// The parameter of the command of the secondary button.
    pub fn secondary_button_command_parameter(&self) -> Option<BoxedValue> {
        self.get_value(Self::secondary_button_command_parameter_property())
    }

    pub fn set_secondary_button_command_parameter(&self, value: Option<BoxedValue>) {
        self.set_value(Self::secondary_button_command_parameter_property(), value);
    }

    /// The command the close button executes.
    pub fn close_button_command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::close_button_command_property())
    }

    pub fn set_close_button_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::close_button_command_property(), value);
    }

    /// The parameter of the command of the close button.
    pub fn close_button_command_parameter(&self) -> Option<BoxedValue> {
        self.get_value(Self::close_button_command_parameter_property())
    }

    pub fn set_close_button_command_parameter(&self, value: Option<BoxedValue>) {
        self.set_value(Self::close_button_command_parameter_property(), value);
    }

    /// Occurs when the dialog has opened.
    pub fn opened(&self, handler: impl Fn() + 'static) -> Subscription {
        let handlers = self.opened.clone();
        let token = handlers.add(Rc::new(handler));
        Subscription::new(Disposable::create(move || {
            handlers.remove(token);
        }))
    }

    /// Occurs before the dialog closes; a handler can cancel or defer it.
    pub fn closing(&self, handler: impl Fn(&ContentDialogClosingEventArgs) + 'static) -> Subscription {
        Self::subscribe(&self.closing, handler)
    }

    /// Occurs when the dialog has closed.
    pub fn closed(&self, handler: impl Fn(&ContentDialogClosedEventArgs) + 'static) -> Subscription {
        Self::subscribe(&self.closed, handler)
    }

    /// Occurs when the primary button is invoked.
    pub fn primary_button_click(
        &self,
        handler: impl Fn(&ContentDialogButtonClickEventArgs) + 'static,
    ) -> Subscription {
        Self::subscribe(&self.primary_click, handler)
    }

    /// Occurs when the secondary button is invoked.
    pub fn secondary_button_click(
        &self,
        handler: impl Fn(&ContentDialogButtonClickEventArgs) + 'static,
    ) -> Subscription {
        Self::subscribe(&self.secondary_click, handler)
    }

    /// Occurs when the close button is invoked.
    pub fn close_button_click(
        &self,
        handler: impl Fn(&ContentDialogButtonClickEventArgs) + 'static,
    ) -> Subscription {
        Self::subscribe(&self.close_click, handler)
    }

    fn subscribe<T: 'static>(handlers: &Handlers<T>, handler: impl Fn(&T) + 'static) -> Subscription {
        let handlers = handlers.clone();
        let token = handlers.add(Rc::new(handler));
        Subscription::new(Disposable::create(move || {
            handlers.remove(token);
        }))
    }

    fn raise<T>(handlers: &Handlers<T>, args: &T) {
        for (_, handler) in handlers.snapshot().iter() {
            handler(args);
        }
    }

    /// Whether the dialog is open.
    pub fn is_open(&self) -> bool {
        self.operation.borrow().is_some()
    }

    /// Shows the dialog over the window of `host` (WinUI's `ShowAsync`, with `host` as its
    /// `XamlRoot`). The returned future gives the result when the dialog closes.
    pub fn show_async(&self, host: &Visual) -> Result<ContentDialogOperation, ContentDialogError> {
        if self.is_open() {
            return Err(ContentDialogError::AlreadyOpen);
        }
        let top_level = TopLevel::get_top_level(Some(host)).ok_or(ContentDialogError::NoHost)?;
        let overlay = OverlayLayer::get_overlay_layer(host).ok_or(ContentDialogError::NoHost)?;
        let another_open = overlay
            .children()
            .snapshot()
            .iter()
            .any(|child| child.clone().cast::<ContentDialog>().is_some_and(|dialog| dialog.is_open()));
        if another_open {
            return Err(ContentDialogError::AnotherDialogOpen);
        }

        *self.previous_focus.borrow_mut() =
            top_level.focus_manager().get_focused_element().map(|element| element.downgrade());
        let state = Rc::new(OperationState::default());
        *self.operation.borrow_mut() = Some(state.clone());
        self.hide_in_progress.set(false);
        self.click_in_progress.set(false);
        self.pseudo_classes().set(":showing", true);
        overlay.children().add(self.to_ref().upcast::<Control>());
        self.update_visual_state();
        top_level.update_layout();
        self.set_initial_focus();
        self.play_transition(true, || {});
        for (_, handler) in self.opened.snapshot().iter() {
            handler();
        }
        Ok(ContentDialogOperation { state })
    }

    /// Closes the dialog with [`ContentDialogResult::None`]; `Closing` can still keep it open.
    pub fn hide(&self) {
        self.hide_internal(ContentDialogResult::None);
    }

    /// `HideInternal`: raises `Closing` and closes unless a handler cancels.
    fn hide_internal(&self, result: ContentDialogResult) {
        if !self.is_open() || self.hide_in_progress.replace(true) {
            return;
        }
        let deferral = Rc::new(DeferralState::default());
        let args = ContentDialogClosingEventArgs { result, cancel: Rc::default(), deferral: deferral.clone() };
        Self::raise(&self.closing, &args);
        let weak = self.to_ref().downgrade();
        deferral.run_when_complete(move || {
            if let Some(this) = weak.upgrade() {
                if args.cancel() {
                    this.hide_in_progress.set(false);
                } else {
                    this.play_hide(result);
                }
            }
        });
    }

    fn play_hide(&self, result: ContentDialogResult) {
        let weak = self.to_ref().downgrade();
        self.play_transition(false, move || {
            if let Some(this) = weak.upgrade() {
                this.finish_closing(result);
            }
        });
    }

    /// `OnFinishedClosing`: out of the overlay, the focus back, the result and `Closed`.
    fn finish_closing(&self, result: ContentDialogResult) {
        let Some(state) = self.operation.borrow_mut().take() else {
            return;
        };
        self.pseudo_classes().set(":showing", false);
        self.hide_in_progress.set(false);
        if let Some(overlay) = self.get_visual_parent().and_then(|parent| parent.cast::<OverlayLayer>()) {
            overlay.children().remove(&self.to_ref().upcast::<Control>());
        }
        let previous = self.previous_focus.borrow_mut().take();
        if let Some(previous) = previous.and_then(|previous| previous.upgrade()) {
            previous.focus();
        }
        state.complete(result);
        Self::raise(&self.closed, &ContentDialogClosedEventArgs { result });
    }

    /// `OnCommandButtonClicked`: the click event, then the command and the closing.
    fn on_command_button(&self, button: ContentDialogButton) {
        if !self.is_open() || self.hide_in_progress.get() || self.click_in_progress.replace(true) {
            return;
        }
        let (handlers, command, parameter, result) = match button {
            ContentDialogButton::Primary => (
                &self.primary_click,
                self.primary_button_command(),
                self.primary_button_command_parameter(),
                ContentDialogResult::Primary,
            ),
            ContentDialogButton::Secondary => (
                &self.secondary_click,
                self.secondary_button_command(),
                self.secondary_button_command_parameter(),
                ContentDialogResult::Secondary,
            ),
            _ => (
                &self.close_click,
                self.close_button_command(),
                self.close_button_command_parameter(),
                ContentDialogResult::None,
            ),
        };
        let deferral = Rc::new(DeferralState::default());
        let args = ContentDialogButtonClickEventArgs { cancel: Rc::default(), deferral: deferral.clone() };
        Self::raise(handlers, &args);
        let weak = self.to_ref().downgrade();
        deferral.run_when_complete(move || {
            let Some(this) = weak.upgrade() else {
                return;
            };
            this.click_in_progress.set(false);
            if args.cancel() {
                return;
            }
            if let Some(command) = command {
                if command.can_execute(parameter.as_ref()) {
                    command.execute(parameter.as_ref());
                }
            }
            this.hide_internal(result);
        });
    }

    /// `ExecuteCloseAction`: the close button when it can be invoked, otherwise a plain hide.
    fn execute_close_action(&self) {
        if self.is_button_available(ContentDialogButton::Close) {
            self.on_command_button(ContentDialogButton::Close);
        } else {
            self.hide_internal(ContentDialogResult::None);
        }
    }

    /// Whether `button` has a text and is enabled.
    fn is_button_available(&self, button: ContentDialogButton) -> bool {
        let has_text = |text: Option<String>| text.is_some_and(|text| !text.is_empty());
        match button {
            ContentDialogButton::None => false,
            ContentDialogButton::Primary => has_text(self.primary_button_text()) && self.is_primary_button_enabled(),
            ContentDialogButton::Secondary => {
                has_text(self.secondary_button_text()) && self.is_secondary_button_enabled()
            }
            ContentDialogButton::Close => has_text(self.close_button_text()),
        }
    }

    fn update_visual_state(&self) {
        let has_text = |text: Option<String>| text.is_some_and(|text| !text.is_empty());
        let primary = has_text(self.primary_button_text());
        let secondary = has_text(self.secondary_button_text());
        let close = has_text(self.close_button_text());
        let current = button_state(primary, secondary, close);
        let classes = self.pseudo_classes();
        for state in BUTTON_STATES {
            classes.set(state, state == current);
        }
        let default = self.default_button();
        classes.set(":primary-default", default == ContentDialogButton::Primary && primary);
        classes.set(":secondary-default", default == ContentDialogButton::Secondary && secondary);
        classes.set(":close-default", default == ContentDialogButton::Close && close);
        classes.set(":full-size", self.full_size_desired());

        // `UpdateTitleSpaceVisibility`.
        if let Some(title) = self.parts.borrow().as_ref().and_then(|parts| parts.title.clone()) {
            title.set_is_visible(self.title().is_some() || self.title_template().is_some());
        }
    }

    /// `SetInitialFocusElement`: the first focusable element of the content, else the default
    /// button, else the first focusable command button.
    fn set_initial_focus(&self) {
        let parts = self.parts.borrow();
        let Some(parts) = parts.as_ref() else {
            return;
        };
        let first_in = |part: &Option<Ref<Control>>| {
            part.as_ref()
                .and_then(|part| FocusManager::find_first_focusable_element_in(&part.clone().upcast()))
        };
        if first_in(&parts.content).is_some_and(|element| element.focus()) {
            return;
        }
        let default = match self.default_button() {
            ContentDialogButton::Primary => parts.primary.as_ref(),
            ContentDialogButton::Secondary => parts.secondary.as_ref(),
            ContentDialogButton::Close => parts.close.as_ref(),
            ContentDialogButton::None => None,
        };
        if default.is_some_and(|button| button.focus()) {
            return;
        }
        if let Some(element) = first_in(&parts.command_space) {
            element.focus();
        }
    }

    /// The opening or closing transition of the template (`DialogShowingStates`), on the opacity
    /// and the render transform, which the compositor draws. Without a clock (no application is
    /// running) the dialog changes at once.
    fn play_transition(&self, showing: bool, done: impl FnOnce() + 'static) {
        let background = self.parts.borrow().as_ref().map(|parts| parts.background.clone());
        let Some(background) = background else {
            done();
            return;
        };
        // The value the dialog rests at once the animation ends, so that it never flashes back.
        background.set_opacity(if showing { 1.0 } else { 0.0 });
        let has_clock = background.clock().is_some()
            || FerroLocator::current().get_service::<dyn IGlobalClock>().is_some();
        if !has_clock {
            done();
            return;
        }
        let target: Ref<Visual> = background.upcast();
        let (from_opacity, to_opacity) = if showing { (0.0, 1.0) } else { (1.0, 0.0) };
        let (from_scale, to_scale, scale_ms) =
            if showing { (OUTER_SCALE, 1.0, SHOW_SCALE_MS) } else { (1.0, OUTER_SCALE, HIDE_SCALE_MS) };

        let fade = Animation::new();
        fade.set_fill_mode(FillMode::None);
        fade.set_duration(SignedDuration::from_milliseconds(FADE_MS));
        for (cue, value) in [(0.0, from_opacity), (1.0, to_opacity)] {
            let setters: Vec<Rc<dyn IAnimationSetter>> = vec![Setter::new(Visual::opacity_property(), value)];
            fade.children().add(KeyFrame::with_cue(Cue::new(cue), setters));
        }
        drop(fade.run_async_with_clock(&target, None, CancellationToken::new()));

        let scale = Animation::new();
        scale.set_fill_mode(FillMode::None);
        scale.set_duration(SignedDuration::from_milliseconds(scale_ms));
        // `ControlFastOutSlowInKeySpline`.
        scale.set_easing(Easing::new(SplineEasing::with_points(0.0, 0.0, 0.0, 1.0)));
        for (cue, value) in [(0.0, from_scale), (1.0, to_scale)] {
            let setters: Vec<Rc<dyn IAnimationSetter>> = vec![
                Setter::new(ScaleTransform::scale_x_property(), value),
                Setter::new(ScaleTransform::scale_y_property(), value),
            ];
            scale.children().add(KeyFrame::with_cue(Cue::new(cue), setters));
        }
        let task = scale.run_async_with_clock(&target, None, CancellationToken::new());
        drop(Dispatcher::ui_thread().spawn_local(async move {
            // A transition that cannot run must not keep the dialog from closing.
            drop(task.await);
            done();
        }));
    }
}
