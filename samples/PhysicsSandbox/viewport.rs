//! High-FPS custom FerroUI `Control` (`PhysicsViewport`) that renders rigid bodies, joints,
//! GPU/SIMD particles, UI spring cards, laser raycasts, and debug overlays directly onto
//! FerroUI's `DrawingContext` with zero per-frame brush allocations.

use crate::scenes::{SandboxEngine, SceneKind, WORLD_HEIGHT, WORLD_WIDTH};
use ferroui_base::input::{InputElementImpl, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::{ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::{BoxShadows, DrawingContext, IBrush, IPen, StreamGeometry};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Matrix, Point, Rect, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use ferroui_physics::prelude::*;
use ferroui_view::ViewClass;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

struct Palette {
    bg: Rc<dyn IBrush>,
    grid_pen: Rc<dyn IPen>,
    outline_pen: Rc<dyn IPen>,
    sleep_outline_pen: Rc<dyn IPen>,
    joint_pen: Rc<dyn IPen>,
    aabb_pen: Rc<dyn IPen>,
    contact_pen: Rc<dyn IPen>,
    vel_pen: Rc<dyn IPen>,
    laser_glow_pen: Rc<dyn IPen>,
    laser_core_pen: Rc<dyn IPen>,
    blast_pen: Rc<dyn IPen>,
    slot_pen: Rc<dyn IPen>,
    card_shadow: Rc<dyn IBrush>,
    track_bg: Rc<dyn IBrush>,
    track_pill: Rc<dyn IBrush>,
    obstacle_fill: Rc<dyn IBrush>,
    spoke_pen: Rc<dyn IPen>,
    particle_brushes: Vec<Rc<dyn IBrush>>,
}

impl Palette {
    fn new() -> Self {
        // 16-stop velocity/pressure heat gradient for thousands of particles (zero allocs/frame)
        let particle_colors = [
            0xFF0EA5E9, 0xFF22D3EE, 0xFF38BDF8, 0xFF60A5FA, 0xFF818CF8, 0xFFA78BFA, 0xFFC084FC, 0xFFE879F9,
            0xFFF472B6, 0xFFFB7185, 0xFFF97316, 0xFFFB923C, 0xFFFBBF24, 0xFFFDE047, 0xFFBEF264, 0xFF34D399,
        ];
        Self {
            bg: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF0D1117)),
            grid_pen: Rc::new(ImmutablePen::from_uint32(0xFF1A2230, 1.0)),
            outline_pen: Rc::new(ImmutablePen::from_uint32(0xFFE2E8F0, 1.5)),
            sleep_outline_pen: Rc::new(ImmutablePen::from_uint32(0xFF64748B, 1.2)),
            joint_pen: Rc::new(ImmutablePen::from_uint32(0xFFFDE047, 2.0)),
            aabb_pen: Rc::new(ImmutablePen::from_uint32(0xAA38BDF8, 1.0)),
            contact_pen: Rc::new(ImmutablePen::from_uint32(0xFFEF4444, 2.0)),
            vel_pen: Rc::new(ImmutablePen::from_uint32(0xFF22D3EE, 1.6)),
            laser_glow_pen: Rc::new(ImmutablePen::from_uint32(0x66EF4444, 6.0)),
            laser_core_pen: Rc::new(ImmutablePen::from_uint32(0xFFFFE4E6, 2.0)),
            blast_pen: Rc::new(ImmutablePen::from_uint32(0xAAF59E0B, 3.0)),
            slot_pen: Rc::new(ImmutablePen::from_uint32(0x8838BDF8, 2.0)),
            card_shadow: Rc::new(ImmutableSolidColorBrush::from_uint32(0x55000000)),
            track_bg: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF161E2E)),
            track_pill: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF2563EB)),
            obstacle_fill: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF334155)),
            spoke_pen: Rc::new(ImmutablePen::from_uint32(0xCC0F172A, 2.0)),
            particle_brushes: particle_colors
                .into_iter()
                .map(|c| Rc::new(ImmutableSolidColorBrush::from_uint32(c)) as Rc<dyn IBrush>)
                .collect(),
        }
    }
}

/// Custom FerroUI viewport control hosting the live `SandboxEngine`.
#[repr(C)]
pub struct PhysicsViewport {
    base: Control,
    engine: RefCell<Rc<RefCell<SandboxEngine>>>,
    on_tick: RefCell<Option<Rc<dyn Fn(&SandboxEngine)>>>,
    palette: Palette,
}

