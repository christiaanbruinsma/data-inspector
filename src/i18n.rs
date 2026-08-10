use std::ffi::{CString, c_char, c_int};
use std::os::raw::c_ulong;

use crate::config;

// Data Inspector targets GNU/Linux/Flatpak, where gettext is provided by libc.
// Keep the small FFI boundary here so the application does not need an
// additional runtime wrapper crate solely for gettext initialization.
unsafe extern "C" {
    fn setlocale(category: c_int, locale: *const c_char) -> *mut c_char;
    fn bindtextdomain(domainname: *const c_char, dirname: *const c_char) -> *mut c_char;
    fn bind_textdomain_codeset(domainname: *const c_char, codeset: *const c_char) -> *mut c_char;
    fn textdomain(domainname: *const c_char) -> *mut c_char;
}

// LC_ALL on the glibc-based GNOME/Flatpak runtime used by Data Inspector.
const LC_ALL: c_int = 6;

pub fn init() {
    let locale = CString::new("").expect("empty locale contains no NUL bytes");
    let domain =
        CString::new(config::gettext_package()).expect("gettext domain contains no NUL bytes");
    let localedir =
        CString::new(config::localedir()).expect("locale directory contains no NUL bytes");
    let utf8 = CString::new("UTF-8").expect("UTF-8 contains no NUL bytes");

    unsafe {
        let _ = setlocale(LC_ALL, locale.as_ptr());
        let _ = bindtextdomain(domain.as_ptr(), localedir.as_ptr());
        let _ = bind_textdomain_codeset(domain.as_ptr(), utf8.as_ptr());
        let _ = textdomain(domain.as_ptr());
    }
}

pub fn gettext(message: &str) -> String {
    gtk::glib::dgettext(Some(config::gettext_package()), message).to_string()
}

pub fn ngettext(singular: &str, plural: &str, count: usize) -> String {
    gtk::glib::dngettext(
        Some(config::gettext_package()),
        singular,
        plural,
        count as c_ulong,
    )
    .to_string()
}

pub fn replace_named(mut message: String, values: &[(&str, String)]) -> String {
    for (name, value) in values {
        message = message.replace(&format!("{{{name}}}"), value);
    }
    message
}
