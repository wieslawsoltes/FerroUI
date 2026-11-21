//! Fifteen pinned CAD scenarios on native controls and themes. The exact-width reduction
//! scenario runs with real text services in the native sample tests. Copyright Wiesław Šoltés,
//! MIT.
use crate::compat::{FontIcon, ImageIcon};
use crate::{
    controls::*,
    primitives::RibbonIconPresenter,
    runtime_sample::{self as sample, Mounted, RuntimeHost},
    theming::{RibbonTheme, RibbonThemeResources},
};
use ferroribbon_core::{
    RibbonGroupState, RibbonMinimizeBehavior, RibbonVisibilityMode,
    theming::{RibbonChromeStyle, RibbonThemePalette, RibbonThemeStyle},
};
use ferroui_base::{
    BoxedValue, CornerRadius, Ref, Visual,
    animation::SignedDuration,
    controls::ResourceDictionary,
    data::core::ValueTypes,
    media::{Color, SolidColorBrush},
    metadata::MarkupDelegate,
    platform::{AssetLoader, register_assets},
    styling::ThemeVariant,
    threading::{Dispatcher, DispatcherFrame, DispatcherPriority, DispatcherTimer},
};
use ferroui_controls::{
    Button, Control, PathIcon, StackPanel, TextBlock, TopLevel,
    automation::peers::ButtonAutomationPeer, presenters::ContentPresenter, shapes::Path,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

fn descendants(root: &Control) -> Vec<Ref<Visual>> {
    let mut result = Vec::new();
    let mut pending = root
        .visual_children()
        .snapshot()
        .iter()
        .rev()
        .cloned()
        .collect::<Vec<_>>();
    while let Some(visual) = pending.pop() {
        assert!(
            result.iter().all(|old: &Ref<Visual>| !old.ptr_eq(&visual)),
            "repeated visual"
        );
        pending.extend(visual.visual_children().snapshot().iter().rev().cloned());
        result.push(visual);
    }
    result
}
fn named<T: ferroui_base::ObjectType>(root: &Control, name: &str) -> Ref<T> {
    descendants(root)
        .into_iter()
        .find_map(|v| {
            v.cast::<Control>()
                .filter(|c| c.name().as_deref() == Some(name))
                .and_then(|c| c.cast())
        })
        .unwrap()
}
fn pump(duration: Duration) {
    let frame = DispatcherFrame::new();
    let end = frame.clone();
    let stop = DispatcherTimer::run_once(
        move || end.set_continue(false),
        duration,
        DispatcherPriority::NORMAL,
    );
    Dispatcher::current_dispatcher().push_frame(&frame);
    stop.dispose();
}
fn with_slide_out() -> (Ref<Ribbon>, Ref<RibbonGroup>) {
    let ribbon = sample::create();
    let group = sample::find_group(&ribbon, "clipboard");
    for (id, label, icon) in [
        ("pasteSpecial", "Paste Special", "\u{e77f}"),
        ("clipboardHistory", "Clipboard History", "\u{e81c}"),
    ] {
        let button = RibbonButton::new();
        button.set_id(Some(id.into()));
        button.set_label(Some(label.into()));
        button.set_icon(Some(Rc::new(icon.to_owned())));
        group.slide_out_items().add(Some(button.upcast()));
    }
    (ribbon, group)
}
fn radius(resources: &RibbonThemeResources, variant: ThemeVariant) -> CornerRadius {
    let dictionary = resources
        .theme_dictionary(&variant)
        .unwrap()
        .as_object()
        .unwrap()
        .to_ref()
        .cast::<ResourceDictionary>()
        .unwrap();
    *dictionary
        .get(&"RibbonCommandBarCornerRadius".into())
        .unwrap()
        .downcast_ref::<CornerRadius>()
        .unwrap()
}
struct RestoreTheme;
impl Drop for RestoreTheme {
    fn drop(&mut self) {
        RibbonTheme::apply_with_style(
            Some(RibbonThemePalette::word()),
            RibbonChromeStyle::Neutral,
            RibbonThemeStyle::Office,
        )
        .unwrap();
        RibbonTheme::reset_test_cache();
    }
}
#[test]
fn cad_style_switches_surfaces_and_back() {
    crate::group_tests::run(|| {
        let _restore = RestoreTheme;
        let mut ribbon = None;
        let host = RuntimeHost::with_setup(
            || {
                let r = sample::create();
                ribbon = Some(r.clone());
                r.upcast()
            },
            Some(1400.),
            || {
                RibbonTheme::reset_test_cache();
                RibbonTheme::apply_with_style(
                    Some(RibbonThemePalette::cad()),
                    RibbonChromeStyle::Neutral,
                    RibbonThemeStyle::Cad,
                )
                .unwrap();
            },
        );
        let resources = RibbonTheme::resources();
        assert_eq!(RibbonTheme::style(), RibbonThemeStyle::Cad);
        assert_eq!(
            resources
                .get_brush("RibbonCommandBarBackgroundBrush", Some("Dark"))
                .unwrap()
                .color(),
            Color::from_argb(255, 0x3b, 0x44, 0x53)
        );
        assert_eq!(
            resources
                .get_brush("RibbonGroupCaptionBackgroundBrush", Some("Dark"))
                .unwrap()
                .color(),
            Color::from_argb(255, 0x32, 0x3a, 0x47)
        );
        assert_eq!(
            radius(&resources, ThemeVariant::dark()),
            CornerRadius::uniform(0.)
        );
        assert!(ribbon.unwrap().bounds().height > 0.);
        RibbonTheme::apply_with_style(
            Some(RibbonThemePalette::word()),
            RibbonChromeStyle::Neutral,
            RibbonThemeStyle::Office,
        )
        .unwrap();
        host.settle();
        assert_eq!(
            resources
                .get_brush("RibbonGroupCaptionBackgroundBrush", Some("Light"))
                .unwrap()
                .color(),
            Color::from_argb(0, 255, 255, 255)
        );
        assert_eq!(
            radius(&resources, ThemeVariant::light()),
            CornerRadius::uniform(8.)
        );
    });
}
#[test]
fn expanded_panel_opens_pins_and_returns_with_its_tab() {
    crate::group_tests::run(|| {
        let (r, group) = with_slide_out();
        let m = Mounted::with_ribbon(r, 1400.);
        assert!(group.slide_out_button().unwrap().is_visible());
        assert!(
            m.ribbon
                .find_item(Some("pasteSpecial".into()))
                .unwrap()
                .is::<RibbonButton>()
        );
        group.open_slide_out();
        m.settle();
        assert!(group.is_slide_out_open());
        group.set_is_slide_out_pinned(true);
        m.ribbon.select_tab_id("insert");
        m.settle();
        assert!(!group.is_slide_out_open());
        m.ribbon.select_tab_id("home");
        m.settle();
        pump(Duration::from_millis(400));
        assert!(group.is_slide_out_open());
        group.set_is_slide_out_pinned(false);
        group.close_slide_out();
        m.settle();
        assert!(!group.is_slide_out_open());
    });
}
#[test]
fn collapsed_group_popup_includes_the_expanded_part() {
    crate::group_tests::run(|| {
        let (r, group) = with_slide_out();
        let m = Mounted::with_ribbon(r, 1400.);
        m.ribbon
            .set_visibility_mode(RibbonVisibilityMode::PanelButtons);
        m.settle();
        group.open_popup();
        m.settle();
        assert!(group.is_popup_open());
        let expanded = sample::item::<RibbonButton>(&m.ribbon, "pasteSpecial");
        assert!(TopLevel::get_top_level(Some(&expanded)).is_some());
        group.close_popup();
    });
}
#[test]
fn minimize_states_cycle_like_autocad() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let full = m.ribbon.bounds().height;
        m.ribbon
            .set_minimize_behavior(RibbonMinimizeBehavior::CycleAll);
        m.ribbon.toggle_minimized();
        m.settle();
        assert_eq!(
            m.ribbon.visibility_mode(),
            RibbonVisibilityMode::PanelButtons
        );
        assert!(m.ribbon.is_minimized());
        let font = sample::find_group(&m.ribbon, "font");
        assert_eq!(font.state(), RibbonGroupState::Collapsed);
        assert!(font.collapsed_button().unwrap().is_visible());
        m.ribbon.toggle_minimized();
        m.settle();
        assert_eq!(
            m.ribbon.visibility_mode(),
            RibbonVisibilityMode::PanelTitles
        );
        assert!(font.collapsed_button().unwrap().icon().is_none());
        assert!(m.ribbon.bounds().height < full - 20.);
        m.ribbon.toggle_minimized();
        m.settle();
        assert_eq!(m.ribbon.visibility_mode(), RibbonVisibilityMode::TabsOnly);
        m.ribbon.toggle_minimized();
        m.settle();
        assert_eq!(m.ribbon.visibility_mode(), RibbonVisibilityMode::AlwaysShow);
        assert_ne!(font.state(), RibbonGroupState::Collapsed);
        m.ribbon
            .set_minimize_behavior(RibbonMinimizeBehavior::PanelTitles);
        m.ribbon.toggle_minimized();
        assert_eq!(
            m.ribbon.visibility_mode(),
            RibbonVisibilityMode::PanelTitles
        );
        m.ribbon.toggle_minimized();
        assert_eq!(m.ribbon.visibility_mode(), RibbonVisibilityMode::AlwaysShow);
    });
}
#[test]
fn panel_titles_can_be_hidden() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let full = m.ribbon.bounds().height;
        m.ribbon.set_show_group_captions(false);
        m.settle();
        assert!(m.ribbon.bounds().height < full);
        assert!(!m.ribbon.get_state().unwrap().show_group_captions());
        m.ribbon.set_show_group_captions(true);
        m.settle();
        assert_eq!(m.ribbon.bounds().height, full);
    });
}
#[test]
fn panels_float_persist_and_return() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        m.ribbon.set_can_float_groups(true);
        let group = sample::find_group(&m.ribbon, "paragraph");
        group.float(Some(ferroui_base::Point::new(80., 320.)));
        m.settle();
        assert!(group.is_floating() && group.is_floating_panel_open());
        assert!(!group.is_visible());
        assert!(
            m.ribbon
                .floating_groups()
                .iter()
                .flatten()
                .any(|g| g.ptr_eq(&group))
        );
        assert!(
            TopLevel::get_top_level(Some(&sample::item::<RibbonToggleButton>(
                &m.ribbon, "center"
            )))
            .is_some()
        );
        m.ribbon.select_tab_id("insert");
        m.settle();
        assert!(group.is_floating_panel_open());
        let json = m.ribbon.save_state_to_json().unwrap();
        m.ribbon.return_all_panels_to_ribbon();
        m.ribbon.select_tab_id("home");
        m.settle();
        assert!(!group.is_floating() && !group.is_floating_panel_open());
        assert!(group.is_visible());
        let restored = Mounted::new(1400.);
        assert!(restored.ribbon.load_state_from_json(Some(json)).unwrap());
        restored.settle();
        let next = sample::find_group(&restored.ribbon, "paragraph");
        assert!(next.is_floating());
        assert_eq!(next.floating_position().x, 80.);
        next.return_to_ribbon();
    });
}
#[test]
fn visibility_menus_list_tabs_and_panels() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        m.ribbon.set_is_visibility_menu_enabled(true);
        let tabs = m.ribbon.build_show_tabs_menu().unwrap();
        assert!(
            tabs.items()
                .iter()
                .flatten()
                .filter_map(|v| v.cast::<crate::compat::ToggleMenuFlyoutItem>())
                .any(|i| i.text().as_deref() == Some("Insert") && i.is_checked())
        );
        assert_eq!(
            m.ribbon
                .build_show_panels_menu(m.ribbon.find_tab("home").unwrap())
                .unwrap()
                .items()
                .count(),
            5
        );
        assert_eq!(m.ribbon.build_minimize_behavior_menu().items().count(), 4);
    });
}
#[test]
fn progressive_tooltip_extends_after_the_delay() {
    crate::group_tests::run(|| {
        struct Reset(SignedDuration);
        impl Drop for Reset {
            fn drop(&mut self) {
                RibbonScreenTipService::set_extended_delay(self.0);
            }
        }
        let _reset = Reset(RibbonScreenTipService::extended_delay());
        RibbonScreenTipService::set_extended_delay(SignedDuration::from_ticks(9_000_000));
        let tip = RibbonScreenTip::new();
        tip.set_title(Some("Line".into()));
        tip.set_description(Some("Creates straight line segments.".into()));
        tip.set_extended_description(Some("Specify points to draw connected segments.".into()));
        tip.set_extended_image(Some(Rc::new("\u{e8c6}".to_string())));
        let host = RuntimeHost::new(tip.clone().upcast(), Some(400.));
        assert!(!tip.is_extended());
        pump(Duration::from_millis(1000));
        host.settle();
        assert!(tip.is_extended());
        let copy = RibbonScreenTipService::create(
            Some("Line".into()),
            Some(Control::boxed(tip.clone())),
            Some("L".into()),
        )
        .unwrap();
        let copy = ValueTypes::as_object(&*copy.unwrap())
            .unwrap()
            .cast::<RibbonScreenTip>()
            .unwrap();
        assert_eq!(copy.extended_description(), tip.extended_description());
    });
}
#[test]
fn application_menu_shows_sub_commands_recent_items_and_search() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let menu = RibbonApplicationMenu::new();
        menu.set_ribbon(Some(m.ribbon.clone()));
        let save = RibbonApplicationMenuItem::new();
        save.set_id(Some("saveAs".into()));
        save.set_label(Some("Save As".into()));
        let template = RibbonApplicationMenuItem::new();
        template.set_id(Some("saveTemplate".into()));
        template.set_label(Some("Drawing Template".into()));
        template.set_description(Some("Save as a template.".into()));
        save.items().add(Some(template.clone()));
        let new = RibbonApplicationMenuItem::new();
        new.set_id(Some("new".into()));
        new.set_label(Some("New".into()));
        menu.items().add(Some(new));
        menu.items().add(Some(save.clone()));
        let recent = RibbonApplicationMenuRecentItem::new();
        recent.set_title(Some("Floor Plan.dwg".into()));
        recent.set_path(Some("C:\\Projects".into()));
        recent.set_is_pinned(true);
        menu.recent_items().add(Some(recent));
        let invoked = Rc::new(RefCell::new(None));
        let out = invoked.clone();
        let token = menu.item_invoked(move |_, e| {
            out.replace(Some(e.item()));
        });
        let host = RuntimeHost::new(menu.clone().upcast(), Some(640.));
        assert!(menu.shown_item().is_none());
        menu.invoke(save.clone());
        assert!(menu.shown_item().unwrap().ptr_eq(&save));
        assert!(invoked.borrow().is_none());
        menu.invoke(template.clone());
        assert!(invoked.borrow().as_ref().unwrap().ptr_eq(&template));
        assert!(menu.search_menu("template").iter().any(|r| {
            r.entry().unwrap().target().is_some_and(|t| {
                crate::metadata::model_objects::to_native(Some(t))
                    .unwrap()
                    .and_then(|v| ValueTypes::as_object(&*v))
                    .is_some_and(|o| o.ptr_eq(&template))
            })
        }));
        assert!(
            menu.search_menu("bold")
                .iter()
                .any(|r| r.entry().unwrap().target().is_some_and(|t| {
                    crate::metadata::model_objects::to_native(Some(t))
                        .unwrap()
                        .and_then(|v| ValueTypes::as_object(&*v))
                        .and_then(|o| o.cast::<RibbonToggleButton>())
                        .is_some_and(|b| b.id().as_deref() == Some("bold"))
                }))
        );
        token.dispose();
        host.settle();
    });
}
#[test]
fn combo_selection_box_template_shows_the_selected_item() {
    crate::group_tests::run(|| {
        let combo = RibbonComboBox::new();
        combo.set_items_source(Some(Rc::new(ferroui_controls::ItemsSource::from_strs([
            "0", "Walls", "Doors",
        ]))));
        combo.set_input_width(160.);
        let template = selected_item_template();
        combo.set_selection_box_template(Some(template));
        let host = RuntimeHost::new(combo.clone().upcast(), Some(300.));
        combo.set_selected_item(Some(Rc::new("Walls".to_string())));
        host.settle();
        let selection = named::<ContentPresenter>(&combo, "PART_SelectionBox");
        assert!(selection.is_visible());
        assert_eq!(
            selection
                .content()
                .unwrap()
                .downcast_ref::<String>()
                .map(String::as_str),
            Some("Walls")
        );
        combo.set_is_editable(true);
        host.settle();
        assert!(!selection.is_visible());
    });
}
fn selected_item_template() -> Rc<dyn ferroui_controls::templates::IDataTemplate> {
    ferroui_controls::templates::FuncDataTemplate::new(
        |_| true,
        |value, _| {
            let text = TextBlock::new();
            text.set_text(
                value
                    .as_ref()
                    .and_then(|v| v.downcast_ref::<String>())
                    .map(String::as_str),
            );
            Some(text.upcast())
        },
        false,
    )
}
#[test]
fn layered_stroke_icons_render_paths() {
    crate::group_tests::run(|| {
        let presenter = RibbonIconPresenter::new();
        presenter.set_icon(Some(Rc::new(
            "{stroke=1.8}M4,28 L28,4|{color=#3DA9F5}M2,26 h4 v4 h-4 Z".to_owned(),
        )));
        presenter.set_icon_size(32.);
        presenter.set_foreground(Some(SolidColorBrush::from_uint32(0xff808080).into()));
        let _host = RuntimeHost::new(presenter.clone().upcast(), Some(64.));
        let paths = descendants(&presenter)
            .into_iter()
            .filter_map(|v| v.cast::<Path>())
            .collect::<Vec<_>>();
        assert_eq!(paths.len(), 2);
        assert!(
            paths[0].stroke_thickness() > 1.
                && paths[0].stroke().is_some()
                && paths[0].fill().is_none()
        );
        assert_eq!(
            paths[1]
                .fill()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color()
                .b,
            0xf5
        );
    });
}
#[test]
fn floating_panels_follow_ribbon_visibility() {
    crate::group_tests::run(|| {
        let m = Mounted::new(1400.);
        let group = sample::find_group(&m.ribbon, "editing");
        group.float(Some(ferroui_base::Point::new(60., 300.)));
        m.settle();
        assert!(group.is_floating_panel_open());
        m.ribbon.set_is_visible(false);
        m.settle();
        assert!(!group.is_floating_panel_open());
        assert!(group.is_floating());
        m.ribbon.set_is_visible(true);
        m.settle();
        assert!(group.is_floating_panel_open());
        m.ribbon.suspend_popups();
        assert!(!group.is_floating_panel_open());
        m.ribbon.resume_popups();
        assert!(group.is_floating_panel_open());
        group.return_to_ribbon();
    });
}
#[test]
fn menu_icons_convert_paths_images_and_use_the_converter() {
    crate::group_tests::run(|| {
        // Native resource configuration maps the source packaged-application context.
        register_assets(
            "ferroribbon_runtime_cad",
            &[(
                "Assets/icon.png",
                include_bytes!("../../tests/fixtures/images/bitmap-3x2.png"),
            )],
        );
        AssetLoader::invalidate_assembly_cache("ferroribbon_runtime_cad");
        let resources = ferroui_controls::Application::current()
            .unwrap()
            .resources();
        struct RestoreRoot(Ref<ResourceDictionary>, Option<Option<BoxedValue>>);
        impl Drop for RestoreRoot {
            fn drop(&mut self) {
                let key = RibbonIconPresenter::IMAGE_BASE_URI_RESOURCE;
                if let Some(value) = self.1.take() {
                    self.0.set(key, value);
                } else {
                    self.0.remove(&key.into());
                }
            }
        }
        let key = RibbonIconPresenter::IMAGE_BASE_URI_RESOURCE;
        let _restore_root = RestoreRoot(
            resources.clone(),
            resources.try_get_resource(&key.into(), None),
        );
        resources.set(
            key,
            Some(Rc::new("ferres://ferroribbon_runtime_cad/".to_owned())),
        );
        assert!(
            RibbonItemHelper::create_menu_icon(Some(Rc::new("M0,0 L10,0 L10,10 Z".to_string())))
                .unwrap()
                .unwrap()
                .is::<PathIcon>()
        );
        assert!(
            RibbonItemHelper::create_menu_icon(Some(Rc::new(
                "[stroke=1.5]M0,0 L10,10".to_string()
            )))
            .unwrap()
            .is_none()
        );
        assert!(
            RibbonItemHelper::create_menu_icon(Some(Rc::new(
                "ms-appx:///Assets/icon.png".to_string()
            )))
            .unwrap()
            .unwrap()
            .is::<ImageIcon>()
        );
        let entry = crate::compat::MenuFlyoutItem::new();
        entry.set_text(Some("Open".into()));
        RibbonMenu::set_icon(&entry, Some(Rc::new("\u{e8e5}".to_string())));
        assert!(
            ValueTypes::as_object(&*entry.icon().unwrap())
                .unwrap()
                .is::<FontIcon>()
        );
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                RibbonItemHelper::set_menu_icon_converter(None);
            }
        }
        let _reset = Reset;
        RibbonItemHelper::set_menu_icon_converter(Some(MarkupDelegate::new(|args| {
            let text = args
                .first()
                .and_then(Option::as_ref)
                .and_then(|v| v.downcast_ref::<String>());
            text.filter(|t| !t.is_empty() && t.starts_with('['))
                .map(|_| {
                    let icon = FontIcon::new();
                    icon.set_glyph(Some("\u{e8c6}".into()));
                    Control::boxed(icon)
                })
        })));
        assert!(
            RibbonItemHelper::create_menu_icon(Some(Rc::new(
                "[stroke=1.5]M0,0 L10,10".to_string()
            )))
            .unwrap()
            .unwrap()
            .is::<FontIcon>()
        );
    });
}
#[test]
fn status_bar_can_be_icon_only() {
    crate::group_tests::run(|| {
        let grid = RibbonToggleButton::new();
        grid.set_id(Some("grid".into()));
        grid.set_label(Some("Grid".into()));
        grid.set_icon(Some(Rc::new("\u{e80a}".to_string())));
        let bar = RibbonStatusBar::new();
        bar.items().add(Some(grid.clone().upcast()));
        let host = RuntimeHost::new(bar.clone().upcast(), Some(600.));
        assert!(grid.actual_show_label());
        bar.set_show_labels(false);
        host.settle();
        assert!(!grid.actual_show_label());
    });
}
#[test]
fn title_bar_icon_opens_the_application_menu() {
    crate::group_tests::run(|| {
        let ribbon = sample::create();
        ribbon.set_backstage(None);
        let menu = sample::menu(&["New"]);
        ribbon.set_application_menu(Some(menu.clone().upcast()));
        let bar = RibbonTitleBar::new();
        bar.set_ribbon(Some(ribbon.clone()));
        bar.set_app_icon(Some(Rc::new("\u{e8a5}".to_string())));
        bar.set_is_app_icon_menu_enabled(true);
        let stack = StackPanel::new();
        stack.children().add(bar.clone());
        stack.children().add(ribbon);
        let host = RuntimeHost::new(stack.upcast(), Some(1400.));
        let button = named::<Button>(&bar, "PART_AppButton");
        assert!(button.is_visible());
        let clicked = Rc::new(Cell::new(false));
        let out = clicked.clone();
        let token = bar.app_icon_click(move |_, _| out.set(true));
        ButtonAutomationPeer::new(&button).invoke().unwrap();
        host.settle();
        assert!(clicked.get());
        assert!(menu.is_open());
        assert!(menu.flyout_presenter_theme().is_some());
        menu.hide();
        token.dispose();
    });
}
