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
  "mode": "system",
  "titlebar": true,
  "bottom_bar": true,
  "title_icon": true,
  "toggle_composition": true,
  "fork": true,
  "close": true,
  "minimize": true,
  "restore": true,
  "maximize": true,
  "preserve_vm": true,
  "resizable": true,
  "resize_button": true,
  "rotate_buttons": false,
  "always_on_top": false
}
```

Scrollbar placement lives in the window options beside the scrollbar mode:

```json
{
  "scrollbars": "both",
  "vertical_scrollbar_side": "right",
  "horizontal_scrollbar_side": "top"
}
```

The readable layout also serializes decoration flags, for example:

```text
decoration-flags [titlebar, bottom-bar, title-icon, close, minimize, restore, maximize, resizable]
```

The generated Rust maps these fields to `vui2::WindowDecorationOptions`, so RADS
can emit top and bottom bar toggles, per-button visibility, title icon
visibility, resize button visibility, and scrollbar side choices directly.

## Controls and Events

Controls carry a stable kind and a generated handler name. The current default
event is:

- `click` for `button` and `check-box`
- `change` for `text-box`
- `select` for `list-box`
- `draw` for `canvas`
- `ready` for other controls

Button controls may also carry an optional Twemoji glyph property. RADS renders
that glyph through the same TRUEOS Twemoji atlas used by the window decoration
preview and title glyph picker:

```json
{
  "key": "glyph",
  "value": "💾"
}
```

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
