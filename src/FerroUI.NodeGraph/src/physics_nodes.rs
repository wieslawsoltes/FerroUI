//! **Parametric Physics Component Library** (`feature = "physics"`) bridging
//! `ferroui-nodegraph` directly to `ferroui-physics`!
//!
//! Lets you visually design rigid-body structures, Keplerian orbital systems, suspension bridges,
//! motorized pendulums, force fields, tearable XPBD cloth, and pressure-inflated soft bodies
//! using Rhino Grasshopper-style DataTrees, sliders, and math expressions.

use crate::data_tree::Value;
use crate::graph::NodeGraph;
use crate::node::{EvalContext, FnComponent, NodeRegistry};
use crate::types::{NodeCategory, NodeKind, ParamAccess, ParamWidget, PortSpec, PortType};
use ferroui_physics::prelude::*;

/// Tag used on invisible outer arena walls so viewports can distinguish them from user geometry.
pub const GRAPH_ARENA_WALL_TAG: u64 = 0xFFFF_0001;

#[derive(Clone, Debug)]
pub struct BodySpec {
    pub position: Vec2,
    pub velocity: Vec2,
    pub angle: f32,
    pub angular_velocity: f32,
    pub shape: Shape2D,
    pub density: f32,
    pub restitution: f32,
    pub friction: f32,
    pub linear_damping: f32,
    pub is_static: bool,
    pub color: u32,
    pub trail: bool,
}

#[derive(Clone, Debug)]
pub enum JointSpec {
    Pendulum {
        pivot: Vec2,
        bob_pos: Vec2,
        bob_radius: f32,
        motor_speed: f32,
        color: u32,
    },
    ChainBridge {
        start: Vec2,
        end: Vec2,
        links: usize,
        plank_thickness: f32,
        color: u32,
    },
}

#[derive(Clone, Debug)]
pub struct SoftBodySpec {
    pub center: Vec2,
    pub radius: f32,
    pub ring_points: usize,
    pub pressure: f32,
    pub color: u32,
}

#[derive(Clone, Debug)]
pub struct ClothSpec {
    pub origin: Vec2,
    pub cols: usize,
    pub rows: usize,
    pub spacing: f32,
    pub tear_stretch: f32,
}

#[derive(Clone, Debug)]
pub struct WorldConfigSpec {
    pub gravity: Vec2,
    pub arena_walls: bool,
    pub high_precision: bool,
}

impl Default for WorldConfigSpec {
    fn default() -> Self {
        Self {
            gravity: Vec2::new(0.0, 9.81),
            arena_walls: true,
            high_precision: false,
        }
    }
}

/// Complete compiled parametric physics scene produced by a `physics.scene_output` node!
#[derive(Clone, Debug, Default)]
pub struct CompiledPhysicsBlueprint {
    pub config: WorldConfigSpec,
    pub bodies: Vec<BodySpec>,
    pub joints: Vec<JointSpec>,
    pub force_fields: Vec<ForceField>,
    pub soft_bodies: Vec<SoftBodySpec>,
    pub cloths: Vec<ClothSpec>,
}

/// Live instantiated `ferroui-physics` scene ready to simulate and render.
pub struct BuiltPhysicsScene {
    pub world: PhysicsWorld,
    pub soft_bodies: Vec<(SoftBody2D, u32)>,
    pub cloth: Option<ClothMesh>,
    pub trail_bodies: Vec<(BodyHandle, u32)>,
}

