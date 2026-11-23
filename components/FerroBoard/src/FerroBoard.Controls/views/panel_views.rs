//! Derived from ScreenKeyboard.Uno/Views/PanelViews.cs (Apache-2.0).
use super::{
    FontWeightsEx, KeyStyles, TextKeyDataFactory,
    padded_grid::PaddedGrid,
    view_helpers::{ViewError, format_one, native_text, nullable_text},
};
use crate::{
    KeyboardStrings,
    icons::IconFactory,
    primitives::{Brushes, FocusNeutral, TouchButton, UniformWrapPanel},
    themes::{KeyboardTheme, KeyboardThemes},
};
use ferroboard_core::{
    clipboard::ClipboardItem,
    engine::{EngineError, KeyboardEngine},
    input::IKeyboardScheduler,
    keys::{KeyCode, KeyIcon, KeyType, KeyboardUiMode},
    layouts::{LayoutType, TextKeyData},
    text::Utf16Text,
};
use ferroui_base::{
    CornerRadius, FerroObjectImpl, FerroObjectImplExt, Ref, StyledElementImpl, Thickness,
    VisualImpl, ferro_class, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::{HorizontalAlignment, LayoutableImpl, Orientation, VerticalAlignment},
    media::{Color, FontWeight, IBrush, TextAlignment, TextTrimming, TextWrapping},
};
use ferroui_controls::{
    Border, ColumnDefinition, Control, ControlImpl, Grid, GridLength, GridUnitType, PanelImpl,
    RowDefinition, ScrollViewer, StackPanel, TextBlock, automation::AutomationProperties,
    primitives::ScrollBarVisibility,
};
use ferroui_system::events::HandlerList;
use std::time::Duration;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

pub(crate) struct PanelUi;
impl PanelUi {
    pub fn button(
        scheduler: Rc<dyn IKeyboardScheduler>,
        theme: &KeyboardTheme,
        function: bool,
    ) -> Ref<TouchButton> {
        let b = TouchButton::new();
        b.set_corner_radius(CornerRadius::uniform(theme.key_corner_radius()));
        b.set_margin(Thickness::uniform(3.0));
        b.set_scheduler(scheduler);
        b.set_colors(
            if function {
                theme.function_key_background()
            } else {
                theme.key_background()
            },
            if function {
                theme.function_key_pressed_background()
            } else {
                theme.key_pressed_background()
            },
        );
        b
    }
    pub fn title(
        text: Option<Utf16Text>,
        theme: &KeyboardTheme,
    ) -> Result<Ref<TextBlock>, ViewError> {
        let title = TextBlock::new();
        title.set_text(nullable_text(text)?.as_deref());
        title.set_foreground(Some(Brushes::get(theme.panel_foreground()).into()));
        title.set_font_size(theme.smartbar_font_size());
        title.set_font_weight(FontWeightsEx::semi_bold());
        title.set_vertical_alignment(VerticalAlignment::Center);
        title.set_margin(Thickness::new(8.0, 0.0, 8.0, 0.0));
        Ok(title)
    }
}
fn set_text(
    button: &TouchButton,
    text: Option<Utf16Text>,
    size: f64,
    fg: Rc<dyn IBrush>,
) -> Result<(), ViewError> {
    button.set_text_nullable(nullable_text(text)?.as_deref(), size, Some(fg), None);
    Ok(())
}
fn set_icon(
    button: &TouchButton,
    icon: KeyIcon,
    size: f64,
    fg: Rc<dyn IBrush>,
    name: Option<Utf16Text>,
) -> Result<(), ViewError> {
    button.set_icon(icon, size, Some(fg), nullable_text(name)?.as_deref())?;
    Ok(())
}

