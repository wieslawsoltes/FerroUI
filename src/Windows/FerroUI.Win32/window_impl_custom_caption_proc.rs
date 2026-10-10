//! The window procedure of a window whose client area extends into its
//! frame: the hit tests of the frame and of the drawn title bar, the
//! non-client input of the caption buttons as client input, and the system
//! menu.
//!
//! The first part of the file is the decisions, as functions of values; it
//! is compiled, and tested, on every host. The procedure itself is the
//! second part.

use crate::interop::unmanaged_methods::{HitTestValues, SysCommands, RECT};
use ferroui_base::input::WindowDecorationsElementRole;
use ferroui_base::PixelPoint;
use ferroui_controls::WindowState;

/// The hit test of the frame of a window with an extended client area.
///
/// `rc_window` is the rectangle of the window on the screen, `rc_frame`
/// the frame of the style without a caption (negative at the left and top),
/// `border_thickness` the widths of the resize borders and, at the top, the
/// height of the title bar. `caption_area_hit_test` is what the title bar
/// answers (nothing in full screen).
pub(crate) fn hit_test_nca(
    pt_mouse: PixelPoint,
    rc_window: RECT,
    rc_frame: RECT,
    border_thickness: RECT,
    caption_area_hit_test: i32,
) -> i32 {
    // Determine if the hit test is for resizing. Default middle (1,1).
    let mut row = 1usize;
    let mut col = 1usize;
    let mut on_resize_border = false;

    // Determine if the point is at the left or right of the window.
    if pt_mouse.x >= rc_window.left && pt_mouse.x < rc_window.left + border_thickness.left {
        col = 0; // left side
    } else if pt_mouse.x < rc_window.right && pt_mouse.x >= rc_window.right - border_thickness.right {
        col = 2; // right side
    }

    // Determine if the point is at the top or bottom of the window.
    if pt_mouse.y >= rc_window.top && pt_mouse.y < rc_window.top + border_thickness.top {
        on_resize_border = pt_mouse.y < (rc_window.top - rc_frame.top);

        // Two cases where we have a valid row 0 hit test:
        // - window resize border (top resize border hit)
        // - area below resize border that is actual titlebar (caption hit).
        if on_resize_border || col == 1 {
            row = 0;
        }
    } else if pt_mouse.y < rc_window.bottom && pt_mouse.y >= rc_window.bottom - border_thickness.bottom {
        row = 2;
    }

    let hit_zones = [
        HitTestValues::HTTOPLEFT,
        if on_resize_border { HitTestValues::HTTOP } else { caption_area_hit_test },
        HitTestValues::HTTOPRIGHT,
        HitTestValues::HTLEFT,
        HitTestValues::HTNOWHERE,
        HitTestValues::HTRIGHT,
        HitTestValues::HTBOTTOMLEFT,
        HitTestValues::HTBOTTOM,
        HitTestValues::HTBOTTOMRIGHT,
    ];

    hit_zones[row * 3 + col]
}

/// The hit test value of the role an element of the drawn decorations has.
pub(crate) fn hit_test_from_chrome_role(role: WindowDecorationsElementRole) -> i32 {
    match role {
        // DecorationsElement/User = interactive chrome element (e.g., caption button)
        // - signal to redirect NC input to client input.
        WindowDecorationsElementRole::DecorationsElement => HitTestValues::HTCLIENT,
        WindowDecorationsElementRole::User => HitTestValues::HTCLIENT,
        WindowDecorationsElementRole::TitleBar => HitTestValues::HTCAPTION,
        WindowDecorationsElementRole::ResizeN => HitTestValues::HTTOP,
        WindowDecorationsElementRole::ResizeS => HitTestValues::HTBOTTOM,
        WindowDecorationsElementRole::ResizeE => HitTestValues::HTRIGHT,
        WindowDecorationsElementRole::ResizeW => HitTestValues::HTLEFT,
        WindowDecorationsElementRole::ResizeNE => HitTestValues::HTTOPRIGHT,
        WindowDecorationsElementRole::ResizeNW => HitTestValues::HTTOPLEFT,
        WindowDecorationsElementRole::ResizeSE => HitTestValues::HTBOTTOMRIGHT,
        WindowDecorationsElementRole::ResizeSW => HitTestValues::HTBOTTOMLEFT,
        WindowDecorationsElementRole::CloseButton => HitTestValues::HTCLOSE,
        WindowDecorationsElementRole::MinimizeButton => HitTestValues::HTMINBUTTON,
        WindowDecorationsElementRole::MaximizeButton => HitTestValues::HTMAXBUTTON,
        WindowDecorationsElementRole::FullScreenButton => HitTestValues::HTCLIENT,
        _ => HitTestValues::HTNOWHERE,
    }
}