impl CompiledPhysicsBlueprint {
    /// Materializes this parametric blueprint into a live [`PhysicsWorld`] + soft bodies + cloth.
    pub fn build(&self, world_width: f32, world_height: f32) -> BuiltPhysicsScene {
        let mut world = PhysicsWorld::new(self.config.gravity);
        world.config = if self.config.high_precision {
            SolverConfig::high_precision()
        } else {
            SolverConfig::game_default()
        };

        if self.config.arena_walls {
            let wall_mat = PhysicsMaterial::new(0.0, 0.35, 0.65);
            let t = 0.7_f32;
            let floor = BodyBuilder::static_body(Shape2D::rect(world_width + 2.0, t))
                .position((world_width * 0.5, world_height - t * 0.5))
                .material(wall_mat)
                .user_data(GRAPH_ARENA_WALL_TAG);
            let ceil = BodyBuilder::static_body(Shape2D::rect(world_width + 2.0, t))
                .position((world_width * 0.5, -t * 0.5))
                .material(wall_mat)
                .user_data(GRAPH_ARENA_WALL_TAG);
            let left = BodyBuilder::static_body(Shape2D::rect(t, world_height + 2.0))
                .position((-t * 0.5, world_height * 0.5))
                .material(wall_mat)
                .user_data(GRAPH_ARENA_WALL_TAG);
            let right = BodyBuilder::static_body(Shape2D::rect(t, world_height + 2.0))
                .position((world_width + t * 0.5, world_height * 0.5))
                .material(wall_mat)
                .user_data(GRAPH_ARENA_WALL_TAG);
            world.add_body(floor);
            world.add_body(ceil);
            world.add_body(left);
            world.add_body(right);
        }

        for ff in &self.force_fields {
            world.add_force_field(ff.clone());
        }

        let mut trail_bodies = Vec::new();
        for b in &self.bodies {
            let builder = if b.is_static {
                BodyBuilder::static_body(b.shape.clone())
            } else {
                BodyBuilder::dynamic(b.shape.clone())
            };
            let h = world.add_body(
                builder
                    .position(b.position)
                    .angle(b.angle)
                    .linear_velocity(b.velocity)
                    .angular_velocity(b.angular_velocity)
                    .density(b.density.max(0.05))
                    .restitution(b.restitution.clamp(0.0, 0.99))
                    .friction(b.friction.clamp(0.0, 1.5))
                    .linear_damping(b.linear_damping.max(0.0))
                    .can_sleep(!b.trail)
                    .user_data(b.color as u64),
            );
            if b.trail && !b.is_static {
                trail_bodies.push((h, b.color));
            }
        }

        for j in &self.joints {
            match *j {
                JointSpec::Pendulum { pivot, bob_pos, bob_radius, motor_speed, color } => {
                    let anchor = world.add_body(
                        BodyBuilder::static_body(Shape2D::circle(0.16))
                            .position(pivot)
                            .user_data(0xFF94A3B8),
                    );
                    let bob = world.add_body(
                        BodyBuilder::dynamic(Shape2D::circle(bob_radius.clamp(0.15, 1.4)))
                            .position(bob_pos)
                            .density(4.5)
                            .restitution(0.7)
                            .can_sleep(false)
                            .user_data(color as u64),
                    );
                    let dist = (bob_pos - pivot).length().max(0.3);
                    world.add_joint(Joint::distance(anchor, bob, Vec2::ZERO, Vec2::ZERO, dist));
                    if motor_speed.abs() > 0.01 {
                        let tangent = (bob_pos - pivot).perp().normalize_or_zero();
                        if let Some(body) = world.body_mut(bob) {
                            body.linear_velocity = tangent * motor_speed;
                        }
                    }
                }
                JointSpec::ChainBridge { start, end, links, plank_thickness, color } => {
                    let links = links.clamp(2, 24);
                    let left = world.add_body(
                        BodyBuilder::static_body(Shape2D::circle(0.16))
                            .position(start)
                            .user_data(0xFF94A3B8),
                    );
                    let right = world.add_body(
                        BodyBuilder::static_body(Shape2D::circle(0.16))
                            .position(end)
                            .user_data(0xFF94A3B8),
                    );
                    let span = end - start;
                    let total_len = span.length().max(1.0);
                    let dir = span / total_len;
                    let seg_len = total_len / (links as f32 + 0.35);
                    let link_w = (seg_len * 0.92).max(0.25);
                    let ang = dir.y.atan2(dir.x);

                    let mut prev = left;
                    let mut prev_local = Vec2::ZERO;
                    for i in 0..links {
                        let t = (i as f32 + 0.5) / links as f32;
                        let pos = start + span * t;
                        let plank = world.add_body(
                            BodyBuilder::dynamic(Shape2D::rect(link_w, plank_thickness.clamp(0.08, 0.45)))
                                .position(pos)
                                .angle(ang)
                                .density(1.4)
                                .user_data(color as u64),
                        );
                        world.add_joint(Joint::revolute(
                            prev,
                            plank,
                            prev_local,
                            Vec2::new(-link_w * 0.46, 0.0),
                        ));
                        prev = plank;
                        prev_local = Vec2::new(link_w * 0.46, 0.0);
                    }
                    world.add_joint(Joint::revolute(prev, right, prev_local, Vec2::ZERO));
                }
            }
        }

        let bounds = Aabb2::new(Vec2::new(0.2, 0.2), Vec2::new(world_width - 0.2, world_height - 0.78));
        let soft_bodies: Vec<(SoftBody2D, u32)> = self
            .soft_bodies
            .iter()
            .map(|sb| {
                let body = SoftBody2D::circle_blob(
                    sb.center,
                    sb.radius.clamp(0.3, 1.8),
                    sb.ring_points.clamp(8, 24),
                    sb.pressure.clamp(0.4, 3.0),
                    0.0004,
                    bounds,
                    sb.color,
                );
                (body, sb.color)
            })
            .collect();

        let cloth = self.cloths.first().map(|c| {
            ClothMesh::grid(
                c.origin,
                c.cols.clamp(4, 24),
                c.rows.clamp(4, 18),
                c.spacing.clamp(0.18, 0.65),
                0.00018,
                Some(c.tear_stretch.clamp(1.4, 4.0)),
                bounds,
            )
        });

        BuiltPhysicsScene { world, soft_bodies, cloth, trail_bodies }
    }
}

/// Extracts the most recent [`CompiledPhysicsBlueprint`] from any `physics.scene_output` node in `graph`.
pub fn extract_physics_blueprint(graph: &NodeGraph) -> Option<CompiledPhysicsBlueprint> {
    for n in graph.nodes.values() {
        if n.type_id == "physics.scene_output" {
            if let Some(tree) = n.output_tree(0) {
                if let Some(bp) = tree
                    .first_item()
                    .and_then(|v| v.downcast_custom::<CompiledPhysicsBlueprint>())
                {
                    return Some(bp.clone());
                }
            }
        }
    }
    None
}

