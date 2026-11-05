//! The sample windows of the SystemBackdrops and TitleBar examples: the windows the buttons open,
//! their backdrops and controllers, and the TitleBar behaviours the source handlers drive. The
//! platform's drawing of the materials and the window dragging are not tested here.

use crate::window_samples::{self, BackdropType, BackdropWindow};
use crate::{examples, tests::start};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::rendering::composition::system_backdrops::{
    DesktopAcrylicController, DesktopAcrylicKind, MicaController, MicaKind,
};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::Ref;
use ferroui_controls::media::{DesktopAcrylicBackdrop, MicaBackdrop};
use ferroui_controls::{
    Button, ComboBox, Control, Frame, NavigationView, RadioButtons, StackPanel, TextBlock, TextBox, TitleBar,
    ToggleSwitch, Window,
};

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap()
}

fn click(root: &Ref<Control>, name: &str) {
    root.get_control::<Button>(name)
        .raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
}

fn content(window: &Ref<Window>) -> Ref<Control> {
    window
        .content()
        .as_ref()
        .and_then(Control::from_boxed)
        .expect("the sample window has its content")
}

fn close_all() {
    for window in window_samples::opened_windows() {
        window.close();
    }
}

fn status(window: &Ref<Window>) -> String {
    content(window).get_control::<TextBlock>("tbChangeStatus").text().unwrap_or_default()
}

#[test]
fn system_backdrops_page_opens_the_built_in_backdrop_window_and_switches_its_backdrop() {
    let _application = start();
    let root = page("SystemBackdrops");
    for index in 1..=3 {
        assert!(root.find_control::<StackPanel>(&format!("SystemBackdropsExample{index}")).is_some());
    }
    assert!(window_samples::opened_windows().is_empty());
    // createBuiltInWindow_Click.
    click(&root, "ShowBuiltInBackdropWindow");
    let windows = window_samples::opened_windows();
    assert_eq!(windows.len(), 1);
    let window = windows[0].clone();
    assert_eq!(window.title().as_deref(), Some("SystemBackdrop sample window"));
    assert!(window.extend_client_area_to_decorations_hint());
    let content = content(&window);
    let backdrops = content.get_control::<ComboBox>("backdropComboBox");
    assert_eq!(backdrops.items().count(), 4);
    assert_eq!(backdrops.selected_index(), 0);
    let mica_supported = MicaController::is_supported();
    let acrylic_supported = DesktopAcrylicController::is_supported();
    let backdrop = window.system_backdrop();
    if mica_supported {
        let mica = backdrop.as_ref().and_then(|backdrop| backdrop.cast::<MicaBackdrop>()).expect("a MicaBackdrop");
        assert_eq!(mica.kind(), MicaKind::Base);
        assert_eq!(status(&window), "");
    } else {
        // Mica isn't supported: the window tries Acrylic, as SetBackdrop does.
        assert!(status(&window).contains("Mica isn't supported. Trying Acrylic."));
        assert_eq!(
            backdrop.as_ref().and_then(|backdrop| backdrop.cast::<DesktopAcrylicBackdrop>()).is_some(),
            acrylic_supported
        );
    }
    // BackdropComboBox_SelectionChanged: 1 MicaAlt, 2 Acrylic, 3 None.
    backdrops.set_selected_index(1);
    if mica_supported {
        let mica = window.system_backdrop().and_then(|backdrop| backdrop.cast::<MicaBackdrop>()).expect("a MicaBackdrop");
        assert_eq!(mica.kind(), MicaKind::BaseAlt);
    }
    backdrops.set_selected_index(2);
    assert_eq!(
        window.system_backdrop().and_then(|backdrop| backdrop.cast::<DesktopAcrylicBackdrop>()).is_some(),
        acrylic_supported
    );
    backdrops.set_selected_index(3);
    assert!(window.system_backdrop().is_none());
    assert_eq!(
        content.get_control::<TextBlock>("BackdropAnnouncement").text().as_deref(),
        Some("Background changed to None")
    );
    // ThemeComboBox_SelectionChanged: the window's theme; without a backdrop the content is white
    // in the light theme and black in the dark one.
    let themes = content.get_control::<ComboBox>("themeComboBox");
    themes.set_selected_index(2);
    assert_eq!(window.requested_theme_variant(), Some(ThemeVariant::dark()));
    themes.set_selected_index(1);
    assert_eq!(window.requested_theme_variant(), Some(ThemeVariant::light()));
    themes.set_selected_index(0);
    assert_eq!(window.requested_theme_variant(), None);
    // The window lives until it is closed.
    window.close();
    assert!(window_samples::opened_windows().is_empty());
}

