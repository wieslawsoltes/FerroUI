// Copyright (c) 2026 Wiesław Šoltés. MIT. Native services/event boundary of CadPage.
use super::CadPage;
use crate::cad::{
    CadDrawingCanvas, CadLayer,
    cad_view_model::{CadPageAction, CadSceneAction, CadSurface},
};
use ferroribbon::{
    compat::FontIcon,
    controls::{Ribbon, RibbonApplicationMenuRecentItem, RibbonButton, RibbonToggleButton},
    theming::RibbonTheme,
};
use ferroui_base::{
    CornerRadius, Point, Ref, Thickness,
    media::{Colors, IBrush, SolidColorBrush},
    metadata::{MarkupDelegate, from_markup_value},
    reactive::Disposable,
    styling::{ControlTheme, Setter, ThemeVariant},
};
#[cfg(not(target_arch = "wasm32"))]
use ferroui_controls::TopLevel;
#[cfg(any(windows, target_arch = "wasm32"))]
use ferroui_controls::primitives::popup_positioning::PopupPositionerConstraintAdjustment;
use ferroui_controls::{
    Application, Border, Button, Control, Flyout, FlyoutPresenter, TextBlock,
    primitives::TemplatedControl,
    shapes::{Ellipse, Rectangle},
};
use std::rc::Rc;