#[repr(C)]
pub(crate) struct ClipboardPanelView {
    base: Grid,
    engine: Rc<KeyboardEngine>,
    scheduler: Rc<dyn IKeyboardScheduler>,
    header: Ref<Grid>,
    scroller: Ref<ScrollViewer>,
    theme: RefCell<Arc<KeyboardTheme>>,
    header_height: Cell<f64>,
    history_subscription: Cell<Option<u64>>,
}
ferro_class!(ClipboardPanelView:Grid);
ferro_impl_classes!(ClipboardPanelView:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
impl FerroObjectImpl for ClipboardPanelView {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_is_tab_stop(false);
        this.row_definitions()
            .add(RowDefinition::with_height(GridLength::AUTO));
        this.row_definitions()
            .add(RowDefinition::with_height(GridLength::STAR));
        this.scroller
            .set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        this.scroller
            .set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        this.scroller.set_is_tab_stop(false);
        Grid::set_row(&this.scroller, 1);
        this.children().add(this.header.clone());
        this.children().add(this.scroller.clone());
        let weak_view = this.to_ref().downgrade();
        let token = this
            .engine
            .clipboard_history()
            .changed()
            .add(Rc::new(move |_| {
                let Some(view) = weak_view.upgrade() else {
                    return;
                };
                if view.is_visible() {
                    view.refresh().expect("clipboard panel history change");
                }
            }));
        this.history_subscription.set(Some(token));
    }
}
impl ClipboardPanelView {
    pub fn new(engine: Rc<KeyboardEngine>, scheduler: Rc<dyn IKeyboardScheduler>) -> Ref<Self> {
        instantiate(Self {
            base: Grid::construct(),
            engine,
            scheduler,
            header: Grid::new(),
            scroller: ScrollViewer::new(),
            theme: RefCell::new(KeyboardThemes::floris_day()),
            header_height: Cell::new(44.0),
            history_subscription: Cell::new(None),
        })
    }
    pub fn header_height(&self) -> f64 {
        self.header_height.get()
    }
    pub fn set_header_height(&self, v: f64) {
        self.header_height.set(v);
    }
    pub fn set_theme(&self, theme: Arc<KeyboardTheme>) -> Result<(), ViewError> {
        *self.theme.borrow_mut() = theme.clone();
        self.set_background(Some(Brushes::get(theme.panel_background()).into()));
        self.refresh()
    }
    pub fn refresh(&self) -> Result<(), ViewError> {
        self.build_header()?;
        self.build_items()?;
        FocusNeutral::apply(self);
        Ok(())
    }
    /// Explicit native cleanup; Drop also removes owned hooks and private visual resources.
    pub fn release_subscriptions(&self) {
        if let Some(token) = self.history_subscription.take() {
            self.engine.clipboard_history().changed().remove(token);
        }
        self.header.children().clear();
        self.scroller.set_content(None);
    }
    fn theme(&self) -> Arc<KeyboardTheme> {
        self.theme.borrow().clone()
    }
    fn build_header(&self) -> Result<(), ViewError> {
        self.header.children().clear();
        self.header.column_definitions().clear();
        self.header.set_height(self.header_height());
        for length in [
            GridLength::AUTO,
            GridLength::STAR,
            GridLength::AUTO,
            GridLength::AUTO,
        ] {
            self.header
                .column_definitions()
                .add(ColumnDefinition::with_width(length));
        }
        let theme = self.theme();
        let abc = PanelUi::button(self.scheduler.clone(), &theme, true);
        abc.set_width(self.header_height() * 1.4);
        set_text(
            &abc,
            KeyboardStrings::abc(),
            theme.function_key_font_size() * 0.9,
            Brushes::get(theme.function_key_foreground()).into(),
        )?;
        let weak_view = self.to_ref().downgrade();
        abc.click(move |_| {
            let Some(view) = weak_view.upgrade() else {
                return;
            };
            view.engine
                .set_ui_mode(KeyboardUiMode::Text)
                .expect("clipboard ABC")
        });
        self.header.children().add(abc);
        let title = PanelUi::title(KeyboardStrings::clipboard(), &self.theme())?;
        Grid::set_column(&title, 1);
        self.header.children().add(title);
        let theme = self.theme();
        let fg = Brushes::get(theme.panel_foreground());
        let toggle = TouchButton::new();
        toggle.set_width(self.header_height());
        toggle.set_corner_radius(CornerRadius::uniform(self.header_height() / 2.0));
        toggle.set_scheduler(self.scheduler.clone());
        toggle.set_colors(
            theme.action_button_background(),
            theme.action_button_pressed_background(),
        );
        let enabled = self.engine.settings().clipboard_history_enabled();
        set_icon(
            &toggle,
            if enabled {
                KeyIcon::Clipboard
            } else {
                KeyIcon::Incognito
            },
            20.0,
            if enabled {
                Brushes::get(theme.panel_selected_foreground()).into()
            } else {
                fg.clone().into()
            },
            if enabled {
                KeyboardStrings::disable_clipboard_history()
            } else {
                KeyboardStrings::enable_clipboard_history()
            },
        )?;
        let weak_view = self.to_ref().downgrade();
        toggle.click(move |_| {
            let Some(view) = weak_view.upgrade() else {
                return;
            };
            let s = view.engine.settings();
            s.set_clipboard_history_enabled(!s.clipboard_history_enabled());
            view.refresh().expect("clipboard toggle refresh");
        });
        Grid::set_column(&toggle, 2);
        self.header.children().add(toggle);
        let theme = self.theme();
        let clear = TouchButton::new();
        clear.set_width(self.header_height());
        clear.set_corner_radius(CornerRadius::uniform(self.header_height() / 2.0));
        clear.set_scheduler(self.scheduler.clone());
        clear.set_colors(
            theme.action_button_background(),
            theme.action_button_pressed_background(),
        );
        set_icon(
            &clear,
            KeyIcon::Delete,
            20.0,
            fg.into(),
            KeyboardStrings::clear_clipboard_history(),
        )?;
        let weak_view = self.to_ref().downgrade();
        clear.click(move |_| {
            let Some(view) = weak_view.upgrade() else {
                return;
            };
            view.engine.clipboard_history().clear(false);
        });
        Grid::set_column(&clear, 3);
        self.header.children().add(clear);
        Ok(())
    }
    fn build_items(&self) -> Result<(), ViewError> {
        let items = self
            .engine
            .clipboard_history()
            .items()
            .map_err(EngineError::Clipboard)?;
        if items.is_empty() {
            let text = TextBlock::new();
            text.set_text(
                nullable_text(if self.engine.settings().clipboard_history_enabled() {
                    KeyboardStrings::clipboard_empty()
                } else {
                    KeyboardStrings::clipboard_disabled()
                })?
                .as_deref(),
            );
            text.set_foreground(Some(
                Brushes::get(self.theme().key_hint_foreground()).into(),
            ));
            text.set_text_wrapping(TextWrapping::Wrap);
            text.set_text_alignment(TextAlignment::Center);
            text.set_horizontal_alignment(HorizontalAlignment::Center);
            text.set_margin(Thickness::new(16.0, 32.0, 16.0, 0.0));
            self.scroller.set_content(Some(Control::boxed(text)));
            return Ok(());
        }
        let panel = UniformWrapPanel::new();
        panel.set_item_width(220.0);
        panel.set_item_height(84.0);
        panel.set_margin(Thickness::uniform(4.0));
        for item in items {
            panel.children().add(self.create_item(item)?);
        }
        self.scroller.set_content(Some(Control::boxed(panel)));
        Ok(())
    }
    fn create_item(&self, item: Rc<ClipboardItem>) -> Result<Ref<TouchButton>, ViewError> {
        let theme = self.theme();
        let card = TouchButton::new();
        card.set_margin(Thickness::uniform(4.0));
        card.set_corner_radius(CornerRadius::uniform(12.0));
        card.set_scheduler(self.scheduler.clone());
        card.set_colors(
            theme.panel_item_background(),
            KeyStyles::darken(theme.panel_item_background(), 0.92),
        );
        let grid = PaddedGrid::new(Thickness::new(10.0, 6.0, 4.0, 4.0));
        let content = grid.content_grid();
        content
            .column_definitions()
            .add(ColumnDefinition::with_width(GridLength::STAR));
        content
            .column_definitions()
            .add(ColumnDefinition::with_width(GridLength::AUTO));
        let text = TextBlock::new();
        text.set_text(Some(&native_text(&item.text)?));
        text.set_foreground(Some(Brushes::get(theme.key_foreground()).into()));
        text.set_text_wrapping(TextWrapping::Wrap);
        text.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
        text.set_max_lines(3);
        text.set_font_size(13.0);
        text.set_vertical_alignment(VerticalAlignment::Top);
        text.set_is_hit_test_visible(false);
        content.children().add(text);
        let actions = StackPanel::new();
        actions.set_orientation(Orientation::Vertical);
        actions.set_vertical_alignment(VerticalAlignment::Center);
        let pinned = item.is_pinned();
        let pin = self.small_action(
            if pinned { KeyIcon::Pin } else { KeyIcon::Unpin },
            if pinned {
                KeyboardStrings::unpin()
            } else {
                KeyboardStrings::pin()
            },
            if pinned {
                theme.panel_selected_foreground()
            } else {
                theme.key_hint_foreground()
            },
        )?;
        let weak_view = self.to_ref().downgrade();
        let i = item.clone();
        pin.click(move |_| {
            let Some(view) = weak_view.upgrade() else {
                return;
            };
            view.engine
                .clipboard_history()
                .set_pinned(&i, !i.is_pinned())
        });
        let delete = self.small_action(
            KeyIcon::Close,
            KeyboardStrings::delete(),
            self.theme().key_hint_foreground(),
        )?;
        let weak_view = self.to_ref().downgrade();
        let i = item.clone();
        delete.click(move |_| {
            let Some(view) = weak_view.upgrade() else {
                return;
            };
            view.engine.clipboard_history().remove(&i);
        });
        actions.children().add(pin);
        actions.children().add(delete);
        Grid::set_column(&actions, 1);
        content.children().add(actions);
        card.set_content(grid.upcast());
        let name = format_one(KeyboardStrings::paste_format().as_ref(), &item.text)?;
        AutomationProperties::set_name(&card, Some(&native_text(&name)?));
        let weak_view = self.to_ref().downgrade();
        card.click(move |_| {
            let Some(view) = weak_view.upgrade() else {
                return;
            };
            view.engine
                .input_text(&item.text)
                .expect("clipboard history paste")
        });
        Ok(card)
    }
    fn small_action(
        &self,
        icon: KeyIcon,
        name: Option<Utf16Text>,
        color: Color,
    ) -> Result<Ref<TouchButton>, ViewError> {
        let button = TouchButton::new();
        button.set_width(32.0);
        button.set_height(32.0);
        button.set_corner_radius(CornerRadius::uniform(16.0));
        button.set_scheduler(self.scheduler.clone());
        let theme = self.theme();
        button.set_colors(
            theme.action_button_background(),
            theme.action_button_pressed_background(),
        );
        set_icon(&button, icon, 16.0, Brushes::get(color).into(), name)?;
        Ok(button)
    }
}

