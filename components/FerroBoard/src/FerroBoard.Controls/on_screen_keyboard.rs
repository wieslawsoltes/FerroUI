//! Derived from ScreenKeyboard.Uno/OnScreenKeyboard.cs (Apache-2.0).
//! Native focus/clipboard/markup limitations are tracked separately in docs/native-api.json.
use crate::{
    KeyboardStrings,
    icons::IconFactory,
    input::{PasswordBoxInputTarget, ScreenKeyboardInput, TextBoxInputTarget},
    primitives::{Brushes, FocusNeutral, TouchButton, source_max, source_min},
    services::{
        ClipboardMonitor, DispatcherKeyboardScheduler, KeyboardHaptics, SystemClipboardService,
    },
    themes::{KeyboardTheme, KeyboardThemes, ThemeList, ThemeSubscription},
    views::{
        ClipboardPanelView, EditingPanelView, EmojiPanelView, EmojiSearchState, KeysView,
        PaddedGrid, SmartbarAction, SmartbarView, SubtypePickerView,
    },
};
use ferroboard_core::{
    clipboard::InMemoryClipboardService,
    editor::{
        CapitalizationMode, EnterAction, ITextInputTarget, InputAttributes, InputKind,
        TextBufferTarget,
    },
    engine::{EngineError, KeyboardEngine},
    input::{IKeyboardScheduler, KeyboardTouchProcessor},
    keys::{KeyIcon, KeyboardMode, KeyboardUiMode},
    layouts::{ComputedKey, KeyLabels, KeyboardGeometryOptions},
    settings::{FeedbackKind, KeyboardSettings, OneHandedMode, ThemeMode},
    text::{TextUtils, Utf16Text},
};
use ferroui_base::{
    CornerRadius, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Point, Ref, StyledElementImpl, StyledProperty, Thickness,
    Visual, VisualImpl, WeakRef, ferro_class, ferro_impl_classes, ferro_properties,
    input::{InputElement, InputElementImpl, PointerEventArgs},
    instantiate,
    interactivity::{InteractiveImpl, RoutedEventHandlerToken, RoutingStrategies},
    layout::{HorizontalAlignment, LayoutableImpl, Orientation, VerticalAlignment},
    media::{Color, FontFamily},
    styling::ThemeVariant,
    threading::{Dispatcher, DispatcherPriority},
};
use ferroui_controls::{
    Border, Canvas, ColumnDefinition, ContentControlImpl, Control, ControlImpl, Grid, GridLength,
    RowDefinition, StackPanel, TextBlock, TextBox, TopLevel, UserControl,
    primitives::TemplatedControlImpl,
};
use ferroui_system::{
    IDisposable, Subscription, component_model::INotifyPropertyChanged, events::Event,
};
use std::{
    any::TypeId,
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

pub type KeyboardControlResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Clone, Copy)]
struct EngineSubscriptions {
    keyboard_changed: u64,
    state_changed: u64,
    suggestions_changed: u64,
    hide_requested: u64,
    settings_requested: u64,
    voice_input_requested: u64,
    subtype_picker_requested: u64,
    resize_mode_requested: u64,
    feedback_requested: u64,
    settings_changed: u64,
}
#[derive(Clone, Copy)]
struct ViewSubscriptions {
    hide: u64,
    back: u64,
    search: u64,
    manage: u64,
    popup: u64,
    pressed: u64,
    released: u64,
}

// Only native adapters are owned by this view. Custom targets retain their own
// lifetimes through the Core ITextInputTarget contract.
#[derive(Clone)]
enum OwnedInputAdapter {
    Text(Rc<TextBoxInputTarget>),
    Password(Rc<PasswordBoxInputTarget>),
}
impl OwnedInputAdapter {
    fn input_target(&self) -> Rc<dyn ITextInputTarget> {
        match self {
            Self::Text(target) => target.clone(),
            Self::Password(target) => target.clone(),
        }
    }
    fn detach(&self) {
        match self {
            Self::Text(target) => target.detach(),
            Self::Password(target) => target.detach(),
        }
    }
    fn is_target(&self, target: &Rc<dyn ITextInputTarget>) -> bool {
        Rc::ptr_eq(&self.input_target(), target)
    }
}

#[derive(Clone)]
pub struct EnterActionRequestedEventArgs {
    element: Option<Ref<Visual>>,
    action: EnterAction,
    handled: Rc<Cell<bool>>,
}
impl EnterActionRequestedEventArgs {
    pub fn element(&self) -> Option<Ref<Visual>> {
        self.element.clone()
    }
    pub fn action(&self) -> EnterAction {
        self.action
    }
    pub fn handled(&self) -> bool {
        self.handled.get()
    }
    pub fn set_handled(&self, value: bool) {
        self.handled.set(value);
    }
    pub fn new(element: Option<Ref<Visual>>, action: EnterAction) -> Self {
        Self {
            element,
            action,
            handled: Rc::new(Cell::new(false)),
        }
    }
}
// Theme notifications post numeric identities to the owning UI thread. The map is weak: neither
// a global theme registration nor an already queued notification owns a keyboard control.
ferroui_base::ferro_thread_local! { static THEME_OWNERS: RefCell<HashMap<u64, WeakRef<OnScreenKeyboard>>> = RefCell::new(HashMap::new()); }
static NEXT_THEME_OWNER: AtomicU64 = AtomicU64::new(1);

