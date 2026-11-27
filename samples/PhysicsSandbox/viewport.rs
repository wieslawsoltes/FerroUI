//! Studio-grade hardware-rendered FerroUI `PhysicsViewport` control featuring:
//! - Major/minor studio coordinate grid with meter ruler ticks
//! - Specular bevel highlights, realistic zig-zag spring coils, animated buoyant water waves
//! - XPBD stress-colored tearable cloth meshes and glowing pressure-inflated jelly soft bodies
//! - Character controller visor + slope & ground-normal telemetry
//! - Floating translucent in-viewport HUD card with live 90-frame performance sparkline

use crate::scenes::{SandboxEngine, SceneKind, ARENA_WALL_TAG, WORLD_HEIGHT, WORLD_WIDTH};
use ferroui_base::input::{InputElementImpl, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::{ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::{
    BoxShadows, DrawingContext, FlowDirection, FormattedText, IBrush, IPen, StreamGeometry, Typeface,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::utilities::locale;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Matrix, Point, Rect, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use ferroui_physics::prelude::*;
use ferroui_view::ViewClass;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{Duration, Instant};

struct Palette {
    bg: Rc<dyn IBrush>,
    minor_grid_pen: Rc<dyn IPen>,
    major_grid_pen: Rc<dyn IPen>,
    arena_border_pen: Rc<dyn IPen>,
    outline_pen: Rc<dyn IPen>,
    sleep_outline_pen: Rc<dyn IPen>,
    highlight_pen: Rc<dyn IPen>,
    joint_pen: Rc<dyn IPen>,
    spring_pen: Rc<dyn IPen>,
    aabb_pen: Rc<dyn IPen>,
    contact_pen: Rc<dyn IPen>,
    vel_pen: Rc<dyn IPen>,
    laser_glow_pen: Rc<dyn IPen>,
    laser_core_pen: Rc<dyn IPen>,
    blast_pen: Rc<dyn IPen>,
    slot_pen: Rc<dyn IPen>,
    tether_pen: Rc<dyn IPen>,
    water_fill: Rc<dyn IBrush>,
    water_crest_pen: Rc<dyn IPen>,
    card_shadow: Rc<dyn IBrush>,
    track_bg: Rc<dyn IBrush>,
    track_pill: Rc<dyn IBrush>,
    obstacle_fill: Rc<dyn IBrush>,
    spoke_pen: Rc<dyn IPen>,
    hud_bg: Rc<dyn IBrush>,
    hud_border: Rc<dyn IPen>,
    hud_text: Rc<dyn IBrush>,
    hud_muted: Rc<dyn IBrush>,
    hud_accent: Rc<dyn IBrush>,
    sparkline_pen: Rc<dyn IPen>,
    sparkline_budget_pen: Rc<dyn IPen>,
    cloth_pens: Vec<Rc<dyn IPen>>,
    particle_brushes: Vec<Rc<dyn IBrush>>,
    char_fill: Rc<dyn IBrush>,
    char_visor: Rc<dyn IBrush>,
    solar_corona: Rc<dyn IBrush>,
}

impl Palette {
    fn new() -> Self {
        let particle_colors = [
            0xFF0284C7, 0xFF0EA5E9, 0xFF22D3EE, 0xFF38BDF8, 0xFF60A5FA, 0xFF818CF8, 0xFFA78BFA, 0xFFC084FC,
            0xFFE879F9, 0xFFF472B6, 0xFFFB7185, 0xFFF97316, 0xFFFB923C, 0xFFFBBF24, 0xFFFDE047, 0xFF34D399,
        ];
        let cloth_colors = [0xFF38BDF8, 0xFF60A5FA, 0xFFA78BFA, 0xFFF59E0B, 0xFFEF4444];
        Self {
            bg: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF090D16)),
            minor_grid_pen: Rc::new(ImmutablePen::from_uint32(0xFF101726, 1.0)),
            major_grid_pen: Rc::new(ImmutablePen::from_uint32(0xFF192338, 1.0)),
            arena_border_pen: Rc::new(ImmutablePen::from_uint32(0xFF2B3B58, 1.5)),
            outline_pen: Rc::new(ImmutablePen::from_uint32(0xFFE2E8F0, 1.5)),
            sleep_outline_pen: Rc::new(ImmutablePen::from_uint32(0xFF64748B, 1.2)),
            highlight_pen: Rc::new(ImmutablePen::from_uint32(0x55FFFFFF, 1.4)),
            joint_pen: Rc::new(ImmutablePen::from_uint32(0xFFFDE047, 2.0)),
            spring_pen: Rc::new(ImmutablePen::from_uint32(0xFF38BDF8, 1.8)),
            aabb_pen: Rc::new(ImmutablePen::from_uint32(0x9938BDF8, 1.0)),
            contact_pen: Rc::new(ImmutablePen::from_uint32(0xFFFB7185, 2.0)),
            vel_pen: Rc::new(ImmutablePen::from_uint32(0xFF22D3EE, 1.6)),
            laser_glow_pen: Rc::new(ImmutablePen::from_uint32(0x66EF4444, 7.0)),
            laser_core_pen: Rc::new(ImmutablePen::from_uint32(0xFFFFF1F2, 2.0)),
            blast_pen: Rc::new(ImmutablePen::from_uint32(0xCCF59E0B, 3.0)),
            slot_pen: Rc::new(ImmutablePen::from_uint32(0x7738BDF8, 1.8)),
            tether_pen: Rc::new(ImmutablePen::from_uint32(0xCC34D399, 1.8)),
            water_fill: Rc::new(ImmutableSolidColorBrush::from_uint32(0x440EA5E9)),
            water_crest_pen: Rc::new(ImmutablePen::from_uint32(0xCC38BDF8, 2.2)),
            card_shadow: Rc::new(ImmutableSolidColorBrush::from_uint32(0x66000000)),
            track_bg: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF111827)),
            track_pill: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF38BDF8)),
            obstacle_fill: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF1E293B)),
            spoke_pen: Rc::new(ImmutablePen::from_uint32(0xCC090D16, 2.0)),
            hud_bg: Rc::new(ImmutableSolidColorBrush::from_uint32(0xCC0D1322)),
            hud_border: Rc::new(ImmutablePen::from_uint32(0xFF26334D, 1.0)),
            hud_text: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFFF1F5F9)),
            hud_muted: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF94A3B8)),
            hud_accent: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF38BDF8)),
            sparkline_pen: Rc::new(ImmutablePen::from_uint32(0xFF34D399, 1.8)),
            sparkline_budget_pen: Rc::new(ImmutablePen::from_uint32(0x55F59E0B, 1.0)),
            cloth_pens: cloth_colors
                .into_iter()
                .map(|c| Rc::new(ImmutablePen::from_uint32(c, 1.7)) as Rc<dyn IPen>)
                .collect(),
            particle_brushes: particle_colors
                .into_iter()
                .map(|c| Rc::new(ImmutableSolidColorBrush::from_uint32(c)) as Rc<dyn IBrush>)
                .collect(),
            char_fill: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFF38BDF8)),
            char_visor: Rc::new(ImmutableSolidColorBrush::from_uint32(0xFFFDE047)),
            solar_corona: Rc::new(ImmutableSolidColorBrush::from_uint32(0x28FBBF24)),
        }
    }
}

