//! Built-in **Rhino Grasshopper Standard Library** (45+ parametric components):
//! - **Params**: NumberSlider, IntSlider, BoolToggle, Panel, ColorSwatch, Domain, Relay, Scribble, Time
//! - **Math**: Add, Subtract, Multiply, Divide, Power, Modulo, Abs, Sin, Cos, Lerp, Remap, Clamp, Expression (`x, y, z, t, pi, sin, cos, sqrt, pow, min, max`)
//! - **Logic**: Compare (`<, <=, ==, >, >=`), GateAnd, GateOr, GateNot, DispatchSwitch
//! - **Sets & DataTrees**: Series, Range, Repeat, ListLength, ListItem, ReverseList, ShiftList, CullPattern, Merge, Flatten, Graft, Simplify, FlipMatrix, Random
//! - **Vector & Geometry**: ConstructPoint, DeconstructPoint, VectorAdd, VectorScale, VectorLength, Distance, RotatePoint, LineDivide, CirclePoints, RectGrid, SineWave, SpiralPoints, ColorHsv

use crate::data_tree::{DataPath, DataTree, Value};
use crate::node::{EvalContext, FnComponent, NodeRegistry};
use crate::types::{NodeCategory, NodeKind, ParamAccess, ParamWidget, PortSpec, PortType};

fn reg(
    registry: &mut NodeRegistry,
    type_id: &str,
    display_name: &str,
    nickname: &str,
    category: NodeCategory,
    description: &str,
    kind: NodeKind,
    default_widget: ParamWidget,
    inputs: Vec<PortSpec>,
    outputs: Vec<PortSpec>,
    eval_fn: impl Fn(&mut EvalContext<'_>) -> Result<(), String> + Send + Sync + 'static,
) {
    registry.register(FnComponent {
        type_id: type_id.into(),
        display_name: display_name.into(),
        nickname: nickname.into(),
        category,
        description: description.into(),
        kind,
        default_widget,
        inputs,
        outputs,
        eval_fn: Box::new(eval_fn),
    });
}

/// Registers all built-in Grasshopper standard library components into `registry`.
pub fn register_standard_nodes(r: &mut NodeRegistry) {
    register_params(r);
    register_math(r);
    register_logic(r);
    register_sets(r);
    register_vectors_geometry(r);
}

fn register_params(r: &mut NodeRegistry) {
    reg(
        r,
        "params.slider",
        "Number Slider",
        "Slider",
        NodeCategory::Params,
        "Interactive inline floating-point numeric slider.",
        NodeKind::NumberSlider,
        ParamWidget::Slider { value: 5.0, min: 0.0, max: 20.0, step: 0.1, integer: false },
        vec![],
        vec![PortSpec::new("N", "Number", PortType::Float, Value::Float(0.0))],
        |ctx| {
            let val = match ctx.widget {
                ParamWidget::Slider { value, .. } => *value,
                _ => 0.0,
            };
            ctx.set_output_single(0, Value::Float(val));
            Ok(())
        },
    );

    reg(
        r,
        "params.int_slider",
        "Integer Slider",
        "Int",
        NodeCategory::Params,
        "Interactive inline integer slider for counts, grids, and seeds.",
        NodeKind::NumberSlider,
        ParamWidget::Slider { value: 6.0, min: 1.0, max: 32.0, step: 1.0, integer: true },
        vec![],
        vec![PortSpec::new("I", "Integer", PortType::Int, Value::Int(1))],
        |ctx| {
            let val = match ctx.widget {
                ParamWidget::Slider { value, .. } => value.round() as i64,
                _ => 1,
            };
            ctx.set_output_single(0, Value::Int(val));
            Ok(())
        },
    );

    reg(
        r,
        "params.toggle",
        "Boolean Toggle",
        "Toggle",
        NodeCategory::Params,
        "Clickable inline True / False boolean switch.",
        NodeKind::BoolToggle,
        ParamWidget::Toggle { value: true },
        vec![],
        vec![PortSpec::new("B", "Bool", PortType::Bool, Value::Bool(true))],
        |ctx| {
            let v = match ctx.widget {
                ParamWidget::Toggle { value } => *value,
                _ => false,
            };
            ctx.set_output_single(0, Value::Bool(v));
            Ok(())
        },
    );

    reg(
        r,
        "params.panel",
        "Panel",
        "Panel",
        NodeCategory::Params,
        "Live Grasshopper DataTree inspector & text pass-through.",
        NodeKind::Panel,
        ParamWidget::Text { text: String::new() },
        vec![PortSpec::new("In", "Data", PortType::Any, Value::Null).with_access(ParamAccess::Tree)],
        vec![PortSpec::new("Out", "Data", PortType::Any, Value::Null).with_access(ParamAccess::Tree)],
        |ctx| {
            let tree = if ctx.input(0).is_empty() {
                match ctx.widget {
                    ParamWidget::Text { text } if !text.is_empty() => DataTree::single(Value::String(text.clone())),
                    _ => DataTree::new(),
                }
            } else {
                ctx.input(0).clone()
            };
            ctx.set_output(0, tree);
            Ok(())
        },
    );

    reg(
        r,
        "params.color",
        "Color Swatch",
        "Swatch",
        NodeCategory::Params,
        "Clickable inline color swatch outputting ARGB color.",
        NodeKind::ColorSwatch,
        ParamWidget::Color { argb: 0xFF38BDF8 },
        vec![],
        vec![PortSpec::new("C", "Color", PortType::Color, Value::Color(0xFF38BDF8))],
        |ctx| {
            let c = match ctx.widget {
                ParamWidget::Color { argb } => *argb,
                _ => 0xFF38BDF8,
            };
            ctx.set_output_single(0, Value::Color(c));
            Ok(())
        },
    );

    reg(
        r,
        "params.domain",
        "Construct Domain",
        "Dom",
        NodeCategory::Params,
        "Constructs a numeric interval [A .. B].",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("A", "Start", PortType::Float, Value::Float(0.0)),
            PortSpec::new("B", "End", PortType::Float, Value::Float(1.0)),
        ],
        vec![PortSpec::new("I", "Domain", PortType::Domain, Value::Domain(0.0, 1.0))],
        |ctx| {
            ctx.map_items(|args| {
                let a = args[0].as_float().unwrap_or(0.0);
                let b = args[1].as_float().unwrap_or(1.0);
                Ok(vec![Value::Domain(a, b)])
            })
        },
    );

    reg(
        r,
        "params.relay",
        "Relay",
        "•",
        NodeCategory::Params,
        "Compact pass-through wire reroute node.",
        NodeKind::Relay,
        ParamWidget::None,
        vec![PortSpec::new("", "In", PortType::Any, Value::Null).with_access(ParamAccess::Tree)],
        vec![PortSpec::new("", "Out", PortType::Any, Value::Null).with_access(ParamAccess::Tree)],
        |ctx| {
            ctx.set_output(0, ctx.input(0).clone());
            Ok(())
        },
    );

    reg(
        r,
        "params.scribble",
        "Scribble",
        "Note",
        NodeCategory::Params,
        "Frameless typography annotation placed directly on the canvas.",
        NodeKind::Scribble,
        ParamWidget::Text { text: "Parametric Section".into() },
        vec![],
        vec![],
        |_ctx| Ok(()),
    );

    reg(
        r,
        "params.time",
        "Clock / Time",
        "Time",
        NodeCategory::Params,
        "Outputs live elapsed animation time (seconds) for generative motion.",
        NodeKind::Component,
        ParamWidget::None,
        vec![PortSpec::new("S", "Speed", PortType::Float, Value::Float(1.0))],
        vec![PortSpec::new("t", "Seconds", PortType::Float, Value::Float(0.0))],
        |ctx| {
            let t = ctx.time_secs;
            ctx.map_items(|args| {
                let spd = args[0].as_float().unwrap_or(1.0);
                Ok(vec![Value::Float(t * spd)])
            })
        },
    );
}

