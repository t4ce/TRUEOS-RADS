# TRUEOS RADS Architecture

TRUEOS RADS is a local-first rapid application design surface for TRUEOS/UI2
apps. The current implementation is intentionally compact: one Rust binary
serves a browser UI, keeps one active project in memory, writes generated
project files to disk, and runs build-style jobs against that generated project.

## Runtime Shape

```text
Browser UI
  static/index.html
  static/app.js
  static/styles.css
      |
      | HTTP JSON + server-sent events
      v
Axum server
  /api/palette
  /api/project
  /api/project/control
  /api/jobs
  /api/events
      |
      +--> Designer model: palette, controls, inspector data
      +--> Generator: project directories and JSON/Rust/template files
      +--> Job manager: async check/build/pack/full-auto tasks
      v
rads-workspace/<project-slug>/
  app.blueprint.json
  rads.project.json
  ui/main.ui2
  ui/main.ui2.json
  Cargo.toml
  src/main.rs
  src/ui.rs
  src/events.rs
  package/package.blueprint.json
  package/manifest.trueos.json
  README.md
```

## Main Components

| Component | Responsibility | Current backing module |
| --- | --- | --- |
| Server | Bind `127.0.0.1:7377`, serve the UI, expose JSON APIs, stream job events. | `src/server.rs` |
| Model | Serialize the RADS project, TRUEOS blueprint, UI2 windows, controls, events, and geometry. | `src/model.rs` |
| Designer | Provide grouped palettes, snap/align helpers, object-inspector data, and request-shaped control mutations. | `src/designer.rs` |
| Generator | Create project directories and write generated JSON, Cargo, Rust, and manifest files. | `src/generator.rs` |
| Templates | Render the generated app's `Cargo.toml`, UI2 layout, Rust modules, package manifest, and README. | `src/templates.rs` |
| Jobs | Track recent jobs, run `cargo check`/`cargo build`, simulate packaging, and broadcast SSE updates. | `src/jobs.rs` |
| Watcher | Watch a project tree and schedule full-auto jobs after create/modify events when runtime watch is enabled. | `src/watcher.rs` |

## State Model

The server owns an `AppState` with:

- `workspace`: the root where generated projects are written.
- `active`: an in-memory `Option<RadsProject>` guarded by a Tokio mutex.
- `jobs`: a clonable `JobManager` with an in-memory recent-job list and a broadcast channel.
- `runtime`: watch state for the active project.
- `full_auto`: an atomic toggle used by watcher-driven automation.

Only one project is active at a time. Creating or loading a project replaces
the active pointer and refreshes the optional watcher. Adding controls, saving,
and updating the main window rewrite generated project files.

## Generated Project Contract

A generated RADS project is both a design artifact and a buildable TRUEOS app
stub. The stable files are:

- `rads.project.json`: the complete RADS design document.
- `app.blueprint.json`: TRUEOS app identity, schema, entrypoint, UI layout, metadata, and capabilities.
- `package/package.blueprint.json`: package identity and output artifact declarations.
- `ui/main.ui2`: readable UI2 layout with decoration flags and event bindings.
- `ui/main.ui2.json`: the first UI2 window, including geometry, decorations, controls, properties, and handlers.
- `Cargo.toml`: generated Rust package for the app.
- `src/main.rs`: generated app entrypoint that delegates UI creation and event wiring.
- `src/ui.rs`: generated UI2 window creation helper.
- `src/events.rs`: generated event handler stubs.
- `package/manifest.trueos.json`: package-level metadata pointing back to app, package, and layout blueprints.
- `README.md`: generated project notes.

The generator currently writes the first window to `ui/main.ui2.json`. Multi-window
generation should preserve this contract while adding deterministic file names
for additional windows.

## Job Flow

Jobs are created through `POST /api/jobs` after a project is active.

```text
queued -> running -> passed
queued -> running -> failed
```

Each transition or log line is stored on the job and emitted as an SSE `job`
event. The recent-job buffer keeps the latest 100 jobs. `check` and `build`
run Cargo in the generated project directory. `pack` currently validates the
expected plan through simulated pack steps. `auto` runs check first and then
pack if check succeeds.

## Design Constraints

- RADS is a userspace tool; generated projects are plain files under the local workspace.
- The web UI should stay a thin, inspectable shell over the JSON model until editing semantics harden.
- Project JSON is the source of truth for regeneration.
- Generated code should stay boring and predictable so diffs are reviewable.
- Jobs should emit enough lines for the UI to explain what happened without requiring server logs.

## Current Gaps

- Only one active project is held in memory.
- Object-inspector data is modeled in Rust; the browser UI may lag newer inspector/editing helpers.
- Packaging is simulated rather than producing a final `.tapp`.
- API errors are plain strings; a structured error envelope would make clients easier to harden.