fn reg(
    r: &mut NodeRegistry,
    id: &str,
    name: &str,
    nick: &str,
    desc: &str,
    inputs: Vec<PortSpec>,
    outputs: Vec<PortSpec>,
    eval_fn: impl Fn(&mut EvalContext<'_>) -> Result<(), String> + Send + Sync + 'static,
) {
    r.register(FnComponent {
        type_id: id.into(),
        display_name: name.into(),
        nickname: nick.into(),
        category: NodeCategory::Physics,
        description: desc.into(),
        kind: NodeKind::Component,
        default_widget: ParamWidget::None,
        inputs,
        outputs,
        eval_fn: Box::new(eval_fn),
    });
}

/// Registers all Parametric Physics nodes into `registry`.
pub fn register_physics_nodes(r: &mut NodeRegistry) {
    reg(
        r,
        "physics.shape_box",
        "Box Shape",
        "Box",
        "Creates a rectangular collision Shape2D(W, H).",
        vec![
            PortSpec::new("W", "Width", PortType::Float, Value::Float(0.6)),
            PortSpec::new("H", "Height", PortType::Float, Value::Float(0.6)),
        ],
        vec![PortSpec::new("S", "Shape", PortType::Custom("PhysicsShape".into()), Value::Null)],
        |ctx| {
            ctx.map_items(|args| {
                let w = args[0].as_float().unwrap_or(0.6).clamp(0.12, 8.0) as f32;
                let h = args[1].as_float().unwrap_or(0.6).clamp(0.12, 8.0) as f32;
                Ok(vec![Value::custom("PhysicsShape", format!("Box({w:.2}×{h:.2})"), Shape2D::rect(w, h))])
            })
        },
    );

    reg(
        r,
        "physics.shape_circle",
        "Circle Shape",
        "Circ",
        "Creates a circular collision Shape2D(R).",
        vec![PortSpec::new("R", "Radius", PortType::Float, Value::Float(0.32))],
        vec![PortSpec::new("S", "Shape", PortType::Custom("PhysicsShape".into()), Value::Null)],
        |ctx| {
            ctx.map_items(|args| {
                let rad = args[0].as_float().unwrap_or(0.32).clamp(0.08, 3.0) as f32;
                Ok(vec![Value::custom("PhysicsShape", format!("Circle(r={rad:.2})"), Shape2D::circle(rad))])
            })
        },
    );

    reg(
        r,
        "physics.shape_polygon",
        "Regular Polygon",
        "Poly",
        "Creates an N-sided convex polygon Shape2D.",
        vec![
            PortSpec::new("N", "Sides", PortType::Int, Value::Int(6)),
            PortSpec::new("R", "Radius", PortType::Float, Value::Float(0.34)),
        ],
        vec![PortSpec::new("S", "Shape", PortType::Custom("PhysicsShape".into()), Value::Null)],
        |ctx| {
            ctx.map_items(|args| {
                let sides = args[0].as_int().unwrap_or(6).clamp(3, 8) as usize;
                let rad = args[1].as_float().unwrap_or(0.34).clamp(0.1, 2.5) as f32;
                Ok(vec![Value::custom(
                    "PhysicsShape",
                    format!("Poly({sides}, r={rad:.2})"),
                    Shape2D::regular_polygon(sides, rad),
                )])
            })
        },
    );

    reg(
        r,
        "physics.rigid_body",
        "Rigid Body",
        "Body",
        "Constructs parametric RigidBody specs from Point lists/trees with Grasshopper lacing!",
        vec![
            PortSpec::new("Pt", "Position", PortType::Vec2, Value::Vec2([8.0, 4.0])),
            PortSpec::new("S", "Shape", PortType::Custom("PhysicsShape".into()), Value::Null),
            PortSpec::new("V", "Velocity", PortType::Vec2, Value::Vec2([0.0, 0.0])),
            PortSpec::new("e", "Bounciness", PortType::Float, Value::Float(0.45)),
            PortSpec::new("St", "Static", PortType::Bool, Value::Bool(false)),
            PortSpec::new("C", "Color", PortType::Color, Value::Color(0xFF38BDF8)),
            PortSpec::new("Tr", "Orbit Trail", PortType::Bool, Value::Bool(false)),
        ],
        vec![PortSpec::new("B", "BodySpec", PortType::Custom("BodySpec".into()), Value::Null)],
        |ctx| {
            ctx.map_items(|args| {
                let p = args[0].as_vec2().unwrap_or([8.0, 4.0]);
                let shape = args[1]
                    .downcast_custom::<Shape2D>()
                    .cloned()
                    .unwrap_or_else(|| Shape2D::rect(0.58, 0.58));
                let v = args[2].as_vec2().unwrap_or([0.0, 0.0]);
                let rest = args[3].as_float().unwrap_or(0.45) as f32;
                let is_static = args[4].as_bool().unwrap_or(false);
                let color = args[5].as_color().unwrap_or(0xFF38BDF8);
                let trail = args[6].as_bool().unwrap_or(false);
                let spec = BodySpec {
                    position: Vec2::new(p[0] as f32, p[1] as f32),
                    velocity: Vec2::new(v[0] as f32, v[1] as f32),
                    angle: 0.0,
                    angular_velocity: 0.0,
                    shape,
                    density: 1.0,
                    restitution: rest,
                    friction: 0.5,
                    linear_damping: if trail { 0.0 } else { 0.02 },
                    is_static,
                    color,
                    trail,
                };
                Ok(vec![Value::custom(
                    "BodySpec",
                    format!("Body({:.1},{:.1})", p[0], p[1]),
                    spec,
                )])
            })
        },
    );

    reg(
        r,
        "physics.pendulum",
        "Wrecking Pendulum",
        "Pend",
        "Creates a distance-constrained swinging wrecking ball pendulum.",
        vec![
            PortSpec::new("Pv", "Pivot", PortType::Vec2, Value::Vec2([10.0, 1.2])),
            PortSpec::new("Bb", "Bob Pos", PortType::Vec2, Value::Vec2([13.6, 3.6])),
            PortSpec::new("R", "Bob Radius", PortType::Float, Value::Float(0.62)),
            PortSpec::new("Sp", "Swing Speed", PortType::Float, Value::Float(4.5)),
            PortSpec::new("C", "Color", PortType::Color, Value::Color(0xFFF59E0B)),
        ],
        vec![PortSpec::new("J", "JointSpec", PortType::Custom("JointSpec".into()), Value::Null)],
        |ctx| {
            ctx.map_items(|args| {
                let pv = args[0].as_vec2().unwrap_or([10.0, 1.2]);
                let bb = args[1].as_vec2().unwrap_or([13.6, 3.6]);
                let rad = args[2].as_float().unwrap_or(0.62) as f32;
                let spd = args[3].as_float().unwrap_or(4.5) as f32;
                let col = args[4].as_color().unwrap_or(0xFFF59E0B);
                let j = JointSpec::Pendulum {
                    pivot: Vec2::new(pv[0] as f32, pv[1] as f32),
                    bob_pos: Vec2::new(bb[0] as f32, bb[1] as f32),
                    bob_radius: rad,
                    motor_speed: spd,
                    color: col,
                };
                Ok(vec![Value::custom("JointSpec", "Pendulum", j)])
            })
        },
    );

    reg(
        r,
        "physics.chain_bridge",
        "Suspension Bridge",
        "Bridge",
        "Builds a parametric multi-link revolute-jointed suspension bridge between A and B.",
        vec![
            PortSpec::new("A", "Left Anchor", PortType::Vec2, Value::Vec2([2.4, 5.4])),
            PortSpec::new("B", "Right Anchor", PortType::Vec2, Value::Vec2([13.6, 5.4])),
            PortSpec::new("N", "Links", PortType::Int, Value::Int(11)),
            PortSpec::new("C", "Color", PortType::Color, Value::Color(0xFF2DD4BF)),
        ],
        vec![PortSpec::new("J", "JointSpec", PortType::Custom("JointSpec".into()), Value::Null)],
        |ctx| {
            ctx.map_items(|args| {
                let a = args[0].as_vec2().unwrap_or([2.4, 5.4]);
                let b = args[1].as_vec2().unwrap_or([13.6, 5.4]);
                let n = args[2].as_int().unwrap_or(11).clamp(2, 24) as usize;
                let col = args[3].as_color().unwrap_or(0xFF2DD4BF);
                let j = JointSpec::ChainBridge {
                    start: Vec2::new(a[0] as f32, a[1] as f32),
                    end: Vec2::new(b[0] as f32, b[1] as f32),
                    links: n,
                    plank_thickness: 0.18,
                    color: col,
                };
                Ok(vec![Value::custom("JointSpec", format!("Bridge({n} links)"), j)])
            })
        },
    );

    reg(
        r,
        "physics.gravity_well",
        "Newtonian Star",
        "Gravity",
        "Creates an exact inverse-square Newtonian orbital gravity field (a = μ / r²).",
        vec![
            PortSpec::new("C", "Center", PortType::Vec2, Value::Vec2([8.2, 5.0])),
            PortSpec::new("μ", "Star Mass", PortType::Float, Value::Float(48.0)),
        ],
        vec![PortSpec::new("F", "ForceField", PortType::Custom("ForceField".into()), Value::Null)],
        |ctx| {
            ctx.map_items(|args| {
                let c = args[0].as_vec2().unwrap_or([8.2, 5.0]);
                let mu = args[1].as_float().unwrap_or(48.0).clamp(1.0, 250.0) as f32;
                let ff = ForceField::NewtonianGravity {
                    center: Vec2::new(c[0] as f32, c[1] as f32),
                    mu,
                    min_radius: 0.85,
                    max_radius: 50.0,
                };
                Ok(vec![Value::custom("ForceField", format!("Gravity(μ={mu:.0})"), ff)])
            })
        },
    );

    reg(
        r,
        "physics.wind_field",
        "Wind / Vortex",
        "Wind",
        "Creates a global directional Wind or Swirl Vortex force field.",
        vec![
            PortSpec::new("Ax", "Wind X", PortType::Float, Value::Float(3.5)),
            PortSpec::new("Ay", "Wind Y", PortType::Float, Value::Float(0.0)),
            PortSpec::new("Vx", "Vortex Spin", PortType::Float, Value::Float(0.0)),
        ],
        vec![PortSpec::new("F", "ForceField", PortType::Custom("ForceField".into()), Value::Null)],
        |ctx| {
            ctx.map_items(|args| {
                let ax = args[0].as_float().unwrap_or(3.5) as f32;
                let ay = args[1].as_float().unwrap_or(0.0) as f32;
                let spin = args[2].as_float().unwrap_or(0.0) as f32;
                let ff = if spin.abs() > 0.1 {
                    ForceField::Vortex {
                        center: Vec2::new(8.0, 5.0),
                        tangent_strength: spin,
                        inward_strength: 1.8,
                        radius: 7.0,
                    }
                } else {
                    ForceField::Wind { acceleration: Vec2::new(ax, ay), region: None }
                };
                Ok(vec![Value::custom("ForceField", format!("Wind({ax:.1},{ay:.1})"), ff)])
            })
        },
    );

    reg(
        r,
        "physics.soft_body",
        "Jelly SoftBody",
        "Jelly",
        "Generates pressure-inflated XPBD squishy jelly soft bodies at positions.",
        vec![
            PortSpec::new("Pt", "Center", PortType::Vec2, Value::Vec2([11.0, 3.0])),
            PortSpec::new("R", "Radius", PortType::Float, Value::Float(0.85)),
            PortSpec::new("Pr", "Pressure", PortType::Float, Value::Float(1.35)),
            PortSpec::new("C", "Color", PortType::Color, Value::Color(0xFFF472B6)),
        ],
        vec![PortSpec::new("SB", "SoftBody", PortType::Custom("SoftBodySpec".into()), Value::Null)],
        |ctx| {
            ctx.map_items(|args| {
                let p = args[0].as_vec2().unwrap_or([11.0, 3.0]);
                let rad = args[1].as_float().unwrap_or(0.85) as f32;
                let pr = args[2].as_float().unwrap_or(1.35) as f32;
                let col = args[3].as_color().unwrap_or(0xFFF472B6);
                let spec = SoftBodySpec {
                    center: Vec2::new(p[0] as f32, p[1] as f32),
                    radius: rad,
                    ring_points: 14,
                    pressure: pr,
                    color: col,
                };
                Ok(vec![Value::custom("SoftBodySpec", format!("Jelly(r={rad:.2})"), spec)])
            })
        },
    );

    reg(
        r,
        "physics.cloth",
        "Tearable Cloth",
        "Cloth",
        "Generates an XPBD tearable hanging cloth curtain grid.",
        vec![
            PortSpec::new("O", "Origin", PortType::Vec2, Value::Vec2([1.6, 1.0])),
            PortSpec::new("W", "Cols", PortType::Int, Value::Int(14)),
            PortSpec::new("H", "Rows", PortType::Int, Value::Int(10)),
            PortSpec::new("Sp", "Spacing", PortType::Float, Value::Float(0.36)),
        ],
        vec![PortSpec::new("Cl", "Cloth", PortType::Custom("ClothSpec".into()), Value::Null)],
        |ctx| {
            ctx.map_items(|args| {
                let o = args[0].as_vec2().unwrap_or([1.6, 1.0]);
                let cols = args[1].as_int().unwrap_or(14).clamp(4, 24) as usize;
                let rows = args[2].as_int().unwrap_or(10).clamp(4, 18) as usize;
                let spacing = args[3].as_float().unwrap_or(0.36) as f32;
                let spec = ClothSpec {
                    origin: Vec2::new(o[0] as f32, o[1] as f32),
                    cols,
                    rows,
                    spacing,
                    tear_stretch: 2.1,
                };
                Ok(vec![Value::custom("ClothSpec", format!("Cloth({cols}×{rows})"), spec)])
            })
        },
    );

    reg(
        r,
        "physics.world_config",
        "World Config",
        "World",
        "Configures global gravity, arena walls, and high-precision solver mode.",
        vec![
            PortSpec::new("Gy", "Gravity Y", PortType::Float, Value::Float(9.81)),
            PortSpec::new("Wl", "Arena Walls", PortType::Bool, Value::Bool(true)),
            PortSpec::new("HP", "High Prec.", PortType::Bool, Value::Bool(false)),
        ],
        vec![PortSpec::new("Cfg", "WorldConfig", PortType::Custom("WorldConfig".into()), Value::Null)],
        |ctx| {
            let gy = ctx.input(0).first_item().and_then(Value::as_float).unwrap_or(9.81) as f32;
            let walls = ctx.input(1).first_item().and_then(Value::as_bool).unwrap_or(true);
            let hp = ctx.input(2).first_item().and_then(Value::as_bool).unwrap_or(false);
            let cfg = WorldConfigSpec { gravity: Vec2::new(0.0, gy), arena_walls: walls, high_precision: hp };
            ctx.set_output_single(0, Value::custom("WorldConfig", format!("World(g={gy:.1})"), cfg));
            Ok(())
        },
    );

    reg(
        r,
        "physics.scene_output",
        "Physics Scene Output",
        "SceneOut",
        "Terminal Grasshopper sink compiling all Bodies, Joints, Fields, SoftBodies, and Cloth into the live Physics Studio viewport!",
        vec![
            PortSpec::new("Cfg", "WorldConfig", PortType::Custom("WorldConfig".into()), Value::Null).with_access(ParamAccess::Tree),
            PortSpec::new("B", "Bodies", PortType::Custom("BodySpec".into()), Value::Null).with_access(ParamAccess::Tree),
            PortSpec::new("J", "Joints", PortType::Custom("JointSpec".into()), Value::Null).with_access(ParamAccess::Tree),
            PortSpec::new("F", "Fields", PortType::Custom("ForceField".into()), Value::Null).with_access(ParamAccess::Tree),
            PortSpec::new("SB", "SoftBodies", PortType::Custom("SoftBodySpec".into()), Value::Null).with_access(ParamAccess::Tree),
            PortSpec::new("Cl", "Cloth", PortType::Custom("ClothSpec".into()), Value::Null).with_access(ParamAccess::Tree),
        ],
        vec![PortSpec::new("Out", "Blueprint", PortType::Custom("PhysicsBlueprint".into()), Value::Null)],
        |ctx| {
            let mut bp = CompiledPhysicsBlueprint::default();
            if let Some(cfg) = ctx.input(0).first_item().and_then(|v| v.downcast_custom::<WorldConfigSpec>()) {
                bp.config = cfg.clone();
            }
            for item in ctx.input(1).all_items() {
                if let Some(b) = item.downcast_custom::<BodySpec>() {
                    bp.bodies.push(b.clone());
                }
            }
            for item in ctx.input(2).all_items() {
                if let Some(j) = item.downcast_custom::<JointSpec>() {
                    bp.joints.push(j.clone());
                }
            }
            for item in ctx.input(3).all_items() {
                if let Some(f) = item.downcast_custom::<ForceField>() {
                    bp.force_fields.push(f.clone());
                }
            }
            for item in ctx.input(4).all_items() {
                if let Some(sb) = item.downcast_custom::<SoftBodySpec>() {
                    bp.soft_bodies.push(sb.clone());
                }
            }
            for item in ctx.input(5).all_items() {
                if let Some(cl) = item.downcast_custom::<ClothSpec>() {
                    bp.cloths.push(cl.clone());
                }
            }
            let summary = format!(
                "{} bodies · {} joints · {} fields",
                bp.bodies.len(),
                bp.joints.len(),
                bp.force_fields.len()
            );
            ctx.set_output_single(0, Value::custom("PhysicsBlueprint", summary, bp));
            Ok(())
        },
    );
}

// =============================================================================
// 4 Rich One-Click Rhino Grasshopper Parametric Physics Templates!
// =============================================================================

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhysicsGraphTemplate {
    AttractorGraphMapper,
    PyramidPendulum,
    KeplerianSolarSystem,
    SuspensionBridgeRain,
    ClothJellyWindTunnel,
}

impl PhysicsGraphTemplate {
    pub const ALL: [Self; 5] = [
        Self::AttractorGraphMapper,
        Self::PyramidPendulum,
        Self::KeplerianSolarSystem,
        Self::SuspensionBridgeRain,
        Self::ClothJellyWindTunnel,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::AttractorGraphMapper => "🎨 Attractor & GraphMapper",
            Self::PyramidPendulum => "🧱 Pyramid & Pendulum",
            Self::KeplerianSolarSystem => "🪐 Keplerian Solar System",
            Self::SuspensionBridgeRain => "🌉 Bridge & Rain",
            Self::ClothJellyWindTunnel => "🌊 Cloth & Jelly Tunnel",
        }
    }

    pub fn build(self, reg: &NodeRegistry) -> NodeGraph {
        match self {
            Self::AttractorGraphMapper => build_attractor_graph_mapper(reg),
            Self::PyramidPendulum => build_pyramid_pendulum_graph(reg),
            Self::KeplerianSolarSystem => build_solar_system_graph(reg),
            Self::SuspensionBridgeRain => build_bridge_rain_graph(reg),
            Self::ClothJellyWindTunnel => build_cloth_jelly_graph(reg),
        }
    }
}

