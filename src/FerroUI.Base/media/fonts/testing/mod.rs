//! Test infrastructure for everything that needs fonts: fonts built in code
//! (no font files), a platform typeface serving them, a configurable font
//! backend, an in-memory asset loader and a locator scope binding them.
//!
//! ```ignore
//! let _scope = TestFontScope::start();                       // the default font set
//! let glyph_typeface = Typeface::from_name("Noto Mono").glyph_typeface();
//!
//! let font_manager = TestFontManagerImpl::new("Test Sans");  // a custom font set
//! font_manager.add_font(TestFontBuilder::new("Test Sans").codepoints(&[(0x20, 0x7E)]).build());
//! let _scope = TestFontScope::with_font_manager(font_manager);
//! ```
//!
//! [`test_fonts`] has a set of named fonts and an asset loader embedding them.

mod test_asset_loader;
mod test_font_builder;
mod test_font_manager_impl;
mod test_font_scope;
pub mod test_fonts;
mod test_platform_typeface;

pub use test_asset_loader::TestAssetLoader;
pub use test_font_builder::TestFontBuilder;
pub use test_font_manager_impl::TestFontManagerImpl;
pub use test_font_scope::TestFontScope;
pub use test_platform_typeface::TestPlatformTypeface;
