//! Tests of the Windows Runtime helpers against the system. They run where
//! the tests of the crate run on Windows (the Windows jobs of CI); each
//! prints what the system answered.

use super::native_win_rt_methods::{HStringInterop, NativeWinRTMethods};
use super::win_rt_api_information::WinRTApiInformation;
use super::win_rt_property_value::WinRTPropertyValue;
use super::{
    IAccessibilitySettings, IApiInformationStatics, IGlobalizationPreferencesStatics, IInspectable, IPropertyValue,
    IUISettings3, PropertyType, TrustLevel, UIColorType,
};
use crate::interop::unmanaged_methods::co_task_mem_free;
use ferroui_microcom::{Guid, Interface};

#[test]
fn a_string_handle_holds_its_text() {
    let text = "Windows.UI.ViewManagement.UISettings \u{2713} \u{1F600}";
    let string = HStringInterop::new(Some(text)).unwrap();
    assert_ne!(string.handle(), 0);
    assert_eq!(string.value().as_deref(), Some(text));

    // A borrowed handle reads the same text and does not delete it.
    let borrowed = HStringInterop::from_handle(string.handle(), false);
    assert_eq!(borrowed.value().as_deref(), Some(text));
    drop(borrowed);
    assert_eq!(string.value().as_deref(), Some(text));

    assert_eq!(HStringInterop::new(None).unwrap().value(), None);
    // The empty string is the null handle.
    assert_eq!(HStringInterop::new(Some("")).unwrap().handle(), 0);
}

#[test]
fn the_api_information_knows_the_types_of_the_system() {
    let factory = NativeWinRTMethods::create_activation_factory::<IApiInformationStatics>(
        "Windows.Foundation.Metadata.ApiInformation",
    );
    println!("ApiInformation factory: {:?}", factory.as_ref().map(|_| "created"));
    assert!(factory.is_ok());

    assert!(WinRTApiInformation::is_type_present("Windows.Foundation.Metadata.ApiInformation"));
    assert!(!WinRTApiInformation::is_type_present("Windows.Foundation.Metadata.NoSuchType"));
    assert!(WinRTApiInformation::is_method_present("Windows.Foundation.Metadata.ApiInformation", "IsTypePresent"));
    assert!(!WinRTApiInformation::is_method_present("Windows.Foundation.Metadata.ApiInformation", "NoSuchMethod"));
    assert!(WinRTApiInformation::is_method_present_with_arity(
        "Windows.Foundation.Metadata.ApiInformation",
        "IsTypePresent",
        1
    ));
    assert!(WinRTApiInformation::is_api_contract_present_by_major("Windows.Foundation.UniversalApiContract", 1));
    assert!(!WinRTApiInformation::is_api_contract_present_by_major("Windows.Foundation.UniversalApiContract", 1000));
    assert!(WinRTApiInformation::is_api_contract_present_by_major_and_minor(
        "Windows.Foundation.UniversalApiContract",
        1,
        0
    ));
    assert!(WinRTApiInformation::is_enum_named_value_present("Windows.Foundation.PropertyType", "SingleArray"));
    assert!(WinRTApiInformation::is_read_only_property_present("Windows.Foundation.Uri", "Host"));
    assert!(WinRTApiInformation::is_property_present("Windows.Foundation.Uri", "Host"));
    assert!(!WinRTApiInformation::is_writeable_property_present("Windows.Foundation.Uri", "Host"));
    assert!(!WinRTApiInformation::is_event_present("Windows.Foundation.Uri", "NoSuchEvent"));
}