#[repr(C)]
pub struct PhysicsViewport {
    base: Control,
    engine: RefCell<Rc<RefCell<SandboxEngine>>>,
    on_tick: RefCell<Option<Rc<dyn Fn(&SandboxEngine)>>>,
    palette: Palette,
    brush_cache: RefCell<HashMap<u32, Rc<dyn IBrush>>>,
    trail_pen_cache: RefCell<HashMap<u32, Rc<dyn IPen>>>,
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
        let input: Ref<ferroui_base::input::InputElement> = this.to_ref().upcast();
        e.pointer().capture(Some(&input));
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

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        e.pointer().capture(None);
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

        // Strictly clip all viewport rendering to this control's layout bounds so nothing ever
        // overlaps sibling panels (left tool rail, top header, right inspector, or status bar).
        let clip_state = dc.push_clip(Rect::new(0.0, 0.0, w, h));
        let mut dc = dc.scope(clip_state);

        // Studio dark canvas background
        dc.fill_rectangle(&this.palette.bg, Rect::new(0.0, 0.0, w, h), 0.0);

        // Minor 0.5m and Major 1.0m coordinate studio grid
        let arena_w = WORLD_WIDTH as f64 * scale;
        let arena_h = WORLD_HEIGHT as f64 * scale;
        for step in 0..=((WORLD_WIDTH * 2.0) as i32) {
            let sx = ox + (step as f64 * 0.5) * scale;
            let pen = if step % 2 == 0 { &this.palette.major_grid_pen } else { &this.palette.minor_grid_pen };
            dc.draw_line(pen, Point::new(sx, oy), Point::new(sx, oy + arena_h));
        }
        for step in 0..=((WORLD_HEIGHT * 2.0) as i32) {
            let sy = oy + (step as f64 * 0.5) * scale;
            let pen = if step % 2 == 0 { &this.palette.major_grid_pen } else { &this.palette.minor_grid_pen };
            dc.draw_line(pen, Point::new(ox, sy), Point::new(ox + arena_w, sy));
        }
        dc.draw_rectangle_outline(&this.palette.arena_border_pen, Rect::new(ox, oy, arena_w, arena_h), 4.0);

