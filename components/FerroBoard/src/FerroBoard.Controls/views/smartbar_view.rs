//! Derived from ScreenKeyboard.Uno/Views/SmartbarView.cs (Apache-2.0).
use super::{
    FontWeightsEx,
    view_helpers::{ViewError, format_one, native_text, nullable_text},
};
use crate::{
    KeyboardStrings,
    icons::IconFactory,
    primitives::{Brushes, FocusNeutral, TouchButton, source_max, source_min},
    themes::{KeyboardTheme, KeyboardThemes, ThemeList},
};
use ferroboard_core::{
    engine::{EngineError, EngineResult, KeyboardEngine},
    input::IKeyboardScheduler,
    keys::{KeyCode, KeyIcon, KeyType, KeyboardMode, KeyboardUiMode},
    layouts::{SharedList, TextKeyData},
    settings::{OneHandedMode, SmartbarLayout},
    text::{Suggestion, SuggestionKind, SuggestionList, Utf16Text},
};
use ferroui_base::{
    CornerRadius, FerroObjectImpl, FerroObjectImplExt, Ref, StyledElementImpl, Thickness,
    VisualImpl, ferro_class, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::{HorizontalAlignment, LayoutableImpl, Orientation, VerticalAlignment},
    media::{FontFamily, IBrush, TextTrimming},
};
use ferroui_controls::{
    Border, ColumnDefinition, Control, ControlImpl, Grid, GridLength, GridUnitType, PanelImpl,
    ScrollViewer, StackPanel, TextBlock, automation::AutomationProperties,
    primitives::ScrollBarVisibility,
};
use ferroui_system::events::HandlerList;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};

pub type SmartbarExecute = Rc<dyn Fn(&Rc<KeyboardEngine>) -> EngineResult<()>>;
pub type SmartbarIsActive = Rc<dyn Fn(&Rc<KeyboardEngine>) -> bool>;
/// Native quick action. Callbacks are synchronous Rust closures; errors propagate.
/// Markup reads the same action references. Closures are supplied at the composition root.
#[derive(Clone)]
pub struct SmartbarAction {
    pub id: Option<Utf16Text>,
    pub icon: KeyIcon,
    pub name: Option<Utf16Text>,
    pub execute: Option<SmartbarExecute>,
    pub is_active: Option<SmartbarIsActive>,
}
impl SmartbarAction {
    pub fn new(
        id: Option<Utf16Text>,
        icon: KeyIcon,
        name: Option<Utf16Text>,
        execute: Option<SmartbarExecute>,
        is_active: Option<SmartbarIsActive>,
    ) -> Rc<Self> {
        Rc::new(Self {
            id,
            icon,
            name,
            execute,
            is_active,
        })
    }
}
pub(crate) type ActionList = ThemeList<Option<Rc<SmartbarAction>>>;
pub(crate) type EmojiSelect = Rc<dyn Fn(Option<Utf16Text>) -> Result<(), ViewError>>;
pub(crate) struct EmojiSearchState {
    pub query: RefCell<Option<Utf16Text>>,
    pub results: RefCell<Option<SharedList<Option<Utf16Text>>>>,
    pub select: Option<EmojiSelect>,
}
impl EmojiSearchState {
    pub fn new(select: Option<EmojiSelect>) -> Rc<Self> {
        Rc::new(Self {
            query: RefCell::new(Some(Utf16Text::default())),
            results: RefCell::new(Some(Rc::new(RefCell::new(Vec::new())))),
            select,
        })
    }
}
pub(crate) struct TextKeyDataFactory;
impl TextKeyDataFactory {
    pub fn function(code: i32) -> Rc<TextKeyData> {
        TextKeyData::function(
            code,
            if code == KeyCode::Delete {
                KeyType::EnterEditing
            } else {
                KeyType::Function
            },
            None,
        )
    }
}