/// Whether non-client input over a place with this hit test value of the
/// visuals is delivered as client input: only over buttons.
pub(crate) fn is_redirected_hit_test(visual_hit_test: i32) -> bool {
    matches!(
        visual_hit_test,
        HitTestValues::HTMINBUTTON
            | HitTestValues::HTMAXBUTTON
            | HitTestValues::HTCLOSE
            | HitTestValues::HTHELP
            | HitTestValues::HTMENU
            | HitTestValues::HTSYSMENU
    )
}

/// The state of the system menu of a window: which commands are enabled,
/// and the default command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SystemMenuState {
    pub restore: bool,
    pub move_: bool,
    pub size: bool,
    pub minimize: bool,
    pub maximize: bool,
    pub default_item: i32,
}

pub(crate) fn system_menu_state(
    state: WindowState,
    is_resizable: bool,
    is_minimizable: bool,
    is_maximizable: bool,
) -> SystemMenuState {
    let is_minimized = state == WindowState::Minimized;
    let is_maximized = state == WindowState::Maximized;
    let is_full_screen = state == WindowState::FullScreen;
    let is_normal = state == WindowState::Normal;

    SystemMenuState {
        restore: is_minimized || is_maximized || is_full_screen,
        move_: is_normal || is_full_screen,
        size: is_normal && is_resizable,
        minimize: !is_minimized && !is_full_screen && is_minimizable,
        maximize: !is_maximized && !is_full_screen && is_maximizable,
        default_item: if is_minimized || is_maximized || is_full_screen {
            SysCommands::SC_RESTORE
        } else if is_normal && is_maximizable {
            SysCommands::SC_MAXIMIZE
        } else {
            SysCommands::SC_CLOSE
        },
    }
}

/// A point of the screen as the parameter of a message.
pub(crate) fn make_l_param(point: PixelPoint) -> isize {
    (((point.y & 0xffff) << 16) | (point.x & 0xffff)) as isize
}

#[cfg(windows)]
mod imp {
    use super::*;
    use crate::interop::unmanaged_methods::*;
    use crate::window_impl::WindowImpl;
    use crate::window_impl_app_wnd_proc::{point_from_l_param, to_int32};
    use ferroui_base::input::raw::{IRawInputEventArgs, RawPointerEventArgs, RawPointerEventType};
    use ferroui_base::input::{InputElement, RawInputModifiers};
    use ferroui_base::{Point, Visual};
    use ferroui_controls::platform::Win32Properties;
    use std::rc::Rc;

    impl WindowImpl {
        fn hit_test_nca(&self, hwnd: isize, l_param: isize) -> i32 {
            // Get the point coordinates for the hit test (screen space).
            let pt_mouse = point_from_l_param(l_param);

            // Get the window rectangle.
            let rc_window = get_window_rect(hwnd);

            // Get the frame rectangle, adjusted for the style without a caption.
            let mut rc_frame = RECT::default();
            let mut border_thickness = RECT::default();

            let is_maximized = get_window_placement(hwnd).show_cmd == ShowWindowCommand::SHOW_MAXIMIZED;
            if !is_maximized {
                let style = self.get_style();
                if style.contains(WindowStyles::WS_THICKFRAME) {
                    self.adjust_window_rect(&mut rc_frame, style & !WindowStyles::WS_CAPTION, WindowStyles::empty());
                    self.adjust_window_rect(&mut border_thickness, style, WindowStyles::empty());

                    border_thickness.left *= -1;
                    border_thickness.top *= -1;
                }
            }

            if self.extend_title_bar_hint() >= 0.0 {
                border_thickness.top = (self.extended_margins_value().top * self.scaling()) as i32;
            }

            let caption_area_hit_test = if self.window_state_impl() == WindowState::FullScreen {
                HitTestValues::HTNOWHERE
            } else {
                HitTestValues::HTCAPTION
            };

            super::hit_test_nca(pt_mouse, rc_window, rc_frame, border_thickness, caption_area_hit_test)
        }