/// Showcases Grasshopper's most iconic real FerroUI visual widgets wired live to `ferroui-physics`:
/// 2D `MdSlider` Attractor Pad -> `PointAttractor` -> interactive `GraphMapper` curve -> `ColorHsv` +
/// parametric `Circle` sizes + `ParamViewer` DataTree inspector!
fn build_attractor_graph_mapper(reg: &NodeRegistry) -> NodeGraph {
    let mut g = NodeGraph::new("Grasshopper Attractor & GraphMapper Field");

    let md = g.add_node(reg, "params.md_slider", 30.0, 30.0).unwrap();
    let s_cols = g.add_node(reg, "params.int_slider", 30.0, 210.0).unwrap();
    g.configure_slider(s_cols, "Cols", 8.0, 4.0, 12.0, 1.0, true);
    let s_rows = g.add_node(reg, "params.int_slider", 30.0, 272.0).unwrap();
    g.configure_slider(s_rows, "Rows", 4.0, 2.0, 7.0, 1.0, true);
    let s_rad = g.add_node(reg, "params.slider", 30.0, 334.0).unwrap();
    g.configure_slider(s_rad, "Range", 7.2, 2.5, 13.0, 0.2, false);

    // Map MD Slider [0..1]x[0..1] into arena [2..14] x [1.5..8.5]
    let map_x = g.add_node(reg, "math.lerp", 330.0, 24.0).unwrap();
    g.set_input_default(map_x, 0, Value::Float(2.0));
    g.set_input_default(map_x, 1, Value::Float(14.0));
    let map_y = g.add_node(reg, "math.lerp", 330.0, 114.0).unwrap();
    g.set_input_default(map_y, 0, Value::Float(1.5));
    g.set_input_default(map_y, 1, Value::Float(8.5));
    let attr_pt = g.add_node(reg, "vector.point2d", 520.0, 62.0).unwrap();

    let grid = g.add_node(reg, "geom.rect_grid", 330.0, 224.0).unwrap();
    g.set_input_default(grid, 0, Value::Vec2([8.0, 7.4]));
    g.set_input_default(grid, 1, Value::Float(1.25));
    g.set_input_default(grid, 2, Value::Float(-1.15));
    g.set_input_default(grid, 5, Value::Bool(false));

    let attr = g.add_node(reg, "geom.attractor", 520.0, 194.0).unwrap();
    let gmap = g.add_node(reg, "params.graph_mapper", 720.0, 110.0).unwrap();
    let pview = g.add_node(reg, "params.param_viewer", 520.0, 330.0).unwrap();

    let rad_map = g.add_node(reg, "math.lerp", 955.0, 96.0).unwrap();
    g.set_input_default(rad_map, 0, Value::Float(0.18));
    g.set_input_default(rad_map, 1, Value::Float(0.56));
    let shape = g.add_node(reg, "physics.shape_circle", 1150.0, 96.0).unwrap();
    let hsv = g.add_node(reg, "geom.color_hsv", 955.0, 224.0).unwrap();

    let bodies = g.add_node(reg, "physics.rigid_body", 1150.0, 204.0).unwrap();
    g.set_input_default(bodies, 3, Value::Float(0.72));
    let wcfg = g.add_node(reg, "physics.world_config", 955.0, 344.0).unwrap();
    g.set_input_default(wcfg, 0, Value::Float(2.5));
    let scene_out = g.add_node(reg, "physics.scene_output", 1365.0, 204.0).unwrap();

    let _ = g.connect(md, 1, map_x, 2, false);
    let _ = g.connect(md, 2, map_y, 2, false);
    let _ = g.connect(map_x, 0, attr_pt, 0, false);
    let _ = g.connect(map_y, 0, attr_pt, 1, false);

    let _ = g.connect(s_cols, 0, grid, 3, false);
    let _ = g.connect(s_rows, 0, grid, 4, false);

    let _ = g.connect(grid, 0, attr, 0, false);
    let _ = g.connect(attr_pt, 0, attr, 1, false);
    let _ = g.connect(s_rad, 0, attr, 2, false);
    let _ = g.connect(grid, 0, pview, 0, false);

    let _ = g.connect(attr, 0, gmap, 0, false);
    let _ = g.connect(gmap, 0, rad_map, 2, false);
    let _ = g.connect(rad_map, 0, shape, 0, false);
    let _ = g.connect(gmap, 0, hsv, 0, false);

    let _ = g.connect(grid, 0, bodies, 0, false);
    let _ = g.connect(shape, 0, bodies, 1, false);
    let _ = g.connect(hsv, 0, bodies, 5, false);

    let _ = g.connect(wcfg, 0, scene_out, 0, false);
    let _ = g.connect(bodies, 0, scene_out, 1, false);

    g.create_group("2D Attractor & Grid Controls", 0x2210B981, vec![md, s_cols, s_rows, s_rad]);
    g.create_group("Grasshopper Attractor + GraphMapper", 0x2238BDF8, vec![map_x, map_y, attr_pt, grid, attr, gmap, pview]);
    g.create_group("Parametric Physics Output", 0x22A78BFA, vec![rad_map, shape, hsv, bodies, wcfg, scene_out]);
    g
}