#[test]
fn controller_windows_drive_mica_and_desktop_acrylic_controllers_with_a_configuration() {
    let _application = start();
    let root = page("SystemBackdrops");
    // createCustomMicaWindow_Click and createCustomDesktopAcrylicWindow_Click open windows.
    click(&root, "ShowMicaControllerWindow");
    click(&root, "ShowDesktopAcrylicControllerWindow");
    assert_eq!(window_samples::opened_windows().len(), 2);
    close_all();

    let mica_window =
        BackdropWindow::open_with_controllers(&root, &[BackdropType::Mica, BackdropType::MicaAlt, BackdropType::None])
            .unwrap();
    let window = mica_window.window();
    let backdrops = content(&window).get_control::<ComboBox>("backdropComboBox");
    assert_eq!(backdrops.items().count(), 3);
    // The controllers draw the material; the window has no SystemBackdrop object.
    assert!(window.system_backdrop().is_none());
    if MicaController::is_supported() {
        assert_eq!(mica_window.current(), BackdropType::Mica);
        assert_eq!(mica_window.mica_controller().expect("a Mica controller").kind(), MicaKind::Base);
        let configuration = mica_window.configuration().expect("a configuration");
        assert!(configuration.is_input_active());
        backdrops.set_selected_index(1);
        assert_eq!(mica_window.current(), BackdropType::MicaAlt);
        assert_eq!(mica_window.mica_controller().expect("a Mica controller").kind(), MicaKind::BaseAlt);
    }
    // None removes the controllers and the configuration.
    backdrops.set_selected_index(2);
    assert_eq!(mica_window.current(), BackdropType::None);
    assert!(mica_window.mica_controller().is_none());
    assert!(mica_window.acrylic_controller().is_none());
    assert!(mica_window.configuration().is_none());
    window.close();

    let acrylic_window = BackdropWindow::open_with_controllers(
        &root,
        &[BackdropType::Acrylic, BackdropType::AcrylicThin, BackdropType::None],
    )
    .unwrap();
    let window = acrylic_window.window();
    let backdrops = content(&window).get_control::<ComboBox>("backdropComboBox");
    if DesktopAcrylicController::is_supported() {
        assert_eq!(acrylic_window.current(), BackdropType::Acrylic);
        assert_eq!(
            acrylic_window.acrylic_controller().expect("an acrylic controller").kind(),
            DesktopAcrylicKind::Base
        );
        backdrops.set_selected_index(1);
        assert_eq!(acrylic_window.current(), BackdropType::AcrylicThin);
        assert_eq!(
            acrylic_window.acrylic_controller().expect("an acrylic controller").kind(),
            DesktopAcrylicKind::Thin
        );
        // Window_Closed: no controller uses the closed window.
        window.close();
        assert!(acrylic_window.acrylic_controller().is_none());
        assert!(acrylic_window.configuration().is_none());
    } else {
        assert!(status(&window).contains("Acrylic Base isn't supported. Switching to default color."));
        window.close();
    }
    assert!(window_samples::opened_windows().is_empty());
}

#[test]
fn title_bar_configuration_binds_title_subtitle_and_button_visibility_to_the_options() {
    let _application = start();
    let root = page("TitleBar");
    for index in 1..=3 {
        assert!(root.find_control::<StackPanel>(&format!("TitleBarExample{index}")).is_some());
    }
    let bar = root.get_control::<TitleBar>("TitleBarControl");
    assert_eq!(bar.title(), "WinUI Gallery");
    assert_eq!(bar.subtitle(), "Preview");
    assert!(!bar.is_back_button_visible());
    assert!(!bar.is_pane_toggle_button_visible());
    assert!(bar.icon_source().is_some());
    assert!(bar.content().is_some());
    assert!(bar.right_header().is_some());
    root.get_control::<TextBox>("TitleBox").set_text(Some("Sample title"));
    root.get_control::<TextBox>("SubtitleBox").set_text(Some("Sample subtitle"));
    assert_eq!(bar.title(), "Sample title");
    assert_eq!(bar.subtitle(), "Sample subtitle");
    root.get_control::<ToggleSwitch>("BackButtonToggle").set_is_checked(Some(true));
    assert!(bar.is_back_button_visible());
    root.get_control::<ToggleSwitch>("PaneToggle").set_is_checked(Some(true));
    assert!(bar.is_pane_toggle_button_visible());
    root.get_control::<ToggleSwitch>("BackButtonToggle").set_is_checked(Some(false));
    assert!(!bar.is_back_button_visible());
}

