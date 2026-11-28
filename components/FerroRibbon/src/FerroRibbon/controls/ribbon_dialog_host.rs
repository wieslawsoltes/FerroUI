//! Shared native modal template host used by color and ribbon customization dialogs.
//! Source part names/visibility/focus follow the pinned Uno snapshot (Microsoft MIT).
//! This native adaptation is additional API, not a source RibbonSpace control.
#[cfg(test)]
use super::RibbonColorDialogHost;
use super::ribbon_dialog_buttons::DialogButtonEvents;
use super::ribbon_dialog_lifecycle::DialogLifecycle;
use super::ribbon_dialog_session::RibbonDialogSession;
use super::{
    RibbonDialogButtonClickEventArgs, RibbonDialogClosedEventArgs, RibbonDialogClosingEventArgs,
    RibbonDialogOpenedEventArgs,
};
use crate::compat::ContentDialogButton;
use ferroui_base::input::ICommand;
use ferroui_base::{
    BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty, Rect, Ref, StyledElementImpl,
    StyledProperty, Thickness, VisualImpl, WeakRef,
    data::BindingPriority,
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    input::{
        FocusManager, InputElement, InputElementImpl, KeyModifiers, KeyboardNavigation,
        KeyboardNavigationMode, NavigationMethod,
    },
    instantiate,
    interactivity::{InteractiveImpl, RoutedEventHandlerToken},
    layout::{Layoutable, LayoutableImpl, VerticalAlignment},
    reactive::Subscription,
};
use ferroui_controls::{
    Application, Button, ColumnDefinition, ContentControl, ContentControlImpl,
    ContentControlImplExt, Control, ControlImpl, Grid, GridLength, ScrollViewer, TextBlock,
    TopLevel,
    automation::{
        AutomationError, AutomationProperties,
        peers::{
            AutomationControlType, AutomationPeer, AutomationPeerImpl, AutomationPeerImplExt,
            ControlAutomationPeer, ControlAutomationPeerImpl,
        },
        provider::{IWindowProvider, WindowInteractionState},
    },
    presenters::ContentPresenter,
    primitives::{
        ScrollBarVisibility, TemplateAppliedEventArgs, TemplatedControlImpl,
        TemplatedControlImplExt,
    },
    templates::IDataTemplate,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

ferroui_base::ferro_thread_local! {static DEFAULTS:RefCell<Vec<(WeakRef<Application>,WeakRef<ferroui_base::controls::ResourceDictionary>)>>=const{RefCell::new(Vec::new())};}
fn ensure_defaults() {
    let Some(app) = Application::current() else {
        return;
    };
    let found = DEFAULTS.with_borrow_mut(|cache| {
        cache.retain(|(a, d)| a.upgrade().is_some() && d.upgrade().is_some());
        cache
            .iter()
            .any(|(a, _)| a.upgrade().as_ref() == Some(&app))
    });
    if found {
        return;
    }
    let value =
        crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/ColorDialog.xaml")
            .expect("color dialog defaults")
            .expect("color dialog dictionary");
    let dictionary = ferroui_base::data::core::ValueTypes::as_object(&*value)
        .expect("resource dictionary object")
        .cast::<ferroui_base::controls::ResourceDictionary>()
        .expect("color dialog dictionary type");
    // Platform defaults have the lowest merged-dictionary priority. Application
    // local and later merged resources can override every source key and theme.
    app.resources()
        .merged_dictionaries()
        .insert(0, dictionary.clone().into());
    DEFAULTS.with_borrow_mut(|cache| cache.push((app.downgrade(), dictionary.downgrade())));
}

#[derive(Default)]
struct Parts {
    title: Option<Ref<ContentControl>>,
    content: Option<Ref<ContentPresenter>>,
    commands: Option<Ref<Grid>>,
    first_spacer: Option<Ref<ColumnDefinition>>,
    secondary_column: Option<Ref<ColumnDefinition>>,
    primary: Option<Ref<Button>>,
    secondary: Option<Ref<Button>>,
    close: Option<Ref<Button>>,
    input_pane: Option<InputPaneParts>,
    clicks: Vec<(Ref<Button>, RoutedEventHandlerToken)>,
    subscriptions: Vec<Rc<dyn ferroui_base::reactive::IDisposable>>,
}
impl Parts {
    fn detach(&mut self) {
        for subscription in self.subscriptions.drain(..) {
            subscription.dispose();
        }
        for (button, token) in self.clicks.drain(..) {
            button.remove_handler(Button::click_event(), token);
        }
        if let Some(content) = self.content.take() {
            content.set_content(None);
            content.set_content_template(None);
        }
    }
}
#[derive(Clone)]
struct InputPaneParts {
    layout_root: Ref<Layoutable>,
    background: Ref<Layoutable>,
    content_scroll: Ref<ScrollViewer>,
}

pub(super) struct InputPaneAdjustment {
    pub(super) scroll: Ref<ScrollViewer>,
    _values: Vec<Subscription>,
}

#[repr(C)]
pub struct RibbonDialogHost {
    base: ContentControl,
    parts: RefCell<Parts>,
    action: RefCell<Option<DialogAction>>,
    pub(super) button_events: DialogButtonEvents,
    pub(super) lifecycle: DialogLifecycle,
    pub(super) after_closed: RefCell<Vec<Box<dyn FnOnce()>>>,
    default_visual: RefCell<Option<DefaultVisual>>,
    default_styles: RefCell<[Option<DialogTheme>; 3]>,
    template_generation: Cell<u64>,
    updating_default: Cell<bool>,
    pending_default: Cell<bool>,
}
type DialogAction = Rc<dyn Fn(ContentDialogButton)>;
type DialogTheme = Option<Ref<ferroui_base::styling::ControlTheme>>;
struct DefaultVisual {
    button: Ref<Button>,
    theme: DialogTheme,
    value: Rc<dyn ferroui_base::reactive::IDisposable>,
}
ferro_class!(RibbonDialogHost:ContentControl);
ferro_impl_classes!(RibbonDialogHost:VisualImpl,LayoutableImpl,InteractiveImpl);
ferro_class_info!(RibbonDialogHost {new:RibbonDialogHost::new,markup:{namespace:"FerroRibbon.Controls",content:Content,events:[Closing(Option<BoxedValue>,Rc<RibbonDialogClosingEventArgs>){add:RibbonDialogHost::add_closing,remove:RibbonDialogHost::remove_closing},Closed(Option<BoxedValue>,Rc<RibbonDialogClosedEventArgs>){add:RibbonDialogHost::add_closed,remove:RibbonDialogHost::remove_closed},Opened(Option<BoxedValue>,Rc<RibbonDialogOpenedEventArgs>){add:RibbonDialogHost::add_opened,remove:RibbonDialogHost::remove_opened},PrimaryButtonClick(Option<BoxedValue>,Rc<RibbonDialogButtonClickEventArgs>){add:RibbonDialogHost::add_primary_button_click,remove:RibbonDialogHost::remove_primary_button_click},SecondaryButtonClick(Option<BoxedValue>,Rc<RibbonDialogButtonClickEventArgs>){add:RibbonDialogHost::add_secondary_button_click,remove:RibbonDialogHost::remove_secondary_button_click},CloseButtonClick(Option<BoxedValue>,Rc<RibbonDialogButtonClickEventArgs>){add:RibbonDialogHost::add_close_button_click,remove:RibbonDialogHost::remove_close_button_click}],static_properties:[HighContrastVariant:ferroui_base::styling::ThemeVariant{get:crate::theming::RibbonThemeResources::high_contrast_variant}]}});
ferro_properties! {impl RibbonDialogHost {
 pub fn primary_button_command_property()->StyledProperty<Option<Rc<dyn ICommand>>> {FerroProperty::register::<Self, _>("PrimaryButtonCommand",None)}
 pub fn primary_button_command_parameter_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<Self, _>("PrimaryButtonCommandParameter",None)}
 pub fn secondary_button_command_property()->StyledProperty<Option<Rc<dyn ICommand>>> {FerroProperty::register::<Self, _>("SecondaryButtonCommand",None)}
 pub fn secondary_button_command_parameter_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<Self, _>("SecondaryButtonCommandParameter",None)}
 pub fn close_button_command_property()->StyledProperty<Option<Rc<dyn ICommand>>> {FerroProperty::register::<Self, _>("CloseButtonCommand",None)}
 pub fn close_button_command_parameter_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<Self, _>("CloseButtonCommandParameter",None)}
 pub fn title_property()->StyledProperty<Option<BoxedValue>> {FerroProperty::register::<Self,_>("Title",None)}
 pub fn title_template_property()->StyledProperty<Option<Rc<dyn IDataTemplate>>> {FerroProperty::register::<Self,_>("TitleTemplate",None)}
 pub fn primary_button_text_property()->StyledProperty<Option<String>> {FerroProperty::register::<Self,_>("PrimaryButtonText",None)}
 pub fn secondary_button_text_property()->StyledProperty<Option<String>> {FerroProperty::register::<Self,_>("SecondaryButtonText",None)}
 pub fn close_button_text_property()->StyledProperty<Option<String>> {FerroProperty::register::<Self,_>("CloseButtonText",None)}
 pub fn is_primary_button_enabled_property()->StyledProperty<bool> {FerroProperty::register::<Self,_>("IsPrimaryButtonEnabled",true)}
 pub fn is_secondary_button_enabled_property()->StyledProperty<bool> {FerroProperty::register::<Self,_>("IsSecondaryButtonEnabled",true)}
 pub fn primary_button_style_property()->StyledProperty<Option<Ref<ferroui_base::styling::ControlTheme>>> {FerroProperty::register::<Self,_>("PrimaryButtonStyle",None)}
 pub fn secondary_button_style_property()->StyledProperty<Option<Ref<ferroui_base::styling::ControlTheme>>> {FerroProperty::register::<Self,_>("SecondaryButtonStyle",None)}
 pub fn close_button_style_property()->StyledProperty<Option<Ref<ferroui_base::styling::ControlTheme>>> {FerroProperty::register::<Self,_>("CloseButtonStyle",None)}
 pub fn default_button_property()->StyledProperty<ContentDialogButton> {FerroProperty::register::<Self,_>("DefaultButton",ContentDialogButton::None)}
}}
macro_rules! property {
    ($get:ident,$set:ident,$p:ident,$t:ty) => {
        pub fn $get(&self) -> $t {
            self.get_value(Self::$p())
        }
        pub fn $set(&self, value: $t) {
            self.set_value(Self::$p(), value)
        }
    };
}
impl RibbonDialogHost {
    pub fn construct() -> Self {
        Self {
            base: ContentControl::construct(),
            parts: RefCell::default(),
            action: RefCell::default(),
            button_events: DialogButtonEvents::default(),
            lifecycle: DialogLifecycle::default(),
            after_closed: RefCell::default(),
            default_visual: RefCell::default(),
            default_styles: RefCell::default(),
            template_generation: Cell::new(0),
            updating_default: Cell::new(false),
            pending_default: Cell::new(false),
        }
    }
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    property!(
        primary_button_command,
        set_primary_button_command,
        primary_button_command_property,
        Option<Rc<dyn ICommand>>
    );
    property!(
        primary_button_command_parameter,
        set_primary_button_command_parameter,
        primary_button_command_parameter_property,
        Option<BoxedValue>
    );
    property!(
        secondary_button_command,
        set_secondary_button_command,
        secondary_button_command_property,
        Option<Rc<dyn ICommand>>
    );
    property!(
        secondary_button_command_parameter,
        set_secondary_button_command_parameter,
        secondary_button_command_parameter_property,
        Option<BoxedValue>
    );
    property!(
        close_button_command,
        set_close_button_command,
        close_button_command_property,
        Option<Rc<dyn ICommand>>
    );
    property!(
        close_button_command_parameter,
        set_close_button_command_parameter,
        close_button_command_parameter_property,
        Option<BoxedValue>
    );
    property!(title, set_title, title_property, Option<BoxedValue>);
    property!(
        default_button,
        set_default_button,
        default_button_property,
        ContentDialogButton
    );
    property!(
        title_template,
        set_title_template,
        title_template_property,
        Option<Rc<dyn IDataTemplate>>
    );
    property!(
        primary_button_text,
        set_primary_button_text,
        primary_button_text_property,
        Option<String>
    );
    property!(
        secondary_button_text,
        set_secondary_button_text,
        secondary_button_text_property,
        Option<String>
    );
    property!(
        close_button_text,
        set_close_button_text,
        close_button_text_property,
        Option<String>
    );
    property!(
        is_primary_button_enabled,
        set_is_primary_button_enabled,
        is_primary_button_enabled_property,
        bool
    );
    property!(
        is_secondary_button_enabled,
        set_is_secondary_button_enabled,
        is_secondary_button_enabled_property,
        bool
    );
    property!(
        primary_button_style,
        set_primary_button_style,
        primary_button_style_property,
        Option<Ref<ferroui_base::styling::ControlTheme>>
    );
    property!(
        secondary_button_style,
        set_secondary_button_style,
        secondary_button_style_property,
        Option<Ref<ferroui_base::styling::ControlTheme>>
    );
    property!(
        close_button_style,
        set_close_button_style,
        close_button_style_property,
        Option<Ref<ferroui_base::styling::ControlTheme>>
    );
    pub(super) fn complete_button(&self, button: ContentDialogButton) {
        let generation = self.dialog_generation();
        let (command, parameter) = match button {
            ContentDialogButton::Primary => (
                self.primary_button_command(),
                self.primary_button_command_parameter(),
            ),
            ContentDialogButton::Secondary => (
                self.secondary_button_command(),
                self.secondary_button_command_parameter(),
            ),
            ContentDialogButton::Close => (
                self.close_button_command(),
                self.close_button_command_parameter(),
            ),
            _ => return,
        };
        if let Some(command) = command
            && command.can_execute(parameter.as_ref())
            && self.dialog_generation() == generation
        {
            command.execute(parameter.as_ref());
        }
        if self.dialog_generation() == generation {
            self.request_close(button);
        }
    }
    pub(super) fn invoke_close_action(&self, button: ContentDialogButton) {
        let action = self.action.borrow().clone();
        if let Some(action) = action {
            action(button);
        }
    }
    pub(crate) fn on_button_action(&self, action: Option<DialogAction>) {
        self.action.replace(action);
    }
    // Existing host tests/consumers observe only acceptance; one underlying path
    // now retains the distinct secondary/close signal for real dialog callers.
    #[cfg(test)]
    pub(crate) fn on_action(&self, action: Option<Rc<dyn Fn(bool)>>) {
        self.on_button_action(action.map(|action| {
            Rc::new(move |button| action(button == ContentDialogButton::Primary)) as DialogAction
        }));
    }
    pub fn primary_button(&self) -> Option<Ref<Button>> {
        self.parts.borrow().primary.clone()
    }
    pub fn close_button(&self) -> Option<Ref<Button>> {
        self.parts.borrow().close.clone()
    }
    pub fn secondary_button(&self) -> Option<Ref<Button>> {
        self.parts.borrow().secondary.clone()
    }
    pub fn command_space(&self) -> Option<Ref<Grid>> {
        self.parts.borrow().commands.clone()
    }
    pub fn content_presenter(&self) -> Option<Ref<ContentPresenter>> {
        self.parts.borrow().content.clone()
    }
    pub fn focus_dialog(&self) -> bool {
        // Native focus can synchronously run user handlers which replace the
        // template. Read each source phase when reached, without keeping a
        // parts borrow across framework traversal or callbacks.
        let content = self.parts.borrow().content.clone();
        if content
            .and_then(|c| {
                FocusManager::find_first_focusable_element_in(&c.clone().upcast::<InputElement>())
            })
            .is_some_and(|e| e.focus_with(NavigationMethod::Unspecified, KeyModifiers::NONE))
        {
            return true;
        }
        let default = {
            let p = self.parts.borrow();
            match self.default_button() {
                ContentDialogButton::Primary => p.primary.clone(),
                ContentDialogButton::Secondary => p.secondary.clone(),
                ContentDialogButton::Close => p.close.clone(),
                _ => None,
            }
        };
        if default
            .as_ref()
            .is_some_and(|b| b.focus_with(NavigationMethod::Unspecified, KeyModifiers::NONE))
        {
            return true;
        }
        let commands = self.parts.borrow().commands.clone();
        commands
            .and_then(|c| {
                FocusManager::find_first_focusable_element_in(&c.clone().upcast::<InputElement>())
            })
            .is_some_and(|e| e.focus_with(NavigationMethod::Unspecified, KeyModifiers::NONE))
    }
    fn update_states(&self) {
        let primary = self.primary_button_text().is_some_and(|s| !s.is_empty());
        let secondary = self.secondary_button_text().is_some_and(|s| !s.is_empty());
        let close = self.close_button_text().is_some_and(|s| !s.is_empty());
        let any = primary || secondary || close;
        // State setters can synchronously invoke user property observers. Keep
        // only strong target handles here, never a borrow of the mutable parts.
        let (
            title,
            commands,
            primary_button,
            secondary_button,
            close_button,
            first_spacer,
            secondary_column,
        ) = {
            let p = self.parts.borrow();
            (
                p.title.clone(),
                p.commands.clone(),
                p.primary.clone(),
                p.secondary.clone(),
                p.close.clone(),
                p.first_spacer.clone(),
                p.secondary_column.clone(),
            )
        };
        if let Some(title) = &title {
            title.set_is_visible(self.title().is_some() || self.title_template().is_some());
        }
        if let Some(commands) = &commands {
            commands.set_is_visible(any);
        }
        for (button, visible) in [
            (&primary_button, primary),
            (&secondary_button, secondary),
            (&close_button, close),
        ] {
            if let Some(button) = button {
                button.set_is_visible(!any || visible);
            }
        }
        // Demo-selected Fluent v2 states, including resetting previous state
        // values on every transition. The legacy template used four columns.
        let all = primary && secondary && close;
        if let Some(button) = &primary_button {
            Grid::set_column(
                button,
                if primary && !secondary && !close {
                    4
                } else {
                    0
                },
            );
        }
        if let Some(button) = &secondary_button {
            Grid::set_column(
                button,
                if all {
                    2
                } else if secondary && !close {
                    4
                } else {
                    0
                },
            );
        }
        if let Some(first) = &first_spacer {
            if !all {
                first.set_width(GridLength::new(0., ferroui_controls::GridUnitType::Pixel));
            } else {
                // The original state setter refers to this resource directly;
                // SecondSpacer can have an independent downstream local value.
                let anchor: &ferroui_base::StyledElement = match &commands {
                    Some(commands) => commands,
                    None => self,
                };
                if let Some(Some(value)) = anchor.try_find_resource(
                    &"ContentDialogButtonSpacing".into(),
                    anchor.actual_theme_variant().as_ref(),
                ) && let Some(spacing) = value.downcast_ref::<GridLength>()
                {
                    first.set_width(*spacing);
                }
            }
        }
        if let Some(column) = &secondary_column {
            column.set_width(if all {
                GridLength::STAR
            } else {
                GridLength::new(0., ferroui_controls::GridUnitType::Pixel)
            });
        }
        // Source DefaultButtonStates only changes Style. Native IsDefault and
        // IsCancel install root key handlers and invoke Button.Click, whereas
        // source OnPopupKeyDown calls Process* directly. Do not add that path.
        self.update_default_visual();
        if let Some(session) = RibbonDialogSession::for_host(self) {
            session.schedule_input_pane_adjustment();
        }
    }
    fn clear_default_visual(&self) {
        let old = self.default_visual.take();
        if let Some(old) = old {
            old.value.dispose();
        }
    }
    pub(super) fn adjust_for_input_pane(
        &self,
        owner: &TopLevel,
        pane: Rect,
    ) -> Option<InputPaneAdjustment> {
        // The source positions the dialog 12 DIPs above the input pane,
        // keeping its minimum height available even when the pane is tall.
        const BOTTOM_MARGIN: f64 = 12.;
        let parts = self.parts.borrow().input_pane.clone()?;
        let bounds_in_owner = |element: &Layoutable| {
            element.transform_to_visual(owner).map(|transform| {
                Rect::from_size(element.bounds().size()).transform_to_aabb(transform)
            })
        };
        let layout = bounds_in_owner(&parts.layout_root)?;
        let dialog = bounds_in_owner(&parts.background)?;
        if pane.height <= 0. || pane.y >= dialog.bottom() + BOTTOM_MARGIN {
            return None;
        }

        let minimum = f64::from(parts.background.min_height() as f32);
        let bottom = layout.height - (pane.y - layout.y).max(minimum) + BOTTOM_MARGIN;
        // These are temporary visual state values. Disposing the leases
        // restores local values and bindings set by the application template.
        let priority = BindingPriority::Animation;
        let values = [
            parts.background.set_value_with_priority(
                Layoutable::margin_property(),
                Thickness::new(0., 0., 0., bottom),
                priority,
            ),
            parts.background.set_value_with_priority(
                Layoutable::vertical_alignment_property(),
                VerticalAlignment::Bottom,
                priority,
            ),
            parts.content_scroll.set_value_with_priority(
                ScrollViewer::vertical_scroll_bar_visibility_property(),
                ScrollBarVisibility::Auto,
                priority,
            ),
        ]
        .into_iter()
        .flatten()
        .map(Subscription::new)
        .collect();
        Some(InputPaneAdjustment {
            scroll: parts.content_scroll,
            _values: values,
        })
    }