fn build_pyramid_pendulum_graph(reg: &NodeRegistry) -> NodeGraph {
    let mut g = NodeGraph::new("Parametric Pyramid & Wrecking Pendulum");

    let s_cols = g.add_node(reg, "params.int_slider", 30.0, 40.0).unwrap();
    g.configure_slider(s_cols, "Base Columns", 7.0, 3.0, 11.0, 1.0, true);

    let s_size = g.add_node(reg, "params.slider", 30.0, 104.0).unwrap();
    g.configure_slider(s_size, "Brick Size (m)", 0.62, 0.35, 0.95, 0.02, false);

    let s_bounce = g.add_node(reg, "params.slider", 30.0, 168.0).unwrap();
    g.configure_slider(s_bounce, "Bounciness", 0.30, 0.0, 0.95, 0.05, false);

    let s_grav = g.add_node(reg, "params.slider", 30.0, 232.0).unwrap();
    g.configure_slider(s_grav, "Gravity (m/s²)", 9.81, -5.0, 22.0, 0.5, false);

    let s_swing = g.add_node(reg, "params.slider", 30.0, 308.0).unwrap();
    g.configure_slider(s_swing, "Pendulum Speed", 5.8, 0.0, 14.0, 0.2, false);

    let s_pyr = g.add_node(reg, "params.toggle", 30.0, 372.0).unwrap();
    g.configure_toggle(s_pyr, "Pyramid Mode", true);

    let neg_size = g.add_node(reg, "math.neg", 285.0, 125.0).unwrap();
    let grid = g.add_node(reg, "geom.rect_grid", 460.0, 40.0).unwrap();
    g.set_input_default(grid, 0, Value::Vec2([5.5, 8.95]));

    let box_shape = g.add_node(reg, "physics.shape_box", 460.0, 210.0).unwrap();
    let hue_series = g.add_node(reg, "sets.series", 285.0, 215.0).unwrap();
    g.set_input_default(hue_series, 0, Value::Float(0.52));
    g.set_input_default(hue_series, 1, Value::Float(0.065));
    let hsv = g.add_node(reg, "geom.color_hsv", 460.0, 300.0).unwrap();

    let body = g.add_node(reg, "physics.rigid_body", 710.0, 70.0).unwrap();
    let pend = g.add_node(reg, "physics.pendulum", 710.0, 270.0).unwrap();
    g.set_input_default(pend, 0, Value::Vec2([9.8, 1.4]));
    g.set_input_default(pend, 1, Value::Vec2([14.2, 4.4]));
    g.set_input_default(pend, 2, Value::Float(0.68));

    let wcfg = g.add_node(reg, "physics.world_config", 710.0, 420.0).unwrap();
    let scene_out = g.add_node(reg, "physics.scene_output", 960.0, 170.0).unwrap();
    let panel = g.add_node(reg, "params.panel", 960.0, 355.0).unwrap();

    let _ = g.connect(s_size, 0, neg_size, 0, false);
    let _ = g.connect(s_size, 0, grid, 1, false);
    let _ = g.connect(neg_size, 0, grid, 2, false);
    let _ = g.connect(s_cols, 0, grid, 3, false);
    let _ = g.connect(s_cols, 0, grid, 4, false);
    let _ = g.connect(s_pyr, 0, grid, 5, false);

    let _ = g.connect(s_size, 0, box_shape, 0, false);
    let _ = g.connect(s_size, 0, box_shape, 1, false);
    let _ = g.connect(s_cols, 0, hue_series, 2, false);
    let _ = g.connect(hue_series, 0, hsv, 0, false);

    let _ = g.connect(grid, 0, body, 0, false);
    let _ = g.connect(box_shape, 0, body, 1, false);
    let _ = g.connect(s_bounce, 0, body, 3, false);
    let _ = g.connect(hsv, 0, body, 5, false);

    let _ = g.connect(s_swing, 0, pend, 3, false);
    let _ = g.connect(s_grav, 0, wcfg, 0, false);

    let _ = g.connect(wcfg, 0, scene_out, 0, false);
    let _ = g.connect(body, 0, scene_out, 1, false);
    let _ = g.connect(pend, 0, scene_out, 2, false);
    let _ = g.connect(scene_out, 0, panel, 0, false);

    g.create_group("Parameters", 0x2210B981, vec![s_cols, s_size, s_bounce, s_grav, s_swing, s_pyr]);
    g.create_group("Parametric Structure", 0x2238BDF8, vec![neg_size, grid, box_shape, hue_series, hsv, body]);
    g.create_group("Simulation Output", 0x22A78BFA, vec![pend, wcfg, scene_out, panel]);
    g
}

