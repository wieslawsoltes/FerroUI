//! `Microsoft.UI.Xaml.Media.Shadow` and `ThemeShadow` of WinUI (phase 7 of
//! docs/porting/winui-composition.md, not from upstream), ported from
//! microsoft-ui-xaml: `dxaml/xcp/core/inc/ThemeShadow.h`,
//! `dxaml/xcp/components/graphics/ThemeShadow.cpp`,
//! `dxaml/xcp/components/Collection/UIElementWeakCollection.cpp` and
//! `dxaml/xcp/components/graphics/inc/DropShadowRecipe.h`.
//!
//! An element casts the shadow of the `Shadow` property of
//! [`Visual`](crate::Visual); how it is drawn is in
//! `rendering::composition::theme_shadow_visual`.

use crate::media::{BoxShadow, Color};
use crate::rendering::composition::CompositionBrush;
use crate::styling::ThemeVariant;
use crate::{ferro_class, ferro_impl_classes, instantiate, FerroObject, FerroObjectImpl, Ref, Thickness, Visual, WeakRef};
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

/// The base class of the shadows an element can cast (WinUI
/// `Microsoft.UI.Xaml.Media.Shadow`). [`ThemeShadow`] is the one shadow.
#[repr(C)]
pub struct Shadow {
    base: FerroObject,
}

ferro_class!(Shadow: FerroObject);
ferro_impl_classes!(Shadow: FerroObjectImpl);

impl Shadow {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    /// The shadow as a theme shadow, if it is one.
    pub fn as_theme_shadow(&self) -> Option<&ThemeShadow> {
        self.downcast_ref::<ThemeShadow>()
    }
}

/// The shadow of the theme (WinUI `Microsoft.UI.Xaml.Media.ThemeShadow`): set
/// as the `Shadow` of an element, the element casts the shadow of its
/// elevation, the Z of its `Translation`, as the controls of WinUI do under
/// their popups (32 for a flyout, 16 for a tool tip, 128 for a content
/// dialog).
///
/// WinUI 3 draws every theme shadow as a drop shadow
/// ([`is_drop_shadow_mode`](Self::is_drop_shadow_mode)): an ambient and a
/// directional shadow around the element, the darker in the dark theme,
/// none in a high contrast theme. A `ThemeShadow` can be shared by several
/// elements; it has no appearance of its own.
///
/// [`receivers`](Self::receivers) are the elements the projected shadows of
/// WinUI (the root and the open popups by default) would fall on. WinUI 3
/// keeps the collection and its validation but does not draw projected
/// shadows, so the receivers do not change the drawing here either.
#[repr(C)]
pub struct ThemeShadow {
    base: Shadow,
    receivers: UIElementWeakCollection,
    /// The mask of the shadow (WinUI's internal `ThemeShadow.Mask`, used by
    /// the projected shadows).
    mask: RefCell<Option<Rc<CompositionBrush>>>,
}

ferro_class!(ThemeShadow: Shadow);
crate::ferro_class_info!(ThemeShadow { new: ThemeShadow::new });
ferro_impl_classes!(ThemeShadow: FerroObjectImpl);

impl ThemeShadow {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self {
            base: Shadow::construct(),
            receivers: UIElementWeakCollection::new(true),
            mask: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// `ThemeShadow.IsDropShadowMode`: whether theme shadows are drawn as drop
    /// shadows around their casters (`true` in WinUI 3, whose projected
    /// shadows are disabled: `CThemeShadow::IsDropShadowMode`).
    pub fn is_drop_shadow_mode() -> bool {
        true
    }

    /// `ThemeShadow.Receivers`: the elements the shadow is cast on in the
    /// projected mode. An ancestor of an element that casts the shadow
    /// cannot be a receiver.
    pub fn receivers(&self) -> &UIElementWeakCollection {
        &self.receivers
    }

    /// Whether the receivers were set (WinUI's `HasCustomReceivers`): the
    /// projected mode casts the shadow on the root and the popups otherwise.
    pub fn has_custom_receivers(&self) -> bool {
        self.receivers.size() > 0
    }

    /// The mask of the shadow (internal in WinUI, set by the connected
    /// animations for the projected shadows).
    pub fn mask(&self) -> Option<Rc<CompositionBrush>> {
        self.mask.borrow().clone()
    }

    pub fn set_mask(&self, value: Option<Rc<CompositionBrush>>) {
        *self.mask.borrow_mut() = value;
    }

