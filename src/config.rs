pub const BASE_APP_ID: &str = "io.github.christiaanbruinsma.DataInspector";
pub const VERSION: &str = "0.9.0";
pub const GETTEXT_PACKAGE: &str = "data-inspector";

pub fn app_id() -> &'static str {
    option_env!("APP_ID").unwrap_or(BASE_APP_ID)
}

pub fn gettext_package() -> &'static str {
    option_env!("GETTEXT_PACKAGE").unwrap_or(GETTEXT_PACKAGE)
}

pub fn localedir() -> &'static str {
    option_env!("LOCALEDIR").unwrap_or("/usr/share/locale")
}
