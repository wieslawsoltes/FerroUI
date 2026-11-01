//! Shared shell chrome, with native button semantics, accessible names and dimensioned
//! path artwork independent of the installed symbol fonts.
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, Geometry, Stretch, TextTrimming};
use ferroui_controls::{Border, Button, StackPanel, TextBlock};
use ferroui_view::prelude::*;

use super::appearance::{Appearance, Layout, Palette};
use ferroui_markup_xaml::markup_extensions::DynamicResourceExtension;
use std::rc::Rc;

pub(super) fn look() -> Appearance {
    use_context::<Signal<Appearance>>()
        .expect("shell appearance context")
        .get_untracked()
}
pub(super) fn colors() -> Palette {
    look().palette()
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) enum Icon {
    Mark,
    Pin,
    Unpin,
    Bell,
    Settings,
    Sound,
    Brightness,
    Network,
    Battery,
    Applications,
    Workspaces,
    Search,
    Close,
    Back,
    Forward,
    Window,
    Minimize,
    Maximize,
    RestoreWindow,
    More,
    Appearance,
}

// The logo consists of two interlocking folded strips; the other glyphs are
// compact strokes inside a 24-unit square, optically centered in their buttons.
pub(super) fn glyph(icon: Icon, size: f64, color: &'static str) -> Element {
    let data = match icon {
        Icon::Pin => "M7 2 H17 V4 L15 5 V11 L18 14 V16 H13 V22 L11 20 V16 H6 V14 L9 11 V5 L7 4 Z",
        Icon::Unpin => "M4 2 L22 20 L20 22 L13 15 V22 L11 20 V16 H6 V14 L9 11 V10 L2 4 Z M10 2 H17 V4 L15 5 V8 Z",
        Icon::Bell => "M12 1 A2 2 0 0 1 14 3 Q19 4 19 11 V15 L21 18 V19 H3 V18 L5 15 V11 Q5 4 10 3 A2 2 0 0 1 12 1 Z M12 5 Q7 5 7 11 V16 L6 17 H18 L17 16 V11 Q17 5 12 5 Z M9 21 H15 Q12 25 9 21 Z",
        Icon::Settings => "M1 6 H5 V4 H9 V6 H23 V8 H9 V10 H5 V8 H1 Z M1 16 H15 V14 H19 V16 H23 V18 H19 V20 H15 V18 H1 Z",
        Icon::Sound => "M2 8 H6 L12 3 V21 L6 16 H2 Z M15 7 Q21 12 15 17 V14 Q18 12 15 10 Z M17 2 Q29 12 17 22 V19 Q25 12 17 5 Z",
        Icon::Brightness => "M11 0 H13 V4 H11 Z M11 20 H13 V24 H11 Z M0 11 H4 V13 H0 Z M20 11 H24 V13 H20 Z M12 6 A6 6 0 1 1 12 18 A6 6 0 1 1 12 6 Z",
        Icon::Network => "M0 7 Q12 -3 24 7 L22 9 Q12 1 2 9 Z M4 12 Q12 5 20 12 L18 14 Q12 9 6 14 Z M8 17 Q12 13 16 17 L12 22 Z",
        Icon::Battery => "M1 5 H21 V9 H23 V15 H21 V19 H1 Z M3 7 V17 H19 V7 Z M5 9 H13 V15 H5 Z",
        Icon::Mark => "F1 M2 15 L10 2 H17 L9 15 Z M8 21 L15 9 H22 L15 21 Z",
        Icon::Applications => "F1 M2 2 H10 V10 H2 Z M14 2 H22 V10 H14 Z M2 14 H10 V22 H2 Z M14 14 H22 V22 H14 Z",
        Icon::Workspaces => "M2 3 H18 V6 H5 V17 H2 Z M7 8 H22 V21 H7 Z M10 11 V18 H19 V11 Z",
        Icon::Search => "M9 1 A8 8 0 1 0 14 15 L21 22 L23 20 L16 13 A8 8 0 0 0 9 1 Z M9 4 A5 5 0 1 1 9 14 A5 5 0 1 1 9 4 Z",
        Icon::Close => "M4 2 L12 10 L20 2 L22 4 L14 12 L22 20 L20 22 L12 14 L4 22 L2 20 L10 12 L2 4 Z",
        Icon::Back => "M15 2 L5 12 L15 22 L17 20 L9 12 L17 4 Z",
        Icon::Forward => "M9 2 L19 12 L9 22 L7 20 L15 12 L7 4 Z",
        Icon::Window => "M1 3 H23 V21 H1 Z M4 8 V18 H20 V8 Z",
        Icon::Maximize => "M2 2 H22 V22 H2 Z M4 5 V20 H20 V5 Z",
        Icon::RestoreWindow => "M7 2 H22 V17 H19 V14 H20 V5 H9 V7 H7 Z M2 8 H17 V23 H2 Z M4 11 V21 H15 V11 Z",
        Icon::More => "M1 10 H5 V14 H1 Z M10 10 H14 V14 H10 Z M19 10 H23 V14 H19 Z",
        Icon::Minimize => "M2 10 H22 V13 H2 Z",
        Icon::Appearance => "M12 1 A11 11 0 1 0 12 23 A11 11 0 1 0 12 1 Z M12 4 A8 8 0 0 1 12 20 Z",
    };
    // Path controls can share a geometry. The parsed glyph never changes and
    // all of its consumers live on the UI thread.
    thread_local! {
        static GLYPHS: std::cell::RefCell<std::collections::HashMap<Icon, ferroui_base::Ref<Geometry>>> =
            std::cell::RefCell::default();
    }
    let geometry = GLYPHS.with(|glyphs| {
        glyphs
            .borrow_mut()
            .entry(icon)
            .or_insert_with(|| Geometry::parse(data).expect("shell icon"))
            .clone()
    });
    path()
        .data(geometry)
        .fill(color)
        .stretch(Stretch::Uniform)
        .width(size)
        .height(size)
        .horizontal_alignment(H::Center)
        .vertical_alignment(V::Center)
        .is_hit_test_visible(false)
        .into()
}

