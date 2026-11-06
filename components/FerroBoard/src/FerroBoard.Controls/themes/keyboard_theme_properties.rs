// Generated from Themes/KeyboardTheme.cs; see ../NOTICE.md.
pub struct KeyboardTheme {
    state: RwLock<ThemeState>,
}
struct ThemeState {
    id: Option<Utf16Text>,
    name: Option<Utf16Text>,
    is_dark: bool,
    background: Color,
    key_background: Color,
    key_pressed_background: Color,
    key_foreground: Color,
    key_hint_foreground: Color,
    function_key_background: Color,
    function_key_pressed_background: Color,
    function_key_foreground: Color,
    accent_key_background: Color,
    accent_key_pressed_background: Color,
    accent_key_foreground: Color,
    space_key_foreground: Color,
    caps_lock_foreground: Color,
    key_border_color: Color,
    key_border_thickness: f64,
    key_shadow_color: Color,
    key_shadow_depth: f64,
    key_corner_radius: f64,
    popup_background: Color,
    popup_foreground: Color,
    popup_selected_background: Color,
    popup_selected_foreground: Color,
    popup_border_color: Color,
    popup_corner_radius: f64,
    smartbar_background: Color,
    smartbar_foreground: Color,
    suggestion_highlight_foreground: Color,
    divider_color: Color,
    action_button_background: Color,
    action_button_pressed_background: Color,
    panel_background: Color,
    panel_foreground: Color,
    panel_item_background: Color,
    panel_selected_foreground: Color,
    one_handed_background: Color,
    one_handed_foreground: Color,
    glide_trail_color: Color,
    glide_trail_thickness: f64,
    font_family: Option<Utf16Text>,
    key_font_size: f64,
    function_key_font_size: f64,
    hint_font_size: f64,
    space_font_size: f64,
    icon_size: f64,
    smartbar_font_size: f64,
    emoji_font_size: f64,
    uppercase_labels_when_shifted: bool,
    key_rules: Option<ThemeList<Option<Arc<KeyStyleRule>>>>,
}
impl Default for KeyboardTheme {
    fn default() -> Self {
        Self {
            state: RwLock::new(ThemeState {
                id: Some(Utf16Text::from("custom")),
                name: Some(Utf16Text::from("Custom")),
                is_dark: false,
                background: Color::from_uint32(0xFFE0E0E0),
                key_background: Color::from_uint32(0xFFFFFFFF),
                key_pressed_background: Color::from_uint32(0xFFF5F5F5),
                key_foreground: Color::from_uint32(0xFF000000),
                key_hint_foreground: Color::from_uint32(0xFF5F5F5F),
                function_key_background: Color::from_uint32(0xFFFFFFFF),
                function_key_pressed_background: Color::from_uint32(0xFFF5F5F5),
                function_key_foreground: Color::from_uint32(0xFF000000),
                accent_key_background: Color::from_uint32(0xFF4CAF50),
                accent_key_pressed_background: Color::from_uint32(0xFF388E3C),
                accent_key_foreground: Color::from_uint32(0xFFFFFFFF),
                space_key_foreground: Color::from_uint32(0xFF5F5F5F),
                caps_lock_foreground: Color::from_uint32(0xFFFF9800),
                key_border_color: Color::from_uint32(0x00000000),
                key_border_thickness: 0.0,
                key_shadow_color: Color::from_uint32(0x33000000),
                key_shadow_depth: 1.5,
                key_corner_radius: 8.0,
                popup_background: Color::from_uint32(0xFFEEEEEE),
                popup_foreground: Color::from_uint32(0xFF000000),
                popup_selected_background: Color::from_uint32(0xFFBDBDBD),
                popup_selected_foreground: Color::from_uint32(0xFF000000),
                popup_border_color: Color::from_uint32(0x1F000000),
                popup_corner_radius: 10.0,
                smartbar_background: Color::from_uint32(0xFFE0E0E0),
                smartbar_foreground: Color::from_uint32(0xFF121212),
                suggestion_highlight_foreground: Color::from_uint32(0xFF388E3C),
                divider_color: Color::from_uint32(0x40000000),
                action_button_background: Color::from_uint32(0x00000000),
                action_button_pressed_background: Color::from_uint32(0x1F000000),
                panel_background: Color::from_uint32(0xFFE0E0E0),
                panel_foreground: Color::from_uint32(0xFF121212),
                panel_item_background: Color::from_uint32(0xFFFFFFFF),
                panel_selected_foreground: Color::from_uint32(0xFF4CAF50),
                one_handed_background: Color::from_uint32(0xFFE8F5E9),
                one_handed_foreground: Color::from_uint32(0xFF424242),
                glide_trail_color: Color::from_uint32(0xFF4CAF50),
                glide_trail_thickness: 7.0,
                font_family: None,
                key_font_size: 22.0,
                function_key_font_size: 16.0,
                hint_font_size: 11.0,
                space_font_size: 13.0,
                icon_size: 22.0,
                smartbar_font_size: 16.0,
                emoji_font_size: 28.0,
                uppercase_labels_when_shifted: true,
                key_rules: Some(ThemeList::default()),
            }),
        }
    }
}
impl KeyboardTheme {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }
    pub fn id(&self) -> Option<Utf16Text> {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .id
            .clone()
    }
    pub fn set_id(&self, value: Option<Utf16Text>) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .id = value;
    }
    pub fn name(&self) -> Option<Utf16Text> {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .name
            .clone()
    }
    pub fn set_name(&self, value: Option<Utf16Text>) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .name = value;
    }
    pub fn is_dark(&self) -> bool {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_dark
    }
    pub fn set_is_dark(&self, value: bool) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_dark = value;
    }
    pub fn background(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .background
    }
    pub fn set_background(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .background = value;
    }
    pub fn key_background(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_background
    }
    pub fn set_key_background(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_background = value;
    }
    pub fn key_pressed_background(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_pressed_background
    }
    pub fn set_key_pressed_background(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_pressed_background = value;
    }
    pub fn key_foreground(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_foreground
    }
    pub fn set_key_foreground(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_foreground = value;
    }
    pub fn key_hint_foreground(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_hint_foreground
    }
    pub fn set_key_hint_foreground(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_hint_foreground = value;
    }
    pub fn function_key_background(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .function_key_background
    }
    pub fn set_function_key_background(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .function_key_background = value;
    }
    pub fn function_key_pressed_background(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .function_key_pressed_background
    }
    pub fn set_function_key_pressed_background(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .function_key_pressed_background = value;
    }
    pub fn function_key_foreground(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .function_key_foreground
    }
    pub fn set_function_key_foreground(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .function_key_foreground = value;
    }
    pub fn accent_key_background(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .accent_key_background
    }
    pub fn set_accent_key_background(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .accent_key_background = value;
    }
    pub fn accent_key_pressed_background(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .accent_key_pressed_background
    }
    pub fn set_accent_key_pressed_background(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .accent_key_pressed_background = value;
    }
    pub fn accent_key_foreground(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .accent_key_foreground
    }
    pub fn set_accent_key_foreground(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .accent_key_foreground = value;
    }
    pub fn space_key_foreground(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .space_key_foreground
    }
    pub fn set_space_key_foreground(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .space_key_foreground = value;
    }
    pub fn caps_lock_foreground(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .caps_lock_foreground
    }
    pub fn set_caps_lock_foreground(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .caps_lock_foreground = value;
    }
    pub fn key_border_color(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_border_color
    }
    pub fn set_key_border_color(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_border_color = value;
    }
    pub fn key_border_thickness(&self) -> f64 {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_border_thickness
    }
    pub fn set_key_border_thickness(&self, value: f64) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_border_thickness = value;
    }
    pub fn key_shadow_color(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_shadow_color
    }
    pub fn set_key_shadow_color(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_shadow_color = value;
    }
    pub fn key_shadow_depth(&self) -> f64 {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_shadow_depth
    }
    pub fn set_key_shadow_depth(&self, value: f64) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_shadow_depth = value;
    }
    pub fn key_corner_radius(&self) -> f64 {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_corner_radius
    }
    pub fn set_key_corner_radius(&self, value: f64) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_corner_radius = value;
    }
    pub fn popup_background(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .popup_background
    }
    pub fn set_popup_background(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .popup_background = value;
    }
    pub fn popup_foreground(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .popup_foreground
    }
    pub fn set_popup_foreground(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .popup_foreground = value;
    }
    pub fn popup_selected_background(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .popup_selected_background
    }
    pub fn set_popup_selected_background(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .popup_selected_background = value;
    }
    pub fn popup_selected_foreground(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .popup_selected_foreground
    }
    pub fn set_popup_selected_foreground(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .popup_selected_foreground = value;
    }
    pub fn popup_border_color(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .popup_border_color
    }
    pub fn set_popup_border_color(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .popup_border_color = value;
    }
    pub fn popup_corner_radius(&self) -> f64 {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .popup_corner_radius
    }
    pub fn set_popup_corner_radius(&self, value: f64) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .popup_corner_radius = value;
    }
    pub fn smartbar_background(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .smartbar_background
    }
    pub fn set_smartbar_background(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .smartbar_background = value;
    }
    pub fn smartbar_foreground(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .smartbar_foreground
    }
    pub fn set_smartbar_foreground(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .smartbar_foreground = value;
    }
    pub fn suggestion_highlight_foreground(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .suggestion_highlight_foreground
    }
    pub fn set_suggestion_highlight_foreground(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .suggestion_highlight_foreground = value;
    }
    pub fn divider_color(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .divider_color
    }
    pub fn set_divider_color(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .divider_color = value;
    }
    pub fn action_button_background(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .action_button_background
    }
    pub fn set_action_button_background(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .action_button_background = value;
    }
    pub fn action_button_pressed_background(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .action_button_pressed_background
    }
    pub fn set_action_button_pressed_background(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .action_button_pressed_background = value;
    }
    pub fn panel_background(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .panel_background
    }
    pub fn set_panel_background(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .panel_background = value;
    }
    pub fn panel_foreground(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .panel_foreground
    }
    pub fn set_panel_foreground(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .panel_foreground = value;
    }
    pub fn panel_item_background(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .panel_item_background
    }
    pub fn set_panel_item_background(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .panel_item_background = value;
    }
    pub fn panel_selected_foreground(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .panel_selected_foreground
    }
    pub fn set_panel_selected_foreground(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .panel_selected_foreground = value;
    }
    pub fn one_handed_background(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .one_handed_background
    }
    pub fn set_one_handed_background(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .one_handed_background = value;
    }
    pub fn one_handed_foreground(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .one_handed_foreground
    }
    pub fn set_one_handed_foreground(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .one_handed_foreground = value;
    }
    pub fn glide_trail_color(&self) -> Color {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .glide_trail_color
    }
    pub fn set_glide_trail_color(&self, value: Color) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .glide_trail_color = value;
    }
    pub fn glide_trail_thickness(&self) -> f64 {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .glide_trail_thickness
    }
    pub fn set_glide_trail_thickness(&self, value: f64) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .glide_trail_thickness = value;
    }
    pub fn font_family(&self) -> Option<Utf16Text> {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .font_family
            .clone()
    }
    pub fn set_font_family(&self, value: Option<Utf16Text>) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .font_family = value;
    }
    pub fn key_font_size(&self) -> f64 {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_font_size
    }
    pub fn set_key_font_size(&self, value: f64) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_font_size = value;
    }
    pub fn function_key_font_size(&self) -> f64 {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .function_key_font_size
    }
    pub fn set_function_key_font_size(&self, value: f64) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .function_key_font_size = value;
    }
    pub fn hint_font_size(&self) -> f64 {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .hint_font_size
    }
    pub fn set_hint_font_size(&self, value: f64) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .hint_font_size = value;
    }
    pub fn space_font_size(&self) -> f64 {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .space_font_size
    }
    pub fn set_space_font_size(&self, value: f64) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .space_font_size = value;
    }
    pub fn icon_size(&self) -> f64 {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .icon_size
    }
    pub fn set_icon_size(&self, value: f64) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .icon_size = value;
    }
    pub fn smartbar_font_size(&self) -> f64 {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .smartbar_font_size
    }
    pub fn set_smartbar_font_size(&self, value: f64) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .smartbar_font_size = value;
    }
    pub fn emoji_font_size(&self) -> f64 {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .emoji_font_size
    }
    pub fn set_emoji_font_size(&self, value: f64) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .emoji_font_size = value;
    }
    pub fn uppercase_labels_when_shifted(&self) -> bool {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .uppercase_labels_when_shifted
    }
    pub fn set_uppercase_labels_when_shifted(&self, value: bool) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .uppercase_labels_when_shifted = value;
    }
    pub fn key_rules(&self) -> Option<ThemeList<Option<Arc<KeyStyleRule>>>> {
        self.state
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_rules
            .clone()
    }
    pub fn set_key_rules(&self, value: Option<ThemeList<Option<Arc<KeyStyleRule>>>>) {
        self.state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .key_rules = value;
    }
}
impl PartialEq for KeyboardTheme {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for KeyboardTheme {}
type ColorProperty = (
    &'static str,
    fn(&KeyboardTheme) -> Color,
    fn(&KeyboardTheme, Color),
);
const COLOR_PROPERTIES: &[ColorProperty] = &[
    (
        "background",
        KeyboardTheme::background,
        KeyboardTheme::set_background,
    ),
    (
        "keyBackground",
        KeyboardTheme::key_background,
        KeyboardTheme::set_key_background,
    ),
    (
        "keyPressedBackground",
        KeyboardTheme::key_pressed_background,
        KeyboardTheme::set_key_pressed_background,
    ),
    (
        "keyForeground",
        KeyboardTheme::key_foreground,
        KeyboardTheme::set_key_foreground,
    ),
    (
        "keyHintForeground",
        KeyboardTheme::key_hint_foreground,
        KeyboardTheme::set_key_hint_foreground,
    ),
    (
        "functionKeyBackground",
        KeyboardTheme::function_key_background,
        KeyboardTheme::set_function_key_background,
    ),
    (
        "functionKeyPressedBackground",
        KeyboardTheme::function_key_pressed_background,
        KeyboardTheme::set_function_key_pressed_background,
    ),
    (
        "functionKeyForeground",
        KeyboardTheme::function_key_foreground,
        KeyboardTheme::set_function_key_foreground,
    ),
    (
        "accentKeyBackground",
        KeyboardTheme::accent_key_background,
        KeyboardTheme::set_accent_key_background,
    ),
    (
        "accentKeyPressedBackground",
        KeyboardTheme::accent_key_pressed_background,
        KeyboardTheme::set_accent_key_pressed_background,
    ),
    (
        "accentKeyForeground",
        KeyboardTheme::accent_key_foreground,
        KeyboardTheme::set_accent_key_foreground,
    ),
    (
        "spaceKeyForeground",
        KeyboardTheme::space_key_foreground,
        KeyboardTheme::set_space_key_foreground,
    ),
    (
        "capsLockForeground",
        KeyboardTheme::caps_lock_foreground,
        KeyboardTheme::set_caps_lock_foreground,
    ),
    (
        "keyBorderColor",
        KeyboardTheme::key_border_color,
        KeyboardTheme::set_key_border_color,
    ),
    (
        "keyShadowColor",
        KeyboardTheme::key_shadow_color,
        KeyboardTheme::set_key_shadow_color,
    ),
    (
        "popupBackground",
        KeyboardTheme::popup_background,
        KeyboardTheme::set_popup_background,
    ),
    (
        "popupForeground",
        KeyboardTheme::popup_foreground,
        KeyboardTheme::set_popup_foreground,
    ),
    (
        "popupSelectedBackground",
        KeyboardTheme::popup_selected_background,
        KeyboardTheme::set_popup_selected_background,
    ),
    (
        "popupSelectedForeground",
        KeyboardTheme::popup_selected_foreground,
        KeyboardTheme::set_popup_selected_foreground,
    ),
    (
        "popupBorderColor",
        KeyboardTheme::popup_border_color,
        KeyboardTheme::set_popup_border_color,
    ),
    (
        "smartbarBackground",
        KeyboardTheme::smartbar_background,
        KeyboardTheme::set_smartbar_background,
    ),
    (
        "smartbarForeground",
        KeyboardTheme::smartbar_foreground,
        KeyboardTheme::set_smartbar_foreground,
    ),
    (
        "suggestionHighlightForeground",
        KeyboardTheme::suggestion_highlight_foreground,
        KeyboardTheme::set_suggestion_highlight_foreground,
    ),
    (
        "dividerColor",
        KeyboardTheme::divider_color,
        KeyboardTheme::set_divider_color,
    ),
    (
        "actionButtonBackground",
        KeyboardTheme::action_button_background,
        KeyboardTheme::set_action_button_background,
    ),
    (
        "actionButtonPressedBackground",
        KeyboardTheme::action_button_pressed_background,
        KeyboardTheme::set_action_button_pressed_background,
    ),
    (
        "panelBackground",
        KeyboardTheme::panel_background,
        KeyboardTheme::set_panel_background,
    ),
    (
        "panelForeground",
        KeyboardTheme::panel_foreground,
        KeyboardTheme::set_panel_foreground,
    ),
    (
        "panelItemBackground",
        KeyboardTheme::panel_item_background,
        KeyboardTheme::set_panel_item_background,
    ),
    (
        "panelSelectedForeground",
        KeyboardTheme::panel_selected_foreground,
        KeyboardTheme::set_panel_selected_foreground,
    ),
    (
        "oneHandedBackground",
        KeyboardTheme::one_handed_background,
        KeyboardTheme::set_one_handed_background,
    ),
    (
        "oneHandedForeground",
        KeyboardTheme::one_handed_foreground,
        KeyboardTheme::set_one_handed_foreground,
    ),
    (
        "glideTrailColor",
        KeyboardTheme::glide_trail_color,
        KeyboardTheme::set_glide_trail_color,
    ),
];
type NumberProperty = (
    &'static str,
    fn(&KeyboardTheme) -> f64,
    fn(&KeyboardTheme, f64),
);
const NUMBER_PROPERTIES: &[NumberProperty] = &[
    (
        "keyBorderThickness",
        KeyboardTheme::key_border_thickness,
        KeyboardTheme::set_key_border_thickness,
    ),
    (
        "keyShadowDepth",
        KeyboardTheme::key_shadow_depth,
        KeyboardTheme::set_key_shadow_depth,
    ),
    (
        "keyCornerRadius",
        KeyboardTheme::key_corner_radius,
        KeyboardTheme::set_key_corner_radius,
    ),
    (
        "popupCornerRadius",
        KeyboardTheme::popup_corner_radius,
        KeyboardTheme::set_popup_corner_radius,
    ),
    (
        "glideTrailThickness",
        KeyboardTheme::glide_trail_thickness,
        KeyboardTheme::set_glide_trail_thickness,
    ),
    (
        "keyFontSize",
        KeyboardTheme::key_font_size,
        KeyboardTheme::set_key_font_size,
    ),
    (
        "functionKeyFontSize",
        KeyboardTheme::function_key_font_size,
        KeyboardTheme::set_function_key_font_size,
    ),
    (
        "hintFontSize",
        KeyboardTheme::hint_font_size,
        KeyboardTheme::set_hint_font_size,
    ),
    (
        "spaceFontSize",
        KeyboardTheme::space_font_size,
        KeyboardTheme::set_space_font_size,
    ),
    (
        "iconSize",
        KeyboardTheme::icon_size,
        KeyboardTheme::set_icon_size,
    ),
    (
        "smartbarFontSize",
        KeyboardTheme::smartbar_font_size,
        KeyboardTheme::set_smartbar_font_size,
    ),
    (
        "emojiFontSize",
        KeyboardTheme::emoji_font_size,
        KeyboardTheme::set_emoji_font_size,
    ),
];
