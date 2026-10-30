// Copyright 2021, 2024 The Android Open Source Project.
// Copyright 2026 FerroUI contributors (Rust port).
// SPDX-License-Identifier: Apache-2.0
//! AndroidX's float CAM16 + HCT neutral-tone calculation for Android API31–33 imports.
//!
//! Ported from Material3 `internal/colorUtil/{Cam,CamUtils,Frame,HCTSolver}.android.kt`
//! at e4d933c98df7baa7ea2b83eea6e086915961e0ff. The platform's thirteen actual
//! system tones bypass this calculation. Only AndroidX's additional neutral tones use it.
//! Float32 boundaries are intentional: replacing them with the otherwise more precise
//! desktop HCT implementation can select a different 8-bit channel near a rounding boundary.

use ferroui_base::media::Color;
use std::f64::consts::{PI, TAU};
use std::sync::OnceLock;

const XYZ_TO_CAM: [[f32; 3]; 3] = [
    [0.401288, 0.650173, -0.051461],
    [-0.250268, 1.204414, 0.045854],
    [-0.002079, 0.048952, 0.953127],
];
const RGB_TO_XYZ: [[f64; 3]; 3] = [
    [0.41233895, 0.35762064, 0.18051042],
    [0.2126, 0.7152, 0.0722],
    [0.01932141, 0.11916382, 0.95034478],
];
const SCALED_DISCOUNT_FROM_LINRGB: [[f64; 3]; 3] = [
    [
        0.001200833568784504,
        0.002389694492170889,
        0.0002795742885861124,
    ],
    [
        0.0005891086651375999,
        0.0029785502573438758,
        0.0003270666104008398,
    ],
    [
        0.00010146692491640572,
        0.0005364214359186694,
        0.0032979401770712076,
    ],
];
const LINRGB_FROM_SCALED_DISCOUNT: [[f64; 3]; 3] = [
    [1373.2198709594231, -1100.4251190754821, -7.278681089101213],
    [-271.815969077903, 559.6580465940733, -32.46047482791194],
    [1.9622899599665666, -57.173814538844006, 308.7233197812385],
];
const Y_FROM_LINRGB: [f64; 3] = [0.2126, 0.7152, 0.0722];

