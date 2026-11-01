//! Installed application artwork. GTK's independent icon-theme resolver
//! honors GIcon file and themed-icon forms, icon caches and pixmap fallbacks.
//! It does not open a GDK display or start a second GUI event loop.
use super::DesktopApplication;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::platform::native_library::{NativeLibrary, NativeLibraryError};
use ferroui_base::platform::{AlphaFormat, PixelFormat};
use ferroui_base::{PixelSize, Vector};
use std::cell::{OnceCell, RefCell};
use std::collections::VecDeque;
use std::ffi::{c_char, c_int, c_void, CString};
use std::rc::Rc;
use std::{ptr, slice};

type Object = *mut c_void;

ferroui_base::native_functions! {
    struct Api(gtk, gio, pixbuf, object) {
        gtk::gtk_icon_theme_new: unsafe extern "C" fn() -> Object;
        gtk::gtk_icon_theme_set_custom_theme: unsafe extern "C" fn(Object, *const c_char);
        gio::g_icon_new_for_string: unsafe extern "C" fn(*const c_char, *mut Object) -> Object;
        gtk::gtk_icon_theme_lookup_by_gicon: unsafe extern "C" fn(Object, Object, c_int, c_int) -> Object;
        gtk::gtk_icon_info_load_icon: unsafe extern "C" fn(Object, *mut Object) -> Object;
        pixbuf::gdk_pixbuf_get_width: unsafe extern "C" fn(Object) -> c_int;
        pixbuf::gdk_pixbuf_get_height: unsafe extern "C" fn(Object) -> c_int;
        pixbuf::gdk_pixbuf_get_n_channels: unsafe extern "C" fn(Object) -> c_int;
        pixbuf::gdk_pixbuf_get_rowstride: unsafe extern "C" fn(Object) -> c_int;
        pixbuf::gdk_pixbuf_read_pixels: unsafe extern "C" fn(Object) -> *const u8;
        object::g_object_unref: unsafe extern "C" fn(Object);
    }
}

struct Resolver {
    api: Api,
    theme: Object,
}

impl Resolver {
    fn new() -> Result<Self, NativeLibraryError> {
        let api = Api::load(
            &NativeLibrary::open(c"libgtk-3.so.0")?,
            &NativeLibrary::open(c"libgio-2.0.so.0")?,
            &NativeLibrary::open(c"libgdk_pixbuf-2.0.so.0")?,
            &NativeLibrary::open(c"libgobject-2.0.so.0")?,
        )?;
        // SAFETY: GTK creates a non-null, caller-owned theme. hicolor is the
        // freedesktop application-icon fallback installed across desktops.
        let theme = unsafe { (api.gtk_icon_theme_new)() };
        unsafe { (api.gtk_icon_theme_set_custom_theme)(theme, c"hicolor".as_ptr()) };
        Ok(Self { api, theme })
    }

    fn decode(&self, icon: &str) -> Option<(PixelSize, Vec<u8>)> {
        const SIZE: i32 = 64; // A 32-DIP launcher tile at 2x.
        const GTK_ICON_LOOKUP_FORCE_SIZE: c_int = 16;
        let icon = CString::new(icon).ok()?;
        let api = &self.api;
        // SAFETY: all input objects are live on this thread. The three calls
        // return owned objects or NULL and accept a null GError receiver.
        unsafe {
            let source = (api.g_icon_new_for_string)(icon.as_ptr(), ptr::null_mut());
            if source.is_null() {
                return None;
            }
            let info = (api.gtk_icon_theme_lookup_by_gicon)(
                self.theme,
                source,
                SIZE,
                GTK_ICON_LOOKUP_FORCE_SIZE,
            );
            (api.g_object_unref)(source);
            if info.is_null() {
                return None;
            }
            let pixels = (api.gtk_icon_info_load_icon)(info, ptr::null_mut());
            (api.g_object_unref)(info);
            if pixels.is_null() {
                return None;
            }
            let width = (api.gdk_pixbuf_get_width)(pixels);
            let height = (api.gdk_pixbuf_get_height)(pixels);
            let channels = (api.gdk_pixbuf_get_n_channels)(pixels);
            let stride = (api.gdk_pixbuf_get_rowstride)(pixels);
            let data = (api.gdk_pixbuf_read_pixels)(pixels);
            let valid = (1..=SIZE).contains(&width)
                && (1..=SIZE).contains(&height)
                && matches!(channels, 3 | 4)
                && stride >= channels * width
                && !data.is_null();
            let mut rgba = Vec::new();
            if valid {
                rgba.reserve_exact((width * height * 4) as usize);
                for y in 0..height {
                    // The final GdkPixbuf row may have no trailing padding.
                    let row = slice::from_raw_parts(
                        data.add(y as usize * stride as usize),
                        (width * channels) as usize,
                    );
                    if channels == 4 {
                        rgba.extend_from_slice(row);
                    } else {
                        for rgb in row.chunks_exact(3) {
                            rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
                        }
                    }
                }
            }
            (api.g_object_unref)(pixels);
            valid.then_some((PixelSize::new(width, height), rgba))
        }
    }
}

impl Drop for Resolver {
    fn drop(&mut self) {
        // SAFETY: this resolver owns the theme and all borrowers are gone.
        unsafe { (self.api.g_object_unref)(self.theme) };
    }
}

/// Cache only icons that have been visible. Negative entries avoid repeated
/// failed lookups while filtering. The decoded payload is bounded to 2 MiB.
#[derive(Default)]
pub struct ApplicationIcons {
    resolver: OnceCell<Option<Resolver>>,
    entries: RefCell<VecDeque<(String, Option<Rc<Bitmap>>)>>,
}

impl ApplicationIcons {
    pub fn get(&self, app: &DesktopApplication) -> Option<Rc<Bitmap>> {
        if app.icon.is_empty() {
            return None;
        }
        if let Some((_, image)) = self
            .entries
            .borrow()
            .iter()
            .find(|(key, _)| key == &app.icon)
        {
            return image.clone();
        }
        let resolver = self
            .resolver
            .get_or_init(|| Resolver::new().ok())
            .as_ref()?;
        let image = resolver.decode(&app.icon).map(|(size, rgba)| {
            Rc::new(Bitmap::from_pixels(
                PixelFormat::RGBA8888,
                AlphaFormat::Unpremul,
                &rgba,
                size,
                Vector::new(96.0, 96.0),
                size.width * 4,
            ))
        });
        let mut entries = self.entries.borrow_mut();
        if entries.len() == 128 {
            entries.pop_front();
        }
        entries.push_back((app.icon.clone(), image.clone()));
        image
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installed_file_icons_decode_without_a_gtk_display() {
        // The icon service is optional on minimal systems.
        let Ok(resolver) = Resolver::new() else {
            return;
        };
        let path =
            std::env::temp_dir().join(format!("ferro-desktop-icon-{}.xpm", std::process::id()));
        std::fs::write(
            &path,
            b"/* XPM */\nstatic char *icon[] = {\n\"1 1 1 1\",\n\"a c #FF7700\",\n\"a\"};\n",
        )
        .unwrap();
        let image = resolver.decode(path.to_str().unwrap());
        std::fs::remove_file(path).unwrap();
        let (size, bytes) = image.expect("decode installed file icon");
        assert!(size.width <= 64 && size.height <= 64);
        assert!(bytes.chunks_exact(4).all(|rgba| rgba == [255, 119, 0, 255]));
    }
}
