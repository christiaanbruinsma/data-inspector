# Data Inspector — Golden Standard

Data Inspector is the canonical engineering reference for the Rust-based GNOME/GTK/libadwaita applications in this suite.

This document defines the reusable architecture, native integration rules, packaging conventions, and QA gates that other applications should follow. It does **not** require other applications to copy Data Inspector's feature model, data formats, or exact screen layout.

## Normative language

- **MUST** — required for conformance unless the rule is not applicable to the app.
- **SHOULD** — preferred default; deviations need a clear technical reason.
- **MAY** — optional.
- **N/A** — genuinely not applicable; not a substitute for an untested gate.

## 1. Evidence-first baseline

A rule becomes part of the Golden Standard only after it has been demonstrated by source inspection, build output, runtime behavior, validation tooling, or an installed Flatpak.

For technical work:

- Facts MUST be tied to code, runtime output, logs, tests, or official tooling.
- Unknown behavior MUST remain unknown until tested.
- A hypothesis MUST have a falsifiable check.
- Product code SHOULD NOT be changed before the failing boundary or cause is demonstrated, except for an explicitly agreed diagnostic-only change.
- Changes SHOULD be small and isolated. Unrelated code MUST NOT be rewritten during a targeted fix.

## 2. Canonical technology baseline

The reference implementation uses:

- Rust 2024 edition
- GTK4
- libadwaita
- Meson as the GNOME build layer
- Cargo for Rust compilation and tests
- GNU gettext for localization
- Flatpak with the GNOME Platform/SDK
- the `org.freedesktop.Sdk.Extension.rust-stable` SDK extension

Application-specific crates and feature modules are **not** part of the Golden Standard.

## 3. Repository and source hygiene

A conforming repository MUST:

- contain only active product code and intentionally retained support files;
- contain no obsolete product names, dead feature modules, stale desktop files, stale schemas, or unused resources;
- preserve `Cargo.lock` for release reproducibility;
- keep project documentation under `docs/` when it is not primary README material;
- keep feature-specific implementation separate from reusable platform concerns where practical.

Before a release candidate is called stable, a repo-wide legacy-name scan MUST return zero unintended matches.

## 4. Build identity and profiles

Stable and development builds SHOULD come from the same source tree and templates.

Data Inspector uses:

- stable ID: `io.github.christiaanbruinsma.DataInspector`
- development ID: `io.github.christiaanbruinsma.DataInspector.Devel`

The active application ID MUST consistently drive:

- desktop file name and desktop ID;
- AppStream/MetaInfo component ID;
- application icon installation name;
- symbolic application icon installation name;
- About/application identity;
- runtime application registration.

Stable and `.Devel` variants MUST NOT require duplicated UI/resource implementations merely to change identity.

Normal development verification uses GNOME Builder's regular **Build** action. **Clean/Rebuild** SHOULD only be used when the toolchain/configuration changed or stale artifacts have been demonstrated.

## 5. Native icon standard

Normal interface icons MUST be requested by semantic `icon-name` through GTK.

They MUST NOT:

- hardcode SVG or PNG files for normal UI actions;
- detect Fluent, Zorin, Papirus, Adwaita, Breeze, or another user icon theme;
- alter the active user's icon theme to make an icon resolve;
- silently substitute a semantically unrelated icon because it happens to exist.

Icon choices SHOULD be centralized in a dedicated module such as `src/icons.rs`, with the intended semantic meaning documented.

Before an icon is accepted:

1. its semantic meaning MUST match the action;
2. its name MUST be verified rather than assumed from memory;
3. runtime resolution MUST be checked in the application environment;
4. the installed Flatpak MUST be checked against the host's active icon theme.

Suite-wide sidebar toggles use:

- left sidebar: `panel-left-symbolic`
- right sidebar / Inspector: `panel-right-symbolic`

When an appropriate semantic icon cannot be relied on across runtimes/themes, the application MAY bundle that icon as a `hicolor` fallback. Such fallbacks MUST be installed under the semantic name and MUST NOT unnecessarily override the user's normal icon theme.

Data Inspector's debug builds perform a one-time `GtkIconTheme::has_icon()` audit for all required semantic UI icons. Other apps SHOULD implement the same development guard when they have a defined required icon set.

Application identity icons are separate assets from normal action icons. They SHOULD be installed under `hicolor` using the active application ID.

See [ICON-AUDIT.md](ICON-AUDIT.md) for the canonical Data Inspector icon audit.

## 6. Native GTK/libadwaita UI conventions

Applications SHOULD use native GTK/libadwaita widgets and behavior before introducing custom UI mechanisms.

Suite conventions include:

- no more than two sidebars unless a different structure is explicitly justified;
- a right-side contextual panel is treated as the Inspector pattern;
- non-delete destructive-adjacent actions such as closing/removing an item from the current view remain neutral at rest and use native destructive treatment only where appropriate;
- explicit Delete actions use native destructive treatment;
- custom destructive CSS SHOULD NOT replace native libadwaita styling;
- separators/dividers MUST retain clear native spacing from adjacent cards, boxes, and controls;
- About and Quit belong in the application/header menu unless the app has a stronger native reason otherwise;
- ordinary UI icons follow the semantic icon standard above.

