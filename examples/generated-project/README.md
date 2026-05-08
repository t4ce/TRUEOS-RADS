# Hello UI2

Generated with TRUEOS RADS.

This checked-in specimen uses fixed UUIDs and `/tmp/rads-workspace/hello-ui2`
as a stable placeholder root. A live RADS run writes the same file families with
fresh UUIDs and a local workspace path.

## Files

- `app.blueprint.json`: app metadata and UI2 capabilities.
- `package/package.blueprint.json`: package metadata and output artifacts.
- `ui/main.ui2`: readable UI2 layout with serialized decorations and event bindings.
- `ui/main.ui2.json`: JSON copy of the main window model.
- `ui/index.html`: generated HTML description for preview/code surfaces.
- `ui/styles.css`: generated CSS description for preview/code surfaces.
- `src/ui.rs`: UI2 window creation helper.
- `src/events.rs`: generated event stubs.

## Run

```sh
cargo check
```