#[test]
fn the_settings_of_the_user_are_read_through_activation() {
    // The types the platform settings read: present on a desktop system;
    // a system without them is reported, not failed.
    if WinRTApiInformation::is_type_present("Windows.UI.ViewManagement.UISettings") {
        let ui_settings =
            NativeWinRTMethods::create_instance::<IUISettings3>("Windows.UI.ViewManagement.UISettings").unwrap();
        let accent = ui_settings.get_color_value(UIColorType::Accent).unwrap();
        let background = ui_settings.get_color_value(UIColorType::Background).unwrap();
        println!("UISettings: accent {accent:?}, background {background:?}");
        assert_eq!(accent.a, 255);

        // An object of the Windows Runtime names its class.
        let class_name = HStringInterop::from_handle(ui_settings.get_runtime_class_name().unwrap(), true);
        assert_eq!(class_name.value().as_deref(), Some("Windows.UI.ViewManagement.UISettings"));
    } else {
        println!("UISettings: the type is not present on this system");
    }

    if WinRTApiInformation::is_type_present("Windows.UI.ViewManagement.AccessibilitySettings") {
        let accessibility_settings = NativeWinRTMethods::create_instance::<IAccessibilitySettings>(
            "Windows.UI.ViewManagement.AccessibilitySettings",
        )
        .unwrap();
        let high_contrast = accessibility_settings.high_contrast().unwrap();
        let scheme = HStringInterop::from_handle(accessibility_settings.high_contrast_scheme().unwrap(), true);
        println!("AccessibilitySettings: high contrast {high_contrast}, scheme {:?}", scheme.value());
    } else {
        println!("AccessibilitySettings: the type is not present on this system");
    }

    if WinRTApiInformation::is_type_present("Windows.System.UserProfile.GlobalizationPreferences") {
        let preferences = NativeWinRTMethods::create_activation_factory::<IGlobalizationPreferencesStatics>(
            "Windows.System.UserProfile.GlobalizationPreferences",
        )
        .unwrap();
        let languages = preferences.languages().unwrap().expect("the list of languages");
        let size = languages.size().unwrap();
        let first = (size > 0).then(|| HStringInterop::from_handle(languages.get_at(0).unwrap(), true).value());
        println!("GlobalizationPreferences: {size} language(s), the first {first:?}");
        assert!(size > 0);
    } else {
        println!("GlobalizationPreferences: the type is not present on this system");
    }

    assert!(NativeWinRTMethods::create_instance::<IInspectable>("FerroUI.NoSuchClass").is_err());
}

#[test]
fn a_property_value_answers_through_its_interface() {
    // The value is an object behind a COM vtable; the calls below go
    // through the vtable and the thunks, as a call of the system does.
    let value = WinRTPropertyValue::from_single_array(vec![0.25, 0.5, 0.75, 1.0]).into_com();
    assert!(value.type_().unwrap() == PropertyType::SingleArray);
    let mut size = 0;
    // SAFETY: the out parameter is a number of this frame; the block the
    // call returns is freed below with the allocator it came from.
    let numbers = unsafe { value.get_single_array(&mut size) }.unwrap();
    assert_eq!(size, 4);
    // SAFETY: the call succeeded, so the block holds `size` numbers.
    assert_eq!(unsafe { std::slice::from_raw_parts(numbers, 4) }, &[0.25, 0.5, 0.75, 1.0]);
    // SAFETY: the block of the call above, not used again.
    unsafe { co_task_mem_free(numbers.cast()) };

    let class_name = HStringInterop::from_handle(value.get_runtime_class_name().unwrap(), true);
    assert_eq!(class_name.value().as_deref(), Some("FerroUI.Win32.WinRT.WinRTPropertyValue"));
    assert!(value.get_trust_level().unwrap() == TrustLevel::BaseTrust);

    let mut count = 0u64;
    let mut iids: *mut Guid = std::ptr::null_mut();
    // SAFETY: out parameters of this frame; the block is freed below.
    unsafe { value.get_iids(&mut count, &mut iids) }.unwrap();
    assert_eq!(count, 2);
    // SAFETY: the call succeeded, so the block holds two identifiers.
    assert_eq!(unsafe { std::slice::from_raw_parts(iids, 2) }, &[IInspectable::IID, IPropertyValue::IID]);
    // SAFETY: the block of the call above, not used again.
    unsafe { co_task_mem_free(iids.cast()) };

    // The object answers a query for the interfaces it implements.
    assert!(value.cast::<IInspectable>().is_ok());
    assert!(value.cast::<IUISettings3>().is_err());

    // What the value does not hold is "not implemented", through the
    // interface as a failure code.
    let mut unused = 0;
    // SAFETY: an out parameter of this frame.
    assert_eq!(unsafe { value.get_double_array(&mut unused) }.unwrap_err().0, 0x8000_4001);
    // SAFETY: the method does not write to the pointer.
    assert_eq!(unsafe { value.get_point(std::ptr::null_mut()) }.unwrap_err().0, 0x8000_4001);

    let single = WinRTPropertyValue::from_single(1.5).into_com();
    assert!(single.type_().unwrap() == PropertyType::Single);
    assert_eq!(single.get_single().unwrap(), 1.5);
    // SAFETY: an out parameter of this frame.
    assert_eq!(unsafe { single.get_single_array(&mut unused) }.unwrap_err().0, 0x8000_4001);

    let number = WinRTPropertyValue::from_u_int32(7).into_com();
    assert!(number.type_().unwrap() == PropertyType::UInt32);
    assert_eq!(number.get_u_int32().unwrap(), 7);
}