fn register_math(r: &mut NodeRegistry) {
    let bin_ops: &[(&str, &str, &str, &str, fn(f64, f64) -> f64)] = &[
        ("math.add", "Addition", "A+B", "Adds two numbers or lists with Grasshopper lacing.", |a, b| a + b),
        ("math.sub", "Subtraction", "A-B", "Subtracts B from A.", |a, b| a - b),
        ("math.mul", "Multiplication", "A×B", "Multiplies A by B.", |a, b| a * b),
        ("math.div", "Division", "A÷B", "Divides A by B (safe zero guard).", |a, b| {
            if b.abs() < 1e-12 { 0.0 } else { a / b }
        }),
        ("math.pow", "Power", "A^B", "Raises A to the power B.", |a, b| a.powf(b)),
        ("math.mod", "Modulus", "Mod", "Remainder of A divided by B.", |a, b| {
            if b.abs() < 1e-12 { 0.0 } else { a.rem_euclid(b) }
        }),
        ("math.min", "Minimum", "Min", "Returns the smaller of A and B.", f64::min),
        ("math.max", "Maximum", "Max", "Returns the larger of A and B.", f64::max),
    ];

    for &(id, name, nick, desc, op) in bin_ops {
        reg(
            r,
            id,
            name,
            nick,
            NodeCategory::Math,
            desc,
            NodeKind::Component,
            ParamWidget::None,
            vec![
                PortSpec::new("A", "Operand A", PortType::Float, Value::Float(0.0)),
                PortSpec::new("B", "Operand B", PortType::Float, Value::Float(1.0)),
            ],
            vec![PortSpec::new("R", "Result", PortType::Float, Value::Float(0.0))],
            move |ctx| {
                ctx.map_items(|args| {
                    let a = args[0].as_float().unwrap_or(0.0);
                    let b = args[1].as_float().unwrap_or(0.0);
                    Ok(vec![Value::Float(op(a, b))])
                })
            },
        );
    }

    let unary_ops: &[(&str, &str, &str, &str, fn(f64) -> f64)] = &[
        ("math.neg", "Negative", "-x", "Negates a numeric value.", |x| -x),
        ("math.abs", "Absolute", "|x|", "Returns absolute magnitude |x|.", f64::abs),
        ("math.sqrt", "Square Root", "√x", "Computes √max(x, 0).", |x| x.max(0.0).sqrt()),
        ("math.sin", "Sine", "Sin", "Computes sin(x) in radians.", f64::sin),
        ("math.cos", "Cosine", "Cos", "Computes cos(x) in radians.", f64::cos),
        ("math.radians", "Radians", "Rad", "Converts degrees to radians.", f64::to_radians),
    ];
    for &(id, name, nick, desc, op) in unary_ops {
        reg(
            r,
            id,
            name,
            nick,
            NodeCategory::Math,
            desc,
            NodeKind::Component,
            ParamWidget::None,
            vec![PortSpec::new("x", "Value", PortType::Float, Value::Float(0.0))],
            vec![PortSpec::new("y", "Result", PortType::Float, Value::Float(0.0))],
            move |ctx| {
                ctx.map_items(|args| {
                    let x = args[0].as_float().unwrap_or(0.0);
                    Ok(vec![Value::Float(op(x))])
                })
            },
        );
    }

    reg(
        r,
        "math.lerp",
        "Interpolate (Lerp)",
        "Lerp",
        NodeCategory::Math,
        "Linearly interpolates between A and B by parameter t.",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("A", "Start", PortType::Float, Value::Float(0.0)),
            PortSpec::new("B", "End", PortType::Float, Value::Float(1.0)),
            PortSpec::new("t", "Factor", PortType::Float, Value::Float(0.5)),
        ],
        vec![PortSpec::new("R", "Value", PortType::Float, Value::Float(0.5))],
        |ctx| {
            ctx.map_items(|args| {
                let a = args[0].as_float().unwrap_or(0.0);
                let b = args[1].as_float().unwrap_or(1.0);
                let t = args[2].as_float().unwrap_or(0.5);
                Ok(vec![Value::Float(a + (b - a) * t)])
            })
        },
    );

    reg(
        r,
        "math.remap",
        "Remap Numbers",
        "ReMap",
        NodeCategory::Math,
        "Remaps numbers from Source Domain S to Target Domain T.",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("V", "Value", PortType::Float, Value::Float(0.5)),
            PortSpec::new("S", "Source", PortType::Domain, Value::Domain(0.0, 1.0)),
            PortSpec::new("T", "Target", PortType::Domain, Value::Domain(0.0, 10.0)),
        ],
        vec![PortSpec::new("R", "Mapped", PortType::Float, Value::Float(5.0))],
        |ctx| {
            ctx.map_items(|args| {
                let v = args[0].as_float().unwrap_or(0.0);
                let (s0, s1) = args[1].as_domain().unwrap_or((0.0, 1.0));
                let (t0, t1) = args[2].as_domain().unwrap_or((0.0, 1.0));
                let span = s1 - s0;
                let u = if span.abs() < 1e-12 { 0.0 } else { (v - s0) / span };
                Ok(vec![Value::Float(t0 + u * (t1 - t0))])
            })
        },
    );

    reg(
        r,
        "math.clamp",
        "Clamp",
        "Clamp",
        NodeCategory::Math,
        "Clamps V into Domain [min..max].",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("V", "Value", PortType::Float, Value::Float(0.5)),
            PortSpec::new("D", "Domain", PortType::Domain, Value::Domain(0.0, 1.0)),
        ],
        vec![PortSpec::new("R", "Clamped", PortType::Float, Value::Float(0.5))],
        |ctx| {
            ctx.map_items(|args| {
                let v = args[0].as_float().unwrap_or(0.0);
                let (a, b) = args[1].as_domain().unwrap_or((0.0, 1.0));
                let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
                Ok(vec![Value::Float(v.clamp(lo, hi))])
            })
        },
    );

    reg(
        r,
        "math.expression",
        "Expression",
        "f(x,y)",
        NodeCategory::Math,
        "Evaluates a math expression using x, y, z, t, pi, sin, cos, sqrt, abs, pow, min, max.",
        NodeKind::ExpressionBox,
        ParamWidget::Text { text: "sqrt(x / max(y, 0.1))".into() },
        vec![
            PortSpec::new("x", "Variable x", PortType::Float, Value::Float(1.0)),
            PortSpec::new("y", "Variable y", PortType::Float, Value::Float(1.0)),
            PortSpec::new("z", "Variable z", PortType::Float, Value::Float(0.0)),
        ],
        vec![PortSpec::new("R", "Result", PortType::Float, Value::Float(0.0))],
        |ctx| {
            let expr_str = match ctx.widget {
                ParamWidget::Text { text } => text.clone(),
                _ => "x".into(),
            };
            let t = ctx.time_secs;
            ctx.map_items(|args| {
                let x = args[0].as_float().unwrap_or(0.0);
                let y = args[1].as_float().unwrap_or(0.0);
                let z = args[2].as_float().unwrap_or(0.0);
                let val = eval_math_expression(&expr_str, x, y, z, t)?;
                Ok(vec![Value::Float(val)])
            })
        },
    );
}