#[repr(C)]
pub(crate) struct EditingPanelView {
    base: Grid,
    engine: Rc<KeyboardEngine>,
    scheduler: Rc<dyn IKeyboardScheduler>,
    theme: RefCell<Arc<KeyboardTheme>>,
    select_button: RefCell<Option<Ref<TouchButton>>>,
    body: Ref<PaddedGrid>,
    state_subscription: Cell<Option<u64>>,
}
ferro_class!(EditingPanelView:Grid);
ferro_impl_classes!(EditingPanelView:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
impl FerroObjectImpl for EditingPanelView {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_is_tab_stop(false);
        this.children().add(this.body.clone());
        let weak_view = this.to_ref().downgrade();
        let token = this.engine.state_changed().add(Rc::new(move |_| {
            let Some(view) = weak_view.upgrade() else {
                return;
            };
            view.update_select_state().expect("editing selection state")
        }));
        this.state_subscription.set(Some(token));
    }
}
impl EditingPanelView {
    pub fn new(engine: Rc<KeyboardEngine>, scheduler: Rc<dyn IKeyboardScheduler>) -> Ref<Self> {
        instantiate(Self {
            base: Grid::construct(),
            engine,
            scheduler,
            theme: RefCell::new(KeyboardThemes::floris_day()),
            select_button: RefCell::new(None),
            body: PaddedGrid::new(Thickness::uniform(4.0)),
            state_subscription: Cell::new(None),
        })
    }
    pub fn content_grid(&self) -> Ref<Grid> {
        self.body.content_grid()
    }
    pub fn padding(&self) -> Thickness {
        self.body.padding()
    }
    pub fn set_theme(&self, theme: Arc<KeyboardTheme>) -> Result<(), ViewError> {
        *self.theme.borrow_mut() = theme.clone();
        self.set_background(Some(Brushes::get(theme.panel_background()).into()));
        self.build()
    }
    pub fn release_subscriptions(&self) {
        if let Some(token) = self.state_subscription.take() {
            self.engine.state_changed().remove(token);
        }
        self.content_grid().children().clear();
        self.select_button.borrow_mut().take();
    }
    fn build(&self) -> Result<(), ViewError> {
        let grid = self.content_grid();
        grid.children().clear();
        grid.row_definitions().clear();
        grid.column_definitions().clear();
        for _ in 0..4 {
            grid.row_definitions()
                .add(RowDefinition::with_height(GridLength::STAR));
        }
        for _ in 0..5 {
            grid.column_definitions()
                .add(ColumnDefinition::with_width(GridLength::STAR));
        }
        self.add(
            KeyIcon::Home,
            KeyboardStrings::line_start(),
            KeyCode::MoveStartOfLine,
            0,
            0,
            false,
        )?;
        self.add(
            KeyIcon::ArrowUp,
            KeyboardStrings::up(),
            KeyCode::ArrowUp,
            0,
            1,
            true,
        )?;
        self.add(
            KeyIcon::End,
            KeyboardStrings::line_end(),
            KeyCode::MoveEndOfLine,
            0,
            2,
            false,
        )?;
        self.add(
            KeyIcon::ArrowLeft,
            KeyboardStrings::left(),
            KeyCode::ArrowLeft,
            1,
            0,
            true,
        )?;
        let select = self.add(
            KeyIcon::Select,
            KeyboardStrings::select(),
            KeyCode::ClipboardSelect,
            1,
            1,
            false,
        )?;
        *self.select_button.borrow_mut() = Some(select);
        self.add(
            KeyIcon::ArrowRight,
            KeyboardStrings::right(),
            KeyCode::ArrowRight,
            1,
            2,
            true,
        )?;
        self.add(
            KeyIcon::DocumentStart,
            KeyboardStrings::start_of_text(),
            KeyCode::MoveStartOfPage,
            2,
            0,
            false,
        )?;
        self.add(
            KeyIcon::ArrowDown,
            KeyboardStrings::down(),
            KeyCode::ArrowDown,
            2,
            1,
            true,
        )?;
        self.add(
            KeyIcon::DocumentEnd,
            KeyboardStrings::end_of_text(),
            KeyCode::MoveEndOfPage,
            2,
            2,
            false,
        )?;
        self.add(
            KeyIcon::SelectAll,
            KeyboardStrings::select_all(),
            KeyCode::ClipboardSelectAll,
            0,
            3,
            false,
        )?;
        self.add(
            KeyIcon::Copy,
            KeyboardStrings::copy(),
            KeyCode::ClipboardCopy,
            1,
            3,
            false,
        )?;
        self.add(
            KeyIcon::Cut,
            KeyboardStrings::cut(),
            KeyCode::ClipboardCut,
            2,
            3,
            false,
        )?;
        self.add(
            KeyIcon::Paste,
            KeyboardStrings::paste(),
            KeyCode::ClipboardPaste,
            3,
            3,
            false,
        )?;
        self.add(
            KeyIcon::Undo,
            KeyboardStrings::undo(),
            KeyCode::Undo,
            0,
            4,
            false,
        )?;
        self.add(
            KeyIcon::Redo,
            KeyboardStrings::redo(),
            KeyCode::Redo,
            1,
            4,
            false,
        )?;
        self.add(
            KeyIcon::Backspace,
            KeyboardStrings::backspace(),
            KeyCode::Delete,
            2,
            4,
            true,
        )?;
        self.add(
            KeyIcon::Enter,
            KeyboardStrings::enter(),
            KeyCode::Enter,
            3,
            4,
            false,
        )?;
        let theme = self.theme.borrow().clone();
        let abc = PanelUi::button(self.scheduler.clone(), &theme, true);
        set_text(
            &abc,
            KeyboardStrings::abc(),
            theme.function_key_font_size(),
            Brushes::get(theme.function_key_foreground()).into(),
        )?;
        let weak_view = self.to_ref().downgrade();
        abc.click(move |_| {
            let Some(view) = weak_view.upgrade() else {
                return;
            };
            view.engine
                .set_ui_mode(KeyboardUiMode::Text)
                .expect("editing ABC")
        });
        Grid::set_row(&abc, 3);
        Grid::set_column_span(&abc, 3);
        grid.children().add(abc);
        self.update_select_state()?;
        FocusNeutral::apply(self);
        Ok(())
    }
    fn add(
        &self,
        icon: KeyIcon,
        name: Option<Utf16Text>,
        code: i32,
        row: i32,
        column: i32,
        repeat: bool,
    ) -> Result<Ref<TouchButton>, ViewError> {
        let theme = self.theme.borrow().clone();
        let button = PanelUi::button(
            self.scheduler.clone(),
            &theme,
            !matches!(
                code,
                KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::ArrowLeft | KeyCode::ArrowRight
            ),
        );
        set_icon(
            &button,
            icon,
            24.0,
            Brushes::get(theme.function_key_foreground()).into(),
            name,
        )?;
        if repeat {
            button.set_repeat_delay(Some(Duration::from_millis(
                (self.engine.settings().key_repeat_delay()) as u64,
            )));
            button.set_repeat_interval(Duration::from_millis(
                (self.engine.settings().key_repeat_interval().max(40)) as u64,
            ));
        }
        let data = if code == KeyCode::Enter {
            TextKeyData::function(KeyCode::Enter, KeyType::EnterEditing, None)
        } else {
            TextKeyDataFactory::function(code)
        };
        let weak_view = self.to_ref().downgrade();
        button.click(move |_| {
            let Some(view) = weak_view.upgrade() else {
                return;
            };
            view.engine.input_key(&*data).expect("editing action");
            view.update_select_state()
                .expect("editing action selection state");
        });
        Grid::set_row(&button, row);
        Grid::set_column(&button, column);
        self.content_grid().children().add(button.clone());
        Ok(button)
    }
    fn update_select_state(&self) -> Result<(), ViewError> {
        let button = self.select_button.borrow().clone();
        let Some(button) = button else {
            return Ok(());
        };
        let active = self.engine.editor().is_some_and(|e| e.is_selecting());
        let theme = self.theme.borrow().clone();
        set_icon(
            &button,
            KeyIcon::Select,
            22.0,
            Brushes::get(if active {
                theme.panel_selected_foreground()
            } else {
                theme.function_key_foreground()
            })
            .into(),
            KeyboardStrings::select(),
        )?;
        button.set_border_brush(Some(
            Brushes::get(self.theme.borrow().panel_selected_foreground()).into(),
        ));
        button.set_border_thickness(Thickness::uniform(if active { 2.0 } else { 0.0 }));
        Ok(())
    }
}

