//! Full compiled-XAML sample with private, RAM-backed storage supplied at startup.
#[path = "support/memory_storage.rs"]
mod memory_storage;

use ferroboard_sample::{App, sample_storage::SampleStorage};
use ferroui_controls::AppBuilder;
use ferroui_desktop::AppBuilderDesktopExtensions;

fn main() -> std::process::ExitCode {
    let storage = memory_storage::MemoryStorage::private();
    let code = AppBuilder::configure::<App>()
        .with(SampleStorage::new(storage))
        .use_platform_detect()
        .start_with_classic_desktop_lifetime(&[]);
    std::process::ExitCode::from(code as u8)
}
