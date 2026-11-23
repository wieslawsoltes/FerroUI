//! Native staged internal RibbonSearchResultsList from unchanged RibbonSearchBox.cs. Copyright
//! Wiesław Šoltés, MIT. Source row visuals are themselves programmatic.
use crate::controls::{IRibbonItem, RibbonBackstageItem};
use crate::primitives::RibbonIconPresenter;
use crate::theming::RibbonTheme;
#[cfg(test)]
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::{
    RibbonString,
    commands::RibbonCommandDescriptor,
    localization::RibbonStrings,
    search::{RibbonSearchEntry, RibbonSearchResults},
    text::trim,
};
use ferroui_base::{
    BoxedValue, Ref, Thickness,
    data::core::{ValueType, ValueTypes},
    input::KeyboardNavigation,
    layout::{HorizontalAlignment, VerticalAlignment},
    media::{Colors, FontWeight, IBrush, SolidColorBrush, TextTrimming},
    styling::ControlTheme,
    threading::{Dispatcher, DispatcherPriority},
};
use ferroui_controls::{
    Application, Border, Button, ColumnDefinitions, Control, Grid, ScrollViewer, StackPanel,
    TextBlock,
    automation::{AutomationLiveSetting, AutomationProperties},
    primitives::ScrollBarVisibility,
};
#[cfg(test)]
use ferroui_controls::automation::{AutomationElementIdentifiers, peers::ControlAutomationPeer};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
fn text(value: Option<RibbonString>) -> Option<String> {
    value.map(|s| {
        s.to_utf8()
            .expect("Native String cannot represent an unpaired UTF16 surrogate")
    })
}
fn cast<T: ferroui_base::PropertyValue>(value: &BoxedValue) -> Option<T> {
    ValueTypes::try_cast(value, ValueType::of::<T>()).and_then(|v| v.downcast_ref::<T>().cloned())
}
fn icon(entry: &Rc<RibbonSearchEntry>) -> Option<BoxedValue> {
    let target =
        crate::metadata::model_objects::to_native(entry.target()).expect("native search target")?;
    if let Some(item) = cast::<Rc<dyn IRibbonItem>>(&target) {
        return item.icon();
    }
    if let Some(item) =
        ValueTypes::as_object(&*target).and_then(|v| v.cast::<RibbonBackstageItem>())
    {
        return item.icon();
    }
    cast::<Rc<RibbonCommandDescriptor>>(&target)
        .and_then(|v| v.icon())
        .map(|v| v as BoxedValue)
}
type ResultInvoked = Rc<dyn Fn(Rc<RibbonSearchEntry>)>;
pub struct RibbonSearchResultsList {
    root: Ref<ScrollViewer>,
    panel: Ref<StackPanel>,
    rows: RefCell<Vec<(Ref<Button>, Rc<RibbonSearchEntry>)>>,
    highlight: Cell<i32>,
    invoked: RefCell<Option<ResultInvoked>>,
    announcement_generation: Cell<u64>,
}
impl RibbonSearchResultsList {
    pub fn new() -> Rc<Self> {
        let panel = StackPanel::new();
        panel.set_spacing(0.);
        // WinUI StackPanel.Padding has no native Panel equivalent: a padding-only
        // Border retains the same content/scroll extent without recreating row controls.
        let padding = Border::new();
        padding.set_padding(Thickness::uniform(4.));
        padding.set_child(panel.clone());
        let root = ScrollViewer::new();
        root.set_content(Some(Control::boxed(padding)));
        root.set_max_height(420.);
        root.set_vertical_scroll_bar_visibility(ScrollBarVisibility::Auto);
        root.set_horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled);
        // A native live region announces result changes without changing the
        // query editor's accessible name or moving keyboard focus into the rows.
        AutomationProperties::set_live_setting(&root, AutomationLiveSetting::Polite);
        let this = Rc::new(Self {
            root: root.clone(),
            panel,
            rows: RefCell::default(),
            highlight: Cell::new(-1),
            invoked: RefCell::default(),
            announcement_generation: Cell::new(0),
        });
        let weak = Rc::downgrade(&this);
        root.actual_theme_variant_changed(move || {
            if let Some(this) = weak.upgrade() {
                this.highlight(this.highlight.get());
            }
        })
        .detach();
        this
    }
    pub fn root(&self) -> Ref<ScrollViewer> {
        self.root.clone()
    }
    pub fn count(&self) -> i32 {
        i32::try_from(self.rows.borrow().len()).expect("Int32 Count")
    }
    pub fn highlighted(&self) -> Option<Rc<RibbonSearchEntry>> {
        usize::try_from(self.highlight.get())
            .ok()
            .and_then(|i| self.rows.borrow().get(i).map(|r| r.1.clone()))
            .filter(|e| e.is_enabled())
    }
    pub fn set_invoked(&self, callback: Option<ResultInvoked>) {
        self.invoked.replace(callback);
    }
    pub fn show(self: &Rc<Self>, results: &RibbonSearchResults, query: Option<&RibbonString>) {
        self.announcement_generation
            .set(self.announcement_generation.get().wrapping_add(1));
        AutomationProperties::set_name(&self.root, None);
        self.panel.children().clear();
        self.rows.borrow_mut().clear();
        let strings = RibbonStrings::current();
        let blank = query.is_none_or(|q| trim(q.as_utf16()).is_empty());
        if results.is_empty() {
            let label = TextBlock::new();
            label.set_text(
                text(if blank {
                    strings.search_recent()
                } else {
                    strings.search_no_results()
                })
                .as_deref(),
            );
            label.set_margin(Thickness::new(8., 6., 8., 6.));
            label.set_opacity(0.7);
            self.panel.children().add(label);
            self.highlight.set(-1);
            return;
        }
        let label = TextBlock::new();
        label.set_text(
            text(if blank {
                strings.search_recent()
            } else {
                strings.search_actions()
            })
            .as_deref(),
        );
        label.set_font_size(12.);
        label.set_font_weight(FontWeight::SemiBold);
        label.set_margin(Thickness::new(8., 4., 8., 4.));
        self.panel.children().add(label);
        let size = i32::try_from(results.len()).expect("Int32 Count");
        let mut position = 0;
        for result in results.iter() {
            let entry = result
                .expect("NullReference: result.Entry")
                .entry()
                .expect("NullReference: entry.Target");
            let grid = Grid::new();
            grid.set_column_spacing(10.);
            grid.set_column_definitions(
                ColumnDefinitions::parse("20,*,Auto").expect("source fixed column definitions"),
            );
            let presenter = RibbonIconPresenter::new();
            presenter.set_icon(Some(icon(&entry).unwrap_or_else(|| Rc::new(String::new()))));
            presenter.set_icon_size(16.);
            RibbonTheme::set_theme_brush(
                Some(&presenter),
                Some(RibbonIconPresenter::foreground_property().as_property()),
                Some("RibbonIconBrush"),
            )
            .expect("source icon theme brush");
            grid.children().add(presenter);
            let words = StackPanel::new();
            let label = TextBlock::new();
            label.set_text(text(entry.label()).as_deref());
            label.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
            words.children().add(label);
            if let Some(path) = text(entry.path()).filter(|v| !v.is_empty()) {
                let label = TextBlock::new();
                label.set_text(Some(&path));
                label.set_font_size(11.);
                label.set_opacity(0.65);
                label.set_text_trimming(<dyn TextTrimming>::character_ellipsis());
                words.children().add(label);
            }
            Grid::set_column(&words, 1);
            grid.children().add(words);
            if let Some(shortcut) = text(entry.shortcut()).filter(|v| !v.is_empty()) {
                let label = TextBlock::new();
                label.set_text(Some(&shortcut));
                label.set_font_size(11.);
                label.set_opacity(0.65);
                label.set_vertical_alignment(VerticalAlignment::Center);
                Grid::set_column(&label, 2);
                grid.children().add(label);
            }
            let button = Button::new();
            button.set_content(Some(Control::boxed(grid)));
            button.set_is_enabled(entry.is_enabled());
            button.set_horizontal_alignment(HorizontalAlignment::Stretch);
            button.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
            KeyboardNavigation::set_is_tab_stop(&button, false);
            let app = Application::current().expect("NullReference: Application.Current");
            if let Some(style) = app
                .resources()
                .try_get_resource(&"RibbonMenuItemButtonStyle".into(), None)
            {
                button.set_theme(style.map(|v| {
                    ValueTypes::as_object(&*v)
                        .and_then(|o| o.cast::<ControlTheme>())
                        .expect("RibbonMenuItemButtonStyle must be ControlTheme")
                }));
            }
            AutomationProperties::set_name(&button, text(entry.label()).as_deref());
            position += 1;
            AutomationProperties::set_position_in_set(&button, position);
            AutomationProperties::set_size_of_set(&button, size);
            if let Some(path) = text(entry.path()).filter(|s| !s.is_empty()) {
                AutomationProperties::set_help_text(&button, Some(&path));
            }
            let weak = Rc::downgrade(self);
            let captured = entry.clone();
            button.click(move |_, _| {
                if captured.is_enabled()
                    && let Some(this) = weak.upgrade()
                {
                    let invoked = this.invoked.borrow().clone();
                    if let Some(invoked) = invoked {
                        invoked(captured.clone());
                    }
                }
            });
            self.rows.borrow_mut().push((button.clone(), entry));
            self.panel.children().add(button);
        }
        let first = self
            .rows
            .borrow()
            .iter()
            .position(|r| r.1.is_enabled())
            .map(|i| i as i32)
            .unwrap_or(-1);
        self.highlight(first);
    }
    pub fn move_highlight(self: &Rc<Self>, delta: i32) {
        if self.count() == 0 || delta == 0 {
            return;
        }
        let step = delta.signum();
        let mut index = self.highlight.get() + step;
        while index >= 0 && index < self.count() {
            let enabled = self.rows.borrow()[index as usize].1.is_enabled();
            if enabled {
                self.highlight(index);
                return;
            }
            index += step;
        }
    }
    fn highlight(self: &Rc<Self>, index: i32) {
        let changed = index != self.highlight.get();
        self.highlight
            .set(if self.count() == 0 { -1 } else { index });
        let brush = RibbonTheme::get_brush(Some(&self.root), Some("RibbonItemHoverBrush"))
            .expect("source highlight brush");
        // Source for-loop reads Count and current highlight each iteration. Do not
        // retain a RefCell borrow across property callbacks that can reenter Show.
        let mut i = 0usize;
        loop {
            let button = self.rows.borrow().get(i).map(|r| r.0.clone());
            let Some(button) = button else {
                break;
            };
            let background: Rc<dyn IBrush> = if i as i32 == self.highlight.get()
                && let Some(brush) = brush.clone()
            {
                brush.into()
            } else {
                SolidColorBrush::with_color(Colors::TRANSPARENT).into()
            };
            button.set_background(Some(background));
            i += 1;
        }
        if self.highlight.get() >= 0 && self.highlight.get() < self.count() {
            let button = self.rows.borrow()[self.highlight.get() as usize].0.clone();
            button.bring_into_view();
            if changed {
                let entry = self.rows.borrow()[self.highlight.get() as usize].1.clone();
                self.announce(&entry);
            }
        }
    }
    fn announce(self: &Rc<Self>, entry: &RibbonSearchEntry) {
        let label = text(entry.label()).unwrap_or_default();
        let name = match text(entry.path()).filter(|path| !path.is_empty()) {
            Some(path) => format!("{label}, {path}"),
            None => label,
        };
        let generation = self.announcement_generation.get().wrapping_add(1);
        self.announcement_generation.set(generation);
        let weak = Rc::downgrade(self);
        // Show builds the results before opening their popup. Dispatch after
        // attachment and coalesce rapid changes to the latest highlighted row.
        Dispatcher::ui_thread().post_local(
            move || {
                let Some(this) = weak.upgrade() else { return };
                if this.announcement_generation.get() != generation
                    || this.root.visual_root().is_none()
                {
                    return;
                }
                // The peer reports the change of the name, and the native backend
                // turns that live-region event into speech.
                AutomationProperties::set_name(&this.root, Some(&name));
            },
            DispatcherPriority::NORMAL,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroribbon_core::search::{RibbonSearchEntryOptions, RibbonSearchResult};
    use ferroui_base::{interactivity::RoutedEventArgs, threading::CancellationToken};
    use ferroui_headless::FerroHeadlessPlatformExtensions;
    mod source {
        include!("results_fixture.rs");
    }
    fn run(f: impl FnOnce() + Send + 'static) {
        ferroui_headless::HeadlessUnitTestSession::start_new(|| {
            ferroui_controls::AppBuilder::configure::<ferroui_controls::Application>()
                .use_headless(Default::default())
        })
        .dispatch(f, CancellationToken::none())
        .wait()
        .unwrap();
    }
    fn fixture() -> RibbonSearchResults {
        let disabled = RibbonSearchEntry::with_options(
            Some("x".into()),
            Some("Disabled".into()),
            RibbonSearchEntryOptions {
                is_enabled: false,
                ..Default::default()
            },
        );
        let a = RibbonSearchEntry::with_options(
            Some("a".into()),
            Some("Alpha".into()),
            RibbonSearchEntryOptions {
                path: Some("Home".into()),
                shortcut: Some("Alt+A".into()),
                ..Default::default()
            },
        );
        let b = RibbonSearchEntry::new(Some("b".into()), Some("Beta".into()));
        RibbonFixedItems::new(vec![
            Some(RibbonSearchResult::new(Some(disabled.clone()), 4.)),
            Some(RibbonSearchResult::new(Some(a), 3.)),
            Some(RibbonSearchResult::new(Some(disabled), 2.)),
            Some(RibbonSearchResult::new(Some(b), 1.)),
        ])
    }
    #[test]
    fn original_results_navigation_disabled_skip_and_native_row_accessibility() {
        run(|| {
            crate::register_types();
            RibbonTheme::ensure_resources();
            let list = RibbonSearchResultsList::new();
            list.show(&fixture(), Some(&"a".into()));
            assert_eq!(list.count(), 4);
            let mut moves = vec![text(list.highlighted().unwrap().id()).unwrap()];
            for delta in [1, 1, -1, -1, 0, -999, 999] {
                list.move_highlight(delta);
                moves.push(text(list.highlighted().unwrap().id()).unwrap());
            }
            assert_eq!(moves, source::MOVES);
            let buttons: Vec<_> = list.rows.borrow().iter().map(|r| r.0.clone()).collect();
            assert_eq!(
                buttons
                    .iter()
                    .map(|b| AutomationProperties::get_position_in_set(b))
                    .collect::<Vec<_>>(),
                source::POSITIONS
            );
            assert_eq!(
                buttons
                    .iter()
                    .map(|b| AutomationProperties::get_size_of_set(b))
                    .collect::<Vec<_>>(),
                source::SIZES
            );
            assert_eq!(
                buttons
                    .iter()
                    .map(|b| AutomationProperties::get_name(b).unwrap())
                    .collect::<Vec<_>>(),
                source::NAMES
            );
            assert!(
                buttons
                    .iter()
                    .all(|b| !KeyboardNavigation::get_is_tab_stop(b))
            );
            assert!(
                buttons.iter().all(|b| b.focusable()),
                "IsTabStop does not disable pointer/programmatic focus"
            );
            for b in [&buttons[0], &buttons[2]] {
                assert_eq!(
                    b.background()
                        .unwrap()
                        .as_solid_color_brush()
                        .unwrap()
                        .color()
                        .to_uint32(),
                    0x00ffffff,
                    "source Transparent is transparent white"
                );
            }
            let calls = Rc::new(RefCell::new(Vec::new()));
            let observed = calls.clone();
            list.set_invoked(Some(Rc::new(move |entry| {
                observed.borrow_mut().push(text(entry.id()).unwrap());
            })));
            for b in &buttons {
                b.raise_event(&RoutedEventArgs::with_event(Button::click_event()));
            }
            assert_eq!(&*calls.borrow(), source::INVOKED);
            list.set_invoked(None);
            buttons[1].raise_event(&RoutedEventArgs::with_event(Button::click_event()));
            assert_eq!(&*calls.borrow(), source::INVOKED);
        });
    }
    #[test]
    fn native_live_region_coalesces_highlight_announcements_without_moving_query_focus() {
        run(|| {
            crate::register_types();
            RibbonTheme::ensure_resources();
            let app = Application::current().unwrap();
            let theme = ferroui_themes_simple::SimpleTheme::new().as_style();
            app.styles().add(theme.clone());
            let list = RibbonSearchResultsList::new();
            let query = ferroui_controls::TextBox::new();
            AutomationProperties::set_name(&query, Some("Search"));
            let panel = StackPanel::new();
            panel.children().add(query.clone());
            panel.children().add(list.root());
            let window = ferroui_controls::Window::new();
            window.set_content(Some(Control::boxed(panel)));
            window.show();
            window.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            assert!(query.focus());
            let peer = ControlAutomationPeer::create_peer_for_element(&list.root());
            assert_eq!(peer.get_live_setting(), AutomationLiveSetting::Polite);
            let messages = Rc::new(RefCell::new(Vec::new()));
            let observed = messages.clone();
            let token = peer.property_changed(move |event| {
                if std::ptr::eq(
                    event.property(),
                    AutomationElementIdentifiers::name_property(),
                ) {
                    observed.borrow_mut().push(
                        event
                            .new_value()
                            .unwrap()
                            .downcast_ref::<String>()
                            .unwrap()
                            .clone(),
                    );
                }
            });
            list.show(&fixture(), Some(&"a".into()));
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(&*messages.borrow(), &["Alpha, Home"]);
            list.move_highlight(1);
            list.move_highlight(-1);
            list.move_highlight(1);
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(&*messages.borrow(), &["Alpha, Home", "Beta"]);
            assert!(query.is_focused());
            assert_eq!(
                AutomationProperties::get_name(&query).as_deref(),
                Some("Search")
            );
            list.move_highlight(-1);
            list.show(&RibbonFixedItems::new(vec![]), Some(&"missing".into()));
            Dispatcher::ui_thread().run_jobs(None);
            // Clearing the name is reported with the empty name; the pending result is not.
            assert_eq!(
                &*messages.borrow(),
                &["Alpha, Home", "Beta", ""],
                "stale pending result must not be announced"
            );
            assert_eq!(
                peer.get_name(),
                "",
                "empty results must not retain a stale result name"
            );
            token.dispose();
            window.close();
            app.styles().remove(theme);
        });
    }
    #[test]
    fn original_empty_results_headings_and_all_disabled_navigation() {
        run(|| {
            crate::register_types();
            RibbonTheme::ensure_resources();
            let list = RibbonSearchResultsList::new();
            for (query, want) in [(" ", source::RECENT), ("z", source::EMPTY)] {
                list.show(&RibbonFixedItems::new(vec![]), Some(&query.into()));
                assert_eq!(list.count(), 0);
                assert!(list.highlighted().is_none());
                let label = list.panel.children().get(0).cast::<TextBlock>().unwrap();
                assert_eq!(label.text().as_deref(), Some(want));
            }
            let disabled = fixture().get(0);
            list.show(&RibbonFixedItems::new(vec![disabled]), None);
            for delta in [1, -1, i32::MIN, i32::MAX, 0] {
                list.move_highlight(delta);
                assert!(list.highlighted().is_none());
            }
            assert_eq!(list.count(), 1);
            assert_eq!(list.root().max_height(), 420.);
        });
    }
}