#[test]
fn title_bar_drag_regions_window_sets_is_drag_region_and_toggles_dynamic_content() {
    let _application = start();
    let root = page("TitleBar");
    click(&root, "ShowTitleBarDragRegionsWindow");
    let windows = window_samples::opened_windows();
    assert_eq!(windows.len(), 1);
    let window = windows[0].clone();
    assert_eq!(window.title().as_deref(), Some("Drag regions"));
    assert!(window.extend_client_area_to_decorations_hint());
    assert_eq!(window.extend_client_area_title_bar_height_hint(), 48.0);
    assert!(window.system_backdrop().and_then(|backdrop| backdrop.cast::<MicaBackdrop>()).is_some());
    let content = content(&window);
    let badge = content.get_control::<Button>("StatusBadge");
    let status = content.get_control::<TextBlock>("StatusText");
    // BadgeIsDragRegionRadios_SelectionChanged: unset, true, false, unset again.
    let radios = content.get_control::<RadioButtons>("BadgeIsDragRegionRadios");
    assert_eq!(TitleBar::get_is_drag_region(&badge), None);
    radios.set_selected_index(1);
    assert_eq!(TitleBar::get_is_drag_region(&badge), Some(true));
    radios.set_selected_index(2);
    assert_eq!(TitleBar::get_is_drag_region(&badge), Some(false));
    radios.set_selected_index(0);
    assert_eq!(TitleBar::get_is_drag_region(&badge), None);
    // StatusBadge_Click.
    click(&content, "StatusBadge");
    assert_eq!(status.text().as_deref(), Some("Status badge clicked"));
    // ToggleExtraButton_Click adds a button before the badge, then removes it.
    let panel = content.get_control::<StackPanel>("RightHeaderPanel");
    click(&content, "ToggleExtraButton");
    assert_eq!(panel.children().count(), 2);
    assert!(status.text().unwrap_or_default().starts_with("Added a Button to TitleBar.Content."));
    click(&content, "ToggleExtraButton");
    assert_eq!(panel.children().count(), 1);
    assert!(status.text().unwrap_or_default().starts_with("Removed the Button."));
    // RecomputeDragRegions_Click.
    click(&content, "RecomputeDragRegionsButton");
    assert_eq!(status.text().as_deref(), Some("RecomputeDragRegions() called."));
    close_all();
}

#[test]
fn title_bar_window_navigates_the_frame_and_binds_the_back_button_to_can_go_back() {
    let _application = start();
    let root = page("TitleBar");
    click(&root, "ShowTitleBarWindow");
    let windows = window_samples::opened_windows();
    assert_eq!(windows.len(), 1);
    let window = windows[0].clone();
    let content = content(&window);
    let bar = content.get_control::<TitleBar>("titleBar");
    let nav = content.get_control::<NavigationView>("navView");
    let frame = content.get_control::<Frame>("navFrame");
    assert!(bar.is_pane_toggle_button_visible());
    // navView.SelectedItem = the first item: the frame shows the first sample page.
    assert!(frame.content().is_some());
    assert!(!frame.can_go_back());
    assert!(!bar.is_back_button_visible());
    // navView_SelectionChanged: the header and the page of the selected item.
    nav.set_selected_item(nav.menu_items().get_at(2));
    assert_eq!(
        nav.header().and_then(|header| header.downcast_ref::<String>().cloned()).as_deref(),
        Some("Sample Page 3")
    );
    // IsBackButtonVisible="{x:Bind navFrame.CanGoBack}".
    assert!(frame.can_go_back());
    assert!(bar.is_back_button_visible());
    close_all();
}
