// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original preview mechanics. Show/managed host discovery are staged with the concrete DockControl.
use super::{
    dock_data_template_helper::DockDataTemplateHelper, dock_helpers::DockHelpers,
    drag_preview_context::DragPreviewContext,
};
use crate::{
    controls::{
        DragPreviewControl, DragPreviewWindow, ManagedDockWindowDocument, ManagedWindowLayer,
    },
    model_projection,
};
use ferrodock_controls_recycling::from_recycling_value;
use ferrodock_model::core::{IDockable, IFactory};
use ferrodock_settings::DockSettings;
use ferroui_base::{
    data::core::ValueTypes,
    media::{Stretch, VisualBrush},
    threading::{Dispatcher, DispatcherPriority},
    PixelPoint, Point, Rect, Ref, Size, Visual,
};
use ferroui_controls::{Border, Control, SizeToContent, TopLevel, Window};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
const MAX_WINDOW_SIZE_FREEZE_RETRIES: i32 = 4;
const USE_WINDOW_MOVE_COALESCING: bool = cfg!(target_os = "macos");
// Source Monitor is reentrant. No TLS/RefCell borrow is held across native or model callbacks.
// This preserves UI-thread reentry; process-wide/cross-thread Monitor equivalence remains open.
struct PreviewState {
    window: RefCell<Option<Ref<DragPreviewWindow>>>,
    control: RefCell<Option<Ref<DragPreviewControl>>>,
    window_templates_initialized: Cell<bool>,
    managed_templates_initialized: Cell<bool>,
    managed_control: RefCell<Option<Ref<DragPreviewControl>>>,
    managed_layer: RefCell<Option<Ref<ManagedWindowLayer>>>,
    pending_window_position: Cell<PixelPoint>,
    pending_status: RefCell<Option<String>>,
    has_pending_window_move: Cell<bool>,
    window_move_flush_scheduled: Cell<bool>,
    window_size_frozen: Cell<bool>,
    window_size_freeze_scheduled: Cell<bool>,
    window_size_freeze_abandoned: Cell<bool>,
    window_size_freeze_retry_count: Cell<i32>,
    frozen_window_width_pixels: Cell<f64>,
    frozen_window_height_pixels: Cell<f64>,
    frozen_content_width_pixels: Cell<f64>,
    frozen_content_height_pixels: Cell<f64>,
    last_frozen_window_scaling: Cell<f64>,
    window_move_session_id: Cell<i32>,
    window_move_post_sequence: Cell<i64>,
}
impl PreviewState {
    fn new() -> Self {
        Self {
            window: RefCell::new(None),
            control: RefCell::new(None),
            window_templates_initialized: Cell::new(false),
            managed_templates_initialized: Cell::new(false),
            managed_control: RefCell::new(None),
            managed_layer: RefCell::new(None),
            pending_window_position: Cell::new(PixelPoint::default()),
            pending_status: RefCell::new(Some(String::new())),
            has_pending_window_move: Cell::new(false),
            window_move_flush_scheduled: Cell::new(false),
            window_size_frozen: Cell::new(false),
            window_size_freeze_scheduled: Cell::new(false),
            window_size_freeze_abandoned: Cell::new(false),
            window_size_freeze_retry_count: Cell::new(0),
            frozen_window_width_pixels: Cell::new(0.0),
            frozen_window_height_pixels: Cell::new(0.0),
            frozen_content_width_pixels: Cell::new(f64::NAN),
            frozen_content_height_pixels: Cell::new(f64::NAN),
            last_frozen_window_scaling: Cell::new(1.0),
            window_move_session_id: Cell::new(0),
            window_move_post_sequence: Cell::new(0),
        }
    }
    fn window(&self) -> Option<Ref<DragPreviewWindow>> {
        self.window.borrow().clone()
    }
    fn control(&self) -> Option<Ref<DragPreviewControl>> {
        self.control.borrow().clone()
    }
    fn managed_control(&self) -> Option<Ref<DragPreviewControl>> {
        self.managed_control.borrow().clone()
    }
    fn managed_layer(&self) -> Option<Ref<ManagedWindowLayer>> {
        self.managed_layer.borrow().clone()
    }
    fn expect_window(&self) -> Ref<DragPreviewWindow> {
        self.window().expect("source current preview window")
    }
    fn expect_control(&self) -> Ref<DragPreviewControl> {
        self.control().expect("source current preview control")
    }
    fn expect_managed_control(&self) -> Ref<DragPreviewControl> {
        self.managed_control()
            .expect("source current managed preview")
    }
}
ferroui_base::ferro_thread_local! { static STATE: Rc<PreviewState> = Rc::new(PreviewState::new()); }
pub(crate) struct DragPreviewHelper;
impl DragPreviewHelper {
    pub(crate) fn new() -> Self {
        Self
    }
    fn state() -> Rc<PreviewState> {
        STATE.with(Clone::clone)
    }
    fn get_position_within_window(
        _window: &Window,
        position: PixelPoint,
        offset: PixelPoint,
    ) -> PixelPoint {
        PixelPoint::new(
            position.x.wrapping_add(offset.x),
            position.y.wrapping_add(offset.y),
        )
    }
    fn managed_document(item: Rc<dyn IDockable>) -> Option<Rc<ManagedDockWindowDocument>> {
        item.boxed_reference()
            .downcast_ref::<Rc<ManagedDockWindowDocument>>()
            .cloned()
    }
    fn get_preview_size(item: Rc<dyn IDockable>) -> Size {
        if let Some(managed) = Self::managed_document(item.clone()) {
            let bounds = managed.mdi_bounds();
            if !bounds.width.is_nan()
                && bounds.width > 0.0
                && !bounds.height.is_nan()
                && bounds.height > 0.0
            {
                return Size::new(bounds.width, bounds.height);
            }
        }
        let (_, _, mut width, mut height) = item.get_visible_bounds();
        let owner = item.owner().and_then(|o| o.as_dock());
        let mut owner_width = f64::NAN;
        let mut owner_height = f64::NAN;
        if let Some(owner) = owner {
            (_, _, owner_width, owner_height) = owner.get_visible_bounds();
        }
        if width.is_nan() || width <= 0.0 {
            width = if owner_width.is_nan() || owner_width <= 0.0 {
                300.0
            } else {
                owner_width
            };
        }
        if height.is_nan() || height <= 0.0 {
            height = if owner_height.is_nan() || owner_height <= 0.0 {
                400.0
            } else {
                owner_height
            };
        }
        Size::new(width, height)
    }
    fn ensure_data_templates(control: &DragPreviewControl, initialized: &Cell<bool>) {
        if initialized.get() {
            return;
        }
        for template in DockDataTemplateHelper::create_default_data_templates() {
            control.data_templates().add(template);
        }
        initialized.set(true);
    }
    fn clamp_opacity(value: f64) -> f64 {
        if value.is_nan() {
            return 1.0;
        }
        if value < 0.0 {
            return 0.0;
        }
        if value > 1.0 {
            1.0
        } else {
            value
        }
    }
    fn queue_window_move(
        window: &DragPreviewWindow,
        control: &DragPreviewControl,
        target: PixelPoint,
        status: Option<&str>,
    ) {
        let state = Self::state();
        let current_position = window.position();
        let current_status = control.status();
        if state.window_move_flush_scheduled.get()
            && state.has_pending_window_move.get()
            && state.pending_window_position.get() == target
            && state.pending_status.borrow().as_deref() == status
        {
            return;
        }
        if !state.window_move_flush_scheduled.get()
            && current_position == target
            && current_status.as_deref() == status
        {
            return;
        }
        if !USE_WINDOW_MOVE_COALESCING {
            Self::apply_window_move(window, control, target, status);
            return;
        }
        state.pending_window_position.set(target);
        state.pending_status.replace(status.map(str::to_owned));
        state.has_pending_window_move.set(true);
        if !state.window_move_flush_scheduled.get() {
            state.window_move_flush_scheduled.set(true);
            let session = state.window_move_session_id.get();
            let sequence = state.window_move_post_sequence.get().wrapping_add(1);
            state.window_move_post_sequence.set(sequence);
            Dispatcher::ui_thread().post_local(
                move || Self::flush_pending_window_move(session, sequence),
                DispatcherPriority::RENDER,
            );
        }
    }
    fn flush_pending_window_move(session: i32, sequence: i64) {
        let state = Self::state();
        if session != state.window_move_session_id.get()
            || sequence != state.window_move_post_sequence.get()
        {
            return;
        }
        state.window_move_flush_scheduled.set(false);
        if !state.has_pending_window_move.get()
            || state.window().is_none()
            || state.control().is_none()
        {
            state.has_pending_window_move.set(false);
            return;
        }
        let window = state.expect_window();
        let control = state.expect_control();
        let position = state.pending_window_position.get();
        let status = state.pending_status.borrow().clone();
        Self::apply_window_move(&window, &control, position, status.as_deref());
        state.has_pending_window_move.set(false);
        // Preserve the source's post-apply overwrite and subsequent check, including reentry.
        if state.has_pending_window_move.get() && !state.window_move_flush_scheduled.get() {
            state.window_move_flush_scheduled.set(true);
            let session = state.window_move_session_id.get();
            let sequence = state.window_move_post_sequence.get().wrapping_add(1);
            state.window_move_post_sequence.set(sequence);
            Dispatcher::ui_thread().post_local(
                move || Self::flush_pending_window_move(session, sequence),
                DispatcherPriority::RENDER,
            );
        }
    }
    fn apply_window_move(
        window: &DragPreviewWindow,
        control: &DragPreviewControl,
        target: PixelPoint,
        status: Option<&str>,
    ) {
        if control.status().as_deref() != status {
            control.set_status(status.map(str::to_owned));
        }
        let current = window.position();
        let will_move = current != target;
        if will_move {
            window.set_position(target);
        }
        let _applied_position = if will_move { target } else { current };
        let state = Self::state();
        if state.window_size_frozen.get() {
            Self::maintain_frozen_window_size(window, control);
        }
        if !state.window_size_frozen.get()
            && !state.window_size_freeze_scheduled.get()
            && !state.window_size_freeze_abandoned.get()
            && status.is_some_and(|s| !s.is_empty())
        {
            state.window_size_freeze_scheduled.set(true);
            Dispatcher::ui_thread().post_local(
                Self::freeze_window_size_if_needed,
                DispatcherPriority::RENDER,
            );
        }
    }
    fn get_window_scaling(window: &Window) -> f64 {
        let scale = window.render_scaling();
        if scale > 0.0 {
            scale
        } else {
            1.0
        }
    }
    fn maintain_frozen_window_size(window: &DragPreviewWindow, control: &DragPreviewControl) {
        let scale = Self::get_window_scaling(window);
        let state = Self::state();
        if (scale - state.last_frozen_window_scaling.get()).abs() < 0.0001 {
            return;
        }
        state.last_frozen_window_scaling.set(scale);
        let width = state.frozen_window_width_pixels.get() / scale;
        let height = state.frozen_window_height_pixels.get() / scale;
        if (window.width() - width).abs() > 0.01 {
            window.set_width(width);
        }
        if (window.height() - height).abs() > 0.01 {
            window.set_height(height);
        }
        if !state.frozen_content_width_pixels.get().is_nan() {
            let width = state.frozen_content_width_pixels.get() / scale;
            if (control.preview_content_width() - width).abs() > 0.01 {
                control.set_preview_content_width(width);
            }
        }
        if !state.frozen_content_height_pixels.get().is_nan() {
            let height = state.frozen_content_height_pixels.get() / scale;
            if (control.preview_content_height() - height).abs() > 0.01 {
                control.set_preview_content_height(height);
            }
        }
    }
    fn freeze_window_size_if_needed() {
        let state = Self::state();
        state.window_size_freeze_scheduled.set(false);
        if state.window_size_frozen.get()
            || state.window().is_none()
            || !state.expect_window().is_visible()
        {
            return;
        }
        let bounds = state.expect_window().bounds();
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            if state.window_size_freeze_retry_count.get() >= MAX_WINDOW_SIZE_FREEZE_RETRIES {
                state.window_size_freeze_abandoned.set(true);
                return;
            }
            state
                .window_size_freeze_retry_count
                .set(state.window_size_freeze_retry_count.get().wrapping_add(1));
            state.window_size_freeze_scheduled.set(true);
            Dispatcher::ui_thread().post_local(
                Self::freeze_window_size_if_needed,
                DispatcherPriority::BACKGROUND,
            );
            return;
        }
        state.window_size_freeze_retry_count.set(0);
        state.window_size_freeze_abandoned.set(false);
        let scale = Self::get_window_scaling(&state.expect_window());
        state.last_frozen_window_scaling.set(scale);
        state.frozen_window_width_pixels.set(bounds.width * scale);
        state.frozen_window_height_pixels.set(bounds.height * scale);
        state.frozen_content_width_pixels.set(
            if !state
                .control()
                .map_or(f64::NAN, |c| c.preview_content_width())
                .is_nan()
            {
                state.expect_control().preview_content_width() * scale
            } else {
                f64::NAN
            },
        );
        state.frozen_content_height_pixels.set(
            if !state
                .control()
                .map_or(f64::NAN, |c| c.preview_content_height())
                .is_nan()
            {
                state.expect_control().preview_content_height() * scale
            } else {
                f64::NAN
            },
        );
        state
            .expect_window()
            .set_size_to_content(SizeToContent::MANUAL);
        state.expect_window().set_width(bounds.width);
        state.expect_window().set_height(bounds.height);
        state.window_size_frozen.set(true);
    }
    // Exact floating-window continuation of Show, called by its staged original entry point.
    fn show_floating(
        dockable: Rc<dyn IDockable>,
        position: PixelPoint,
        offset: PixelPoint,
        show: bool,
        preferred: Option<Size>,
    ) {
        let state = Self::state();
        if state.window().is_none() || state.control().is_none() {
            let control = DragPreviewControl::new();
            control.set_status(Some(String::new()));
            state.control.replace(Some(control));
            Self::ensure_data_templates(
                &state.expect_control(),
                &state.window_templates_initialized,
            );
            let window = DragPreviewWindow::new();
            window.set_content(Some(Control::boxed(
                state.expect_control().upcast::<Control>(),
            )));
            state.window.replace(Some(window));
        }
        state.expect_control().set_show_content(show);
        if show {
            let size = preferred
                .filter(|s| s.width > 0.0 && s.height > 0.0)
                .unwrap_or_else(|| Self::get_preview_size(dockable.clone()));
            state.expect_control().set_preview_content_width(size.width);
            state
                .expect_control()
                .set_preview_content_height(size.height);
        } else {
            state.expect_control().set_preview_content_width(f64::NAN);
            state.expect_control().set_preview_content_height(f64::NAN);
        }
        let control = state.expect_control();
        control.set_preview_content(Self::build_dockable_preview_content(dockable.clone(), show));
        state
            .expect_window()
            .set_data_context(Some(model_projection::to_dockable_value(dockable)));
        state.expect_control().set_status(Some(String::new()));
        state
            .expect_window()
            .set_opacity(Self::clamp_opacity(DockSettings::drag_preview_opacity()));
        state
            .expect_window()
            .set_size_to_content(SizeToContent::WIDTH_AND_HEIGHT);
        state.expect_window().set_width(
            preferred
                .filter(|s| s.width > 0.0)
                .map_or(f64::NAN, |s| s.width),
        );
        state.expect_window().set_height(
            preferred
                .filter(|s| s.height > 0.0)
                .map_or(f64::NAN, |s| s.height),
        );
        let window = state.expect_window();
        window.set_position(Self::get_position_within_window(
            &state.expect_window(),
            position,
            offset,
        ));
        state
            .pending_window_position
            .set(state.expect_window().position());
        state
            .pending_status
            .replace(state.expect_control().status());
        state.has_pending_window_move.set(false);
        state.window_move_flush_scheduled.set(false);
        state
            .window_move_session_id
            .set(state.window_move_session_id.get().wrapping_add(1));
        state.window_move_post_sequence.set(0);
        state.window_size_frozen.set(false);
        state.window_size_freeze_scheduled.set(false);
        state.window_size_freeze_abandoned.set(false);
        state.window_size_freeze_retry_count.set(0);
        state.frozen_window_width_pixels.set(0.0);
        state.frozen_window_height_pixels.set(0.0);
        state.frozen_content_width_pixels.set(f64::NAN);
        state.frozen_content_height_pixels.set(f64::NAN);
        state.last_frozen_window_scaling.set(1.0);
        if let Some(size) = preferred.filter(|s| s.width > 0.0 && s.height > 0.0) {
            state
                .expect_window()
                .set_size_to_content(SizeToContent::MANUAL);
            state.expect_window().set_width(size.width);
            state.expect_window().set_height(size.height);
        } else {
            state.expect_control().apply_template();
            state.expect_control().measure(Size::INFINITY);
            let initial = state.expect_control().desired_size();
            if initial.width > 0.0 && initial.height > 0.0 {
                state
                    .expect_window()
                    .set_size_to_content(SizeToContent::MANUAL);
                state.expect_window().set_width(initial.width);
                state.expect_window().set_height(initial.height);
            }
        }
        #[cfg(debug_assertions)]
        eprintln!("[Dock PreviewSize Window] preferredDip={} appliedWindowDip=({:.2},{:.2}) sizeToContent={} previewContentDip=({:.2},{:.2})",preferred.map_or_else(|| "null".into(),|p|format!("({:.2},{:.2})",p.width,p.height)),state.expect_window().width(),state.expect_window().height(),{
            let mode=state.expect_window().size_to_content();
            <SizeToContent as ferroui_base::metadata::MarkupTyped>::MARKUP.enum_members.iter()
                .find(|member| (*(member.get)()).value_eq(&mode))
                .map_or_else(||mode.bits().to_string(),|member|member.name.to_owned())
        },state.expect_control().preview_content_width(),state.expect_control().preview_content_height());
        if !state.expect_window().is_visible() {
            state.expect_window().show();
        }
        Dispatcher::ui_thread().post_local(|| {
            let state=Self::state(); if state.window().is_none() || state.control().is_none() { return; }
            let content=state.expect_window().content().as_ref().and_then(Control::from_boxed);
            let wb=state.expect_window().bounds();let cb=content.map_or(Rect::default(),|c|c.bounds());
            #[cfg(debug_assertions)]
            eprintln!("[Dock PreviewSize AfterShow] windowBoundsDip=({:.2},{:.2}) contentBoundsDip=({:.2},{:.2}) previewContentDip=({:.2},{:.2}) scale={:.2}",wb.width,wb.height,cb.width,cb.height,state.expect_control().preview_content_width(),state.expect_control().preview_content_height(),state.expect_window().render_scaling());
            let _=(wb,cb);
        },DispatcherPriority::RENDER);
    }
    pub(crate) fn move_(&self, position: PixelPoint, offset: PixelPoint, status: &str) {
        let state = Self::state();
        if DockHelpers::is_managed_window_hosting_enabled_for_dockable(
            DragPreviewContext::dockable(),
        ) {
            if let Some(layer) = state.managed_layer() {
                if state.managed_control().is_some() {
                    Self::move_managed(&layer, position, offset, Some(status));
                    return;
                }
            }
        }
        if state.window().is_none() || state.control().is_none() {
            return;
        }
        let target = Self::get_position_within_window(&state.expect_window(), position, offset);
        Self::queue_window_move(
            &state.expect_window(),
            &state.expect_control(),
            target,
            Some(status),
        );
    }
    pub(crate) fn hide(&self) {
        let state = Self::state();
        if state.managed_layer().is_some() {
            state
                .managed_layer()
                .expect("source managed layer")
                .hide_overlay("DragPreview");
            if let Some(control) = state.managed_control() {
                control.set_preview_content(None);
            }
            state.managed_layer.replace(None);
            state.managed_control.replace(None);
            state.managed_templates_initialized.set(false);
            DragPreviewContext::clear();
            return;
        }
        DragPreviewContext::clear();
        if state.window().is_none() {
            return;
        }
        state.has_pending_window_move.set(false);
        state.window_move_flush_scheduled.set(false);
        state
            .window_move_session_id
            .set(state.window_move_session_id.get().wrapping_add(1));
        state.window_move_post_sequence.set(0);
        state.window_size_frozen.set(false);
        state.window_size_freeze_scheduled.set(false);
        state.window_size_freeze_abandoned.set(false);
        state.window_size_freeze_retry_count.set(0);
        state.frozen_window_width_pixels.set(0.0);
        state.frozen_window_height_pixels.set(0.0);
        state.frozen_content_width_pixels.set(f64::NAN);
        state.frozen_content_height_pixels.set(f64::NAN);
        state.last_frozen_window_scaling.set(1.0);
        state.expect_window().close();
        state.window.replace(None);
        state.control.replace(None);
        state.window_templates_initialized.set(false);
    }
    fn find_factory(item: Option<Rc<dyn IDockable>>) -> Option<Rc<dyn IFactory>> {
        let mut current = item;
        while let Some(item) = current {
            if let Some(factory) = item.factory() {
                return Some(factory);
            }
            current = item.owner();
        }
        None
    }
    fn is_layer_ready(layer: Option<&ManagedWindowLayer>) -> bool {
        layer.is_some_and(|l| l.is_visible() && l.visual_root().is_some())
    }
    fn is_layer_in_root(layer: Option<&ManagedWindowLayer>, root: Option<&Visual>) -> bool {
        let Some(layer) = layer else {
            return false;
        };
        root.is_none_or(|r| layer.visual_root().is_some_and(|l| l.ptr_eq(&r.to_ref())))
    }
    fn show_managed(
        layer: &ManagedWindowLayer,
        item: Rc<dyn IDockable>,
        position: PixelPoint,
        offset: PixelPoint,
        show: bool,
        preferred: Option<Size>,
    ) {
        let state = Self::state();
        if state.managed_control().is_none() {
            let control = DragPreviewControl::new();
            control.set_status(Some(String::new()));
            state.managed_control.replace(Some(control));
            Self::ensure_data_templates(
                &state.expect_managed_control(),
                &state.managed_templates_initialized,
            );
        }
        state.managed_layer.replace(Some(layer.to_ref()));
        let forced = preferred.filter(|s| s.width > 0.0 && s.height > 0.0);
        let target = forced.unwrap_or_else(|| {
            if show {
                Self::get_preview_size(item.clone())
            } else {
                Size::new(f64::NAN, f64::NAN)
            }
        });
        state
            .expect_managed_control()
            .set_data_context(Some(model_projection::to_dockable_value(item.clone())));
        state
            .expect_managed_control()
            .set_status(Some(String::new()));
        state
            .expect_managed_control()
            .set_opacity(Self::clamp_opacity(DockSettings::drag_preview_opacity()));
        state.expect_managed_control().set_show_content(show);
        if show {
            state
                .expect_managed_control()
                .set_preview_content_width(target.width);
            state
                .expect_managed_control()
                .set_preview_content_height(target.height);
        } else {
            state
                .expect_managed_control()
                .set_preview_content_width(f64::NAN);
            state
                .expect_managed_control()
                .set_preview_content_height(f64::NAN);
        }
        let control = state.expect_managed_control();
        control.set_preview_content(Self::build_managed_preview_content(item.clone(), show));
        let local = Self::get_managed_position(layer, position, offset);
        layer.show_overlay_at(
            "DragPreview",
            state.expect_managed_control().upcast(),
            local,
            forced,
            false,
        );
        state.expect_managed_control().apply_template();
        state.expect_managed_control().measure(Size::INFINITY);
        let mut measured = state.expect_managed_control().desired_size();
        if show
            && Self::managed_document(item).is_some()
            && !target.height.is_nan()
            && target.height > 0.0
        {
            let overage = measured.height - target.height;
            if overage > 0.0 {
                let mut adjusted = target.height - overage;
                if adjusted < 0.0 {
                    adjusted = 0.0;
                }
                state
                    .expect_managed_control()
                    .set_preview_content_height(adjusted);
                state.expect_managed_control().measure(Size::INFINITY);
                measured = state.expect_managed_control().desired_size();
            }
        }
        layer.show_overlay_at(
            "DragPreview",
            state.expect_managed_control().upcast(),
            local,
            Some(forced.unwrap_or(measured)),
            false,
        );
    }
    fn move_managed(
        layer: &ManagedWindowLayer,
        position: PixelPoint,
        offset: PixelPoint,
        status: Option<&str>,
    ) {
        let state = Self::state();
        if state.managed_control().is_none() {
            return;
        }
        let local = Self::get_managed_position(layer, position, offset);
        if state.expect_managed_control().status().as_deref() != status {
            state
                .expect_managed_control()
                .set_status(status.map(str::to_owned));
            state.expect_managed_control().measure(Size::INFINITY);
            let control = state.expect_managed_control();
            let measured = state.expect_managed_control().desired_size();
            layer.show_overlay_at(
                "DragPreview",
                control.upcast(),
                local,
                Some(measured),
                false,
            );
            return;
        }
        let control = state.expect_managed_control();
        let measured = state.expect_managed_control().desired_size();
        layer.show_overlay_at(
            "DragPreview",
            control.upcast(),
            local,
            Some(measured),
            false,
        );
    }
    // Retained source helper; the current source callers use position directly.
    #[allow(dead_code)]
    fn get_managed_bounds(
        layer: &ManagedWindowLayer,
        position: PixelPoint,
        offset: PixelPoint,
        size: Size,
    ) -> Rect {
        let local = Self::get_managed_position(layer, position, offset);
        Rect::new(local.x, local.y, size.width, size.height)
    }
    fn get_managed_position(
        layer: &ManagedWindowLayer,
        position: PixelPoint,
        offset: PixelPoint,
    ) -> Point {
        let Some(top) = layer.visual_root().and_then(|r| r.cast::<TopLevel>()) else {
            return Point::default();
        };
        let target = PixelPoint::new(
            position.x.wrapping_add(offset.x),
            position.y.wrapping_add(offset.y),
        );
        let client = top.point_to_client(target);
        let origin = layer
            .translate_point(Point::default(), &top)
            .unwrap_or_default();
        Point::new(client.x - origin.x, client.y - origin.y)
    }
    fn should_show_preview_content(_item: &dyn IDockable) -> bool {
        DockSettings::show_dockable_preview_on_drag()
    }
    pub(crate) fn build_dockable_preview_content(
        item: Rc<dyn IDockable>,
        show: bool,
    ) -> Option<Ref<Control>> {
        if !show {
            return None;
        }
        let factory = item.factory()?;
        let registered = if item.clone().as_tool().is_some() {
            factory.tool_controls().borrow().get(&item).cloned()
        } else if item.clone().as_document().is_some() {
            factory.document_controls().borrow().get(&item).cloned()
        } else {
            None
        };
        let native = from_recycling_value(&registered?);
        let visual = ValueTypes::as_object(&*native)?.cast::<Visual>()?;
        visual.visual_root()?;
        let brush = VisualBrush::with_visual(visual);
        brush.set_stretch(Stretch::Uniform);
        let border = Border::new();
        border.set_background(Some(brush.into()));
        border.set_clip_to_bounds(true);
        Some(border.upcast())
    }
    fn build_managed_preview_content(item: Rc<dyn IDockable>, show: bool) -> Option<Ref<Control>> {
        if !show {
            return None;
        }
        if let Some(managed) = Self::managed_document(item.clone()) {
            return managed.build(None);
        }
        Self::build_dockable_preview_content(item, show)
    }
}
#[cfg(test)]
#[path = "drag_preview_helper_tests.rs"]
mod tests;