    fn detach_parts(&self) {
        // Dispose outside the host's parts borrow: content/theme notifications
        // can reenter the host or replace its template synchronously.
        self.template_generation
            .set(self.template_generation.get().wrapping_add(1));
        let mut outgoing = self.parts.take();
        self.default_styles.replace(Default::default());
        self.clear_default_visual();
        self.update_default_visual();
        if let Some(session) = RibbonDialogSession::for_host(self) {
            session.clear_input_pane_adjustment();
        }
        outgoing.detach();
    }
    fn update_default_visual(&self) {
        if self.updating_default.replace(true) {
            self.pending_default.set(true);
            return;
        }
        loop {
            self.pending_default.set(false);
            let generation = self.template_generation.get();
            let selected = {
                let p = self.parts.borrow();
                match self.default_button() {
                    ContentDialogButton::Primary => p.primary.clone().map(|b| (b, 0)),
                    ContentDialogButton::Secondary => p.secondary.clone().map(|b| (b, 1)),
                    ContentDialogButton::Close => p.close.clone().map(|b| (b, 2)),
                    _ => None,
                }
            };
            let theme = if let Some((_, index)) = &selected {
                let captured = self.default_styles.borrow()[*index].clone();
                match captured {
                    Some(theme) => theme,
                    None => {
                        // The source's ordinary StaticResource setter is built
                        // once when this VisualState first materializes. Each
                        // state and template instance owns its captured value.
                        // Brush ThemeResources inside that style remain live.
                        let theme = self
                            .try_find_resource(
                                &"AccentButtonStyle".into(),
                                self.actual_theme_variant().as_ref(),
                            )
                            .flatten()
                            .and_then(|v| ferroui_base::data::core::ValueTypes::as_object(&*v))
                            .and_then(|v| v.cast::<ferroui_base::styling::ControlTheme>());
                        // Resource providers can run arbitrary application
                        // code, including replacing and applying the template.
                        // Its new lazy states must resolve their own values.
                        if generation != self.template_generation.get() {
                            continue;
                        }
                        self.default_styles.borrow_mut()[*index] = Some(theme.clone());
                        theme
                    }
                }
            } else {
                None
            };
            let same = self.default_visual.borrow().as_ref().is_some_and(|old| {
                selected.as_ref().is_some_and(|(b, _)| b == &old.button) && theme == old.theme
            });
            if !same {
                self.clear_default_visual();
                if let Some((button, _)) = selected {
                    let value = button
                        .set_value_with_priority(
                            ferroui_base::StyledElement::theme_property(),
                            theme.clone(),
                            ferroui_base::data::BindingPriority::Animation,
                        )
                        .expect("visual-state theme priority");
                    self.default_visual.replace(Some(DefaultVisual {
                        button,
                        theme,
                        value,
                    }));
                }
            }
            if !self.pending_default.replace(false) {
                break;
            }
        }
        self.updating_default.set(false);
    }
}
impl FerroObjectImpl for RibbonDialogHost {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        ensure_defaults();
        this.set_is_tab_stop(false);
        KeyboardNavigation::set_tab_navigation(this, KeyboardNavigationMode::Cycle);
        AutomationProperties::set_is_dialog(this, true);
    }
    fn on_property_changed(this: &Self, change: &ferroui_base::FerroPropertyChangedEventArgs<'_>) {
        if change.property()
            == ferroui_controls::primitives::TemplatedControl::template_property().as_property()
        {
            this.detach_parts();
        }
        Self::parent_on_property_changed(this, change);
        let p = change.property();
        if [
            Self::primary_button_text_property().as_property(),
            Self::secondary_button_text_property().as_property(),
            Self::close_button_text_property().as_property(),
            Self::title_property().as_property(),
            Self::title_template_property().as_property(),
            Self::default_button_property().as_property(),
        ]
        .contains(&p)
        {
            this.update_states();
        }
    }
}
impl StyledElementImpl for RibbonDialogHost {
    fn style_key_override(_: &Self) -> &'static ferroui_base::TypeInfo {
        Self::TYPE
    }
}
impl InputElementImpl for RibbonDialogHost {
    fn is_focus_scope(_: &Self) -> bool {
        true
    }
}
impl ControlImpl for RibbonDialogHost {
    fn on_create_automation_peer(this: &Self) -> Ref<AutomationPeer> {
        instantiate(RibbonDialogAutomationPeer {
            base: ControlAutomationPeer::construct(this),
        })
        .upcast()
    }
}