        /// The procedure of the custom caption. `call_dwp` says whether
        /// the procedure of the application handles the message after it.
        pub(crate) fn custom_caption_proc(
            &self,
            hwnd: isize,
            msg: u32,
            w_param: usize,
            l_param: isize,
            call_dwp: &mut bool,
        ) -> isize {
            let mut e: Option<Rc<dyn IRawInputEventArgs>> = None;
            let dwm_result = dwm_def_window_proc(hwnd, msg, w_param, l_param);
            let mut l_ret = dwm_result.unwrap_or(0);

            *call_dwp = dwm_result.is_none();

            let mouse_in_pointer = self.is_mouse_in_pointer_enabled();
            let timestamp = u64::from(get_message_time() as u32);

            match msg {
                WindowsMessage::WM_DWMCOMPOSITIONCHANGED => {
                    // TODO handle composition changed.
                }

                WindowsMessage::WM_NCHITTEST => {
                    if l_ret == 0 {
                        let mut hittest_result = self.hit_test_nca(hwnd, l_param);
                        if hittest_result == HitTestValues::HTNOWHERE || hittest_result == HitTestValues::HTCAPTION {
                            let visual_hittest_result = self.hit_test_visual(l_param);
                            if visual_hittest_result != HitTestValues::HTNOWHERE {
                                hittest_result = visual_hittest_result;
                            }
                        }

                        if hittest_result != HitTestValues::HTNOWHERE {
                            l_ret = hittest_result as isize;
                            *call_dwp = false;
                        }
                    }
                }

                WindowsMessage::WM_NCRBUTTONUP if to_int32(w_param as isize) == HitTestValues::HTCAPTION => {
                    self.show_system_menu(point_from_l_param(l_param));
                }

                WindowsMessage::WM_INITMENU => {
                    self.update_system_menu(get_system_menu(hwnd, false));
                }

                // Normally, the toolkit doesn't handle non-client input as a special NonClientLeftButtonDown, ignoring move and up events.
                // What makes it a problem, the toolkit has to mark templated caption buttons as a non-client area.
                // Meaning, these buttons no longer can accept normal client input.
                // These messages are needed to explicitly fake this normal client input from non-client messages.
                // For both WM_NCMOUSE and WM_NCPOINTERUPDATE
                WindowsMessage::WM_NCMOUSEMOVE | WindowsMessage::WM_NCLBUTTONDOWN | WindowsMessage::WM_NCLBUTTONUP
                    if !mouse_in_pointer =>
                {
                    if l_ret == 0 {
                        let should_redirect = self.should_redirect_non_client_input(hwnd, l_param);

                        if should_redirect {
                            // Track non-client mouse to receive WM_NCMOUSELEAVE
                            if !self.tracking_non_client_mouse() {
                                track_mouse_event(self.hwnd(), TME_LEAVE | TME_NONCLIENT);
                                self.set_tracking_non_client_mouse(true);
                            }

                            let type_ = match msg {
                                WindowsMessage::WM_NCMOUSEMOVE => RawPointerEventType::Move,
                                WindowsMessage::WM_NCLBUTTONDOWN => RawPointerEventType::LeftButtonDown,
                                _ => RawPointerEventType::LeftButtonUp,
                            };
                            e = Some(Rc::new(RawPointerEventArgs::new(
                                self.mouse_device().device().clone(),
                                timestamp,
                                self.owner(),
                                type_,
                                self.point_to_client_impl(point_from_l_param(l_param)),
                                RawInputModifiers::NONE,
                            )));
                        } else if self.tracking_non_client_mouse() && msg == WindowsMessage::WM_NCMOUSEMOVE {
                            // Mouse moved in NC area but not over caption buttons - send leave event
                            self.set_tracking_non_client_mouse(false);
                            e = Some(self.non_client_leave_event(timestamp));
                        }
                    }
                }

                WindowsMessage::WM_NCMOUSELEAVE if !mouse_in_pointer => {
                    self.set_tracking_non_client_mouse(false);
                    e = Some(self.non_client_leave_event(timestamp));
                }

                WindowsMessage::WM_NCPOINTERUPDATE | WindowsMessage::WM_NCPOINTERDOWN | WindowsMessage::WM_NCPOINTERUP
                    if self.wm_pointer_enabled() =>
                {
                    if l_ret == 0 && self.should_redirect_non_client_input(hwnd, l_param) {
                        let pointer = self.get_device_pointer_info(w_param, 0);
                        let event_type = match msg {
                            WindowsMessage::WM_NCPOINTERUPDATE => RawPointerEventType::Move,
                            WindowsMessage::WM_NCPOINTERDOWN => RawPointerEventType::LeftButtonDown,
                            _ => RawPointerEventType::LeftButtonUp,
                        };
                        e = Some(self.create_pointer_args(&pointer, event_type));
                    }
                }

                _ => {}
            }

            if let Some(e) = e {
                if let Some(input) = self.input_callback() {
                    input(e.clone());
                    if e.handled() {
                        *call_dwp = false;
                        return 0;
                    }
                }
            }

            l_ret
        }

