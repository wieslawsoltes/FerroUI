//! Path geometry written in Rust: [`path!`](crate::path), text checked when the crate is compiled
//! and kept as a static table, and [`path_data`], a fluent builder.
//!
//! Both make the calls that `Geometry::parse` makes for the same path, without linking the path
//! parser. Text given to `Geometry::parse` at run time stays the form for a path that comes from
//! data.

use crate::builder::IntoValue;
use ferroui_base::media::{FillRule, Geometry, PathCommand, SweepDirection};
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::{FerroLocator, LocatorExtensions, Point, Ref, Size};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

/// The geometries of the tables of `path!`, one per table on a thread, for one platform.
#[derive(Default)]
struct SharedGeometries {
    platform: Option<Weak<dyn IPlatformRenderInterface>>,
    geometries: HashMap<(usize, usize), Ref<Geometry>>,
}

ferroui_base::ferro_thread_local! {
    static SHARED: RefCell<SharedGeometries> = RefCell::new(SharedGeometries::default());
}

fn shared_geometry(commands: &'static [PathCommand]) -> Ref<Geometry> {
    let platform = FerroLocator::current().get_service::<dyn IPlatformRenderInterface>();
    SHARED
        .try_with(|shared| {
            let mut shared = shared.borrow_mut();
            let same = match (&shared.platform, &platform) {
                (Some(kept), Some(current)) => {
                    kept.upgrade().is_some_and(|kept| std::ptr::addr_eq(Rc::as_ptr(&kept), Rc::as_ptr(current)))
                }
                _ => false,
            };
            if !same {
                shared.geometries.clear();
                shared.platform = platform.as_ref().map(Rc::downgrade);
            }
            let key = (commands.as_ptr() as usize, commands.len());
            shared.geometries.entry(key).or_insert_with(|| Geometry::from_commands(commands)).clone()
        })
        .unwrap_or_else(|_| Geometry::from_commands(commands))
}

/// Path data that [`path!`](crate::path) checked and wrote as a static table.
///
/// A setter of a geometry takes it (`Path { data: path!("M0 0L10 10z") }`). Every element that
/// names one `path!` holds one geometry, so a list of a thousand icons holds one, and a component
/// that runs again gives its shape the geometry it has. [`to_geometry`](Self::to_geometry) makes
/// a geometry of its own, for one that will be changed.
///
/// ```
/// use ferroui_view::prelude::*;
/// let data = path!("M0 0 L1 1");
/// assert_eq!(data.commands().len(), 3);
/// ```
///
/// Data that does not parse is a compile error:
///
/// ```compile_fail
/// use ferroui_view::prelude::*;
/// let _ = path!("M0 0 X1");
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathData {
    commands: &'static [PathCommand],
}

impl PathData {
    /// The data of a table the macro wrote.
    #[doc(hidden)]
    pub const fn from_static(commands: &'static [PathCommand]) -> Self {
        Self { commands }
    }

    /// The calls on a geometry context, in order: what `PathCommand::record` gives for the text.
    pub const fn commands(&self) -> &'static [PathCommand] {
        self.commands
    }

    /// The geometry all the users of this table share.
    pub fn geometry(&self) -> Ref<Geometry> {
        shared_geometry(self.commands)
    }

    /// A new geometry of the path.
    pub fn to_geometry(&self) -> Ref<Geometry> {
        Geometry::from_commands(self.commands)
    }
}

impl IntoValue<Option<Ref<Geometry>>> for PathData {
    fn into_value(self) -> Option<Ref<Geometry>> {
        Some(self.geometry())
    }
}

impl IntoValue<Ref<Geometry>> for PathData {
    fn into_value(self) -> Ref<Geometry> {
        self.geometry()
    }
}

/// Starts a path written with calls, for a path computed from values:
///
/// ```ignore
/// Path { data: path_data().move_to(0.0, 0.0).line_to(width, 0.0).line_to(width / 2.0, height).close() }
/// ```
///
/// A fixed path is shorter as [`path!`](crate::path).
pub fn path_data() -> PathBuilder {
    PathBuilder::default()
}

/// A path being built ([`path_data`]). A segment with no figure open starts one at the current
/// point, as in path text.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PathBuilder {
    commands: Vec<PathCommand>,
    current: Point,
    start: Point,
    is_open: bool,
}

impl PathBuilder {
    fn open_figure(&mut self) {
        if !self.is_open {
            self.commands.push(PathCommand::BeginFigure(self.current, true));
            self.start = self.current;
            self.is_open = true;
        }
    }

    /// Sets how the inside of the path is decided (`F0`, `F1` in text).
    pub fn fill_rule(mut self, fill_rule: FillRule) -> Self {
        self.commands.push(PathCommand::SetFillRule(fill_rule));
        self
    }

    /// Starts a figure at the point (`M` in text), ending the one that is open.
    pub fn move_to(mut self, x: f64, y: f64) -> Self {
        if self.is_open {
            self.commands.push(PathCommand::EndFigure(false));
            self.is_open = false;
        }
        self.current = Point::new(x, y);
        self.open_figure();
        self
    }

    /// A line to the point (`L`).
    pub fn line_to(mut self, x: f64, y: f64) -> Self {
        self.open_figure();
        self.current = Point::new(x, y);
        self.commands.push(PathCommand::LineTo(self.current, true));
        self
    }

    /// A quadratic curve with the control point `(cx, cy)` to the point (`Q`).
    pub fn quad_to(mut self, cx: f64, cy: f64, x: f64, y: f64) -> Self {
        self.open_figure();
        self.current = Point::new(x, y);
        self.commands.push(PathCommand::QuadraticBezierTo(Point::new(cx, cy), self.current, true));
        self
    }

    /// A cubic curve with two control points to the point (`C`).
    pub fn cubic_to(mut self, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) -> Self {
        self.open_figure();
        self.current = Point::new(x, y);
        self.commands.push(PathCommand::CubicBezierTo(Point::new(x1, y1), Point::new(x2, y2), self.current, true));
        self
    }

    /// An elliptical arc of the radii `(rx, ry)`, turned by `rotation` degrees, to the point (`A`).
    #[allow(clippy::too_many_arguments)]
    pub fn arc_to(
        mut self,
        rx: f64,
        ry: f64,
        rotation: f64,
        is_large_arc: bool,
        sweep_direction: SweepDirection,
        x: f64,
        y: f64,
    ) -> Self {
        self.open_figure();
        self.current = Point::new(x, y);
        self.commands.push(PathCommand::ArcTo(
            self.current,
            Size::new(rx, ry),
            rotation,
            is_large_arc,
            sweep_direction,
            true,
        ));
        self
    }

    /// Closes the figure (`Z`); the current point returns to its start.
    pub fn close(mut self) -> Self {
        if self.is_open {
            self.commands.push(PathCommand::EndFigure(true));
            self.current = self.start;
            self.is_open = false;
        }
        self
    }

    /// The calls on a geometry context, in order, with the last figure ended.
    pub fn commands(&self) -> Vec<PathCommand> {
        let mut commands = self.commands.clone();
        if self.is_open {
            commands.push(PathCommand::EndFigure(false));
        }
        commands
    }

    /// The geometry of the path.
    pub fn to_geometry(&self) -> Ref<Geometry> {
        Geometry::from_commands(&self.commands())
    }
}

impl IntoValue<Option<Ref<Geometry>>> for PathBuilder {
    fn into_value(self) -> Option<Ref<Geometry>> {
        Some(self.to_geometry())
    }
}

impl IntoValue<Ref<Geometry>> for PathBuilder {
    fn into_value(self) -> Ref<Geometry> {
        self.to_geometry()
    }
}
