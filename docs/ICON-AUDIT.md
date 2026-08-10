# Data Inspector — Native Icon Audit

## Rule

Normal interface icons are requested by semantic GTK/Freedesktop/GNOME names.
Data Inspector never detects or hardcodes Fluent, Zorin, Papirus, Adwaita, or
another user icon theme. GTK resolves the active user theme.

Application-bundled icons are only hicolor fallbacks for names that cannot be
relied on in every runtime/theme.

## Audited UI icons

| Context | Semantic icon | Result |
|---|---|---|
| Main menu | `open-menu-symbolic` | Keep — native GTK primary-menu icon |
| Current file close | `window-close-symbolic` | Keep — close/dismiss affordance; not Delete |
| Left sidebar | `panel-left-symbolic` | Keep — suite-standard GNOME/libpanel icon; bundled fallback |
| Structured view | `view-list-symbolic` | Keep — native list/structured-view semantic icon |
| Raw view | `format-text-plaintext-symbolic` | Keep — plain-text semantic icon; bundled fallback |
| Right Inspector | `panel-right-symbolic` | Keep — suite-standard GNOME/libpanel icon; bundled fallback |
| Empty/open state | `document-open-symbolic` | Keep — standardized Open Document action |
| Tree expanded | `pan-down-symbolic` | Keep — native disclosure direction |
| Tree collapsed | `pan-end-symbolic` | Keep — native text-direction-aware disclosure direction |

## Bundled hicolor fallbacks

Only these interface icons are bundled by Data Inspector:

- `panel-left-symbolic`
- `panel-right-symbolic`
- `format-text-plaintext-symbolic`

They are installed in `hicolor/scalable/actions`, so a user theme can provide
its own version first while the application still has a guaranteed fallback.

## Development runtime guard

Debug builds perform a one-time `GtkIconTheme::has_icon()` check for every
semantic UI icon and log PASS/FAIL. The check does not change the active theme
or its search paths.

## Application identity

Data Inspector now ships a dedicated suite-standard application icon pair:

- `io.github.christiaanbruinsma.DataInspector.svg`
- `io.github.christiaanbruinsma.DataInspector-symbolic.svg`

The main icon is installed in `hicolor/scalable/apps` and the symbolic icon in
`hicolor/symbolic/apps`, renamed to the active application ID for stable and
`.Devel` builds. Desktop metadata and the About dialog request the application
ID rather than a generic MIME icon.

Final user-theme QA must be performed on an exported and installed Flatpak;
GNOME Builder development runs are not authoritative for host icon-theme
integration.
