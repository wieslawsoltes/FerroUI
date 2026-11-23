//! Derived from ScreenKeyboard.Uno/Views/EmojiPanelView.cs (Apache-2.0).
use super::{
    TextKeyDataFactory,
    view_helpers::{ViewError, native_text, nullable_text},
};
use crate::{
    KeyboardStrings,
    enum_bridge::parse_enum_case_sensitive,
    icons::IconFactory,
    primitives::{Brushes, FocusNeutral, TouchButton, UniformWrapPanel, source_max, source_min},
    themes::{KeyboardTheme, KeyboardThemes},
};
use ferroboard_core::{
    emoji::{EmojiCatalog, EmojiCategory, EmojiInfo},
    engine::{EngineError, KeyboardEngine},
    input::IKeyboardScheduler,
    keys::{KeyCode, KeyIcon, KeyboardUiMode},
    text::Utf16Text,
};
use ferroui_base::{
    CornerRadius, FerroObjectImpl, FerroObjectImplExt, Point, Ref, StyledElementImpl, Thickness,
    Vector, Visual, VisualImpl, ferro_class, ferro_impl_classes,
    input::{InputElement, InputElementImpl},
    instantiate,
    interactivity::{Interactive, InteractiveImpl, RoutingStrategies},
    layout::{HorizontalAlignment, LayoutableImpl, Orientation, VerticalAlignment},
    media::IBrush,
};
use ferroui_controls::{
    Border, Canvas, ColumnDefinition, Control, ControlImpl, Grid, GridLength, PanelImpl,
    RowDefinition, ScrollViewer, StackPanel, TextBlock, primitives::ScrollBarVisibility,
};
use ferroui_system::events::HandlerList;
use std::time::Duration;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
    sync::Arc,
};
const CATEGORY_NAMES: [&str; 10] = [
    "Recent",
    "SmileysEmotion",
    "PeopleBody",
    "AnimalsNature",
    "FoodDrink",
    "TravelPlaces",
    "Activities",
    "Objects",
    "Symbols",
    "Flags",
];
const EMOTICONS_TAB: &str = "emoticons";
fn category_name(category: EmojiCategory) -> Utf16Text {
    usize::try_from(category.0)
        .ok()
        .and_then(|i| CATEGORY_NAMES.get(i))
        .map_or_else(|| category.0.to_string().as_str().into(), |s| (*s).into())
}
fn emoji_error(e: ferroboard_core::emoji::EmojiError) -> ViewError {
    EngineError::Emoji(e).into()
}
#[repr(C)]
pub(crate) struct EmojiPanelView {
    base: Grid,
    engine: Rc<KeyboardEngine>,
    scheduler: Rc<dyn IKeyboardScheduler>,
    scroller: Ref<ScrollViewer>,
    bottom_bar: Ref<Grid>,
    tabs: Ref<StackPanel>,
    overlay: Ref<Canvas>,
    pages: RefCell<HashMap<Utf16Text, Ref<UniformWrapPanel>>>,
    theme: RefCell<Arc<KeyboardTheme>>,
    current_tab: RefCell<Utf16Text>,
    tone_popup: RefCell<Option<Ref<Border>>>,
    bar_height: Cell<f64>,
    history_subscription: Cell<Option<u64>>,
    search_requested: HandlerList<dyn Fn(&EmojiPanelView)>,
}
ferro_class!(EmojiPanelView:Grid);
ferro_impl_classes!(EmojiPanelView:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,PanelImpl);
impl FerroObjectImpl for EmojiPanelView {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_is_tab_stop(false);
        this.row_definitions()
            .add(RowDefinition::with_height(GridLength::STAR));
        this.row_definitions()
            .add(RowDefinition::with_height(GridLength::AUTO));
        this.scroller
            .set_vertical_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        this.scroller
            .set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        this.scroller.set_is_tab_stop(false);
        this.tabs.set_orientation(Orientation::Horizontal);
        this.tabs
            .set_horizontal_alignment(HorizontalAlignment::Center);
        Grid::set_row(&this.scroller, 0);
        Grid::set_row(&this.bottom_bar, 1);
        Grid::set_row_span(&this.overlay, 2);
        this.children().add(this.scroller.clone());
        this.children().add(this.bottom_bar.clone());
        this.children().add(this.overlay.clone());
        let weak_view = this.to_ref().downgrade();
        let token = this.engine.emoji_history().changed().add(Rc::new(move |_| {
            let Some(view) = weak_view.upgrade() else {
                return;
            };
            view.pages.borrow_mut().remove(&Utf16Text::from("Recent"));
        }));
        this.history_subscription.set(Some(token));
        let weak = this.to_ref().downgrade();
        this.add_handler_with(
            InputElement::pointer_pressed_event(),
            move |_, e| {
                if let Some(view) = weak.upgrade() {
                    let popup = view.tone_popup.borrow().clone();
                    if let Some(popup) = popup {
                        let source = e.source().and_then(|s| s.downcast::<Visual>().ok());
                        if !Self::is_inside(&popup, source) {
                            view.close_tone_popup();
                        }
                    }
                }
            },
            RoutingStrategies::DIRECT | RoutingStrategies::BUBBLE,
            true,
        );
    }
}
impl EmojiPanelView {
    pub fn new(engine: Rc<KeyboardEngine>, scheduler: Rc<dyn IKeyboardScheduler>) -> Ref<Self> {
        instantiate(Self {
            base: Grid::construct(),
            engine,
            scheduler,
            scroller: ScrollViewer::new(),
            bottom_bar: Grid::new(),
            tabs: StackPanel::new(),
            overlay: Canvas::new(),
            pages: RefCell::new(HashMap::new()),
            theme: RefCell::new(KeyboardThemes::floris_day()),
            current_tab: RefCell::new(Utf16Text::default()),
            tone_popup: RefCell::new(None),
            bar_height: Cell::new(44.0),
            history_subscription: Cell::new(None),
            search_requested: HandlerList::new(),
        })
    }
    pub fn bar_height(&self) -> f64 {
        self.bar_height.get()
    }
    pub fn set_bar_height(&self, v: f64) {
        self.bar_height.set(v);
    }
    pub fn search_requested(&self, h: impl Fn(&EmojiPanelView) + 'static) -> u64 {
        self.search_requested.add(Rc::new(h))
    }
    /// Removes this publisher-owned callback; dropping a token alone keeps it registered.
    pub fn remove_search_requested(&self, token: u64) -> bool {
        crate::callback_tokens::remove(&self.search_requested, token)
    }
    fn theme(&self) -> Arc<KeyboardTheme> {
        self.theme.borrow().clone()
    }
    pub fn set_theme(&self, theme: Arc<KeyboardTheme>) -> Result<(), ViewError> {
        *self.theme.borrow_mut() = theme.clone();
        self.set_background(Some(Brushes::get(theme.panel_background()).into()));
        self.pages.borrow_mut().clear();
        self.build_bottom_bar()?;
        FocusNeutral::apply(self);
        let current = self.current_tab.borrow().clone();
        if !current.is_empty() {
            self.show_tab(current)?;
        }
        Ok(())
    }
    pub fn on_shown(&self) -> Result<(), ViewError> {
        self.close_tone_popup();
        let current = self.current_tab.borrow().clone();
        if current.is_empty() {
            self.show_tab(if !self.engine.emoji_history().combined().is_empty() {
                "Recent".into()
            } else {
                "SmileysEmotion".into()
            })?;
        } else if current == Utf16Text::from("Recent") {
            self.show_tab(current)?;
        }
        Ok(())
    }
    /// Explicit native cleanup; Drop also removes history hooks and private visual resources.
    pub fn release_subscriptions(&self) {
        if let Some(token) = self.history_subscription.take() {
            self.engine.emoji_history().changed().remove(token);
        }
        self.pages.borrow_mut().clear();
        self.scroller.set_content(None);
        self.bottom_bar.children().clear();
        self.tabs.children().clear();
        self.overlay.children().clear();
        self.tone_popup.borrow_mut().take();
    }
    fn icon_for(category: EmojiCategory) -> KeyIcon {
        match category {
            EmojiCategory::Recent => KeyIcon::Recent,
            EmojiCategory::SmileysEmotion => KeyIcon::Smiley,
            EmojiCategory::PeopleBody => KeyIcon::Person,
            EmojiCategory::AnimalsNature => KeyIcon::Nature,
            EmojiCategory::FoodDrink => KeyIcon::Food,
            EmojiCategory::TravelPlaces => KeyIcon::Travel,
            EmojiCategory::Activities => KeyIcon::Activity,
            EmojiCategory::Objects => KeyIcon::Objects,
            EmojiCategory::Symbols => KeyIcon::Symbols,
            EmojiCategory::Flags => KeyIcon::Flag,
            _ => KeyIcon::Smiley,
        }
    }
    fn icon(
        &self,
        b: &TouchButton,
        icon: KeyIcon,
        size: f64,
        fg: Rc<dyn IBrush>,
        name: Option<Utf16Text>,
    ) -> Result<(), ViewError> {
        b.set_icon(icon, size, Some(fg), nullable_text(name)?.as_deref())?;
        Ok(())
    }
    fn build_bottom_bar(&self) -> Result<(), ViewError> {
        // The pin keeps a detached ScrollViewer's presenter child parented. Source Uno
        // reuses _tabs in a new scroller; explicitly release the old native content after
        // the source Clear step so the same tab-strip instance can transfer safely.
        let previous = self
            .bottom_bar
            .children()
            .snapshot()
            .iter()
            .find_map(|child| {
                let scroller = child.clone().downcast::<ScrollViewer>().ok()?;
                let content = scroller.content().as_ref().and_then(Control::from_boxed)?;
                (content == self.tabs.clone().upcast::<Control>()).then_some(scroller)
            });
        self.bottom_bar.children().clear();
        if let Some(previous) = previous {
            previous.set_content(None);
        }
        self.bottom_bar.column_definitions().clear();
        self.bottom_bar.set_height(self.bar_height());
        for length in [
            GridLength::AUTO,
            GridLength::STAR,
            GridLength::AUTO,
            GridLength::AUTO,
        ] {
            self.bottom_bar
                .column_definitions()
                .add(ColumnDefinition::with_width(length));
        }
        let theme = self.theme();
        let fg = Brushes::get(theme.panel_foreground());
        let abc = TouchButton::new();
        abc.set_width(self.bar_height() * 1.4);
        abc.set_margin(Thickness::new(4.0, 4.0, 2.0, 4.0));
        abc.set_corner_radius(CornerRadius::uniform(theme.key_corner_radius()));
        abc.set_scheduler(self.scheduler.clone());
        abc.set_colors(
            theme.function_key_background(),
            theme.function_key_pressed_background(),
        );
        abc.set_text_nullable(
            nullable_text(KeyboardStrings::abc())?.as_deref(),
            theme.function_key_font_size() * 0.9,
            Some(Brushes::get(theme.function_key_foreground()).into()),
            None,
        );
        let weak_view = self.to_ref().downgrade();
        abc.click(move |_| {
            let Some(view) = weak_view.upgrade() else {
                return;
            };
            view.engine
                .set_ui_mode(KeyboardUiMode::Text)
                .expect("emoji ABC")
        });
        Grid::set_column(&abc, 0);
        self.bottom_bar.children().add(abc);
        let scroller = ScrollViewer::new();
        scroller.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Hidden);
        scroller.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        scroller.set_is_tab_stop(false);
        scroller.set_content(Some(Control::boxed(self.tabs.clone())));
        Grid::set_column(&scroller, 1);
        self.bottom_bar.children().add(scroller);
        self.build_tabs()?;
        let theme = self.theme();
        let search = TouchButton::new();
        search.set_width(self.bar_height());
        search.set_margin(Thickness::new(2.0, 4.0, 2.0, 4.0));
        search.set_corner_radius(CornerRadius::uniform(self.bar_height() / 2.0));
        search.set_scheduler(self.scheduler.clone());
        search.set_colors(
            theme.action_button_background(),
            theme.action_button_pressed_background(),
        );
        self.icon(
            &search,
            KeyIcon::Search,
            20.0,
            fg.into(),
            KeyboardStrings::search_emoji(),
        )?;
        let weak_view = self.to_ref().downgrade();
        search.click(move |_| {
            let Some(view) = weak_view.upgrade() else {
                return;
            };
            for (_, h) in view.search_requested.snapshot().iter() {
                h(&view);
            }
        });
        Grid::set_column(&search, 2);
        self.bottom_bar.children().add(search);
        let theme = self.theme();
        let back = TouchButton::new();
        back.set_width(self.bar_height() * 1.4);
        back.set_margin(Thickness::new(2.0, 4.0, 4.0, 4.0));
        back.set_corner_radius(CornerRadius::uniform(theme.key_corner_radius()));
        back.set_repeat_delay(Some(Duration::from_millis(
            (self.engine.settings().key_repeat_delay()) as u64,
        )));
        back.set_repeat_interval(Duration::from_millis(
            (self.engine.settings().key_repeat_interval().max(40)) as u64,
        ));
        back.set_scheduler(self.scheduler.clone());
        back.set_colors(
            theme.function_key_background(),
            theme.function_key_pressed_background(),
        );
        self.icon(
            &back,
            KeyIcon::Backspace,
            20.0,
            Brushes::get(theme.function_key_foreground()).into(),
            KeyboardStrings::backspace(),
        )?;
        let weak_view = self.to_ref().downgrade();
        back.click(move |_| {
            let Some(view) = weak_view.upgrade() else {
                return;
            };
            view.engine
                .input_key(&*TextKeyDataFactory::function(KeyCode::Delete))
                .expect("emoji backspace")
        });
        Grid::set_column(&back, 3);
        self.bottom_bar.children().add(back);
        Ok(())
    }
    fn build_tabs(&self) -> Result<(), ViewError> {
        self.tabs.children().clear();
        let mut tabs = vec![(
            Utf16Text::from("Recent"),
            KeyIcon::Recent,
            KeyboardStrings::recent(),
        )];
        for category in self.engine.emoji_catalog()?.categories() {
            let name = category_name(category);
            tabs.push((name.clone(), Self::icon_for(category), Some(name)));
        }
        tabs.push((
            EMOTICONS_TAB.into(),
            KeyIcon::None,
            KeyboardStrings::emoticons(),
        ));
        let size = source_min(self.bar_height() - 8.0, 40.0);
        for (id, icon, name) in tabs {
            let selected = id == *self.current_tab.borrow();
            let theme = self.theme();
            let brush = Brushes::get(if selected {
                theme.panel_selected_foreground()
            } else {
                theme.panel_foreground()
            });
            let button = TouchButton::new();
            button.set_width(size);
            button.set_height(size);
            button.set_corner_radius(CornerRadius::uniform(size / 2.0));
            button.set_margin(Thickness::new(1.0, 0.0, 1.0, 0.0));
            button.set_scheduler(self.scheduler.clone());
            button.set_colors(
                theme.action_button_background(),
                theme.action_button_pressed_background(),
            );
            if icon == KeyIcon::None {
                button.set_text(":-)", 13.0, Some(brush.clone().into()), None);
            } else {
                self.icon(&button, icon, 20.0, brush.clone().into(), name)?;
            }
            if selected {
                button.set_border_brush(Some(brush.into()));
                button.set_border_thickness(Thickness::new(0.0, 0.0, 0.0, 2.0));
            }
            let weak_view = self.to_ref().downgrade();
            button.click(move |_| {
                let Some(view) = weak_view.upgrade() else {
                    return;
                };
                view.show_tab(id.clone()).expect("emoji tab");
            });
            self.tabs.children().add(button);
        }
        FocusNeutral::apply(&self.tabs);
        Ok(())
    }
    fn show_tab(&self, id: Utf16Text) -> Result<(), ViewError> {
        self.close_tone_popup();
        let changed = id != *self.current_tab.borrow();
        *self.current_tab.borrow_mut() = id.clone();
        let cached = self.pages.borrow().get(&id).cloned();
        let page = if let Some(page) = cached {
            page
        } else {
            let page = self.build_page(&id)?;
            self.pages.borrow_mut().insert(id, page.clone());
            page
        };
        FocusNeutral::apply(&page);
        self.scroller.set_content(Some(Control::boxed(page)));
        if changed {
            let old = self.scroller.offset();
            self.scroller.set_offset(Vector::new(old.x, 0.0));
            self.build_tabs()?;
        }
        Ok(())
    }
    fn build_page(&self, id: &Utf16Text) -> Result<Ref<UniformWrapPanel>, ViewError> {
        let theme = self.theme();
        let cell = source_max(40.0, theme.emoji_font_size() * 1.6);
        let page = UniformWrapPanel::new();
        page.set_item_width(cell);
        page.set_item_height(cell);
        page.set_margin(Thickness::uniform(4.0));
        let fg = Brushes::get(theme.panel_foreground());
        if *id == Utf16Text::from(EMOTICONS_TAB) {
            page.set_item_width(cell * 2.0);
            for emoticon in EmojiCatalog::load_emoticons().map_err(emoji_error)? {
                let button = self.create_cell(
                    &emoticon.text,
                    self.theme().smartbar_font_size(),
                    fg.clone().into(),
                )?;
                let weak_view = self.to_ref().downgrade();
                let text = emoticon.text.clone();
                button.click(move |_| {
                    let Some(view) = weak_view.upgrade() else {
                        return;
                    };
                    view.engine.input_text(&text).expect("emoticon input");
                });
                page.children().add(button);
            }
            return Ok(page);
        }
        if *id == Utf16Text::from("Recent") {
            for emoji in self.engine.emoji_history().combined() {
                let info = self
                    .engine
                    .emoji_catalog()?
                    .get_nullable(emoji.as_ref())
                    .map_err(ferroboard_core::engine::EngineError::from)?;
                self.add_emoji_cell(
                    &page,
                    id,
                    emoji.expect("Get(null) failed before cell creation"),
                    info,
                    fg.clone().into(),
                )?;
            }
        } else if let Some(value) = parse_enum_case_sensitive(Some(id), &CATEGORY_NAMES) {
            let catalog = self.engine.emoji_catalog()?;
            let entries = catalog.get_emojis(EmojiCategory(value)).lock().clone();
            for info in entries {
                let emoji = self.default_variant(&info)?;
                self.add_emoji_cell(&page, id, emoji, Some(info), fg.clone().into())?;
            }
        }
        if page.children().count() == 0 {
            page.set_item_width(400.0);
            let text = TextBlock::new();
            text.set_text(nullable_text(KeyboardStrings::recent_emoji_empty())?.as_deref());
            text.set_foreground(Some(
                Brushes::get(self.theme().key_hint_foreground()).into(),
            ));
            text.set_horizontal_alignment(HorizontalAlignment::Center);
            text.set_vertical_alignment(VerticalAlignment::Center);
            text.set_margin(Thickness::new(0.0, 24.0, 0.0, 0.0));
            page.children().add(text);
        }
        Ok(page)
    }
    fn add_emoji_cell(
        &self,
        page: &UniformWrapPanel,
        id: &Utf16Text,
        emoji: Utf16Text,
        info: Option<Arc<EmojiInfo>>,
        fg: Rc<dyn IBrush>,
    ) -> Result<(), ViewError> {
        let button = self.create_cell(&emoji, self.theme().emoji_font_size(), fg)?;
        let weak_view = self.to_ref().downgrade();
        let value = emoji.clone();
        button.click(move |_| {
            let Some(view) = weak_view.upgrade() else {
                return;
            };
            view.engine.input_emoji(&value).expect("emoji input");
        });
        if info.as_ref().is_some_and(|i| i.has_skin_tones()) || *id == Utf16Text::from("Recent") {
            let weak_view = self.to_ref().downgrade();
            button.long_press(move |anchor| {
                let Some(view) = weak_view.upgrade() else {
                    return;
                };
                view.on_emoji_long_press(anchor, emoji.clone(), info.clone())
                    .expect("emoji tone popup")
            });
        }
        page.children().add(button);
        Ok(())
    }
    fn default_variant(&self, info: &EmojiInfo) -> Result<Utf16Text, ViewError> {
        if !info.has_skin_tones() {
            return Ok(info.value.clone());
        }
        let tone = self
            .engine
            .emoji_history()
            .get_preferred_tone(&info.value)
            .unwrap_or_else(|| self.engine.settings().emoji_skin_tone());
        info.with_skin_tone(tone).map_err(emoji_error)
    }
    fn create_cell(
        &self,
        text: &Utf16Text,
        font_size: f64,
        fg: Rc<dyn IBrush>,
    ) -> Result<Ref<TouchButton>, ViewError> {
        let button = TouchButton::new();
        button.set_corner_radius(CornerRadius::uniform(8.0));
        button.set_scheduler(self.scheduler.clone());
        let theme = self.theme();
        button.set_colors(
            theme.action_button_background(),
            theme.action_button_pressed_background(),
        );
        button.set_text(&native_text(text)?, font_size, Some(fg), None);
        Ok(button)
    }
    fn on_emoji_long_press(
        &self,
        anchor: &TouchButton,
        emoji: Utf16Text,
        info: Option<Arc<EmojiInfo>>,
    ) -> Result<(), ViewError> {
        self.close_tone_popup();
        let mut variants = Vec::new();
        if let Some(info) = info.filter(|i| i.has_skin_tones()) {
            variants.push(info.value.clone());
            for v in info.variants().lock().iter().take(5) {
                variants.push(v.clone());
            }
        }
        let recent = *self.current_tab.borrow() == Utf16Text::from("Recent");
        if variants.is_empty() && !recent {
            return Ok(());
        }
        let cell = anchor.bounds().height;
        let row = StackPanel::new();
        row.set_orientation(Orientation::Horizontal);
        for value in variants {
            let button = self.create_cell(
                &value,
                self.theme().emoji_font_size(),
                Brushes::get(self.theme().popup_foreground()).into(),
            )?;
            button.set_width(cell);
            button.set_height(cell);
            let weak_view = self.to_ref().downgrade();
            button.click(move |_| {
                let Some(view) = weak_view.upgrade() else {
                    return;
                };
                view.close_tone_popup();
                view.engine
                    .input_emoji(&value)
                    .expect("emoji variant input");
                let current = view.current_tab.borrow().clone();
                view.pages.borrow_mut().remove(&current);
                if *view.current_tab.borrow() != Utf16Text::from("Recent") {
                    let current = view.current_tab.borrow().clone();
                    view.show_tab(current).expect("emoji variant page refresh");
                }
            });
            row.children().add(button);
        }
        if recent {
            let pinned = self
                .engine
                .emoji_history()
                .pinned()
                .lock()
                .clone()
                .contains(&Some(emoji.clone()));
            let theme = self.theme();
            let pin = TouchButton::new();
            pin.set_width(cell);
            pin.set_height(cell);
            pin.set_corner_radius(CornerRadius::uniform(8.0));
            pin.set_scheduler(self.scheduler.clone());
            pin.set_colors(
                theme.action_button_background(),
                theme.action_button_pressed_background(),
            );
            self.icon(
                &pin,
                if pinned { KeyIcon::Unpin } else { KeyIcon::Pin },
                20.0,
                Brushes::get(theme.popup_foreground()).into(),
                if pinned {
                    KeyboardStrings::unpin()
                } else {
                    KeyboardStrings::pin()
                },
            )?;
            let weak_view = self.to_ref().downgrade();
            let value = emoji.clone();
            pin.click(move |_| {
                let Some(view) = weak_view.upgrade() else {
                    return;
                };
                view.close_tone_popup();
                let h = view.engine.emoji_history();
                if pinned {
                    h.unpin(&value)
                } else {
                    h.pin(&value)
                }
                .expect("emoji pin");
                view.show_tab("Recent".into())
                    .expect("emoji recent pin refresh");
            });
            row.children().add(pin);
            let theme = self.theme();
            let remove = TouchButton::new();
            remove.set_width(cell);
            remove.set_height(cell);
            remove.set_corner_radius(CornerRadius::uniform(8.0));
            remove.set_scheduler(self.scheduler.clone());
            remove.set_colors(
                theme.action_button_background(),
                theme.action_button_pressed_background(),
            );
            self.icon(
                &remove,
                KeyIcon::Delete,
                20.0,
                Brushes::get(theme.popup_foreground()).into(),
                KeyboardStrings::remove(),
            )?;
            let weak_view = self.to_ref().downgrade();
            remove.click(move |_| {
                let Some(view) = weak_view.upgrade() else {
                    return;
                };
                view.close_tone_popup();
                let h = view.engine.emoji_history();
                h.remove(&emoji).expect("emoji recent remove");
                h.unpin(&emoji).expect("emoji recent unpin");
                view.show_tab("Recent".into())
                    .expect("emoji recent remove refresh");
            });
            row.children().add(remove);
        }
        let theme = self.theme();
        let popup = Border::new();
        popup.set_child(row.clone());
        popup.set_background(Some(Brushes::get(theme.popup_background()).into()));
        popup.set_border_brush(Some(Brushes::get(theme.popup_border_color()).into()));
        popup.set_border_thickness(Thickness::uniform(1.0));
        popup.set_corner_radius(CornerRadius::uniform(theme.popup_corner_radius()));
        popup.set_padding(Thickness::uniform(2.0));
        *self.tone_popup.borrow_mut() = Some(popup.clone());
        let position = anchor
            .translate_point(Point::new(0.0, 0.0), self)
            .ok_or(ViewError::TransformUnavailable)?;
        let point = popup_position(
            position,
            anchor.bounds().width,
            cell,
            self.bounds().width,
            row.children().count(),
        );
        Canvas::set_left(&popup, point.x);
        Canvas::set_top(&popup, point.y);
        FocusNeutral::apply(&popup);
        self.overlay.children().add(popup);
        Ok(())
    }
    fn is_inside(container: &Visual, mut element: Option<Ref<Visual>>) -> bool {
        while let Some(current) = element {
            if current == container.to_ref() {
                return true;
            }
            element = current.visual_parent();
        }
        false
    }
    fn close_tone_popup(&self) {
        let popup = self.tone_popup.borrow().clone();
        if let Some(popup) = popup {
            self.overlay.children().remove(popup);
            self.tone_popup.borrow_mut().take();
        }
    }
}
fn popup_position(
    position: Point,
    anchor_width: f64,
    cell: f64,
    panel_width: f64,
    count: usize,
) -> Point {
    let width = count as f64 * cell + 6.0;
    let value = position.x + anchor_width / 2.0 - width / 2.0;
    let max = source_max(0.0, panel_width - width);
    let x = if value < 0.0 {
        0.0
    } else if value > max {
        max
    } else {
        value
    };
    Point::new(x, source_max(0.0, position.y - cell - 8.0))
}
#[cfg(test)]
#[path = "emoji_panel_view_tests.rs"]
mod tests;

impl Drop for EmojiPanelView {
    fn drop(&mut self) {
        self.release_subscriptions();
    }
}
