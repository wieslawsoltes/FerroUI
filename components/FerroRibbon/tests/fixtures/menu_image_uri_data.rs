// Generated from the unchanged source menu URI fixture; do not edit.
pub const CASES: &[(&str, &str, &str)] = &[
    ("  Assets/cut.png  ", "ms-appx", "  Assets/cut.png"),
    ("\tAssets/cut.png\r\n", "ms-appx", "\tAssets/cut.png"),
    ("ms-appx:///Assets/cut.png  ", "ms-appx", "Assets/cut.png"),
    (
        "\u{85}Assets/a.png\u{a0}",
        "ms-appx",
        "\u{85}Assets/a.png\u{a0}",
    ),
    ("  ms-appx:///Assets/cut.png  ", "ms-appx", "Assets/cut.png"),
    (
        "ferres://menu_uri_resource_probe/Assets/cut.png\t",
        "ferres",
        "Assets/cut.png",
    ),
    (" Assets/cut.png ", "ms-appx", " Assets/cut.png"),
    (" ms-appx:///Assets/cut.png ", "ms-appx", "Assets/cut.png"),
    ("\tAssets/cut.png\t", "ms-appx", "\tAssets/cut.png"),
    ("\tms-appx:///Assets/cut.png\t", "ms-appx", "Assets/cut.png"),
    ("\rAssets/cut.png\r", "ms-appx", "\rAssets/cut.png"),
    ("\rms-appx:///Assets/cut.png\r", "ms-appx", "Assets/cut.png"),
    ("\nAssets/cut.png\n", "ms-appx", "\nAssets/cut.png"),
    ("\nms-appx:///Assets/cut.png\n", "ms-appx", "Assets/cut.png"),
    (
        "\u{b}Assets/cut.png\u{b}",
        "ms-appx",
        "\u{b}Assets/cut.png\u{b}",
    ),
    (
        "\u{c}Assets/cut.png\u{c}",
        "ms-appx",
        "\u{c}Assets/cut.png\u{c}",
    ),
];