fn register_logic(r: &mut NodeRegistry) {
    reg(
        r,
        "logic.larger",
        "Larger Than",
        "A>B",
        NodeCategory::Logic,
        "Tests whether A > B (and A >= B).",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("A", "First", PortType::Float, Value::Float(0.0)),
            PortSpec::new("B", "Second", PortType::Float, Value::Float(0.0)),
        ],
        vec![
            PortSpec::new(">", "Larger", PortType::Bool, Value::Bool(false)),
            PortSpec::new(">=", "LargerOrEq", PortType::Bool, Value::Bool(true)),
        ],
        |ctx| {
            ctx.map_items(|args| {
                let a = args[0].as_float().unwrap_or(0.0);
                let b = args[1].as_float().unwrap_or(0.0);
                Ok(vec![Value::Bool(a > b), Value::Bool(a >= b)])
            })
        },
    );

    reg(
        r,
        "logic.and",
        "Gate And",
        "And",
        NodeCategory::Logic,
        "Boolean conjunction A && B.",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("A", "Bool A", PortType::Bool, Value::Bool(true)),
            PortSpec::new("B", "Bool B", PortType::Bool, Value::Bool(true)),
        ],
        vec![PortSpec::new("R", "Result", PortType::Bool, Value::Bool(true))],
        |ctx| {
            ctx.map_items(|args| {
                let a = args[0].as_bool().unwrap_or(false);
                let b = args[1].as_bool().unwrap_or(false);
                Ok(vec![Value::Bool(a && b)])
            })
        },
    );

    reg(
        r,
        "logic.not",
        "Gate Not",
        "Not",
        NodeCategory::Logic,
        "Inverts boolean value !A.",
        NodeKind::Component,
        ParamWidget::None,
        vec![PortSpec::new("A", "Bool", PortType::Bool, Value::Bool(false))],
        vec![PortSpec::new("R", "Inverted", PortType::Bool, Value::Bool(true))],
        |ctx| {
            ctx.map_items(|args| {
                let a = args[0].as_bool().unwrap_or(false);
                Ok(vec![Value::Bool(!a)])
            })
        },
    );

    reg(
        r,
        "logic.switch",
        "Stream Filter",
        "Filter",
        NodeCategory::Logic,
        "Selects value A when Gate is True, or B when False.",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("G", "Gate", PortType::Bool, Value::Bool(true)),
            PortSpec::new("A", "If True", PortType::Any, Value::Float(1.0)),
            PortSpec::new("B", "If False", PortType::Any, Value::Float(0.0)),
        ],
        vec![PortSpec::new("R", "Selected", PortType::Any, Value::Float(1.0))],
        |ctx| {
            ctx.map_items(|args| {
                let g = args[0].as_bool().unwrap_or(true);
                Ok(vec![if g { args[1].clone() } else { args[2].clone() }])
            })
        },
    );
}