    /// Called by [`Visual`] when the shadow becomes or stops being its
    /// `Shadow`: the elements that use the shadow, for the validation of
    /// the receivers.
    pub(crate) fn add_owner(&self, owner: &Visual) {
        self.receivers.owners.borrow_mut().push(owner.to_ref().downgrade());
    }

    pub(crate) fn remove_owner(&self, owner: &Visual) {
        self.receivers.owners.borrow_mut().retain(|o| o.upgrade().is_some_and(|o| !std::ptr::eq(&*o, owner)));
    }
}

/// The error of a change of the receivers of a [`ThemeShadow`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeShadowError {
    /// `ERROR_ANCESTOR_ELEMENT_CANNOT_BE_SHADOW_RECEIVER`: the element is an
    /// ancestor of an element that casts the shadow.
    AncestorElementCannotBeShadowReceiver,
    /// An index past the end of the collection.
    IndexOutOfRange,
}

impl fmt::Display for ThemeShadowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ThemeShadowError::AncestorElementCannotBeShadowReceiver => {
                f.write_str("an ancestor of an element that casts the shadow cannot receive it")
            }
            ThemeShadowError::IndexOutOfRange => f.write_str("the index is out of range"),
        }
    }
}

impl std::error::Error for ThemeShadowError {}

/// A collection of elements held weakly (WinUI
/// `Microsoft.UI.Xaml.UIElementWeakCollection`, `IVector<UIElement>`): an
/// element that is gone reads as `None` and stays in the collection until it
/// is removed.
pub struct UIElementWeakCollection {
    items: RefCell<Vec<WeakRef<Visual>>>,
    /// Whether ancestors of the owners are refused (the receivers of a theme
    /// shadow).
    disallow_ancestors: bool,
    /// The elements that use the owner of the collection.
    owners: RefCell<Vec<WeakRef<Visual>>>,
}

impl UIElementWeakCollection {
    fn new(disallow_ancestors: bool) -> Self {
        Self { items: RefCell::new(Vec::new()), disallow_ancestors, owners: RefCell::new(Vec::new()) }
    }

    /// `Size`.
    pub fn size(&self) -> usize {
        self.items.borrow().len()
    }

    /// `GetAt`: the element at `index`, `None` when it is gone.
    pub fn get_at(&self, index: usize) -> Option<Ref<Visual>> {
        self.items.borrow().get(index).and_then(WeakRef::upgrade)
    }

    /// `IndexOf`.
    pub fn index_of(&self, element: &Visual) -> Option<usize> {
        self.items.borrow().iter().position(|item| item.upgrade().is_some_and(|item| std::ptr::eq(&*item, element)))
    }

    /// Whether `element` is in the collection.
    pub fn contains(&self, element: &Visual) -> bool {
        self.index_of(element).is_some()
    }

    /// `Append`.
    pub fn append(&self, element: &Visual) -> Result<(), ThemeShadowError> {
        self.validate(element)?;
        self.items.borrow_mut().push(element.to_ref().downgrade());
        Ok(())
    }

    /// `InsertAt`.
    pub fn insert_at(&self, index: usize, element: &Visual) -> Result<(), ThemeShadowError> {
        if index > self.size() {
            return Err(ThemeShadowError::IndexOutOfRange);
        }
        self.validate(element)?;
        self.items.borrow_mut().insert(index, element.to_ref().downgrade());
        Ok(())
    }

    /// `SetAt`.
    pub fn set_at(&self, index: usize, element: &Visual) -> Result<(), ThemeShadowError> {
        if index >= self.size() {
            return Err(ThemeShadowError::IndexOutOfRange);
        }
        self.validate(element)?;
        self.items.borrow_mut()[index] = element.to_ref().downgrade();
        Ok(())
    }

    /// `RemoveAt`.
    pub fn remove_at(&self, index: usize) -> Result<(), ThemeShadowError> {
        if index >= self.size() {
            return Err(ThemeShadowError::IndexOutOfRange);
        }
        self.items.borrow_mut().remove(index);
        Ok(())
    }

    /// Removes `element`; `false` when it is not in the collection.
    pub fn remove(&self, element: &Visual) -> bool {
        match self.index_of(element) {
            Some(index) => {
                self.items.borrow_mut().remove(index);
                true
            }
            None => false,
        }
    }