The exact Data Inspector three-area layout is a reference pattern, not a mandatory layout for every application.

## 7. Localization standard

English is the source/fallback language.

The suite localization order is:

1. English
2. Dutch
3. German
4. French
5. Spanish
6. Italian
7. Portuguese

Runtime UI and desktop/AppStream metadata SHOULD use the same supported language set when applicable.

Data Inspector keeps gettext initialization and helpers behind a small `i18n` boundary. Other applications SHOULD likewise avoid scattering localization plumbing throughout feature code.

User data, file contents, identifiers, and other inspected source values MUST NOT be translated.

## 8. Flatpak and packaging

The application SHOULD be built and tested in the same Flatpak/GNOME SDK context used by GNOME Builder.

Runtime permissions MUST be minimal and justified by actual functionality.

Cargo network access during development or release-candidate dependency resolution is not a publication standard. Dependencies SHOULD be vendored before publishing through a build service that requires offline/reproducible sources.

The installed application MUST contain its generated MetaInfo file under:

`/app/share/metainfo/<APP_ID>.metainfo.xml`

The generated desktop file, MetaInfo ID, launchable desktop ID, app icons, and runtime application ID MUST agree.

A local single-file `.flatpak` preview in GNOME Software is **not** an authoritative AppStream QA surface. For local bundle QA, use `appstreamcli` validation and inspect the installed Flatpak contents. Repository/store presentation can be verified separately when the app is published through an AppStream-producing repository.

## 9. GNOME Builder QA commands

Checks that depend on the Flatpak SDK SHOULD run through GNOME Builder's **Build Pipeline** locality rather than the host shell.

Data Inspector uses these project commands:

### Clippy (strict)

- Shell command: `cargo clippy --all-targets --all-features -- -D warnings`
- Working directory: `$SRCDIR/`
- Locality: `Build Pipeline`
- Use Subshell: off

### Cargo Tests

- Shell command: `cargo test --all-targets --all-features`
- Working directory: `$SRCDIR/`
- Locality: `Build Pipeline`
- Use Subshell: off

### AppStream Validate

- Shell command: `appstreamcli validate --explain data/io.github.christiaanbruinsma.DataInspector.Devel.metainfo.xml`
- Working directory: `$BUILDDIR/`
- Locality: `Build Pipeline`
- Use Subshell: off

Equivalent commands in another app MUST use that app's generated identity/path.

`cargo fmt` and `cargo fmt --check` do not require GTK system libraries and may be run from the project source directory.

## 10. Required QA gates

A release candidate is not stable merely because it compiles.

All applicable gates MUST pass:

1. **Source hygiene** — no unintended legacy/dead product content.
2. **Build** — normal GNOME Builder build succeeds.
3. **Formatting** — `cargo fmt --check` succeeds.
4. **Clippy strict** — all targets/features pass with `-D warnings`.
5. **Tests** — all automated tests pass.
6. **Runtime smoke** — the app launches and core interactions do not panic/crash.
7. **Icon audit** — all required semantic icons resolve.
8. **Localization smoke** — supported languages and English fallback behave correctly.
9. **AppStream validation** — generated MetaInfo passes `appstreamcli validate --explain`.
10. **Installed Flatpak QA** — exported package installs, launches, and integrates correctly with the host.
11. **Feature-specific smoke/robustness** — app-specific workflows and malformed/edge-case inputs are tested where relevant.

After a code change that can affect runtime behavior, a final normal Build/Run MUST be performed before calling the candidate stable.

See [RELEASE-CHECKLIST.md](RELEASE-CHECKLIST.md) for the Data Inspector release checklist.

## 11. Golden baseline evidence — 2026-08-10

The Data Inspector reference baseline was verified with the following evidence:

- normal GNOME Builder build: PASS;
- `cargo fmt --check`: PASS;
- strict Clippy across all targets/features: PASS with zero warnings/errors;
- automated tests: 12/12 PASS;
- runtime launch: PASS;
- semantic icon runtime audit under `Fluent-grey`: 9/9 PASS;
- generated AppStream MetaInfo validation: PASS;
- exported and installed `.Devel` Flatpak: PASS;
- installed application launch after final metadata patch: PASS.

The recurring EGL/ZINK messages observed during Builder runs were environmental GPU/graphics warnings and did not prevent Data Inspector from starting or passing its runtime/icon checks.

## 12. Conformance for other apps

When another application is audited against this standard, classify each applicable item as:

- **PASS** — conforms to the Golden Standard;
- **DEVIATION** — differs; the technical reason and impact must be stated;
- **N/A** — rule is genuinely not applicable.

An untested item is **NOT TESTED**, not PASS or N/A.

The purpose of this standard is consistency of engineering quality and native platform behavior, not forced duplication of Data Inspector's app-specific features.
