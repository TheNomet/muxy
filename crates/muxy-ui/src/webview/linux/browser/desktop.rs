use std::ffi::{CStr, c_char, c_void};
use std::path::PathBuf;

use libloading::Library;

#[repr(C)]
struct List {
    data: *mut c_void,
    next: *mut Self,
    previous: *mut Self,
}

type Name = unsafe extern "C" fn(*mut c_void) -> *const c_char;
type Release = unsafe extern "C" fn(*mut c_void);

pub(super) fn installed_browsers() -> Vec<(String, PathBuf)> {
    unsafe { discover() }.unwrap_or_default()
}

unsafe fn discover() -> Result<Vec<(String, PathBuf)>, libloading::Error> {
    unsafe {
        let gio = Library::new("libgio-2.0.so.0")?;
        let object = Library::new("libgobject-2.0.so.0")?;
        let glib = Library::new("libglib-2.0.so.0")?;
        let apps = gio.get::<unsafe extern "C" fn(*const c_char) -> *mut List>(
            b"g_app_info_get_all_for_type\0",
        )?;
        let name = gio.get::<Name>(b"g_app_info_get_display_name\0")?;
        let file = gio.get::<Name>(b"g_desktop_app_info_get_filename\0")?;
        let shown =
            gio.get::<unsafe extern "C" fn(*mut c_void) -> i32>(b"g_app_info_should_show\0")?;
        let unref = object.get::<Release>(b"g_object_unref\0")?;
        let free = glib.get::<Release>(b"g_list_free\0")?;
        let head = apps(c"x-scheme-handler/https".as_ptr());
        let mut item = head;
        let mut browsers = Vec::new();
        while !item.is_null() {
            let app = (*item).data;
            let (name, file) = (name(app), file(app));
            if shown(app) != 0 && !name.is_null() && !file.is_null() {
                browsers.push((
                    CStr::from_ptr(name).to_string_lossy().into_owned(),
                    PathBuf::from(CStr::from_ptr(file).to_string_lossy().into_owned()),
                ));
            }
            unref(app);
            item = (*item).next;
        }
        free(head.cast());
        browsers.sort();
        browsers.dedup();
        Ok(browsers)
    }
}
