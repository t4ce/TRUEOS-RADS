# HTML/CSS UI2 Description

The browser UI is a thin HTML/CSS/JavaScript shell over the RADS JSON model. It
does not own the canonical project shape; it visualizes and edits the same UI2
window, control, blueprint, and job data that the Rust APIs generate.

## Current Shell

`static/index.html` provides the root `#app` node, loads
`/static/styles.css`, and runs `/static/app.js` as a module.

The current UI is organized as:

| Surface | DOM/CSS signal | Model data |
| --- | --- | --- |
| Top bar | `.topbar` | New project, check/build/pack jobs, full-auto toggle, backend status. |
| Project tree | `.project-pane`, `.tree-row` | Active project, windows, controls, and blueprint capabilities. |
| Tool palette | `.palette-section`, `.tabs`, `.tool` | Palette categories and `ControlKind` values. |
| Editor stage | `.designer`, `.design-stage`, `.window-frame` | Selected window, zoom, grid, decorations, and controls. |
| Object inspector | `.inspector-pane`, `.field`, `.check` | Window/control geometry, captions, events, and decoration flags. |
| Job console | `.console`, `.log`, `.jobs` | Local log lines and recent job status. |

## UI2-To-CSS Mapping

Control kinds serialize in kebab case and are rendered as CSS classes on
`.control` nodes:

| UI2 kind | CSS class | Current visual role |
| --- | --- | --- |
| `button` | `.control.button` | Basic clickable command rectangle. |
| `label` | `.control.label` | Text label without a visible frame. |
| `text-box` | `.control.text-box` | Text input placeholder surface. |
| `check-box` | `.control.check-box` | Checkbox-style label with a square affordance. |
| `panel` | `.control.panel` | Framed container block. |
| `list-box` | `.control.list-box` | Data list block. |
| `canvas` | `.control.canvas` | Drawing surface block. |
| `menu` | `.control.menu` | Navigation/menu block. |
| `toolbar` | `.control.toolbar` | Dock-like command strip. |

The editor stage reads `geometry.x`, `geometry.y`, `geometry.w`, and
`geometry.h` directly into absolute positioning. Window decorations render as a
title bar when `decorations.titlebar` is true and the local decorations toggle
is enabled.

## Generated HTML/CSS Description

Generated projects can carry an HTML/CSS description alongside the UI2 layout:

- `ui/index.html`: template-provided markup for preview/code display.
- `ui/styles.css`: template-provided stylesheet for preview/code display.
- `Ui2Window.ui_description.html`: the window-scoped HTML description shown in
  the object inspector.
- `Ui2Window.ui_description.css`: the window-scoped CSS description shown in the
  object inspector.

The generated HTML/CSS description is not a replacement for `ui/main.ui2`.
`ui/main.ui2` remains the layout contract consumed by TRUEOS/UI2. The HTML/CSS
files give the browser UI and future preview tab a familiar inspection surface.

## Preview, Editor, And Code Tabs

Explicit Preview, Editor, and Code tabs are the UI2 surface contract for the
next UI source pass.

| Tab | Responsibility | Current status |
| --- | --- | --- |
| Preview | Render the selected UI2 window read-only, with production-like decorations and no resize handles. | Spec only. The editor stage can approximate the view when guides and selection are off. |
| Editor | Provide the design surface for selection, drag, resize, snap, palette add, and inspector edits. | Present as `.designer` and `.design-stage`. |
| Code | Show generated `ui/main.ui2`, `ui/main.ui2.json`, `src/ui.rs`, `src/events.rs`, `app.blueprint.json`, and package metadata. | Spec only. Generated files already exist on disk and are covered by tests. |

Tab state should be local UI state. Switching tabs must not mutate
`rads.project.json`; only explicit inspector, palette, drag/resize, window, or
save operations should rewrite generated files.

The Code tab should use the project file APIs when available:

- `GET /api/project/files`
- `GET /api/project/file?path=ui/main.ui2`
- `GET /api/project/file/{*path}`
- `POST` or `PUT /api/project/file` for writable generated edits

## Featurecheck Expectations

- The editor must keep `.window-frame` dimensions consistent with
  `Ui2Window.geometry`.
- The preview tab must not expose `.resize-handle` or editing data attributes.
- The code tab must render generated file contents from the active project root
  and label each file by its project-relative path.
- The code tab must include `ui/index.html` and `ui/styles.css` when a template
  provides them.
- The selected control should remain selected when moving between Editor and
  Code, but Preview should be visually read-only.
