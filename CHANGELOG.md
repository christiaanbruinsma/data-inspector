# Changelog

## 0.9.1 — 2026-08-12

- Embed gettext translation catalogs directly in the standalone Flatpak release bundle.
- Ensure Dutch, German, French, Spanish, Italian, and Portuguese remain available when installing the GitHub `.flatpak` without a separate `.Locale` extension.
- No application feature, data-processing, or UI behavior changes.

## 0.9.0 — 2026-08-09

- Establish Data Inspector as a native Rust/GTK4/libadwaita structured-data inspector.
- Add local JSON opening, drag-and-drop, parsing, structured navigation, raw-source view, search, and contextual inspection.
- Add CSV/TSV loading with delimiter detection, header detection/override, generated column names, virtualized `GtkColumnView` rendering, sorting, resizing, search, and inferred cell types.
- Preserve decoded Raw source fidelity while stripping a leading UTF-8 BOM only from parser input.
- Add robustness coverage for malformed JSON, invalid UTF-8, BOM-prefixed JSON/CSV, quoted delimiters, multiline values, ragged CSV records, empty values, and large 10k/50k/100k CSV fixtures.
- Add middle-mouse drag panning for wide tabular data.
- Add contextual Inspector copy actions with native libadwaita toast feedback.
- Remove the redundant Inspector “Copy” section heading while keeping the existing copy actions unchanged.
- Add native right-click JSON context actions for copying a complete selected item, including descendants, or its JSON path.
- Add native right-click CSV/TSV context actions for copying a cell, full row as tab-separated text, or full column as newline-separated text while preserving true empty values.
- Clean up per-cell context popovers during `GtkColumnView` item teardown to avoid GTK finalization warnings.
- Keep the Current file close action neutral at rest and apply native destructive styling only on hover when a file is loaded.
- Use semantic GTK/Freedesktop/GNOME icon names and keep bundled `hicolor` assets limited to required fallbacks.
- Add the dedicated suite-standard Data Inspector main and symbolic application icons and wire them into desktop/About metadata and `hicolor` app-icon installation.
- Add left and right sidebar toggles using the GNOME/libpanel `panel-left-symbolic` and `panel-right-symbolic` icon pair.
- Use compact symbolic Structured and Raw view toggles with tooltips.
- Move data search into the main working area below the document title.
- Add dedicated development and release Flatpak manifests for the `.Devel` and stable application IDs.
- Add native gettext localization for Dutch, German, French, Spanish, Italian, and Portuguese, with English as the source/fallback language.
- Localize desktop and AppStream metadata for the same language set.
- Set the release candidate version to 0.9.0.
