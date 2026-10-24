//! The `Shadow` of an element (WinUI `UIElement.Shadow`, phase 7 of
//! docs/porting/winui-composition.md): the element casts the theme shadow of
//! its elevation, drawn by a [`ThemeShadowVisual`] that the parent of the
//! element shows just before the element's own composition visual.
//!
//! Ported from microsoft-ui-xaml (`HWCompNodeWinRT.cpp`: `NeedsDropShadowVisual`,
//! `UpdateDropShadowVisual`, `UpdateDropShadowRecipeCornerRadius`,
//! `GetThemeShadowFromDropShadowCaster`; `uielement.cpp`: the `Shadow` of a
//! popup delegated to its child).

use super::{ThemeShadowState, Visual};
use crate::media::{DropShadowRecipe, ThemeShadow, ThemeShadowTheme};
use crate::rendering::composition::{CompositionVisual, ThemeShadowVisual};
use crate::{CornerRadius, FerroPropertyRegistry, Ref, Vector};
use std::rc::Rc;

impl Visual {
    /// `UIElement.Shadow`: the shadow the element casts, a
    /// [`ThemeShadow`](crate::media::ThemeShadow) (the one shadow there is).
    /// The shadow of a popup is cast by the child of the popup.
    pub fn shadow(&self) -> Option<Ref<crate::media::Shadow>> {
        self.get_value(Self::shadow_property())
    }

    pub fn set_shadow(&self, value: Option<Ref<crate::media::Shadow>>) {
        self.set_value(Self::shadow_property(), value)
    }

    /// The theme shadow the visual casts and the element whose elevation (the
    /// Z of its `Translation`) the shadow has: its own `ThemeShadow`, or, for
    /// the child of a popup with a theme shadow, the popup's (with the
    /// popup's elevation, which wins over the child's own shadow). `None` for
    /// a visual that delegates its shadow to its child (a popup) and for one
    /// without a theme shadow.
    pub fn theme_shadow_caster(&self) -> Option<(Ref<ThemeShadow>, Ref<Visual>)> {
        let (popup, delegates) = self
            .read_composition_extras(|extras| {
                extras.theme_shadow.as_ref().map(|state| (state.popup.clone(), state.delegates_to_child))
            })
            .flatten()?;
        if delegates {
            return None;
        }
        if let Some(popup) = popup.and_then(|popup| popup.upgrade()) {
            if let Some(shadow) = popup.shadow().and_then(|shadow| shadow.downcast::<ThemeShadow>().ok()) {
                return Some((shadow, popup));
            }
        }
        let shadow = self.shadow()?.downcast::<ThemeShadow>().ok()?;
        Some((shadow, self.to_ref()))
    }

    /// Whether the visual casts a theme shadow, or may (a cheap test of the
    /// composition state, without the property values).
    pub(super) fn may_cast_theme_shadow(&self) -> bool {
        self.read_composition_extras(|extras| extras.theme_shadow.is_some()).unwrap_or(false)
    }

    /// The visual that draws the theme shadow of this visual, while there is
    /// one.
    pub fn theme_shadow_visual(&self) -> Option<Rc<ThemeShadowVisual>> {
        self.read_composition_extras(|extras| extras.theme_shadow.as_ref().and_then(|state| state.visual.clone()))
            .flatten()
    }

    /// For a popup (WinUI's `Popup`, whose `Shadow` is cast by its child):
    /// whether the shadow of this visual is cast by its child rather than by
    /// itself.
    pub fn set_theme_shadow_delegated_to_child(&self, value: bool) {
        self.with_composition_extras(|extras| {
            extras.theme_shadow.get_or_insert_with(ThemeShadowState::default).delegates_to_child = value;
        });
        self.theme_shadow_changed();
    }

    /// For the child of a popup: the popup whose `Shadow` the child casts as
    /// its own (`NeedsDropShadowVisual`: "Child of Popup with Shadow is
    /// implicitly a shadow caster").
    pub fn set_theme_shadow_popup(&self, popup: Option<&Visual>) {
        self.with_composition_extras(|extras| {
            extras.theme_shadow.get_or_insert_with(ThemeShadowState::default).popup =
                popup.map(|popup| popup.to_ref().downgrade());
        });
        self.theme_shadow_changed();
    }

    /// The `Shadow` of the visual changed: the shadow learns its new owner,
    /// the visual is drawn again and its parent shows or drops the shadow
    /// visual.
    pub(super) fn on_shadow_changed(
        &self,
        old: Option<&Ref<crate::media::Shadow>>,
        new: Option<&Ref<crate::media::Shadow>>,
    ) {
        if let Some(old) = old.and_then(|old| old.as_theme_shadow()) {
            old.remove_owner(self);
        }
        if let Some(new) = new.and_then(|new| new.as_theme_shadow()) {
            new.add_owner(self);
        }
        if new.is_some() {
            self.with_composition_extras(|extras| {
                extras.theme_shadow.get_or_insert_with(ThemeShadowState::default);
            });
        }
        self.theme_shadow_changed();
    }

