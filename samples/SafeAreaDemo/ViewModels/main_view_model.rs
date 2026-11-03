//! Port of `ViewModels/MainViewModel.cs`.

use ferroui_base::animation::easings::{IEasing, LinearEasing};
use ferroui_base::animation::SignedDuration;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{ferro_markup_type, Rect, Thickness};
use ferroui_controls::platform::{IInputPane, IInsetsManager, InputPaneState, InputPaneStateEventArgs, SafeAreaChangedArgs};
use ferroui_controls::{Application, Control, TopLevel};
use mini_mvvm::ViewModelBase;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The view model of the main view: the options of the insets manager of the top-level of the
/// view and the state of its input pane.
pub struct MainViewModel {
    base: ViewModelBase,
    this: Weak<MainViewModel>,
    use_safe_area: Cell<bool>,
    display_edge_to_edge: Cell<bool>,
    insets_manager: RefCell<Option<Rc<dyn IInsetsManager>>>,
    hide_system_bars: Cell<bool>,
    auto_safe_area_padding: Cell<bool>,
    input_pane: RefCell<Option<Rc<dyn IInputPane>>>,
    input_pane_easing: RefCell<Option<Rc<dyn IEasing>>>,
    input_pane_duration: Cell<Option<SignedDuration>>,
    canvas_size: Cell<Rect>,
    /// The handler added to `SafeAreaChanged` of the insets manager (`+=`), removed by the
    /// next [`initialize`](Self::initialize) (`-=`).
    safe_area_changed: RefCell<Option<Rc<dyn IDisposable>>>,
    /// The handler added to `StateChanged` of the input pane, as above.
    state_changed: RefCell<Option<Rc<dyn IDisposable>>>,
}

impl PartialEq for MainViewModel {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl INotifyPropertyChanged for MainViewModel {
    fn property_changed(&self) -> &Event<str> {
        self.base.property_changed()
    }
}

impl MainViewModel {
    pub fn new() -> Rc<MainViewModel> {
        Rc::new_cyclic(|this| Self {
            base: ViewModelBase::new(),
            this: this.clone(),
            use_safe_area: Cell::new(true),
            display_edge_to_edge: Cell::new(false),
            insets_manager: RefCell::new(None),
            hide_system_bars: Cell::new(false),
            auto_safe_area_padding: Cell::new(false),
            input_pane: RefCell::new(None),
            input_pane_easing: RefCell::new(None),
            input_pane_duration: Cell::new(None),
            canvas_size: Cell::new(Rect::default()),
            safe_area_changed: RefCell::new(None),
            state_changed: RefCell::new(None),
        })
    }

    pub fn input_pane_state(&self) -> InputPaneState {
        self.input_pane.borrow().as_ref().map_or(InputPaneState::Closed, |input_pane| input_pane.state())
    }

    pub fn input_pane_easing(&self) -> Option<Rc<dyn IEasing>> {
        self.input_pane_easing.borrow().clone()
    }

    pub fn input_pane_duration(&self) -> Option<SignedDuration> {
        self.input_pane_duration.get()
    }

    pub fn input_pane_marker_margin(&self) -> Thickness {
        if self.input_pane_state() == InputPaneState::Open {
            Thickness::new(0.0, 0.0, 0.0, f64::max(0.0, self.canvas_size().height - self.input_pane_rect().top()))
        } else {
            Thickness::default()
        }
    }

    pub fn input_pane_rect(&self) -> Rect {
        self.input_pane.borrow().as_ref().map_or_else(Rect::default, |input_pane| input_pane.occluded_rect())
    }

    pub fn canvas_size(&self) -> Rect {
        self.canvas_size.get()
    }

    pub fn set_canvas_size(&self, value: Rect) {
        self.canvas_size.set(value);
    }

    pub fn safe_area_padding(&self) -> Thickness {
        if !self.auto_safe_area_padding.get() {
            self.insets_manager.borrow().as_ref().map_or_else(Thickness::default, |insets| insets.safe_area_padding())
        } else {
            Thickness::default()
        }
    }

