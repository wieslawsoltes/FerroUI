//! The remaining WinUI members of `Visual` (phase 2, not from upstream):
//! `RotationAngleInDegrees`, `RotationAxis`, the relative adjustments,
//! `IsHitTestVisible`, `IsPixelSnappingEnabled`, the 3D modes and
//! `ParentForTransform`.

use super::{
    CompositionBackfaceVisibility, CompositionBorderMode, CompositionCompositeMode, CompositionVisual,
    ICompositionObject,
};
use crate::{Vector, Vector3D};
use std::rc::Rc;

impl CompositionVisual {
    /// The rotation about the rotation axis in degrees: a view of
    /// [`rotation_angle`](Self::rotation_angle).
    ///
    /// Deviation (winui-composition.md): not an animation target of its own;
    /// animate `RotationAngle`.
    pub fn rotation_angle_in_degrees(&self) -> f32 {
        self.rotation_angle().to_degrees()
    }

    pub fn set_rotation_angle_in_degrees(&self, value: f32) {
        self.set_rotation_angle(value.to_radians())
    }

    /// The axis [`rotation_angle`](Self::rotation_angle) rotates about
    /// (default `(0, 0, 1)`; animatable as `RotationAxis`).
    ///
    /// Deviation (winui-composition.md): an axis along Z (or a zero axis)
    /// rotates in the plane; any other axis rotates in 3D, of which the 2D
    /// renderers show the projection, as for `Orientation`.
    pub fn rotation_axis(&self) -> Vector3D {
        self.visual_props().rotation_axis()
    }

    pub fn set_rotation_axis(&self, value: Vector3D) {
        self.visual_props().set_rotation_axis(self, value)
    }

    /// The part of the size of the parent added to the offset: the offset of
    /// the visual is `Offset + RelativeOffsetAdjustment * parent size`
    /// (animatable as `RelativeOffsetAdjustment`; Z is not adjusted, the
    /// parent has no depth).
    pub fn relative_offset_adjustment(&self) -> Vector3D {
        self.visual_props().relative_offset_adjustment()
    }

    pub fn set_relative_offset_adjustment(&self, value: Vector3D) {
        self.visual_props().set_relative_offset_adjustment(self, value)
    }

    /// The part of the size of the parent added to the size: the size of
    /// the visual is `Size + RelativeSizeAdjustment * parent size`
    /// (animatable as `RelativeSizeAdjustment`). `(1, 1)` with a zero `Size`
    /// makes the visual as large as its parent.
    pub fn relative_size_adjustment(&self) -> Vector {
        self.visual_props().relative_size_adjustment()
    }

    pub fn set_relative_size_adjustment(&self, value: Vector) {
        self.visual_props().set_relative_size_adjustment(self, value)
    }

    /// The size of the visual with its relative size adjustment applied: what
    /// is rendered and hit tested. [`size`](Self::size) is the `Size`
    /// property alone.
    pub fn effective_size(&self) -> Vector {
        let size = self.size();
        let relative = self.relative_size_adjustment();
        if relative == Vector::default() {
            return size;
        }
        let parent = self.parent().map(|parent| parent.effective_size()).unwrap_or_default();
        size + Vector::multiply(relative, parent)
    }

    /// Whether the visual and its subtree take part in hit testing (default
    /// `true`).
    pub fn is_hit_test_visible(&self) -> bool {
        self.visual_props().is_hit_test_visible()
    }

    pub fn set_is_hit_test_visible(&self, value: bool) {
        self.visual_props().set_is_hit_test_visible(self, value)
    }

    /// Whether the offset of the visual is snapped to device pixels.
    ///
    /// Deviation (winui-composition.md): the translation of the transform of
    /// the visual relative to its parent is rounded to device pixels of its
    /// target, not its position in the window.
    pub fn is_pixel_snapping_enabled(&self) -> bool {
        self.visual_props().is_pixel_snapping_enabled()
    }

    pub fn set_is_pixel_snapping_enabled(&self, value: bool) {
        self.visual_props().set_is_pixel_snapping_enabled(self, value)
    }

    /// How the edges of the visual are antialiased. Stored only.
    pub fn border_mode(&self) -> CompositionBorderMode {
        self.visual_props().border_mode()
    }

    pub fn set_border_mode(&self, value: CompositionBorderMode) {
        self.visual_props().set_border_mode(self, value)
    }

    /// Whether the back of the visual is shown when it is turned in 3D.
    /// Stored only.
    pub fn backface_visibility(&self) -> CompositionBackfaceVisibility {
        self.visual_props().backface_visibility()
    }

    pub fn set_backface_visibility(&self, value: CompositionBackfaceVisibility) {
        self.visual_props().set_backface_visibility(self, value)
    }

    /// How the visual is blended with what is behind it. Stored only.
    pub fn composite_mode(&self) -> CompositionCompositeMode {
        self.visual_props().composite_mode()
    }

    pub fn set_composite_mode(&self, value: CompositionCompositeMode) {
        self.visual_props().set_composite_mode(self, value)
    }

    /// The visual whose coordinate space the transform of this visual is
    /// relative to, instead of its parent: the visual is drawn where its
    /// parent is, in the place its offset and transform give it inside
    /// `parent_for_transform`.
    ///
    /// Deviation (winui-composition.md): the two visuals must be in one tree
    /// (otherwise the transform is relative to the parent); clips and
    /// opacity still come from the parent chain.
    pub fn parent_for_transform(&self) -> Option<Rc<CompositionVisual>> {
        self.visual_props()
            .parent_for_transform()
            .and_then(|visual| visual.into_any_rc().downcast::<CompositionVisual>().ok())
    }

    pub fn set_parent_for_transform(&self, value: Option<Rc<CompositionVisual>>) {
        self.visual_props().set_parent_for_transform(self, value.map(|visual| visual as Rc<dyn ICompositionObject>))
    }
}