impl CadPage {
    pub(super) fn connect_page_actions(&self) {
        self.configure_application_menu();
        type ToggleAction = fn(bool) -> CadSceneAction;
        let toggles: [(&str, ToggleAction); 12] = [
            ("UcsToggle", |v| {
                CadSceneAction::Visibility(CadSurface::Ucs, v)
            }),
            ("ViewCubeToggle", |v| {
                CadSceneAction::Visibility(CadSurface::ViewCube, v)
            }),
            ("NavBarToggle", |v| {
                CadSceneAction::Visibility(CadSurface::Navigation, v)
            }),
            ("CommandLineToggle", |v| {
                CadSceneAction::Visibility(CadSurface::CommandLine, v)
            }),
            ("FileTabsToggle", |v| {
                CadSceneAction::Visibility(CadSurface::FileTabs, v)
            }),
            ("LayoutTabsToggle", |v| {
                CadSceneAction::Visibility(CadSurface::LayoutTabs, v)
            }),
            ("GridToggle", CadSceneAction::Grid),
            ("LineweightToggle", CadSceneAction::Lineweights),
            ("ModelToggle", CadSceneAction::Model),
            ("CleanScreenToggle", CadSceneAction::CleanScreen),
            ("NavPan", CadSceneAction::Pan),
            ("PanelTitlesToggle", CadSceneAction::PanelTitles),
        ];
        for (name, action) in toggles {
            let toggle = self.named::<RibbonToggleButton>(name);
            let weak_toggle = toggle.downgrade();
            let model = Rc::downgrade(&self.view_model());
            toggle.click(move |_, _| {
                if let (Some(toggle), Some(model)) = (weak_toggle.upgrade(), model.upgrade()) {
                    model
                        .page_action()
                        .execute(Some(&CadPageAction::Scene(action(
                            toggle.is_checked() == Some(true),
                        ))));
                }
            });
        }
        for (name, action) in [
            ("CycleButton", CadSceneAction::CycleMinimized),
            ("FloatLayersButton", CadSceneAction::FloatLayers),
            ("ReturnPanelsButton", CadSceneAction::ReturnPanels),
        ] {
            let model = Rc::downgrade(&self.view_model());
            self.named::<RibbonButton>(name).click(move |_, _| {
                if let Some(model) = model.upgrade() {
                    model
                        .page_action()
                        .execute(Some(&CadPageAction::Scene(action)));
                }
            });
        }
        for (name, action) in [
            (
                "CommandClose",
                CadPageAction::Scene(CadSceneAction::CloseCommandLine),
            ),
            ("AppMenuExit", CadPageAction::Exit),
            ("AppMenuOptions", CadPageAction::Options),
        ] {
            let model = Rc::downgrade(&self.view_model());
            self.named::<Button>(name).click(move |_, _| {
                if let Some(model) = model.upgrade() {
                    model.page_action().execute(Some(&action));
                }
            });
        }
        let model = Rc::downgrade(&self.view_model());
        self.layer_drop_down()
            .add_committed(Some(MarkupDelegate::new(move |args| {
                let e = from_markup_value::<
                    Rc<ferroribbon::controls::RibbonComboBoxCommittedEventArgs>,
                >(&args[1])
                .expect("CAD layer commit");
                let name = from_markup_value::<Rc<CadLayer>>(&e.item())
                    .map(|layer| layer.name().to_owned());
                if let Some(model) = model.upgrade() {
                    model
                        .page_action()
                        .execute(Some(&CadPageAction::LayerCommitted(name)));
                }
                None
            })));
        let model = Rc::downgrade(&self.view_model());
        self.subscriptions
            .borrow_mut()
            .push(self.application_menu().item_invoked(move |_, args| {
                if let Some(model) = model.upgrade() {
                    model
                        .page_action()
                        .execute(Some(&CadPageAction::ApplicationItem(args.item().label())));
                }
            }));
        let model = Rc::downgrade(&self.view_model());
        self.subscriptions
            .borrow_mut()
            .push(self.application_menu().recent_item_invoked(
                move |_, item: &Ref<RibbonApplicationMenuRecentItem>| {
                    if let Some(model) = model.upgrade() {
                        model
                            .page_action()
                            .execute(Some(&CadPageAction::RecentItem(item.title())));
                    }
                },
            ));
        let model = Rc::downgrade(&self.view_model());
        self.named::<CadDrawingCanvas>("Canvas")
            .add_cursor_moved(Some(MarkupDelegate::new(move |args| {
                let point = from_markup_value::<Point>(&args[1]).expect("CAD CursorMoved point");
                if let Some(model) = model.upgrade() {
                    model.page_action().execute(Some(&CadPageAction::Scene(
                        CadSceneAction::CursorMoved(point.x, point.y),
                    )));
                }
                None
            })));
        let page = self.to_ref().downgrade();
        let ribbon = self.cad_ribbon();
        let token = ribbon.add_property_changed_handler(Rc::new(move |args| {
            if args.property() == Ribbon::show_group_captions_property().as_property()
                && let Some(page) = page.upgrade()
            {
                page.named::<RibbonToggleButton>("PanelTitlesToggle")
                    .set_is_checked(Some(page.cad_ribbon().show_group_captions()));
            }
        }));
        let weak = ribbon.downgrade();
        self.subscriptions
            .borrow_mut()
            .push(Disposable::create(move || {
                if let Some(ribbon) = weak.upgrade() {
                    ribbon.remove_property_changed_handler(token);
                }
            }));
        self.apply_application_menu_presenter();
    }
    fn apply_application_menu_presenter(&self) {
        let Some(base) = Application::current()
            .and_then(|app| {
                app.resources()
                    .try_get_resource(&"RibbonFlyoutPresenterStyle".into(), None)
            })
            .flatten()
            .and_then(|v| ferroui_base::data::core::ValueTypes::as_object(&*v))
            .and_then(|v| v.cast::<ControlTheme>())
        else {
            return;
        };
        let theme = ControlTheme::with_setters(
            FlyoutPresenter::TYPE,
            [
                Setter::new(TemplatedControl::padding_property(), Thickness::uniform(0.)),
                Setter::new(
                    TemplatedControl::corner_radius_property(),
                    CornerRadius::uniform(2.),
                ),
            ],
        );
        theme.set_based_on(Some(base));
        self.named::<Flyout>("AppMenuFlyout")
            .set_flyout_presenter_theme(theme);
    }
    pub(super) fn apply_scene_action(&self, action: CadSceneAction) {
        let canvas = self.named::<CadDrawingCanvas>("Canvas");
        match action {
            CadSceneAction::Visibility(surface, visible) => {
                let name = match surface {
                    CadSurface::Ucs => "UcsIcon",
                    CadSurface::ViewCube => "ViewCube",
                    CadSurface::Navigation => "NavBar",
                    CadSurface::CommandLine => "CommandLine",
                    CadSurface::FileTabs => "FileTabsRow",
                    CadSurface::LayoutTabs => "LayoutTabs",
                };
                self.named::<Control>(name).set_is_visible(visible);
            }
            CadSceneAction::CloseCommandLine => {
                self.named::<Control>("CommandLine").set_is_visible(false);
                self.named::<RibbonToggleButton>("CommandLineToggle")
                    .set_is_checked(Some(false));
            }
            CadSceneAction::Grid(value) => canvas.set_is_grid_visible(value),
            CadSceneAction::Lineweights(value) => {
                canvas.set_show_lineweights(value);
                canvas.redraw();
            }
            CadSceneAction::Pan(value) => canvas.set_is_pan_mode(value),
            CadSceneAction::Model(value) => self
                .named::<RibbonToggleButton>("ModelToggle")
                .set_label(Some(if value { "MODEL" } else { "PAPER" }.into())),
            CadSceneAction::CleanScreen(value) => self.set_clean_screen(value),
            CadSceneAction::CycleMinimized => self.cad_ribbon().toggle_minimized(),
            CadSceneAction::PanelTitles(value) => self.cad_ribbon().set_show_group_captions(value),
            CadSceneAction::FloatLayers => self.float_layers_panel(),
            CadSceneAction::ReturnPanels => self.cad_ribbon().return_all_panels_to_ribbon(),
            CadSceneAction::CursorMoved(x, y) => self
                .named::<TextBlock>("Coordinates")
                .set_text(Some(&coordinate_text(x, y))),
        }
    }
    fn set_clean_screen(&self, clean: bool) {
        self.cad_ribbon().set_visibility_mode(if clean {
            ferroribbon_core::RibbonVisibilityMode::FullScreen
        } else {
            ferroribbon_core::RibbonVisibilityMode::AlwaysShow
        });
        self.named::<Control>("TitleBar").set_is_visible(!clean);
        self.named::<Control>("FileTabsRow").set_is_visible(
            !clean
                && self
                    .named::<RibbonToggleButton>("FileTabsToggle")
                    .is_checked()
                    == Some(true),
        );
    }
    pub fn float_layers_panel(&self) {
        let group = self.layers_panel();
        if group.is_floating() {
            return;
        }
        let canvas = self.named::<CadDrawingCanvas>("Canvas");
        let root = canvas
            .visual_root()
            .expect("CAD FloatLayersPanel requires a visual root");
        let origin = canvas
            .translate_point(Point::default(), &root)
            .expect("CAD canvas-to-root transform");
        group.float(Some(Point::new(origin.x + 120., origin.y + 70.)));
    }
    fn configure_application_menu(&self) {
        let flyout = self.named::<Flyout>("AppMenuFlyout");
        #[cfg(any(windows, target_arch = "wasm32"))]
        {
            // WinUI and Uno WebAssembly host the menu inside the owner's view.
            flyout.popup().set_should_use_overlay_layer(true);
            flyout
                .set_placement_constraint_adjustment(PopupPositionerConstraintAdjustment::SLIDE_X);
            flyout.set_vertical_offset(4.);
        }
        #[cfg(windows)]
        Self::limit_application_menu_to_owner(&flyout);
        #[cfg(not(target_arch = "wasm32"))]
        Self::close_application_menu_on_owner_resize(&flyout);
    }

