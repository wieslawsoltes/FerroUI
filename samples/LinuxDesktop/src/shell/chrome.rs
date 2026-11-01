//! Shared shell chrome, with native button semantics, accessible names and dimensioned
//! path artwork independent of the installed symbol fonts.
use ferroui_base::layout::{HorizontalAlignment as H, Orientation, VerticalAlignment as V};
use ferroui_base::media::{FontWeight, Geometry, Stretch, TextTrimming};
use ferroui_controls::{Border, Button, StackPanel, TextBlock};
use ferroui_view::prelude::*;

use super::appearance::{Appearance, Layout};
use ferroui_markup_xaml::markup_extensions::DynamicResourceExtension;
use std::rc::Rc;

#[derive(Clone, Copy)]
pub(super) struct Palette {
    pub text: &'static str,
    pub muted: &'static str,
    pub accent: &'static str,
    pub edge: &'static str,
    pub surface: &'static str,
    pub tile: &'static str,
    pub selected: &'static str,
    pub on_selected: &'static str,
    pub radius: i32,
}

pub(super) fn palette(look: Appearance) -> Palette {
    match look.layout() {
        Layout::Taskbar => Palette {
            text: "#1C2025",
            muted: "#555E69",
            accent: "#005FB8",
            edge: "#C3CBD6",
            surface: "#F2F5FA",
            tile: "#E7EDF5",
            selected: "#D1E8FC",
            on_selected: "#003868",
            radius: 9,
        },
        Layout::MenuBarDock => Palette {
            text: "#FBF7FE",
            muted: "#D4CADA",
            accent: "#DFC2FE",
            edge: "#716477",
            surface: "#342F3C",
            tile: "#423C4B",
            selected: "#78519B",
            on_selected: "#FFFFFF",
            radius: 19,
        },
        Layout::BottomNavigation => Palette {
            text: "#EFF4E9",
            muted: "#B9C9B9",
            accent: "#A0D5B7",
            edge: "#41564D",
            surface: "#182922",
            tile: "#253A30",
            selected: "#385D48",
            on_selected: "#DBFFE3",
            radius: 28,
        },
    }
}

pub(super) fn look() -> Appearance {
    use_context::<Signal<Appearance>>()
        .expect("shell appearance context")
        .get_untracked()
}
pub(super) fn colors() -> Palette {
    palette(look())
}

#[derive(Clone, Copy)]
pub(super) enum Icon {
    Mark,
    Applications,
    Workspaces,
    Search,
    Close,
    Back,
    Forward,
    Window,
    Minimize,
    Move,
    Appearance,
}

// The logo consists of two interlocking folded strips; the other glyphs are
// compact strokes inside a 24-unit square, optically centered in their buttons.
pub(super) fn glyph(icon: Icon, size: f64, color: &'static str) -> Element {
    let data = match icon {
        Icon::Mark => "F1 M2 15 L10 2 H17 L9 15 Z M8 21 L15 9 H22 L15 21 Z",
        Icon::Applications => "F1 M2 2 H10 V10 H2 Z M14 2 H22 V10 H14 Z M2 14 H10 V22 H2 Z M14 14 H22 V22 H14 Z",
        Icon::Workspaces => "M2 3 H18 V6 H5 V17 H2 Z M7 8 H22 V21 H7 Z M10 11 V18 H19 V11 Z",
        Icon::Search => "M9 1 A8 8 0 1 0 14 15 L21 22 L23 20 L16 13 A8 8 0 0 0 9 1 Z M9 4 A5 5 0 1 1 9 14 A5 5 0 1 1 9 4 Z",
        Icon::Close => "M4 2 L12 10 L20 2 L22 4 L14 12 L22 20 L20 22 L12 14 L4 22 L2 20 L10 12 L2 4 Z",
        Icon::Back => "M15 2 L5 12 L15 22 L17 20 L9 12 L17 4 Z",
        Icon::Forward => "M9 2 L19 12 L9 22 L7 20 L15 12 L7 4 Z",
        Icon::Window => "M1 3 H23 V21 H1 Z M4 8 V18 H20 V8 Z",
        Icon::Minimize => "M2 10 H22 V13 H2 Z",
        Icon::Appearance => "M12 1 A11 11 0 1 0 12 23 A11 11 0 1 0 12 1 Z M12 4 A8 8 0 0 1 12 20 Z",
        Icon::Move => "M1 11 H16 L11 6 L13 4 L21 12 L13 20 L11 18 L16 13 H1 Z",
    };
    path()
        .data(Geometry::parse(data).expect("shell icon"))
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

pub(super) fn label(value: &str, size: f64, color: impl Into<Ink>) -> El<TextBlock> {
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
    label(value, 10.0, Ink::Accent)
        .letter_spacing(2.2)
        .font_weight(FontWeight::SemiBold)
}

pub(super) fn row(spacing: f64) -> El<StackPanel> {
    stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(spacing)
        .vertical_alignment(V::Center)
}

pub(super) fn panel() -> El<Border> {
    border()
        .background(colors().surface)
        .border_brush(resource("ControlStrokeColorDefaultBrush"))
        .border_thickness(1)
        .corner_radius(colors().radius)
}

pub(super) fn action(name: &str, callback: impl Fn() + 'static) -> El<Button> {
    button()
        .automation_properties_name(name)
        .tool_tip_tip(name)
        .horizontal_content_alignment(H::Center)
        .vertical_content_alignment(V::Center)
        .padding((12.0, 6.0))
        .on_click(move |_| callback())
}

pub(super) fn icon_action(name: &str, icon: Icon, callback: impl Fn() + 'static) -> El<Button> {
    action(name, callback)
        .width(34.0)
        .height(34.0)
        .padding(8.0)
        .child(glyph(icon, 15.0, colors().text))
}