const CRITICAL_PLANES: [f64; 255] = [
    0.015176349177441876,
    0.045529047532325624,
    0.07588174588720938,
    0.10623444424209313,
    0.13658714259697685,
    0.16693984095186062,
    0.19729253930674434,
    0.2276452376616281,
    0.2579979360165119,
    0.28835063437139563,
    0.3188300904430532,
    0.350925934958123,
    0.3848314933096426,
    0.42057480301049466,
    0.458183274052838,
    0.4976837250274023,
    0.5391024159806381,
    0.5824650784040898,
    0.6277969426914107,
    0.6751227633498623,
    0.7244668422128921,
    0.775853049866786,
    0.829304845476233,
    0.8848452951698498,
    0.942497089126609,
    1.0022825574869039,
    1.0642236851973577,
    1.1283421258858297,
    1.1946592148522128,
    1.2631959812511864,
    1.3339731595349034,
    1.407011200216447,
    1.4823302800086415,
    1.5599503113873272,
    1.6398909516233677,
    1.7221716113234105,
    1.8068114625156377,
    1.8938294463134073,
    1.9832442801866852,
    2.075074464868551,
    2.1693382909216234,
    2.2660538449872063,
    2.36523901573795,
    2.4669114995532007,
    2.5710888059345764,
    2.6777882626779785,
    2.7870270208169257,
    2.898822059350997,
    3.0131901897720907,
    3.1301480604002863,
    3.2497121605402226,
    3.3718988244681087,
    3.4967242352587946,
    3.624204428461639,
    3.754355295633311,
    3.887192587735158,
    4.022731918402185,
    4.160988767090289,
    4.301978482107941,
    4.445716283538092,
    4.592217266055746,
    4.741496401646282,
    4.893568542229298,
    5.048448422192488,
    5.20615066083972,
    5.3666897647573375,
    5.5300801301023865,
    5.696336044816294,
    5.865471690767354,
    6.037501145825082,
    6.212438385869475,
    6.390297286737924,
    6.571091626112461,
    6.7548350853498045,
    6.941541251256611,
    7.131223617812143,
    7.323895587840543,
    7.5195704746346665,
    7.7182615035334345,
    7.919981813454504,
    8.124744458384042,
    8.332562408825165,
    8.543448553206703,
    8.757415699253682,
    8.974476575321063,
    9.194643831691977,
    9.417930041841839,
    9.644347703669503,
    9.873909240696694,
    10.106627003236781,
    10.342513269534024,
    10.58158024687427,
    10.8238400726681,
    11.069304815507364,
    11.317986476196008,
    11.569896988756009,
    11.825048221409341,
    12.083451977536606,
    12.345119996613247,
    12.610063955123938,
    12.878295467455942,
    13.149826086772048,
    13.42466730586372,
    13.702830557985108,
    13.984327217668513,
    14.269168601521828,
    14.55736596900856,
    14.848930523210871,
    15.143873411576273,
    15.44220572664832,
    15.743938506781891,
    16.04908273684337,
    16.35764934889634,
    16.66964922287304,
    16.985093187232053,
    17.30399201960269,
    17.62635644741625,
    17.95219714852476,
    18.281524751807332,
    18.614349837764564,
    18.95068293910138,
    19.290534541298456,
    19.633915083172692,
    19.98083495742689,
    20.331304511189067,
    20.685334046541502,
    21.042933821039977,
    21.404114048223256,
    21.76888489811322,
    22.137256497705877,
    22.50923893145328,
    22.884842241736916,
    23.264076429332462,
    23.6469514538663,
    24.033477234264016,
    24.42366364919083,
    24.817520537484558,
    25.21505769858089,
    25.61628489293138,
    26.021211842414342,
    26.429848230738664,
    26.842203703840827,
    27.258287870275353,
    27.678110301598522,
    28.10168053274597,
    28.529008062403893,
    28.96010235337422,
    29.39497283293396,
    29.83362889318845,
    30.276079891419332,
    30.722335150426627,
    31.172403958865512,
    31.62629557157785,
    32.08401920991837,
    32.54558406207592,
    33.010999283389665,
    33.4802739966603,
    33.953417292456834,
    34.430438229418264,
    34.911345834551085,
    35.39614910352207,
    35.88485700094671,
    36.37747846067349,
    36.87402238606382,
    37.37449765026789,
    37.87891309649659,
    38.38727753828926,
    38.89959975977785,
    39.41588851594697,
    39.93615253289054,
    40.460400508064545,
    40.98864111053629,
    41.520882981230194,
    42.05713473317016,
    42.597404951718396,
    43.141702194811224,
    43.6900349931913,
    44.24241185063697,
    44.798841244188324,
    45.35933162437017,
    45.92389141541209,
    46.49252901546552,
    47.065252796817916,
    47.64207110610409,
    48.22299226451468,
    48.808024568002054,
    49.3971762874833,
    49.9904556690408,
    50.587870934119984,
    51.189430279724725,
    51.79514187861014,
    52.40501387947288,
    53.0190544071392,
    53.637271562750364,
    54.259673423945976,
    54.88626804504493,
    55.517063457223934,
    56.15206766869424,
    56.79128866487574,
    57.43473440856916,
    58.08241284012621,
    58.734331877617365,
    59.39049941699807,
    60.05092333227251,
    60.715611475655585,
    61.38457167773311,
    62.057811747619894,
    62.7353394731159,
    63.417162620860914,
    64.10328893648692,
    64.79372614476921,
    65.48848194977529,
    66.18756403501224,
    66.89098006357258,
    67.59873767827808,
    68.31084450182222,
    69.02730813691093,
    69.74813616640164,
    70.47333615344107,
    71.20291564160104,
    71.93688215501312,
    72.67524319850172,
    73.41800625771542,
    74.16517879925733,
    74.9167682708136,
    75.67278210128072,
    76.43322770089146,
    77.1981124613393,
    77.96744375590167,
    78.74122893956174,
    79.51947534912904,
    80.30219030335869,
    81.08938110306934,
    81.88105503125999,
    82.67721935322541,
    83.4778813166706,
    84.28304815182372,
    85.09272707154808,
    85.90692527145302,
    86.72564993000343,
    87.54890820862819,
    88.3767072518277,
    89.2090541872801,
    90.04595612594655,
    90.88742016217518,
    91.73345337380438,
    92.58406282226491,
    93.43925555268066,
    94.29903859396902,
    95.16341895893969,
    96.03240364439274,
    96.9059996312159,
    97.78421388448044,
    98.6670533535366,
    99.55452497210776,
];