    #[cfg(windows)]
    fn limit_application_menu_to_owner(flyout: &Ref<Flyout>) {
        let weak = flyout.downgrade();
        flyout
            .opening(move |_| {
                let Some(flyout) = weak.upgrade() else { return };
                let Some(anchor) = flyout.target() else {
                    return;
                };
                let Some(root) = TopLevel::get_top_level(Some(&anchor)) else {
                    return;
                };
                let Some(bottom) =
                    anchor.translate_point(Point::new(0., anchor.bounds().height), &root)
                else {
                    return;
                };
                let size = root.client_size();
                let popup = flyout.popup();
                popup.set_max_width(size.width);
                popup.set_max_height((size.height - bottom.y - flyout.vertical_offset()).max(0.));
            })
            .detach();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn close_application_menu_on_owner_resize(flyout: &Ref<Flyout>) {
        use ferroui_base::reactive::Subscription;
        use std::cell::RefCell;

        let resize_subscription = Rc::new(RefCell::new(None::<Subscription>));
        let active_subscription = resize_subscription.clone();
        let weak = flyout.downgrade();
        flyout
            .opened(move || {
                let Some(flyout) = weak.upgrade() else { return };
                let Some(root) = flyout
                    .target()
                    .and_then(|anchor| TopLevel::get_top_level(Some(&anchor)))
                else {
                    return;
                };
                let weak = flyout.downgrade();
                let subscription = root.property_changed(move |change| {
                    if change.property() == TopLevel::client_size_property().as_property()
                        && let Some(flyout) = weak.upgrade()
                    {
                        flyout.hide();
                    }
                });
                active_subscription.replace(Some(Subscription::new(subscription)));
            })
            .detach();
        flyout
            .closed(move || {
                resize_subscription.take();
            })
            .detach();
    }

    pub fn show_application_menu(&self) {
        let button = self
            .named::<ferroribbon::controls::RibbonTitleBar>("TitleBar")
            .get_visual_descendants()
            .filter_map(|v| v.cast::<Button>())
            .find(|b| b.name().as_deref() == Some("PART_AppButton"));
        self.cad_ribbon()
            .invoke_application_button_at(button.map(|button| button.upcast()));
    }
    pub(super) fn on_cad_loaded(&self) {
        self.apply_menu_icons()
            .expect("CAD loaded menu icon conversion");
        self.apply_chrome_brushes();
        self.update_document_tab_visuals();
        if self.initialized.replace(true) {
            return;
        }
        let size = self.bounds().size();
        self.named::<CadDrawingCanvas>("Canvas")
            .set_crosshair_cursor(initial_cursor(size.width, size.height));
    }

    pub(super) fn apply_chrome_brushes(&self) {
        let dark = self.actual_theme_variant() != Some(ThemeVariant::light());
        let fixed = |dark_argb, light_argb| {
            Some(Rc::<dyn IBrush>::from(SolidColorBrush::from_uint32(
                if dark { dark_argb } else { light_argb },
            )))
        };
        self.named::<TextBlock>("ViewportLabel")
            .set_foreground(fixed(0xffc9d1db, 0xff3f4855));
        self.named::<Ellipse>("CubeRing")
            .set_stroke(fixed(0xff5f6e86, 0xffa9b4c2));
        let cube_face = self.named::<Border>("CubeFace");
        cube_face.set_background(fixed(0xff4a5568, 0xfff7f8f9));
        cube_face.set_border_brush(fixed(0xff8c99ad, 0xff8c97a5));
        for name in ["CubeText", "CubeN", "CubeS", "CubeE", "CubeW", "WcsText"] {
            self.named::<TextBlock>(name)
                .set_foreground(fixed(0xffe1e6ec, 0xff2f3742));
        }
        self.named::<FontIcon>("WcsChevron")
            .set_foreground(self.named::<TextBlock>("CubeText").foreground());
        let wcs_badge = self.named::<Border>("WcsBadge");
        wcs_badge.set_background(fixed(0xff3b4453, 0xfff0f1f3));
        wcs_badge.set_border_brush(cube_face.border_brush());

        let nav_bar = self.named::<Border>("NavBar");
        nav_bar.set_background(Some(
            self.chrome_resource_brush("RibbonCommandBarBackgroundBrush"),
        ));
        nav_bar.set_border_brush(Some(self.chrome_resource_brush("RibbonPopupBorderBrush")));
        self.named::<Border>("CommandHistoryBox")
            .set_background(fixed(0xd92b313b, 0xe6f0f1f3));
        self.named::<TextBlock>("CommandHistory")
            .set_foreground(fixed(0xffaeb8c4, 0xff4e5866));
        let command_bar = self.named::<Border>("CommandBar");
        command_bar.set_background(Some(
            self.chrome_resource_brush("RibbonCommandBarBackgroundBrush"),
        ));
        command_bar.set_border_brush(Some(self.chrome_resource_brush("RibbonPopupBorderBrush")));
        self.named::<TextBlock>("CommandPrompt")
            .set_foreground(Some(
                self.chrome_resource_brush("RibbonSecondaryForegroundBrush"),
            ));

        self.named::<Rectangle>("MTextFrame")
            .set_stroke(fixed(0xffe1e6ec, 0xff2f3742));
        let text = self.named::<TextBlock>("MTextText");
        text.set_foreground(fixed(0xfff2f2f2, 0xff1f242b));
        self.named::<Rectangle>("MTextCaret")
            .set_fill(text.foreground());
    }

    pub(super) fn chrome_resource_brush(&self, key: &str) -> Rc<dyn IBrush> {
        RibbonTheme::get_brush(Some(self), Some(key))
            .expect("nonempty CAD theme brush key")
            .map(Into::into)
            .unwrap_or_else(|| SolidColorBrush::with_color(Colors::GRAY).into())
    }
}
pub(super) fn initial_cursor(width: f64, height: f64) -> Point {
    let y = height * 0.32;
    Point::new(width * 0.62, if y.is_nan() { y } else { y.max(200.) })
}
pub(super) fn coordinate_text(x: f64, y: f64) -> String {
    use ferroui_base::data::converters::composite_format::{FormatArg, format};
    format(
        "{0:0.0000}, {1:0.0000}, 0.0000",
        &[FormatArg::F64(x), FormatArg::F64(y)],
    )
    .expect("fixed invariant CAD coordinate format")
}
