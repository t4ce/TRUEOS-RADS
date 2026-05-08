# RADS Multiwindow Demo 1778201382

Generated with TRUEOS RADS.

Template: Form App (`form-app`)
App kind: UI2 App

## Files

- `app.blueprint.json`: app metadata and capabilities.
- `package/package.blueprint.json`: package metadata and output artifacts.
- `ui/main.ui2`: readable UI2 layout with serialized decorations and event bindings.
- `ui/main.ui2.json`: JSON copy of the main window model.
- `ui/windows/`: per-window JSON, HTML, and CSS files for secondary UI2 windows.
- `ui/index.html`: starter markup for the main window.
- `ui/styles.css`: starter stylesheet for the main window.
- `src/ui.rs`: UI2 window creation helper.
- `src/main.rs`: app entrypoint.
- `src/events.rs`: generated event stubs.

## Run

```sh
cargo check
```