        fn non_client_leave_event(&self, timestamp: u64) -> Rc<dyn IRawInputEventArgs> {
            Rc::new(RawPointerEventArgs::new(
                self.mouse_device().device().clone(),
                timestamp,
                self.owner(),
                RawPointerEventType::LeaveWindow,
                Point::new(-1.0, -1.0),
                RawInputModifiers::NONE,
            ))
        }

        fn hit_test_visual(&self, l_param: isize) -> i32 {
            let position = self.point_to_client_impl(point_from_l_param(l_param));

            let Some(owner) = self.try_owner() else {
                return HitTestValues::HTNOWHERE;
            };

            // First, check new cross-platform ElementRole via chrome hit-test
            if let Some(chrome_role) = owner.hit_test_chrome_element(position) {
                return hit_test_from_chrome_role(chrome_role);
            }

            // Fall back to Win32-specific NonClientHitTestResult attached property
            if let Some(window) = owner.try_root_element() {
                let visual = window.get_visual_at_filtered(position, &|x: &Visual| {
                    if let Some(ie) = x.downcast_ref::<InputElement>() {
                        if !ie.is_hit_test_visible() || !ie.is_effectively_visible() {
                            return false;
                        }
                    }

                    true
                });

                if let Some(visual) = visual {
                    return Win32Properties::get_non_client_hit_test_result(&visual) as i32;
                }
            }

            HitTestValues::HTNOWHERE
        }

        fn should_redirect_non_client_input(&self, hwnd: isize, l_param: isize) -> bool {
            // We touched frame borders or caption, don't redirect.
            let nca = self.hit_test_nca(hwnd, l_param);
            if nca != HitTestValues::HTNOWHERE && nca != HitTestValues::HTCAPTION {
                return false;
            }

            // Redirect only for buttons.
            is_redirected_hit_test(self.hit_test_visual(l_param))
        }

        fn show_system_menu(&self, screen_point: PixelPoint) {
            let hwnd = self.hwnd();
            let menu = get_system_menu(hwnd, false);
            if menu == 0 {
                return;
            }

            set_foreground_window(hwnd);

            let command = track_popup_menu(
                menu,
                TrackPopupMenuFlags::TPM_RIGHTBUTTON | TrackPopupMenuFlags::TPM_RETURNCMD,
                screen_point.x,
                screen_point.y,
                hwnd,
            );

            post_message(hwnd, WindowsMessage::WM_NULL, 0, 0);

            if command != 0 {
                if command == SysCommands::SC_RESTORE && self.window_state_impl() == WindowState::FullScreen {
                    self.set_window_state_impl(WindowState::Normal);
                } else {
                    send_message(hwnd, WindowsMessage::WM_SYSCOMMAND, command as usize, make_l_param(screen_point));
                }
            }
        }

        fn update_system_menu(&self, menu: isize) {
            if menu == 0 {
                return;
            }

            let properties = self.window_properties();
            let state = system_menu_state(
                self.window_state_impl(),
                properties.is_resizable,
                properties.is_minimizable,
                properties.is_maximizable,
            );

            set_system_menu_item_enabled(menu, SysCommands::SC_RESTORE, state.restore);
            set_system_menu_item_enabled(menu, SysCommands::SC_MOVE, state.move_);
            set_system_menu_item_enabled(menu, SysCommands::SC_SIZE, state.size);
            set_system_menu_item_enabled(menu, SysCommands::SC_MINIMIZE, state.minimize);
            set_system_menu_item_enabled(menu, SysCommands::SC_MAXIMIZE, state.maximize);

            set_menu_default_item(menu, state.default_item as u32, 0);
        }
    }