#[repr(C)]
struct RibbonDialogAutomationPeer {
    base: ControlAutomationPeer,
}
ferro_class!(RibbonDialogAutomationPeer:ControlAutomationPeer);
ferro_class_info!(RibbonDialogAutomationPeer {
    interfaces: [Rc<dyn IWindowProvider> => |peer| Rc::new(DialogWindowProvider(peer)) as Rc<dyn IWindowProvider>]
});
impl FerroObjectImpl for RibbonDialogAutomationPeer {}
impl ControlAutomationPeerImpl for RibbonDialogAutomationPeer {}
impl AutomationPeerImpl for RibbonDialogAutomationPeer {
    fn get_automation_control_type_core(_: &Self) -> AutomationControlType {
        AutomationControlType::Window
    }

    fn get_name_core(this: &Self) -> Option<String> {
        use ferroui_base::data::core::ValueTypes;

        let name = Self::parent_get_name_core(this);
        if name.as_deref().is_some_and(|name| !name.trim().is_empty()) {
            return name;
        }

        let host = this.owner().cast::<RibbonDialogHost>()?;
        if let Some(title) = host.title() {
            if let Some(control) = ValueTypes::as_object(&*title).and_then(|o| o.cast::<Control>())
            {
                return Some(ControlAutomationPeer::create_peer_for_element(&control).get_name());
            }
            return Some(ValueTypes::to_display_string(Some(&title)));
        }

        let content = host.content()?;
        if let Some(text) = ValueTypes::as_object(&*content).and_then(|o| o.cast::<TextBlock>()) {
            return text.text();
        }
        content.downcast_ref::<String>().cloned()
    }
}