#[repr(C)]
pub(crate) struct SmartbarView {
    base: Grid,
    engine: Rc<KeyboardEngine>,
    scheduler: Rc<dyn IKeyboardScheduler>,
    toggle: Ref<TouchButton>,
    hide: Ref<TouchButton>,
    center: Ref<Grid>,
    suggestions: Ref<Grid>,
    actions_scroller: Ref<ScrollViewer>,
    actions: Ref<StackPanel>,
    search: Ref<Grid>,
    theme: RefCell<Arc<KeyboardTheme>>,
    actions_expanded: Cell<bool>,
    bar_height: Cell<f64>,
    action_list: RefCell<Option<ActionList>>,
    emoji_search: RefCell<Option<Rc<EmojiSearchState>>>,
    hide_requested: HandlerList<dyn Fn(&SmartbarView)>,
    search_back_requested: HandlerList<dyn Fn(&SmartbarView)>,
}
ferro_class!(SmartbarView:Grid);
ferro_impl_classes!(SmartbarView:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
impl FerroObjectImpl for SmartbarView {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_is_tab_stop(false);
        for value in [GridLength::AUTO, GridLength::STAR, GridLength::AUTO] {
            this.column_definitions()
                .add(ColumnDefinition::with_width(value));
        }
        this.toggle.set_scheduler(this.scheduler.clone());
        this.hide.set_scheduler(this.scheduler.clone());
        let weak = this.to_ref().downgrade();
        this.toggle.click(move |_| {
            if let Some(this) = weak.upgrade() {
                if this.emoji_search().is_some() {
                    for (_, h) in this.search_back_requested.snapshot().iter() {
                        h(&this);
                    }
                    return;
                }
                this.actions_expanded.set(!this.actions_expanded.get());
                this.refresh().expect("smartbar toggle");
            }
        });
        let weak = this.to_ref().downgrade();
        this.hide.click(move |_| {
            if let Some(this) = weak.upgrade() {
                for (_, h) in this.hide_requested.snapshot().iter() {
                    h(&this);
                }
            }
        });
        Grid::set_column(&this.toggle, 0);
        Grid::set_column(&this.center, 1);
        Grid::set_column(&this.hide, 2);
        horizontal_scroll(&this.actions_scroller);
        this.actions.set_orientation(Orientation::Horizontal);
        this.actions
            .set_horizontal_alignment(HorizontalAlignment::Center);
        this.actions_scroller
            .set_content(Some(Control::boxed(this.actions.clone())));
        this.center.children().add(this.suggestions.clone());
        this.center.children().add(this.actions_scroller.clone());
        this.center.children().add(this.search.clone());
        this.children().add(this.toggle.clone());
        this.children().add(this.center.clone());
        this.children().add(this.hide.clone());
        *this.action_list.borrow_mut() = Some(Self::create_default_actions());
    }
}
fn horizontal_scroll(s: &ScrollViewer) {
    s.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Hidden);
    s.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Disabled);
    s.set_is_tab_stop(false);
}
impl SmartbarView {
    pub fn construct(engine: Rc<KeyboardEngine>, scheduler: Rc<dyn IKeyboardScheduler>) -> Self {
        Self {
            base: Grid::construct(),
            engine,
            scheduler,
            toggle: TouchButton::new(),
            hide: TouchButton::new(),
            center: Grid::new(),
            suggestions: Grid::new(),
            actions_scroller: ScrollViewer::new(),
            actions: StackPanel::new(),
            search: Grid::new(),
            theme: RefCell::new(KeyboardThemes::floris_day()),
            actions_expanded: Cell::new(false),
            bar_height: Cell::new(44.0),
            action_list: RefCell::new(None),
            emoji_search: RefCell::new(None),
            hide_requested: HandlerList::new(),
            search_back_requested: HandlerList::new(),
        }
    }
    pub fn new(engine: Rc<KeyboardEngine>, scheduler: Rc<dyn IKeyboardScheduler>) -> Ref<Self> {
        instantiate(Self::construct(engine, scheduler))
    }
    /// Rust owner cleanup for source GC-collectible child callback cycles. Does not change
    /// engine subscriptions or external old-button lifetimes; no automatic Unloaded policy.
    pub fn release_visual_children(&self) {
        self.actions.children().clear();
        self.suggestions.children().clear();
        self.search.children().clear();
    }
    pub fn actions(&self) -> Option<ActionList> {
        self.action_list.borrow().clone()
    }
    pub fn set_actions(&self, value: Option<ActionList>) {
        *self.action_list.borrow_mut() = value;
    }
    pub fn emoji_search(&self) -> Option<Rc<EmojiSearchState>> {
        self.emoji_search.borrow().clone()
    }
    pub fn set_emoji_search(&self, value: Option<Rc<EmojiSearchState>>) {
        *self.emoji_search.borrow_mut() = value;
    }
    pub fn hide_requested(&self, h: impl Fn(&SmartbarView) + 'static) -> u64 {
        self.hide_requested.add(Rc::new(h))
    }
    /// Removes this publisher-owned callback; dropping a token alone keeps it registered.
    pub fn remove_hide_requested(&self, token: u64) -> bool {
        crate::callback_tokens::remove(&self.hide_requested, token)
    }
    pub fn search_back_requested(&self, h: impl Fn(&SmartbarView) + 'static) -> u64 {
        self.search_back_requested.add(Rc::new(h))
    }
    /// Removes this publisher-owned callback; dropping a token alone keeps it registered.
    pub fn remove_search_back_requested(&self, token: u64) -> bool {
        crate::callback_tokens::remove(&self.search_back_requested, token)
    }
    pub fn set_theme(&self, theme: Arc<KeyboardTheme>, height: f64) -> Result<(), ViewError> {
        *self.theme.borrow_mut() = theme.clone();
        self.bar_height.set(height);
        self.set_height(height);
        self.set_background(Some(Brushes::get(theme.smartbar_background()).into()));
        self.refresh()
    }
    fn create_default_actions() -> ActionList {
        let list = ActionList::default();
        macro_rules! add {
            ($id:literal,$icon:ident,$name:ident,$execute:expr) => {
                list.lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push(Some(SmartbarAction::new(
                        Some($id.into()),
                        KeyIcon::$icon,
                        KeyboardStrings::$name(),
                        Some(Rc::new($execute)),
                        None,
                    )));
            };
            ($id:literal,$icon:ident,$name:ident,$execute:expr,$active:expr) => {
                list.lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push(Some(SmartbarAction::new(
                        Some($id.into()),
                        KeyIcon::$icon,
                        KeyboardStrings::$name(),
                        Some(Rc::new($execute)),
                        Some(Rc::new($active)),
                    )));
            };
        }
        add!("undo", Undo, undo, |e| e
            .input_key(&*TextKeyDataFactory::function(KeyCode::Undo)));
        add!("redo", Redo, redo, |e| e
            .input_key(&*TextKeyDataFactory::function(KeyCode::Redo)));
        add!("emoji", Emoji, emoji, |e| e
            .set_ui_mode(KeyboardUiMode::Media));
        add!("clipboard", Clipboard, clipboard, |e| e
            .set_ui_mode(KeyboardUiMode::Clipboard));
        add!("editing", TextEditing, text_editing, |e| e
            .set_ui_mode(KeyboardUiMode::Editing));
        add!("numpad", Numpad, number_pad, |e| e
            .set_mode(KeyboardMode::NumericAdvanced));
        add!(
            "onehanded",
            OneHandedRight,
            one_handed_mode,
            |e| {
                let s = e.settings();
                s.set_one_handed_mode(if s.one_handed_mode() == OneHandedMode::Off {
                    OneHandedMode::Right
                } else {
                    OneHandedMode::Off
                });
                Ok(())
            },
            |e| e.settings().one_handed_mode() != OneHandedMode::Off
        );
        add!(
            "split",
            SplitKeyboard,
            split_keyboard,
            |e| {
                let s = e.settings();
                s.set_split_keyboard(!s.split_keyboard());
                Ok(())
            },
            |e| e.settings().split_keyboard()
        );
        add!(
            "floating",
            Floating,
            floating_keyboard,
            |e| {
                let s = e.settings();
                s.set_floating(!s.floating());
                Ok(())
            },
            |e| e.settings().floating()
        );
        add!("resize", ResizeHeight, resize_keyboard, |e| e.input_key(
            &*TextKeyDataFactory::function(KeyCode::ToggleResizeMode)
        ));
        add!(
            "incognito",
            Incognito,
            incognito,
            |e| {
                let s = e.settings();
                s.set_incognito_mode(!s.incognito_mode());
                Ok(())
            },
            |e| e.settings().incognito_mode()
        );
        add!(
            "autocorrect",
            Autocorrect,
            auto_correct,
            |e| {
                let s = e.settings();
                s.set_auto_correct(!s.auto_correct());
                Ok(())
            },
            |e| e.settings().auto_correct()
        );
        add!("language", Language, languages, |e| e.input_key(
            &*TextKeyDataFactory::function(KeyCode::ShowSubtypePicker)
        ));
        add!("settings", Settings, settings, |e| e
            .input_key(&*TextKeyDataFactory::function(KeyCode::Settings)));
        list
    }
    pub fn refresh(&self) -> Result<(), ViewError> {
        let theme = self.theme.borrow().clone();
        let fg = Brushes::get(theme.smartbar_foreground());
        let icon = source_min(22.0, self.bar_height.get() * 0.5);
        let width = source_max(40.0, self.bar_height.get());
        if let Some(search) = self.emoji_search() {
            self.toggle.set_width(width);
            self.icon(
                &self.toggle,
                KeyIcon::ChevronLeft,
                icon,
                fg.clone().into(),
                KeyboardStrings::back(),
            )?;
            self.toggle.set_colors(
                theme.action_button_background(),
                theme.action_button_pressed_background(),
            );
            self.hide.set_is_visible(false);
            self.suggestions.set_is_visible(false);
            self.actions_scroller.set_is_visible(false);
            self.search.set_is_visible(true);
            self.build_search(&search, fg.into())?;
            FocusNeutral::apply(self);
            return Ok(());
        }
        self.search.set_is_visible(false);
        self.hide.set_is_visible(true);
        self.hide.set_width(width);
        self.icon(
            &self.hide,
            KeyIcon::HideKeyboard,
            icon,
            fg.clone().into(),
            KeyboardStrings::hide_keyboard(),
        )?;
        self.hide.set_colors(
            theme.action_button_background(),
            theme.action_button_pressed_background(),
        );
        let suggestions = self.engine.suggestions();
        let layout = self.engine.settings().smartbar_layout();
        let mut show = layout == SmartbarLayout::ActionsOnly
            || self.actions_expanded.get()
            || layout == SmartbarLayout::SuggestionsWithActions
                && suggestions.len().map_err(EngineError::from)? == 0;
        if layout == SmartbarLayout::SuggestionsOnly {
            show = false;
        }
        self.toggle
            .set_is_visible(layout == SmartbarLayout::SuggestionsWithActions);
        self.toggle.set_width(width);
        self.icon(
            &self.toggle,
            if show && self.actions_expanded.get() {
                KeyIcon::ChevronLeft
            } else {
                KeyIcon::ChevronRight
            },
            icon,
            fg.clone().into(),
            KeyboardStrings::quick_actions(),
        )?;
        self.toggle.set_colors(
            theme.action_button_background(),
            theme.action_button_pressed_background(),
        );
        self.suggestions.set_is_visible(!show);
        self.actions_scroller.set_is_visible(show);
        if show {
            self.build_actions(fg.into(), icon, width)?;
        } else {
            self.build_suggestions(&suggestions)?;
        }
        FocusNeutral::apply(self);
        Ok(())
    }
    fn icon(
        &self,
        button: &TouchButton,
        icon: KeyIcon,
        size: f64,
        fg: Rc<dyn IBrush>,
        name: Option<Utf16Text>,
    ) -> Result<(), ViewError> {
        let name = nullable_text(name)?;
        button.set_icon(icon, size, Some(fg), name.as_deref())?;
        Ok(())
    }
    fn build_actions(&self, fg: Rc<dyn IBrush>, size: f64, width: f64) -> Result<(), ViewError> {
        self.actions.children().clear();
        let actions = self.actions().ok_or(ViewError::NullReference)?;
        let actions = actions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        for action in actions {
            let action = action.ok_or(ViewError::NullReference)?;
            let button = TouchButton::new();
            button.set_width(width);
            button.set_height(self.bar_height.get());
            button.set_corner_radius(CornerRadius::uniform(self.bar_height.get() / 2.0));
            button.set_scheduler(self.scheduler.clone());
            let active = action.is_active.as_ref().is_some_and(|f| f(&self.engine));
            let theme = self.theme.borrow().clone();
            let name = nullable_text(action.name.clone())?;
            button.set_icon(
                action.icon,
                size,
                Some(if active {
                    Brushes::get(theme.suggestion_highlight_foreground()).into()
                } else {
                    fg.clone()
                }),
                name.as_deref(),
            )?;
            let theme = self.theme.borrow().clone();
            button.set_colors(
                theme.action_button_background(),
                theme.action_button_pressed_background(),
            );
            let weak_view = self.to_ref().downgrade();
            button.click(move |_| {
                let Some(view) = weak_view.upgrade() else {
                    return;
                };
                let execute = action
                    .execute
                    .as_ref()
                    .expect("null SmartbarAction.Execute");
                execute(&view.engine).expect("smartbar action");
                view.actions_expanded.set(false);
                view.refresh().expect("smartbar action refresh");
            });
            self.actions.children().add(button);
        }
        Ok(())
    }
    fn build_suggestions(&self, suggestions: &SuggestionList) -> Result<(), ViewError> {
        self.suggestions.children().clear();
        self.suggestions.column_definitions().clear();
        let count = suggestions.len().map_err(EngineError::from)?;
        if count == 0 {
            return Ok(());
        }
        let theme = self.theme.borrow().clone();
        let family = nullable_text(theme.font_family())?
            .as_deref()
            .map(FontFamily::new);
        for i in 0..count {
            if i > 0 {
                self.suggestions
                    .column_definitions()
                    .add(ColumnDefinition::with_value(1.0, GridUnitType::Pixel));
                let divider = Border::new();
                let theme = self.theme.borrow().clone();
                divider.set_background(Some(Brushes::get(theme.divider_color()).into()));
                divider.set_margin(Thickness::new(
                    0.0,
                    self.bar_height.get() * 0.28,
                    0.0,
                    self.bar_height.get() * 0.28,
                ));
                divider.set_is_hit_test_visible(false);
                Grid::set_column(
                    &divider,
                    self.suggestions.column_definitions().count() as i32 - 1,
                );
                self.suggestions.children().add(divider);
            }
            self.suggestions
                .column_definitions()
                .add(ColumnDefinition::with_value(1.0, GridUnitType::Star));
            let suggestion = suggestions.get(i).map_err(EngineError::from)?;
            let button = TouchButton::new();
            button.set_margin(Thickness::new(2.0, 3.0, 2.0, 3.0));
            button.set_corner_radius(CornerRadius::uniform(8.0));
            button.set_scheduler(self.scheduler.clone());
            let theme = self.theme.borrow().clone();
            button.set_colors(
                theme.action_button_background(),
                theme.action_button_pressed_background(),
            );
            let suggestion = suggestion.ok_or(ViewError::NullReference)?;
            let fg = Brushes::get(if suggestion.is_auto_commit() {
                theme.suggestion_highlight_foreground()
            } else {
                theme.smartbar_foreground()
            });
            if suggestion.kind() == SuggestionKind::Clipboard {
                let panel = StackPanel::new();
                panel.set_orientation(Orientation::Horizontal);
                panel.set_horizontal_alignment(HorizontalAlignment::Center);
                panel.set_spacing(6.0);
                panel.children().add(IconFactory::create(
                    KeyIcon::Paste,
                    16.0,
                    Some(fg.clone().into()),
                )?);
                let text = TextBlock::new();
                let label = Utf16Text::from_units(
                    suggestion
                        .text()
                        .ok_or(ViewError::NullReference)?
                        .units()
                        .iter()
                        .map(|&u| if u == 10 { 32 } else { u })
                        .collect::<Vec<_>>(),
                );
                text.set_text(Some(&native_text(&label)?));
                text.set_font_size(theme.smartbar_font_size() * 0.9);
                text.set_foreground(Some(fg.into()));
                text.set_vertical_alignment(VerticalAlignment::Center);
                text.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
                text.set_max_width(180.0);
                panel.children().add(text);
                button.set_content(panel.upcast());
                let label = super::view_helpers::format_one_nullable(
                    KeyboardStrings::paste_format().as_ref(),
                    suggestion.text().as_ref(),
                )?;
                AutomationProperties::set_name(&button, Some(&native_text(&label)?));
            } else {
                let label = nullable_text(suggestion.text())?;
                let text = button.set_text_nullable(
                    label.as_deref(),
                    if suggestion.kind() == SuggestionKind::Emoji {
                        theme.smartbar_font_size() * 1.3
                    } else {
                        theme.smartbar_font_size()
                    },
                    Some(fg.into()),
                    family.clone(),
                );
                if suggestion.is_auto_commit() {
                    text.set_font_weight(FontWeightsEx::semi_bold());
                }
            }
            let engine = self.engine.clone();
            let item = suggestion.clone();
            button.click(move |_| {
                engine
                    .commit_suggestion(&item)
                    .expect("commit smartbar suggestion")
            });
            if suggestion.is_removable() {
                let engine = self.engine.clone();
                let item = suggestion.clone();
                button.long_press(move |_| {
                    engine
                        .remove_suggestion(&item)
                        .expect("remove smartbar suggestion")
                });
            }
            Grid::set_column(
                &button,
                self.suggestions.column_definitions().count() as i32 - 1,
            );
            self.suggestions.children().add(button);
        }
        Ok(())
    }
    fn build_search(
        &self,
        search: &Rc<EmojiSearchState>,
        fg: Rc<dyn IBrush>,
    ) -> Result<(), ViewError> {
        self.search.children().clear();
        self.search.column_definitions().clear();
        self.search
            .column_definitions()
            .add(ColumnDefinition::with_value(0.4, GridUnitType::Star));
        self.search
            .column_definitions()
            .add(ColumnDefinition::with_value(1.0, GridUnitType::Star));
        let theme = self.theme.borrow().clone();
        let border = Border::new();
        border.set_background(Some(Brushes::get(theme.key_background()).into()));
        border.set_corner_radius(CornerRadius::uniform(self.bar_height.get() / 2.0));
        border.set_margin(Thickness::new(0.0, 5.0, 6.0, 5.0));
        border.set_padding(Thickness::new(12.0, 0.0, 12.0, 0.0));
        let panel = StackPanel::new();
        panel.set_orientation(Orientation::Horizontal);
        panel.set_vertical_alignment(VerticalAlignment::Center);
        panel.set_spacing(6.0);
        panel.children().add(IconFactory::create(
            KeyIcon::Search,
            16.0,
            Some(Brushes::get(theme.key_hint_foreground()).into()),
        )?);
        let query = search
            .query
            .borrow()
            .clone()
            .ok_or(ViewError::NullReference)?;
        let label = if query.is_empty() {
            nullable_text(KeyboardStrings::search_emoji())?
        } else {
            let mut units = query.units().to_vec();
            units.push(124);
            Some(native_text(&Utf16Text::from_units(units))?)
        };
        let text = TextBlock::new();
        text.set_text(label.as_deref());
        text.set_foreground(Some(
            Brushes::get(if query.is_empty() {
                theme.key_hint_foreground()
            } else {
                theme.key_foreground()
            })
            .into(),
        ));
        text.set_font_size(theme.smartbar_font_size() * 0.9);
        text.set_vertical_alignment(VerticalAlignment::Center);
        text.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
        panel.children().add(text);
        border.set_child(panel);
        self.search.children().add(border);
        let scroller = ScrollViewer::new();
        horizontal_scroll(&scroller);
        let results = StackPanel::new();
        results.set_orientation(Orientation::Horizontal);
        let values = search
            .results
            .borrow()
            .clone()
            .ok_or(ViewError::NullReference)?;
        let snapshot = values.borrow().clone();
        for emoji in snapshot {
            let button = TouchButton::new();
            button.set_width(self.bar_height.get());
            button.set_height(self.bar_height.get());
            button.set_corner_radius(CornerRadius::uniform(8.0));
            button.set_scheduler(self.scheduler.clone());
            let theme = self.theme.borrow().clone();
            button.set_colors(
                theme.action_button_background(),
                theme.action_button_pressed_background(),
            );
            let label = nullable_text(emoji.clone())?;
            button.set_text_nullable(
                label.as_deref(),
                theme.smartbar_font_size() * 1.3,
                Some(fg.clone()),
                None,
            );
            let state = search.clone();
            button.click(move |_| {
                state.select.as_ref().expect("null EmojiSearchState.Select")(emoji.clone())
                    .expect("emoji search select")
            });
            results.children().add(button);
        }
        if values.borrow().is_empty()
            && !search
                .query
                .borrow()
                .as_ref()
                .ok_or(ViewError::NullReference)?
                .is_empty()
        {
            let text = TextBlock::new();
            text.set_text(nullable_text(KeyboardStrings::no_emoji_found())?.as_deref());
            text.set_foreground(Some(
                Brushes::get(self.theme.borrow().key_hint_foreground()).into(),
            ));
            text.set_vertical_alignment(VerticalAlignment::Center);
            text.set_margin(Thickness::new(8.0, 0.0, 0.0, 0.0));
            results.children().add(text);
        }
        scroller.set_content(Some(Control::boxed(results)));
        Grid::set_column(&scroller, 1);
        self.search.children().add(scroller);
        Ok(())
    }
}
#[cfg(test)]
#[path = "smartbar_view_tests.rs"]
mod tests;

impl Drop for SmartbarView {
    fn drop(&mut self) {
        self.release_visual_children();
    }
}