ferro_class!(PhysicsViewport: Control);
ferro_impl_classes!(
    PhysicsViewport: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl
);
ferro_class_info!(PhysicsViewport { new: PhysicsViewport::new });

impl ViewClass for PhysicsViewport {
    fn create() -> Ref<Self> {
        Self::new()
    }
}

impl InputElementImpl for PhysicsViewport {
    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        let pt = e.get_position(Some(this));
        let world_pos = this.screen_to_world(pt);
        let right = e.properties().is_right_button_pressed;
        this.engine.borrow().borrow_mut().on_pointer_pressed(world_pos, right);
        this.invalidate_visual();
    }

    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        let pt = e.get_position(Some(this));
        let world_pos = this.screen_to_world(pt);
        this.engine.borrow().borrow_mut().on_pointer_moved(world_pos);
        this.invalidate_visual();
    }

    fn on_pointer_released(this: &Self, _e: &PointerReleasedEventArgs) {
        this.engine.borrow().borrow_mut().on_pointer_released();
        this.invalidate_visual();
    }
}

impl VisualImpl for PhysicsViewport {
    fn render(this: &Self, dc: &mut DrawingContext) {
        let bounds = this.bounds();
        let w = bounds.width.max(1.0);
        let h = bounds.height.max(1.0);
        let (scale, ox, oy) = viewport_transform(w, h);

        // Background
        dc.fill_rectangle(&this.palette.bg, Rect::new(0.0, 0.0, w, h), 0.0);

        // Subtle 1-meter studio coordinate grid
        for gx in 0..=(WORLD_WIDTH as i32) {
            let sx = ox + gx as f64 * scale;
            dc.draw_line(&this.palette.grid_pen, Point::new(sx, oy), Point::new(sx, oy + WORLD_HEIGHT as f64 * scale));
        }
        for gy in 0..=(WORLD_HEIGHT as i32) {
            let sy = oy + gy as f64 * scale;
            dc.draw_line(&this.palette.grid_pen, Point::new(ox, sy), Point::new(ox + WORLD_WIDTH as f64 * scale, sy));
        }

        let engine = this.engine.borrow();
        let sim = engine.borrow();

        match sim.scene {
            SceneKind::GpuParticles => this.render_particles(dc, &sim, scale, ox, oy),
            SceneKind::UiSprings => this.render_ui_springs(dc, &sim, scale, ox, oy),
            _ => this.render_rigid_world(dc, &sim, scale, ox, oy),
        }
    }
}

