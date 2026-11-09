use super::*;
use crate::cpu::scenes_rasterized;
use crate::test_scenes::{diffuse, model, orthographic, quad, snapshot, white_ambient, SIZE};
use ferroui_base::media::Color;

fn scene() -> Arc<Scene3DSnapshot> {
    snapshot(
        orthographic(),
        vec![model(quad(1, -0.5, -0.5, 0.5, 0.5, 0.0), vec![diffuse(Color::from_argb(255, 255, 0, 0))], vec![])],
        vec![white_ambient()],
    )
}

#[test]
fn an_unchanged_scene_is_not_rasterized_again() {
    let mut cache = Scene3DOutputCache::<Arc<Scene3DImage>>::new();
    let scene = scene();
    let before = scenes_rasterized();
    let first = cache.get_or_render(&scene, SIZE, 1.0, CpuRenderOptions::default(), |image| Some(Arc::new(image)));
    let second = cache.get_or_render(&scene, SIZE, 1.0, CpuRenderOptions::default(), |_| panic!("not made again"));
    let (first, second) = (first.expect("an output"), second.expect("an output"));
    assert!(Arc::ptr_eq(&first, &second));
    assert_eq!(before + 1, scenes_rasterized());
    assert_eq!([255, 0, 0, 255], first.pixel(50, 50));
}

#[test]
fn a_new_size_scale_or_option_renders_again_and_replaces_the_output_of_the_scene() {
    let mut cache = Scene3DOutputCache::<Arc<Scene3DImage>>::new();
    let scene = scene();
    let before = scenes_rasterized();
    let make = |image| Some(Arc::new(image));
    cache.get_or_render(&scene, SIZE, 1.0, CpuRenderOptions::default(), make);
    cache.get_or_render(&scene, PixelSize::new(200, 200), 2.0, CpuRenderOptions::default(), make);
    cache.get_or_render(&scene, PixelSize::new(200, 200), 2.0, CpuRenderOptions { antialiased: false }, make);
    assert_eq!(before + 3, scenes_rasterized());
    assert_eq!(1, cache.len());

    // Another snapshot, even of the same generations, is another scene.
    cache.get_or_render(&self::scene(), SIZE, 1.0, CpuRenderOptions::default(), make);
    assert_eq!(before + 4, scenes_rasterized());
    assert_eq!(2, cache.len());
}

#[test]
fn the_output_of_a_dropped_scene_is_dropped_and_the_cache_is_bounded() {
    let mut cache = Scene3DOutputCache::<u32>::with_capacity(2);
    let kept = scene();
    cache.get_or_render(&kept, SIZE, 1.0, CpuRenderOptions::default(), |_| Some(1));
    {
        let dropped = scene();
        cache.get_or_render(&dropped, SIZE, 1.0, CpuRenderOptions::default(), |_| Some(2));
        assert_eq!(2, cache.len());
    }
    let other = scene();
    cache.get_or_render(&other, SIZE, 1.0, CpuRenderOptions::default(), |_| Some(3));
    assert_eq!(2, cache.len());
    assert_eq!(Some(1), cache.get_or_render(&kept, SIZE, 1.0, CpuRenderOptions::default(), |_| Some(9)));

    let third = scene();
    cache.get_or_render(&third, SIZE, 1.0, CpuRenderOptions::default(), |_| Some(4));
    assert_eq!(2, cache.len());
    // `other` was used least recently and made room.
    assert_eq!(Some(5), cache.get_or_render(&other, SIZE, 1.0, CpuRenderOptions::default(), |_| Some(5)));

    // An output the backend cannot make is not kept.
    cache.clear();
    assert!(cache.get_or_render(&kept, SIZE, 1.0, CpuRenderOptions::default(), |_| None).is_none());
    assert!(cache.is_empty());
}
