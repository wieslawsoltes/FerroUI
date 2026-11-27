//! Scene nodes: the primitives Osprey Native encodes (`ui::Encode::numbers`), decoded into types
//! and checked whole (docs/osprey-native-host.md, section 5). Each primitive is one array, its
//! type code first:
//!
//! | Code | Primitive | Values after the code |
//! |---|---|---|
//! | 0 | rectangle | x, y (centre), w, h, angle in radians, colour, depth |
//! | 1 | disc | x, y, radius, colour, depth |
//! | 2 | line strip | pair count, x0, y0, ..., colour, stroke width, depth |
//! | 3 | sprite | atlas index, x, y, w, h, angle, tint colour, depth |
//! | 4 | glyphs | text, x, y, size, colour, depth |
//! | 5 | path | steps, outline, paint, transform, depth |
//! | 6 | lettering | text, x, y, size, colour, depth, anchor, weight |
//!
//! A colour is one integer packed `0xRRGGBBAA`. The drawing is in `surface.rs`.

use serde_json::Value;

use crate::boundary::Streamed;

/// An sRGB colour with straight alpha, each channel from 0 to 1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tint {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

impl Tint {
    /// A colour packed `0xRRGGBBAA`.
    pub fn packed(colour: i64) -> Self {
        let byte = |shift: u32| ((colour >> shift) & 0xFF) as f64 / 255.0;
        Self {
            r: byte(24),
            g: byte(16),
            b: byte(8),
            a: byte(0),
        }
    }

    /// A colour from hue, saturation and lightness (each 0 to 1).
    pub fn hsl(h: f64, s: f64, l: f64, a: f64) -> Self {
        let chroma = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let sector = (h.rem_euclid(1.0)) * 6.0;
        let x = chroma * (1.0 - (sector % 2.0 - 1.0).abs());
        let (r, g, b) = match sector as u32 {
            0 => (chroma, x, 0.0),
            1 => (x, chroma, 0.0),
            2 => (0.0, chroma, x),
            3 => (0.0, x, chroma),
            4 => (x, 0.0, chroma),
            _ => (chroma, 0.0, x),
        };
        let m = l - chroma / 2.0;
        Self {
            r: r + m,
            g: g + m,
            b: b + m,
            a,
        }
    }
}

/// One step of a path, in the order a path builder takes them.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    Move(f64, f64),
    Line(f64, f64),
    /// End point, then the control point.
    Quad(f64, f64, f64, f64),
    /// End point, then the two control points.
    Cubic(f64, f64, f64, f64, f64, f64),
    /// Radii, rotation in degrees, large arc, sweep, end point.
    Arc {
        rx: f64,
        ry: f64,
        rotation: f64,
        large: bool,
        sweep: bool,
        x: f64,
        y: f64,
    },
    /// Corners and whether the polygon closes.
    Polygon {
        closed: bool,
        corners: Vec<(f64, f64)>,
    },
    Close,
}

/// How a path is painted: filled, or stroked with a width, bevel joins and dashes.
#[derive(Debug, Clone, PartialEq)]
pub enum Outline {
    Fill,
    Stroke {
        width: f64,
        bevel: bool,
        dashes: Vec<f64>,
    },
}

/// A path's paint: one colour, or a two-stop linear gradient (angle in degrees).
#[derive(Debug, Clone, PartialEq)]
pub enum Paint {
    Solid(Tint),
    Linear {
        angle: f64,
        from: Tint,
        from_at: f64,
        to: Tint,
        to_at: f64,
        oklab: bool,
    },
}

/// One decoded primitive and its depth.
#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    Rect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        angle: f64,
        colour: i64,
    },
    Disc {
        x: f64,
        y: f64,
        radius: f64,
        colour: i64,
    },
    Strip {
        points: Vec<(f64, f64)>,
        colour: i64,
        stroke: f64,
    },
    Sprite {
        index: i64,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        angle: f64,
        colour: i64,
    },
    Glyphs {
        text: String,
        x: f64,
        y: f64,
        size: f64,
        colour: i64,
    },
    Path {
        steps: Vec<Step>,
        outline: Outline,
        paint: Paint,
        transform: Option<(f64, f64, f64)>,
    },
    Lettering {
        text: String,
        x: f64,
        y: f64,
        size: f64,
        colour: i64,
        anchor: i64,
        weight: f64,
    },
}

/// A primitive with the depth that orders it among the others (back first).
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    pub primitive: Primitive,
    pub depth: i64,
}

/// A decoded scene node: its unit space, zoom, atlas and shapes in drawing order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scene {
    pub width: f64,
    pub height: f64,
    pub zoom: f64,
    pub atlas: Vec<String>,
    pub shapes: Vec<Shape>,
}

impl Scene {
    /// Whether the scene is in window pixels (zero width and height): no clip and no scale.
    pub fn in_window_pixels(&self) -> bool {
        self.width == 0.0 && self.height == 0.0
    }
}