fn build_solar_system_graph(reg: &NodeRegistry) -> NodeGraph {
    let mut g = NodeGraph::new("Keplerian Orbital Solar System");

    let s_count = g.add_node(reg, "params.int_slider", 30.0, 40.0).unwrap();
    g.configure_slider(s_count, "Planets", 5.0, 2.0, 8.0, 1.0, true);

    let s_mu = g.add_node(reg, "params.slider", 30.0, 108.0).unwrap();
    g.configure_slider(s_mu, "Star Mass (μ)", 48.0, 16.0, 96.0, 2.0, false);

    let s_r0 = g.add_node(reg, "params.slider", 30.0, 176.0).unwrap();
    g.configure_slider(s_r0, "Inner Orbit (m)", 1.45, 1.1, 2.5, 0.05, false);

    let s_dr = g.add_node(reg, "params.slider", 30.0, 244.0).unwrap();
    g.configure_slider(s_dr, "Orbit Spacing", 0.78, 0.45, 1.1, 0.05, false);

    let s_prad = g.add_node(reg, "params.slider", 30.0, 312.0).unwrap();
    g.configure_slider(s_prad, "Planet Radius", 0.25, 0.14, 0.42, 0.02, false);

    let radii = g.add_node(reg, "sets.series", 280.0, 60.0).unwrap();
    let circ_pts = g.add_node(reg, "geom.circle_points", 480.0, 40.0).unwrap();
    g.set_input_default(circ_pts, 0, Value::Vec2([8.2, 5.0]));

    // Exact Keplerian orbital speed v = sqrt(mu / r) via live Expression node!
    let expr = g.add_node(reg, "math.expression", 480.0, 190.0).unwrap();
    g.configure_text(expr, "Kepler v=√(μ/r)", "sqrt(x / max(y, 0.5))");

    let vel_vec = g.add_node(reg, "vector.scale", 720.0, 150.0).unwrap();
    let p_shape = g.add_node(reg, "physics.shape_circle", 480.0, 310.0).unwrap();
    let star_shape = g.add_node(reg, "physics.shape_circle", 480.0, 390.0).unwrap();
    g.set_input_default(star_shape, 0, Value::Float(0.62));

    let star_body = g.add_node(reg, "physics.rigid_body", 720.0, 360.0).unwrap();
    g.set_input_default(star_body, 0, Value::Vec2([8.2, 5.0]));
    g.set_input_default(star_body, 4, Value::Bool(true));
    g.set_input_default(star_body, 5, Value::Color(0xFFFBBF24));

    let planets = g.add_node(reg, "physics.rigid_body", 930.0, 60.0).unwrap();
    g.set_input_default(planets, 3, Value::Float(0.95));
    g.set_input_default(planets, 5, Value::Color(0xFF38BDF8));
    g.set_input_default(planets, 6, Value::Bool(true)); // Orbit trails ON!

    let grav_well = g.add_node(reg, "physics.gravity_well", 720.0, 265.0).unwrap();
    g.set_input_default(grav_well, 0, Value::Vec2([8.2, 5.0]));

    let wcfg = g.add_node(reg, "physics.world_config", 930.0, 350.0).unwrap();
    g.set_input_default(wcfg, 0, Value::Float(0.0)); // Zero-G space!
    g.set_input_default(wcfg, 2, Value::Bool(true)); // High precision 8-substep!

    let scene_out = g.add_node(reg, "physics.scene_output", 1150.0, 170.0).unwrap();
    let panel = g.add_node(reg, "params.panel", 1150.0, 355.0).unwrap();

    let _ = g.connect(s_r0, 0, radii, 0, false);
    let _ = g.connect(s_dr, 0, radii, 1, false);
    let _ = g.connect(s_count, 0, radii, 2, false);

    let _ = g.connect(radii, 0, circ_pts, 1, false);
    let _ = g.connect(s_count, 0, circ_pts, 2, false);

    let _ = g.connect(s_mu, 0, expr, 0, false);
    let _ = g.connect(radii, 0, expr, 1, false);

    let _ = g.connect(circ_pts, 1, vel_vec, 0, false);
    let _ = g.connect(expr, 0, vel_vec, 1, false);

    let _ = g.connect(s_prad, 0, p_shape, 0, false);
    let _ = g.connect(star_shape, 0, star_body, 1, false);

    let _ = g.connect(circ_pts, 0, planets, 0, false);
    let _ = g.connect(p_shape, 0, planets, 1, false);
    let _ = g.connect(vel_vec, 0, planets, 2, false);

    let _ = g.connect(s_mu, 0, grav_well, 1, false);

    let _ = g.connect(wcfg, 0, scene_out, 0, false);
    let _ = g.connect(planets, 0, scene_out, 1, true);
    let _ = g.connect(star_body, 0, scene_out, 1, true); // Multi-wire input merge!
    let _ = g.connect(grav_well, 0, scene_out, 3, false);
    let _ = g.connect(scene_out, 0, panel, 0, false);

    g.create_group("Orbital Sliders", 0x2210B981, vec![s_count, s_mu, s_r0, s_dr, s_prad]);
    g.create_group("Kepler Math v = √(μ/r)", 0x2238BDF8, vec![radii, circ_pts, expr, vel_vec]);
    g.create_group("Solar System Output", 0x22F59E0B, vec![p_shape, star_shape, star_body, planets, grav_well, wcfg, scene_out, panel]);
    g
}