        // Subtle ruler meter ticks along top edge (zero per-frame String allocations)
        const RULER_TICKS: [(i32, &str); 7] = [
            (2, "2m"),
            (4, "4m"),
            (6, "6m"),
            (8, "8m"),
            (10, "10m"),
            (12, "12m"),
            (14, "14m"),
        ];
        for (gx, lbl) in RULER_TICKS {
            let sx = ox + gx as f64 * scale;
            draw_label(&mut dc, lbl, Point::new(sx + 3.0, oy + 3.0), 9.5, &this.palette.hud_muted);
        }

        let engine = this.engine.borrow();
        let sim = engine.borrow();

        // Clip all world scene content cleanly inside the 16x10m arena border so gravity rings,
        // shockwaves, and laser trails never bleed outside the studio grid!
        {
            let arena_clip = dc.push_clip(Rect::new(ox, oy, arena_w, arena_h));
            let mut adc = dc.scope(arena_clip);
            match sim.scene {
                SceneKind::GpuParticles => this.render_particles(&mut adc, &sim, scale, ox, oy),
                SceneKind::UiSprings => this.render_ui_springs(&mut adc, &sim, scale, ox, oy),
                _ => {
                    this.render_rigid_world(&mut adc, &sim, scale, ox, oy);
                    if sim.cloth.is_some() || !sim.soft_bodies.is_empty() {
                        this.render_cloth_and_softbodies(&mut adc, &sim, scale, ox, oy);
                    }
                    if sim.character.is_some() {
                        this.render_character(&mut adc, &sim, scale, ox, oy);
                    }
                }
            }
        }

