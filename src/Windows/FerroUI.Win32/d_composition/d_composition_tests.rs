//! Tests of the DirectComposition bindings against the system. They run
//! where the tests of the crate run on Windows; each prints what the
//! system answered.

use super::{IDCompositionDesktopDevice, IDCompositionDevice2, NativeMethods, DCOMPOSITION_FRAME_STATISTICS};

#[test]
fn the_system_creates_a_composition_device() {
    // Without a rendering device, as a device that only composes.
    let device = NativeMethods::d_composition_create_device2::<IDCompositionDesktopDevice>(None);
    println!("DCompositionCreateDevice2: {:?}", device.as_ref().map(|_| "created"));
    let device = device.expect("a composition device");

    // The desktop device is a device of the second version.
    let device2 = device.cast::<IDCompositionDevice2>().expect("IDCompositionDevice2");

    // A visual is created and the empty batch is committed.
    let visual = device2.create_visual();
    println!("CreateVisual: {:?}", visual.as_ref().map(|_| "created"));
    assert!(visual.is_ok());
    let committed = device2.commit();
    println!("Commit: {committed:?}");
    assert!(committed.is_ok());

    // The statistics of the frames: written through the layout the port
    // declares. A session without a compositor may refuse; a session with
    // one reports its rate and a time frequency.
    let mut statistics = DCOMPOSITION_FRAME_STATISTICS::default();
    // SAFETY: a structure of this frame with the layout of the system,
    // which the device fills during the call.
    let result = unsafe { device2.get_frame_statistics(&mut statistics) };
    println!("GetFrameStatistics: {result:?} {statistics:?}");
    if result.is_ok() {
        assert!(statistics.time_frequency > 0);
    }
}