    fn set_system_menu_item_enabled(menu: isize, command: i32, enabled: bool) {
        enable_menu_item(
            menu,
            command as u32,
            MF_BYCOMMAND | if enabled { MF_ENABLED } else { MF_DISABLED | MF_GRAYED },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: RECT = RECT { left: 100, top: 100, right: 500, bottom: 400 };
    // The frame of a sizing border of 8 pixels without a caption, and the
    // border thickness with a title bar of 31 pixels.
    const FRAME: RECT = RECT { left: -8, top: -8, right: 8, bottom: 8 };
    const BORDER: RECT = RECT { left: 8, top: 31, right: 8, bottom: 8 };

    fn hit(x: i32, y: i32) -> i32 {
        hit_test_nca(PixelPoint::new(x, y), WINDOW, FRAME, BORDER, HitTestValues::HTCAPTION)
    }

    #[test]
    fn the_nine_zones_of_a_restored_window() {
        assert_eq!(hit(102, 102), HitTestValues::HTTOPLEFT);
        assert_eq!(hit(300, 102), HitTestValues::HTTOP);
        assert_eq!(hit(498, 102), HitTestValues::HTTOPRIGHT);
        assert_eq!(hit(102, 250), HitTestValues::HTLEFT);
        assert_eq!(hit(300, 250), HitTestValues::HTNOWHERE);
        assert_eq!(hit(498, 250), HitTestValues::HTRIGHT);
        assert_eq!(hit(102, 398), HitTestValues::HTBOTTOMLEFT);
        assert_eq!(hit(300, 398), HitTestValues::HTBOTTOM);
        assert_eq!(hit(498, 398), HitTestValues::HTBOTTOMRIGHT);
    }

    #[test]
    fn below_the_resize_border_the_title_bar_is_the_caption() {
        // The first 8 rows are the resize border, the rest of the 31 the caption.
        assert_eq!(hit(300, 107), HitTestValues::HTTOP);
        assert_eq!(hit(300, 108), HitTestValues::HTCAPTION);
        assert_eq!(hit(300, 130), HitTestValues::HTCAPTION);
        assert_eq!(hit(300, 131), HitTestValues::HTNOWHERE);
        // At the sides of the title bar, below the resize border, the row
        // is the middle one: the left and right borders.
        assert_eq!(hit(102, 120), HitTestValues::HTLEFT);
        assert_eq!(hit(498, 120), HitTestValues::HTRIGHT);
    }

    #[test]
    fn a_maximized_window_has_a_caption_and_no_borders() {
        // Maximized: no frame and no border thickness but the title bar.
        let border = RECT { left: 0, top: 31, right: 0, bottom: 0 };
        let hit = |x, y| hit_test_nca(PixelPoint::new(x, y), WINDOW, RECT::default(), border, HitTestValues::HTCAPTION);
        assert_eq!(hit(100, 100), HitTestValues::HTCAPTION);
        assert_eq!(hit(499, 110), HitTestValues::HTCAPTION);
        assert_eq!(hit(100, 250), HitTestValues::HTNOWHERE);
        assert_eq!(hit(300, 399), HitTestValues::HTNOWHERE);
    }

    #[test]
    fn a_full_screen_window_has_no_caption() {
        let border = RECT { left: 0, top: 31, right: 0, bottom: 0 };
        let hit = hit_test_nca(PixelPoint::new(300, 110), WINDOW, RECT::default(), border, HitTestValues::HTNOWHERE);
        assert_eq!(hit, HitTestValues::HTNOWHERE);
    }

    #[test]
    fn a_point_outside_the_window_is_nowhere() {
        assert_eq!(hit(99, 250), HitTestValues::HTNOWHERE);
        assert_eq!(hit(500, 250), HitTestValues::HTNOWHERE);
        assert_eq!(hit(300, 99), HitTestValues::HTNOWHERE);
        assert_eq!(hit(300, 400), HitTestValues::HTNOWHERE);
    }

    #[test]
    fn the_roles_of_the_drawn_decorations() {
        use WindowDecorationsElementRole as Role;
        assert_eq!(hit_test_from_chrome_role(Role::DecorationsElement), HitTestValues::HTCLIENT);
        assert_eq!(hit_test_from_chrome_role(Role::User), HitTestValues::HTCLIENT);
        assert_eq!(hit_test_from_chrome_role(Role::TitleBar), HitTestValues::HTCAPTION);
        assert_eq!(hit_test_from_chrome_role(Role::ResizeN), HitTestValues::HTTOP);
        assert_eq!(hit_test_from_chrome_role(Role::ResizeS), HitTestValues::HTBOTTOM);
        assert_eq!(hit_test_from_chrome_role(Role::ResizeE), HitTestValues::HTRIGHT);
        assert_eq!(hit_test_from_chrome_role(Role::ResizeW), HitTestValues::HTLEFT);
        assert_eq!(hit_test_from_chrome_role(Role::ResizeNE), HitTestValues::HTTOPRIGHT);
        assert_eq!(hit_test_from_chrome_role(Role::ResizeNW), HitTestValues::HTTOPLEFT);
        assert_eq!(hit_test_from_chrome_role(Role::ResizeSE), HitTestValues::HTBOTTOMRIGHT);
        assert_eq!(hit_test_from_chrome_role(Role::ResizeSW), HitTestValues::HTBOTTOMLEFT);
        assert_eq!(hit_test_from_chrome_role(Role::CloseButton), HitTestValues::HTCLOSE);
        assert_eq!(hit_test_from_chrome_role(Role::MinimizeButton), HitTestValues::HTMINBUTTON);
        assert_eq!(hit_test_from_chrome_role(Role::MaximizeButton), HitTestValues::HTMAXBUTTON);
        assert_eq!(hit_test_from_chrome_role(Role::FullScreenButton), HitTestValues::HTCLIENT);
        assert_eq!(hit_test_from_chrome_role(Role::None), HitTestValues::HTNOWHERE);
    }

    #[test]
    fn only_buttons_get_non_client_input_as_client_input() {
        for value in [
            HitTestValues::HTMINBUTTON,
            HitTestValues::HTMAXBUTTON,
            HitTestValues::HTCLOSE,
            HitTestValues::HTHELP,
            HitTestValues::HTMENU,
            HitTestValues::HTSYSMENU,
        ] {
            assert!(is_redirected_hit_test(value));
        }
        for value in [HitTestValues::HTNOWHERE, HitTestValues::HTCLIENT, HitTestValues::HTCAPTION, HitTestValues::HTTOP] {
            assert!(!is_redirected_hit_test(value));
        }
    }

    #[test]
    fn the_system_menu_of_each_state() {
        let normal = system_menu_state(WindowState::Normal, true, true, true);
        assert_eq!(
            normal,
            SystemMenuState {
                restore: false,
                move_: true,
                size: true,
                minimize: true,
                maximize: true,
                default_item: SysCommands::SC_MAXIMIZE
            }
        );

        let fixed = system_menu_state(WindowState::Normal, false, false, false);
        assert!(!fixed.size && !fixed.minimize && !fixed.maximize && fixed.move_);
        assert_eq!(fixed.default_item, SysCommands::SC_CLOSE);

        let maximized = system_menu_state(WindowState::Maximized, true, true, true);
        assert!(maximized.restore && !maximized.move_ && !maximized.size && maximized.minimize && !maximized.maximize);
        assert_eq!(maximized.default_item, SysCommands::SC_RESTORE);

        let minimized = system_menu_state(WindowState::Minimized, true, true, true);
        assert!(minimized.restore && !minimized.minimize && minimized.maximize);
        assert_eq!(minimized.default_item, SysCommands::SC_RESTORE);

        let full_screen = system_menu_state(WindowState::FullScreen, true, true, true);
        assert!(full_screen.restore && full_screen.move_ && !full_screen.size);
        assert!(!full_screen.minimize && !full_screen.maximize);
        assert_eq!(full_screen.default_item, SysCommands::SC_RESTORE);
    }

    #[test]
    fn a_screen_point_as_a_message_parameter() {
        assert_eq!(make_l_param(PixelPoint::new(10, 20)), 0x0014_000a);
        // Negative coordinates (a monitor left of or above the primary).
        assert_eq!(make_l_param(PixelPoint::new(-1, -2)), 0xfffe_ffffu32 as i32 as isize);
        assert_eq!(
            crate::window_impl_app_wnd_proc::point_from_l_param(make_l_param(PixelPoint::new(-300, 45))),
            PixelPoint::new(-300, 45)
        );
    }
}