#[derive(Clone, Copy)]
struct Frame {
    n: f32,
    aw: f32,
    nbb: f32,
    c: f32,
    nc: f32,
    rgb_d: [f32; 3],
    fl: f32,
    z: f32,
}

// Kotlin/JVM's Float transcendental overloads evaluate in Double and round to Float.
fn pow32(value: f32, power: f32) -> f32 {
    f64::from(value).powf(f64::from(power)) as f32
}
fn sqrt32(value: f32) -> f32 {
    f64::from(value).sqrt() as f32
}
fn cos32(value: f32) -> f32 {
    f64::from(value).cos() as f32
}

fn dot32(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn dot64(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn frame() -> &'static Frame {
    static VALUE: OnceLock<Frame> = OnceLock::new();
    VALUE.get_or_init(|| {
        let white = [95.047_f32, 100.0, 108.883];
        let cone = XYZ_TO_CAM.map(|row| dot32(white, row));
        let adapting_luminance = (200.0 / PI * y_from_lstar(50.0) / 100.0) as f32;
        let f = 0.8_f32 + 2.0_f32 / 10.0_f32;
        let amount = (f - 0.9_f32) * 10.0_f32;
        let c = (1.0_f32 - amount) * 0.59_f32 + amount * 0.69_f32;
        let d = (f
            * (1.0_f32
                - (1.0_f32 / 3.6_f32)
                    * f64::from((-adapting_luminance - 42.0_f32) / 92.0_f32).exp() as f32))
            .clamp(0.0, 1.0);
        let rgb_d = cone.map(|value| d * (100.0_f32 / value) + 1.0_f32 - d);
        let k = 1.0_f32 / (5.0_f32 * adapting_luminance + 1.0_f32);
        let k4 = k * k * k * k;
        let k4_f = 1.0_f32 - k4;
        let fl = k4 * adapting_luminance
            + 0.1_f32 * k4_f * k4_f * (5.0 * f64::from(adapting_luminance)).cbrt() as f32;
        let n = y_from_lstar(50.0) as f32 / white[1];
        let z = 1.48_f32 + sqrt32(n);
        let nbb = 0.725_f32 / pow32(n, 0.2_f32);
        let adapted = std::array::from_fn::<_, 3, _>(|i| {
            let a = pow32(fl * rgb_d[i] * cone[i] / 100.0_f32, 0.42_f32);
            400.0_f32 * a / (a + 27.13_f32)
        });
        let aw = (2.0_f32 * adapted[0] + adapted[1] + 0.05_f32 * adapted[2]) * nbb;
        Frame {
            n,
            aw,
            nbb,
            c,
            nc: f,
            rgb_d,
            fl,
            z,
        }
    })
}

fn hue_chroma(color: Color) -> (f32, f32) {
    let f = frame();
    let linear = [color.r, color.g, color.b].map(|channel| {
        let normalized = f32::from(channel) / 255.0_f32;
        if normalized <= 0.04045_f32 {
            normalized / 12.92_f32 * 100.0_f32
        } else {
            pow32((normalized + 0.055_f32) / 1.055_f32, 2.4_f32) * 100.0_f32
        }
    });
    let xyz = RGB_TO_XYZ.map(|row| dot64(linear.map(f64::from), row) as f32);
    let cone = XYZ_TO_CAM.map(|row| dot32(xyz, row));
    let adapted = std::array::from_fn::<_, 3, _>(|i| {
        let d = f.rgb_d[i] * cone[i];
        let a = pow32(f.fl * d.abs() / 100.0_f32, 0.42_f32);
        d.signum() * 400.0_f32 * a / (a + 27.13_f32)
    });
    let [r, g, b] = adapted;
    let a = (11.0_f32 * r + -12.0_f32 * g + b) / 11.0_f32;
    let b = (r + g - 2.0_f32 * b) / 9.0_f32;
    let u = (20.0_f32 * r + 20.0_f32 * g + 21.0_f32 * adapted[2]) / 20.0_f32;
    let p2 = (40.0_f32 * r + 20.0_f32 * g + adapted[2]) / 20.0_f32;
    let degrees = f64::from(b).atan2(f64::from(a)) as f32 * 180.0_f32 / std::f32::consts::PI;
    let hue = if degrees < 0.0 {
        degrees + 360.0_f32
    } else if degrees >= 360.0 {
        degrees - 360.0_f32
    } else {
        degrees
    };
    let j = 100.0_f32 * pow32(p2 * f.nbb / f.aw, f.c * f.z);
    let hue_prime = if f64::from(hue) < 20.14 {
        hue + 360.0_f32
    } else {
        hue
    };
    let e_hue =
        0.25_f32 * (cos32(hue_prime * std::f32::consts::PI / 180.0_f32 + 2.0_f32) + 3.8_f32);
    let p1 = 50000.0_f32 / 13.0_f32 * e_hue * f.nc * f.nbb;
    let t = p1 * sqrt32(a * a + b * b) / (u + 0.305_f32);
    let alpha = pow32(t, 0.9_f32) * pow32(1.64_f32 - pow32(0.29_f32, f.n), 0.73_f32);
    (hue, alpha * sqrt32(j / 100.0_f32))
}

pub(super) fn set_luminance(color: Color, tone: u8) -> Color {
    let (hue, chroma) = hue_chroma(color);
    solve(f64::from(hue), f64::from(chroma), f64::from(tone))
}

fn y_from_lstar(tone: f64) -> f64 {
    if tone > 8.0 {
        ((tone + 16.0) / 116.0).powf(3.0) * 100.0
    } else {
        tone / (24389.0 / 27.0) * 100.0
    }
}

fn delinearized(value: f64) -> f64 {
    let normalized = value / 100.0;
    255.0
        * if normalized <= 0.0031308 {
            normalized * 12.92
        } else {
            1.055 * normalized.powf(1.0 / 2.4) - 0.055
        }
}

fn color_from_linear(rgb: [f64; 3]) -> Color {
    let [r, g, b] = rgb.map(|value| delinearized(value).round().clamp(0.0, 255.0) as u8);
    Color::from_rgb(r, g, b)
}

fn solve(hue: f64, chroma: f64, tone: f64) -> Color {
    if chroma < 0.0001 || tone < 0.0001 || tone > 99.9999 {
        return color_from_linear([y_from_lstar(tone); 3]);
    }
    let hue = hue.rem_euclid(360.0) / 180.0 * PI;
    let y = y_from_lstar(tone);
    exact_result(hue, chroma, y).unwrap_or_else(|| bisect_limit(y, hue))
}

fn inverse_adaptation(value: f64) -> f64 {
    let magnitude = value.abs();
    value.signum()
        * (27.13 * magnitude / (400.0 - magnitude))
            .max(0.0)
            .powf(1.0 / 0.42)
}

fn exact_result(hue: f64, chroma: f64, y: f64) -> Option<Color> {
    let f = frame();
    let mut j = y.sqrt() * 11.0;
    let inner = 1.0 / (1.64 - 0.29_f64.powf(f64::from(f.n))).powf(0.73);
    let e_hue = 0.25 * ((hue + 2.0).cos() + 3.8);
    let p1 = e_hue * (50000.0 / 13.0) * f64::from(f.nc) * f64::from(f.nbb);
    let h_sin = hue.sin();
    let h_cos = hue.cos();
    for iteration in 0..5 {
        let normalized = j / 100.0;
        let alpha = if chroma == 0.0 || j == 0.0 {
            0.0
        } else {
            chroma / normalized.sqrt()
        };
        let t = (alpha * inner).powf(1.0 / 0.9);
        let ac = f64::from(f.aw) * normalized.powf(1.0 / f64::from(f.c) / f64::from(f.z));
        let p2 = ac / f64::from(f.nbb);
        let gamma = 23.0 * (p2 + 0.305) * t / (23.0 * p1 + 11.0 * t * h_cos + 108.0 * t * h_sin);
        let a = gamma * h_cos;
        let b = gamma * h_sin;
        let scaled = [
            inverse_adaptation((460.0 * p2 + 451.0 * a + 288.0 * b) / 1403.0),
            inverse_adaptation((460.0 * p2 - 891.0 * a - 261.0 * b) / 1403.0),
            inverse_adaptation((460.0 * p2 - 220.0 * a - 6300.0 * b) / 1403.0),
        ];
        let rgb = LINRGB_FROM_SCALED_DISCOUNT.map(|row| dot64(scaled, row));
        if rgb.iter().any(|&component| component < 0.0) {
            return None;
        }
        let actual_y = dot64(Y_FROM_LINRGB, rgb);
        if actual_y <= 0.0 {
            return None;
        }
        if iteration == 4 || (actual_y - y).abs() < 0.002 {
            if rgb.iter().any(|&component| component > 100.01) {
                return None;
            }
            return Some(color_from_linear(rgb));
        }
        j -= (actual_y - y) * j / (2.0 * actual_y);
    }
    None
}

fn hue_of(rgb: [f64; 3]) -> f64 {
    let [r, g, b] = SCALED_DISCOUNT_FROM_LINRGB.map(|row| {
        let component = dot64(rgb, row);
        let factor = component.abs().powf(0.42);
        component.signum() * 400.0 * factor / (factor + 27.13)
    });
    let a = (11.0 * r + -12.0 * g + b) / 11.0;
    let b = (r + g - 2.0 * b) / 9.0;
    b.atan2(a)
}

fn cyclic(a: f64, b: f64, c: f64) -> bool {
    (b - a + PI * 8.0) % TAU < (c - a + PI * 8.0) % TAU
}

fn vertex(y: f64, n: usize) -> Option<[f64; 3]> {
    let a = if n % 4 <= 1 { 0.0 } else { 100.0 };
    let b = if n % 2 == 0 { 0.0 } else { 100.0 };
    let (rgb, candidate) = if n < 4 {
        let r = (y - a * Y_FROM_LINRGB[1] - b * Y_FROM_LINRGB[2]) / Y_FROM_LINRGB[0];
        ([r, a, b], r)
    } else if n < 8 {
        let g = (y - b * Y_FROM_LINRGB[0] - a * Y_FROM_LINRGB[2]) / Y_FROM_LINRGB[1];
        ([b, g, a], g)
    } else {
        let blue = (y - a * Y_FROM_LINRGB[0] - b * Y_FROM_LINRGB[1]) / Y_FROM_LINRGB[2];
        ([a, b, blue], blue)
    };
    (0.0..=100.0).contains(&candidate).then_some(rgb)
}

fn bisect_segment(y: f64, target: f64) -> ([f64; 3], [f64; 3]) {
    let mut left = [-1.0; 3];
    let mut right = left;
    let mut left_hue = 0.0;
    let mut right_hue = 0.0;
    let mut initialized = false;
    let mut uncut = true;
    for n in 0..12 {
        let Some(mid) = vertex(y, n) else {
            continue;
        };
        let mid_hue = hue_of(mid);
        if !initialized {
            left = mid;
            right = mid;
            left_hue = mid_hue;
            right_hue = mid_hue;
            initialized = true;
            continue;
        }
        if uncut || cyclic(left_hue, mid_hue, right_hue) {
            uncut = false;
            if cyclic(left_hue, target, mid_hue) {
                right = mid;
                right_hue = mid_hue;
            } else {
                left = mid;
                left_hue = mid_hue;
            }
        }
    }
    (left, right)
}

fn set_coordinate(source: [f64; 3], coordinate: f64, target: [f64; 3], axis: usize) -> [f64; 3] {
    let t = if target[axis] == source[axis] {
        target[axis]
    } else {
        (coordinate - source[axis]) / (target[axis] - source[axis])
    };
    std::array::from_fn(|i| source[i] + (target[i] - source[i]) * t)
}

fn bisect_limit(y: f64, target_hue: f64) -> Color {
    let (mut left, mut right) = bisect_segment(y, target_hue);
    let mut left_hue = hue_of(left);
    for axis in 0..3 {
        if left[axis] == right[axis] {
            continue;
        }
        let left_delinearized = delinearized(left[axis]) - 0.5;
        let right_delinearized = delinearized(right[axis]) - 0.5;
        let (mut left_plane, mut right_plane) = if left[axis] < right[axis] {
            (
                left_delinearized.floor() as i32,
                right_delinearized.ceil() as i32,
            )
        } else {
            (
                left_delinearized.ceil() as i32,
                right_delinearized.floor() as i32,
            )
        };
        for _ in 0..8 {
            if (right_plane - left_plane).abs() <= 1 {
                break;
            }
            let mid_plane = ((left_plane + right_plane) as f64 / 2.0).floor() as i32;
            let mid = set_coordinate(left, CRITICAL_PLANES[mid_plane as usize], right, axis);
            let mid_hue = hue_of(mid);
            if cyclic(left_hue, target_hue, mid_hue) {
                right = mid;
                right_plane = mid_plane;
            } else {
                left = mid;
                left_hue = mid_hue;
                left_plane = mid_plane;
            }
        }
    }
    color_from_linear(std::array::from_fn(|i| (left[i] + right[i]) / 2.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_cam16_matches_android_reference_primary_coordinates() {
        for (argb, expected_hue, expected_chroma) in [
            (0xffff0000, 27.408225, 113.35789),
            (0xff00ff00, 142.13983, 108.41006),
            (0xff0000ff, 282.7882, 87.23074),
        ] {
            let (hue, chroma) = hue_chroma(Color::from_uint32(argb));
            assert!((hue - expected_hue).abs() < 0.001, "{argb:#x}: hue {hue}");
            assert!(
                (chroma - expected_chroma).abs() < 0.001,
                "{argb:#x}: chroma {chroma}"
            );
        }
    }

    #[test]
    fn tone_extremes_are_opaque_black_and_white() {
        for base in [0x80605d66, 0xff0000ff, 0xffff0000, 0xff00ff00] {
            assert_eq!(
                set_luminance(Color::from_uint32(base), 0).to_uint32(),
                0xff000000
            );
            assert_eq!(
                set_luminance(Color::from_uint32(base), 100).to_uint32(),
                0xffffffff
            );
        }
    }

    #[test]
    fn all_added_neutral_tones_preserve_luminance_and_gamut() {
        use material_colors::color::Argb;
        for base in [
            0xff605d66, 0xff62615b, 0xff576366, 0xff0000ff, 0xff00ff00, 0xffff0000,
        ] {
            for tone in [4, 6, 12, 17, 22, 24, 87, 92, 94, 96, 98] {
                let color = set_luminance(Color::from_uint32(base), tone);
                assert_eq!(color.a, 255);
                let actual = Argb::new(color.a, color.r, color.g, color.b).as_lstar();
                assert!(
                    (actual - f64::from(tone)).abs() < 0.4,
                    "{base:#x} tone {tone}: {actual}"
                );
            }
        }
    }
}
