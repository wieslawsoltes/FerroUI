use crate::*;
use std::collections::BTreeMap;
fn c(s: &str) -> Color {
    Color::from_hex(s).unwrap()
}
fn add(d: &mut Document, p: NodeId, n: Node) -> NodeId {
    d.insert_node(p, n).unwrap()
}
fn r(
    d: &mut Document,
    p: NodeId,
    name: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    radius: f64,
) -> NodeId {
    add(
        d,
        p,
        Node::new(NodeKind::Rectangle, name, x, y, w, h)
            .with_fill(c(fill))
            .with_radius(radius),
    )
}
fn t(
    d: &mut Document,
    p: NodeId,
    name: &str,
    value: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    size: f64,
    weight: u16,
    fill: &str,
) -> NodeId {
    add(
        d,
        p,
        Node::new(NodeKind::Text, name, x, y, w, h)
            .with_text(value, size, c(fill))
            .with_weight(weight),
    )
}
fn e(
    d: &mut Document,
    p: NodeId,
    name: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
) -> NodeId {
    add(
        d,
        p,
        Node::new(NodeKind::Ellipse, name, x, y, w, h).with_fill(c(fill)),
    )
}
fn l(
    d: &mut Document,
    p: NodeId,
    name: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    fill: &str,
    width: f64,
) -> NodeId {
    let mut n = Node::new(NodeKind::Line, name, x, y, w, h);
    n.strokes = vec![Stroke::new(c(fill), width)];
    n.strokes[0].cap = StrokeCap::Round;
    add(d, p, n)
}
fn ct(
    d: &mut Document,
    p: NodeId,
    value: &str,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    size: f64,
    weight: u16,
    fill: &str,
) -> NodeId {
    let id = t(d, p, value, value, x, y, w, h, size, weight, fill);
    d.nodes.get_mut(&id).unwrap().text_align = TextAlign::Center;
    id
}
fn icon(
    d: &mut Document,
    p: NodeId,
    name: &str,
    x: f64,
    y: f64,
    ps: &[Point],
    stroke: &str,
    closed: bool,
) -> NodeId {
    let mut n = Node::new(NodeKind::Vector, name, x, y, 24.0, 24.0);
    n.path = VectorPath::polygon(ps, closed);
    n.strokes = vec![Stroke::new(c(stroke), 1.8)];
    n.strokes[0].cap = StrokeCap::Round;
    n.strokes[0].join = StrokeJoin::Round;
    add(d, p, n)
}
fn status(d: &mut Document, p: NodeId) {
    t(
        d,
        p,
        "Status / time",
        "9:41",
        28.0,
        17.0,
        55.0,
        22.0,
        14.0,
        650,
        "#151A22",
    );
    for i in 0..4 {
        r(
            d,
            p,
            "Status / signal",
            300.0 + i as f64 * 4.0,
            29.0 - i as f64 * 2.0,
            2.4,
            3.0 + i as f64 * 2.0,
            "#151A22",
            0.8,
        );
    }
    r(
        d,
        p,
        "Status / battery",
        342.0,
        21.0,
        22.0,
        11.0,
        "#151A22",
        3.0,
    );
    r(
        d,
        p,
        "Status / battery terminal",
        365.0,
        24.0,
        2.0,
        5.0,
        "#151A22",
        1.0,
    );
    icon(
        d,
        p,
        "Status / wifi",
        318.0,
        16.0,
        &[
            Point::new(1.0, 10.0),
            Point::new(7.0, 5.0),
            Point::new(13.0, 10.0),
        ],
        "#151A22",
        false,
    );
}
fn nav(d: &mut Document, p: NodeId, active: usize) -> Vec<NodeId> {
    r(
        d,
        p,
        "Navigation / surface",
        0.0,
        753.0,
        390.0,
        91.0,
        "#FFFFFF",
        0.0,
    );
    l(
        d,
        p,
        "Navigation / divider",
        0.0,
        753.0,
        390.0,
        0.0,
        "#ECEEF1",
        1.0,
    );
    let mut ids = Vec::new();
    for (i, label) in ["Home", "Cards", "Activity", "Profile"].iter().enumerate() {
        let a = add(
            d,
            p,
            Node::new(
                NodeKind::Group,
                format!("Navigation / {label}"),
                30.0 + i as f64 * 93.0,
                765.0,
                56.0,
                54.0,
            ),
        );
        let fg = if i == active { "#334721" } else { "#888F9A" };
        if i == active {
            r(
                d,
                a,
                "Active indicator",
                5.0,
                0.0,
                46.0,
                30.0,
                "#E8F7D0",
                12.0,
            );
        }
        match i {
            0 => {
                icon(
                    d,
                    a,
                    "Home icon",
                    16.0,
                    3.0,
                    &[
                        Point::new(1.0, 10.0),
                        Point::new(12.0, 1.0),
                        Point::new(23.0, 10.0),
                        Point::new(21.0, 10.0),
                        Point::new(21.0, 22.0),
                        Point::new(14.0, 22.0),
                        Point::new(14.0, 15.0),
                        Point::new(9.0, 15.0),
                        Point::new(9.0, 22.0),
                        Point::new(3.0, 22.0),
                        Point::new(3.0, 10.0),
                    ],
                    fg,
                    true,
                );
            }
            1 => {
                let id = r(d, a, "Card icon", 17.0, 7.0, 23.0, 16.0, "#FFFFFF00", 3.0);
                d.nodes.get_mut(&id).unwrap().strokes = vec![Stroke::new(c(fg), 1.7)];
                l(d, a, "Card stripe", 17.0, 12.0, 23.0, 0.0, fg, 1.7);
            }
            2 => {
                for j in 0..3 {
                    r(
                        d,
                        a,
                        "Activity bar",
                        18.0 + j as f64 * 7.0,
                        18.0 - j as f64 * 5.0,
                        4.0,
                        8.0 + j as f64 * 5.0,
                        fg,
                        1.5,
                    );
                }
            }
            _ => {
                let id = e(d, a, "Profile head", 24.0, 4.0, 10.0, 10.0, "#FFFFFF00");
                d.nodes.get_mut(&id).unwrap().strokes = vec![Stroke::new(c(fg), 1.6)];
                icon(
                    d,
                    a,
                    "Profile shoulders",
                    17.0,
                    5.0,
                    &[
                        Point::new(2.0, 22.0),
                        Point::new(3.0, 17.0),
                        Point::new(8.0, 14.0),
                        Point::new(15.0, 14.0),
                        Point::new(20.0, 17.0),
                        Point::new(21.0, 22.0),
                    ],
                    fg,
                    false,
                );
            }
        }
        ct(d, a, label, 0.0, 34.0, 56.0, 18.0, 10.0, 550, fg);
        ids.push(a)
    }
    r(
        d,
        p,
        "Home indicator",
        130.0,
        830.0,
        130.0,
        4.0,
        "#151A22",
        2.0,
    );
    ids
}
/// Original mobile-finance concept with editable layers, tokens and prototype links.
pub fn seeded_document() -> Document {
    let mut d = Document::new("Orbit — Mobile banking");
    let page = d.pages[0].id;
    d.pages[0].name = "01 · App design".into();
    let home = add(
        &mut d,
        page,
        Node::new(
            NodeKind::Frame,
            "Dashboard · iPhone 16",
            0.0,
            0.0,
            390.0,
            844.0,
        )
        .with_fill(c("#F8F9FB"))
        .with_radius(32.0),
    );
    let cards = add(
        &mut d,
        page,
        Node::new(
            NodeKind::Frame,
            "Cards · iPhone 16",
            486.0,
            0.0,
            390.0,
            844.0,
        )
        .with_fill(c("#F8F9FB"))
        .with_radius(32.0),
    );
    let system = add(
        &mut d,
        page,
        Node::new(
            NodeKind::Frame,
            "Components · Orbit system",
            986.0,
            0.0,
            420.0,
            844.0,
        )
        .with_fill(c("#FFFFFF"))
        .with_radius(24.0),
    );
    status(&mut d, home);
    status(&mut d, cards);
    t(
        &mut d,
        home,
        "Greeting",
        "Good morning, Alex",
        24.0,
        67.0,
        285.0,
        23.0,
        14.0,
        450,
        "#737B88",
    );
    t(
        &mut d,
        home,
        "Page title",
        "Your money,\nin one place.",
        24.0,
        98.0,
        295.0,
        98.0,
        34.0,
        700,
        "#151A22",
    );
    e(&mut d, home, "Avatar", 324.0, 63.0, 42.0, 42.0, "#E7EBD3");
    ct(
        &mut d, home, "A", 324.0, 72.0, 42.0, 24.0, 16.0, 700, "#5F6944",
    );
    let balance = add(
        &mut d,
        home,
        Node::new(NodeKind::Frame, "Balance card", 24.0, 213.0, 342.0, 180.0)
            .with_fill(c("#1F2926"))
            .with_radius(24.0),
    );
    d.nodes.get_mut(&balance).unwrap().fills = vec![Fill {
        paint: Paint::LinearGradient {
            start: Point::new(0.0, 0.0),
            end: Point::new(1.0, 1.0),
            stops: vec![
                GradientStop {
                    offset: 0.0,
                    color: c("#243C32"),
                },
                GradientStop {
                    offset: 1.0,
                    color: c("#17221E"),
                },
            ],
        },
        opacity: 1.0,
        visible: true,
    }];
    e(
        &mut d,
        balance,
        "Outer orbit",
        241.0,
        -43.0,
        193.0,
        193.0,
        "#405A443F",
    );
    e(
        &mut d,
        balance,
        "Inner orbit",
        279.0,
        -15.0,
        119.0,
        119.0,
        "#58734B3D",
    );
    t(
        &mut d,
        balance,
        "Balance label",
        "TOTAL BALANCE",
        22.0,
        22.0,
        215.0,
        20.0,
        10.0,
        600,
        "#B9C6BE",
    );
    t(
        &mut d,
        balance,
        "Balance amount",
        "$24,680.00",
        22.0,
        54.0,
        292.0,
        50.0,
        36.0,
        650,
        "#FFFFFF",
    );
    r(
        &mut d,
        balance,
        "Growth badge",
        22.0,
        126.0,
        57.0,
        24.0,
        "#C8F17A",
        12.0,
    );
    ct(
        &mut d, balance, "+8.2%", 22.0, 131.0, 57.0, 16.0, 10.0, 650, "#294026",
    );
    t(
        &mut d,
        balance,
        "Growth label",
        "this month",
        89.0,
        131.0,
        144.0,
        18.0,
        11.0,
        450,
        "#BECAC2",
    );
    let mut actions = Vec::new();
    for (i, label) in ["Send", "Request", "Top up", "More"].iter().enumerate() {
        let a = add(
            &mut d,
            home,
            Node::new(
                NodeKind::Group,
                format!("Quick action / {label}"),
                30.0 + i as f64 * 91.0,
                417.0,
                58.0,
                78.0,
            ),
        );
        e(
            &mut d,
            a,
            "Action circle",
            4.0,
            0.0,
            50.0,
            50.0,
            if i == 0 { "#C8F17A" } else { "#EDEFF2" },
        );
        if i < 2 {
            let ps = if i == 0 {
                vec![
                    Point::new(3.0, 19.0),
                    Point::new(20.0, 2.0),
                    Point::new(8.0, 2.0),
                    Point::new(20.0, 2.0),
                    Point::new(20.0, 14.0),
                ]
            } else {
                vec![
                    Point::new(20.0, 3.0),
                    Point::new(3.0, 20.0),
                    Point::new(3.0, 8.0),
                    Point::new(3.0, 20.0),
                    Point::new(15.0, 20.0),
                ]
            };
            icon(&mut d, a, "Action arrow", 18.0, 13.0, &ps, "#252E24", false);
        } else if i == 2 {
            l(
                &mut d,
                a,
                "Plus horizontal",
                20.0,
                25.0,
                18.0,
                0.0,
                "#252E24",
                1.9,
            );
            l(
                &mut d,
                a,
                "Plus vertical",
                29.0,
                16.0,
                0.0,
                18.0,
                "#252E24",
                1.9,
            );
        } else {
            for j in 0..3 {
                e(
                    &mut d,
                    a,
                    "More dot",
                    19.0 + j as f64 * 8.0,
                    23.0,
                    4.0,
                    4.0,
                    "#252E24",
                );
            }
        }
        ct(
            &mut d, a, label, 0.0, 59.0, 58.0, 18.0, 11.0, 550, "#3E4550",
        );
        actions.push(a)
    }
    t(
        &mut d,
        home,
        "Activity title",
        "Recent activity",
        24.0,
        527.0,
        220.0,
        26.0,
        18.0,
        650,
        "#151A22",
    );
    t(
        &mut d, home, "See all", "See all", 316.0, 532.0, 51.0, 18.0, 11.0, 550, "#747D87",
    );
    for (i, (initial, name, detail, amount, bg, fg)) in [
        (
            "F",
            "Figma",
            "Software · Today",
            "−$15.00",
            "#EBE2FC",
            "#7655B3",
        ),
        (
            "S",
            "Salary",
            "Income · Yesterday",
            "+$4,250.00",
            "#E4EFD5",
            "#477340",
        ),
        (
            "S",
            "Spotify",
            "Subscription · Oct 14",
            "−$10.99",
            "#DFF2E2",
            "#2A813E",
        ),
    ]
    .iter()
    .enumerate()
    {
        let y = 570.0 + i as f64 * 57.0;
        e(
            &mut d,
            home,
            &format!("{name} / icon"),
            24.0,
            y,
            40.0,
            40.0,
            bg,
        );
        ct(
            &mut d,
            home,
            initial,
            24.0,
            y + 9.0,
            40.0,
            25.0,
            17.0,
            700,
            fg,
        );
        t(
            &mut d,
            home,
            name,
            name,
            76.0,
            y + 1.0,
            154.0,
            21.0,
            13.0,
            600,
            "#242A34",
        );
        t(
            &mut d,
            home,
            detail,
            detail,
            76.0,
            y + 22.0,
            178.0,
            18.0,
            10.0,
            450,
            "#8B919B",
        );
        let id = t(
            &mut d,
            home,
            amount,
            amount,
            258.0,
            y + 10.0,
            107.0,
            23.0,
            12.0,
            600,
            if i == 1 { "#4F7F39" } else { "#242A34" },
        );
        d.nodes.get_mut(&id).unwrap().text_align = TextAlign::Right;
    }
    let hn = nav(&mut d, home, 0);

    t(
        &mut d,
        cards,
        "Cards title",
        "Your cards",
        24.0,
        69.0,
        240.0,
        45.0,
        30.0,
        700,
        "#151A22",
    );
    e(
        &mut d, cards, "Add card", 327.0, 67.0, 38.0, 38.0, "#EDEFF2",
    );
    l(
        &mut d,
        cards,
        "Add horizontal",
        338.0,
        86.0,
        16.0,
        0.0,
        "#29322C",
        1.8,
    );
    l(
        &mut d,
        cards,
        "Add vertical",
        346.0,
        78.0,
        0.0,
        16.0,
        "#29322C",
        1.8,
    );
    let card = add(
        &mut d,
        cards,
        Node::new(
            NodeKind::Frame,
            "Orbit Visa / virtual card",
            24.0,
            138.0,
            342.0,
            205.0,
        )
        .with_fill(c("#C8F17A"))
        .with_radius(22.0),
    );
    e(
        &mut d,
        card,
        "Card orbit 1",
        208.0,
        -18.0,
        191.0,
        191.0,
        "#A9D86466",
    );
    e(
        &mut d,
        card,
        "Card orbit 2",
        244.0,
        18.0,
        122.0,
        122.0,
        "#8EC95155",
    );
    t(
        &mut d, card, "Brand", "orbit", 24.0, 22.0, 128.0, 39.0, 26.0, 750, "#243522",
    );
    t(
        &mut d,
        card,
        "Visa brand",
        "VISA",
        256.0,
        30.0,
        65.0,
        29.0,
        20.0,
        750,
        "#243522",
    );
    r(
        &mut d, card, "EMV chip", 24.0, 86.0, 31.0, 24.0, "#ACC875", 5.0,
    );
    l(
        &mut d,
        card,
        "EMV vertical",
        39.0,
        87.0,
        0.0,
        22.0,
        "#81A353",
        0.8,
    );
    l(
        &mut d,
        card,
        "EMV horizontal",
        25.0,
        98.0,
        28.0,
        0.0,
        "#81A353",
        0.8,
    );
    t(
        &mut d,
        card,
        "Card number",
        "5412  ••••  ••••  9081",
        24.0,
        126.0,
        285.0,
        31.0,
        20.0,
        550,
        "#243522",
    );
    t(
        &mut d,
        card,
        "Cardholder",
        "ALEX MORGAN",
        24.0,
        172.0,
        196.0,
        19.0,
        10.0,
        650,
        "#4C693B",
    );
    t(
        &mut d, card, "Expiry", "08 / 29", 274.0, 172.0, 65.0, 19.0, 10.0, 650, "#4C693B",
    );
    e(
        &mut d,
        cards,
        "Carousel active",
        182.0,
        360.0,
        6.0,
        6.0,
        "#3D5531",
    );
    e(
        &mut d,
        cards,
        "Carousel inactive",
        195.0,
        360.0,
        6.0,
        6.0,
        "#D4DAD0",
    );
    for (i, label) in ["Freeze", "Details", "Settings"].iter().enumerate() {
        let a = add(
            &mut d,
            cards,
            Node::new(
                NodeKind::Group,
                format!("Card action / {label}"),
                39.0 + i as f64 * 116.0,
                390.0,
                83.0,
                69.0,
            ),
        );
        e(&mut d, a, "Action circle", 17.0, 0.0, 48.0, 48.0, "#EDEFF2");
        if i == 0 {
            for angle in [0.0, 60.0, 120.0] {
                let id = l(
                    &mut d,
                    a,
                    "Freeze spoke",
                    29.0,
                    24.0,
                    24.0,
                    0.0,
                    "#354134",
                    1.5,
                );
                d.nodes.get_mut(&id).unwrap().rotation = angle;
            }
        } else {
            for j in 0..3 {
                l(
                    &mut d,
                    a,
                    "Action rail",
                    30.0,
                    17.0 + j as f64 * 7.0,
                    22.0,
                    0.0,
                    "#354134",
                    1.6,
                );
                if i == 2 {
                    e(
                        &mut d,
                        a,
                        "Settings knob",
                        33.0 + (j % 2) as f64 * 11.0,
                        14.0 + j as f64 * 7.0,
                        6.0,
                        6.0,
                        "#354134",
                    );
                }
            }
        }
        ct(
            &mut d, a, label, 0.0, 56.0, 83.0, 18.0, 11.0, 550, "#3E4550",
        );
    }
    let limit = add(
        &mut d,
        cards,
        Node::new(NodeKind::Frame, "Spending limit", 24.0, 497.0, 342.0, 132.0)
            .with_fill(c("#FFFFFF"))
            .with_radius(18.0),
    );
    t(
        &mut d,
        limit,
        "Limit heading",
        "Monthly spending limit",
        18.0,
        19.0,
        258.0,
        23.0,
        13.0,
        600,
        "#2E3540",
    );
    t(
        &mut d,
        limit,
        "Limit used",
        "$4,800",
        18.0,
        51.0,
        111.0,
        28.0,
        20.0,
        650,
        "#252E24",
    );
    t(
        &mut d,
        limit,
        "Limit total",
        "of $8,000",
        93.0,
        58.0,
        162.0,
        22.0,
        11.0,
        450,
        "#89919B",
    );
    r(
        &mut d,
        limit,
        "Progress track",
        18.0,
        95.0,
        306.0,
        7.0,
        "#EDF1E8",
        4.0,
    );
    r(
        &mut d,
        limit,
        "Progress value",
        18.0,
        95.0,
        184.0,
        7.0,
        "#B4DF70",
        4.0,
    );
    r(
        &mut d,
        cards,
        "Security notice",
        24.0,
        650.0,
        342.0,
        70.0,
        "#EDF1E7",
        16.0,
    );
    e(
        &mut d,
        cards,
        "Security icon",
        40.0,
        669.0,
        31.0,
        31.0,
        "#D7E3C6",
    );
    ct(
        &mut d, cards, "✓", 40.0, 674.0, 31.0, 22.0, 16.0, 650, "#4B623A",
    );
    t(
        &mut d,
        cards,
        "Security title",
        "You're in control",
        83.0,
        663.0,
        246.0,
        22.0,
        12.0,
        650,
        "#405333",
    );
    t(
        &mut d,
        cards,
        "Security subtitle",
        "Manage your card, anytime.",
        83.0,
        686.0,
        246.0,
        19.0,
        10.0,
        450,
        "#7A866F",
    );
    let cn = nav(&mut d, cards, 1);
    t(
        &mut d,
        system,
        "System eyebrow",
        "ORBIT / DESIGN SYSTEM",
        28.0,
        31.0,
        363.0,
        20.0,
        10.0,
        650,
        "#79828A",
    );
    t(
        &mut d,
        system,
        "System headline",
        "Designed to move.",
        28.0,
        68.0,
        369.0,
        43.0,
        28.0,
        700,
        "#19211D",
    );
    t(
        &mut d,
        system,
        "System introduction",
        "A calmer way to feel in control.",
        28.0,
        116.0,
        365.0,
        25.0,
        13.0,
        450,
        "#7C8587",
    );
    t(
        &mut d,
        system,
        "Buttons section",
        "01  /  BUTTONS",
        28.0,
        177.0,
        362.0,
        20.0,
        10.0,
        650,
        "#7C8587",
    );
    let primary = add(
        &mut d,
        system,
        Node::new(
            NodeKind::Component,
            "Button / Primary",
            28.0,
            215.0,
            364.0,
            52.0,
        )
        .with_fill(c("#C8F17A"))
        .with_radius(14.0),
    );
    let label = ct(
        &mut d, primary, "Continue", 0.0, 16.0, 364.0, 24.0, 14.0, 650, "#2C3C24",
    );
    d.nodes.get_mut(&label).unwrap().constraints = Constraints {
        horizontal: Constraint::Stretch,
        vertical: Constraint::Center,
    };
    d.nodes.get_mut(&primary).unwrap().variant_properties = BTreeMap::from([
        ("Type".into(), "Primary".into()),
        ("Size".into(), "Large".into()),
        ("State".into(), "Default".into()),
    ]);
    let secondary = add(
        &mut d,
        system,
        Node::new(
            NodeKind::Component,
            "Button / Secondary",
            28.0,
            281.0,
            364.0,
            52.0,
        )
        .with_fill(c("#FFFFFF"))
        .with_radius(14.0),
    );
    d.nodes.get_mut(&secondary).unwrap().strokes = vec![Stroke::new(c("#DDE2DB"), 1.0)];
    ct(
        &mut d,
        secondary,
        "View details",
        0.0,
        16.0,
        364.0,
        24.0,
        14.0,
        600,
        "#394338",
    );
    t(
        &mut d,
        system,
        "Input section",
        "02  /  INPUT",
        28.0,
        374.0,
        362.0,
        20.0,
        10.0,
        650,
        "#7C8587",
    );
    let input = add(
        &mut d,
        system,
        Node::new(
            NodeKind::Component,
            "Input / Amount",
            28.0,
            410.0,
            364.0,
            80.0,
        )
        .with_fill(c("#F7F8F6"))
        .with_radius(14.0),
    );
    t(
        &mut d,
        input,
        "Amount label",
        "Amount",
        16.0,
        13.0,
        295.0,
        20.0,
        10.0,
        500,
        "#7C8587",
    );
    t(
        &mut d,
        input,
        "Amount value",
        "$ 0.00",
        16.0,
        38.0,
        254.0,
        29.0,
        22.0,
        650,
        "#263221",
    );
    t(
        &mut d, input, "Currency", "USD", 304.0, 44.0, 52.0, 21.0, 11.0, 600, "#7C8587",
    );
    t(
        &mut d,
        system,
        "Typography section",
        "03  /  TYPOGRAPHY",
        28.0,
        534.0,
        362.0,
        20.0,
        10.0,
        650,
        "#7C8587",
    );
    t(
        &mut d,
        system,
        "Display specimen",
        "Aa  Inter Display",
        28.0,
        573.0,
        370.0,
        43.0,
        27.0,
        650,
        "#1B261C",
    );
    t(
        &mut d,
        system,
        "Body specimen",
        "Purposeful by design. Human by default.",
        28.0,
        624.0,
        366.0,
        25.0,
        13.0,
        450,
        "#77816F",
    );
    t(
        &mut d,
        system,
        "Palette section",
        "04  /  CORE PALETTE",
        28.0,
        686.0,
        362.0,
        20.0,
        10.0,
        650,
        "#7C8587",
    );
    let palette = [
        ("Leaf", "#C8F17A"),
        ("Forest", "#243C32"),
        ("Ink", "#151A22"),
        ("Mist", "#F8F9FB"),
        ("Stone", "#8B919B"),
    ];
    for (i, (name, hex)) in palette.iter().enumerate() {
        r(
            &mut d,
            system,
            &format!("Palette / {name}"),
            28.0 + i as f64 * 75.0,
            725.0,
            64.0,
            54.0,
            hex,
            12.0,
        );
        t(
            &mut d,
            system,
            name,
            name,
            28.0 + i as f64 * 75.0,
            789.0,
            74.0,
            20.0,
            9.0,
            500,
            "#79828A",
        );
    }
    let notes = add(
        &mut d,
        page,
        Node::new(NodeKind::Frame, "Project notes", 0.0, 930.0, 876.0, 146.0)
            .with_fill(c("#252525"))
            .with_radius(16.0),
    );
    t(
        &mut d,
        notes,
        "Notes heading",
        "Orbit. Room to grow.",
        24.0,
        24.0,
        790.0,
        32.0,
        24.0,
        600,
        "#F3F3F3",
    );
    t(&mut d,notes,"Notes body","An original mobile banking concept. Every layer is editable.\nExplore components, prototype the flow, and make it yours.",24.0,75.0,803.0,48.0,14.0,400,"#93999A");
    for (name, hex) in palette {
        let id = d.allocate_id();
        d.styles.insert(
            id,
            Style {
                id,
                name: format!("Orbit / {name}"),
                description: format!("Orbit core palette · {name}"),
                value: StyleValue::Fill {
                    fills: vec![Fill::solid(c(hex))],
                },
            },
        );
        let id = d.allocate_id();
        d.variables.insert(
            id,
            Variable {
                id,
                name: format!("color/{}/default", name.to_lowercase()),
                collection: "Orbit".into(),
                values: BTreeMap::from([
                    ("Default".into(), VariableValue::Color(c(hex))),
                    (
                        "Dark".into(),
                        VariableValue::Color(if name == "Mist" {
                            c("#151A22")
                        } else if name == "Ink" {
                            c("#F8F9FB")
                        } else {
                            c(hex)
                        }),
                    ),
                ]),
            },
        );
    }
    for (name, size, weight) in [
        ("Display", 34.0, 700),
        ("Heading", 24.0, 650),
        ("Body", 14.0, 400),
        ("Caption", 11.0, 450),
    ] {
        let id = d.allocate_id();
        d.styles.insert(
            id,
            Style {
                id,
                name: format!("Type / {name}"),
                description: String::new(),
                value: StyleValue::Text {
                    family: "Inter".into(),
                    size,
                    weight,
                    line_height: 1.4,
                    letter_spacing: 0.0,
                },
            },
        );
    }
    let transition = Transition {
        kind: TransitionKind::SmartAnimate,
        duration_ms: 300,
        easing: Easing::EaseInOut,
    };
    for source in [hn[1], actions[0]] {
        d.nodes
            .get_mut(&source)
            .unwrap()
            .interactions
            .push(Interaction {
                trigger: Trigger::Click,
                action: PrototypeAction::NavigateTo { target: cards },
                transition: transition.clone(),
                delay_ms: 0,
                key: None,
            });
    }
    d.nodes
        .get_mut(&cn[0])
        .unwrap()
        .interactions
        .push(Interaction {
            trigger: Trigger::Click,
            action: PrototypeAction::NavigateTo { target: home },
            transition,
            delay_ms: 0,
            key: None,
        });
    d.pages[0].prototype_start = Some(home);
    let comment = d.allocate_id();
    d.comments.push(CommentThread {
        id: comment,
        node: Some(card),
        position: Point::new(864.0, 151.0),
        resolved: false,
        messages: vec![CommentMessage {
            author: "Alex".into(),
            body: "Explore a physical-card variant using the same component tokens.".into(),
            created_at: "2026-10-17T12:00:00Z".into(),
        }],
    });
    d.guides.insert(
        page,
        vec![
            Guide {
                horizontal: false,
                position: 0.0,
            },
            Guide {
                horizontal: true,
                position: 0.0,
            },
        ],
    );
    let second = d.add_page("02 · Playground");
    let welcome = add(
        &mut d,
        second,
        Node::new(NodeKind::Frame, "Your next idea", 0.0, 0.0, 800.0, 600.0)
            .with_fill(c("#FFFFFF"))
            .with_radius(8.0),
    );
    t(
        &mut d,
        welcome,
        "Playground title",
        "Make something great.",
        64.0,
        64.0,
        672.0,
        78.0,
        42.0,
        700,
        "#151A22",
    );
    t(
        &mut d,
        welcome,
        "Playground instructions",
        "Draw a frame. Add some shapes. Build a component.\nThis canvas is yours.",
        64.0,
        174.0,
        672.0,
        87.0,
        20.0,
        400,
        "#8B919B",
    );
    d.validate().expect("authored sample must be valid");
    d
}