mod host_discovery {
    // Copyright (c) Wiesław Šoltés. All rights reserved.
    // Licensed under the MIT license. See LICENSE.TXT in the workspace root.
    use crate::{
        controls::{DockControl, ManagedWindowLayer},
        internal::{
            dock_helpers::DockHelpers, drag_preview_context::DragPreviewContext,
            drag_preview_helper::DragPreviewHelper, managed_window_registry::ManagedWindowRegistry,
        },
    };
    use ferrodock_model::core::IDockable;
    use ferroui_base::{PixelPoint, Ref, Size, Visual};
    use std::{any::TypeId, rc::Rc};
    impl DragPreviewHelper {
        pub(crate) fn show(
            &self,
            item: Rc<dyn IDockable>,
            position: PixelPoint,
            offset: PixelPoint,
            context: Option<&Visual>,
            preferred: Option<Size>,
        ) {
            if Self::managed_document(item.clone()).is_some() {
                self.hide();
                return;
            }
            let show = Self::should_show_preview_content(&*item);
            DragPreviewContext::set_is_active(
                show && Self::managed_document(item.clone()).is_some(),
            );
            DragPreviewContext::set_dockable(Some(item.clone()));
            if DockHelpers::is_managed_window_hosting_enabled_for_dockable(Some(item.clone())) {
                if let Some(layer) = Self::try_get_managed_layer(item.clone(), context) {
                    Self::show_managed(&layer, item, position, offset, show, preferred);
                    return;
                }
            }
            Self::show_floating(item, position, offset, show, preferred);
        }
        fn try_get_managed_layer(
            item: Rc<dyn IDockable>,
            context: Option<&Visual>,
        ) -> Option<Ref<ManagedWindowLayer>> {
            let root = context.and_then(Visual::visual_root);
            if let Some(context) = context {
                let resolved = ManagedWindowLayer::try_get_layer(Some(context));
                if Self::is_layer_ready(resolved.as_deref())
                    && Self::is_layer_in_root(resolved.as_deref(), root.as_deref())
                {
                    return resolved;
                }
            }
            let factory = Self::find_factory(Some(item))?;
            let registered = ManagedWindowRegistry::try_get_layer(&factory);
            if Self::is_layer_ready(registered.as_deref())
                && Self::is_layer_in_root(registered.as_deref(), root.as_deref())
            {
                return registered;
            }
            for control in factory.dock_controls().iter() {
                let Some(control) = control
                    .boxed_interface(TypeId::of::<Ref<DockControl>>())
                    .and_then(|v| v.downcast_ref::<Ref<DockControl>>().cloned())
                else {
                    continue;
                };
                let candidate = control
                    .get_visual_descendants()
                    .find_map(|v| v.cast::<ManagedWindowLayer>());
                if Self::is_layer_ready(candidate.as_deref())
                    && Self::is_layer_in_root(candidate.as_deref(), root.as_deref())
                {
                    return candidate;
                }
            }
            None
        }
    }
}
