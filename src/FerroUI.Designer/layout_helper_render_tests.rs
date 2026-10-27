//! Real renderer evidence for extension-provided adorners and cached control ghosts.
use super::*;
use ferroui_base::media::imaging::{PngBitmapEncoderOptions, RenderTargetBitmap};
use ferroui_base::platform::{IFontManagerImpl, IPlatformRenderInterface, ITextShaperImpl};
use ferroui_base::{
    FerroLocator, LocatorExtensions, Matrix, PixelRect, PixelSize, Size, Thickness,
};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{Border, Control, TextBlock};
use ferroui_harfbuzz::HarfBuzzTextShaper;
use ferroui_vello::{VelloOptions, VelloPlatform, VelloRenderingMode};

#[test]
fn native_affine_helpers_and_cached_control_ghost_render_without_runtime_mutation() {
    let locator = FerroLocator::enter_scope();
    VelloPlatform::initialize_with_options(VelloOptions::with_rendering_mode(
        VelloRenderingMode::Cpu,
    ));
    FerroLocator::current_mutable()
        .bind::<dyn ITextShaperImpl>()
        .to_constant(Rc::new(HarfBuzzTextShaper::new()));
    let services = TestServices {
        render_interface: FerroLocator::current().get_service::<dyn IPlatformRenderInterface>(),
        font_manager_impl: FerroLocator::current().get_service::<dyn IFontManagerImpl>(),
        text_shaper_impl: FerroLocator::current().get_service::<dyn ITextShaperImpl>(),
        ..TestServices::new()
    };
    let app = UnitTestApplication::start(services);
    let source = Border::new();
    source.set_background(Some(brush(0xff2868c8)));
    source.set_padding(Thickness::uniform(12.0));
    let title = TextBlock::new();
    title.set_text(Some("Native control"));
    title.set_foreground(Some(brush(0xfff2f3f5)));
    source.set_child(Some(title.upcast::<Control>()));
    source.set_width(140.0);
    source.set_height(50.0);
    source.measure(Size::new(140.0, 50.0));
    source.arrange(Rect::new(0.0, 0.0, 140.0, 50.0));
    let before = source.bounds();
    let ghost = DragVisualGhost::capture(
        &source.clone().upcast(),
        Matrix::create_translation(35.0, 345.0),
        1.0,
    )
    .unwrap();
    assert_eq!(ghost.pixel_count(), 140 * 50);
    let image = RenderTargetBitmap::new(PixelSize::new(880, 450));
    {
        let mut context = image.create_drawing_context();
        context.draw_rectangle(
            Some(&brush(0xff17181c)),
            None,
            Rect::new(0.0, 0.0, 880.0, 450.0),
            0.0,
            0.0,
            &BoxShadows::default(),
        );
        let mut scene = tests::fixture_scene();
        scene.placement = Some(ferroui_designer_tools::LayoutPlacement {
            container: 1,
            index: 1,
            attributes: Vec::new(),
            bounds: ferroui_designer_tools::Rect::new(16.0, 110.0, 248.0, 0.0),
            description: "Insert between children".into(),
            rejection: None,
        });
        let left = LayoutHelperSnapshot::new(
            scene.clone(),
            Affine {
                dx: 35.0,
                dy: 55.0,
                ..Affine::IDENTITY
            },
        );
        render_layout_helpers_with_state(
            &mut context,
            &left,
            1.0,
            &LayoutHelperRenderState {
                viewport: Some(Rect::new(0.0, 0.0, 880.0, 450.0)),
                hovered_handle: Some(0),
                active_handle: None,
            },
        );
        scene.placement.as_mut().unwrap().bounds =
            ferroui_designer_tools::Rect::new(60.0, 55.0, 160.0, 110.0);
        scene.placement.as_mut().unwrap().rejection = Some("Content holder is occupied".into());
        let right = LayoutHelperSnapshot::new(
            scene,
            Affine {
                m11: 1.35,
                m12: 0.1,
                m21: 0.2,
                m22: 0.95,
                dx: 425.0,
                dy: 55.0,
            },
        );
        render_layout_helpers(&mut context, &right, 1.0);
        ghost.render(&mut context, Matrix::create_translation(210.0, 0.0), 0.75);
        render_manipulation_metrics(
            &mut context,
            Rect::new(245.0, 345.0, 140.0, 50.0),
            &[
                Rect::new(35.0, 345.0, 140.0, 50.0),
                Rect::new(455.0, 345.0, 140.0, 50.0),
            ],
            &[],
            1.0,
        );
    }
    assert_eq!(source.bounds(), before);
    assert_eq!(source.width(), 140.0);
    let mut pixels = vec![0; 880 * 450 * 4];
    image.copy_pixels(PixelRect::new(0, 0, 880, 450), &mut pixels, 880 * 4);
    let background = &pixels[..4];
    let changed =
        |x: usize, y: usize| &pixels[(y * 880 + x) * 4..(y * 880 + x) * 4 + 4] != background;
    assert!(
        changed(250, 350),
        "the cached ghost paints the native blue Border at its moved position"
    );
    assert!(
        !changed(40, 350),
        "rendering a ghost does not move or paint its original control into the overlay"
    );
    assert!(
        !changed(235, 205),
        "final semantic placement replaces stale provider placement rectangles and labels"
    );
    let distinct = pixels
        .chunks_exact(4)
        .map(|pixel| (pixel[0], pixel[1], pixel[2]))
        .collect::<std::collections::BTreeSet<_>>();
    assert!(
        distinct.len() > 100,
        "the renderer paints native shaped labels and antialiased geometry"
    );
    if let Some(directory) = std::env::var_os("FERROUI_DESIGNER_RENDER_OUTPUT") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        image
            .save_to_file(
                directory
                    .join("designer-layout-overlay-affine.png")
                    .to_str()
                    .unwrap(),
                &PngBitmapEncoderOptions::default().into(),
            )
            .unwrap();
    }
    image.dispose();
    drop(ghost);
    drop(app);
    drop(locator);
}