    /// `RemoveAtEnd`.
    pub fn remove_at_end(&self) -> Result<(), ThemeShadowError> {
        match self.items.borrow_mut().pop() {
            Some(_) => Ok(()),
            None => Err(ThemeShadowError::IndexOutOfRange),
        }
    }

    /// `Clear`.
    pub fn clear(&self) {
        self.items.borrow_mut().clear();
    }

    /// `GetMany` from the start: the elements that are still alive.
    pub fn items(&self) -> Vec<Ref<Visual>> {
        self.items.borrow().iter().filter_map(WeakRef::upgrade).collect()
    }

    /// `ReplaceAll`.
    pub fn replace_all(&self, elements: &[&Visual]) -> Result<(), ThemeShadowError> {
        for element in elements {
            self.validate(element)?;
        }
        *self.items.borrow_mut() = elements.iter().map(|element| element.to_ref().downgrade()).collect();
        Ok(())
    }

    /// `ValidateItem`: an element that is an ancestor of an element that
    /// uses the owner of the collection is refused (`CheckForAncestor`,
    /// which follows the parent of an element and the popup of a popup's
    /// child).
    fn validate(&self, element: &Visual) -> Result<(), ThemeShadowError> {
        if !self.disallow_ancestors {
            return Ok(());
        }
        for owner in self.owners.borrow().iter().filter_map(WeakRef::upgrade) {
            let mut current = Some(owner);
            while let Some(ancestor) = current {
                if std::ptr::eq(&*ancestor, element) {
                    return Err(ThemeShadowError::AncestorElementCannotBeShadowReceiver);
                }
                current = parent_following_popups(&ancestor);
            }
        }
        Ok(())
    }
}

/// The parent of an element, or the logical parent of the root of a popup
/// (WinUI's `GetParentFollowPopups`).
fn parent_following_popups(element: &Visual) -> Option<Ref<Visual>> {
    if let Some(parent) = element.visual_parent() {
        return Some(parent);
    }
    element.parent().and_then(|parent| parent.downcast::<Visual>().ok())
}

/// The theme a theme shadow is drawn for (WinUI's `ElementTheme` as
/// `GetDropShadowRecipe` takes it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeShadowTheme {
    Light,
    Dark,
    /// A high contrast theme: no shadow (`NeedsDropShadowVisual`).
    HighContrast,
}

impl ThemeShadowTheme {
    /// The theme of the theme variant in effect for an element: a variant
    /// that is or inherits `HighContrast` (the high contrast variant of the
    /// WinUI theme) is high contrast, one that is or inherits `Dark` is
    /// dark, anything else light.
    pub fn of_variant(variant: Option<&ThemeVariant>) -> ThemeShadowTheme {
        let mut current = variant.cloned();
        let mut dark = false;
        while let Some(variant) = current {
            if variant.key().as_str() == Some("HighContrast") {
                return ThemeShadowTheme::HighContrast;
            }
            if variant == ThemeVariant::dark() {
                dark = true;
            }
            current = variant.inherit_variant();
        }
        if dark {
            ThemeShadowTheme::Dark
        } else {
            ThemeShadowTheme::Light
        }
    }

    /// The theme of `element`.
    pub fn of(element: &Visual) -> ThemeShadowTheme {
        Self::of_variant(element.actual_theme_variant().as_ref())
    }
}

/// The values of the drop shadow of a theme shadow (WinUI's
/// `DropShadowRecipe`, `GetDropShadowRecipe`): an ambient and a directional
/// shadow of the elevation of an element.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropShadowRecipe {
    /// The corner radii of the rounded rectangle that casts the shadow.
    pub radius_x: f64,
    pub radius_y: f64,
    /// The elevation: half the Z of the translation, at most 64.
    pub elevation: f32,
    pub ambient_blur_radius: f32,
    pub directional_blur_radius: f32,
    /// How far each shadow is moved down.
    pub ambient_y_offset: f32,
    pub directional_y_offset: f32,
    pub ambient_opacity: f32,
    pub directional_opacity: f32,
    /// Black with the opacity in the alpha (truncated, as WinUI converts it).
    pub ambient_color: Color,
    pub directional_color: Color,
    /// How far the shadow reaches around the element.
    pub insets: Thickness,
}