        // Floating Pro Studio telemetry overlay + real-time frame-time sparkline
        if sim.show_hud_graph {
            this.render_hud_overlay(&mut dc, &sim, ox + 14.0, oy + 20.0);
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
            brush_cache: RefCell::new(HashMap::with_capacity(32)),
            trail_pen_cache: RefCell::new(HashMap::with_capacity(16)),
        }
    }

    #[inline]
    fn cached_brush(&self, argb: u32) -> Rc<dyn IBrush> {
        let mut cache = self.brush_cache.borrow_mut();
        cache
            .entry(argb)
            .or_insert_with(|| Rc::new(ImmutableSolidColorBrush::from_uint32(argb)) as Rc<dyn IBrush>)
            .clone()
    }

    #[inline]
    fn cached_trail_pen(&self, argb: u32) -> Rc<dyn IPen> {
        let key = (argb & 0x00FF_FFFF) | 0x6600_0000;
        let mut cache = self.trail_pen_cache.borrow_mut();
        cache
            .entry(key)
            .or_insert_with(|| Rc::new(ImmutablePen::from_uint32(key, 1.8)) as Rc<dyn IPen>)
            .clone()
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.set_clip_to_bounds(true);

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

    fn render_hud_overlay(&self, dc: &mut DrawingContext, sim: &SandboxEngine, x: f64, y: f64) {
        let card_w = 248.0;
        let card_h = 84.0;
        let rect = Rect::new(x, y, card_w, card_h);
        dc.fill_rectangle(&self.palette.hud_bg, rect, 8.0);
        dc.draw_rectangle_outline(&self.palette.hud_border, rect, 8.0);

        let title = format!("{}  ·  {:.0} FPS", sim.scene.title(), sim.telemetry.fps);
        draw_label(dc, &title, Point::new(x + 10.0, y + 7.0), 12.0, &self.palette.hud_text);
        let sub = format!("{}  ·  {:.2} ms", sim.telemetry.backend_label, sim.telemetry.step_ms);
        draw_label(dc, &sub, Point::new(x + 10.0, y + 23.0), 10.0, &self.palette.hud_accent);

        // 90-frame step-time sparkline
        let gx = x + 10.0;
        let gy = y + 42.0;
        let gw = card_w - 20.0;
        let gh = 32.0;
        // 4ms reference budget line
        let max_ms = 6.0_f32;
        let budget_y = gy + gh - (2.0 / max_ms) as f64 * gh;
        dc.draw_line(&self.palette.sparkline_budget_pen, Point::new(gx, budget_y), Point::new(gx + gw, budget_y));

        let n = sim.step_ms_history.len();
        if n >= 2 {
            let step_x = gw / (90.0 - 1.0);
            let mut prev = Point::new(
                gx,
                gy + gh - (sim.step_ms_history[0].min(max_ms) / max_ms) as f64 * gh,
            );
            for (i, &ms) in sim.step_ms_history.iter().enumerate().skip(1) {
                let pt = Point::new(
                    gx + i as f64 * step_x,
                    gy + gh - (ms.min(max_ms) / max_ms) as f64 * gh,
                );
                dc.draw_line(&self.palette.sparkline_pen, prev, pt);
                prev = pt;
            }
        }
    }

    fn render_rigid_world(&self, dc: &mut DrawingContext, sim: &SandboxEngine, scale: f64, ox: f64, oy: f64) {
        let to_screen = |p: Vec2| Point::new(ox + p.x as f64 * scale, oy + p.y as f64 * scale);

        // 1. Force fields (gravity well rings & animated water buoyancy zones)
        for field in sim.world.force_fields() {
            match *field {
                ForceField::Vortex { center, radius, .. } | ForceField::Radial { center, radius, .. } => {
                    let c = to_screen(center);
                    for ring in [0.35_f64, 0.62, 0.85, 1.0] {
                        dc.draw_ellipse_at(
                            None,
                            Some(&self.palette.slot_pen),
                            c,
                            radius as f64 * ring * scale,
                            radius as f64 * ring * scale,
                        );
                    }
                }
                ForceField::NewtonianGravity { center, .. } => {
                    let c = to_screen(center);
                    // Crisp Keplerian orbital reference rings matching the 5 planetary orbits + soft solar corona
                    dc.draw_ellipse_at(Some(&self.palette.solar_corona), None, c, 1.05 * scale, 1.05 * scale);
                    for r in [1.45_f64, 2.25, 3.05, 3.85, 4.55] {
                        dc.draw_ellipse_at(None, Some(&self.palette.major_grid_pen), c, r * scale, r * scale);
                    }
                }
                ForceField::Buoyancy { region, .. } => {
                    let tl = to_screen(region.min);
                    let br = to_screen(region.max);
                    dc.fill_rectangle(
                        &self.palette.water_fill,
                        Rect::new(tl.x, tl.y, br.x - tl.x, br.y - tl.y),
                        4.0,
                    );
                    // Animated shimmering sine-wave water crest
                    let segments = 18;
                    let mut prev = Point::new(tl.x, tl.y);
                    for i in 1..=segments {
                        let frac = i as f64 / segments as f64;
                        let wx = tl.x + (br.x - tl.x) * frac;
                        let wave = (frac * 12.0 + sim.sim_time as f64 * 4.2).sin() * 3.5;
                        let cur = Point::new(wx, tl.y + wave);
                        dc.draw_line(&self.palette.water_crest_pen, prev, cur);
                        prev = cur;
                    }
                }
                ForceField::Wind { .. } => {}
            }
        }

        // 1b. Keplerian orbital trails in Ray & Orbit scene
        for (_, trail, color) in &sim.orbit_trails {
            if trail.len() < 2 {
                continue;
            }
            let trail_pen = self.cached_trail_pen(*color);
            let mut prev = to_screen(trail[0]);
            for &pt in trail.iter().skip(1) {
                let cur = to_screen(pt);
                dc.draw_line(&trail_pen, prev, cur);
                prev = cur;
            }
        }

        // 2. Rigid bodies
        for body in sim.world.bodies() {
            if body.user_data == ARENA_WALL_TAG {
                continue;
            }
            let argb = if body.user_data != 0 { body.user_data as u32 } else { 0xFF38BDF8 };
            let fill = self.cached_brush(argb);
            let pen = if body.is_sleeping { &self.palette.sleep_outline_pen } else { &self.palette.outline_pen };

            match &body.shape {
                Shape2D::Circle { radius } => {
                    let c = to_screen(body.position);
                    let r = *radius as f64 * scale;
                    dc.draw_ellipse_at(Some(&fill), Some(pen), c, r, r);
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
                    if w > 12.0 && h > 10.0 {
                        g.draw_line(
                            &self.palette.highlight_pen,
                            Point::new(c.x - w * 0.42, c.y - h * 0.42),
                            Point::new(c.x + w * 0.42, c.y - h * 0.42),
                        );
                    }
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

        // 3. Joints (with realistic zig-zag spring coils for elastic joints!)
        if sim.show_joints {
            for joint in sim.world.joints() {
                if let Some(a) = sim.world.body(joint.body_a) {
                    let b = sim.world.body(joint.body_b);
                    let (pa, pb) = joint.world_anchors(a, b);
                    let sa = to_screen(pa);
                    let sb = to_screen(pb);
                    let is_spring = match joint.kind {
                        JointKind::Wheel { .. } | JointKind::Target { .. } => true,
                        JointKind::Distance { frequency_hz, .. } => frequency_hz > 0.0,
                        _ => false,
                    };
                    if is_spring {
                        draw_spring_coil(dc, &self.palette.spring_pen, sa, sb, 7, 5.5);
                    } else {
                        dc.draw_line(&self.palette.joint_pen, sa, sb);
                    }
                    dc.draw_ellipse_at(None, Some(&self.palette.joint_pen), sa, 3.2, 3.2);
                }
            }
        }

        // 4. Multi-bounce laser rays, turret mount & closest_point proximity tether
        if !sim.laser_segments.is_empty() {
            let emitter = to_screen(sim.laser_origin);
            dc.draw_ellipse_at(Some(&self.palette.obstacle_fill), Some(&self.palette.laser_core_pen), emitter, 7.0, 7.0);
        }
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
        if let Some((from, to, dist)) = sim.closest_tether {
            let sf = to_screen(from);
            let st = to_screen(to);
            dc.draw_line(&self.palette.tether_pen, sf, st);
            dc.draw_ellipse_at(None, Some(&self.palette.tether_pen), st, 4.0, 4.0);
            draw_label(
                dc,
                &format!("{dist:.2}m"),
                Point::new((sf.x + st.x) * 0.5 + 4.0, (sf.y + st.y) * 0.5 - 12.0),
                10.0,
                &self.palette.hud_accent,
            );
        }

        // 5. Contact points & normals
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

    fn render_cloth_and_softbodies(
        &self,
        dc: &mut DrawingContext,
        sim: &SandboxEngine,
        scale: f64,
        ox: f64,
        oy: f64,
    ) {
        let to_screen = |p: Vec2| Point::new(ox + p.x as f64 * scale, oy + p.y as f64 * scale);

        // XPBD Tearable Cloth with stress heatmap coloring (cyan -> purple -> amber -> red!)
        if let Some(cloth) = sim.cloth.as_ref() {
            let n_pens = self.palette.cloth_pens.len();
            for c in &cloth.constraints {
                if c.broken {
                    continue;
                }
                // Only draw structural links for crisp uncluttered lattice visuals
                let a = cloth.nodes[c.a].pos;
                let b = cloth.nodes[c.b].pos;
                let stress = ((c.stretch_ratio - 1.0) / 0.85).clamp(0.0, 0.999);
                let pen = &self.palette.cloth_pens[(stress * n_pens as f32) as usize];
                dc.draw_line(pen, to_screen(a), to_screen(b));
            }
            for node in &cloth.nodes {
                if node.inv_mass <= 0.0 {
                    dc.draw_ellipse_at(
                        Some(&self.palette.hud_accent),
                        Some(&self.palette.outline_pen),
                        to_screen(node.pos),
                        4.5,
                        4.5,
                    );
                }
            }
        }

        // XPBD Pressure-Inflated Jelly Soft Bodies
        for sb in &sim.soft_bodies {
            if sb.nodes.len() < 3 {
                continue;
            }
            let geom = StreamGeometry::new();
            {
                let mut ctx = geom.open();
                ctx.begin_figure(to_screen(sb.nodes[0].pos), true);
                for n in &sb.nodes[1..] {
                    ctx.line_to(to_screen(n.pos), true);
                }
                ctx.end_figure(true);
            }
            let fill = self.cached_brush(sb.color_argb);
            let base_geom: Ref<ferroui_base::media::Geometry> = geom.upcast();
            dc.draw_geometry(Some(&fill), Some(&self.palette.outline_pen), &base_geom);

            // Inner glowing pressure core
            let c = to_screen(sb.center());
            dc.draw_ellipse_at(None, Some(&self.palette.highlight_pen), c, 10.0, 10.0);
        }
    }

    fn render_character(&self, dc: &mut DrawingContext, sim: &SandboxEngine, scale: f64, ox: f64, oy: f64) {
        let Some(ctrl) = sim.character.as_ref() else { return };
        let to_screen = |p: Vec2| Point::new(ox + p.x as f64 * scale, oy + p.y as f64 * scale);

        if let Shape2D::Capsule { half_height, radius } = ctrl.shape {
            let c = to_screen(ctrl.position);
            let r = radius as f64 * scale;
            let w = r * 2.0;
            let h = (half_height + radius) as f64 * 2.0 * scale;
            dc.draw_rectangle(
                Some(&self.palette.char_fill),
                Some(&self.palette.outline_pen),
                Rect::new(c.x - w * 0.5, c.y - h * 0.5, w, h),
                r,
                r,
                &BoxShadows::default(),
            );
            // Expressive directional visor
            let face = if ctrl.velocity.x >= -0.05 { 1.0 } else { -1.0 };
            dc.fill_rectangle(
                &self.palette.char_visor,
                Rect::new(c.x + face * r * 0.25 - 6.0, c.y - h * 0.26, 12.0, 6.0),
                3.0,
            );
            // Ground normal vector indicator
            if ctrl.last_result.is_grounded {
                let foot = to_screen(ctrl.position + Vec2::new(0.0, half_height + radius));
                let ntip = to_screen(
                    ctrl.position + Vec2::new(0.0, half_height + radius) + ctrl.last_result.ground_normal * 0.75,
                );
                dc.draw_line(&self.palette.tether_pen, foot, ntip);
                dc.draw_ellipse_at(None, Some(&self.palette.tether_pen), foot, 5.0, 5.0);
            }
        }
    }

    fn render_particles(&self, dc: &mut DrawingContext, sim: &SandboxEngine, scale: f64, ox: f64, oy: f64) {
        let to_screen = |x: f32, y: f32| Point::new(ox + x as f64 * scale, oy + y as f64 * scale);

        let b = sim.particles.config.bounds;
        let tl = to_screen(b.min.x, b.min.y);
        let br = to_screen(b.max.x, b.max.y);
        dc.draw_rectangle_outline(&self.palette.arena_border_pen, Rect::new(tl.x, tl.y, br.x - tl.x, br.y - tl.y), 6.0);

        for obs in &sim.particles.obstacles {
            let c = to_screen(obs.center[0], obs.center[1]);
            let r = obs.radius as f64 * scale;
            dc.draw_ellipse_at(Some(&self.palette.obstacle_fill), Some(&self.palette.outline_pen), c, r, r);
        }

        let r_px = (sim.particles.config.particle_radius as f64 * scale * 1.12).max(2.0);
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

        // 1. Magnetic dock slots (with live velocity-projected landing highlight!)
        for (i, &anchor) in sim.ui_snap.anchors.iter().enumerate() {
            let c = to_screen(anchor);
            let w = 2.96 * scale;
            let h = 1.76 * scale;
            let is_projected = sim.ui_projected_slot == Some(i);
            let pen = if is_projected { &self.palette.tether_pen } else { &self.palette.slot_pen };
            if is_projected {
                dc.fill_rectangle(
                    &self.palette.water_fill,
                    Rect::new(c.x - w * 0.5, c.y - h * 0.5, w, h),
                    12.0,
                );
            }
            dc.draw_rectangle_outline(pen, Rect::new(c.x - w * 0.5, c.y - h * 0.5, w, h), 12.0);
            let lbl = if is_projected { "LAND HERE" } else { "DOCK SLOT" };
            draw_label(dc, lbl, Point::new(c.x - 28.0, c.y - 6.0), 9.5, &self.palette.hud_muted);
        }

        // 2. Live Analytical Spring Step-Response Oscilloscope (top-right inside UI Lab!)
        let osc_tl = to_screen(Vec2::new(11.75, 0.7));
        let osc_br = to_screen(Vec2::new(15.65, 3.65));
        let osc_rect = Rect::new(osc_tl.x, osc_tl.y, osc_br.x - osc_tl.x, osc_br.y - osc_tl.y);
        dc.fill_rectangle(&self.palette.hud_bg, osc_rect, 10.0);
        dc.draw_rectangle_outline(&self.palette.hud_border, osc_rect, 10.0);
        let cfg = sim.spring_preset.config();
        let peak = sim.ui_curve_samples.iter().copied().fold(1.0_f32, f32::max);
        let overshoot_pct = ((peak - 1.0) * 100.0).max(0.0);
        draw_label(
            dc,
            &format!("{}  ·  ζ={:.2}  ·  +{:.0}% overshoot", sim.spring_preset.label(), cfg.damping_ratio(), overshoot_pct),
            Point::new(osc_rect.x + 10.0, osc_rect.y + 8.0),
            10.5,
            &self.palette.hud_accent,
        );
        // Target=1.0 dashed reference line + analytical response curve
        let gx = osc_rect.x + 12.0;
        let gy = osc_rect.y + 34.0;
        let gw = (osc_rect.width - 24.0).max(10.0);
        let gh = (osc_rect.height - 46.0).max(10.0);
        let target_y = gy + gh * (1.0 - 1.0 / 1.55);
        dc.draw_line(&self.palette.sparkline_budget_pen, Point::new(gx, target_y), Point::new(gx + gw, target_y));
        if sim.ui_curve_samples.len() >= 2 {
            let n = sim.ui_curve_samples.len();
            let mut prev = Point::new(gx, gy + gh);
            for (i, &val) in sim.ui_curve_samples.iter().enumerate() {
                let px = gx + (i as f64 / (n - 1) as f64) * gw;
                let py = gy + gh * (1.0 - (val.clamp(0.0, 1.55) as f64 / 1.55));
                let cur = Point::new(px, py);
                dc.draw_line(&self.palette.sparkline_pen, prev, cur);
                prev = cur;
            }
        }

        // 3. Expandable Radial Spring FAB Menu (bottom-right above carousel)
        let fab_c = to_screen(sim.ui_fab_center);
        for (sp, lbl, col) in &sim.ui_fab_items {
            let p = sp.value();
            if p.distance(sim.ui_fab_center) > 0.15 {
                let sp_pt = to_screen(p);
                dc.draw_line(&self.palette.slot_pen, fab_c, sp_pt);
                let orb_fill = self.cached_brush(*col);
                dc.draw_ellipse_at(Some(&orb_fill), Some(&self.palette.outline_pen), sp_pt, 18.0, 18.0);
                draw_label(dc, lbl, Point::new(sp_pt.x - 11.0, sp_pt.y - 6.0), 10.0, &self.palette.hud_text);
            }
        }
        dc.draw_ellipse_at(Some(&self.palette.track_pill), Some(&self.palette.outline_pen), fab_c, 22.0, 22.0);
        draw_label(
            dc,
            if sim.ui_fab_open { "×" } else { "+" },
            Point::new(fab_c.x - 5.0, fab_c.y - 10.0),
            16.0,
            &self.palette.hud_text,
        );
        draw_label(dc, "Radial Spring FAB", Point::new(fab_c.x - 44.0, fab_c.y + 26.0), 9.5, &self.palette.hud_muted);

        // 4. Glowing 10-node SpringChain cursor ribbon
        for i in 1..sim.ui_ribbon.nodes.len() {
            let p0 = to_screen(sim.ui_ribbon.nodes[i - 1].value());
            let p1 = to_screen(sim.ui_ribbon.nodes[i].value());
            dc.draw_line(&self.palette.spring_pen, p0, p1);
            let r = (6.5 - i as f64 * 0.45).max(1.8);
            dc.draw_ellipse_at(Some(&self.palette.hud_accent), None, p1, r, r);
        }

        // 5. Draggable spring cards with titles, drop shadow, scale spring, and velocity tilt
        for item in &sim.ui_cards {
            let pos = item.card.position.value();
            let target = item.card.position.target();
            let sc = item.card.scale.value as f64;
            let tilt = item.card.tilt_radians.value as f64;

            if pos.distance(target) > 0.08 {
                draw_spring_coil(dc, &self.palette.joint_pen, to_screen(pos), to_screen(target), 6, 4.5);
            }

            let c = to_screen(pos);
            let w = item.half_size.x as f64 * 2.0 * scale * sc;
            let h = item.half_size.y as f64 * 2.0 * scale * sc;
            let tf = Matrix::create_rotation_at(tilt, c);
            let scope = dc.push_transform(tf);
            let mut g = dc.scope(scope);

            g.fill_rectangle(
                &self.palette.card_shadow,
                Rect::new(c.x - w * 0.5 + 6.0, c.y - h * 0.5 + 10.0, w, h),
                12.0,
            );
            let fill = self.cached_brush(item.accent_argb);
            g.draw_rectangle(
                Some(&fill),
                Some(&self.palette.outline_pen),
                Rect::new(c.x - w * 0.5, c.y - h * 0.5, w, h),
                12.0,
                12.0,
                &BoxShadows::default(),
            );
            g.draw_line(
                &self.palette.highlight_pen,
                Point::new(c.x - w * 0.42, c.y - h * 0.42),
                Point::new(c.x + w * 0.42, c.y - h * 0.42),
            );
            draw_label(&mut g, item.label, Point::new(c.x - w * 0.38, c.y - 16.0), 13.0, &self.palette.hud_text);
            draw_label(&mut g, item.subtitle, Point::new(c.x - w * 0.38, c.y + 4.0), 10.0, &self.palette.hud_text);
        }

        // 6. Rubber-band InertiaScroller carousel strip with live overscroll tension indicator
        let strip_tl = to_screen(Vec2::new(0.6, 7.55));
        let strip_br = to_screen(Vec2::new(WORLD_WIDTH - 0.6, 9.42));
        let strip_rect = Rect::new(strip_tl.x, strip_tl.y, strip_br.x - strip_tl.x, strip_br.y - strip_tl.y);
        let overscrolling = sim.ui_scroller.position < sim.ui_scroller.min_bound - 0.05
            || sim.ui_scroller.position > sim.ui_scroller.max_bound + 0.05;
        dc.fill_rectangle(&self.palette.track_bg, strip_rect, 12.0);
        dc.draw_rectangle_outline(
            if overscrolling { &self.palette.blast_pen } else { &self.palette.slot_pen },
            strip_rect,
            12.0,
        );

        let clip = dc.push_clip(strip_rect);
        {
            let mut g = dc.scope(clip);
            let scroll = sim.ui_scroller.position;
            let pill_colors = [0xFF0284C7, 0xFF7C3AED, 0xFFDB2777, 0xFF059669, 0xFFD97706];
            let pill_titles = ["Album 1", "Album 2", "Album 3", "Album 4", "Album 5"];
            for i in 0..5 {
                let wx = 4.0 + i as f32 * 3.0 - scroll;
                let p = to_screen(Vec2::new(wx, 8.48));
                let pw = 2.4 * scale;
                let ph = 1.25 * scale;
                let fill = self.cached_brush(pill_colors[i]);
                g.draw_rectangle(
                    Some(&fill),
                    Some(&self.palette.outline_pen),
                    Rect::new(p.x - pw * 0.5, p.y - ph * 0.5, pw, ph),
                    10.0,
                    10.0,
                    &BoxShadows::default(),
                );
                draw_label(&mut g, pill_titles[i], Point::new(p.x - 26.0, p.y - 8.0), 12.0, &self.palette.hud_text);
            }
        }
        for (i, &detent) in sim.ui_scroller.detents.iter().enumerate() {
            let dot = to_screen(Vec2::new(6.8 + i as f32 * 0.6, 9.72));
            let active = (sim.ui_scroller.position - detent).abs() < 1.2;
            let brush = if active { &self.palette.track_pill } else { &self.palette.obstacle_fill };
            dc.draw_ellipse_at(Some(brush), None, dot, 4.5, 4.5);
        }
    }
}

fn draw_label(dc: &mut DrawingContext, text: &str, origin: Point, size: f64, brush: &Rc<dyn IBrush>) {
    let ft = FormattedText::new(
        text,
        locale!("en-US"),
        FlowDirection::LeftToRight,
        Typeface::default(),
        size,
        Some(brush.clone()),
    );
    dc.draw_text(&ft, origin);
}

fn draw_spring_coil(dc: &mut DrawingContext, pen: &Rc<dyn IPen>, a: Point, b: Point, coils: usize, amp: f64) {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 8.0 {
        dc.draw_line(pen, a, b);
        return;
    }
    let ux = dx / len;
    let uy = dy / len;
    let px = -uy;
    let py = ux;

    let mut prev = a;
    let steps = coils * 2;
    for i in 1..steps {
        let t = i as f64 / steps as f64;
        let side = if i % 2 == 1 { amp } else { -amp };
        let cur = Point::new(a.x + dx * t + px * side, a.y + dy * t + py * side);
        dc.draw_line(pen, prev, cur);
        prev = cur;
    }
    dc.draw_line(pen, prev, b);
}

fn viewport_transform(width: f64, height: f64) -> (f64, f64, f64) {
    let pad = 16.0;
    let avail_w = (width - pad * 2.0).max(1.0);
    let avail_h = (height - pad * 2.0).max(1.0);
    let sx = avail_w / WORLD_WIDTH as f64;
    let sy = avail_h / WORLD_HEIGHT as f64;
    let scale = sx.min(sy).max(1.0);
    let ox = (width - WORLD_WIDTH as f64 * scale) * 0.5;
    let oy = (height - WORLD_HEIGHT as f64 * scale) * 0.5;
    (scale, ox, oy)
}
