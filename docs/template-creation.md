# Template Creation

TRUEOS RADS templates are generated from the Rust project model rather than
copied from a loose starter directory. The source of truth is a `RadsProject`
with one or more `Ui2Window` values, app and package blueprints, and the control
tree that renders into UI2 layout text.

## Creation Flow

1. Validate the display name with `validate_project_name`.
2. Derive a stable slug with `slugify`.
3. Build the model with `RadsProject::from_valid_name` or
   `RadsProject::starter`.
4. Write the project with `generator::create_project` or
   `generator::write_project_files`.
5. Render file bodies through `templates::*`.

The generated root is `<workspace>/<slug>/`.

## Generated File Contract

Every starter template must write these files:

| File | Purpose |
| --- | --- |
| `rads.project.json` | Complete RADS design document and regeneration source. |
| `app.blueprint.json` | TRUEOS app schema, identity, entrypoint, UI layout, capabilities, and metadata. |
| `package/package.blueprint.json` | Package schema, package identity, app reference, artifact plan, and metadata. |
| `ui/main.ui2` | Readable UI2 layout with window decorations, controls, properties, and events. |
| `ui/main.ui2.json` | JSON copy of the main window model. |
| `ui/index.html` | Template-provided HTML description for UI2 preview/code surfaces. |
| `ui/styles.css` | Template-provided CSS description for UI2 preview/code surfaces. |
| `Cargo.toml` | Generated Rust package metadata for the TRUEOS app stub. |
| `src/main.rs` | Entrypoint that creates the main UI2 window and wires events. |
| `src/ui.rs` | UI2 window creation helper with `MAIN_LAYOUT` and decoration constants. |
| `src/events.rs` | Generated event stubs for each control binding. |
| `package/manifest.trueos.json` | Package manifest tying app/package blueprints and layout together. |
| `README.md` | Generated project notes and the local `cargo check` command. |

## Built-In Template Defaults

The default template is a small form app. The built-in template catalog also
includes blank, canvas, and tool-window starters. All variants share the file
contract above and may provide different starter HTML, CSS, capabilities,
window geometry, and controls.

The default form template creates one `MainWindow` with system-style
decorations and three controls:

- `titleLabel`: label with a `ready` handler.
- `runButton`: button with a `click` handler.
- `inputText`: text box with a `change` handler.

Enabled capabilities are `ui2.window` and `ui2.events`. `fs.user` and
`net.client` are present but disabled so future capability changes remain
visible in diffs.

## Adding A Template Variant

Template variants should preserve the generated file contract unless the schema
version changes. Additive variants can safely change:

- starter controls and captions
- default window geometry
- default capabilities
- starter `ui/index.html` and `ui/styles.css`
- default UI2 window options and HTML/CSS description
- generated README notes
- app/package metadata

Changing file names, schema strings, layout paths, package manifest keys, or
handler naming rules should include a smoke test that parses both generated
files and `examples/generated-project/**`.

## Fixture Refresh Checklist

When the generated shape intentionally changes:

1. Regenerate a temp project through `generator::create_project`.
2. Update `examples/generated-project/**` with deterministic UUIDs and the
   stable `/tmp/rads-workspace/hello-ui2` root.
3. Update `docs/featurecheck-matrix.md` if the status or evidence changes.
4. Run `cargo fmt --check` and `cargo test`.
