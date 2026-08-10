# Data Inspector v0.9.0 Release Candidate QA

## Build identity

- [ ] Build the release manifest: `io.github.christiaanbruinsma.DataInspector.yml`.
- [ ] Confirm application ID: `io.github.christiaanbruinsma.DataInspector`.
- [ ] Confirm About/version reports `0.9.0`.
- [ ] Preserve the generated `Cargo.lock` in the release source before final publication.

## Static Rust gates

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --locked --all-targets --all-features -- -D warnings`
- [ ] `cargo test --locked`

If `Cargo.lock` does not yet exist, run one normal Cargo build/generate-lockfile first, review the lockfile, then rerun the locked gates.

## Runtime smoke

- [ ] Open JSON via file dialog.
- [ ] Open JSON via drag and drop.
- [ ] Expand/collapse nested JSON.
- [ ] Search JSON.
- [ ] Structured / Raw switching.
- [ ] Inspector selection, Copy Path, Copy Value and toast feedback.
- [ ] JSON right-click Copy Item (including children) and Copy Path.
- [ ] Open CSV and TSV.
- [ ] Verify delimiter/header detection and header override.
- [ ] Sort and resize CSV/TSV columns.
- [ ] Search CSV/TSV.
- [ ] CSV/TSV right-click Copy Cell, Copy Row and Copy Column.
- [ ] Verify empty CSV/TSV cells copy as empty values, not the visual em dash.
- [ ] Verify middle-mouse table panning.
- [ ] Verify 100k-row fixture opens with the expected row count.

## Robustness smoke

- [ ] Malformed JSON reports parse location.
- [ ] UTF-8 BOM JSON opens.
- [ ] UTF-8 BOM CSV opens.
- [ ] Invalid UTF-8 fails safely.
- [ ] Quoted delimiter CSV opens correctly.
- [ ] Multiline CSV opens correctly.
- [ ] Ragged CSV fails/handles according to the proven baseline without crashing.
- [ ] Empty files/values do not crash.

## Localization smoke

- [ ] Installed Flatpak follows the system language with English as fallback.
- [ ] Smoke `LANGUAGE=nl`, `de`, `fr`, `es`, `it`, and `pt`.
- [ ] Confirm translated labels, menus, tooltips, Inspector text, error/toast feedback, generated column names, and JSON/CSV type labels.
- [ ] Confirm JSON/CSV/TSV data values themselves are never translated.

## Installed Flatpak QA

- [ ] Export/install the stable-ID Flatpak.
- [ ] Launch the installed app successfully.
- [ ] Confirm semantic icons follow the active host icon theme.
- [ ] Confirm no missing-icon placeholders.
- [ ] Confirm the Data Inspector app icon appears in the launcher, Software/AppStream, and About dialog.
- [ ] Confirm the app-ID symbolic icon resolves in the installed runtime.
- [ ] Confirm left `panel-left-symbolic` and right `panel-right-symbolic` sidebar toggles.
- [ ] Confirm Structured and Raw icons.
- [ ] Confirm divider/card/control spacing remains native and not flush.

## Release metadata

- [ ] AppStream metadata validates in the release environment.
- [ ] Desktop metadata validates in the release environment.
- [ ] README and CHANGELOG describe v0.9.0.
- [ ] No obsolete product name remains in release content.
- [ ] Final source archive contains the intended icons/assets and `Cargo.lock`.
