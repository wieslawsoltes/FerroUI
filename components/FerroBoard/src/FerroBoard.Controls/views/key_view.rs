//! Derived from ScreenKeyboard.Uno/Views/KeyView.cs (Apache-2.0).
use super::{KeysView, key_automation_peer::KeyAutomationPeer};
use crate::{
    KeyboardStrings,
    icons::{IconError, IconFactory},
    primitives::Brushes,
    themes::{KeyboardTheme, ThemeError},
};
use ferroboard_core::{
    keys::{KeyCode, KeyIcon, KeyType, KeyboardMode, ShiftState},
    layouts::ComputedKey,
    text::{TextUtils, Utf16Text},
};
use ferroui_base::{
    CornerRadius, FerroObjectImpl, FerroObjectImplExt, Ref, StyledElementImpl, Thickness,
    VisualImpl, WeakRef, ferro_class, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment},
    media::{Color, FontFamily, TextAlignment, TextTrimming, TextWrapping},
};
use ferroui_controls::{
    Border, Canvas, Control, ControlImpl, Grid, PanelImpl, TextBlock,
    automation::AutomationProperties, shapes::Path,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Debug)]
pub(crate) enum KeyViewError {
    Theme(ThemeError),
    Icon(IconError),
    UnpairedSurrogate(&'static str),
}
impl From<ThemeError> for KeyViewError {
    fn from(e: ThemeError) -> Self {
        Self::Theme(e)
    }
}
impl From<IconError> for KeyViewError {
    fn from(e: IconError) -> Self {
        Self::Icon(e)
    }
}
impl std::fmt::Display for KeyViewError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Theme(e) => write!(f, "{e}"),
            Self::Icon(e) => write!(f, "{e}"),
            Self::UnpairedSurrogate(p) => write!(f, "native {p} cannot store unpaired UTF-16"),
        }
    }
}
impl std::error::Error for KeyViewError {}
#[derive(Clone, Copy, Debug)]
pub(crate) struct KeyStyle {
    pub background: Color,
    pub foreground: Color,
    pub font_size: f64,
    pub corner_radius: f64,
}
pub(crate) struct KeyStyles;
impl KeyStyles {
    pub fn is_function_key(key: &ComputedKey) -> bool {
        let data = key.data();
        let data = data.data();
        (!matches!(data.key_type(), KeyType::Character | KeyType::Numeric) && !data.is_space())
            || data.code() == KeyCode::Enter
    }
    pub fn resolve(
        theme: &KeyboardTheme,
        key: &ComputedKey,
        mode: KeyboardMode,
        shift: ShiftState,
        pressed: bool,
        font_scale: f64,
    ) -> Result<KeyStyle, ThemeError> {
        let function = Self::is_function_key(key);
        let mut background = if function {
            if pressed {
                theme.function_key_pressed_background()
            } else {
                theme.function_key_background()
            }
        } else if pressed {
            theme.key_pressed_background()
        } else {
            theme.key_background()
        };
        let mut foreground = if function {
            theme.function_key_foreground()
        } else {
            theme.key_foreground()
        };
        let label = key.label().unwrap_or_default();
        let space = key.data().data().is_space();
        let mut font_size = if space {
            theme.space_font_size()
        } else if label.len() > 1 && TextUtils::get_code_points(&label).len() > 1 {
            theme.function_key_font_size()
        } else {
            theme.key_font_size()
        };
        if space {
            foreground = theme.space_key_foreground();
        }
        if key.code() == KeyCode::Enter {
            background = if pressed {
                theme.accent_key_pressed_background()
            } else {
                theme.accent_key_background()
            };
            foreground = theme.accent_key_foreground();
        }
        if key.code() == KeyCode::Shift && shift == ShiftState::CapsLock {
            foreground = theme.caps_lock_foreground();
        }
        let mut corner = theme.key_corner_radius();
        let rules = theme.key_rules().ok_or(ThemeError::NullReference)?;
        let rules = rules
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        for rule in rules {
            let rule = rule.ok_or(ThemeError::NullReference)?;
            if !rule.matches(key.code(), key.key_type(), mode, shift)? {
                continue;
            }
            if pressed && let Some(pb) = rule.pressed_background() {
                background = pb;
            } else if let Some(bg) = rule.background()
                && (!pressed || rule.pressed_background().is_none())
            {
                background = if pressed { Self::darken(bg, 0.85) } else { bg };
            }
            if let Some(v) = rule.foreground() {
                foreground = v;
            }
            if let Some(v) = rule.font_size() {
                font_size = v;
            }
            if let Some(v) = rule.corner_radius() {
                corner = v;
            }
        }
        Ok(KeyStyle {
            background,
            foreground,
            font_size: font_size * font_scale,
            corner_radius: corner,
        })
    }
    pub fn darken(c: Color, factor: f64) -> Color {
        Color {
            a: c.a,
            r: byte(c.r as f64 * factor),
            g: byte(c.g as f64 * factor),
            b: byte(c.b as f64 * factor),
        }
    }
}
fn byte(value: f64) -> u8 {
    (value as i32) as u8
}
fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() || a > b || (a == b && !a.is_sign_negative()) {
        a
    } else {
        b
    }
}
fn min(a: f64, b: f64) -> f64 {
    if a.is_nan() || a < b || (a == b && a.is_sign_negative()) {
        a
    } else {
        b
    }
}
fn native_text(text: &Utf16Text, field: &'static str) -> Result<String, KeyViewError> {
    text.to_utf8()
        .map_err(|_| KeyViewError::UnpairedSurrogate(field))
}