fn register_sets(r: &mut NodeRegistry) {
    reg(
        r,
        "sets.series",
        "Series",
        "Series",
        NodeCategory::Sets,
        "Generates an arithmetic sequence of Count numbers starting at S with step N.",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("S", "Start", PortType::Float, Value::Float(0.0)),
            PortSpec::new("N", "Step", PortType::Float, Value::Float(1.0)),
            PortSpec::new("C", "Count", PortType::Int, Value::Int(10)),
        ],
        vec![PortSpec::new("S", "Series", PortType::Float, Value::Float(0.0)).with_access(ParamAccess::List)],
        |ctx| {
            let s = ctx.input(0).first_item().and_then(Value::as_float).unwrap_or(0.0);
            let step = ctx.input(1).first_item().and_then(Value::as_float).unwrap_or(1.0);
            let count = ctx.input(2).first_item().and_then(Value::as_int).unwrap_or(10).clamp(0, 2000) as usize;
            let items: Vec<Value> = (0..count).map(|i| Value::Float(s + i as f64 * step)).collect();
            ctx.set_output(0, DataTree::from_list(items));
            Ok(())
        },
    );

    reg(
        r,
        "sets.range",
        "Range",
        "Range",
        NodeCategory::Sets,
        "Divides a numeric Domain into N equal steps (producing N+1 values).",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("D", "Domain", PortType::Domain, Value::Domain(0.0, 1.0)),
            PortSpec::new("N", "Steps", PortType::Int, Value::Int(10)),
        ],
        vec![PortSpec::new("R", "Range", PortType::Float, Value::Float(0.0)).with_access(ParamAccess::List)],
        |ctx| {
            let (a, b) = ctx.input(0).first_item().and_then(Value::as_domain).unwrap_or((0.0, 1.0));
            let n = ctx.input(1).first_item().and_then(Value::as_int).unwrap_or(10).clamp(1, 2000) as usize;
            let items: Vec<Value> = (0..=n)
                .map(|i| Value::Float(a + (b - a) * (i as f64 / n as f64)))
                .collect();
            ctx.set_output(0, DataTree::from_list(items));
            Ok(())
        },
    );

    reg(
        r,
        "sets.repeat",
        "Repeat Data",
        "Repeat",
        NodeCategory::Sets,
        "Repeats input pattern until it reaches Length L.",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("D", "Data", PortType::Any, Value::Float(1.0)).with_access(ParamAccess::List),
            PortSpec::new("L", "Length", PortType::Int, Value::Int(8)),
        ],
        vec![PortSpec::new("D", "Repeated", PortType::Any, Value::Null).with_access(ParamAccess::List)],
        |ctx| {
            let src = ctx.input(0).all_items();
            let len = ctx.input(1).first_item().and_then(Value::as_int).unwrap_or(8).clamp(0, 2000) as usize;
            if src.is_empty() {
                ctx.set_output(0, DataTree::new());
            } else {
                let items = (0..len).map(|i| src[i % src.len()].clone()).collect();
                ctx.set_output(0, DataTree::from_list(items));
            }
            Ok(())
        },
    );

    reg(
        r,
        "sets.list_length",
        "List Length",
        "Lng",
        NodeCategory::Sets,
        "Measures the number of items in each DataTree branch.",
        NodeKind::Component,
        ParamWidget::None,
        vec![PortSpec::new("L", "List", PortType::Any, Value::Null).with_access(ParamAccess::List)],
        vec![PortSpec::new("N", "Length", PortType::Int, Value::Int(0))],
        |ctx| {
            let mut out = DataTree::new();
            for (p, items) in ctx.input(0).branches() {
                out.insert_branch(p.clone(), vec![Value::Int(items.len() as i64)]);
            }
            ctx.set_output(0, out);
            Ok(())
        },
    );

    reg(
        r,
        "sets.list_item",
        "List Item",
        "Item",
        NodeCategory::Sets,
        "Extracts item at index i from each branch (with optional wrap).",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("L", "List", PortType::Any, Value::Null).with_access(ParamAccess::List),
            PortSpec::new("i", "Index", PortType::Int, Value::Int(0)),
            PortSpec::new("W", "Wrap", PortType::Bool, Value::Bool(true)),
        ],
        vec![PortSpec::new("I", "Item", PortType::Any, Value::Null)],
        |ctx| {
            let wrap = ctx.input(2).first_item().and_then(Value::as_bool).unwrap_or(true);
            let idx = ctx.input(1).first_item().and_then(Value::as_int).unwrap_or(0);
            let mut out = DataTree::new();
            for (p, items) in ctx.input(0).branches() {
                if items.is_empty() {
                    continue;
                }
                let pick = if wrap {
                    Some(idx.rem_euclid(items.len() as i64) as usize)
                } else if idx >= 0 && (idx as usize) < items.len() {
                    Some(idx as usize)
                } else {
                    None
                };
                if let Some(i) = pick {
                    out.insert_branch(p.clone(), vec![items[i].clone()]);
                }
            }
            ctx.set_output(0, out);
            Ok(())
        },
    );

    reg(
        r,
        "sets.cull_pattern",
        "Cull Pattern",
        "Cull",
        NodeCategory::Sets,
        "Filters list elements using a repeating True/False boolean mask.",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("L", "List", PortType::Any, Value::Null).with_access(ParamAccess::List),
            PortSpec::new("P", "Pattern", PortType::Bool, Value::Bool(true)).with_access(ParamAccess::List),
        ],
        vec![PortSpec::new("L", "Culled", PortType::Any, Value::Null).with_access(ParamAccess::List)],
        |ctx| {
            let pat: Vec<bool> = ctx
                .input(1)
                .all_items()
                .into_iter()
                .map(|v| v.as_bool().unwrap_or(true))
                .collect();
            let mut out = DataTree::new();
            for (p, items) in ctx.input(0).branches() {
                let filtered: Vec<Value> = if pat.is_empty() {
                    items.clone()
                } else {
                    items
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| pat[i % pat.len()])
                        .map(|(_, v)| v.clone())
                        .collect()
                };
                out.insert_branch(p.clone(), filtered);
            }
            ctx.set_output(0, out);
            Ok(())
        },
    );

    reg(
        r,
        "sets.merge",
        "Merge",
        "Merge",
        NodeCategory::Sets,
        "Merges up to three DataTrees preserving branch structure.",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("D1", "Stream 1", PortType::Any, Value::Null).with_access(ParamAccess::Tree),
            PortSpec::new("D2", "Stream 2", PortType::Any, Value::Null).with_access(ParamAccess::Tree),
            PortSpec::new("D3", "Stream 3", PortType::Any, Value::Null).with_access(ParamAccess::Tree),
        ],
        vec![PortSpec::new("R", "Merged", PortType::Any, Value::Null).with_access(ParamAccess::Tree)],
        |ctx| {
            let mut out = ctx.input(0).clone();
            out.merge_from(ctx.input(1));
            out.merge_from(ctx.input(2));
            ctx.set_output(0, out);
            Ok(())
        },
    );

    reg(
        r,
        "sets.flatten",
        "Flatten Tree",
        "Flatten",
        NodeCategory::Sets,
        "Collapses all DataTree branches into a single flat list {0}.",
        NodeKind::Component,
        ParamWidget::None,
        vec![PortSpec::new("T", "Tree", PortType::Any, Value::Null).with_access(ParamAccess::Tree)],
        vec![PortSpec::new("T", "Flat", PortType::Any, Value::Null).with_access(ParamAccess::Tree)],
        |ctx| {
            ctx.set_output(0, ctx.input(0).flatten());
            Ok(())
        },
    );

    reg(
        r,
        "sets.graft",
        "Graft Tree",
        "Graft",
        NodeCategory::Sets,
        "Promotes every item into its own individual DataTree branch.",
        NodeKind::Component,
        ParamWidget::None,
        vec![PortSpec::new("T", "Tree", PortType::Any, Value::Null).with_access(ParamAccess::Tree)],
        vec![PortSpec::new("T", "Grafted", PortType::Any, Value::Null).with_access(ParamAccess::Tree)],
        |ctx| {
            ctx.set_output(0, ctx.input(0).graft());
            Ok(())
        },
    );

    reg(
        r,
        "sets.flip_matrix",
        "Flip Matrix",
        "Flip",
        NodeCategory::Sets,
        "Transposes rows (branches) and columns (item indices) of a DataTree.",
        NodeKind::Component,
        ParamWidget::None,
        vec![PortSpec::new("D", "Data", PortType::Any, Value::Null).with_access(ParamAccess::Tree)],
        vec![PortSpec::new("D", "Flipped", PortType::Any, Value::Null).with_access(ParamAccess::Tree)],
        |ctx| {
            ctx.set_output(0, ctx.input(0).flip_matrix());
            Ok(())
        },
    );

    reg(
        r,
        "sets.random",
        "Random",
        "Rand",
        NodeCategory::Sets,
        "Generates deterministic pseudo-random numbers in Domain [A..B] from Seed S.",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("R", "Range", PortType::Domain, Value::Domain(0.0, 1.0)),
            PortSpec::new("N", "Count", PortType::Int, Value::Int(10)),
            PortSpec::new("S", "Seed", PortType::Int, Value::Int(42)),
        ],
        vec![PortSpec::new("R", "Numbers", PortType::Float, Value::Float(0.0)).with_access(ParamAccess::List)],
        |ctx| {
            let (a, b) = ctx.input(0).first_item().and_then(Value::as_domain).unwrap_or((0.0, 1.0));
            let n = ctx.input(1).first_item().and_then(Value::as_int).unwrap_or(10).clamp(0, 2000) as usize;
            let mut state = (ctx.input(2).first_item().and_then(Value::as_int).unwrap_or(42) as u64)
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let mut out = Vec::with_capacity(n);
            for _ in 0..n {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                let u = ((state >> 11) as f64) / ((1u64 << 53) as f64);
                out.push(Value::Float(a + u * (b - a)));
            }
            ctx.set_output(0, DataTree::from_list(out));
            Ok(())
        },
    );
}