pub(super) enum Ink {
    Primary,
    Secondary,
    Accent,
    Fixed(&'static str),
}
impl From<&'static str> for Ink {
    fn from(color: &'static str) -> Self {
        Self::Fixed(color)
    }
}

/// Resolves against the active compiled theme and follows resource or variant changes.
pub(super) fn resource(key: &'static str) -> Bind<Rc<DynamicResourceExtension>> {
    Bind(DynamicResourceExtension::with_resource_key(Some(Rc::new(
        key.to_owned(),
    ))))
}

pub(super) fn shell_text(value: &str, size: f64, color: impl Into<Ink>) -> El<TextBlock> {
    let text = text_block()
        .text(value)
        .font_size(size)
        .font_family(resource("ContentControlThemeFontFamily"))
        .vertical_alignment(V::Center)
        .text_trimming(<dyn TextTrimming>::character_ellipsis());
    match color.into() {
        Ink::Primary => text.foreground(resource("TextFillColorPrimaryBrush")),
        Ink::Secondary => text.foreground(resource("TextFillColorSecondaryBrush")),
        Ink::Accent => text.foreground(resource("AccentTextFillColorPrimaryBrush")),
        Ink::Fixed(color) => text.foreground(color),
    }
}

pub(super) fn eyebrow(value: &str) -> El<TextBlock> {
    shell_text(value, 10.0, colors().accent)
        .letter_spacing(2.2)
        .font_weight(FontWeight::SemiBold)
}

pub(super) fn row(spacing: f64) -> El<StackPanel> {
    stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(spacing)
        .vertical_alignment(V::Center)
}

pub(super) fn shell_panel() -> El<Border> {
    let frame = if look().winui().is_some() {
        border().background(resource("SolidBackgroundFillColorSecondaryBrush"))
            .border_brush(resource("SurfaceStrokeColorFlyoutBrush"))
    } else {
        border().background(colors().surface)
            .border_brush(resource("ControlStrokeColorDefaultBrush"))
    };
    frame
        .border_thickness(1)
        .corner_radius(colors().radius)
}

pub(super) fn action(name: &str, callback: impl Fn() + 'static) -> El<Button> {
    let button = if look().layout() == Layout::BottomNavigation {
        button().class("tonal")
    } else {
        button()
    };
    let button = if look().winui().is_some() {
        button.padding(resource("ButtonPadding"))
    } else {
        button.padding((12.0, 6.0))
    };
    button
        .automation_properties_name(name)
        .tool_tip_tip(name)
        .horizontal_content_alignment(H::Center)
        .vertical_content_alignment(V::Center)
        .on_click(move |_| callback())
}

pub(super) fn icon_action(name: &str, icon: Icon, callback: impl Fn() + 'static) -> El<Button> {
    action(name, callback)
        .width(34.0)
        .height(34.0)
        .padding(8.0)
        .child(glyph(icon, 15.0, colors().text))
}
