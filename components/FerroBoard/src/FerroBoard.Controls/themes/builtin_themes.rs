// Generated from Themes/KeyboardThemes.cs private palette algorithms and call table.
// See ../NOTICE.md. Regenerate with scripts/generate_builtin_themes.py.
#[allow(clippy::too_many_arguments)]
fn floris(
    id: &str,
    name: &str,
    dark: bool,
    borderless: bool,
    primary: &str,
    primary_variant: &str,
    secondary: &str,
    background: &str,
    surface: &str,
    surface_variant: &str,
    popup_surface: &str,
    focused_popup: &str,
    on_background: &str,
    on_surface: &str,
    on_surface_variant: &str,
    one_hand_bg: &str,
    one_hand_fg: &str,
) -> Arc<KeyboardTheme> {
    let t = KeyboardTheme::new();
    t.set_id(Some(id.into()));
    t.set_name(Some(name.into()));
    t.set_is_dark(dark);
    t.set_background(hex(background));
    t.set_key_background(if borderless {
        hex("#00000000")
    } else {
        hex(surface)
    });
    t.set_key_pressed_background(hex(surface_variant));
    t.set_key_foreground(hex(on_surface));
    t.set_key_hint_foreground(hex(on_surface_variant));
    t.set_function_key_background(if borderless {
        hex("#00000000")
    } else {
        hex(surface)
    });
    t.set_function_key_pressed_background(hex(surface_variant));
    t.set_function_key_foreground(hex(on_surface));
    t.set_accent_key_background(hex(primary));
    t.set_accent_key_pressed_background(hex(primary_variant));
    t.set_accent_key_foreground(hex("#F0F0F0"));
    t.set_space_key_foreground(hex(on_surface_variant));
    t.set_caps_lock_foreground(hex(secondary));
    t.set_key_shadow_color(if borderless {
        hex("#00000000")
    } else if dark {
        hex("#66000000")
    } else {
        hex("#33000000")
    });
    t.set_key_shadow_depth(if borderless { 0.0 } else { 1.5 });
    t.set_popup_background(hex(popup_surface));
    t.set_popup_foreground(hex(on_surface));
    t.set_popup_selected_background(hex(focused_popup));
    t.set_popup_selected_foreground(hex(on_surface));
    t.set_popup_border_color(if dark {
        hex("#33FFFFFF")
    } else {
        hex("#1F000000")
    });
    t.set_smartbar_background(hex(background));
    t.set_smartbar_foreground(hex(on_background));
    t.set_suggestion_highlight_foreground(hex(primary));
    t.set_divider_color(if dark {
        hex("#40FFFFFF")
    } else {
        hex("#40000000")
    });
    t.set_action_button_pressed_background(if dark {
        hex("#26FFFFFF")
    } else {
        hex("#1F000000")
    });
    t.set_panel_background(hex(background));
    t.set_panel_foreground(hex(on_background));
    t.set_panel_item_background(hex(surface));
    t.set_panel_selected_foreground(hex(primary));
    t.set_one_handed_background(hex(one_hand_bg));
    t.set_one_handed_foreground(hex(one_hand_fg));
    t.set_glide_trail_color(hex(primary));
    t
}
#[allow(clippy::too_many_arguments)]
fn custom(
    id: &str,
    name: &str,
    dark: bool,
    background: &str,
    key: &str,
    key_pressed: &str,
    key_fg: &str,
    function: &str,
    function_pressed: &str,
    accent: &str,
    accent_pressed: &str,
    accent_fg: &str,
    hint: &str,
    popup: &str,
    popup_selected: &str,
    trail: &str,
    corner: &str,
    font: Option<&str>,
    secondary: Option<&str>,
) -> Arc<KeyboardTheme> {
    let t = KeyboardTheme::new();
    t.set_id(Some(id.into()));
    t.set_name(Some(name.into()));
    t.set_is_dark(dark);
    t.set_background(hex(background));
    t.set_key_background(hex(key));
    t.set_key_pressed_background(hex(key_pressed));
    t.set_key_foreground(hex(key_fg));
    t.set_key_hint_foreground(hex(hint));
    t.set_function_key_background(hex(function));
    t.set_function_key_pressed_background(hex(function_pressed));
    t.set_function_key_foreground(hex(key_fg));
    t.set_accent_key_background(hex(accent));
    t.set_accent_key_pressed_background(hex(accent_pressed));
    t.set_accent_key_foreground(hex(accent_fg));
    t.set_space_key_foreground(hex(hint));
    t.set_caps_lock_foreground(hex(secondary.unwrap_or(accent)));
    t.set_key_shadow_color(if dark {
        hex("#55000000")
    } else {
        hex("#2A000000")
    });
    t.set_key_shadow_depth(1.0);
    t.set_key_corner_radius(corner.parse::<f64>().expect("pinned theme corner"));
    t.set_popup_background(hex(popup));
    t.set_popup_foreground(hex(key_fg));
    t.set_popup_selected_background(hex(popup_selected));
    t.set_popup_selected_foreground(hex(accent_fg));
    t.set_popup_border_color(if dark {
        hex("#33FFFFFF")
    } else {
        hex("#1F000000")
    });
    t.set_smartbar_background(hex(background));
    t.set_smartbar_foreground(hex(key_fg));
    t.set_suggestion_highlight_foreground(hex(accent));
    t.set_divider_color(if dark {
        hex("#40FFFFFF")
    } else {
        hex("#33000000")
    });
    t.set_action_button_pressed_background(if dark {
        hex("#26FFFFFF")
    } else {
        hex("#1F000000")
    });
    t.set_panel_background(hex(background));
    t.set_panel_foreground(hex(key_fg));
    t.set_panel_item_background(hex(key));
    t.set_panel_selected_foreground(hex(accent));
    t.set_one_handed_background(hex(function));
    t.set_one_handed_foreground(hex(key_fg));
    t.set_glide_trail_color(hex(trail));
    t.set_font_family(font.map(Into::into));
    t
}
fn create_built_in() -> Vec<Arc<KeyboardTheme>> {
    vec![
        floris(
            "floris_day",
            "Floris Day",
            false,
            false,
            "#4CAF50",
            "#388E3C",
            "#FF9800",
            "#E0E0E0",
            "#FFFFFF",
            "#F5F5F5",
            "#EEEEEE",
            "#BDBDBD",
            "#121212",
            "#000000",
            "#5F5F5F",
            "#E8F5E9",
            "#424242",
        ),
        floris(
            "floris_day_borderless",
            "Floris Day (borderless)",
            false,
            true,
            "#4CAF50",
            "#388E3C",
            "#FF9800",
            "#E0E0E0",
            "#FFFFFF",
            "#D0D0D0",
            "#EEEEEE",
            "#BDBDBD",
            "#121212",
            "#000000",
            "#5F5F5F",
            "#E8F5E9",
            "#424242",
        ),
        floris(
            "floris_night",
            "Floris Night",
            true,
            false,
            "#4CAF50",
            "#388E3C",
            "#F57C00",
            "#212121",
            "#424242",
            "#616161",
            "#757575",
            "#BDBDBD",
            "#DCDCDC",
            "#FFFFFF",
            "#A0A0A0",
            "#1B5E20",
            "#EEEEEE",
        ),
        floris(
            "floris_night_borderless",
            "Floris Night (borderless)",
            true,
            true,
            "#4CAF50",
            "#388E3C",
            "#F57C00",
            "#212121",
            "#424242",
            "#3A3A3A",
            "#757575",
            "#BDBDBD",
            "#DCDCDC",
            "#FFFFFF",
            "#A0A0A0",
            "#1B5E20",
            "#EEEEEE",
        ),
        floris(
            "floris_pure_night",
            "Floris Pure Night (AMOLED)",
            true,
            false,
            "#388E3C",
            "#306D32",
            "#FF9800",
            "#000000",
            "#212121",
            "#3D3D3D",
            "#424242",
            "#707070",
            "#EEEEEE",
            "#EEEEEE",
            "#B3FFFFFF",
            "#1B5E20",
            "#EEEEEE",
        ),
        custom(
            "material_light",
            "Material Light",
            false,
            "#E8EAED",
            "#FFFFFF",
            "#DADCE0",
            "#202124",
            "#D2D5DA",
            "#BDC1C6",
            "#1A73E8",
            "#1557B0",
            "#FFFFFF",
            "#5F6368",
            "#FFFFFF",
            "#1A73E8",
            "#1A73E8",
            "6",
            None,
            None,
        ),
        custom(
            "material_dark",
            "Material Dark",
            true,
            "#202124",
            "#3C4043",
            "#5F6368",
            "#E8EAED",
            "#2D2F31",
            "#4A4D51",
            "#8AB4F8",
            "#AECBFA",
            "#202124",
            "#9AA0A6",
            "#3C4043",
            "#8AB4F8",
            "#8AB4F8",
            "6",
            None,
            None,
        ),
        custom(
            "ocean",
            "Ocean",
            true,
            "#0B1D2E",
            "#16324A",
            "#21476A",
            "#E3F2FD",
            "#10273C",
            "#1A3A58",
            "#00B8D4",
            "#0097A7",
            "#00202A",
            "#7FA7C9",
            "#1C3D5C",
            "#00B8D4",
            "#18FFFF",
            "10",
            None,
            Some("#FFB74D"),
        ),
        custom(
            "rose", "Rosé", false, "#FBE9EC", "#FFFFFF", "#F8D7DD", "#3E2327", "#F5D0D7",
            "#EDB8C2", "#D81B60", "#AD1457", "#FFFFFF", "#9C6B74", "#FFFFFF", "#D81B60", "#EC407A",
            "12", None, None,
        ),
        custom(
            "nord",
            "Nord",
            true,
            "#2E3440",
            "#3B4252",
            "#4C566A",
            "#ECEFF4",
            "#343B48",
            "#434C5E",
            "#88C0D0",
            "#81A1C1",
            "#2E3440",
            "#A3ABB9",
            "#434C5E",
            "#88C0D0",
            "#88C0D0",
            "8",
            None,
            Some("#EBCB8B"),
        ),
        custom(
            "high_contrast",
            "High Contrast",
            true,
            "#000000",
            "#000000",
            "#333333",
            "#FFFFFF",
            "#000000",
            "#333333",
            "#FFFF00",
            "#CCCC00",
            "#000000",
            "#FFFF00",
            "#000000",
            "#FFFF00",
            "#FFFF00",
            "4",
            None,
            Some("#00FFFF"),
        ),
        custom(
            "terminal",
            "Terminal",
            true,
            "#000000",
            "#0A0F0A",
            "#133313",
            "#33FF66",
            "#050805",
            "#0F260F",
            "#1B5E20",
            "#2E7D32",
            "#CCFFCC",
            "#1FAA4A",
            "#0A160A",
            "#33FF66",
            "#33FF66",
            "2",
            Some("Courier New"),
            None,
        ),
    ]
}