    /// Something that decides whether or how the visual casts a theme shadow
    /// changed (outside of a render pass): the shadow visual of a visual that
    /// casts none any more is released, and the visual and its parent are
    /// synchronized again.
    fn theme_shadow_changed(&self) {
        if self.theme_shadow_caster().is_none() {
            self.release_theme_shadow_visual();
        }
        self.invalidate_visual();
        if let (Some(parent), Some(source)) = (self.visual_parent(), self.presentation_source()) {
            source.renderer().recalculate_children(&parent);
        }
    }

    /// The composition visual that draws the theme shadow of this visual,
    /// created when the visual casts one and has a composition visual, and
    /// brought up to date: what the parent of the visual shows just before
    /// the visual's own composition visual.
    pub(super) fn ensure_theme_shadow_composition_visual(&self) -> Option<Rc<CompositionVisual>> {
        let (shadow, elevation) = self.theme_shadow_caster()?;
        let caster = self.composition_visual()?;
        let existing = self.theme_shadow_visual().filter(|existing| {
            Rc::ptr_eq(existing.visual().compositor(), caster.compositor()) && !existing.visual().is_disposed()
        });
        let shadow_visual = match existing {
            Some(existing) => existing,
            None => {
                self.release_theme_shadow_visual();
                let created = Rc::new(ThemeShadowVisual::new(&caster));
                self.with_composition_extras(|extras| {
                    extras.theme_shadow.get_or_insert_with(ThemeShadowState::default).visual = Some(created.clone());
                });
                created
            }
        };
        self.configure_theme_shadow_visual(&shadow_visual, &shadow, &elevation);
        Some(shadow_visual.visual().clone())
    }

    /// Brings the shadow visual of the visual up to date, when it has one
    /// (part of the synchronization of the composition properties).
    pub(super) fn update_theme_shadow_visual(&self) {
        let Some(shadow_visual) = self.theme_shadow_visual() else { return };
        match self.theme_shadow_caster() {
            Some((shadow, elevation)) => self.configure_theme_shadow_visual(&shadow_visual, &shadow, &elevation),
            None => shadow_visual.update(&DropShadowRecipe::new(0.0, ThemeShadowTheme::Light), Vector::default(), false),
        }
    }

    /// Releases the shadow visual (the visual casts no shadow any more, or
    /// leaves its compositor).
    pub(super) fn release_theme_shadow_visual(&self) {
        let released = self
            .with_composition_extras(|extras| extras.theme_shadow.as_mut().and_then(|state| state.visual.take()));
        if let Some(released) = released {
            released.dispose();
        }
    }

    /// `UpdateDropShadowVisual`: the recipe of the elevation of `elevation`
    /// (the Z of its translation) in the theme of this visual, with the
    /// corner radius of the nearest rounded element, over the size of this
    /// visual; hidden while this visual is hidden and in a high contrast
    /// theme.
    fn configure_theme_shadow_visual(
        &self,
        shadow_visual: &ThemeShadowVisual,
        _shadow: &Ref<ThemeShadow>,
        elevation: &Ref<Visual>,
    ) {
        let theme = ThemeShadowTheme::of(self);
        let (radius_x, radius_y) = self.theme_shadow_corner_radius();
        let recipe =
            DropShadowRecipe::new(elevation.translation().z as f32, theme).with_corner_radius(radius_x, radius_y);
        let size = self.bounds().size();
        let visible = theme != ThemeShadowTheme::HighContrast
            && self.composition_visual().is_some_and(|visual| visual.visible());
        shadow_visual.update(&recipe, Vector::new(size.width, size.height), visible);
    }

    /// `UpdateDropShadowRecipeCornerRadius`: from the visual up through its
    /// ancestors, the first element that is rounded: the radii of a
    /// rectangle (`RadiusX`, `RadiusY`) or the largest corner of a
    /// `CornerRadius` (a control, a border, a panel).
    fn theme_shadow_corner_radius(&self) -> (f64, f64) {
        let registry = FerroPropertyRegistry::instance();
        let mut current = Some(self.to_ref());
        while let Some(element) = current {
            let object: &crate::FerroObject = &element;
            if let (Some(radius_x), Some(radius_y)) =
                (registry.find_registered_for(object, "RadiusX"), registry.find_registered_for(object, "RadiusY"))
            {
                let x = object.get_value_untyped(radius_x).downcast_ref::<f64>().copied().unwrap_or(0.0);
                let y = object.get_value_untyped(radius_y).downcast_ref::<f64>().copied().unwrap_or(0.0);
                if x != 0.0 || y != 0.0 {
                    return (x, y);
                }
            } else if let Some(property) = registry.find_registered_for(object, "CornerRadius") {
                if let Some(radius) = object.get_value_untyped(property).downcast_ref::<CornerRadius>() {
                    let largest =
                        radius.top_left.max(radius.top_right).max(radius.bottom_left.max(radius.bottom_right));
                    if largest != 0.0 {
                        return (largest, largest);
                    }
                }
            }
            current = element.visual_parent();
        }
        (0.0, 0.0)
    }
}
