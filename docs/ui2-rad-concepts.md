# UI2 and RAD Concepts

TRUEOS RADS treats a UI2 app as a generated project whose design surface,
runtime bootstrap, and package metadata are all derived from one JSON model.
The goal is fast iteration: create a window, place controls, inspect generated
files, run checks, and package the app without hand-copying a starter tree.

## Vocabulary

| Term | Meaning in RADS |
| --- | --- |
| RADS project | The complete design document plus generated files for one app. |
| Blueprint | TRUEOS app identity and capability declaration in `app.blueprint.json`. |
| UI2 window | A top-level window with caption, geometry, decorations, and controls. |
| Control | A UI element with a kind, name, caption, rectangle, properties, and event bindings. |
| Object inspector | The right-side view of selected window/control properties. |
| Tool palette | The left-side list of controls that can be added to a window. |
| Job | A check, build, pack, or full-auto task run against the generated project. |

## Project Model

At design time, RADS stores enough information to regenerate the app:

```text
RadsProject
  id
  name
  slug
  root
  blueprint
  package
  windows[]
    id
    name
    caption
    geometry
    decorations
    controls[]
      id
      kind
      name
      caption
      geometry
      properties[]
      events[]
```

This model is persisted in `rads.project.json`. Window-specific data is emitted
to both `ui/main.ui2.json` and the readable `ui/main.ui2` layout.

## Coordinates and Geometry

RADS uses integer rectangles:

```json
{
  "x": 32,
  "y": 86,
  "w": 128,
  "h": 38
}
```

`x` and `y` are signed so future layouts can represent off-canvas or relative
positions. `w` and `h` are unsigned because controls cannot have negative size.

## Decorations

Window decorations are stored separately from geometry:

```json
{
  "titlebar": true,
  "close": true,
  "minimize": true,
  "maximize": true,
  "resizable": true,
  "always_on_top": false
}
```

The readable layout also serializes decoration flags, for example:

```text
decoration-flags [titlebar, close, minimize, maximize, resizable]
```

The generated Rust currently maps window creation to `WindowDecorationMode::System`.
The JSON keeps richer flags so future generators can target more decoration
modes without changing the design file.

## Controls and Events

Controls carry a stable kind and a generated handler name. The current default
event is:

- `click` for `button` and `check-box`
- `change` for `text-box`
- `select` for `list-box`
- `draw` for `canvas`
- `ready` for other controls

Example:

```json
{
  "kind": "button",
  "name": "runButton",
  "caption": "Click me",
  "events": [
    {
      "event": "click",
      "handler": "on_run_button_click"
    }
  ]
}
```

The generated `src/events.rs` includes handler stubs for these bindings. They
are intentionally simple so generated code remains easy to review.

## Capabilities

Blueprint capabilities declare what the app expects from TRUEOS. Starter apps
enable `ui2.window` and `ui2.events` by default and include disabled
placeholders for:

- `fs.user`
- `net.client`

This makes capability elevation explicit in diffs when the app grows.

## Full-Auto Intent

The `auto` job is the RAD loop in one command:

```text
generate files -> cargo check -> package plan -> .bp artifact when the packer is available
```

Today, generation happens when the project changes, `cargo check` runs for
`check` and `auto`, watcher-driven full-auto can be toggled through runtime
state, and packaging streams the Blueprints packer when RADS can discover it.
The intended future behavior is for install verification and kernel-side
inspection to close the loop after artifact creation.