/// Checks a scene's `shapes` member: every primitive decodes.
pub fn check_shapes(value: &Value) -> Result<(), String> {
    let items = value.as_array().ok_or("shapes is not an array")?;
    for (index, item) in items.iter().enumerate() {
        shape(item).map_err(|problem| format!("primitive {index}: {problem}"))?;
    }
    Ok(())
}

/// Decodes a checked scene node, adding the shapes the application streamed for it, and sorts
/// every shape by depth, stably (so shapes of one depth keep the order they came in).
pub fn decode(node: &Value, streamed: &[Streamed]) -> Result<Scene, String> {
    let real = |name: &str| {
        node.get(name)
            .and_then(Value::as_f64)
            .ok_or_else(|| format!("scene has no {name}"))
    };
    let atlas = node
        .get("atlas")
        .and_then(Value::as_array)
        .map(|names| {
            names
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let mut shapes = node
        .get("shapes")
        .and_then(Value::as_array)
        .ok_or("scene has no shapes")?
        .iter()
        .map(shape)
        .collect::<Result<Vec<_>, _>>()?;
    shapes.extend(
        streamed
            .iter()
            .map(streamed_shape)
            .collect::<Result<Vec<_>, _>>()?,
    );
    shapes.sort_by_key(|shape| shape.depth);
    Ok(Scene {
        width: real("width")?,
        height: real("height")?,
        zoom: real("zoom")?,
        atlas,
        shapes,
    })
}

/// A streamed rect, disc or sprite as a shape.
fn streamed_shape(item: &Streamed) -> Result<Shape, String> {
    let [a, b, c, d, e] = item.values;
    let primitive = match item.code {
        0 => Primitive::Rect {
            x: a,
            y: b,
            w: c,
            h: d,
            angle: e,
            colour: item.colour,
        },
        1 => Primitive::Disc {
            x: a,
            y: b,
            radius: c,
            colour: item.colour,
        },
        3 => Primitive::Sprite {
            index: item.index,
            x: a,
            y: b,
            w: c,
            h: d,
            angle: e,
            colour: item.colour,
        },
        code => {
            return Err(format!(
                "a streamed shape has the code {code}; only rects, discs and sprites stream"
            ))
        }
    };
    Ok(Shape {
        primitive,
        depth: item.depth,
    })
}

/// Reads the values of one primitive array in order.
struct Reader<'a> {
    items: &'a [Value],
    at: usize,
}

impl<'a> Reader<'a> {
    fn next(&mut self) -> Result<&'a Value, String> {
        let item = self.items.get(self.at).ok_or("too few values")?;
        self.at += 1;
        Ok(item)
    }

    fn real(&mut self) -> Result<f64, String> {
        self.next()?
            .as_f64()
            .filter(|value| value.is_finite())
            .ok_or_else(|| "a value is not a finite number".into())
    }

    fn whole(&mut self) -> Result<i64, String> {
        self.next()?
            .as_i64()
            .ok_or_else(|| "a value is not an integer".into())
    }

    fn text(&mut self) -> Result<String, String> {
        self.next()?
            .as_str()
            .map(str::to_owned)
            .ok_or_else(|| "a value is not text".into())
    }

    fn list(&mut self) -> Result<&'a [Value], String> {
        self.next()?
            .as_array()
            .map(Vec::as_slice)
            .ok_or_else(|| "a value is not an array".into())
    }

    fn flag(&mut self) -> Result<bool, String> {
        match self.whole()? {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(format!("a flag is {other}, not 0 or 1")),
        }
    }

    fn done(&self) -> Result<(), String> {
        match self.at == self.items.len() {
            true => Ok(()),
            false => Err(format!(
                "{} values where {} were expected",
                self.items.len(),
                self.at
            )),
        }
    }
}