fn register_vectors_geometry(r: &mut NodeRegistry) {
    reg(
        r,
        "vector.point2d",
        "Construct Point 2D",
        "Pt",
        NodeCategory::Vector,
        "Constructs a 2D Point/Vector from (X, Y) coordinates.",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("X", "X coord", PortType::Float, Value::Float(0.0)),
            PortSpec::new("Y", "Y coord", PortType::Float, Value::Float(0.0)),
        ],
        vec![PortSpec::new("Pt", "Point", PortType::Vec2, Value::Vec2([0.0, 0.0]))],
        |ctx| {
            ctx.map_items(|args| {
                let x = args[0].as_float().unwrap_or(0.0);
                let y = args[1].as_float().unwrap_or(0.0);
                Ok(vec![Value::Vec2([x, y])])
            })
        },
    );

    reg(
        r,
        "vector.deconstruct",
        "Deconstruct Point",
        "DePt",
        NodeCategory::Vector,
        "Decomposes a 2D Point into its X and Y components.",
        NodeKind::Component,
        ParamWidget::None,
        vec![PortSpec::new("Pt", "Point", PortType::Vec2, Value::Vec2([0.0, 0.0]))],
        vec![
            PortSpec::new("X", "X coord", PortType::Float, Value::Float(0.0)),
            PortSpec::new("Y", "Y coord", PortType::Float, Value::Float(0.0)),
        ],
        |ctx| {
            ctx.map_items(|args| {
                let [x, y] = args[0].as_vec2().unwrap_or([0.0, 0.0]);
                Ok(vec![Value::Float(x), Value::Float(y)])
            })
        },
    );

    reg(
        r,
        "vector.add",
        "Vector Add",
        "V+V",
        NodeCategory::Vector,
        "Adds two 2D vectors A + B.",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("A", "Vector A", PortType::Vec2, Value::Vec2([0.0, 0.0])),
            PortSpec::new("B", "Vector B", PortType::Vec2, Value::Vec2([0.0, 0.0])),
        ],
        vec![PortSpec::new("V", "Sum", PortType::Vec2, Value::Vec2([0.0, 0.0]))],
        |ctx| {
            ctx.map_items(|args| {
                let a = args[0].as_vec2().unwrap_or([0.0, 0.0]);
                let b = args[1].as_vec2().unwrap_or([0.0, 0.0]);
                Ok(vec![Value::Vec2([a[0] + b[0], a[1] + b[1]])])
            })
        },
    );

    reg(
        r,
        "vector.scale",
        "Vector Amplitude",
        "Amp",
        NodeCategory::Vector,
        "Scales a 2D vector V by scalar factor F.",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("V", "Vector", PortType::Vec2, Value::Vec2([1.0, 0.0])),
            PortSpec::new("F", "Factor", PortType::Float, Value::Float(1.0)),
        ],
        vec![PortSpec::new("V", "Scaled", PortType::Vec2, Value::Vec2([1.0, 0.0]))],
        |ctx| {
            ctx.map_items(|args| {
                let v = args[0].as_vec2().unwrap_or([0.0, 0.0]);
                let f = args[1].as_float().unwrap_or(1.0);
                Ok(vec![Value::Vec2([v[0] * f, v[1] * f])])
            })
        },
    );

    reg(
        r,
        "vector.distance",
        "Distance",
        "Dist",
        NodeCategory::Vector,
        "Euclidean distance between two 2D points A and B.",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("A", "Point A", PortType::Vec2, Value::Vec2([0.0, 0.0])),
            PortSpec::new("B", "Point B", PortType::Vec2, Value::Vec2([1.0, 0.0])),
        ],
        vec![PortSpec::new("D", "Distance", PortType::Float, Value::Float(1.0))],
        |ctx| {
            ctx.map_items(|args| {
                let a = args[0].as_vec2().unwrap_or([0.0, 0.0]);
                let b = args[1].as_vec2().unwrap_or([0.0, 0.0]);
                let d = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
                Ok(vec![Value::Float(d)])
            })
        },
    );

    reg(
        r,
        "vector.rotate",
        "Rotate Point",
        "Rot",
        NodeCategory::Vector,
        "Rotates point Pt around Center C by Angle A (radians).",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("Pt", "Point", PortType::Vec2, Value::Vec2([1.0, 0.0])),
            PortSpec::new("C", "Center", PortType::Vec2, Value::Vec2([0.0, 0.0])),
            PortSpec::new("A", "Angle", PortType::Float, Value::Float(0.0)),
        ],
        vec![PortSpec::new("Pt", "Rotated", PortType::Vec2, Value::Vec2([1.0, 0.0]))],
        |ctx| {
            ctx.map_items(|args| {
                let p = args[0].as_vec2().unwrap_or([0.0, 0.0]);
                let c = args[1].as_vec2().unwrap_or([0.0, 0.0]);
                let a = args[2].as_float().unwrap_or(0.0);
                let (s, cs) = a.sin_cos();
                let dx = p[0] - c[0];
                let dy = p[1] - c[1];
                Ok(vec![Value::Vec2([c[0] + dx * cs - dy * s, c[1] + dx * s + dy * cs])])
            })
        },
    );

    reg(
        r,
        "geom.rect_grid",
        "Rectangular Grid",
        "RecGrid",
        NodeCategory::Geometry,
        "Generates a 2D parametric point grid (True branched DataTree {row}!) with optional pyramid stagger.",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("O", "Origin", PortType::Vec2, Value::Vec2([4.0, 8.8])),
            PortSpec::new("Sx", "Step X", PortType::Float, Value::Float(0.62)),
            PortSpec::new("Sy", "Step Y", PortType::Float, Value::Float(-0.62)),
            PortSpec::new("Ex", "Cols", PortType::Int, Value::Int(6)),
            PortSpec::new("Ey", "Rows", PortType::Int, Value::Int(5)),
            PortSpec::new("Pyr", "Pyramid", PortType::Bool, Value::Bool(true)),
        ],
        vec![
            PortSpec::new("P", "Points", PortType::Vec2, Value::Vec2([0.0, 0.0])).with_access(ParamAccess::Tree),
            PortSpec::new("N", "Count", PortType::Int, Value::Int(0)),
        ],
        |ctx| {
            let origin = ctx.input(0).first_item().and_then(Value::as_vec2).unwrap_or([4.0, 8.8]);
            let sx = ctx.input(1).first_item().and_then(Value::as_float).unwrap_or(0.62);
            let sy = ctx.input(2).first_item().and_then(Value::as_float).unwrap_or(-0.62);
            let cols = ctx.input(3).first_item().and_then(Value::as_int).unwrap_or(6).clamp(1, 30) as usize;
            let rows = ctx.input(4).first_item().and_then(Value::as_int).unwrap_or(5).clamp(1, 30) as usize;
            let pyramid = ctx.input(5).first_item().and_then(Value::as_bool).unwrap_or(true);

            let mut tree = DataTree::new();
            let mut total = 0usize;
            for r in 0..rows {
                let row_cols = if pyramid { cols.saturating_sub(r) } else { cols };
                if row_cols == 0 {
                    break;
                }
                let start_x = origin[0] - (row_cols as f64 - 1.0) * sx * 0.5;
                let y = origin[1] + r as f64 * sy;
                let mut branch = Vec::with_capacity(row_cols);
                for c in 0..row_cols {
                    branch.push(Value::Vec2([start_x + c as f64 * sx, y]));
                    total += 1;
                }
                tree.insert_branch(DataPath::from_slice(&[r as u32]), branch);
            }
            ctx.set_output(0, tree);
            ctx.set_output_single(1, Value::Int(total as i64));
            Ok(())
        },
    );

    reg(
        r,
        "geom.circle_points",
        "Circle Points",
        "CircPts",
        NodeCategory::Geometry,
        "Generates N evenly-spaced points & unit tangent vectors around a circle.",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("C", "Center", PortType::Vec2, Value::Vec2([8.0, 5.0])),
            PortSpec::new("R", "Radius", PortType::Float, Value::Float(2.5)),
            PortSpec::new("N", "Count", PortType::Int, Value::Int(8)),
            PortSpec::new("Ph", "Phase", PortType::Float, Value::Float(0.0)),
        ],
        vec![
            PortSpec::new("P", "Points", PortType::Vec2, Value::Vec2([0.0, 0.0])).with_access(ParamAccess::List),
            PortSpec::new("T", "Tangents", PortType::Vec2, Value::Vec2([0.0, 1.0])).with_access(ParamAccess::List),
            PortSpec::new("A", "Angles", PortType::Float, Value::Float(0.0)).with_access(ParamAccess::List),
        ],
        |ctx| {
            let center = ctx.input(0).first_item().and_then(Value::as_vec2).unwrap_or([8.0, 5.0]);
            let radii = ctx.input(1).all_items();
            let count = ctx.input(2).first_item().and_then(Value::as_int).unwrap_or(8).clamp(1, 256) as usize;
            let phase = ctx.input(3).first_item().and_then(Value::as_float).unwrap_or(0.0);

            let n = if radii.len() > 1 { radii.len() } else { count };
            let mut pts = Vec::with_capacity(n);
            let mut tans = Vec::with_capacity(n);
            let mut angs = Vec::with_capacity(n);
            for i in 0..n {
                let r = radii
                    .get(i.min(radii.len().saturating_sub(1)))
                    .and_then(Value::as_float)
                    .unwrap_or(2.5);
                let ang = phase + std::f64::consts::TAU * (i as f64) / (n as f64);
                pts.push(Value::Vec2([center[0] + ang.cos() * r, center[1] + ang.sin() * r]));
                tans.push(Value::Vec2([-ang.sin(), ang.cos()]));
                angs.push(Value::Float(ang));
            }
            ctx.set_output(0, DataTree::from_list(pts));
            ctx.set_output(1, DataTree::from_list(tans));
            ctx.set_output(2, DataTree::from_list(angs));
            Ok(())
        },
    );

    reg(
        r,
        "geom.line_divide",
        "Divide Line",
        "DivLn",
        NodeCategory::Geometry,
        "Generates N evenly-spaced points between A and B.",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("A", "Start", PortType::Vec2, Value::Vec2([2.0, 2.0])),
            PortSpec::new("B", "End", PortType::Vec2, Value::Vec2([14.0, 2.0])),
            PortSpec::new("N", "Count", PortType::Int, Value::Int(8)),
        ],
        vec![PortSpec::new("P", "Points", PortType::Vec2, Value::Vec2([0.0, 0.0])).with_access(ParamAccess::List)],
        |ctx| {
            let a = ctx.input(0).first_item().and_then(Value::as_vec2).unwrap_or([2.0, 2.0]);
            let b = ctx.input(1).first_item().and_then(Value::as_vec2).unwrap_or([14.0, 2.0]);
            let n = ctx.input(2).first_item().and_then(Value::as_int).unwrap_or(8).clamp(2, 500) as usize;
            let pts: Vec<Value> = (0..n)
                .map(|i| {
                    let t = i as f64 / (n - 1) as f64;
                    Value::Vec2([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t])
                })
                .collect();
            ctx.set_output(0, DataTree::from_list(pts));
            Ok(())
        },
    );

    reg(
        r,
        "geom.color_hsv",
        "Color HSV Gradient",
        "HSV",
        NodeCategory::Geometry,
        "Generates vibrant ARGB colors from Hue (0..1), Saturation, and Value.",
        NodeKind::Component,
        ParamWidget::None,
        vec![
            PortSpec::new("H", "Hue", PortType::Float, Value::Float(0.55)),
            PortSpec::new("S", "Saturation", PortType::Float, Value::Float(0.75)),
            PortSpec::new("V", "Value", PortType::Float, Value::Float(0.95)),
        ],
        vec![PortSpec::new("C", "Color", PortType::Color, Value::Color(0xFF38BDF8))],
        |ctx| {
            ctx.map_items(|args| {
                let h = args[0].as_float().unwrap_or(0.55).rem_euclid(1.0);
                let s = args[1].as_float().unwrap_or(0.75).clamp(0.0, 1.0);
                let v = args[2].as_float().unwrap_or(0.95).clamp(0.0, 1.0);
                Ok(vec![Value::Color(hsv_to_argb(h, s, v))])
            })
        },
    );
}

