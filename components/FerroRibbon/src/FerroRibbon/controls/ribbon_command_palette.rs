//! Staged RibbonCommandPalette from unchanged RibbonSearchBox.cs. Copyright Wiesław Šoltés, MIT.
//! Native TopLevel/Popup, Rust iterators and closures replace XamlRoot/enumerable/delegate
//! overloads.
use super::ribbon_search_results::RibbonSearchResultsList;
use crate::controls::Ribbon;
use crate::theming::RibbonTheme;
use ferroribbon_core::{
    RibbonString,
    localization::RibbonStrings,
    model::RibbonItemsSource,
    search::{RibbonSearchEngine, RibbonSearchEntry, RibbonSearchResults},
};
use ferroui_base::{
    CornerRadius, Rect, Ref, Thickness,
    input::{Key, KeyboardNavigation, KeyboardNavigationMode},
    styling::ThemeVariant,
    utilities::math_utilities::{max, min},
};
use ferroui_controls::{
    Border, Control, PlacementMode, StackPanel, TextBox, TopLevel,
    automation::AutomationProperties,
    primitives::{
        Popup,
        popup_positioning::{PopupAnchor, PopupGravity, PopupPositionerConstraintAdjustment},
    },
};
use std::{cell::Cell, rc::Rc};
pub type RibbonPaletteExecute = Rc<dyn Fn(Rc<RibbonSearchEntry>)>;
type Search = Rc<dyn Fn(&RibbonString) -> RibbonSearchResults>;
fn text(s: Option<RibbonString>) -> Option<String> {
    s.map(|v| {
        v.to_utf8()
            .expect("native String cannot represent malformed UTF16")
    })
}
pub struct RibbonCommandPalette;
ferroui_base::ferro_markup_type!(static RibbonCommandPalette{namespace:"FerroRibbon.Controls",methods:[static fn Show(Ref<Ribbon>)->Option<Ref<Popup>> =>|ribbon:Ref<Ribbon>|RibbonCommandPalette::show(&ribbon)]});
impl RibbonCommandPalette {
    /// Opens the palette for an attached ribbon. An unattached ribbon has no
    /// native popup host and returns None.
    pub fn show(ribbon: &Ribbon) -> Option<Ref<Popup>> {
        let root = TopLevel::get_top_level(Some(ribbon))?;
        let searched = ribbon.to_ref();
        let execute = ribbon.to_ref();
        Some(Self::show_core(
            root,
            Rc::new(move |query| searched.search(Some(query), 16)),
            Rc::new(move |entry| {
                execute.execute_search_entry(Some(entry));
            }),
            ribbon.actual_theme_variant(),
        ))
    }
    /// Opens a standalone palette. Entries are snapshotted once, retaining the
    /// entry allocations; the Rust callback runs once for an enabled selection.
    pub fn show_entries(
        root: Ref<TopLevel>,
        entries: impl IntoIterator<Item = Rc<RibbonSearchEntry>>,
        execute: impl Fn(Rc<RibbonSearchEntry>) + 'static,
        theme: Option<ThemeVariant>,
    ) -> Ref<Popup> {
        let engine = RibbonSearchEngine::new();
        engine.set_entries(Some(RibbonItemsSource::from_values(
            entries.into_iter().map(Some).collect(),
        )));
        let searched = engine.clone();
        Self::show_core(
            root,
            Rc::new(move |query| searched.search(Some(query), 16)),
            Rc::new(move |entry| {
                engine.mark_used(entry.id());
                execute(entry);
            }),
            theme,
        )
    }
    fn show_core(
        root: Ref<TopLevel>,
        search: Search,
        execute: RibbonPaletteExecute,
        theme: Option<ThemeVariant>,
    ) -> Ref<Popup> {
        RibbonTheme::ensure_resources();
        let previous = root.focus_manager().get_focused_element();
        let results = RibbonSearchResultsList::new();
        // Reuse the existing source Uno single-line/null text adapter.
        let edit = crate::compat::RibbonInputTextBox::new().upcast::<TextBox>();
        let placeholder = text(RibbonStrings::current().command_palette());
        edit.set_placeholder_text(placeholder.as_deref());
        edit.set_margin(Thickness::new(8., 8., 8., 4.));
        AutomationProperties::set_name(&edit, placeholder.as_deref());
        let panel = StackPanel::new();
        panel.children().add(edit.clone());
        panel.children().add(results.root());
        let chrome = Border::new();
        chrome.set_child(panel);
        let width = max(0., min(600., root.client_size().width - 40.));
        chrome.set_width(width);
        chrome.set_border_thickness(Thickness::uniform(1.));
        chrome.set_corner_radius(CornerRadius::uniform(8.));
        chrome.set_value(
            ThemeVariant::requested_theme_variant_property(),
            Some(theme.unwrap_or_default()),
        );
        KeyboardNavigation::set_tab_navigation(&chrome, KeyboardNavigationMode::Cycle);
        RibbonTheme::set_theme_brush(
            Some(&chrome),
            Some(Border::background_property().as_property()),
            Some("RibbonPopupBackgroundBrush"),
        )
        .expect("palette background");
        RibbonTheme::set_theme_brush(
            Some(&chrome),
            Some(Border::border_brush_property().as_property()),
            Some("RibbonPopupBorderBrush"),
        )
        .expect("palette border");
        let popup = Popup::new();
        popup.set_child(chrome);
        popup.set_is_light_dismiss_enabled(true);
        popup.set_popup_parent(Some(&root.clone().upcast::<Control>()));
        popup.set_placement_target(root.clone().upcast::<Control>());
        popup.set_placement(PlacementMode::AnchorAndGravity);
        popup.set_placement_rect(Some(Rect::default()));
        popup.set_placement_anchor(PopupAnchor::TOP | PopupAnchor::LEFT);
        popup.set_placement_gravity(PopupGravity::BOTTOM | PopupGravity::RIGHT);
        popup.set_placement_constraint_adjustment(PopupPositionerConstraintAdjustment::NONE);
        popup.set_horizontal_offset(max(0., (root.client_size().width - width) / 2.));
        popup.set_vertical_offset(72.);
        let executing = Rc::new(Cell::new(false));
        let previous_closed = previous.clone();
        let closing = executing.clone();
        popup
            .closed(move || {
                if !closing.get()
                    && let Some(previous) = previous_closed.as_ref()
                    && previous.visual_root().is_some()
                {
                    previous.focus();
                }
            })
            .detach();
        let weak_popup = popup.downgrade();
        let run: RibbonPaletteExecute = Rc::new(move |entry| {
            if !entry.is_enabled() {
                return;
            }
            if let Some(previous) = previous.as_ref()
                && previous.visual_root().is_some()
            {
                previous.focus();
            }
            executing.set(true);
            if let Some(popup) = weak_popup.upgrade() {
                popup.set_is_open(false);
            }
            execute(entry);
        });
        let invoked = run.clone();
        results.set_invoked(Some(invoked));
        let edit_weak = edit.downgrade();
        let refreshed = results.clone();
        let searched = search.clone();
        edit.text_changing(move |_, _| {
            if let Some(edit) = edit_weak.upgrade() {
                let query = RibbonString::from(edit.text().unwrap_or_default());
                refreshed.show(&searched(&query), Some(&query));
            }
        });
        let navigated = results.clone();
        let closed = popup.downgrade();
        edit.key_down(move |_, e| match e.key {
            Key::Down => {
                navigated.move_highlight(1);
                e.set_handled(true);
            }
            Key::Up => {
                navigated.move_highlight(-1);
                e.set_handled(true);
            }
            Key::Enter => {
                if let Some(entry) = navigated.highlighted() {
                    run(entry);
                }
                e.set_handled(true);
            }
            Key::Escape => {
                if let Some(popup) = closed.upgrade() {
                    popup.set_is_open(false);
                }
                e.set_handled(true);
            }
            _ => {}
        });
        let query = RibbonString::from("");
        results.show(&search(&query), Some(&query));
        popup.set_is_open(true);
        edit.focus();
        popup
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroribbon_core::search::RibbonSearchEntry;
    use ferroui_base::{
        Size,
        threading::{CancellationToken, Dispatcher},
    };
    use ferroui_controls::{Application, Window};
    use ferroui_headless::FerroHeadlessPlatformExtensions;
    fn run(f: impl FnOnce() + Send + 'static) {
        ferroui_headless::HeadlessUnitTestSession::start_new(|| {
            ferroui_controls::AppBuilder::configure::<Application>()
                .use_headless(Default::default())
        })
        .dispatch(f, CancellationToken::none())
        .wait()
        .unwrap();
    }
    fn entries() -> Vec<Rc<RibbonSearchEntry>> {
        let a = RibbonSearchEntry::new(Some("a".into()), Some("Alpha".into()));
        let b = RibbonSearchEntry::new(Some("b".into()), Some("Beta".into()));
        vec![a, b]
    }
    #[test]
    fn unattached_ribbon_does_not_open_a_palette() {
        run(|| {
            crate::register_types();
            assert!(RibbonCommandPalette::show(&Ribbon::new()).is_none());
        });
    }
    #[test]
    fn original_palette_geometry_and_native_popup_ownership() {
        run(|| {
            crate::register_types();
            for (width, want, offset) in [(20., 0., 10.), (400., 360., 20.), (1000., 600., 200.)] {
                let theme = ferroui_themes_simple::SimpleTheme::new().as_style();
                let app = Application::current().unwrap();
                app.styles().add(theme.clone());
                let w = Window::new();
                w.set_width(width);
                w.set_height(600.);
                w.show();
                w.measure(Size::new(width, 600.));
                w.arrange(Rect::new(0., 0., width, 600.));
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(w.client_size().width, width);
                RibbonCommandPalette::show_entries(w.clone().upcast(), entries(), |_| {}, None);
                let popups = w.opened_popups();
                assert_eq!(
                    popups.len(),
                    1,
                    "static void Show must keep its actual native popup alive"
                );
                let popup = popups[0].clone();
                let chrome = popup.child().unwrap().cast::<Border>().unwrap();
                assert_eq!(chrome.width(), want);
                assert_eq!(popup.horizontal_offset(), offset);
                assert_eq!(popup.vertical_offset(), 72.);
                assert!(popup.is_light_dismiss_enabled());
                assert_eq!(
                    KeyboardNavigation::get_tab_navigation(&chrome),
                    KeyboardNavigationMode::Cycle
                );
                popup.set_is_open(false);
                assert_eq!(w.opened_popups().len(), 0);
                w.close();
                app.styles().remove(theme);
            }
        });
    }

    #[test]
    fn palette_keyboard_navigation_invokes_once_and_closes_actual_popup() {
        run(|| {
            crate::register_types();
            let app = Application::current().unwrap();
            let theme = ferroui_themes_simple::SimpleTheme::new().as_style();
            app.styles().add(theme.clone());
            let w = Window::new();
            w.set_width(800.);
            w.set_height(500.);
            w.show();
            w.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            let calls = Rc::new(std::cell::RefCell::new(Vec::new()));
            let seen = calls.clone();
            RibbonCommandPalette::show_entries(
                w.clone().upcast(),
                entries(),
                move |e| seen.borrow_mut().push(e.id().unwrap().to_utf8().unwrap()),
                None,
            );
            w.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            let popup = w.opened_popups()[0].clone();
            let panel = popup
                .child()
                .unwrap()
                .cast::<Border>()
                .unwrap()
                .child()
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let edit = panel.children().get(0).cast::<TextBox>().unwrap();
            // Source empty query is recent-only: a new standalone engine has no rows.
            edit.set_text(Some("a"));
            Dispatcher::ui_thread().run_jobs(None);
            for key in [Key::Down, Key::Enter] {
                let mut e = ferroui_base::input::KeyEventArgs::new();
                e.set_routed_event(Some(ferroui_base::input::InputElement::key_down_event()));
                e.key = key;
                edit.raise_event(&e);
                assert!(e.handled());
            }
            assert_eq!(&*calls.borrow(), &["b"]);
            assert!(!popup.is_open());
            assert_eq!(w.opened_popups().len(), 0);
            w.close();
            app.styles().remove(theme);
        });
    }

    #[test]
    fn palette_enter_uses_current_query_before_async_text_changed() {
        run(|| {
            crate::register_types();
            let app = Application::current().unwrap();
            let theme = ferroui_themes_simple::SimpleTheme::new().as_style();
            app.styles().add(theme.clone());
            let window = Window::new();
            window.set_width(800.);
            window.set_height(500.);
            window.show();
            window.update_layout();
            let invoked = Rc::new(std::cell::RefCell::new(Vec::new()));
            let observed = invoked.clone();
            let popup = RibbonCommandPalette::show_entries(
                window.clone().upcast(),
                entries(),
                move |entry| observed.borrow_mut().push(entry.id()),
                None,
            );
            window.update_layout();
            Dispatcher::ui_thread().run_jobs(None);
            let edit = popup
                .child()
                .unwrap()
                .cast::<Border>()
                .unwrap()
                .child()
                .unwrap()
                .cast::<StackPanel>()
                .unwrap()
                .children()
                .get(0)
                .cast::<TextBox>()
                .unwrap();
            edit.set_text(Some("Alpha"));
            Dispatcher::ui_thread().run_jobs(None);

            edit.set_text(Some("Beta"));
            let mut event = ferroui_base::input::KeyEventArgs::new();
            event.key = Key::Enter;
            event.set_routed_event(Some(ferroui_base::input::InputElement::key_down_event()));
            edit.raise_event(&event);
            assert!(event.handled());
            assert_eq!(&*invoked.borrow(), &[Some("b".into())]);
            assert!(!popup.is_open());
            window.close();
            app.styles().remove(theme);
        });
    }
}