fn build_bridge_rain_graph(reg: &NodeRegistry) -> NodeGraph {
    let mut g = NodeGraph::new("Parametric Suspension Bridge & Polygon Rain");

    let s_links = g.add_node(reg, "params.int_slider", 30.0, 40.0).unwrap();
    g.configure_slider(s_links, "Bridge Links", 11.0, 5.0, 18.0, 1.0, true);

    let s_balls = g.add_node(reg, "params.int_slider", 30.0, 110.0).unwrap();
    g.configure_slider(s_balls, "Falling Shapes", 10.0, 3.0, 22.0, 1.0, true);

    let s_sides = g.add_node(reg, "params.int_slider", 30.0, 180.0).unwrap();
    g.configure_slider(s_sides, "Polygon Sides", 6.0, 3.0, 8.0, 1.0, true);

    let s_wind = g.add_node(reg, "params.slider", 30.0, 250.0).unwrap();
    g.configure_slider(s_wind, "Crosswind (m/s²)", 1.5, -10.0, 10.0, 0.5, false);

    let s_bounce = g.add_node(reg, "params.slider", 30.0, 320.0).unwrap();
    g.configure_slider(s_bounce, "Bounciness", 0.55, 0.0, 0.95, 0.05, false);

    let line_pts = g.add_node(reg, "geom.line_divide", 300.0, 110.0).unwrap();
    g.set_input_default(line_pts, 0, Value::Vec2([3.2, 1.4]));
    g.set_input_default(line_pts, 1, Value::Vec2([12.8, 2.2]));

    let poly = g.add_node(reg, "physics.shape_polygon", 300.0, 215.0).unwrap();
    let bodies = g.add_node(reg, "physics.rigid_body", 540.0, 120.0).unwrap();
    g.set_input_default(bodies, 5, Value::Color(0xFFF472B6));

    let bridge = g.add_node(reg, "physics.chain_bridge", 540.0, 30.0).unwrap();
    let wind = g.add_node(reg, "physics.wind_field", 540.0, 290.0).unwrap();
    let wcfg = g.add_node(reg, "physics.world_config", 540.0, 385.0).unwrap();
    let scene_out = g.add_node(reg, "physics.scene_output", 810.0, 160.0).unwrap();

    let _ = g.connect(s_links, 0, bridge, 2, false);
    let _ = g.connect(s_balls, 0, line_pts, 2, false);
    let _ = g.connect(s_sides, 0, poly, 0, false);
    let _ = g.connect(line_pts, 0, bodies, 0, false);
    let _ = g.connect(poly, 0, bodies, 1, false);
    let _ = g.connect(s_bounce, 0, bodies, 3, false);
    let _ = g.connect(s_wind, 0, wind, 0, false);

    let _ = g.connect(wcfg, 0, scene_out, 0, false);
    let _ = g.connect(bodies, 0, scene_out, 1, false);
    let _ = g.connect(bridge, 0, scene_out, 2, false);
    let _ = g.connect(wind, 0, scene_out, 3, false);

    g.create_group("Controls", 0x2210B981, vec![s_links, s_balls, s_sides, s_wind, s_bounce]);
    g.create_group("Bridge & Rain Assembly", 0x2238BDF8, vec![line_pts, poly, bodies, bridge, wind, wcfg, scene_out]);
    g
}