#[repr(C)]
pub(crate) struct KeyView {
    base: Grid,
    shadow: Ref<Border>,
    face: Ref<Border>,
    face_content: Ref<Grid>,
    label: Ref<TextBlock>,
    hint: Ref<TextBlock>,
    sub_label: Ref<TextBlock>,
    icon_kind: Cell<KeyIcon>,
    icon_size: Cell<f64>,
    icon: RefCell<Option<Ref<Path>>>,
    key: Rc<ComputedKey>,
    is_pressed: Cell<bool>,
    activation_owner: RefCell<Option<WeakRef<KeysView>>>,
}
ferro_class!(KeyView:Grid);
ferro_impl_classes!(KeyView:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,PanelImpl);
impl ControlImpl for KeyView {
    fn on_create_automation_peer(
        this: &Self,
    ) -> Ref<ferroui_controls::automation::peers::AutomationPeer> {
        KeyAutomationPeer::new(this).upcast()
    }
}
impl FerroObjectImpl for KeyView {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        this.set_is_hit_test_visible(false);
        this.set_is_tab_stop(false);
        for border in [&this.shadow, &this.face] {
            border.set_is_hit_test_visible(false);
        }
        this.face_content.set_is_hit_test_visible(false);
        this.label
            .set_horizontal_alignment(HorizontalAlignment::Center);
        this.label.set_vertical_alignment(VerticalAlignment::Center);
        this.label.set_text_alignment(TextAlignment::Center);
        this.label
            .set_text_trimming(<dyn TextTrimming>::character_ellipsis());
        this.label.set_text_wrapping(TextWrapping::NoWrap);
        this.label.set_is_hit_test_visible(false);
        this.hint
            .set_horizontal_alignment(HorizontalAlignment::Right);
        this.hint.set_vertical_alignment(VerticalAlignment::Top);
        this.hint.set_margin(Thickness::new(0.0, 2.0, 4.0, 0.0));
        this.hint.set_is_hit_test_visible(false);
        this.sub_label
            .set_horizontal_alignment(HorizontalAlignment::Center);
        this.sub_label
            .set_vertical_alignment(VerticalAlignment::Bottom);
        this.sub_label.set_is_hit_test_visible(false);
        this.face_content.children().add(this.label.clone());
        this.face_content.children().add(this.hint.clone());
        this.face_content.children().add(this.sub_label.clone());
        this.face.set_child(this.face_content.clone());
        this.children().add(this.shadow.clone());
        this.children().add(this.face.clone());
    }
}
impl KeyView {
    pub fn construct(key: Rc<ComputedKey>) -> Self {
        Self {
            base: Grid::construct(),
            shadow: Border::new(),
            face: Border::new(),
            face_content: Grid::new(),
            label: TextBlock::new(),
            hint: TextBlock::new(),
            sub_label: TextBlock::new(),
            icon_kind: Cell::new(KeyIcon::None),
            icon_size: Cell::new(0.0),
            icon: RefCell::new(None),
            key,
            is_pressed: Cell::new(false),
            activation_owner: RefCell::new(None),
        }
    }
    pub fn new(key: Rc<ComputedKey>) -> Ref<Self> {
        instantiate(Self::construct(key))
    }
    pub fn key(&self) -> Rc<ComputedKey> {
        self.key.clone()
    }
    pub(super) fn set_activation_owner(&self, owner: WeakRef<KeysView>) {
        *self.activation_owner.borrow_mut() = Some(owner);
    }
    pub(super) fn invoke(
        &self,
    ) -> Result<(), ferroui_controls::automation::ElementNotEnabledError> {
        let parent = self
            .activation_owner
            .borrow()
            .as_ref()
            .and_then(WeakRef::upgrade);
        let parent = parent.ok_or_else(|| {
            ferroui_controls::automation::ElementNotEnabledError::with_message(
                "Keyboard view no longer exists.",
            )
        })?;
        parent.invoke_key(self)
    }
    pub fn is_pressed(&self) -> bool {
        self.is_pressed.get()
    }
    pub fn set_is_pressed(&self, value: bool) {
        self.is_pressed.set(value);
    }
    pub fn update(
        &self,
        theme: &KeyboardTheme,
        mode: KeyboardMode,
        shift: ShiftState,
        font_scale: f64,
        font_family: Option<FontFamily>,
    ) -> Result<(), KeyViewError> {
        let key = self.key();
        let bounds = key.visible_bounds();
        self.set_width(max(0.0, bounds.width));
        self.set_height(max(0.0, bounds.height));
        Canvas::set_left(self, bounds.x);
        Canvas::set_top(self, bounds.y);
        self.set_is_visible(key.is_visible());
        let style = KeyStyles::resolve(theme, &key, mode, shift, self.is_pressed(), font_scale)?;
        let corner = CornerRadius::uniform(min(
            style.corner_radius,
            min(bounds.width, bounds.height) / 2.0,
        ));
        self.face
            .set_background(Some(Brushes::get(style.background).into()));
        self.face.set_corner_radius(corner);
        self.face
            .set_border_brush(Some(Brushes::get(theme.key_border_color()).into()));
        self.face
            .set_border_thickness(Thickness::uniform(theme.key_border_thickness()));
        let shadow_visible = theme.key_shadow_depth() > 0.0
            && theme.key_shadow_color().a > 0
            && style.background.a > 0;
        self.shadow.set_is_visible(shadow_visible);
        if shadow_visible {
            self.shadow
                .set_background(Some(Brushes::get(theme.key_shadow_color()).into()));
            self.shadow.set_corner_radius(corner);
            self.shadow.set_margin(Thickness::new(
                0.0,
                theme.key_shadow_depth(),
                0.0,
                -theme.key_shadow_depth(),
            ));
        }
        let foreground = Brushes::get(style.foreground);
        let max_font = max(8.0, bounds.height * 0.52);
        if key.icon() != KeyIcon::None && IconFactory::has_icon(key.icon()) {
            self.label.set_is_visible(false);
            let size = min(
                theme.icon_size() * font_scale,
                max(10.0, min(bounds.height, bounds.width) * 0.6),
            );
            if self.icon.borrow().is_none()
                || self.icon_kind.get() != key.icon()
                || (self.icon_size.get() - size).abs() > 0.1
            {
                let old = self.icon.borrow().clone();
                if let Some(old) = old {
                    self.face_content.children().remove(old.upcast::<Control>());
                }
                // Source stores the new icon only AFTER Create succeeds; a failure leaves the old
                // cached handle (now detached) and collapsed label, which a later update can retry.
                let icon = IconFactory::create(key.icon(), size, Some(foreground.clone().into()))?;
                *self.icon.borrow_mut() = Some(icon.clone());
                icon.set_horizontal_alignment(HorizontalAlignment::Center);
                icon.set_vertical_alignment(VerticalAlignment::Center);
                self.face_content
                    .children()
                    .insert(0, icon.upcast::<Control>());
                self.icon_kind.set(key.icon());
                self.icon_size.set(size);
            }
            let icon = self.icon.borrow().clone().expect("created/cached icon");
            icon.set_fill(Some(foreground.into()));
            icon.set_is_visible(true);
        } else {
            if let Some(icon) = self.icon.borrow().as_ref() {
                icon.set_is_visible(false);
            }
            self.label.set_is_visible(true);
            let label = key.label().unwrap_or_default();
            self.label.set_text(Some(&native_text(&label, "label")?));
            self.label.set_font_size(min(
                style.font_size,
                if key.data().data().is_space() {
                    max_font * 0.7
                } else {
                    max_font
                },
            ));
            self.label.set_foreground(Some(foreground.into()));
            self.label.set_max_width(max(0.0, bounds.width - 4.0));
            if let Some(font_family) = &font_family {
                self.label.set_font_family(font_family.clone());
            }
        }
        if let Some(hint) = key.hint_label().filter(|s| !s.is_empty())
            && bounds.height > 30.0
        {
            self.hint.set_is_visible(true);
            self.hint.set_text(Some(&native_text(&hint, "hint")?));
            self.hint.set_font_size(min(
                theme.hint_font_size() * font_scale,
                bounds.height * 0.3,
            ));
            self.hint
                .set_foreground(Some(Brushes::get(theme.key_hint_foreground()).into()));
            if let Some(font_family) = &font_family {
                self.hint.set_font_family(font_family.clone());
            }
        } else {
            self.hint.set_is_visible(false);
        }
        if let Some(label) = key.sub_label().filter(|s| !s.is_empty())
            && bounds.height > 30.0
        {
            self.sub_label.set_is_visible(true);
            self.sub_label
                .set_text(Some(&native_text(&label, "sub-label")?));
            self.sub_label.set_font_size(min(
                theme.hint_font_size() * font_scale,
                bounds.height * 0.22,
            ));
            self.sub_label
                .set_foreground(Some(Brushes::get(theme.key_hint_foreground()).into()));
            self.sub_label.set_margin(Thickness::new(
                0.0,
                0.0,
                0.0,
                max(1.0, bounds.height * 0.06),
            ));
            self.label
                .set_margin(Thickness::new(0.0, 0.0, 0.0, bounds.height * 0.18));
        } else {
            self.sub_label.set_is_visible(false);
            self.label.set_margin(Thickness::default());
        }
        let name = Self::get_automation_name(&key);
        let name = name
            .as_ref()
            .map(|s| native_text(s, "automation name"))
            .transpose()?;
        AutomationProperties::set_name(self, name.as_deref());
        Ok(())
    }
    pub fn get_automation_name(key: &ComputedKey) -> Option<Utf16Text> {
        match key.code() {
            KeyCode::Shift => KeyboardStrings::shift(),
            KeyCode::Delete => KeyboardStrings::backspace(),
            KeyCode::Enter => KeyboardStrings::enter(),
            KeyCode::Space => KeyboardStrings::space(),
            KeyCode::LanguageSwitch => KeyboardStrings::switch_language(),
            KeyCode::ImeUiModeMedia => KeyboardStrings::emoji(),
            _ => Some(
                key.label()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| IconFactory::name_value((key.icon()).0).as_str().into()),
            ),
        }
    }
}
/// Source FontWeightsEx.SemiBold value used by other keyboard UI components.
pub(crate) struct FontWeightsEx;
impl FontWeightsEx {
    pub fn semi_bold() -> ferroui_base::media::FontWeight {
        ferroui_base::media::FontWeight::SemiBold
    }
}
#[cfg(test)]
#[path = "key_view_tests.rs"]
mod tests;
