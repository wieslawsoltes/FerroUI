use ferroboard_sample::{
    App,
    sample_storage::{FileSampleStoragePlatform, SampleStorage},
};
use ferroui_android::{
    AndroidApplicationExtensions, FerroAndroidApplication,
    interop::java::{call_object, string_of},
};
use ferroui_base::utilities::CultureInfo;
use ferroui_controls::AppBuilder;
use std::{path::PathBuf, rc::Rc};

pub fn build() -> AppBuilder {
    let locale = ferroboard_core::text::locale_text::system_locale_name()
        .expect("FerroBoard Android could not read the system locale");
    CultureInfo::set_current_formatting(CultureInfo::get_culture_info(&locale));
    let context = FerroAndroidApplication::context();
    let directory = call_object(&context, "getFilesDir", "()Ljava/io/File;", &[])
        .expect("Android application files directory is unavailable");
    let path = call_object(&directory, "getAbsolutePath", "()Ljava/lang/String;", &[])
        .expect("Android application files path is unavailable");
    let folder = PathBuf::from(string_of(&path)).join("FerroBoard");
    assert!(
        folder.is_absolute(),
        "Android files directory must be absolute"
    );
    AppBuilder::configure::<App>()
        .use_android()
        .with(Rc::new(
            ferroboard_controls::services::KeyboardHapticService::native(),
        ))
        .with(SampleStorage::new(Rc::new(FileSampleStoragePlatform::new(
            folder,
        ))))
}
