//! The fonts upstream embeds as manifest resources of its render test
//! assembly (`Assets/*.ttf`), under the same resource names in the assembly
//! of this crate. The files are the ones the Skia crate keeps for its unit
//! tests (`src/Skia/FerroUI.Skia/test_assets/assets`, copied unmodified
//! from the assets of upstream's render tests): they are not copied a
//! second time.

macro_rules! assets {
    ($($name:literal),* $(,)?) => {
        pub static RESOURCES: &[(&str, &[u8])] = &[
            $((
                concat!("FerroUI.Skia.RenderTests.Assets.", $name),
                include_bytes!(concat!("../../src/Skia/FerroUI.Skia/test_assets/assets/", $name)),
            ),)*
        ];
    };
}

assets![
    "AdobeBlank2VF.ttf",
    "Inter-Bold.ttf",
    "Inter-Regular.ttf",
    "InterVariable.ttf",
    "Manrope-Light.ttf",
    "MiSans-Normal.ttf",
    "NISC18030.ttf",
    "NotoMono-Regular.ttf",
    "NotoSans-Italic.ttf",
    "NotoSansArabic-Regular.ttf",
    "NotoSansDeseret-Regular.ttf",
    "NotoSansHebrew-Regular.ttf",
    "NotoSansMiao-Regular.ttf",
    "NotoSansTamil-Regular.ttf",
    "PointMatch.ttf",
    "SourceSerif4_36pt-Italic.ttf",
    "TwitterColorEmoji-SVGinOT.ttf",
];