fn build_cloth_jelly_graph(reg: &NodeRegistry) -> NodeGraph {
    let mut g = NodeGraph::new("XPBD Tearable Cloth & Jelly SoftBody Wind Tunnel");

    let s_cols = g.add_node(reg, "params.int_slider", 30.0, 40.0).unwrap();
    g.configure_slider(s_cols, "Cloth Width", 14.0, 6.0, 20.0, 1.0, true);

    let s_jellies = g.add_node(reg, "params.int_slider", 30.0, 110.0).unwrap();
    g.configure_slider(s_jellies, "Jelly Count", 3.0, 1.0, 6.0, 1.0, true);

    let s_press = g.add_node(reg, "params.slider", 30.0, 180.0).unwrap();
    g.configure_slider(s_press, "Jelly Pressure", 1.45, 0.6, 2.4, 0.05, false);

    let s_wind = g.add_node(reg, "params.slider", 30.0, 250.0).unwrap();
    g.configure_slider(s_wind, "Wind Gust", 4.2, -10.0, 12.0, 0.4, false);

    let line_pts = g.add_node(reg, "geom.line_divide", 300.0, 110.0).unwrap();
    g.set_input_default(line_pts, 0, Value::Vec2([9.2, 2.2]));
    g.set_input_default(line_pts, 1, Value::Vec2([13.8, 3.6]));

    let cloth = g.add_node(reg, "physics.cloth", 540.0, 30.0).unwrap();
    let jelly = g.add_node(reg, "physics.soft_body", 540.0, 160.0).unwrap();
    let wind = g.add_node(reg, "physics.wind_field", 540.0, 295.0).unwrap();

    // Static bumper pegs under the jellies
    let peg_pts = g.add_node(reg, "geom.line_divide", 300.0, 340.0).unwrap();
    g.set_input_default(peg_pts, 0, Value::Vec2([9.4, 6.6]));
    g.set_input_default(peg_pts, 1, Value::Vec2([13.6, 6.6]));
    g.set_input_default(peg_pts, 2, Value::Int(3));
    let peg_shape = g.add_node(reg, "physics.shape_circle", 300.0, 440.0).unwrap();
    g.set_input_default(peg_shape, 0, Value::Float(0.55));
    let pegs = g.add_node(reg, "physics.rigid_body", 540.0, 390.0).unwrap();
    g.set_input_default(pegs, 4, Value::Bool(true));
    g.set_input_default(pegs, 5, Value::Color(0xFF475569));

    let scene_out = g.add_node(reg, "physics.scene_output", 820.0, 160.0).unwrap();

    let _ = g.connect(s_cols, 0, cloth, 1, false);
    let _ = g.connect(s_jellies, 0, line_pts, 2, false);
    let _ = g.connect(line_pts, 0, jelly, 0, false);
    let _ = g.connect(s_press, 0, jelly, 2, false);
    let _ = g.connect(s_wind, 0, wind, 0, false);
    let _ = g.connect(peg_pts, 0, pegs, 0, false);
    let _ = g.connect(peg_shape, 0, pegs, 1, false);

    let _ = g.connect(pegs, 0, scene_out, 1, false);
    let _ = g.connect(wind, 0, scene_out, 3, false);
    let _ = g.connect(jelly, 0, scene_out, 4, false);
    let _ = g.connect(cloth, 0, scene_out, 5, false);

    g.create_group("Tunnel Controls", 0x2210B981, vec![s_cols, s_jellies, s_press, s_wind]);
    g.create_group("Deformables & Bumpers", 0x22FB7185, vec![line_pts, cloth, jelly, wind, peg_pts, peg_shape, pegs, scene_out]);
    g
}
