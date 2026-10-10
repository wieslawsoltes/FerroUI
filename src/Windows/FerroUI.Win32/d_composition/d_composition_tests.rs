//! Tests of the DirectComposition bindings against the system. They run
//! where the tests of the crate run on Windows; each prints what the
//! system answered.

use super::{IDCompositionDesktopDevice, IDCompositionDevice2, NativeMethods, DCOMPOSITION_FRAME_STATISTICS};

#[test]
fn the_system_creates_a_composition_device() {
    // Without a rendering device, as a device that only composes.
    let device = NativeMethods::d_composition_create_device2::<IDCompositionDesktopDevice>(None);
    println!("DCompositionCreateDevice2: {:?}", device.as_ref().map(|_| "created"));
    let Ok(device) = device else {
        // Session 0, where services run (and the tools of a virtual machine
        // host start their commands), has no compositor: the device is not
        // created there (the first run of this test in the virtual machine).
        // Anywhere else it has to be.
        #[link(name = "kernel32", kind = "raw-dylib")]
        extern "system" {
            fn GetCurrentProcessId() -> u32;
            fn ProcessIdToSessionId(process_id: u32, session_id: *mut u32) -> i32;
        }
        let mut session = u32::MAX;
        // SAFETY: a number of this frame the system writes to.
        let known = unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session) } != 0;
        assert!(known && session == 0, "a composition device could not be created (session {session})");
        println!("session 0: the system creates no composition device here");
        return;
    };

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
