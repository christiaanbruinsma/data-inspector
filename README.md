# Data Inspector

Data Inspector is a local-first GNOME utility for inspecting structured data without uploading it to a third-party service. Version 0.9.0 focuses on fast, read-only inspection of JSON, CSV, and TSV files in a native GTK4/libadwaita interface.

## v0.9.0 scope

- Open JSON, CSV, and TSV files with the native file dialog or drag and drop.
- Parse JSON locally with `serde_json`.
- Browse JSON objects and arrays in an expandable structured view.
- Parse CSV/TSV locally with the Rust `csv` crate.
- Detect comma, semicolon, and tab delimiters; `.tsv` files use tab explicitly.
- Detect a simple header row or generate `Column N` names when no header is detected; ambiguous files can override this with the First row is header switch.
- Render CSV/TSV as a virtualized GTK `ColumnView` with resizable and sortable columns.
- Search JSON keys/paths/types/values or CSV cell values.
- Switch between Structured and Raw views without rewriting the source.
- Inspect a selected JSON node or CSV cell in the right inspector, including inferred CSV cell type.
- Copy JSON paths/values and CSV cell locations/values with native toast confirmation.
- Right-click JSON data to copy a complete selected item, including descendants, or its path.
- Right-click CSV/TSV data to copy a cell, full row, or full column while preserving true empty values.
- Pan wide CSV/TSV tables horizontally and vertically by holding the middle mouse button and dragging.
- Keep empty CSV cells visually identifiable without modifying their underlying value.
- Show file and format-specific overview information in the left sidebar.
- Follow the active GTK icon theme through semantic icon names, with bundled `hicolor` fallbacks only for required non-universal icons.
- Read-only by design.
- Localized runtime UI and desktop/AppStream metadata for English (fallback), Dutch, German, French, Spanish, Italian, and Portuguese.

## Technical baseline

- Rust 2024 edition
- GTK4
- libadwaita
- `serde_json`
- `csv`
- Meson
- GNU gettext / Meson i18n
- Flatpak / GNOME Platform 50

The layout follows the established suite pattern: file/search controls on the left, the working view in the center, and a contextual inspector on the right. Internally, format-specific documents are separated behind `DataDocument`, so additional formats can be added without turning the UI into format-specific conditionals throughout the application.

## Development build

Open `io.github.christiaanbruinsma.DataInspector.Devel.yml` in GNOME Builder and use the normal **Build** action.

Development app ID:

`io.github.christiaanbruinsma.DataInspector.Devel`

## Release candidate

For the v0.9.0 release identity use:

`io.github.christiaanbruinsma.DataInspector.yml`

Release app ID:

`io.github.christiaanbruinsma.DataInspector`

The release manifest builds with `-Dprofile=default`, which selects the non-development app ID and Cargo release profile. The manifest currently permits Cargo network access for dependency resolution; vendor dependencies before publishing through a build service that requires fully offline sources.

## Engineering documentation

- [Golden Standard](docs/GOLDEN-STANDARD.md) — canonical engineering, native integration, packaging, and QA reference for the Rust GNOME app suite.
- [Native Icon Audit](docs/ICON-AUDIT.md) — audited semantic icon choices and runtime fallback policy.
- [Release Checklist](docs/RELEASE-CHECKLIST.md) — Data Inspector release-candidate QA gates.
