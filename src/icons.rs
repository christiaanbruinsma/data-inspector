//! Semantic icon names used by Data Inspector.
//!
//! Normal UI icons are requested by semantic name so GTK can resolve them
//! through the user's active icon theme. Only icons that are not guaranteed by
//! the normal GTK/GNOME icon set are bundled as hicolor fallbacks by the app.

/// Primary application menu. Native GTK uses this name for menu buttons.
pub const MAIN_MENU: &str = "open-menu-symbolic";

/// Close/remove the currently loaded file from the view.
///
/// There is no standardized `document-close` action name. The existing close
/// affordance intentionally uses the conventional close glyph rather than a
/// delete/clear icon, because the file is not modified or deleted.
pub const CURRENT_FILE_CLOSE: &str = "window-close-symbolic";

/// Left application sidebar toggle (GNOME/libpanel semantic icon).
pub const LEFT_SIDEBAR: &str = "panel-left-symbolic";

/// Structured/list representation of the loaded data.
pub const STRUCTURED_VIEW: &str = "view-list-symbolic";

/// Raw/plain-text representation of the loaded data.
pub const RAW_VIEW: &str = "format-text-plaintext-symbolic";

/// Right Inspector sidebar toggle (GNOME/libpanel semantic icon).
pub const RIGHT_SIDEBAR: &str = "panel-right-symbolic";

/// Empty-state action/context for opening a data document.
pub const OPEN_DOCUMENT: &str = "document-open-symbolic";

/// Expanded disclosure indicator.
pub const DISCLOSURE_EXPANDED: &str = "pan-down-symbolic";

/// Collapsed disclosure indicator; `end` follows text direction.
pub const DISCLOSURE_COLLAPSED: &str = "pan-end-symbolic";

/// Every semantic UI icon that must resolve at runtime.
pub const REQUIRED_UI_ICONS: &[&str] = &[
    MAIN_MENU,
    CURRENT_FILE_CLOSE,
    LEFT_SIDEBAR,
    STRUCTURED_VIEW,
    RAW_VIEW,
    RIGHT_SIDEBAR,
    OPEN_DOCUMENT,
    DISCLOSURE_EXPANDED,
    DISCLOSURE_COLLAPSED,
];

/// Report missing semantic UI icons during development.
///
/// This deliberately does not alter the icon theme or add theme-specific
/// search paths. GTK remains responsible for resolving the active user theme
/// and its inherited/fallback themes.
#[cfg(debug_assertions)]
pub fn audit_runtime() {
    let Some(display) = gtk::gdk::Display::default() else {
        eprintln!("[icon-audit] GTK display unavailable; runtime icon audit skipped");
        return;
    };

    let theme = gtk::IconTheme::for_display(&display);
    eprintln!("[icon-audit] active theme: {}", theme.theme_name());

    let mut missing = 0usize;
    for &icon_name in REQUIRED_UI_ICONS {
        if !theme.has_icon(icon_name) {
            missing += 1;
            eprintln!("[icon-audit] MISSING: {icon_name}");
        }
    }

    if missing == 0 {
        eprintln!(
            "[icon-audit] PASS: all {} semantic UI icons resolve",
            REQUIRED_UI_ICONS.len()
        );
    } else {
        eprintln!(
            "[icon-audit] FAIL: {missing}/{} semantic UI icons do not resolve",
            REQUIRED_UI_ICONS.len()
        );
    }
}