    pub fn view_padding(&self) -> Thickness {
        if self.use_safe_area.get() {
            self.safe_area_padding()
        } else {
            Thickness::default()
        }
    }

    pub fn use_safe_area(&self) -> bool {
        self.use_safe_area.get()
    }

    pub fn set_use_safe_area(&self, value: bool) {
        self.use_safe_area.set(value);

        self.base.raise_property_changed("UseSafeArea");

        self.raise_safe_area_changed();
    }

    pub fn display_edge_to_edge(&self) -> bool {
        self.display_edge_to_edge.get()
    }

    pub fn set_display_edge_to_edge(&self, value: bool) {
        self.display_edge_to_edge.set(value);

        let insets_manager = self.insets_manager.borrow().clone();
        if let Some(insets_manager) = insets_manager {
            insets_manager.set_display_edge_to_edge_preference(value);
        }

        self.base.raise_property_changed("DisplayEdgeToEdge");

        self.raise_safe_area_changed();
    }

    pub fn hide_system_bars(&self) -> bool {
        self.hide_system_bars.get()
    }

    pub fn set_hide_system_bars(&self, value: bool) {
        self.hide_system_bars.set(value);

        let insets_manager = self.insets_manager.borrow().clone();
        if let Some(insets_manager) = insets_manager {
            insets_manager.set_is_system_bar_visible(Some(!value));
        }

        self.base.raise_property_changed("HideSystemBars");

        self.raise_safe_area_changed();
    }

    pub fn auto_safe_area_padding(&self) -> bool {
        self.auto_safe_area_padding.get()
    }

    pub fn set_auto_safe_area_padding(&self, value: bool) {
        self.auto_safe_area_padding.set(value);

        self.base.raise_property_changed("AutoSafeAreaPadding");
        self.raise_safe_area_changed();
    }

    pub(crate) fn initialize(
        &self,
        main_view: &Control,
        insets_manager: Option<Rc<dyn IInsetsManager>>,
        input_pane: Option<Rc<dyn IInputPane>>,
    ) {
        // `_insetsManager.SafeAreaChanged -= ...` and `_inputPane.StateChanged -= ...`.
        let safe_area_changed = self.safe_area_changed.borrow_mut().take();
        if let Some(safe_area_changed) = safe_area_changed {
            safe_area_changed.dispose();
        }
        let state_changed = self.state_changed.borrow_mut().take();
        if let Some(state_changed) = state_changed {
            state_changed.dispose();
        }

        self.auto_safe_area_padding.set(TopLevel::get_auto_safe_area_padding(main_view));
        *self.insets_manager.borrow_mut() = insets_manager.clone();

        if let Some(insets_manager) = insets_manager {
            // The handler holds the view model weakly: the insets manager outlives it.
            let this = self.this.clone();
            *self.safe_area_changed.borrow_mut() = Some(insets_manager.safe_area_changed(Rc::new(move |e| {
                if let Some(this) = this.upgrade() {
                    this.insets_manager_safe_area_changed(e);
                }
            })));

            self.display_edge_to_edge.set(insets_manager.display_edge_to_edge_preference());
            self.hide_system_bars.set(!insets_manager.is_system_bar_visible().unwrap_or(false));
        }

        *self.input_pane.borrow_mut() = input_pane.clone();
        if let Some(input_pane) = input_pane {
            let this = self.this.clone();
            *self.state_changed.borrow_mut() = Some(input_pane.state_changed(Rc::new(move |e| {
                if let Some(this) = this.upgrade() {
                    this.input_pane_on_state_changed(e);
                }
            })));
        }
        self.raise_keyboard_changed();
    }

    fn input_pane_on_state_changed(&self, e: &InputPaneStateEventArgs) {
        self.input_pane_duration.set(Some(SignedDuration::from(e.animation_duration())));
        *self.input_pane_easing.borrow_mut() = Some(e.easing().cloned().unwrap_or_else(|| Rc::new(LinearEasing)));
        self.raise_keyboard_changed();
    }