#[repr(C)]
pub(crate) struct SubtypePickerView {
    base: Grid,
    engine: Rc<KeyboardEngine>,
    scheduler: Rc<dyn IKeyboardScheduler>,
    theme: RefCell<Arc<KeyboardTheme>>,
    manage_requested: HandlerList<dyn Fn(&SubtypePickerView)>,
}
ferro_class!(SubtypePickerView:Grid);
ferro_impl_classes!(SubtypePickerView:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
impl FerroObjectImpl for SubtypePickerView {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_is_tab_stop(false);
        this.set_background(Some(Brushes::get(Color::from_argb(0x66, 0, 0, 0)).into()));
        let weak = this.to_ref().downgrade();
        this.pointer_pressed(move |_, e| {
            e.set_handled(true);
            if let Some(view) = weak.upgrade() {
                view.close();
            }
        });
    }
}
impl SubtypePickerView {
    pub fn new(engine: Rc<KeyboardEngine>, scheduler: Rc<dyn IKeyboardScheduler>) -> Ref<Self> {
        instantiate(Self {
            base: Grid::construct(),
            engine,
            scheduler,
            theme: RefCell::new(KeyboardThemes::floris_day()),
            manage_requested: HandlerList::new(),
        })
    }
    pub fn set_theme(&self, theme: Arc<KeyboardTheme>) {
        *self.theme.borrow_mut() = theme;
    }
    pub fn manage_requested(&self, h: impl Fn(&SubtypePickerView) + 'static) -> u64 {
        self.manage_requested.add(Rc::new(h))
    }
    /// Removes this publisher-owned callback; dropping a token alone keeps it registered.
    pub fn remove_manage_requested(&self, token: u64) -> bool {
        crate::callback_tokens::remove(&self.manage_requested, token)
    }
    pub fn open(&self) -> Result<(), ViewError> {
        self.children().clear();
        let list = StackPanel::new();
        list.set_spacing(2.0);
        let subtypes = self.engine.subtypes().borrow().clone();
        for subtype in subtypes {
            let active = subtype.id() == self.engine.active_subtype().id();
            let button = TouchButton::new();
            button.set_height(44.0);
            button.set_corner_radius(CornerRadius::uniform(8.0));
            button.set_padding(Thickness::new(12.0, 0.0, 12.0, 0.0));
            button.set_scheduler(self.scheduler.clone());
            let theme = self.theme.borrow().clone();
            button.set_colors(
                theme.action_button_background(),
                theme.action_button_pressed_background(),
            );
            let layout = self
                .engine
                .resources()
                .get_layout(
                    LayoutType::Characters,
                    &subtype
                        .layouts()
                        .ok_or(EngineError::NullReference)?
                        .characters(),
                )
                .map_err(EngineError::Resource)?
                .and_then(|l| l.label());
            let layout = match layout {
                Some(value) => Some(value),
                None => {
                    subtype
                        .layouts()
                        .ok_or(EngineError::NullReference)?
                        .characters()
                        .component_id
                }
            };
            let mut label = KeyboardEngine::get_subtype_display_name(&subtype)?
                .units()
                .to_vec();
            label.extend("  ·  ".encode_utf16());
            if let Some(layout) = layout {
                label.extend_from_slice(layout.units());
            }
            let label = native_text(&Utf16Text::from_units(label))?;
            let text = TextBlock::new();
            text.set_text(Some(&label));
            AutomationProperties::set_name(&button, Some(&label));
            let theme = self.theme.borrow().clone();
            text.set_foreground(Some(
                Brushes::get(if active {
                    theme.panel_selected_foreground()
                } else {
                    theme.popup_foreground()
                })
                .into(),
            ));
            text.set_font_weight(if active {
                FontWeight::SemiBold
            } else {
                FontWeight::Normal
            });
            text.set_vertical_alignment(VerticalAlignment::Center);
            text.set_font_size(15.0);
            text.set_is_hit_test_visible(false);
            button.set_content(text.upcast());
            let weak_view = self.to_ref().downgrade();
            button.click(move |_| {
                let Some(view) = weak_view.upgrade() else {
                    return;
                };
                view.engine
                    .set_active_subtype(subtype.clone())
                    .expect("subtype selection");
                view.close();
            });
            list.children().add(button);
        }
        let theme = self.theme.borrow().clone();
        let manage = TouchButton::new();
        manage.set_height(44.0);
        manage.set_corner_radius(CornerRadius::uniform(8.0));
        manage.set_padding(Thickness::new(12.0, 0.0, 12.0, 0.0));
        manage.set_scheduler(self.scheduler.clone());
        manage.set_colors(
            theme.action_button_background(),
            theme.action_button_pressed_background(),
        );
        let panel = StackPanel::new();
        panel.set_orientation(Orientation::Horizontal);
        panel.set_spacing(8.0);
        panel.set_vertical_alignment(VerticalAlignment::Center);
        panel.children().add(IconFactory::create(
            KeyIcon::Settings,
            18.0,
            Some(Brushes::get(theme.popup_foreground()).into()),
        )?);
        let text = TextBlock::new();
        let label = nullable_text(KeyboardStrings::manage_languages())?;
        text.set_text(label.as_deref());
        AutomationProperties::set_name(&manage, label.as_deref());
        text.set_foreground(Some(
            Brushes::get(self.theme.borrow().popup_foreground()).into(),
        ));
        text.set_font_size(15.0);
        text.set_vertical_alignment(VerticalAlignment::Center);
        panel.children().add(text);
        manage.set_content(panel.upcast());
        let weak_view = self.to_ref().downgrade();
        manage.click(move |_| {
            let Some(view) = weak_view.upgrade() else {
                return;
            };
            view.close();
            for (_, h) in view.manage_requested.snapshot().iter() {
                h(&view);
            }
        });
        list.children().add(manage);
        let theme = self.theme.borrow().clone();
        let card = Border::new();
        card.set_background(Some(Brushes::get(theme.popup_background()).into()));
        card.set_border_brush(Some(Brushes::get(theme.popup_border_color()).into()));
        card.set_border_thickness(Thickness::uniform(1.0));
        card.set_corner_radius(CornerRadius::uniform(theme.popup_corner_radius() + 4.0));
        card.set_padding(Thickness::uniform(6.0));
        card.set_min_width(260.0);
        card.set_max_width(420.0);
        card.set_horizontal_alignment(HorizontalAlignment::Center);
        card.set_vertical_alignment(VerticalAlignment::Center);
        let scroller = ScrollViewer::new();
        scroller.set_content(Some(Control::boxed(list)));
        scroller.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        scroller.set_is_tab_stop(false);
        card.set_child(scroller);
        card.pointer_pressed(|_, e| e.set_handled(true));
        self.children().add(card);
        FocusNeutral::apply(self);
        self.set_is_visible(true);
        Ok(())
    }
    pub fn close(&self) {
        self.set_is_visible(false);
    }
    pub fn release_visual_children(&self) {
        self.children().clear();
    }
}
#[cfg(test)]
#[path = "panel_views_tests.rs"]
mod tests;

impl Drop for ClipboardPanelView {
    fn drop(&mut self) {
        self.release_subscriptions();
    }
}
impl Drop for EditingPanelView {
    fn drop(&mut self) {
        self.release_subscriptions();
    }
}
impl Drop for SubtypePickerView {
    fn drop(&mut self) {
        self.release_visual_children();
    }
}