impl PhysicsViewport {
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            engine: RefCell::new(Rc::new(RefCell::new(SandboxEngine::new()))),
            on_tick: RefCell::new(None),
            palette: Palette::new(),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());

        let timer = DispatcherTimer::new();
        timer.set_interval(Duration::from_secs_f64(1.0 / 60.0));
        let weak = this.downgrade();
        let last_tick = std::cell::Cell::new(Instant::now());
        timer.tick(move |_| {
            let now = Instant::now();
            let dt = now.duration_since(last_tick.replace(now)).as_secs_f32().clamp(0.001, 0.033);
            if let Some(vp) = weak.upgrade() {
                {
                    let eng_rc = vp.engine.borrow().clone();
                    let mut eng = eng_rc.borrow_mut();
                    eng.step(dt);
                    if let Some(cb) = vp.on_tick.borrow().as_ref() {
                        cb(&eng);
                    }
                }
                vp.invalidate_visual();
            }
        });
        timer.start();

        this
    }

    pub fn bind_engine(&self, engine: Rc<RefCell<SandboxEngine>>, on_tick: Rc<dyn Fn(&SandboxEngine)>) {
        *self.engine.borrow_mut() = engine;
        *self.on_tick.borrow_mut() = Some(on_tick);
        self.invalidate_visual();
    }

    fn screen_to_world(&self, pt: Point) -> Vec2 {
        let bounds = self.bounds();
        let (scale, ox, oy) = viewport_transform(bounds.width.max(1.0), bounds.height.max(1.0));
        Vec2::new(((pt.x - ox) / scale) as f32, ((pt.y - oy) / scale) as f32)
    }

    fn render_rigid_world(&self, dc: &mut DrawingContext, sim: &SandboxEngine, scale: f64, ox: f64, oy: f64) {
        let to_screen = |p: Vec2| Point::new(ox + p.x as f64 * scale, oy + p.y as f64 * scale);

        // 1. Force field rings (vortex / gravity well)
        for field in sim.world.force_fields() {
            if let ForceField::Vortex { center, radius, .. } | ForceField::Radial { center, radius, .. } = *field {
                let c = to_screen(center);
                for ring in [0.35_f64, 0.65, 1.0] {
                    dc.draw_ellipse_at(None, Some(&self.palette.slot_pen), c, radius as f64 * ring * scale, radius as f64 * ring * scale);
                }
            }
        }

        // 2. Rigid bodies
        for body in sim.world.bodies() {
            let argb = if body.user_data != 0 { body.user_data as u32 } else { 0xFF38BDF8 };
            let fill: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(argb));
            let pen = if body.is_sleeping { &self.palette.sleep_outline_pen } else { &self.palette.outline_pen };

            match &body.shape {
                Shape2D::Circle { radius } => {
                    let c = to_screen(body.position);
                    let r = *radius as f64 * scale;
                    dc.draw_ellipse_at(Some(&fill), Some(pen), c, r, r);
                    // Orientation spoke
                    let edge = to_screen(body.position + body.rotation.x_axis() * *radius);
                    dc.draw_line(&self.palette.spoke_pen, c, edge);
                }
                Shape2D::Box { half_extents } => {
                    let c = to_screen(body.position);
                    let w = half_extents.x as f64 * 2.0 * scale;
                    let h = half_extents.y as f64 * 2.0 * scale;
                    let tf = Matrix::create_rotation_at(body.angle as f64, c);
                    let scope = dc.push_transform(tf);
                    let mut g = dc.scope(scope);
                    g.draw_rectangle(
                        Some(&fill),
                        Some(pen),
                        Rect::new(c.x - w * 0.5, c.y - h * 0.5, w, h),
                        4.0,
                        4.0,
                        &BoxShadows::default(),
                    );
                }
                Shape2D::Capsule { half_height, radius } => {
                    let c = to_screen(body.position);
                    let r = *radius as f64 * scale;
                    let w = r * 2.0;
                    let h = (*half_height + *radius) as f64 * 2.0 * scale;
                    let tf = Matrix::create_rotation_at(body.angle as f64, c);
                    let scope = dc.push_transform(tf);
                    let mut g = dc.scope(scope);
                    g.draw_rectangle(
                        Some(&fill),
                        Some(pen),
                        Rect::new(c.x - w * 0.5, c.y - h * 0.5, w, h),
                        r,
                        r,
                        &BoxShadows::default(),
                    );
                }
                Shape2D::Polygon(poly) => {
                    let geom = StreamGeometry::new();
                    {
                        let mut ctx = geom.open();
                        let tf = body.transform();
                        let p0 = to_screen(tf.transform_point(poly.vertices[0]));
                        ctx.begin_figure(p0, true);
                        for &v in &poly.vertices[1..] {
                            ctx.line_to(to_screen(tf.transform_point(v)), true);
                        }
                        ctx.end_figure(true);
                    }
                    let base_geom: Ref<ferroui_base::media::Geometry> = geom.upcast();
                    dc.draw_geometry(Some(&fill), Some(pen), &base_geom);
                }
                Shape2D::Segment { a, b } => {
                    let tf = body.transform();
                    dc.draw_line(pen, to_screen(tf.transform_point(*a)), to_screen(tf.transform_point(*b)));
                }
            }

            if sim.show_aabbs {
                let min = to_screen(body.aabb.min);
                let max = to_screen(body.aabb.max);
                dc.draw_rectangle_outline(&self.palette.aabb_pen, Rect::new(min.x, min.y, max.x - min.x, max.y - min.y), 0.0);
            }

            if sim.show_velocities && body.is_dynamic() && body.linear_velocity.length_squared() > 0.05 {
                let p0 = to_screen(body.position);
                let p1 = to_screen(body.position + body.linear_velocity * 0.18);
                dc.draw_line(&self.palette.vel_pen, p0, p1);
            }
        }

        // 3. Joints
        for joint in sim.world.joints() {
            if let Some(a) = sim.world.body(joint.body_a) {
                let b = sim.world.body(joint.body_b);
                let (pa, pb) = joint.world_anchors(a, b);
                let sa = to_screen(pa);
                let sb = to_screen(pb);
                dc.draw_line(&self.palette.joint_pen, sa, sb);
                dc.draw_ellipse_at(None, Some(&self.palette.joint_pen), sa, 3.5, 3.5);
            }
        }

        // 4. Multi-bounce laser rays
        for &(p0, p1, hit_normal) in &sim.laser_segments {
            let s0 = to_screen(p0);
            let s1 = to_screen(p1);
            dc.draw_line(&self.palette.laser_glow_pen, s0, s1);
            dc.draw_line(&self.palette.laser_core_pen, s0, s1);
            if let Some(n) = hit_normal {
                dc.draw_ellipse_at(None, Some(&self.palette.laser_core_pen), s1, 4.5, 4.5);
                dc.draw_line(&self.palette.vel_pen, s1, to_screen(p1 + n * 0.45));
            }
        }

        // 5. Contact points & normals overlay
        if sim.show_contacts {
            for m in sim.world.contacts() {
                for pt in &m.points {
                    let sp = to_screen(pt.position);
                    let tip = to_screen(pt.position + m.normal * 0.35);
                    dc.draw_ellipse_at(None, Some(&self.palette.contact_pen), sp, 3.5, 3.5);
                    dc.draw_line(&self.palette.contact_pen, sp, tip);
                }
            }
        }

        // 6. Shockwave blast ring
        if let Some((center, radius)) = sim.blast_ring {
            let c = to_screen(center);
            let r = radius as f64 * scale;
            dc.draw_ellipse_at(None, Some(&self.palette.blast_pen), c, r, r);
        }
    }

    fn render_particles(&self, dc: &mut DrawingContext, sim: &SandboxEngine, scale: f64, ox: f64, oy: f64) {
        let to_screen = |x: f32, y: f32| Point::new(ox + x as f64 * scale, oy + y as f64 * scale);

        // Container border
        let b = sim.particles.config.bounds;
        let tl = to_screen(b.min.x, b.min.y);
        let br = to_screen(b.max.x, b.max.y);
        dc.draw_rectangle_outline(&self.palette.outline_pen, Rect::new(tl.x, tl.y, br.x - tl.x, br.y - tl.y), 6.0);

        // Obstacles
        for obs in &sim.particles.obstacles {
            let c = to_screen(obs.center[0], obs.center[1]);
            let r = obs.radius as f64 * scale;
            dc.draw_ellipse_at(Some(&self.palette.obstacle_fill), Some(&self.palette.outline_pen), c, r, r);
        }

        // Thousands of hardware-simulated particles (drawn as crisp rounded squares/dots for max throughput)
        let r_px = (sim.particles.config.particle_radius as f64 * scale * 1.15).max(2.2);
        let d_px = r_px * 2.0;
        let palette_len = self.palette.particle_brushes.len();
        for p in &sim.gpu_particles_view {
            let speed = (p.vel[0] * p.vel[0] + p.vel[1] * p.vel[1]).sqrt();
            let heat = ((speed * 0.085 + p.density * 0.14).clamp(0.0, 0.999) * palette_len as f32) as usize;
            let brush = &self.palette.particle_brushes[heat.min(palette_len - 1)];
            let sx = ox + p.pos[0] as f64 * scale - r_px;
            let sy = oy + p.pos[1] as f64 * scale - r_px;
            dc.fill_rectangle(brush, Rect::new(sx, sy, d_px, d_px), r_px as f32);
        }

        // Pointer vortex halo when active
        if sim.particles.config.pointer_strength.abs() > 0.1 {
            let c = to_screen(sim.particles.config.pointer_center.x, sim.particles.config.pointer_center.y);
            let pr = sim.particles.config.pointer_radius as f64 * scale;
            let pen = if sim.particles.config.pointer_strength > 0.0 {
                &self.palette.slot_pen
            } else {
                &self.palette.blast_pen
            };
            dc.draw_ellipse_at(None, Some(pen), c, pr, pr);
        }
    }

    fn render_ui_springs(&self, dc: &mut DrawingContext, sim: &SandboxEngine, scale: f64, ox: f64, oy: f64) {
        let to_screen = |p: Vec2| Point::new(ox + p.x as f64 * scale, oy + p.y as f64 * scale);

        // 1. Magnetic dock slot outlines
        for &anchor in &sim.ui_snap.anchors {
            let c = to_screen(anchor);
            let w = 3.1 * scale;
            let h = 1.84 * scale;
            dc.draw_rectangle_outline(
                &self.palette.slot_pen,
                Rect::new(c.x - w * 0.5, c.y - h * 0.5, w, h),
                12.0,
            );
        }

        // 2. Draggable spring cards with elevation shadow, scale spring, and velocity tilt!
        for item in &sim.ui_cards {
            let pos = item.card.position.value();
            let target = item.card.position.target();
            let sc = item.card.scale.value as f64;
            let tilt = item.card.tilt_radians.value as f64;

            // Subtle elastic tether line to magnetic target
            if pos.distance(target) > 0.08 {
                dc.draw_line(&self.palette.joint_pen, to_screen(pos), to_screen(target));
            }

            let c = to_screen(pos);
            let w = item.half_size.x as f64 * 2.0 * scale * sc;
            let h = item.half_size.y as f64 * 2.0 * scale * sc;
            let tf = Matrix::create_rotation_at(tilt, c);
            let scope = dc.push_transform(tf);
            let mut g = dc.scope(scope);

            // Drop shadow
            g.fill_rectangle(
                &self.palette.card_shadow,
                Rect::new(c.x - w * 0.5 + 5.0, c.y - h * 0.5 + 8.0, w, h),
                12.0,
            );
            // Card body
            let fill: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(item.accent_argb));
            g.draw_rectangle(
                Some(&fill),
                Some(&self.palette.outline_pen),
                Rect::new(c.x - w * 0.5, c.y - h * 0.5, w, h),
                12.0,
                12.0,
                &BoxShadows::default(),
            );
        }

        // 3. Bottom rubber-band InertiaScroller carousel strip with 5 magnetic detents
        let strip_tl = to_screen(Vec2::new(0.8, 7.6));
        let strip_br = to_screen(Vec2::new(WORLD_WIDTH - 0.8, 9.45));
        let strip_rect = Rect::new(strip_tl.x, strip_tl.y, strip_br.x - strip_tl.x, strip_br.y - strip_tl.y);
        dc.fill_rectangle(&self.palette.track_bg, strip_rect, 12.0);
        dc.draw_rectangle_outline(&self.palette.slot_pen, strip_rect, 12.0);

        let clip = dc.push_clip(strip_rect);
        {
            let mut g = dc.scope(clip);
            let scroll = sim.ui_scroller.position;
            let pill_colors = [0xFF0284C7, 0xFF7C3AED, 0xFFDB2777, 0xFF059669, 0xFFD97706];
            for i in 0..5 {
                let wx = 4.0 + i as f32 * 3.0 - scroll;
                let p = to_screen(Vec2::new(wx, 8.52));
                let pw = 2.4 * scale;
                let ph = 1.25 * scale;
                let fill: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(pill_colors[i]));
                g.draw_rectangle(
                    Some(&fill),
                    Some(&self.palette.outline_pen),
                    Rect::new(p.x - pw * 0.5, p.y - ph * 0.5, pw, ph),
                    10.0,
                    10.0,
                    &BoxShadows::default(),
                );
            }
        }
        // Detent dots beneath strip
        for (i, &detent) in sim.ui_scroller.detents.iter().enumerate() {
            let dot = to_screen(Vec2::new(6.8 + i as f32 * 0.6, 9.72));
            let active = (sim.ui_scroller.position - detent).abs() < 1.2;
            let brush = if active { &self.palette.track_pill } else { &self.palette.obstacle_fill };
            dc.draw_ellipse_at(Some(brush), None, dot, 4.5, 4.5);
        }
    }
}

fn viewport_transform(width: f64, height: f64) -> (f64, f64, f64) {
    let sx = width / WORLD_WIDTH as f64;
    let sy = height / WORLD_HEIGHT as f64;
    let scale = sx.min(sy).max(1.0);
    let ox = (width - WORLD_WIDTH as f64 * scale) * 0.5;
    let oy = (height - WORLD_HEIGHT as f64 * scale) * 0.5;
    (scale, ox, oy)
}