impl RibbonDialogAutomationPeer {
    fn dialog(&self) -> Ref<RibbonDialogHost> {
        self.owner().cast().expect("dialog peer owner")
    }
}

struct DialogWindowProvider(Ref<RibbonDialogAutomationPeer>);

impl IWindowProvider for DialogWindowProvider {
    fn peer(&self) -> Ref<AutomationPeer> {
        self.0.clone().upcast()
    }

    fn close(&self) -> Result<(), AutomationError> {
        let dialog = self.0.dialog();
        if !dialog.is_effectively_enabled() {
            return Err(AutomationError::element_not_enabled());
        }
        dialog.force_close_dialog();
        Ok(())
    }

    fn is_modal(&self) -> Result<bool, AutomationError> {
        let dialog = self.0.dialog();
        if !dialog.is_effectively_enabled() {
            return Err(AutomationError::element_not_enabled());
        }
        Ok(dialog.is_dialog_showing())
    }

    fn is_topmost(&self) -> Result<bool, AutomationError> {
        self.is_modal()
    }

    fn interaction_state(&self) -> Result<WindowInteractionState, AutomationError> {
        Ok(if self.is_modal()? {
            WindowInteractionState::Running
        } else {
            WindowInteractionState::Closing
        })
    }
}

impl ContentControlImpl for RibbonDialogHost {
    fn register_content_presenter(this: &Self, presenter: &ContentPresenter) -> bool {
        if presenter.name().as_deref() == Some("Content") {
            true
        } else {
            Self::parent_register_content_presenter(this, presenter)
        }
    }
}
impl TemplatedControlImpl for RibbonDialogHost {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        this.detach_parts();
        Self::parent_on_apply_template(this, e);
        let scope = e.name_scope();
        let input_pane = match (
            scope.find_as("LayoutRoot"),
            scope.find_as("BackgroundElement"),
            scope.find_as("ContentScrollViewer"),
        ) {
            (Some(layout_root), Some(background), Some(content_scroll)) => Some(InputPaneParts {
                layout_root,
                background,
                content_scroll,
            }),
            _ => None,
        };
        let mut parts = Parts {
            input_pane,
            title: scope.find_as("Title"),
            content: scope.find_as("Content"),
            commands: scope.find_as("CommandSpace"),
            first_spacer: scope.find_as("FirstSpacer"),
            secondary_column: scope.find_as("SecondaryColumn"),
            primary: scope.find_as("PrimaryButton"),
            secondary: scope.find_as("SecondaryButton"),
            close: scope.find_as("CloseButton"),
            clicks: Vec::new(),
            subscriptions: Vec::new(),
        };
        for (button, result) in [
            (&parts.primary, ContentDialogButton::Primary),
            (&parts.secondary, ContentDialogButton::Secondary),
            (&parts.close, ContentDialogButton::Close),
        ] {
            if let Some(button) = button {
                let weak = this.to_ref().downgrade();
                let token = button.click(move |_, _| {
                    if let Some(this) = weak.upgrade() {
                        this.process_button(result);
                    }
                });
                parts.clicks.push((button.clone(), token));
            }
        }
        let resource_anchor = parts
            .commands
            .as_ref()
            .map(|v| v.clone().upcast::<ferroui_base::StyledElement>())
            .unwrap_or_else(|| this.to_ref().upcast::<ferroui_base::StyledElement>());
        this.parts.replace(parts);
        use ferroui_base::{controls::ResourceHostRef, reactive::ObservableExt};
        let weak = this.to_ref().downgrade();
        let subscription = ResourceHostRef::from(&resource_anchor)
            .resource_observable("ContentDialogButtonSpacing", None)
            .subscribe_fn(move |_| {
                if let Some(host) = weak.upgrade() {
                    host.update_states();
                }
            });
        this.parts.borrow_mut().subscriptions.push(subscription);
        this.update_states();
    }
}
impl Drop for RibbonDialogHost {
    fn drop(&mut self) {
        if let Some(old) = self.default_visual.get_mut().take() {
            old.value.dispose();
        }
        self.parts.get_mut().detach();
    }
}

#[cfg(test)]
#[path = "../color_dialog_host_tests.rs"]
mod tests;