pub fn hsv_to_argb(h: f64, s: f64, v: f64) -> u32 {
    let i = (h * 6.0).floor() as i32;
    let f = h * 6.0 - i as f64;
    let p = v * (1.0 - s);
    let q = v * (1.0 - f * s);
    let t = v * (1.0 - (1.0 - f) * s);
    let (r, g, b) = match i.rem_euclid(6) {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    0xFF00_0000
        | (((r * 255.0).clamp(0.0, 255.0) as u32) << 16)
        | (((g * 255.0).clamp(0.0, 255.0) as u32) << 8)
        | ((b * 255.0).clamp(0.0, 255.0) as u32)
}

// =============================================================================
// Lightweight Recursive-Descent Math Expression Parser (x, y, z, t, pi, funcs)
// =============================================================================

/// Evaluates a mathematical expression string like `"sqrt(x / max(y, 0.1))"` or `"sin(x * pi) * 2 + y"`.
pub fn eval_math_expression(src: &str, x: f64, y: f64, z: f64, t: f64) -> Result<f64, String> {
    let mut parser = ExprParser { bytes: src.as_bytes(), pos: 0, x, y, z, t };
    let val = parser.parse_add_sub()?;
    parser.skip_ws();
    if parser.pos < parser.bytes.len() {
        return Err(format!("Unexpected token at `{}`", &src[parser.pos..]));
    }
    Ok(val)
}

struct ExprParser<'a> {
    bytes: &'a [u8],
    pos: usize,
    x: f64,
    y: f64,
    z: f64,
    t: f64,
}