#[repr(C)]
pub struct OnScreenKeyboard {
    base: UserControl,
    root: Ref<Grid>,
    body: Ref<PaddedGrid>,
    main: Ref<Grid>,
    side_panel: Ref<Border>,
    overlay: Ref<Canvas>,
    resize_overlay: Ref<Grid>,
    preview: Ref<Border>,
    preview_text: Ref<TextBlock>,
    popup: Ref<Border>,
    popup_canvas: Ref<Canvas>,
    scheduler: Rc<dyn IKeyboardScheduler>,
    engine: Rc<KeyboardEngine>,
    touch_processor: Rc<KeyboardTouchProcessor>,
    keys_view: Ref<KeysView>,
    smartbar: Ref<SmartbarView>,
    emoji_panel: Ref<EmojiPanelView>,
    clipboard_panel: Ref<ClipboardPanelView>,
    editing_panel: Ref<EditingPanelView>,
    subtype_picker: Ref<SubtypePickerView>,
    effective_theme: RefCell<Arc<KeyboardTheme>>,
    owned_target: RefCell<Option<OwnedInputAdapter>>,
    target_element: RefCell<Option<Ref<Visual>>>,
    search_original_target: RefCell<Option<Rc<dyn ITextInputTarget>>>,
    search_buffer: RefCell<Option<Rc<TextBufferTarget>>>,
    search_state: RefCell<Option<Rc<EmojiSearchState>>>,
    focus_tracking: Cell<bool>,
    loaded: Cell<bool>,
    use_window_clipboard: bool,
    clipboard_monitor: RefCell<Option<Rc<ClipboardMonitor>>>,
    focus_subscription: RefCell<Option<(Ref<TopLevel>, RoutedEventHandlerToken)>>,
    subscriptions: RefCell<Vec<Subscription>>,
    engine_subscriptions: Cell<Option<EngineSubscriptions>>,
    view_subscriptions: Cell<Option<ViewSubscriptions>>,
    theme_subscription: RefCell<Option<(u64, ThemeSubscription)>>,
    shown_ui_mode: Cell<KeyboardUiMode>,
    shown_mode: Cell<KeyboardMode>,
    preview_key: RefCell<Option<Rc<ComputedKey>>>,
    resize_start_y: Cell<f64>,
    resize_start_scale: Cell<f64>,
    resize_pointer: Cell<Option<i32>>,
    resize_coordinates: RefCell<Option<(ferroui_base::WeakRef<Visual>, ferroui_base::Matrix)>>,
    settings_requested: Event<Self>,
    hide_requested: Event<Self>,
    voice_input_requested: Event<Self>,
    enter_action_requested: Event<EnterActionRequestedEventArgs>,
    feedback_requested: Event<FeedbackKind>,
    target_changed: Event<Self>,
}
ferro_class!(OnScreenKeyboard:UserControl);
ferro_impl_classes!(OnScreenKeyboard:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,TemplatedControlImpl,ContentControlImpl,ControlImpl);
ferro_properties! {impl OnScreenKeyboard {
    pub fn settings_property()->StyledProperty<Option<Rc<KeyboardSettings>>> {
        FerroProperty::register::<OnScreenKeyboard,_>("Settings",None)
    }
    pub fn theme_property()->StyledProperty<Option<Arc<KeyboardTheme>>> {
        FerroProperty::register::<OnScreenKeyboard,_>("Theme",None)
    }
    pub fn auto_attach_property()->StyledProperty<bool> {
        FerroProperty::register::<OnScreenKeyboard,_>("AutoAttach",true)
    }
}}
impl FerroObjectImpl for OnScreenKeyboard {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_allow_focus_on_interaction(false);
        this.set_is_tab_stop(false);
        this.set_horizontal_alignment(HorizontalAlignment::Stretch);
        this.set_vertical_alignment(VerticalAlignment::Bottom);
        this.build_visual_tree();
        if this.use_window_clipboard {
            this.engine
                .set_clipboard_service(Rc::new(SystemClipboardService::for_visual(
                    this.to_ref().upcast(),
                )));
        }
        this.install_clipboard_monitor();
        this.set_value(Self::settings_property(), Some(this.engine.settings()));
        this.hook_engine();
        let weak = this.to_ref().downgrade();
        let hide = this.smartbar.hide_requested(move |_| {
            if let Some(owner) = weak.upgrade() {
                owner.on_hide_requested().expect("keyboard hide");
            }
        });
        let weak = this.to_ref().downgrade();
        let back = this.smartbar.search_back_requested(move |_| {
            if let Some(owner) = weak.upgrade() {
                owner.end_emoji_search(true).expect("emoji search back");
            }
        });
        let weak = this.to_ref().downgrade();
        let search = this.emoji_panel.search_requested(move |_| {
            if let Some(owner) = weak.upgrade() {
                owner.begin_emoji_search().expect("emoji search");
            }
        });
        let weak = this.to_ref().downgrade();
        let manage = this.subtype_picker.manage_requested(move |_| {
            if let Some(owner) = weak.upgrade() {
                owner.settings_requested.raise(&owner);
            }
        });
        let weak = this.to_ref().downgrade();
        let popup = this.touch_processor.popup_changed().add(Rc::new(move |_| {
            if let Some(owner) = weak.upgrade() {
                owner.update_popup().expect("key popup");
            }
        }));
        let weak = this.to_ref().downgrade();
        let pressed = this.keys_view.key_pressed_visual(move |_, key| {
            if let Some(owner) = weak.upgrade() {
                owner.show_preview(key.clone()).expect("key preview");
            }
        });
        let weak = this.to_ref().downgrade();
        let released = this.keys_view.key_released_visual(move |_, key| {
            if let Some(owner) = weak.upgrade() {
                owner.hide_preview(Some(key));
            }
        });
        this.view_subscriptions.set(Some(ViewSubscriptions {
            hide,
            back,
            search,
            manage,
            popup,
            pressed,
            released,
        }));
        let weak = this.to_ref().downgrade();
        this.loaded(move |_, _| {
            if let Some(owner) = weak.upgrade() {
                owner.loaded.set(true);
                owner
                    .update_focus_tracking()
                    .expect("keyboard loaded focus");
                owner.apply_theme().expect("keyboard loaded theme");
                owner.update_clipboard_monitor();
            }
        });
        let weak = this.to_ref().downgrade();
        this.unloaded(move |_, _| {
            if let Some(owner) = weak.upgrade() {
                owner.loaded.set(false);
                owner.resize_pointer.set(None);
                owner.resize_coordinates.borrow_mut().take();
                owner.update_clipboard_monitor();
                owner
                    .update_focus_tracking()
                    .expect("keyboard unloaded focus");
                owner
                    .touch_processor
                    .cancel_all()
                    .expect("keyboard unloaded touch");
            }
        });
        let weak = this.to_ref().downgrade();
        this.subscriptions
            .borrow_mut()
            .push(this.actual_theme_variant_changed(move || {
                if let Some(owner) = weak.upgrade() {
                    owner.apply_theme().expect("keyboard actual theme");
                }
            }));
        let weak = this.to_ref().downgrade();
        this.size_changed(move |_, e| {
            if let Some(owner) = weak.upgrade()
                && (e.new_size().width - e.previous_size().width).abs() > 0.5
                && (owner.engine.settings().one_handed_mode() != OneHandedMode::Off
                    || owner.engine.settings().split_keyboard())
            {
                owner.update_layout_mode().expect("keyboard size mode");
            }
        });
        let id = NEXT_THEME_OWNER.fetch_add(1, Ordering::Relaxed);
        THEME_OWNERS.with_borrow_mut(|owners| {
            owners.insert(id, this.to_ref().downgrade());
        });
        let dispatcher = Dispatcher::current_dispatcher();
        let subscription = KeyboardThemes::changed(move || {
            dispatcher.post(
                move || {
                    let owner = THEME_OWNERS
                        .with_borrow(|owners| owners.get(&id).and_then(WeakRef::upgrade));
                    if let Some(owner) = owner {
                        owner.apply_theme().expect("registered keyboard theme");
                    }
                },
                DispatcherPriority::NORMAL,
            )
        });
        this.theme_subscription.replace(Some((id, subscription)));
    }
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::settings_property().as_property() {
            this.on_settings_replaced()
                .expect("keyboard settings replacement");
        } else if change.property() == Self::theme_property().as_property() {
            this.apply_theme().expect("keyboard theme property");
        } else if change.property() == Self::auto_attach_property().as_property() {
            this.update_focus_tracking().expect("keyboard AutoAttach");
        }
    }
}
impl OnScreenKeyboard {
    /// Constructs the complete native keyboard around a supplied engine or the bundled defaults.
    /// Fallible Core resource construction propagates instead of producing an empty keyboard.
    pub fn new(engine: Option<Rc<KeyboardEngine>>) -> KeyboardControlResult<Ref<Self>> {
        Self::with_scheduler(engine, Rc::new(DispatcherKeyboardScheduler::default()))
    }
    /// Deterministic timer injection; advanced port extension in addition to source constructors.
    pub fn with_scheduler(
        engine: Option<Rc<KeyboardEngine>>,
        scheduler: Rc<dyn IKeyboardScheduler>,
    ) -> KeyboardControlResult<Ref<Self>> {
        let engine = if let Some(engine) = engine {
            engine
        } else {
            KeyboardEngine::with_defaults()?
        };
        if engine.clipboard_spawner().is_none() {
            let dispatcher = Dispatcher::current_dispatcher();
            engine.set_clipboard_spawner(Some(Rc::new(move |job| {
                drop(dispatcher.to_task_scheduler().start_local(job));
            })));
        }
        let use_window_clipboard = engine.clipboard_service().as_ref().type_id()
            == TypeId::of::<InMemoryClipboardService>();
        if use_window_clipboard {
            engine.set_clipboard_service(Rc::new(SystemClipboardService::new()));
        }
        let touch_processor = KeyboardTouchProcessor::new(engine.clone(), scheduler.clone());
        Ok(instantiate(Self {
            base: UserControl::construct(),
            root: Grid::new(),
            body: PaddedGrid::new(Thickness::uniform(0.0)),
            main: Grid::new(),
            side_panel: Border::new(),
            overlay: Canvas::new(),
            resize_overlay: Grid::new(),
            preview: Border::new(),
            preview_text: TextBlock::new(),
            popup: Border::new(),
            popup_canvas: Canvas::new(),
            keys_view: KeysView::new(engine.clone(), touch_processor.clone(), scheduler.clone()),
            smartbar: SmartbarView::new(engine.clone(), scheduler.clone()),
            emoji_panel: EmojiPanelView::new(engine.clone(), scheduler.clone()),
            clipboard_panel: ClipboardPanelView::new(engine.clone(), scheduler.clone()),
            editing_panel: EditingPanelView::new(engine.clone(), scheduler.clone()),
            subtype_picker: SubtypePickerView::new(engine.clone(), scheduler.clone()),
            scheduler,
            engine,
            touch_processor,
            effective_theme: RefCell::new(KeyboardThemes::floris_day()),
            owned_target: RefCell::new(None),
            target_element: RefCell::new(None),
            search_original_target: RefCell::new(None),
            search_buffer: RefCell::new(None),
            search_state: RefCell::new(None),
            focus_tracking: Cell::new(false),
            loaded: Cell::new(false),
            use_window_clipboard,
            clipboard_monitor: RefCell::new(None),
            focus_subscription: RefCell::new(None),
            subscriptions: RefCell::new(Vec::new()),
            engine_subscriptions: Cell::new(None),
            view_subscriptions: Cell::new(None),
            theme_subscription: RefCell::new(None),
            shown_ui_mode: Cell::new(KeyboardUiMode(-1)),
            shown_mode: Cell::new(KeyboardMode(-1)),
            preview_key: RefCell::new(None),
            resize_start_y: Cell::new(0.0),
            resize_start_scale: Cell::new(0.0),
            resize_pointer: Cell::new(None),
            resize_coordinates: RefCell::new(None),
            settings_requested: Event::new(),
            hide_requested: Event::new(),
            voice_input_requested: Event::new(),
            enter_action_requested: Event::new(),
            feedback_requested: Event::new(),
            target_changed: Event::new(),
        }))
    }
    // The pinned markup event invoker is add-only and discards the callable's return.
    // Retain cleanup with this native owner so explicit library release removes the handler.
    pub(crate) fn retain_markup_subscription(&self, subscription: Rc<dyn IDisposable>) {
        self.subscriptions.borrow_mut().push(subscription.into());
    }
    pub fn engine(&self) -> Rc<KeyboardEngine> {
        self.engine.clone()
    }
    pub fn touch_processor(&self) -> Rc<KeyboardTouchProcessor> {
        self.touch_processor.clone()
    }
    pub fn settings(&self) -> Rc<KeyboardSettings> {
        self.get_value(Self::settings_property())
            .expect("source settings restored")
    }
    pub fn set_settings(&self, value: Option<Rc<KeyboardSettings>>) {
        self.set_value(Self::settings_property(), value);
    }
    pub fn theme(&self) -> Option<Arc<KeyboardTheme>> {
        self.get_value(Self::theme_property())
    }
    pub fn set_theme(&self, value: Option<Arc<KeyboardTheme>>) {
        self.set_value(Self::theme_property(), value);
    }
    pub fn effective_theme(&self) -> Arc<KeyboardTheme> {
        self.effective_theme.borrow().clone()
    }
    pub fn auto_attach(&self) -> bool {
        self.get_value(Self::auto_attach_property())
    }
    pub fn set_auto_attach(&self, value: bool) {
        self.set_value(Self::auto_attach_property(), value);
    }
    pub fn target_element(&self) -> Option<Ref<Visual>> {
        self.target_element.borrow().clone()
    }
    pub fn smartbar_actions(&self) -> Option<ThemeList<Option<Rc<SmartbarAction>>>> {
        self.smartbar.actions()
    }
    pub fn settings_requested(&self) -> &Event<Self> {
        &self.settings_requested
    }
    pub fn hide_requested(&self) -> &Event<Self> {
        &self.hide_requested
    }
    pub fn voice_input_requested(&self) -> &Event<Self> {
        &self.voice_input_requested
    }
    pub fn enter_action_requested(&self) -> &Event<EnterActionRequestedEventArgs> {
        &self.enter_action_requested
    }
    pub fn feedback_requested(&self) -> &Event<FeedbackKind> {
        &self.feedback_requested
    }
    pub fn target_changed(&self) -> &Event<Self> {
        &self.target_changed
    }
    pub fn attach_to(&self, element: Option<Ref<Visual>>) -> KeyboardControlResult<bool> {
        if element.is_some() && element == self.target_element() {
            return Ok(true);
        }
        self.end_emoji_search(false)?;
        let target: Option<OwnedInputAdapter> = if let Some(element) = element.as_ref() {
            if let Some(tb) = element.downcast_ref::<TextBox>() {
                if ScreenKeyboardInput::is_enabled_for(element) {
                    let weak = self.to_ref().downgrade();
                    let visual = element.clone();
                    let action = Some(Rc::new(move |action| {
                        weak.upgrade().is_some_and(|owner| {
                            owner
                                .raise_enter_action(visual.clone(), action)
                                .expect("keyboard enter action")
                        })
                    })
                        as ferroboard_core::editor::EnterActionHandler);
                    if tb.password_char() != '\0' {
                        let target = PasswordBoxInputTarget::for_keyboard(tb.to_ref(), action)?;
                        Some(OwnedInputAdapter::Password(target))
                    } else {
                        let target = TextBoxInputTarget::for_keyboard(tb.to_ref(), action)?;
                        Some(OwnedInputAdapter::Text(target))
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        if element.is_some() && target.is_none() {
            return Ok(false);
        }
        let old = self.owned_target.borrow().clone();
        if let Some(old) = old {
            old.detach();
        }
        *self.owned_target.borrow_mut() = target.clone();
        *self.target_element.borrow_mut() = element;
        if let Some(target) = target {
            self.engine.attach(target.input_target())?;
        } else {
            self.engine.detach()?;
        }
        self.target_changed.raise(self);
        Ok(true)
    }
    pub fn attach(&self, target: Rc<dyn ITextInputTarget>) -> KeyboardControlResult<()> {
        self.end_emoji_search(false)?;
        let old = self.owned_target.borrow().clone();
        if let Some(old) = old {
            old.detach();
        }
        *self.owned_target.borrow_mut() = None;
        *self.target_element.borrow_mut() = None;
        self.engine.attach(target)?;
        self.target_changed.raise(self);
        Ok(())
    }
    pub fn detach(&self) -> KeyboardControlResult<()> {
        self.attach_to(None).map(|_| ())
    }
    pub fn is_text_input(element: Option<&Visual>) -> bool {
        element.is_some_and(|element| {
            element.downcast_ref::<TextBox>().is_some_and(|tb| {
                (tb.password_char() != '\0' || !tb.is_read_only())
                    && ScreenKeyboardInput::is_enabled_for(element)
            })
        })
    }
    fn build_visual_tree(&self) {
        for _ in 0..2 {
            let row = RowDefinition::new();
            row.set_height(GridLength::auto());
            self.root.row_definitions().add(row);
        }
        Grid::set_row(&self.smartbar, 0);
        Grid::set_row(&self.body, 1);
        self.main.children().add(self.keys_view.clone());
        self.main.children().add(self.emoji_panel.clone());
        self.main.children().add(self.clipboard_panel.clone());
        self.main.children().add(self.editing_panel.clone());
        self.main.children().add(self.subtype_picker.clone());
        self.subtype_picker.set_is_visible(false);
        let body = self.body.content_grid();
        for width in [GridLength::star(), GridLength::auto(), GridLength::star()] {
            let column = ColumnDefinition::new();
            column.set_width(width);
            body.column_definitions().add(column);
        }
        body.children().add(self.main.clone());
        body.children().add(self.side_panel.clone());
        self.resize_overlay.set_is_visible(false);
        let weak = self.to_ref().downgrade();
        self.resize_overlay.pointer_pressed(move |_, e| {
            if let Some(owner) = weak.upgrade() {
                owner.resize_pointer_pressed(e);
            }
        });
        let weak = self.to_ref().downgrade();
        self.resize_overlay.pointer_moved(move |_, e| {
            if let Some(owner) = weak.upgrade() {
                owner.resize_pointer_moved(e);
            }
        });
        let weak = self.to_ref().downgrade();
        self.resize_overlay.pointer_released(move |_, e| {
            if let Some(owner) = weak.upgrade() {
                owner.resize_pointer_released(e);
            }
        });
        // The native pin exposes capture loss instead of Uno PointerCanceled. Reset the source
        // single resize pointer without releasing a different element's newly acquired capture.
        let weak = self.to_ref().downgrade();
        self.resize_overlay.pointer_capture_lost(move |_, _| {
            if let Some(owner) = weak.upgrade() {
                owner.resize_pointer.set(None);
                owner.resize_coordinates.borrow_mut().take();
            }
        });
        self.overlay.set_is_hit_test_visible(false);
        self.preview.set_is_hit_test_visible(false);
        self.popup.set_is_hit_test_visible(false);
        self.popup_canvas.set_is_hit_test_visible(false);
        self.preview.set_is_visible(false);
        self.popup.set_is_visible(false);
        self.preview_text
            .set_horizontal_alignment(HorizontalAlignment::Center);
        self.preview_text
            .set_vertical_alignment(VerticalAlignment::Center);
        self.preview_text.set_is_hit_test_visible(false);
        self.preview.set_child(self.preview_text.clone());
        self.popup.set_child(self.popup_canvas.clone());
        self.overlay.children().add(self.preview.clone());
        self.overlay.children().add(self.popup.clone());
        Grid::set_row_span(&self.overlay, 2);
        Grid::set_row(&self.resize_overlay, 1);
        self.root.children().add(self.smartbar.clone());
        self.root.children().add(self.body.clone());
        self.root.children().add(self.resize_overlay.clone());
        self.root.children().add(self.overlay.clone());
        self.set_content(Some(Control::boxed(self.root.clone())));
    }
    fn install_clipboard_monitor(&self) {
        let reader_owner = self.to_ref().downgrade();
        let sink_owner = reader_owner.clone();
        let monitor = ClipboardMonitor::new(
            move || {
                if let Some(owner) = reader_owner.upgrade() {
                    let service = owner.engine.clipboard_service();
                    if let Some(service) = (service.as_ref() as &dyn std::any::Any)
                        .downcast_ref::<SystemClipboardService>()
                    {
                        return service.read_text();
                    }
                }
                Box::pin(std::future::ready(Err(
                    ferroui_base::input::platform::ClipboardError::canceled(
                        "Keyboard clipboard detached",
                    ),
                )))
            },
            move |text| {
                if let Some(owner) = sink_owner.upgrade() {
                    // RecordClipboard checks the current privacy settings again before mutation.
                    owner
                        .engine
                        .record_clipboard(&text.as_str().into())
                        .expect("native clipboard history change");
                }
            },
        );
        *self.clipboard_monitor.borrow_mut() = Some(Rc::new(monitor));
    }
    fn update_clipboard_monitor(&self) {
        let monitor = self.clipboard_monitor.borrow().clone();
        if let Some(monitor) = monitor {
            monitor.set_enabled(
                self.loaded.get()
                    && self.engine.settings().clipboard_history_enabled()
                    && !self.engine.is_incognito()
                    && self.engine.clipboard_service().as_ref().type_id()
                        == TypeId::of::<SystemClipboardService>(),
            );
        }
    }
    /// Explicit native refresh hook for window activation or an application clipboard notification.
    pub fn refresh_system_clipboard(&self) {
        self.update_clipboard_monitor();
        let monitor = self.clipboard_monitor.borrow().clone();
        if let Some(monitor) = monitor {
            monitor.refresh();
        }
    }
    pub fn clipboard_monitor_error(&self) -> Option<ferroui_base::input::platform::ClipboardError> {
        self.clipboard_monitor
            .borrow()
            .as_ref()
            .and_then(|monitor| monitor.last_error())
    }
    fn hook_engine(&self) {
        macro_rules! hook {
            ($event:ident,$action:expr) => {{
                let weak = self.to_ref().downgrade();
                self.engine.$event().add(Rc::new(move |arg| {
                    if let Some(owner) = weak.upgrade() {
                        ($action)(&owner, arg);
                    }
                }))
            }};
        }
        let keyboard_changed = hook!(keyboard_changed, |owner: &Self, _| owner
            .keys_view
            .refresh(false)
            .expect("keyboard keys changed"));
        let state_changed = hook!(state_changed, |owner: &Self, _| owner
            .on_engine_state_changed()
            .expect("keyboard state"));
        let suggestions_changed = hook!(suggestions_changed, |owner: &Self, _| owner
            .smartbar
            .refresh()
            .expect("keyboard suggestions"));
        let hide_requested = hook!(hide_requested, |owner: &Self, _| owner
            .on_hide_requested()
            .expect("keyboard hide requested"));
        let settings_requested = hook!(settings_requested, |owner: &Self, _| owner
            .settings_requested
            .raise(owner));
        let voice_input_requested = hook!(voice_input_requested, |owner: &Self, _| owner
            .voice_input_requested
            .raise(owner));
        let subtype_picker_requested = hook!(subtype_picker_requested, |owner: &Self, _| owner
            .subtype_picker
            .open()
            .expect("keyboard subtype picker"));
        let resize_mode_requested = hook!(resize_mode_requested, |owner: &Self, _| owner
            .show_resize_overlay()
            .expect("keyboard resize overlay"));
        let feedback_requested = hook!(feedback_requested, |owner: &Self, kind: &FeedbackKind| {
            owner.on_feedback(*kind).expect("keyboard feedback")
        });
        let weak = self.to_ref().downgrade();
        let settings = self.engine.settings();
        let settings_changed = settings.property_changed().add(Rc::new(move |name| {
            if let Some(owner) = weak.upgrade() {
                owner
                    .on_settings_property_changed(name)
                    .expect("keyboard setting changed");
            }
        }));
        self.engine_subscriptions.set(Some(EngineSubscriptions {
            keyboard_changed,
            state_changed,
            suggestions_changed,
            hide_requested,
            settings_requested,
            voice_input_requested,
            subtype_picker_requested,
            resize_mode_requested,
            feedback_requested,
            settings_changed,
        }));
    }
    fn on_settings_replaced(&self) -> KeyboardControlResult<()> {
        let value = self.get_value(Self::settings_property());
        if value
            .as_ref()
            .is_none_or(|value| !Rc::ptr_eq(value, &self.engine.settings()))
        {
            if let Some(value) = value {
                self.engine.settings().copy_from(&value)?;
            }
            self.set_value(Self::settings_property(), Some(self.engine.settings()));
        }
        Ok(())
    }
    fn on_settings_property_changed(&self, name: &str) -> KeyboardControlResult<()> {
        self.update_clipboard_monitor();
        match name {
            "ThemeMode"
            | "LightTheme"
            | "DarkTheme"
            | "KeyHeight"
            | "HeightScale"
            | "FontScale"
            | "KeySpacingHorizontal"
            | "KeySpacingVertical"
            | "OneHandedMode"
            | "OneHandedWidth"
            | "SplitKeyboard"
            | "SplitGapRatio"
            | "MaxKeyboardWidth"
            | "SmartbarEnabled"
            | "SmartbarLayout"
            | "NumberRow" => self.apply_theme()?,
            "IncognitoMode" | "AutoCorrect" => self.smartbar.refresh()?,
            _ => {}
        }
        Ok(())
    }
    fn resolve_theme(&self) -> KeyboardControlResult<Arc<KeyboardTheme>> {
        if let Some(theme) = self.theme() {
            return Ok(theme);
        }
        let settings = self.engine.settings();
        let dark = match settings.theme_mode() {
            ThemeMode::Dark => true,
            ThemeMode::Light => false,
            _ => self.actual_theme_variant() == Some(ThemeVariant::dark()),
        };
        Ok(KeyboardThemes::get_text(Some(&Utf16Text::from(
            if dark {
                settings.dark_theme()
            } else {
                settings.light_theme()
            }
            .as_str(),
        )))?
        .unwrap_or_else(|| {
            if dark {
                KeyboardThemes::floris_night()
            } else {
                KeyboardThemes::floris_day()
            }
        }))
    }
    pub fn apply_theme(&self) -> KeyboardControlResult<()> {
        let theme = self.resolve_theme()?;
        *self.effective_theme.borrow_mut() = theme.clone();
        let settings = self.engine.settings();
        let row_height = settings.key_height() * settings.height_scale();
        let rows = if settings.number_row() { 5.0 } else { 4.0 };
        let smartbar_height = source_max(36.0, row_height * 0.82).round_ties_even();
        self.root
            .set_background(Some(Brushes::get(theme.background()).into()));
        self.body.set_height(rows * row_height + 6.0);
        self.body.set_padding(Thickness::uniform(3.0));
        self.smartbar.set_is_visible(settings.smartbar_enabled());
        self.smartbar.set_theme(theme.clone(), smartbar_height)?;
        let font_family = theme
            .font_family()
            .map(|ff| ff.to_utf8().map(|s| FontFamily::from(s.as_str())))
            .transpose()?;
        self.keys_view.set_font_scale(settings.font_scale());
        self.keys_view.set_label_font_family(font_family.clone());
        self.keys_view
            .set_geometry(Rc::new(KeyboardGeometryOptions::from_values(
                ferroboard_core::layouts::KeyboardGeometryOptionsValues {
                    key_spacing_horizontal: settings.key_spacing_horizontal(),
                    key_spacing_vertical: settings.key_spacing_vertical(),
                    split_gap: 0.0,
                    ..Default::default()
                },
            )));
        self.touch_processor.set_popup_min_y(
            -(if settings.smartbar_enabled() {
                smartbar_height
            } else {
                0.0
            }) - row_height * 2.0,
        );
        self.keys_view.set_theme(theme.clone())?;
        self.emoji_panel.set_bar_height(smartbar_height);
        self.emoji_panel.set_theme(theme.clone())?;
        self.clipboard_panel.set_header_height(smartbar_height);
        self.clipboard_panel.set_theme(theme.clone())?;
        self.editing_panel.set_theme(theme.clone())?;
        self.subtype_picker.set_theme(theme.clone());
        for border in [&self.preview, &self.popup] {
            border.set_background(Some(Brushes::get(theme.popup_background()).into()));
            border.set_border_brush(Some(Brushes::get(theme.popup_border_color()).into()));
            border.set_border_thickness(Thickness::uniform(1.0));
            border.set_corner_radius(CornerRadius::uniform(theme.popup_corner_radius()));
        }
        self.preview_text
            .set_foreground(Some(Brushes::get(theme.popup_foreground()).into()));
        if let Some(font_family) = font_family {
            self.preview_text.set_font_family(font_family);
        }
        self.update_layout_mode()?;
        self.update_panels()?;
        FocusNeutral::apply(&self.root);
        Ok(())
    }
    fn update_layout_mode(&self) -> KeyboardControlResult<()> {
        let settings = self.engine.settings();
        let theme = self.effective_theme();
        self.side_panel.set_child(None::<Ref<Control>>);
        self.side_panel.set_is_visible(false);
        self.main
            .set_max_width(if settings.max_keyboard_width() > 0.0 {
                settings.max_keyboard_width()
            } else {
                f64::INFINITY
            });
        self.main
            .set_horizontal_alignment(HorizontalAlignment::Stretch);
        Grid::set_column_span(&self.main, 3);
        Grid::set_column(&self.main, 0);
        let width = if self.bounds().width > 0.0 {
            self.bounds().width
        } else {
            800.0
        };
        if settings.one_handed_mode() != OneHandedMode::Off {
            let main_width = source_min(width * settings.one_handed_width(), self.main.max_width());
            self.main.set_width(main_width);
            let left = settings.one_handed_mode() == OneHandedMode::Left;
            self.main.set_horizontal_alignment(if left {
                HorizontalAlignment::Left
            } else {
                HorizontalAlignment::Right
            });
            self.side_panel
                .set_width(source_max(48.0, width - main_width - 6.0));
            self.side_panel.set_horizontal_alignment(if left {
                HorizontalAlignment::Right
            } else {
                HorizontalAlignment::Left
            });
            Grid::set_column_span(&self.side_panel, 3);
            self.side_panel
                .set_background(Some(Brushes::get(theme.one_handed_background()).into()));
            self.side_panel
                .set_corner_radius(CornerRadius::uniform(theme.key_corner_radius()));
            let panel = self.build_one_handed_panel(&theme)?;
            FocusNeutral::apply(&panel);
            self.side_panel.set_child(panel);
            self.side_panel.set_is_visible(true);
        } else {
            self.main.set_width(f64::from_bits(0xfff8_0000_0000_0000));
        }
        let split_gap = if settings.split_keyboard()
            && matches!(
                self.engine.mode(),
                KeyboardMode::Characters | KeyboardMode::Symbols | KeyboardMode::Symbols2
            )
            && settings.one_handed_mode() == OneHandedMode::Off
        {
            source_min(width, self.main.max_width()) * settings.split_gap_ratio()
        } else {
            0.0
        };
        let geometry = self.keys_view.geometry();
        if (geometry.split_gap() - split_gap).abs() > 0.1 {
            self.keys_view
                .set_geometry(Rc::new(KeyboardGeometryOptions::from_values(
                    ferroboard_core::layouts::KeyboardGeometryOptionsValues {
                        split_gap,
                        ..geometry.values()
                    },
                )));
        }
        self.keys_view.relayout()?;
        Ok(())
    }
    fn build_one_handed_panel(
        &self,
        theme: &KeyboardTheme,
    ) -> KeyboardControlResult<Ref<StackPanel>> {
        let panel = StackPanel::new();
        panel.set_vertical_alignment(VerticalAlignment::Center);
        panel.set_horizontal_alignment(HorizontalAlignment::Center);
        panel.set_spacing(12.0);
        let exit = TouchButton::new();
        exit.set_width(44.0);
        exit.set_height(44.0);
        exit.set_corner_radius(CornerRadius::uniform(22.0));
        exit.set_scheduler(self.scheduler.clone());
        exit.set_colors(
            theme.action_button_background(),
            theme.action_button_pressed_background(),
        );
        let name = crate::views::view_helpers::nullable_text(KeyboardStrings::exit_one_handed())?;
        exit.set_icon(
            KeyIcon::Resize,
            22.0,
            Some(Brushes::get(theme.one_handed_foreground()).into()),
            name.as_deref(),
        )?;
        let engine = self.engine.clone();
        exit.click(move |_| engine.settings().set_one_handed_mode(OneHandedMode::Off));
        let button = TouchButton::new();
        button.set_width(44.0);
        button.set_height(44.0);
        button.set_corner_radius(CornerRadius::uniform(22.0));
        button.set_scheduler(self.scheduler.clone());
        button.set_colors(
            theme.action_button_background(),
            theme.action_button_pressed_background(),
        );
        let to_left = self.engine.settings().one_handed_mode() == OneHandedMode::Right;
        let name = crate::views::view_helpers::nullable_text(if to_left {
            KeyboardStrings::move_keyboard_left()
        } else {
            KeyboardStrings::move_keyboard_right()
        })?;
        button.set_icon(
            if to_left {
                KeyIcon::ChevronLeft
            } else {
                KeyIcon::ChevronRight
            },
            26.0,
            Some(Brushes::get(theme.one_handed_foreground()).into()),
            name.as_deref(),
        )?;
        let engine = self.engine.clone();
        button.click(move |_| {
            engine.settings().set_one_handed_mode(if to_left {
                OneHandedMode::Left
            } else {
                OneHandedMode::Right
            })
        });
        panel.children().add(exit);
        panel.children().add(button);
        Ok(panel)
    }
    fn on_engine_state_changed(&self) -> KeyboardControlResult<()> {
        self.update_clipboard_monitor();
        if self.shown_mode.get() != self.engine.mode() {
            self.shown_mode.set(self.engine.mode());
            if self.engine.settings().split_keyboard() {
                self.update_layout_mode()?;
            }
        }
        self.update_panels()?;
        self.keys_view.refresh(false)?;
        self.smartbar.refresh()?;
        Ok(())
    }
    fn update_panels(&self) -> KeyboardControlResult<()> {
        let mode = self.engine.ui_mode();
        self.keys_view.set_is_visible(mode == KeyboardUiMode::Text);
        self.emoji_panel
            .set_is_visible(mode == KeyboardUiMode::Media);
        self.clipboard_panel
            .set_is_visible(mode == KeyboardUiMode::Clipboard);
        self.editing_panel
            .set_is_visible(mode == KeyboardUiMode::Editing);
        if self.shown_ui_mode.get() != mode {
            self.shown_ui_mode.set(mode);
            self.touch_processor.cancel_all()?;
            self.hide_preview(None);
            self.subtype_picker.close();
            match mode {
                KeyboardUiMode::Media => self.emoji_panel.on_shown()?,
                KeyboardUiMode::Clipboard => self.clipboard_panel.refresh()?,
                _ => {}
            }
        }
        Ok(())
    }
    fn on_hide_requested(&self) -> KeyboardControlResult<()> {
        self.end_emoji_search(false)?;
        self.hide_requested.raise(self);
        Ok(())
    }
    fn raise_enter_action(
        &self,
        element: Ref<Visual>,
        action: EnterAction,
    ) -> KeyboardControlResult<bool> {
        let args = EnterActionRequestedEventArgs::new(Some(element), action);
        self.enter_action_requested.raise(&args);
        if !args.handled.get() && action == EnterAction::Done {
            self.on_hide_requested()?;
            return Ok(true);
        }
        Ok(args.handled.get())
    }
    fn on_feedback(&self, kind: FeedbackKind) -> KeyboardControlResult<()> {
        let settings = self.engine.settings();
        if settings.haptic_feedback() {
            KeyboardHaptics::perform(kind, settings.haptic_duration())?;
        }
        self.feedback_requested.raise(&kind);
        Ok(())
    }
    fn show_preview(&self, key: Rc<ComputedKey>) -> KeyboardControlResult<()> {
        if !self.engine.settings().show_key_preview()
            || !key.is_character_key().map_err(EngineError::from)?
            || key.label().is_none()
            || self.engine.is_gliding()
        {
            return Ok(());
        }
        let b = key.visible_bounds();
        let origin = self
            .keys_view
            .translate_point(Point::new(b.x, b.y), &self.overlay)
            .ok_or(crate::views::view_helpers::ViewError::TransformUnavailable)?;
        let width = source_max(b.width * 1.3, 36.0);
        let height = source_max(b.height * 1.15, 40.0);
        self.preview.set_width(width);
        self.preview.set_height(height);
        let label = crate::views::view_helpers::nullable_text(key.label())?;
        self.preview_text.set_text(label.as_deref());
        self.preview_text.set_font_size(source_min(
            self.effective_theme().key_font_size() * 1.4 * self.engine.settings().font_scale(),
            height * 0.62,
        ));
        let x = (origin.x + b.width / 2.0 - width / 2.0)
            .clamp(0.0, source_max(0.0, self.bounds().width - width));
        Canvas::set_left(&self.preview, x);
        Canvas::set_top(&self.preview, origin.y - height - 4.0);
        self.preview.set_is_visible(true);
        *self.preview_key.borrow_mut() = Some(key);
        Ok(())
    }
    fn hide_preview(&self, key: Option<&Rc<ComputedKey>>) {
        if key.is_none()
            || key.is_some_and(|key| {
                self.preview_key
                    .borrow()
                    .as_ref()
                    .is_some_and(|shown| Rc::ptr_eq(key, shown))
            })
        {
            self.preview.set_is_visible(false);
            *self.preview_key.borrow_mut() = None;
        }
    }
    fn update_popup(&self) -> KeyboardControlResult<()> {
        let Some(popup) = self.touch_processor.popup() else {
            self.popup.set_is_visible(false);
            self.popup_canvas.children().clear();
            return Ok(());
        };
        self.hide_preview(None);
        let theme = self.effective_theme();
        let frame = popup.frame();
        let origin = self
            .keys_view
            .translate_point(Point::new(frame.x, frame.y), &self.overlay)
            .ok_or(crate::views::view_helpers::ViewError::TransformUnavailable)?;
        self.popup.set_width(frame.width + 2.0);
        self.popup.set_height(frame.height + 2.0);
        Canvas::set_left(&self.popup, origin.x - 1.0);
        Canvas::set_top(&self.popup, origin.y - 1.0);
        self.popup_canvas.children().clear();
        let ctx = self.engine.create_compute_context()?;
        let mut i = 0;
        while i < popup.keys().len() {
            let b = popup
                .bounds()
                .borrow()
                .get(i)
                .copied()
                .ok_or(ferroboard_core::layouts::KeyDataError::Collection(
                    ferroboard_core::layouts::ResourceTextListError::IndexOutOfRange,
                ))
                .map_err(EngineError::Key)?;
            let selected = i as i32 == popup.selected_index();
            let cell = Border::new();
            cell.set_width(b.width);
            cell.set_height(b.height);
            cell.set_corner_radius(CornerRadius::uniform(theme.popup_corner_radius() * 0.7));
            cell.set_background(Some(
                if selected {
                    Brushes::get(theme.popup_selected_background())
                } else {
                    Brushes::transparent()
                }
                .into(),
            ));
            cell.set_is_hit_test_visible(false);
            let fg = Brushes::get(if selected {
                theme.popup_selected_foreground()
            } else {
                theme.popup_foreground()
            });
            let key = popup
                .keys()
                .get(i)
                .map_err(ferroboard_core::layouts::KeyDataError::from)
                .map_err(EngineError::Key)?;
            let (label, icon) =
                KeyLabels::get_display(key.as_ref(), &ctx).map_err(EngineError::Key)?;
            if icon != KeyIcon::None && IconFactory::has_icon(icon) {
                let path =
                    IconFactory::create(icon, source_min(22.0, b.height * 0.5), Some(fg.into()))?;
                path.set_horizontal_alignment(HorizontalAlignment::Center);
                path.set_vertical_alignment(VerticalAlignment::Center);
                cell.set_child(path);
            } else {
                let text = TextBlock::new();
                let label = crate::views::view_helpers::nullable_text(label)?.unwrap_or_default();
                text.set_text(Some(&label));
                text.set_foreground(Some(fg.into()));
                text.set_font_size(source_min(
                    theme.key_font_size() * self.engine.settings().font_scale(),
                    b.height * 0.55,
                ));
                text.set_horizontal_alignment(HorizontalAlignment::Center);
                text.set_vertical_alignment(VerticalAlignment::Center);
                text.set_is_hit_test_visible(false);
                cell.set_child(text);
            }
            Canvas::set_left(&cell, b.x - frame.x);
            Canvas::set_top(&cell, b.y - frame.y);
            self.popup_canvas.children().add(cell);
            i += 1;
        }
        self.popup.set_is_visible(true);
        Ok(())
    }
    fn begin_emoji_search(&self) -> KeyboardControlResult<()> {
        let Some(target) = self.engine.target() else {
            return Ok(());
        };
        *self.search_original_target.borrow_mut() = Some(target);
        let buffer = Rc::new(TextBufferTarget::new(
            Utf16Text::default(),
            Some(Arc::new(InputAttributes::from_values(
                ferroboard_core::editor::InputAttributesValues {
                    kind: InputKind::Search,
                    capitalization: CapitalizationMode::None,
                    allow_suggestions: false,
                    allow_auto_correct: false,
                    is_private: true,
                    ..Default::default()
                },
            ))),
        ));
        *self.search_buffer.borrow_mut() = Some(buffer.clone());
        let weak = self.to_ref().downgrade();
        buffer.set_enter_action_handler(Some(Rc::new(move |_| {
            let Some(owner) = weak.upgrade() else {
                return false;
            };
            let state = owner.search_state.borrow().clone();
            let first = state.and_then(|state| {
                state
                    .results
                    .borrow()
                    .as_ref()
                    .and_then(|results| results.borrow().first().cloned())
                    .flatten()
            });
            if let Some(first) = first {
                owner
                    .insert_search_result(first)
                    .expect("emoji search enter");
            }
            true
        })));
        let weak = self.to_ref().downgrade();
        let state = EmojiSearchState::new(Some(Rc::new(move |emoji| {
            let owner = weak
                .upgrade()
                .ok_or(crate::views::view_helpers::ViewError::OwnerReleased)?;
            owner
                .insert_search_result(
                    emoji.ok_or(crate::views::view_helpers::ViewError::ArgumentNull)?,
                )
                .map_err(crate::views::view_helpers::ViewError::Owner)
        })));
        *self.search_state.borrow_mut() = Some(state.clone());
        let weak = self.to_ref().downgrade();
        buffer.edited().add(Rc::new(move |_| {
            if let Some(owner) = weak.upgrade() {
                owner.update_search_results().expect("emoji search edit");
            }
        }));
        self.engine.attach(buffer)?;
        self.smartbar
            .set_emoji_search(self.search_state.borrow().clone());
        self.update_search_results()
    }
    fn update_search_results(&self) -> KeyboardControlResult<()> {
        let state = self.search_state.borrow().clone();
        let buffer = self.search_buffer.borrow().clone();
        let (Some(state), Some(buffer)) = (state, buffer) else {
            return Ok(());
        };
        let text = buffer.text();
        *state.query.borrow_mut() = Some(text.clone());
        let results = if text.units().iter().all(|&c| TextUtils::is_white_space(c)) {
            self.engine
                .emoji_history()
                .combined()
                .into_iter()
                .take(20)
                .collect()
        } else {
            let entries = self.engine.emoji_catalog()?.search(&text, 40);
            let mut results = Vec::new();
            for entry in entries {
                results.push(Some(
                    entry
                        .with_skin_tone(
                            self.engine
                                .emoji_history()
                                .get_preferred_tone(&entry.value)
                                .unwrap_or(self.engine.settings().emoji_skin_tone()),
                        )
                        .map_err(EngineError::Emoji)?,
                ));
            }
            results
        };
        *state.results.borrow_mut() = Some(Rc::new(RefCell::new(results)));
        self.smartbar.refresh()?;
        Ok(())
    }
    fn insert_search_result(&self, emoji: Utf16Text) -> KeyboardControlResult<()> {
        let target = self.search_original_target.borrow().clone();
        let buffer = self.search_buffer.borrow().clone();
        let (Some(target), Some(_buffer)) = (target, buffer) else {
            return Ok(());
        };
        self.engine.attach(target)?;
        self.engine.input_emoji(&emoji)?;
        let current_buffer = self.search_buffer.borrow().clone();
        if let Some(current_buffer) = current_buffer {
            self.engine.attach(current_buffer)?;
        } else if self.engine.target().is_some() {
            // Source Attach(null) first detaches, then leaves an EditorController with a null
            // target before failing at target.Changed. That nullable editor state is not yet
            // representable by Core; keep the failure explicit after the observable detach.
            self.engine.detach()?;
            return Err(crate::views::view_helpers::ViewError::NullReference.into());
        }
        // Source Attach(null) returns immediately when reentrant callbacks already detached.

        self.update_search_results()
    }
    fn end_emoji_search(&self, return_to_emoji: bool) -> KeyboardControlResult<()> {
        if self.search_state.borrow().is_none() {
            return Ok(());
        }
        let original = self.search_original_target.borrow().clone();
        *self.search_state.borrow_mut() = None;
        *self.search_buffer.borrow_mut() = None;
        *self.search_original_target.borrow_mut() = None;
        self.smartbar.set_emoji_search(None);
        if let Some(original) = original {
            self.engine.attach(original)?;
        }
        if return_to_emoji {
            self.engine.set_ui_mode(KeyboardUiMode::Media)?;
        }
        self.smartbar.refresh()?;
        Ok(())
    }
    pub fn show_resize_overlay(&self) -> KeyboardControlResult<()> {
        let theme = self.effective_theme();
        self.subtype_picker.close();
        self.resize_overlay.children().clear();
        let color = theme.background();
        self.resize_overlay.set_background(Some(
            Brushes::get(Color::from_argb(0xE6, color.r, color.g, color.b)).into(),
        ));
        let fg = Brushes::get(theme.panel_foreground());
        let panel = StackPanel::new();
        panel.set_horizontal_alignment(HorizontalAlignment::Center);
        panel.set_vertical_alignment(VerticalAlignment::Center);
        panel.set_spacing(10.0);
        let icon = IconFactory::create(KeyIcon::ResizeHeight, 32.0, Some(fg.clone().into()))?;
        icon.set_horizontal_alignment(HorizontalAlignment::Center);
        panel.children().add(icon);
        let hint = TextBlock::new();
        let text = crate::views::view_helpers::nullable_text(KeyboardStrings::resize_hint())?;
        hint.set_text(text.as_deref());
        hint.set_foreground(Some(fg.into()));
        hint.set_font_size(15.0);
        hint.set_horizontal_alignment(HorizontalAlignment::Center);
        panel.children().add(hint);
        let buttons = StackPanel::new();
        buttons.set_orientation(Orientation::Horizontal);
        buttons.set_spacing(8.0);
        buttons.set_horizontal_alignment(HorizontalAlignment::Center);
        for done in [false, true] {
            let button = TouchButton::new();
            button.set_padding(Thickness::new(16.0, 8.0, 16.0, 8.0));
            button.set_corner_radius(CornerRadius::uniform(18.0));
            button.set_scheduler(self.scheduler.clone());
            button.set_colors(
                if done {
                    theme.accent_key_background()
                } else {
                    theme.function_key_background()
                },
                if done {
                    theme.accent_key_pressed_background()
                } else {
                    theme.function_key_pressed_background()
                },
            );
            let text = crate::views::view_helpers::nullable_text(if done {
                KeyboardStrings::done()
            } else {
                KeyboardStrings::reset()
            })?;
            button.set_text_nullable(
                text.as_deref(),
                14.0,
                Some(
                    Brushes::get(if done {
                        theme.accent_key_foreground()
                    } else {
                        theme.function_key_foreground()
                    })
                    .into(),
                ),
                None,
            );
            let weak = self.to_ref().downgrade();
            button.click(move |_| {
                let Some(owner) = weak.upgrade() else {
                    return;
                };
                if done {
                    owner.resize_overlay.set_is_visible(false);
                } else {
                    owner.engine.settings().set_height_scale(1.0);
                }
            });
            buttons.children().add(button);
        }
        panel.children().add(buttons);
        self.resize_overlay.children().add(panel);
        FocusNeutral::apply(&self.resize_overlay);
        self.resize_overlay.set_is_visible(true);
        Ok(())
    }
    fn resize_pointer_pressed(&self, e: &PointerEventArgs) {
        self.resize_pointer.set(Some(e.pointer().id()));
        // Native docked layout moves this control while its height changes. Keep
        // the press-time keyboard coordinate space for this captured gesture, so
        // layout feedback cannot turn an upward screen drag into a downward delta.
        // The frozen transform also preserves keyboard-space units under scaling.
        *self.resize_coordinates.borrow_mut() =
            TopLevel::get_top_level(Some(self)).and_then(|root| {
                let root = root.upcast::<Visual>();
                root.transform_to_visual(self)
                    .map(|transform| (root.downgrade(), transform))
            });
        self.resize_start_y.set(self.resize_position(e).y);
        self.resize_start_scale
            .set(self.engine.settings().height_scale());
        e.pointer()
            .capture(Some(&self.resize_overlay.to_ref().upcast::<InputElement>()));
        e.set_handled(true);
    }
    fn resize_pointer_moved(&self, e: &PointerEventArgs) {
        if self.resize_pointer.get() != Some(e.pointer().id()) {
            return;
        }
        let settings = self.engine.settings();
        let rows = if settings.number_row() { 5.0 } else { 4.0 };
        let delta = self.resize_start_y.get() - self.resize_position(e).y;
        let scale = ((self.resize_start_scale.get() + delta / (rows * settings.key_height()))
            * 100.0)
            .round_ties_even()
            / 100.0;
        if (scale - settings.height_scale()).abs() >= 0.02 {
            settings.set_height_scale(scale);
        }
        e.set_handled(true);
    }
    fn resize_pointer_released(&self, e: &PointerEventArgs) {
        self.resize_pointer.set(None);
        self.resize_coordinates.borrow_mut().take();
        if e.pointer().captured().as_ref()
            == Some(&self.resize_overlay.to_ref().upcast::<InputElement>())
        {
            e.pointer().capture(None);
        }
    }
    fn resize_position(&self, e: &PointerEventArgs) -> Point {
        let coordinates = self.resize_coordinates.borrow().clone();
        if let Some((root, transform)) = coordinates
            && let Some(root) = root.upgrade()
        {
            transform.transform(e.get_position(Some(&root)))
        } else {
            e.get_position(Some(self))
        }
    }
    fn update_focus_tracking(&self) -> KeyboardControlResult<()> {
        let should_track = self.auto_attach() && self.loaded.get();
        if should_track == self.focus_tracking.get() {
            return Ok(());
        }
        self.focus_tracking.set(should_track);
        if should_track {
            if let Some(root) = TopLevel::get_top_level(Some(self)) {
                let weak = self.to_ref().downgrade();
                let token = root.add_handler_with(
                    InputElement::got_focus_event(),
                    move |_, e| {
                        if let Some(owner) = weak.upgrade()
                            && let Some(element) = e.new_focused_element.as_ref()
                        {
                            let element = element.clone().upcast::<Visual>();
                            if TopLevel::get_top_level(Some(&element))
                                == TopLevel::get_top_level(Some(&owner))
                                && Self::is_text_input(Some(&element))
                            {
                                owner
                                    .attach_to(Some(element))
                                    .expect("keyboard focused target");
                            }
                        }
                    },
                    RoutingStrategies::DIRECT | RoutingStrategies::BUBBLE,
                    true,
                );
                *self.focus_subscription.borrow_mut() = Some((root.clone(), token));
                if let Some(element) = root.focus_manager().get_focused_element() {
                    let element = element.upcast::<Visual>();
                    if TopLevel::get_top_level(Some(&element)) == Some(root.clone())
                        && Self::is_text_input(Some(&element))
                    {
                        self.attach_to(Some(element))?;
                    }
                }
            }
        } else if let Some((root, token)) = self.focus_subscription.borrow_mut().take() {
            root.remove_handler(InputElement::got_focus_event(), token);
        }
        Ok(())
    }
    /// Releases this view's native subscriptions and owned target adapter. Caller-owned targets
    /// and targets installed by another view sharing the engine remain attached, as with Drop.
    /// This port-only bridge is not an Unloaded policy; Unloaded only stops tracking and touch.
    pub fn release_native_subscriptions(&self) -> KeyboardControlResult<()> {
        let owned = self.owned_target.borrow().clone();
        let search = self.search_buffer.borrow().clone();
        let original = self.search_original_target.borrow().clone();
        self.loaded.set(false);
        self.update_focus_tracking()?;
        self.touch_processor.cancel_all()?;
        self.release_owned_resources();
        self.release_engine_target(owned, search, original)?;
        self.target_changed.raise(self);
        Ok(())
    }
    fn release_engine_target(
        &self,
        owned: Option<OwnedInputAdapter>,
        search: Option<Rc<TextBufferTarget>>,
        original: Option<Rc<dyn ITextInputTarget>>,
    ) -> Result<(), EngineError> {
        // The engine can be shared or synchronously retargeted by callbacks. Only retire this
        // view's adapter/search buffer; external targets remain the caller's responsibility.
        let same_owned = |target: &Rc<dyn ITextInputTarget>| {
            owned.as_ref().is_some_and(|owned| owned.is_target(target))
        };
        let current = self.engine.target();
        let active_search =
            current
                .as_ref()
                .zip(search.as_ref())
                .is_some_and(|(target, search)| {
                    Rc::as_ptr(target) as *const () == Rc::as_ptr(search) as *const ()
                });
        if active_search {
            if let Some(original) = original.filter(|target| !same_owned(target)) {
                self.engine.attach(original)
            } else {
                self.engine.detach()
            }
        } else if current.as_ref().is_some_and(same_owned) {
            self.engine.detach()
        } else {
            Ok(())
        }
    }
    /// Resource teardown only: avoids target-change events or recovering a Ref to a dropping owner.
    fn release_owned_resources(&self) {
        self.resize_pointer.set(None);
        self.resize_coordinates.borrow_mut().take();
        self.loaded.set(false);
        self.focus_tracking.set(false);
        self.clipboard_monitor.borrow_mut().take();
        if let Some((root, token)) = self.focus_subscription.borrow_mut().take() {
            root.remove_handler(InputElement::got_focus_event(), token);
        }
        if let Some(tokens) = self.engine_subscriptions.take() {
            self.engine
                .keyboard_changed()
                .remove(tokens.keyboard_changed);
            self.engine.state_changed().remove(tokens.state_changed);
            self.engine
                .suggestions_changed()
                .remove(tokens.suggestions_changed);
            self.engine.hide_requested().remove(tokens.hide_requested);
            self.engine
                .settings_requested()
                .remove(tokens.settings_requested);
            self.engine
                .voice_input_requested()
                .remove(tokens.voice_input_requested);
            self.engine
                .subtype_picker_requested()
                .remove(tokens.subtype_picker_requested);
            self.engine
                .resize_mode_requested()
                .remove(tokens.resize_mode_requested);
            self.engine
                .feedback_requested()
                .remove(tokens.feedback_requested);
            self.engine
                .settings()
                .property_changed()
                .remove(tokens.settings_changed);
        }
        if let Some(tokens) = self.view_subscriptions.take() {
            self.smartbar.remove_hide_requested(tokens.hide);
            self.smartbar.remove_search_back_requested(tokens.back);
            self.emoji_panel.remove_search_requested(tokens.search);
            self.subtype_picker.remove_manage_requested(tokens.manage);
            self.touch_processor.popup_changed().remove(tokens.popup);
            self.keys_view.remove_key_pressed_visual(tokens.pressed);
            self.keys_view.remove_key_released_visual(tokens.released);
        }
        let subscriptions = std::mem::take(&mut *self.subscriptions.borrow_mut());
        for subscription in subscriptions {
            subscription.dispose();
        }
        let theme_subscription = self.theme_subscription.borrow_mut().take();
        if let Some((id, subscription)) = theme_subscription {
            drop(subscription);
            // During UI-thread teardown the weak TLS table may already have been destroyed.
            // The global hook still has to be removed; there is then no table entry to release.
            let _ = THEME_OWNERS.try_with(|owners| owners.borrow_mut().remove(&id));
        }
        let target = self.owned_target.borrow_mut().take();
        if let Some(target) = target {
            target.detach();
        }
        self.target_element.borrow_mut().take();
        self.search_state.borrow_mut().take();
        self.search_buffer.borrow_mut().take();
        self.search_original_target.borrow_mut().take();
        self.smartbar.set_emoji_search(None);
        self.keys_view.cancel_trail_fade();
        self.keys_view.release_subscriptions();
        self.smartbar.release_visual_children();
        self.emoji_panel.release_subscriptions();
        self.clipboard_panel.release_subscriptions();
        self.editing_panel.release_subscriptions();
        self.subtype_picker.release_visual_children();
        self.side_panel.set_child(None::<Ref<Control>>);
        self.resize_overlay.children().clear();
    }
}
impl Drop for OnScreenKeyboard {
    fn drop(&mut self) {
        let owned = self.owned_target.borrow().clone();
        let search = self.search_buffer.borrow().clone();
        let original = self.search_original_target.borrow().clone();
        self.release_owned_resources();
        if let Err(error) = self.touch_processor.cancel_all() {
            eprintln!("FerroBoard keyboard gesture cleanup failed: {error}");
        }
        if let Err(error) = self.release_engine_target(owned, search, original) {
            // Drop cannot return an error. Explicit release remains fallible for callers that
            // need to handle failures before destruction; failures here remain observable.
            eprintln!("FerroBoard keyboard target cleanup failed: {error}");
        }
    }
}
#[cfg(test)]
#[path = "on_screen_keyboard_tests.rs"]
mod tests;

impl PartialEq for EnterActionRequestedEventArgs {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.handled, &other.handled)
    }
}

#[cfg(test)]
#[path = "native_keyboard_lifetime_tests.rs"]
mod native_lifetime_tests;