    fn insets_manager_safe_area_changed(&self, _e: &SafeAreaChangedArgs) {
        self.raise_safe_area_changed();
    }

    fn raise_safe_area_changed(&self) {
        self.base.raise_property_changed("SafeAreaPadding");
        self.base.raise_property_changed("ViewPadding");
        self.base.raise_property_changed("InputPaneMarkerMargin");
    }

    fn raise_keyboard_changed(&self) {
        self.base.raise_property_changed("InputPaneState");
        self.base.raise_property_changed("InputPaneRect");
        self.base.raise_property_changed("InputPaneEasing");
        self.base.raise_property_changed("InputPaneDuration");
        self.base.raise_property_changed("InputPaneMarkerMargin");
    }

    pub fn set_light(&self) {
        if let Some(application) = Application::current() {
            application.set_requested_theme_variant(Some(ThemeVariant::light()));
        }
    }

    pub fn set_dark(&self) {
        if let Some(application) = Application::current() {
            application.set_requested_theme_variant(Some(ThemeVariant::dark()));
        }
    }

    pub fn set_default(&self) {
        if let Some(application) = Application::current() {
            application.set_requested_theme_variant(Some(ThemeVariant::default()));
        }
    }
}

ferro_markup_type!(class MainViewModel {
    this: Rc<MainViewModel>,
    handles: [MainViewModel, Rc<MainViewModel>, Option<Rc<MainViewModel>>],
    constructors: [() => MainViewModel::new],
    properties: [
        InputPaneState: InputPaneState { get: |this: &Rc<MainViewModel>| this.input_pane_state() },
        InputPaneEasing: Option<Rc<dyn IEasing>> { get: |this: &Rc<MainViewModel>| this.input_pane_easing() },
        InputPaneDuration: Option<SignedDuration> { get: |this: &Rc<MainViewModel>| this.input_pane_duration() },
        InputPaneMarkerMargin: Thickness { get: |this: &Rc<MainViewModel>| this.input_pane_marker_margin() },
        InputPaneRect: Rect { get: |this: &Rc<MainViewModel>| this.input_pane_rect() },
        CanvasSize: Rect {
            get: |this: &Rc<MainViewModel>| this.canvas_size(),
            set: |this: &Rc<MainViewModel>, value: Rect| this.set_canvas_size(value)
        },
        SafeAreaPadding: Thickness { get: |this: &Rc<MainViewModel>| this.safe_area_padding() },
        ViewPadding: Thickness { get: |this: &Rc<MainViewModel>| this.view_padding() },
        UseSafeArea: bool {
            get: |this: &Rc<MainViewModel>| this.use_safe_area(),
            set: |this: &Rc<MainViewModel>, value: bool| this.set_use_safe_area(value)
        },
        DisplayEdgeToEdge: bool {
            get: |this: &Rc<MainViewModel>| this.display_edge_to_edge(),
            set: |this: &Rc<MainViewModel>, value: bool| this.set_display_edge_to_edge(value)
        },
        HideSystemBars: bool {
            get: |this: &Rc<MainViewModel>| this.hide_system_bars(),
            set: |this: &Rc<MainViewModel>, value: bool| this.set_hide_system_bars(value)
        },
        AutoSafeAreaPadding: bool {
            get: |this: &Rc<MainViewModel>| this.auto_safe_area_padding(),
            set: |this: &Rc<MainViewModel>, value: bool| this.set_auto_safe_area_padding(value)
        },
    ],
    methods: [
        fn SetLight() => |this: &Rc<MainViewModel>| this.set_light(),
        fn SetDark() => |this: &Rc<MainViewModel>| this.set_dark(),
        fn SetDefault() => |this: &Rc<MainViewModel>| this.set_default(),
    ],
    notify_property_changed: MainViewModel,
});