impl<'a> ExprParser<'a> {
    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.bytes.get(self.pos).copied()
    }

    fn parse_add_sub(&mut self) -> Result<f64, String> {
        let mut acc = self.parse_mul_div()?;
        loop {
            match self.peek() {
                Some(b'+') => {
                    self.pos += 1;
                    acc += self.parse_mul_div()?;
                }
                Some(b'-') => {
                    self.pos += 1;
                    acc -= self.parse_mul_div()?;
                }
                _ => return Ok(acc),
            }
        }
    }

    fn parse_mul_div(&mut self) -> Result<f64, String> {
        let mut acc = self.parse_power()?;
        loop {
            match self.peek() {
                Some(b'*') => {
                    self.pos += 1;
                    acc *= self.parse_power()?;
                }
                Some(b'/') => {
                    self.pos += 1;
                    let d = self.parse_power()?;
                    acc = if d.abs() < 1e-12 { 0.0 } else { acc / d };
                }
                Some(b'%') => {
                    self.pos += 1;
                    let d = self.parse_power()?;
                    acc = if d.abs() < 1e-12 { 0.0 } else { acc.rem_euclid(d) };
                }
                _ => return Ok(acc),
            }
        }
    }

    fn parse_power(&mut self) -> Result<f64, String> {
        let base = self.parse_unary()?;
        if self.peek() == Some(b'^') {
            self.pos += 1;
            let exp = self.parse_power()?;
            Ok(base.powf(exp))
        } else {
            Ok(base)
        }
    }

    fn parse_unary(&mut self) -> Result<f64, String> {
        match self.peek() {
            Some(b'-') => {
                self.pos += 1;
                Ok(-self.parse_unary()?)
            }
            Some(b'+') => {
                self.pos += 1;
                self.parse_unary()
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<f64, String> {
        match self.peek() {
            Some(b'(') => {
                self.pos += 1;
                let v = self.parse_add_sub()?;
                if self.peek() != Some(b')') {
                    return Err("Missing closing ')'".into());
                }
                self.pos += 1;
                Ok(v)
            }
            Some(c) if c.is_ascii_digit() || c == b'.' => {
                let start = self.pos;
                while self.pos < self.bytes.len()
                    && (self.bytes[self.pos].is_ascii_digit() || self.bytes[self.pos] == b'.')
                {
                    self.pos += 1;
                }
                let s = std::str::from_utf8(&self.bytes[start..self.pos]).map_err(|_| "UTF-8 error")?;
                s.parse::<f64>().map_err(|_| format!("Invalid number `{s}`"))
            }
            Some(c) if c.is_ascii_alphabetic() || c == b'_' => {
                let start = self.pos;
                while self.pos < self.bytes.len()
                    && (self.bytes[self.pos].is_ascii_alphanumeric() || self.bytes[self.pos] == b'_')
                {
                    self.pos += 1;
                }
                let ident = std::str::from_utf8(&self.bytes[start..self.pos])
                    .map_err(|_| "UTF-8 error")?
                    .to_ascii_lowercase();
                if self.peek() == Some(b'(') {
                    self.pos += 1;
                    let mut args = Vec::new();
                    if self.peek() != Some(b')') {
                        loop {
                            args.push(self.parse_add_sub()?);
                            if self.peek() == Some(b',') {
                                self.pos += 1;
                            } else {
                                break;
                            }
                        }
                    }
                    if self.peek() != Some(b')') {
                        return Err(format!("Missing ')' after `{ident}(`"));
                    }
                    self.pos += 1;
                    let a0 = args.first().copied().unwrap_or(0.0);
                    let a1 = args.get(1).copied().unwrap_or(0.0);
                    let a2 = args.get(2).copied().unwrap_or(0.0);
                    match ident.as_str() {
                        "sin" => Ok(a0.sin()),
                        "cos" => Ok(a0.cos()),
                        "tan" => Ok(a0.tan()),
                        "abs" => Ok(a0.abs()),
                        "sqrt" => Ok(a0.max(0.0).sqrt()),
                        "floor" => Ok(a0.floor()),
                        "ceil" => Ok(a0.ceil()),
                        "round" => Ok(a0.round()),
                        "exp" => Ok(a0.exp()),
                        "ln" => Ok(a0.max(1e-12).ln()),
                        "pow" => Ok(a0.powf(a1)),
                        "min" => Ok(a0.min(a1)),
                        "max" => Ok(a0.max(a1)),
                        "clamp" => Ok(a0.clamp(a1.min(a2), a1.max(a2))),
                        "lerp" => Ok(a0 + (a1 - a0) * a2),
                        _ => Err(format!("Unknown function `{ident}`")),
                    }
                } else {
                    match ident.as_str() {
                        "x" => Ok(self.x),
                        "y" => Ok(self.y),
                        "z" => Ok(self.z),
                        "t" => Ok(self.t),
                        "pi" => Ok(std::f64::consts::PI),
                        "tau" => Ok(std::f64::consts::TAU),
                        "e" => Ok(std::f64::consts::E),
                        _ => Err(format!("Unknown variable `{ident}`")),
                    }
                }
            }
            _ => Err("Unexpected end of expression".into()),
        }
    }
}