/// The dispatcher queue of a thread and the compositor of the composition
/// library: what the composition mode of the Windows Runtime starts from.
/// On a thread of its own, which the test gives a queue.
#[test]
fn a_thread_gets_a_dispatcher_queue_and_a_compositor() {
    use super::native_win_rt_methods::{
        DispatcherQueueOptions, DISPATCHERQUEUE_THREAD_APARTMENTTYPE, DISPATCHERQUEUE_THREAD_TYPE,
    };
    use super::{ICompositor, IDispatcherQueueController};

    std::thread::spawn(|| {
        let options = DispatcherQueueOptions {
            dw_size: std::mem::size_of::<DispatcherQueueOptions>() as i32,
            thread_type: DISPATCHERQUEUE_THREAD_TYPE::DQTYPE_THREAD_CURRENT,
            apartment_type: DISPATCHERQUEUE_THREAD_APARTMENTTYPE::DQTAT_COM_NONE,
        };
        assert_eq!(options.dw_size, 12);
        let controller = NativeWinRTMethods::create_dispatcher_queue_controller(options);
        println!("CreateDispatcherQueueController: {:?}", controller.as_ref().map(|_| "created"));
        let controller: ferroui_microcom::ComPtr<IDispatcherQueueController> = controller.expect("a controller");
        let queue = controller.dispatcher_queue();
        println!("DispatcherQueue: {:?}", queue.as_ref().map(|_| "the queue of the thread"));
        assert!(queue.is_ok());

        // The activation factory of the composition library. The reference
        // declares the import and never calls it; the hosted runners of
        // the CI (a server system) answered at their first run that the
        // library, or its export, does not exist there. That is said, not
        // failed: nothing of the backend depends on it.
        match NativeWinRTMethods::get_windows_ui_composition_activation_factory("Windows.UI.Composition.Compositor") {
            Ok(factory) => {
                let instance = factory.activate_instance();
                println!("the activation factory of the composition library: ActivateInstance: {instance:?}");
                let instance = instance.expect("a compositor");
                assert_ne!(instance, 0);
                // SAFETY: the instance is an object the activation
                // returned, whose reference this test owns.
                let unknown =
                    unsafe { ferroui_microcom::ComPtr::<ferroui_microcom::IUnknown>::from_raw(instance as *mut _) }.unwrap();
                assert!(unknown.cast::<ICompositor>().is_ok());
            }
            Err(error) if error.0 == ferroui_microcom::HResult::NOTIMPL.0 => println!(
                "skipped: this system has no Windows.UI.Composition.dll with DllGetActivationFactory ({error}); \
                 the reference declares the import and does not call it"
            ),
            Err(error) => panic!("the activation factory of the composition library: {error}"),
        }

        // The compositor as the composition mode creates it: activated by
        // its class name on a thread with a dispatcher queue. A session
        // that cannot create one says so; the mode is then passed over.
        match NativeWinRTMethods::create_instance::<ICompositor>("Windows.UI.Composition.Compositor") {
            Ok(compositor) => {
                let compositor5 = compositor.cast::<super::ICompositor5>();
                let interop = compositor.cast::<super::ICompositorDesktopInterop>();
                println!(
                    "Windows.UI.Composition.Compositor: activated; ICompositor5 {:?}, ICompositorDesktopInterop {:?}",
                    compositor5.as_ref().map(|_| "yes"),
                    interop.as_ref().map(|_| "yes")
                );
                assert!(interop.is_ok());
            }
            Err(error) => println!("skipped: this session does not activate Windows.UI.Composition.Compositor: {error}"),
        }
    })
    .join()
    .unwrap();
}