impl DropShadowRecipe {
    /// `GetDropShadowRecipe(translationZ, theme)`: no shadow below an
    /// elevation of 2; up to 16 a directional shadow only, of the opacity
    /// `min(elevation / 100 + 0.06, 0.14)` in the light theme and 0.26 in the
    /// dark one; above 16 also an ambient shadow, blurred by a third of the
    /// elevation and moved down by 2, of the opacity 0.15 (light) or 0.37
    /// (dark), the directional one 0.19 (light) or 0.37 (dark). The
    /// directional shadow is blurred by the elevation and moved down by half
    /// of it. The corner radii are zero; see
    /// [`with_corner_radius`](Self::with_corner_radius).
    ///
    /// A high contrast theme has no shadow: both opacities are zero.
    pub fn new(translation_z: f32, theme: ThemeShadowTheme) -> DropShadowRecipe {
        let elevation = (translation_z / 2.0).min(64.0);
        let mut ambient_blur_radius = 2.0f32;
        let mut ambient_y_offset = 0.0f32;
        let mut ambient_opacity = 0.0f32;
        let mut directional_opacity = 0.0f32;

        if elevation < 2.0 {
            // No shadow.
        } else if elevation <= 16.0 {
            // The ambient shadow does not show.
            directional_opacity = match theme {
                ThemeShadowTheme::Light => (elevation / 100.0 + 0.06).min(0.14),
                ThemeShadowTheme::Dark => 0.26,
                ThemeShadowTheme::HighContrast => 0.0,
            };
        } else {
            ambient_blur_radius = elevation / 3.0;
            ambient_y_offset = 2.0;
            (ambient_opacity, directional_opacity) = match theme {
                ThemeShadowTheme::Light => (0.15, 0.19),
                ThemeShadowTheme::Dark => (0.37, 0.37),
                ThemeShadowTheme::HighContrast => (0.0, 0.0),
            };
        }

        let black = |opacity: f32| Color::from_argb((opacity * 255.0) as u8, 0, 0, 0);
        let directional_blur_radius = elevation;
        let directional_y_offset = elevation * 0.5;
        let max_blur_radius = ambient_blur_radius.max(directional_blur_radius).ceil() as f64;
        let top = (ambient_blur_radius - ambient_y_offset).max(directional_blur_radius - directional_y_offset).ceil();
        let bottom = (ambient_blur_radius + ambient_y_offset).max(directional_blur_radius + directional_y_offset).ceil();

        DropShadowRecipe {
            radius_x: 0.0,
            radius_y: 0.0,
            elevation,
            ambient_blur_radius,
            directional_blur_radius,
            ambient_y_offset,
            directional_y_offset,
            ambient_opacity,
            directional_opacity,
            ambient_color: black(ambient_opacity),
            directional_color: black(directional_opacity),
            insets: Thickness::new(max_blur_radius, top as f64, max_blur_radius, bottom as f64),
        }
    }

    /// The recipe with the corner radii of the rounded rectangle that casts
    /// the shadow.
    pub fn with_corner_radius(mut self, radius_x: f64, radius_y: f64) -> DropShadowRecipe {
        self.radius_x = radius_x.max(0.0);
        self.radius_y = radius_y.max(0.0);
        self
    }

    /// Whether the recipe draws anything.
    pub fn is_visible(&self) -> bool {
        self.ambient_color.a > 0 || self.directional_color.a > 0
    }

    /// The two shadows as the box shadows of the rounded rectangle the shadow
    /// is drawn around, the ambient first (`CreateDropShadowVisual` puts it
    /// at the bottom). Each is blurred by one more than its radius, because
    /// the rectangle is one pixel smaller on each side than the element
    /// (`CreateDropShadowVisual` shrinks the dummy shape so that the
    /// antialiased edge of the element lies over the shadow).
    pub fn box_shadows(&self) -> Vec<BoxShadow> {
        let mut shadows = Vec::with_capacity(2);
        if self.ambient_color.a > 0 {
            shadows.push(BoxShadow {
                offset_x: 0.0,
                offset_y: self.ambient_y_offset as f64,
                blur: (self.ambient_blur_radius + 1.0) as f64,
                spread: 0.0,
                color: self.ambient_color,
                is_inset: false,
            });
        }
        if self.directional_color.a > 0 {
            shadows.push(BoxShadow {
                offset_x: 0.0,
                offset_y: self.directional_y_offset as f64,
                blur: (self.directional_blur_radius + 1.0) as f64,
                spread: 0.0,
                color: self.directional_color,
                is_inset: false,
            });
        }
        shadows
    }
}