/// Decodes one primitive array.
fn shape(value: &Value) -> Result<Shape, String> {
    let items = value.as_array().ok_or("a primitive is not an array")?;
    let mut read = Reader { items, at: 0 };
    let code = read.whole()?;
    let primitive = match code {
        0 => Primitive::Rect {
            x: read.real()?,
            y: read.real()?,
            w: read.real()?,
            h: read.real()?,
            angle: read.real()?,
            colour: read.whole()?,
        },
        1 => Primitive::Disc {
            x: read.real()?,
            y: read.real()?,
            radius: read.real()?,
            colour: read.whole()?,
        },
        2 => {
            let count =
                usize::try_from(read.whole()?).map_err(|_| "a strip has a negative pair count")?;
            if count > items.len() {
                return Err("a strip counts more pairs than it holds".into());
            }
            let points = (0..count)
                .map(|_| Ok((read.real()?, read.real()?)))
                .collect::<Result<Vec<_>, String>>()?;
            Primitive::Strip {
                points,
                colour: read.whole()?,
                stroke: read.real()?,
            }
        }
        3 => Primitive::Sprite {
            index: read.whole()?,
            x: read.real()?,
            y: read.real()?,
            w: read.real()?,
            h: read.real()?,
            angle: read.real()?,
            colour: read.whole()?,
        },
        4 => Primitive::Glyphs {
            text: read.text()?,
            x: read.real()?,
            y: read.real()?,
            size: read.real()?,
            colour: read.whole()?,
        },
        5 => {
            let steps = read
                .list()?
                .iter()
                .map(step)
                .collect::<Result<Vec<_>, _>>()?;
            let outline = outline(read.list()?)?;
            let paint = paint(read.list()?)?;
            let transform = match read.list()? {
                [] => None,
                [x, y, scale] => Some((number(x)?, number(y)?, number(scale)?)),
                _ => return Err("a path transform is neither empty nor [x, y, scale]".into()),
            };
            Primitive::Path {
                steps,
                outline,
                paint,
                transform,
            }
        }
        6 => {
            // The only primitive with values after its depth: the anchor and the weight.
            let (text, x, y, size, colour) = (
                read.text()?,
                read.real()?,
                read.real()?,
                read.real()?,
                read.whole()?,
            );
            let depth = read.whole()?;
            let anchor = read.whole()?;
            if !(0..=2).contains(&anchor) {
                return Err(format!("a lettering anchor is {anchor}, not 0, 1 or 2"));
            }
            let primitive = Primitive::Lettering {
                text,
                x,
                y,
                size,
                colour,
                anchor,
                weight: read.real()?,
            };
            read.done()?;
            return Ok(Shape { primitive, depth });
        }
        code => return Err(format!("unknown primitive code {code}")),
    };
    let depth = read.whole()?;
    read.done()?;
    Ok(Shape { primitive, depth })
}

fn number(value: &Value) -> Result<f64, String> {
    value
        .as_f64()
        .filter(|value| value.is_finite())
        .ok_or_else(|| "a value is not a finite number".into())
}

fn step(value: &Value) -> Result<Step, String> {
    let items = value.as_array().ok_or("a path step is not an array")?;
    let mut read = Reader { items, at: 0 };
    let step = match read.whole()? {
        0 => Step::Move(read.real()?, read.real()?),
        1 => Step::Line(read.real()?, read.real()?),
        2 => Step::Quad(read.real()?, read.real()?, read.real()?, read.real()?),
        3 => Step::Cubic(
            read.real()?,
            read.real()?,
            read.real()?,
            read.real()?,
            read.real()?,
            read.real()?,
        ),
        4 => Step::Arc {
            rx: read.real()?,
            ry: read.real()?,
            rotation: read.real()?,
            large: read.flag()?,
            sweep: read.flag()?,
            x: read.real()?,
            y: read.real()?,
        },
        5 => {
            let closed = read.flag()?;
            let rest = items.len() - read.at;
            if !rest.is_multiple_of(2) {
                return Err("a polygon has an odd number of coordinates".into());
            }
            let corners = (0..rest / 2)
                .map(|_| Ok((read.real()?, read.real()?)))
                .collect::<Result<Vec<_>, String>>()?;
            Step::Polygon { closed, corners }
        }
        6 => Step::Close,
        code => return Err(format!("unknown path step {code}")),
    };
    read.done()?;
    Ok(step)
}

fn outline(items: &[Value]) -> Result<Outline, String> {
    let mut read = Reader { items, at: 0 };
    match read.whole()? {
        0 => read.done().map(|()| Outline::Fill),
        1 => {
            let width = read.real()?;
            let bevel = read.flag()?;
            let dashes = items[read.at..]
                .iter()
                .map(number)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Outline::Stroke {
                width,
                bevel,
                dashes,
            })
        }
        code => Err(format!("unknown outline {code}")),
    }
}

fn tint(value: &Value) -> Result<Tint, String> {
    let items = value.as_array().ok_or("a tint is not an array")?;
    let mut read = Reader { items, at: 0 };
    let code = read.whole()?;
    let (a, b, c, alpha) = (read.real()?, read.real()?, read.real()?, read.real()?);
    read.done()?;
    match code {
        0 => Ok(Tint {
            r: a,
            g: b,
            b: c,
            a: alpha,
        }),
        1 => Ok(Tint::hsl(a, b, c, alpha)),
        code => Err(format!("unknown tint {code}")),
    }
}

fn paint(items: &[Value]) -> Result<Paint, String> {
    let mut read = Reader { items, at: 0 };
    let paint = match read.whole()? {
        0 => Paint::Solid(tint(read.next()?)?),
        1 => Paint::Linear {
            angle: read.real()?,
            from: tint(read.next()?)?,
            from_at: read.real()?,
            to: tint(read.next()?)?,
            to_at: read.real()?,
            oklab: read.flag()?,
        },
        code => return Err(format!("unknown paint {code}")),
    };
    read.done()?;
    Ok(paint)
}
