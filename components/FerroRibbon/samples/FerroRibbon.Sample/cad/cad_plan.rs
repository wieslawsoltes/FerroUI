// Copyright (c) 2026 Wiesław Šoltés. MIT. RibbonSpace CAD model construction.
// Private source records become plain Rust enums. No UI or runtime dependency.
#[derive(Clone, Debug)]
pub(super) enum Entity {
    Poly {
        layer: &'static str,
        points: Vec<(f64, f64)>,
        closed: bool,
        fill: bool,
        dashed: bool,
    },
    Arc {
        layer: &'static str,
        center: (f64, f64),
        radius: f64,
        start: f64,
        sweep: f64,
        dashed: bool,
    },
}
impl Entity {
    pub fn key(&self) -> (&'static str, bool, bool) {
        match self {
            Self::Poly {
                layer,
                fill,
                dashed,
                ..
            } => (*layer, *fill, *dashed),
            Self::Arc { layer, dashed, .. } => (*layer, false, *dashed),
        }
    }
}
#[derive(Clone, Debug)]
pub(super) struct PlanText {
    pub point: (f64, f64),
    pub text: String,
    pub height: f64,
    pub angle: f64,
    pub layer: &'static str,
}
#[derive(Debug, Default)]
pub(super) struct Plan {
    pub entities: Vec<Entity>,
    pub texts: Vec<PlanText>,
}
impl Plan {
    pub fn new() -> Self {
        let mut p = Self::default();
        p.build();
        p
    }
    fn rect(&mut self, layer: &'static str, x1: f64, y1: f64, x2: f64, y2: f64, fill: bool) {
        self.entities.push(Entity::Poly {
            layer,
            points: vec![(x1, y1), (x2, y1), (x2, y2), (x1, y2)],
            closed: true,
            fill,
            dashed: false,
        });
    }
    fn line(&mut self, layer: &'static str, x1: f64, y1: f64, x2: f64, y2: f64, dashed: bool) {
        self.entities.push(Entity::Poly {
            layer,
            points: vec![(x1, y1), (x2, y2)],
            closed: false,
            fill: false,
            dashed,
        });
    }
    fn poly(&mut self, layer: &'static str, closed: bool, xy: &[f64]) {
        self.entities.push(Entity::Poly {
            layer,
            points: xy.as_chunks::<2>().0.iter().map(|p| (p[0], p[1])).collect(),
            closed,
            fill: false,
            dashed: false,
        });
    }
    fn circle(&mut self, layer: &'static str, x: f64, y: f64, r: f64) {
        self.arc(layer, x, y, r, 0., 360.);
    }
    fn arc(&mut self, layer: &'static str, x: f64, y: f64, r: f64, start: f64, sweep: f64) {
        self.entities.push(Entity::Arc {
            layer,
            center: (x, y),
            radius: r,
            start,
            sweep,
            dashed: false,
        });
    }
    fn text(&mut self, text: &str, x: f64, y: f64, height: f64, angle: f64, layer: &'static str) {
        self.texts.push(PlanText {
            point: (x, y),
            text: text.to_owned(),
            height,
            angle,
            layer,
        });
    }
    fn wall_x(&mut self, y1: f64, y2: f64, x1: f64, x2: f64, openings: &[(f64, f64)]) {
        let mut x = x1;
        let mut openings = openings.to_vec();
        openings.sort_by(|a, b| opening_order(a.0, b.0));
        for (from, to) in openings {
            self.rect("A-WALL", x, y1, from, y2, true);
            x = to;
        }
        self.rect("A-WALL", x, y1, x2, y2, true);
    }
    fn wall_y(&mut self, x1: f64, x2: f64, y1: f64, y2: f64, openings: &[(f64, f64)]) {
        let mut y = y1;
        let mut openings = openings.to_vec();
        openings.sort_by(|a, b| opening_order(a.0, b.0));
        for (from, to) in openings {
            self.rect("A-WALL", x1, y, x2, from, true);
            y = to;
        }
        self.rect("A-WALL", x1, y, x2, y2, true);
    }
    fn window_x(&mut self, y1: f64, y2: f64, x1: f64, x2: f64) {
        let t = (y2 - y1) / 3.;
        self.rect("A-GLAZ", x1, y1, x2, y2, false);
        self.line("A-GLAZ", x1, y1 + t, x2, y1 + t, false);
        self.line("A-GLAZ", x1, y1 + (2. * t), x2, y1 + (2. * t), false);
    }
    fn window_y(&mut self, x1: f64, x2: f64, y1: f64, y2: f64) {
        let t = (x2 - x1) / 3.;
        self.rect("A-GLAZ", x1, y1, x2, y2, false);
        self.line("A-GLAZ", x1 + t, y1, x1 + t, y2, false);
        self.line("A-GLAZ", x1 + (2. * t), y1, x1 + (2. * t), y2, false);
    }
    fn door(&mut self, hx: f64, hy: f64, w: f64, closed: f64, sweep: f64) {
        let open = (closed + sweep) * std::f64::consts::PI / 180.;
        self.line(
            "A-DOOR",
            hx,
            hy,
            hx + (w * open.cos()),
            hy + (w * open.sin()),
            false,
        );
        self.arc("A-DOOR", hx, hy, w, closed, sweep);
    }
    fn dim_x(&mut self, x1: f64, x2: f64, y: f64, from: f64, text: Option<&str>) {
        self.line("A-ANNO-DIMS", x1, from, x1, y - 150., false);
        self.line("A-ANNO-DIMS", x2, from, x2, y - 150., false);
        self.line("A-ANNO-DIMS", x1 - 150., y, x2 + 150., y, false);
        self.line("A-ANNO-DIMS", x1 - 90., y - 90., x1 + 90., y + 90., false);
        self.line("A-ANNO-DIMS", x2 - 90., y - 90., x2 + 90., y + 90., false);
        self.text(
            text.unwrap_or(&((x2 - x1) as i32).to_string()),
            (x1 + x2) / 2.,
            y + 190.,
            200.,
            0.,
            "A-ANNO-DIMS",
        );
    }
    fn dim_y(&mut self, y1: f64, y2: f64, x: f64, from: f64) {
        self.line("A-ANNO-DIMS", from, y1, x - 150., y1, false);
        self.line("A-ANNO-DIMS", from, y2, x - 150., y2, false);
        self.line("A-ANNO-DIMS", x, y1 - 150., x, y2 + 150., false);
        self.line("A-ANNO-DIMS", x - 90., y1 - 90., x + 90., y1 + 90., false);
        self.line("A-ANNO-DIMS", x - 90., y2 - 90., x + 90., y2 + 90., false);
        self.text(
            &((y2 - y1) as i32).to_string(),
            x - 190.,
            (y1 + y2) / 2.,
            200.,
            90.,
            "A-ANNO-DIMS",
        );
    }
    fn build(&mut self) {
        let outer = 250.;
        let inner = 120.;
        // Exterior walls with window and door openings.
        self.wall_x(
            0.,
            outer,
            0.,
            12000.,
            &[(1200., 3200.), (4000., 5500.), (9000., 10000.)],
        );
        self.wall_x(
            8000. - outer,
            8000.,
            0.,
            12000.,
            &[(1300., 3300.), (5700., 6800.), (9000., 10800.)],
        );
        self.wall_y(0., outer, outer, 8000. - outer, &[(1500., 3400.)]);
        self.wall_y(
            12000. - outer,
            12000.,
            outer,
            8000. - outer,
            &[(1500., 3000.), (5800., 7200.)],
        );
        self.window_x(0., outer, 1200., 3200.);
        self.window_x(0., outer, 4000., 5500.);
        self.window_x(8000. - outer, 8000., 1300., 3300.);
        self.window_x(8000. - outer, 8000., 5700., 6800.);
        self.window_x(8000. - outer, 8000., 9000., 10800.);
        self.window_y(0., outer, 1500., 3400.);
        self.window_y(12000. - outer, 12000., 1500., 3000.);
        self.window_y(12000. - outer, 12000., 5800., 7200.);
        // Interior walls.
        self.wall_x(
            4800. - (inner / 2.),
            4800. + (inner / 2.),
            outer,
            12000. - outer,
            &[(3800., 4700.), (5560., 6360.), (7800., 8700.)],
        );
        self.wall_y(
            6500. - (inner / 2.),
            6500. + (inner / 2.),
            outer,
            4800. - (inner / 2.),
            &[(1300., 3600.)],
        );
        self.wall_y(
            5000. - (inner / 2.),
            5000. + (inner / 2.),
            4800. + (inner / 2.),
            8000. - outer,
            &[],
        );
        self.wall_y(
            7500. - (inner / 2.),
            7500. + (inner / 2.),
            4800. + (inner / 2.),
            8000. - outer,
            &[],
        );
        // Doors.
        self.door(4700., 4860., 900., 180., -90.);
        self.door(6360., 4860., 800., 180., -90.);
        self.door(7800., 4860., 900., 0., 90.);
        self.door(10000., outer, 1000., 180., -90.);
        // Living room: sofa, coffee table, armchair, TV unit, rug outline.
        self.rect("A-FURN", 700., 700., 3300., 1600., false);
        self.rect("A-FURN", 700., 700., 3300., 950., false);
        self.line("A-FURN", 1567., 950., 1567., 1600., false);
        self.line("A-FURN", 2433., 950., 2433., 1600., false);
        self.rect("A-FURN", 1400., 2100., 2600., 2800., false);
        self.rect("A-FURN", 3900., 1500., 4800., 2400., false);
        self.rect("A-FURN", 3900., 1500., 4800., 1700., false);
        self.rect("A-FURN", 900., 4300., 3100., 4680., false);
        self.entities.push(Entity::Poly {
            layer: "A-FURN",
            points: vec![(900., 1850.), (3700., 1850.), (3700., 3500.), (900., 3500.)],
            closed: true,
            fill: false,
            dashed: true,
        });
        // Kitchen: L-shaped counter, sink, hob, dining table with chairs.
        self.poly(
            "A-FURN",
            true,
            &[
                11150., 600., 11750., 600., 11750., 4740., 8800., 4740., 8800., 4140., 11150.,
                4140.,
            ],
        );
        self.rect("A-FURN", 11250., 2000., 11650., 2900., false);
        self.circle("A-FURN", 11450., 2450., 120.);
        for (x, y) in [
            (9500., 4300.),
            (9900., 4300.),
            (9500., 4580.),
            (9900., 4580.),
        ] {
            self.circle("A-FURN", x, y, 110.);
        }
        self.circle("A-FURN", 8600., 2500., 600.);
        self.rect("A-FURN", 8375., 3150., 8825., 3550., false);
        self.rect("A-FURN", 8375., 1450., 8825., 1850., false);
        self.rect("A-FURN", 7550., 2275., 7950., 2725., false);
        self.rect("A-FURN", 9250., 2275., 9650., 2725., false);
        // Bedroom: bed with pillows, night stands, wardrobe.
        self.rect("A-FURN", 900., 5500., 2600., 7600., false);
        self.rect("A-FURN", 1050., 7050., 1650., 7450., false);
        self.rect("A-FURN", 1850., 7050., 2450., 7450., false);
        self.line("A-FURN", 900., 6800., 2600., 6800., false);
        self.line("A-FURN", 900., 6800., 2600., 6400., false);
        self.rect("A-FURN", 350., 7150., 800., 7600., false);
        self.rect("A-FURN", 2700., 7150., 3150., 7600., false);
        self.rect("A-FURN", 3700., 7100., 4850., 7700., false);
        self.line("A-FURN", 3700., 7100., 4850., 7700., false);
        // Bathroom: tub, toilet, basin.
        self.rect("A-FURN", 5150., 6950., 7300., 7700., false);
        self.rect("A-FURN", 5300., 7050., 7150., 7600., false);
        self.circle("A-FURN", 6900., 7325., 60.);
        self.rect("A-FURN", 5150., 5600., 5550., 6100., false);
        self.arc("A-FURN", 5850., 5850., 300., 90., 180.);
        self.line("A-FURN", 5850., 5550., 5850., 6150., false);
        self.rect("A-FURN", 6800., 5050., 7350., 5550., false);
        self.circle("A-FURN", 7075., 5300., 170.);
        // Second bedroom: bed, desk, chair.
        self.rect("A-FURN", 9600., 5300., 11400., 7300., false);
        self.rect("A-FURN", 9800., 6850., 10400., 7200., false);
        self.rect("A-FURN", 10600., 6850., 11200., 7200., false);
        self.line("A-FURN", 9600., 6600., 11400., 6600., false);
        self.rect("A-FURN", 7700., 7100., 9000., 7700., false);
        self.circle("A-FURN", 8350., 6750., 230.);
        // Centre lines of the structural grid.
        self.line("CENTER", 6500., -400., 6500., 8400., true);
        self.line("CENTER", -400., 4800., 12400., 4800., true);
        // Dimensions.
        self.dim_x(0., 12000., -1300., -300., None);
        self.dim_x(0., 6500., -700., -300., None);
        self.dim_x(6500., 12000., -700., -300., None);
        self.dim_y(0., 8000., -1300., -300.);
        self.dim_y(0., 4800., -700., -300.);
        self.dim_y(4800., 8000., -700., -300.);
        // Room names and areas.
        self.text("LIVING ROOM", 3250., 3150., 230., 0., "A-ANNO-TEXT");
        self.text("30.1 m²", 3250., 2830., 170., 0., "A-ANNO-TEXT");
        self.text("KITCHEN / DINING", 9300., 3700., 230., 0., "A-ANNO-TEXT");
        self.text("24.3 m²", 9300., 3380., 170., 0., "A-ANNO-TEXT");
        self.text("BEDROOM", 2500., 6200., 230., 0., "A-ANNO-TEXT");
        self.text("14.9 m²", 2500., 5880., 170., 0., "A-ANNO-TEXT");
        self.text("BATH", 6250., 6550., 200., 0., "A-ANNO-TEXT");
        self.text("BEDROOM 2", 9800., 5950., 200., 0., "A-ANNO-TEXT");
        self.text("FLOOR PLAN  1:100", 6000., -2000., 280., 0., "A-ANNO-TEXT");
    }
}
fn opening_order(a: f64, b: f64) -> std::cmp::Ordering {
    match (a.is_nan(), b.is_nan()) {
        (true, true) => std::cmp::Ordering::Equal,
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        (false, false) => a.partial_cmp(&b).unwrap(),
    }
}
